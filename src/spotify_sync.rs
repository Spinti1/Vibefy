//! Opt-in additions only. No remote deletions, renames or automatic recording guesses.
use crate::{
    addons::Collection,
    api::{
        client::{ApiError, Result as ApiResult},
        models::Track,
    },
    playlist_outbox::Addition,
    spotify_library::{self as library, Entry},
    youtube_browser::{self, Profile},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub account: String,
    pub id: String,
    #[serde(default)]
    pub writes: bool,
    #[serde(default)]
    pub snapshot: String,
}
pub fn account_key(account: &str) -> String {
    format!("{:x}", Sha256::digest(account.as_bytes()))
}
pub fn valid_playlist(id: &str) -> bool {
    id == "liked" || (id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric()))
}
pub fn valid_addition(id: &str, uris: &[String]) -> bool {
    valid_playlist(id)
        && !uris.is_empty()
        && uris.len() <= 100
        && uris.iter().all(|u| crate::spotify_match::valid_uri(u))
}
/// Preserve local additions and Spotify's intentional repeated occurrences.
pub fn append_remote(local: &mut Collection, remote: Vec<crate::player::LocalTrack>) -> usize {
    let counts = crate::playlist_tools::counts(local.tracks.iter().map(|t| t.uri.clone()));
    let mut seen = std::collections::BTreeMap::<String, usize>::new();
    let before = local.tracks.len();
    for track in remote {
        let count = seen.entry(track.uri.clone()).or_default();
        *count += 1;
        if *count > counts.get(&track.uri).copied().unwrap_or(0) {
            local.tracks.push(track);
        }
    }
    local.tracks.len() - before
}
pub async fn session(config: &Path, expected: &str) -> Result<(Profile, String)> {
    let profile = library::profile(config)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Connect Spotify before using this playlist"))?;
    let (account, _) = library::identity(&library::read(&profile, "me", 0, 1).await?)?;
    ensure!(
        account_key(&account) == expected,
        "The Spotify account changed. Reconnect the account that owns this copy."
    );
    Ok((profile, account))
}
pub async fn probe(config: &Path) -> Result<Option<library::Catalog>> {
    let Some(profile) = library::profile(config).await? else {
        return Ok(None);
    };
    let (account, name) = library::identity(&library::read(&profile, "me", 0, 1).await?)?;
    Ok(Some(library::Catalog {
        account,
        name,
        ..Default::default()
    }))
}
pub async fn search(
    config: &Path,
    expected: &str,
    source: &Track,
    query: &str,
) -> Result<Vec<Track>> {
    let wait = library::retry_remaining();
    ensure!(
        wait == 0,
        "Spotify is limiting searches or changes. Try again in {wait} seconds. Local playlist imports remain available."
    );
    let (profile, _) = session(config, expected).await?;
    let response = youtube_browser::spotify_search(&profile, query).await?;
    if response.status == 429 {
        library::observe_rate_limit(response.retry_after.as_deref());
    }
    let value = library::successful(response)?;
    let rows = value["tracks"]["items"]
        .as_array()
        .filter(|v| v.len() <= 50)
        .ok_or_else(|| anyhow::anyhow!("Spotify did not return recording choices"))?;
    let mut tracks = rows
        .iter()
        .filter_map(|v| serde_json::from_value::<Track>(v.clone()).ok())
        .filter(|t| crate::spotify_match::valid_uri(&t.uri) && !t.is_local)
        .collect::<Vec<_>>();
    tracks.sort_by_cached_key(|t| {
        std::cmp::Reverse(crate::spotify_match::score(
            &source.name,
            &source.artist_names(),
            source.duration_ms,
            &t.name,
            &t.artist_names(),
            t.duration_ms,
        ))
    });
    Ok(tracks)
}
pub async fn watch(
    config: &Path,
    binding: &Binding,
    progress: &(dyn Fn(String) + Send + Sync),
) -> Result<(String, Option<Collection>)> {
    ensure!(
        valid_playlist(&binding.id),
        "Invalid watched Spotify playlist"
    );
    let (profile, account) = session(config, &binding.account).await?;
    let (snapshot, title, image) = if binding.id == "liked" {
        let page = library::read(&profile, "me/tracks", 0, 20).await?;
        (
            account_key(&serde_json::to_string(&page)?),
            "Liked Songs".into(),
            None,
        )
    } else {
        let header = library::read(&profile, &format!("playlists/{}", binding.id), 0, 1).await?;
        ensure!(
            header["id"].as_str() == Some(binding.id.as_str()),
            "Spotify returned a different playlist"
        );
        (
            header["snapshot_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            header["name"]
                .as_str()
                .unwrap_or("Spotify playlist")
                .to_owned(),
            None,
        )
    };
    if !snapshot.is_empty() && snapshot == binding.snapshot {
        return Ok((snapshot, None));
    }
    let entry = Entry {
        editable: true,
        id: binding.id.clone(),
        title,
        image,
        count: None,
    };
    let collection = library::import(config, &entry, &account, progress).await?;
    Ok((snapshot, Some(collection)))
}
fn status(response: youtube_browser::Response) -> ApiResult<Value> {
    match response.status {
        200..=299 => Ok(response.body),
        429 => {
            let wait = crate::music_session::retry_after(
                response.retry_after.as_deref(),
                crate::playlist_tools::now(),
            )
            .max(1);
            library::observe_rate_limit(response.retry_after.as_deref());
            Err(ApiError::PlaylistRetryAfter(wait))
        }
        401 => Err(ApiError::NotSignedIn),
        code => Err(ApiError::Status {
            status: code,
            message: format!(
                "Spotify refused this addition (HTTP {code}). The local song is kept."
            ),
        }),
    }
}
async fn before_write(
    profile: &Profile,
    path: &str,
    offset: usize,
    limit: u32,
) -> ApiResult<Value> {
    match library::read_raw(profile, path, offset, limit).await {
        Ok(response) => status(response),
        Err(_) => {
            let wait = library::retry_remaining();
            if wait > 0 {
                Err(ApiError::PlaylistRetryAfter(wait))
            } else {
                Err(ApiError::PlaylistNotSent)
            }
        }
    }
}
pub async fn add(config: &Path, addition: &Addition) -> ApiResult<Option<String>> {
    if addition.position.is_some() || !valid_addition(&addition.playlist_id, &addition.uris) {
        return Err(ApiError::Status {
            status: 400,
            message: "Invalid Spotify playlist addition".into(),
        });
    }
    let wait = library::retry_remaining();
    if wait > 0 {
        return Err(ApiError::PlaylistRetryAfter(wait));
    }
    let profile = library::profile(config)
        .await
        .map_err(|_| ApiError::NotSignedIn)?
        .ok_or(ApiError::NotSignedIn)?;
    youtube_browser::spotify_invalidate(&profile)
        .await
        .map_err(|_| ApiError::PlaylistNotSent)?;
    let (account, _) = library::identity(&before_write(&profile, "me", 0, 1).await?)
        .map_err(|_| ApiError::NotSignedIn)?;
    if account != addition.account {
        return Err(ApiError::NotSignedIn);
    }
    let mut missing = addition.uris.clone();
    if addition.playlist_id != "liked" {
        let header = before_write(
            &profile,
            &format!("playlists/{}", addition.playlist_id),
            0,
            1,
        )
        .await?;
        if header["id"].as_str() != Some(addition.playlist_id.as_str())
            || !(header["owner"]["id"].as_str() == Some(account.as_str())
                || header["collaborative"] == true)
        {
            return Err(ApiError::Status{status:403,message:"Spotify has not confirmed that you can edit this playlist. Your local copy is kept.".into()});
        }
        let mut existing = HashSet::new();
        let mut offset = 0;
        let mut path = format!("playlists/{}/items", addition.playlist_id);
        loop {
            let response = library::read_raw(&profile, &path, offset, 50)
                .await
                .map_err(|_| {
                    let wait = library::retry_remaining();
                    if wait > 0 {
                        ApiError::PlaylistRetryAfter(wait)
                    } else {
                        ApiError::PlaylistNotSent
                    }
                })?;
            let value = if offset == 0 && response.status == 404 {
                path = format!("playlists/{}/tracks", addition.playlist_id);
                before_write(&profile, &path, 0, 50).await?
            } else {
                status(response)?
            };
            let (rows, _, more) = library::page(
                &value,
                offset,
                crate::personal_library::MAX_TRACKS,
            )
            .map_err(|_| {
                ApiError::Decode("Unsupported Spotify playlist page; nothing was sent".into())
            })?;
            for row in rows {
                let item = row
                    .get("item")
                    .filter(|v| !v.is_null())
                    .or_else(|| row.get("track"));
                if let Some(uri) = item.and_then(|v| v["uri"].as_str()) {
                    existing.insert(uri.to_owned());
                }
            }
            offset += rows.len();
            if !more {
                break;
            }
        }
        missing.retain(|u| !existing.contains(u));
    }
    if missing.is_empty() {
        return Ok(None);
    }
    // Identity is checked immediately before the only mutating request.
    let (current, _) = library::identity(&before_write(&profile, "me", 0, 1).await?)
        .map_err(|_| ApiError::NotSignedIn)?;
    if current != addition.account {
        return Err(ApiError::NotSignedIn);
    }
    let response = youtube_browser::spotify_add(&profile, &addition.playlist_id, &missing)
        .await
        .map_err(|_| ApiError::Network("Spotify did not confirm the write".into()))?;
    let value = status(response)?;
    Ok(value["snapshot_id"].as_str().map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spotify_sync_mutations_are_limited_to_song_additions() {
        let uris = vec!["spotify:track:1234567890123456789012".into()];
        assert!(valid_addition("1234567890123456789012", &uris));
        assert!(valid_addition("liked", &uris));
        for id in ["me/player", "../other", "https://evil.test", "123"] {
            assert!(!valid_addition(id, &uris));
        }
        assert!(!valid_addition("liked", &["file:///private".into()]));
        assert!(!valid_addition("liked", &[]));
    }
    #[test]
    fn spotify_sync_merges_additions_without_deleting_pending_songs_or_growing_repeats() {
        let song = |id: &str| crate::player::LocalTrack {
            uri: id.into(),
            ..Default::default()
        };
        let mut local = Collection {
            tracks: vec![song("a"), song("pending")],
            ..Default::default()
        };
        let remote = vec![song("a"), song("a"), song("b")];
        assert_eq!(append_remote(&mut local, remote.clone()), 2);
        assert_eq!(append_remote(&mut local, remote), 0);
        assert_eq!(
            local
                .tracks
                .iter()
                .map(|t| t.uri.as_str())
                .collect::<Vec<_>>(),
            ["a", "pending", "a", "b"]
        );
    }
}
