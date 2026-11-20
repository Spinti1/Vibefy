//! User-initiated, read-only imports from an isolated Spotify web-player session.
//! No developer Client ID, copied browser cookies, write API or playback grant.
use crate::{
    addons::Collection,
    api::models::Track,
    personal_library::MAX_TRACKS,
    youtube_browser::{self, Profile},
};
use anyhow::{Result, bail, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_PLAYLISTS: usize = 2_000;
static RETRY_AT: AtomicU64 = AtomicU64::new(0);
pub fn retry_remaining() -> u64 {
    RETRY_AT
        .load(Ordering::Relaxed)
        .saturating_sub(crate::playlist_tools::now())
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub editable: bool,
    pub id: String,
    pub title: String,
    pub image: Option<String>,
    pub count: Option<usize>,
}
impl Entry {
    pub fn source(&self, account: &str) -> String {
        if self.id == "liked" {
            format!(
                "vibefy:spotify:liked:{:x}",
                Sha256::digest(account.as_bytes())
            )
        } else {
            format!("https://open.spotify.com/playlist/{}", self.id)
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub account: String,
    pub name: String,
    pub entries: Vec<Entry>,
    pub loaded: bool,
    pub error: Option<String>,
}
pub fn player_origin(url: &str) -> bool {
    exact_host(url, &["open.spotify.com"])
}
pub fn metadata_origin(url: &str) -> bool {
    exact_host(url, &["api.spotify.com", "api-partner.spotify.com"])
}
fn exact_host(url: &str, hosts: &[&str]) -> bool {
    reqwest::Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.username().is_empty()
            && u.password().is_none()
            && u.port().is_none()
            && u.host_str().is_some_and(|h| hosts.contains(&h))
    })
}
pub fn allowed_navigation(url: &str) -> bool {
    exact_host(
        url,
        &[
            "open.spotify.com",
            "accounts.spotify.com",
            "www.spotify.com",
            "spotify.com",
            "appleid.apple.com",
        ],
    ) || (crate::youtube_browser::allowed_navigation(url)
        && reqwest::Url::parse(url).is_ok_and(|u| {
            u.host_str()
                .is_some_and(|h| h != "youtube.com" && !h.ends_with(".youtube.com"))
        }))
}
fn valid_id(id: &str) -> bool {
    id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}
pub fn valid_path(path: &str) -> bool {
    if matches!(path, "me" | "me/playlists" | "me/tracks") {
        return true;
    }
    path.strip_prefix("playlists/").is_some_and(|rest| {
        let (id, suffix) = rest.split_once('/').unwrap_or((rest, ""));
        valid_id(id) && matches!(suffix, "" | "items" | "tracks")
    })
}
pub fn observe_rate_limit(header: Option<&str>) {
    let now = crate::playlist_tools::now();
    let seconds = crate::music_session::retry_after(header, now).max(1);
    RETRY_AT.fetch_max(now.saturating_add(seconds), Ordering::Relaxed);
}
fn check_cooldown() -> Result<()> {
    let seconds = RETRY_AT
        .load(Ordering::Relaxed)
        .saturating_sub(crate::playlist_tools::now());
    ensure!(
        seconds == 0,
        "Spotify is limiting library reads. Try again in {seconds} seconds. Your saved playlists were kept."
    );
    Ok(())
}
async fn stored(config: &Path, write: Option<Option<Vec<u8>>>) -> Result<Option<Vec<u8>>> {
    let config = config.to_owned();
    tokio::task::spawn_blocking(move || {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        #[cfg(windows)]
        return crate::credentials::music_account_secret(&config, "spotify-library", write)
            .map_err(|_| anyhow::anyhow!("Cannot access the protected Spotify connection"));
        #[cfg(not(windows))]
        {
            let _ = (config, write);
            bail!("Spotify browser connection is currently available on Windows only")
        }
    })
    .await?
}
pub(crate) async fn profile(config: &Path) -> Result<Option<Profile>> {
    let Some(bytes) = stored(config, None).await? else {
        return Ok(None);
    };
    let mut profile: Profile = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("Reconnect Spotify to repair its local connection"))?;
    ensure!(profile.is_spotify(), "Invalid Spotify connection");
    profile.bind(config)?;
    Ok(Some(profile))
}
pub(crate) async fn read_raw(
    profile: &Profile,
    path: &str,
    offset: usize,
    limit: u32,
) -> Result<youtube_browser::Response> {
    // Rendered-page reads do not use the rate-limited Web API request path.
    let response =
        youtube_browser::spotify_read(profile, path, u32::try_from(offset)?, limit).await?;
    if response.status == 429 {
        observe_rate_limit(response.retry_after.as_deref());
        check_cooldown()?;
    }
    Ok(response)
}
pub(crate) fn successful(response: youtube_browser::Response) -> Result<Value> {
    match response.status {
        200..=299 => Ok(response.body),
        401 => bail!("Your Spotify session expired. Connect Spotify again."),
        403 => bail!(
            "Spotify refused this library read. Reconnect or import a public playlist by link."
        ),
        404 => bail!("This Spotify playlist is no longer accessible to your account."),
        code => bail!(
            "Spotify could not read the library (HTTP {code}). Your saved playlists were kept."
        ),
    }
}
pub(crate) async fn read(
    profile: &Profile,
    path: &str,
    offset: usize,
    limit: u32,
) -> Result<Value> {
    successful(read_raw(profile, path, offset, limit).await?)
}
pub(crate) fn identity(value: &Value) -> Result<(String, String)> {
    let id = value["id"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
        .ok_or_else(|| anyhow::anyhow!("Spotify did not confirm the connected account"))?;
    let name = value["display_name"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 512)
        .unwrap_or("Your Spotify account");
    Ok((id.into(), name.into()))
}
fn image(value: &Value) -> Option<String> {
    value
        .as_array()?
        .iter()
        .filter_map(|i| i["url"].as_str())
        .find(|u| crate::addons::safe_art_url(u))
        .map(str::to_owned)
}
/// Ignore supplied next URLs. The next request always targets a fixed Spotify resource.
pub(crate) fn page(
    value: &Value,
    offset: usize,
    maximum: usize,
) -> Result<(&Vec<Value>, usize, bool)> {
    let total = value["total"]
        .as_u64()
        .filter(|n| *n <= maximum as u64)
        .ok_or_else(|| {
            anyhow::anyhow!("Spotify library exceeds the supported import limit ({maximum})")
        })? as usize;
    let rows = value["items"]
        .as_array()
        .filter(|r| r.len() <= 50)
        .ok_or_else(|| anyhow::anyhow!("Spotify returned an unsupported library page"))?;
    ensure!(
        value["offset"].as_u64() == Some(offset as u64) && offset + rows.len() <= total,
        "Spotify returned inconsistent library pagination"
    );
    let more = !value["next"].is_null();
    ensure!(
        !more || (!rows.is_empty() && offset + rows.len() < total),
        "Spotify returned repeated library pagination"
    );
    ensure!(
        more || offset + rows.len() == total,
        "Spotify returned an incomplete library page. No partial copy was saved."
    );
    Ok((rows, total, more))
}
pub async fn load(
    config: &Path,
    connect: bool,
    progress: &(dyn Fn(String) + Send + Sync),
) -> Result<Option<Catalog>> {
    let previous = profile(config).await?;
    let profile = if connect {
        progress("Finish signing in in the Spotify window…".into());
        let profile = match previous {
            Some(profile) => profile,
            None => Profile::new_spotify(config)?,
        };
        // Retain the same isolated profile on cancellation or service refusal.
        // Retrying sign-in must not leave a new browser directory each time.
        stored(config, Some(Some(serde_json::to_vec(&profile)?))).await?;
        youtube_browser::spotify_connect(config, Some(profile)).await?
    } else if let Some(profile) = previous {
        profile
    } else {
        return Ok(None);
    };
    youtube_browser::spotify_invalidate(&profile).await?;
    progress("Reading playlists from your Spotify web player…".into());
    let (account, name) = identity(&read(&profile, "me", 0, 1).await?)?;
    // Identity and library availability are separate. A list error must not
    // turn a verified signed-in account into a disconnected account.
    let result = load_entries(&profile, &account, progress).await;
    let (entries, error) = match result {
        Ok(entries) => (entries, None),
        Err(error) => (
            vec![Entry {
                editable: true,
                id: "liked".into(),
                title: "Liked Songs".into(),
                image: None,
                count: None,
            }],
            Some(error.to_string()),
        ),
    };
    ensure!(
        identity(&read(&profile, "me", 0, 1).await?)?.0 == account,
        "The Spotify account changed. Reconnect before importing."
    );
    Ok(Some(Catalog {
        account,
        name,
        entries,
        loaded: true,
        error,
    }))
}
async fn load_entries(
    profile: &Profile,
    account: &str,
    progress: &(dyn Fn(String) + Send + Sync),
) -> Result<Vec<Entry>> {
    let count = None;
    let mut entries = vec![Entry {
        editable: true,
        id: "liked".into(),
        title: "Liked Songs".into(),
        image: None,
        count,
    }];
    let mut offset = 0;
    let mut total_before = None;
    let mut seen = HashSet::new();
    loop {
        let value = read(&profile, "me/playlists", offset, 50).await?;
        let (rows, total, more) = page(&value, offset, MAX_PLAYLISTS)?;
        ensure!(
            total_before.is_none_or(|n| n == total),
            "Your Spotify library changed while loading. Refresh the list."
        );
        total_before = Some(total);
        for row in rows {
            if row.is_null() {
                continue;
            }
            let id = row["id"]
                .as_str()
                .filter(|s| valid_id(s))
                .ok_or_else(|| anyhow::anyhow!("Spotify returned an invalid playlist identity"))?;
            if !seen.insert(id.to_owned()) {
                continue;
            }
            let title = row["name"]
                .as_str()
                .filter(|s| !s.trim().is_empty() && s.chars().count() <= 150)
                .ok_or_else(|| anyhow::anyhow!("Spotify returned an invalid playlist name"))?;
            entries.push(Entry {
                editable: row["owner"]["id"].as_str() == Some(account)
                    || row["collaborative"] == true,
                id: id.into(),
                title: title.into(),
                image: image(&row["images"]),
                count: row["items"]["total"]
                    .as_u64()
                    .or_else(|| row["tracks"]["total"].as_u64())
                    .and_then(|n| usize::try_from(n).ok()),
            });
        }
        offset += rows.len();
        if !more {
            break;
        }
        progress(format!("Loading Spotify playlists · {offset}/{total}"));
    }
    Ok(entries)
}
pub async fn disconnect(config: &Path) -> Result<()> {
    if let Some(profile) = profile(config).await? {
        youtube_browser::forget(&profile).await?;
    }
    stored(config, Some(None)).await?;
    Ok(())
}
fn parse_tracks(rows: &[Value]) -> Result<(Vec<crate::player::LocalTrack>, usize)> {
    let mut tracks = Vec::new();
    let mut skipped = 0;
    for row in rows {
        let value = row
            .get("item")
            .filter(|v| !v.is_null())
            .or_else(|| row.get("track"))
            .unwrap_or(&Value::Null);
        if value.is_null() || value["is_local"] == true || value["type"].as_str() == Some("episode")
        {
            skipped += 1;
            continue;
        }
        let track: Track = serde_json::from_value(value.clone())
            .map_err(|_| anyhow::anyhow!("Spotify returned unsupported song metadata"))?;
        ensure!(
            track
                .uri
                .strip_prefix("spotify:track:")
                .is_some_and(valid_id)
                && !track.name.is_empty()
                && !track.artists.is_empty(),
            "Spotify returned incomplete song metadata. No partial copy was saved."
        );
        let mut local = crate::playlist_tools::local_track(&track);
        // Keep the recording edition in the metadata used by the existing resolver.
        if track.explicit && !local.album.to_lowercase().contains("explicit") {
            local.album.push_str(" [Explicit]");
        }
        ensure!(
            crate::personal_library::valid_track(&local),
            "Spotify returned invalid song metadata"
        );
        tracks.push(local);
    }
    Ok((tracks, skipped))
}
pub async fn import(
    config: &Path,
    entry: &Entry,
    account: &str,
    progress: &(dyn Fn(String) + Send + Sync),
) -> Result<Collection> {
    Ok(import_details(config, entry, account, progress).await?.0)
}
pub async fn import_details(
    config: &Path,
    entry: &Entry,
    account: &str,
    progress: &(dyn Fn(String) + Send + Sync),
) -> Result<(Collection, bool)> {
    ensure!(
        entry.id == "liked" || valid_id(&entry.id),
        "Invalid Spotify playlist"
    );
    let profile = profile(config)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Connect Spotify before importing your library"))?;
    youtube_browser::spotify_invalidate(&profile).await?;
    ensure!(
        identity(&read(&profile, "me", 0, 1).await?)?.0 == account,
        "The Spotify account changed. Refresh the library before importing."
    );
    let mut title = entry.title.clone();
    let mut art = entry.image.clone();
    let mut snapshot = None;
    let mut editable = entry.id == "liked";
    if entry.id != "liked" {
        let header = read(&profile, &format!("playlists/{}", entry.id), 0, 1).await?;
        ensure!(
            header["id"].as_str() == Some(entry.id.as_str()),
            "Spotify returned a different playlist"
        );
        title = header["name"]
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.chars().count() <= 150)
            .ok_or_else(|| anyhow::anyhow!("Invalid Spotify playlist name"))?
            .into();
        art = image(&header["images"]);
        editable =
            header["owner"]["id"].as_str() == Some(account) || header["collaborative"] == true;
        snapshot = header["snapshot_id"].as_str().map(str::to_owned);
    }
    let mut path = if entry.id == "liked" {
        "me/tracks".into()
    } else {
        format!("playlists/{}/items", entry.id)
    };
    let mut offset = 0;
    let mut expected = None;
    let mut first = Value::Null;
    let mut tracks = Vec::new();
    let mut skipped = 0;
    loop {
        let response = read_raw(&profile, &path, offset, 50).await?;
        let response = if offset == 0 && response.status == 404 && entry.id != "liked" {
            // Older web-player accounts still expose the previous read route.
            path = format!("playlists/{}/tracks", entry.id);
            read_raw(&profile, &path, 0, 50).await?
        } else {
            response
        };
        let value = successful(response)?;
        let (rows, total, more) = page(&value, offset, MAX_TRACKS)?;
        ensure!(
            expected.is_none_or(|n| n == total),
            "The Spotify playlist changed while importing. Retry; the old local copy was kept."
        );
        expected = Some(total);
        if offset == 0 {
            first = value.clone();
        }
        let (songs, unavailable) = parse_tracks(rows)?;
        tracks.extend(songs);
        skipped += unavailable;
        offset += rows.len();
        progress(format!("Importing {} · {offset}/{total} songs", title));
        if !more {
            break;
        }
    }
    let final_page = read(&profile, &path, 0, 50).await?;
    ensure!(
        first == final_page,
        "The Spotify playlist changed while importing. Retry; the old local copy was kept."
    );
    if let Some(snapshot) = snapshot {
        let header = read(&profile, &format!("playlists/{}", entry.id), 0, 1).await?;
        ensure!(
            header["snapshot_id"].as_str() == Some(&snapshot),
            "The Spotify playlist changed while importing. Retry; the old local copy was kept."
        );
    }
    ensure!(
        identity(&read(&profile, "me", 0, 1).await?)?.0 == account,
        "The Spotify account changed during import. No copy was saved."
    );
    if skipped > 0 {
        progress(format!(
            "{} imported · {skipped} unavailable, local-file or podcast entries skipped",
            title
        ));
    }
    let collection = Collection {
        source: entry.source(account),
        title,
        image: art,
        owner: Some(if skipped == 0 {
            "Imported from Spotify".into()
        } else {
            format!("Imported from Spotify · {skipped} unsupported entries skipped")
        }),
        tracks,
        ..Default::default()
    };
    ensure!(
        crate::personal_library::validate(&collection),
        "Imported Spotify playlist metadata is invalid"
    );
    Ok((collection, editable))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn spotify_import_accepts_only_fixed_read_resources_and_exact_origins() {
        for path in [
            "me",
            "me/playlists",
            "me/tracks",
            "playlists/1234567890123456789012/items",
        ] {
            assert!(valid_path(path));
        }
        for path in [
            "me/player",
            "me/playlists?url=https://evil.test",
            "https://evil.test",
            "playlists/../me",
            "playlists/1234567890123456789012/followers",
        ] {
            assert!(!valid_path(path));
        }
        assert!(player_origin("https://open.spotify.com/collection/tracks"));
        for url in [
            "http://open.spotify.com",
            "https://open.spotify.com.evil.test",
            "https://user@open.spotify.com",
            "https://accounts.spotify.com",
        ] {
            assert!(!player_origin(url));
        }
        assert!(!allowed_navigation("https://evil.test"));
        assert!(!metadata_origin("https://api.spotify.com.evil.test"));
    }
    #[test]
    fn spotify_import_rejects_missing_rows_and_repeated_pages() {
        assert!(
            page(
                &json!({"offset":0,"total":2,"items":[{}],"next":null}),
                0,
                20
            )
            .is_err()
        );
        assert!(
            page(
                &json!({"offset":0,"total":2,"items":[],"next":"https://evil.test"}),
                0,
                20
            )
            .is_err()
        );
        let value = json!({"offset":0,"total":1,"items":[{}],"next":null});
        assert!(page(&value, 0, 20).is_ok());
    }
    #[test]
    fn spotify_import_keeps_repeats_explicit_metadata_and_account_specific_likes() {
        let row = json!({"track":{"type":"track","uri":"spotify:track:1234567890123456789012","name":"Example","explicit":true,"artists":[{"name":"Artist"}],"album":{"name":"Album"}}});
        let (tracks, skipped) = parse_tracks(&[row.clone(), json!({"track":null}), row]).unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(skipped, 1);
        assert_eq!(tracks[0].uri, tracks[1].uri);
        assert!(tracks[0].album.contains("Explicit"));
        let entry = Entry {
            editable: true,
            id: "liked".into(),
            title: "Liked Songs".into(),
            image: None,
            count: None,
        };
        assert_ne!(entry.source("one"), entry.source("two"));
        assert_eq!(entry.source("one"), entry.source("one"));
    }
    #[test]
    fn spotify_import_reads_the_new_item_field_when_legacy_track_is_null() {
        let item = json!({"type":"track","uri":"spotify:track:1234567890123456789012","name":"Example","artists":[{"name":"Artist"}]});
        let (tracks, skipped) = parse_tracks(&[json!({"item":item,"track":null})]).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(skipped, 0);
    }
}
