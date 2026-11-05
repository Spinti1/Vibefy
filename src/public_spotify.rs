//! Read-only public playlist import. No user login, cookies, OAuth client or private data.
//! Uses the public web player's anonymous session and registered playlist query.
use crate::{addons::Collection, api::models::ArtistRef, player::LocalTrack};
use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

const QUERY: &str = "https://api-partner.spotify.com/pathfinder/v2/query";
// Registered queryPlaylist from Spotify's public mobile-web-player.f0eac2fb.js.
// Schema changes fail visibly; no dynamic code execution or authentication fallback.
const QUERY_HASH: &str = "c685ca91fb0019c87ff1225f84e1d85b26cd631fa6187f3f0900da182cb0953d";
const MAX_RESPONSE: usize = 8 * 1024 * 1024;
static RETRY_AT: AtomicU64 = AtomicU64::new(0);
const PUBLIC_REQUIRED: &str = "Make this Spotify playlist public, then paste its playlist link. Private playlists and Liked Songs cannot be imported. Spotify sign-in is not supported.";

fn script<'a>(html: &'a str, id: &str) -> Result<&'a str> {
    ensure!(
        html.len() <= MAX_RESPONSE,
        "Spotify returned an oversized playlist page"
    );
    let marker = format!("id=\"{id}\"");
    let at = html
        .find(&marker)
        .ok_or_else(|| anyhow::anyhow!("{PUBLIC_REQUIRED}"))?;
    let start = html[..at]
        .rfind("<script")
        .ok_or_else(|| anyhow::anyhow!("Unsupported public playlist page"))?;
    ensure!(
        !html[start..at].contains('>'),
        "Unsupported public playlist page"
    );
    let body = &html[at..];
    let body = &body[body
        .find('>')
        .ok_or_else(|| anyhow::anyhow!("Unsupported public playlist page"))?
        + 1..];
    Ok(&body[..body
        .find("</script>")
        .ok_or_else(|| anyhow::anyhow!("Unsupported public playlist page"))?])
}

fn anonymous_session(html: &str, id: &str) -> Result<String> {
    let state: Value = serde_json::from_str(script(html, "__NEXT_DATA__")?)?;
    let page = &state["props"]["pageProps"]["state"];
    ensure!(
        page["data"]["entity"]["uri"].as_str() == Some(&format!("spotify:playlist:{id}")),
        "{PUBLIC_REQUIRED}"
    );
    let token = page["settings"]["session"]["accessToken"].as_str().ok_or_else(|| anyhow::anyhow!("Spotify's public session is unavailable. Try again later; no Spotify login is needed."))?;
    ensure!(
        !token.is_empty() && token.len() <= 8192 && token.bytes().all(|b| b.is_ascii_graphic()),
        "Invalid public playlist session"
    );
    Ok(token.to_owned())
}

fn art(sources: &Value) -> Option<String> {
    sources
        .as_array()?
        .iter()
        .filter_map(|s| {
            let url = s["url"].as_str()?;
            crate::addons::safe_art_url(url)
                .then_some((s["width"].as_u64().unwrap_or(0).min(640), url))
        })
        .max_by_key(|(width, _)| *width)
        .map(|(_, url)| url.to_owned())
}

struct Page {
    title: String,
    image: Option<String>,
    total: usize,
    rows: usize,
    next: Option<usize>,
    tracks: Vec<LocalTrack>,
}
fn parse_page(entity: &Value, id: &str) -> Result<Page> {
    ensure!(
        entity["__typename"].as_str() == Some("Playlist")
            && entity["uri"].as_str() == Some(&format!("spotify:playlist:{id}")),
        "{PUBLIC_REQUIRED}"
    );
    let title = entity["name"]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.chars().count() <= 150)
        .ok_or_else(|| anyhow::anyhow!("Spotify returned an invalid playlist title"))?
        .to_owned();
    let content = &entity["content"];
    let total = content["totalCount"]
        .as_u64()
        .filter(|n| *n <= crate::personal_library::MAX_TRACKS as u64)
        .ok_or_else(|| anyhow::anyhow!("Unsupported playlist size (maximum 20,000 entries)"))?
        as usize;
    let rows = content["items"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Spotify did not return the public playlist contents"))?;
    ensure!(
        rows.len() <= 100,
        "Spotify returned an unexpected playlist page size"
    );
    let next = match &content["pagingInfo"]["nextOffset"] {
        Value::Null => None,
        value => Some(
            value
                .as_u64()
                .filter(|n| *n <= total as u64)
                .ok_or_else(|| anyhow::anyhow!("Invalid playlist pagination"))?
                as usize,
        ),
    };
    let mut tracks = Vec::new();
    for row in rows {
        let track = &row["itemV2"]["data"];
        match track["__typename"].as_str() {
            Some(
                "NotFound" | "RestrictedContent" | "LocalTrack" | "Episode" | "PodcastEpisode",
            ) => continue,
            Some("Track") => (),
            _ => bail!(
                "Spotify returned an unsupported playlist entry. No partial playlist was saved."
            ),
        }
        let uri = track["uri"]
            .as_str()
            .filter(|s| s.strip_prefix("spotify:track:").is_some_and(valid_id))
            .ok_or_else(|| anyhow::anyhow!("Spotify returned an invalid song identity"))?;
        let title = track["name"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 4096)
            .ok_or_else(|| anyhow::anyhow!("Spotify returned a song without a valid title"))?;
        let artists = track["artists"]["items"]
            .as_array()
            .filter(|a| !a.is_empty() && a.len() <= 50)
            .ok_or_else(|| anyhow::anyhow!("Spotify returned a song without artist metadata"))?;
        let artists = artists
            .iter()
            .map(|a| {
                let name = a["profile"]["name"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty() && s.len() <= 1000)
                    .ok_or_else(|| anyhow::anyhow!("Invalid Spotify artist metadata"))?;
                Ok(ArtistRef {
                    name: name.into(),
                    uri: a["uri"]
                        .as_str()
                        .filter(|s| s.strip_prefix("spotify:artist:").is_some_and(valid_id))
                        .map(str::to_owned),
                    ..Default::default()
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let duration_ms = track["duration"]["totalMilliseconds"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid Spotify song duration"))?;
        let album = track["albumOfTrack"]["name"]
            .as_str()
            .filter(|s| s.len() <= 4096)
            .unwrap_or_default()
            .to_owned();
        let image = art(&track["albumOfTrack"]["coverArt"]["sources"]);
        tracks.push(LocalTrack {
            uri: uri.into(),
            name: title.into(),
            title: title.into(),
            artists,
            duration_ms,
            album_name: album.clone(),
            album,
            cover_url: image.clone(),
            art_url: image.clone(),
            art_small_url: image,
            ..Default::default()
        });
    }
    let image = entity["images"]["items"]
        .as_array()
        .and_then(|a| a.iter().find_map(|i| art(&i["sources"])));
    Ok(Page {
        title,
        image,
        total,
        rows: rows.len(),
        next,
        tracks,
    })
}
fn valid_id(id: &str) -> bool {
    id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

async fn body(response: reqwest::Response) -> Result<String> {
    let status = response.status();
    if status.as_u16() == 429 {
        let seconds = crate::music_session::retry_after(
            response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok()),
            crate::playlist_tools::now(),
        );
        RETRY_AT.fetch_max(
            crate::playlist_tools::now().saturating_add(seconds),
            Ordering::Relaxed,
        );
        bail!(
            "Spotify is limiting public imports. Try again in {seconds} seconds. Your local playlists were not changed."
        );
    }
    ensure!(
        !matches!(status.as_u16(), 401 | 403 | 404),
        "{PUBLIC_REQUIRED}"
    );
    ensure!(
        status.is_success(),
        "Spotify's public playlist service is unavailable (HTTP {}). Try again later.",
        status.as_u16()
    );
    ensure!(
        response
            .content_length()
            .is_none_or(|n| n <= MAX_RESPONSE as u64),
        "Spotify response is too large"
    );
    let mut response = response;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            bytes.len().saturating_add(chunk.len()) <= MAX_RESPONSE,
            "Spotify response is too large"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8(bytes)?)
}
async fn query(client: &reqwest::Client, token: &str, id: &str, offset: usize) -> Result<Page> {
    let seconds = RETRY_AT
        .load(Ordering::Relaxed)
        .saturating_sub(crate::playlist_tools::now());
    ensure!(
        seconds == 0,
        "Spotify is limiting public imports. Try again in {seconds} seconds."
    );
    let value: Value = serde_json::from_str(&body(client.post(QUERY).bearer_auth(token).json(&json!({
        "operationName":"queryPlaylist", "variables":{"uri":format!("spotify:playlist:{id}"),"offset":offset,"limit":100,"includeItemV3":false},
        "extensions":{"persistedQuery":{"version":1,"sha256Hash":QUERY_HASH}}
    })).send().await?).await?)?;
    ensure!(
        value.get("errors").is_none(),
        "Spotify could not provide this public playlist. Make sure it is public and try again later. No partial playlist was saved."
    );
    parse_page(&value["data"]["playlistV2"], id)
}

pub async fn import(id: &str, progress: &(dyn Fn(String) + Send + Sync)) -> Result<Collection> {
    ensure!(valid_id(id), "Paste a public Spotify playlist link");
    let seconds = RETRY_AT
        .load(Ordering::Relaxed)
        .saturating_sub(crate::playlist_tools::now());
    ensure!(
        seconds == 0,
        "Spotify is limiting public imports. Try again in {seconds} seconds."
    );
    // A dedicated direct client has no user tokens/cookie store and never follows redirects.
    let client = crate::http::build_collection_client(&crate::settings::ProxyConfig::Off)
        .map_err(|e| anyhow::anyhow!(e))?;
    let html = body(
        client
            .get(format!("https://open.spotify.com/embed/playlist/{id}"))
            .send()
            .await?,
    )
    .await?;
    let token = anonymous_session(&html, id)?;
    let first = query(&client, &token, id, 0).await?;
    let total = first.total;
    let title = first.title.clone();
    let image = first.image.clone();
    let initial_tracks = first.tracks.clone();
    let initial_rows = first.rows;
    let mut tracks = Vec::new();
    let mut offset = 0;
    let mut page = first;
    loop {
        ensure!(
            page.total == total && page.title == title,
            "Playlist changed during import; please retry. No partial playlist was saved."
        );
        let end = offset + page.rows;
        ensure!(
            end <= total && (end == total || page.rows > 0),
            "Spotify returned an incomplete playlist page"
        );
        tracks.extend(page.tracks);
        progress(format!(
            "Importing public Spotify playlist · {end}/{total} entries"
        ));
        match page.next {
            Some(next) => {
                ensure!(
                    next == end && next > offset && next < total,
                    "Spotify returned repeated or incomplete playlist pages. No partial playlist was saved."
                );
                offset = next;
                tokio::time::sleep(Duration::from_millis(120)).await;
                page = query(&client, &token, id, offset).await?;
            }
            None => {
                ensure!(
                    end == total,
                    "Spotify only provided a preview. No partial playlist was saved."
                );
                break;
            }
        }
    }
    if total > initial_rows {
        let check = query(&client, &token, id, 0).await?;
        ensure!(
            check.total == total && check.title == title && check.tracks == initial_tracks,
            "Playlist changed during import; please retry. No partial playlist was saved."
        );
    }
    let skipped = total - tracks.len();
    ensure!(
        total == 0 || !tracks.is_empty(),
        "This playlist contains no importable public songs"
    );
    Ok(Collection {
        title,
        source: format!("https://open.spotify.com/playlist/{id}"),
        image,
        tracks,
        owner: Some(if skipped == 0 {
            "Imported from public Spotify".into()
        } else {
            format!(
                "Imported from public Spotify · {skipped} unavailable or unsupported entries skipped"
            )
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "0123456789012345678901";
    fn track() -> Value {
        json!({"itemV2":{"data":{"__typename":"Track","uri":format!("spotify:track:{ID}"),"name":"Example","artists":{"items":[{"profile":{"name":"Artist"}}]},"duration":{"totalMilliseconds":123456},"albumOfTrack":{"name":"Album","coverArt":{"sources":[{"url":"https://i.scdn.co/image/example","width":640}]}}}}})
    }
    fn entity() -> Value {
        json!({"__typename":"Playlist","uri":format!("spotify:playlist:{ID}"),"name":"Public example","content":{"totalCount":2,"items":[track(),track()],"pagingInfo":{"nextOffset":null}}})
    }
    #[test]
    fn public_spotify_preserves_repeated_songs_order_and_metadata() {
        let page = parse_page(&entity(), ID).unwrap();
        assert_eq!(page.rows, 2);
        assert_eq!(page.tracks.len(), 2);
        assert_eq!(page.tracks[0], page.tracks[1]);
        assert_eq!(page.tracks[0].duration_ms, 123456);
        assert_eq!(page.tracks[0].artists[0].name, "Artist");
    }
    #[test]
    fn public_spotify_rejects_private_missing_wrong_playlist_and_bad_song_id() {
        assert!(parse_page(&json!({"__typename":"NotFound"}), ID).is_err());
        let mut e = entity();
        e["uri"] = json!("spotify:playlist:another");
        assert!(parse_page(&e, ID).is_err());
        let mut e = entity();
        e["content"]["items"][0]["itemV2"]["data"]["uri"] = json!("file:///secret");
        assert!(parse_page(&e, ID).is_err());
    }
    #[test]
    fn public_spotify_rejects_oversized_and_malformed_pages() {
        let mut e = entity();
        e["content"]["totalCount"] = json!(20001);
        assert!(parse_page(&e, ID).is_err());
        let mut e = entity();
        e["content"]["pagingInfo"]["nextOffset"] = json!(-1);
        assert!(parse_page(&e, ID).is_err());
        assert!(script("<div id=\"__NEXT_DATA__\">{}</div>", "__NEXT_DATA__").is_err());
    }
    #[test]
    fn public_spotify_ignores_unsafe_artwork_and_reports_unavailable_rows() {
        let mut e = entity();
        e["content"]["items"][0]["itemV2"]["data"]["albumOfTrack"]["coverArt"]["sources"][0]["url"] =
            json!("file:///secret");
        e["content"]["items"][1] = json!({"itemV2":{"data":{"__typename":"NotFound"}}});
        let p = parse_page(&e, ID).unwrap();
        assert_eq!(p.total, 2);
        assert_eq!(p.tracks.len(), 1);
        assert!(p.tracks[0].art_url.is_none());
    }
    #[test]
    fn public_spotify_accepts_only_matching_anonymous_embed_sessions() {
        let state = json!({"props":{"pageProps":{"state":{"data":{"entity":{"uri":format!("spotify:playlist:{ID}")}},"settings":{"session":{"accessToken":"anonymous-example"}}}}}});
        let html =
            format!("<script id=\"__NEXT_DATA__\" type=\"application/json\">{state}</script>");
        assert_eq!(anonymous_session(&html, ID).unwrap(), "anonymous-example");
        assert!(anonymous_session(&html, "aaaaaaaaaaaaaaaaaaaaaa").is_err());
    }
}
