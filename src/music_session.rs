//! Explicit YouTube Music browser-session import. Never reads browser databases.
//! Protocol reference: sigma67/ytmusicapi (see CREDITS.md); no Python runtime.
use crate::{addons::Collection, music_accounts::LibraryPlaylist, settings::ProxyConfig};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::{collections::HashSet, path::Path, time::Duration};

#[derive(Debug, thiserror::Error)]
pub enum Failure {
    #[error("YouTube Music is limiting requests; sync will retry later")]
    RateLimited(u64),
    #[error("Cannot connect to YouTube Music; nothing was sent")]
    NotSent,
    #[error("YouTube Music rejected the request (HTTP {0}); reconnect or check permissions")]
    Rejected(u16),
    #[error("YouTube Music did not confirm the change; verify the playlist before retrying")]
    Unconfirmed,
}

const ORIGIN: &str = "https://music.youtube.com";
const MAX_SESSION: usize = 64 * 1024;

fn find_section<'a>(value: &'a Value, key: &str, depth: usize) -> Option<&'a Value> {
    if depth > 60 {
        return None;
    }
    if let Some(section) = value.get(key) {
        return Some(section);
    }
    match value {
        Value::Object(map) => map.values().find_map(|v| find_section(v, key, depth + 1)),
        Value::Array(list) => list.iter().find_map(|v| find_section(v, key, depth + 1)),
        _ => None,
    }
}

fn playlist_section(value: &Value) -> Result<&Value> {
    // A page also contains related playlists and suggested songs. Import only
    // the actual playlist shelf (or its append continuation), never those extras.
    let section = value
        .pointer("/continuationContents/musicPlaylistShelfContinuation")
        .or_else(|| {
            value.pointer(
                "/onResponseReceivedActions/0/appendContinuationItemsAction/continuationItems",
            )
        })
        .or_else(|| find_section(value, "musicPlaylistShelfRenderer", 0))
        .ok_or_else(|| anyhow::anyhow!("YouTube Music returned an unsupported playlist page"))?;
    Ok(section)
}
pub fn parse_playlist_page(value: &Value) -> Result<crate::music::Results> {
    crate::music::parse_library_browse(&json!({"contents":playlist_section(value)?}))
}
/// Membership includes unavailable rows. Availability must never cause re-adds.
pub fn playlist_video_ids(value: &Value) -> Result<Vec<String>> {
    fn walk(value: &Value, ids: &mut Vec<String>, depth: usize) {
        if depth > 60 {
            return;
        }
        if let Some(row) = value.get("musicResponsiveListItemRenderer") {
            let id = row
                .pointer("/playlistItemData/videoId")
                .and_then(Value::as_str)
                .or_else(|| {
                    row.pointer("/navigationEndpoint/watchEndpoint/videoId")
                        .and_then(Value::as_str)
                });
            if let Some(id) = id.filter(|id| crate::addons::valid_video_id(id)) {
                ids.push(id.to_owned());
            }
            return;
        }
        match value {
            Value::Object(map) => {
                for child in map.values() {
                    walk(child, ids, depth + 1);
                }
            }
            Value::Array(list) => {
                for child in list {
                    walk(child, ids, depth + 1);
                }
            }
            _ => {}
        }
    }
    let mut ids = Vec::new();
    walk(playlist_section(value)?, &mut ids, 0);
    Ok(ids)
}

pub fn playlist_details(value: &Value) -> (bool, String) {
    let Some(header) = find_section(value, "musicEditablePlaylistDetailHeaderRenderer", 0) else {
        return (false, String::new());
    };
    let editor = find_section(header, "musicPlaylistEditHeaderRenderer", 0);
    let private = editor.and_then(|v| v["privacy"].as_str()) == Some("PRIVATE");
    let description = editor
        .and_then(|v| v.pointer("/description/runs"))
        .and_then(Value::as_array)
        .map(|runs| {
            runs.iter()
                .filter_map(|r| r["text"].as_str())
                .collect::<String>()
        })
        .or_else(|| {
            find_section(header, "musicDescriptionShelfRenderer", 0)
                .and_then(|v| v.pointer("/description/runs"))
                .and_then(Value::as_array)
                .map(|runs| {
                    runs.iter()
                        .filter_map(|r| r["text"].as_str())
                        .collect::<String>()
                })
        })
        .unwrap_or_default();
    (private, description)
}
pub fn retry_after(value: Option<&str>, now: u64) -> u64 {
    value
        .and_then(|value| {
            value.parse::<u64>().ok().or_else(|| {
                jiff::fmt::rfc2822::parse(value)
                    .ok()
                    .map(|date| date.timestamp().as_second().max(0) as u64)
                    .map(|date| date.saturating_sub(now))
            })
        })
        .unwrap_or(60)
        .max(1)
}
pub fn private_playlist_body(title: &str, marker: &str) -> Result<Value> {
    let title: String = title
        .chars()
        .filter(|c| !matches!(c, '<' | '>'))
        .take(150)
        .collect();
    ensure!(
        !title.trim().is_empty() && marker.len() <= 1000,
        "The playlist needs a valid title and marker"
    );
    Ok(json!({"title":title,"description":marker,"privacyStatus":"PRIVATE"}))
}
pub fn append_confirmed(value: &Value, expected: &[String]) -> bool {
    if value["status"].as_str() != Some("STATUS_SUCCEEDED") {
        return false;
    }
    let Some(results) = value["playlistEditResults"].as_array() else {
        return false;
    };
    let ids: Vec<_> = results
        .iter()
        .filter_map(|r| {
            r.pointer("/playlistEditVideoAddedResultData/videoId")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    ids.len() == expected.len()
        && crate::playlist_mirror::counts(ids)
            == crate::playlist_mirror::counts(expected.iter().cloned())
}

pub fn parse_saved_playlists_page(value: &Value) -> Result<crate::music::Results> {
    let section = value
        .pointer("/continuationContents/gridContinuation")
        .or_else(|| {
            value.pointer(
                "/onResponseReceivedActions/0/appendContinuationItemsAction/continuationItems",
            )
        })
        .or_else(|| find_section(value, "gridRenderer", 0))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Cannot open the signed-in Music playlist library; reconnect the session"
            )
        })?;
    crate::music::parse_library_browse(&json!({"contents": section}))
}

// Deliberately no Debug: these fields authorize a browser session.
#[derive(Serialize, Deserialize)]
pub struct Session {
    cookie: String,
    account: String,
    page: Option<String>,
}
impl Session {
    /// Accept ytmusicapi browser.json or request headers copied from DevTools.
    /// Ignore Authorization, Host, URL and other caller-controlled destinations.
    pub fn parse(input: &str) -> Result<Self> {
        ensure!(
            input.len() <= MAX_SESSION,
            "The session import is too large"
        );
        let mut headers = std::collections::HashMap::new();
        if input.trim_start().starts_with('{') {
            let value: Value = serde_json::from_str(input)
                .map_err(|_| anyhow::anyhow!("Invalid browser session JSON"))?;
            for (key, value) in value
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Expected a browser header object"))?
            {
                if let Some(value) = value.as_str() {
                    headers.insert(key.to_ascii_lowercase(), value.trim().to_owned());
                }
            }
        } else {
            let mut lines = input.lines();
            while let Some(line) = lines.next() {
                let line = line.trim();
                if line.is_empty() || line.starts_with(':') {
                    continue;
                }
                if let Some((key, value)) = line.split_once(':') {
                    headers.insert(key.to_ascii_lowercase(), value.trim().to_owned());
                } else if let Some(value) = lines.next() {
                    // Chromium's two-line header display.
                    headers.insert(line.to_ascii_lowercase(), value.trim().to_owned());
                }
            }
        }
        for key in ["origin", "x-origin"] {
            ensure!(
                headers.get(key).is_none_or(|value| value == ORIGIN),
                "Copy headers from music.youtube.com only"
            );
        }
        let cookie = headers.remove("cookie").ok_or_else(|| {
            anyhow::anyhow!("Missing Cookie header; copy a signed-in YouTube Music request")
        })?;
        ensure!(
            !cookie.is_empty() && cookie.is_ascii() && !cookie.bytes().any(|b| b < 32 || b == 127),
            "Invalid session cookie header"
        );
        let account = headers
            .remove("x-goog-authuser")
            .unwrap_or_else(|| "0".into());
        ensure!(
            !account.is_empty()
                && account.len() <= 3
                && account.bytes().all(|b| b.is_ascii_digit()),
            "Invalid Music account selector"
        );
        let page = headers.remove("x-goog-pageid");
        ensure!(
            page.as_ref().is_none_or(|s| !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')),
            "Invalid Music channel selector"
        );
        let session = Self {
            cookie,
            account,
            page,
        };
        session.sapisid()?;
        Ok(session)
    }
    fn sapisid(&self) -> Result<&str> {
        ["__Secure-3PAPISID", "SAPISID"]
            .iter()
            .find_map(|name| {
                self.cookie
                    .split(';')
                    .filter_map(|c| c.trim().split_once('='))
                    .find_map(|(key, value)| (key == *name && !value.is_empty()).then_some(value))
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "The Music session has no signing cookie. Sign in and copy a new request."
                )
            })
    }
    pub fn authorization_at(&self, timestamp: u64) -> Result<String> {
        let hash = Sha1::digest(format!("{timestamp} {} {ORIGIN}", self.sapisid()?).as_bytes());
        Ok(format!("SAPISIDHASH {timestamp}_{hash:x}"))
    }
    pub async fn read(path: &Path) -> Result<Self> {
        use tokio::io::AsyncReadExt;
        let file = tokio::fs::File::open(path)
            .await
            .map_err(|_| anyhow::anyhow!("Cannot open the selected session file"))?;
        let mut bytes = Vec::new();
        file.take((MAX_SESSION + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| anyhow::anyhow!("Cannot read the selected session file"))?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| anyhow::anyhow!("Select a UTF-8 browser header file"))?;
        Self::parse(text)
    }
    async fn request(
        &self,
        client: &reqwest::Client,
        continuation: Option<&str>,
        id: &str,
    ) -> Result<Value> {
        let mut body = json!({"browseId":id});
        let query = if let Some(token) = continuation {
            ensure!(token.len() <= 16384, "Invalid library continuation");
            if id.starts_with("VL") {
                body = json!({"continuation":token});
                None
            } else {
                Some(token)
            }
        } else {
            None
        };
        self.post(client, "browse", body, query).await
    }
    async fn post(
        &self,
        client: &reqwest::Client,
        endpoint: &str,
        mut body: Value,
        continuation: Option<&str>,
    ) -> Result<Value> {
        ensure!(
            matches!(
                endpoint,
                "browse" | "account/account_menu" | "playlist/create" | "browse/edit_playlist"
            ),
            "Unsupported Music operation"
        );
        let date = jiff::Timestamp::now()
            .to_zoned(jiff::tz::TimeZone::UTC)
            .strftime("%Y%m%d")
            .to_string();
        let version = format!("1.{date}.01.00");
        body["context"] = json!({"client":{"clientName":"WEB_REMIX","clientVersion":version,"hl":"en"},"user":{}});
        if let Some(page) = &self.page {
            body["context"]["user"]["onBehalfOfUser"] = page.clone().into();
        }
        let sensitive = |value: &str| -> Result<reqwest::header::HeaderValue> {
            let mut header = reqwest::header::HeaderValue::from_str(value)
                .map_err(|_| anyhow::anyhow!("Invalid Music session header"))?;
            header.set_sensitive(true);
            Ok(header)
        };
        let mut request = client
            .post(format!("https://music.youtube.com/youtubei/v1/{endpoint}"))
            .query(&[("alt", "json")])
            .header("Origin", ORIGIN)
            .header("X-Origin", ORIGIN)
            .header("X-Youtube-Client-Name", "67")
            .header("X-Youtube-Client-Version", version)
            .header("X-Goog-AuthUser", &self.account)
            .header("Cookie", sensitive(&self.cookie)?)
            .header(
                "Authorization",
                sensitive(
                    &self.authorization_at(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                    )?,
                )?,
            )
            .timeout(Duration::from_secs(20));
        if let Some(token) = continuation {
            request = request.query(&[("continuation", token), ("ctoken", token)]);
        }
        if let Some(page) = &self.page {
            request = request.header("X-Goog-PageId", page);
        }
        let response = request.json(&body).send().await.map_err(|error| {
            if error.is_connect() {
                Failure::NotSent
            } else {
                Failure::Unconfirmed
            }
        })?;
        let status = response.status();
        if status.as_u16() == 429 {
            let seconds = retry_after(
                response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok()),
                crate::playlist_mirror::now(),
            );
            return Err(Failure::RateLimited(seconds).into());
        }
        if status.is_client_error() {
            return Err(Failure::Rejected(status.as_u16()).into());
        }
        ensure!(status.is_success(), Failure::Unconfirmed);
        let value = crate::music_accounts::json(response).await?;
        ensure!(
            value.pointer("/responseContext/mainAppWebResponseContext/loggedOut")
                != Some(&Value::Bool(true)),
            "YouTube Music is signed out; import a new session"
        );
        ensure!(
            value.get("error").is_none(),
            "YouTube Music could not open this library"
        );
        Ok(value)
    }
    fn client(proxy: &ProxyConfig) -> Result<reqwest::Client> {
        // Cookie is a custom header: never allow redirects to carry it elsewhere.
        crate::http::build_collection_client(proxy).map_err(anyhow::Error::msg)
    }
    pub async fn identity(&self, proxy: &ProxyConfig) -> Result<(String, String)> {
        let value = self
            .post(
                &Self::client(proxy)?,
                "account/account_menu",
                json!({}),
                None,
            )
            .await?;
        let account = value.pointer("/actions/0/openPopupAction/popup/multiPageMenuRenderer/header/activeAccountHeaderRenderer")
            .ok_or_else(|| anyhow::anyhow!("Music is signed out; reconnect the browser session"))?;
        let name = account
            .pointer("/accountName/runs/0/text")
            .and_then(Value::as_str)
            .unwrap_or("YouTube Music")
            .to_owned();
        let handle = account.pointer("/channelHandle/runs/0/text").and_then(Value::as_str)
            .filter(|s| s.starts_with('@') && s.len() <= 150)
            .ok_or_else(|| anyhow::anyhow!("Open YouTube Music and create/select your channel before enabling playlist sync"))?;
        Ok((
            format!(
                "{}:{}",
                handle.to_lowercase(),
                self.page.as_deref().unwrap_or("")
            ),
            name,
        ))
    }
    pub async fn details(&self, proxy: &ProxyConfig, id: &str) -> Result<(bool, String)> {
        ensure!(valid_playlist_id(id), "Invalid Music playlist identity");
        let value = self
            .request(&Self::client(proxy)?, None, &format!("VL{id}"))
            .await?;
        Ok(playlist_details(&value))
    }
    pub async fn create_private(
        &self,
        proxy: &ProxyConfig,
        title: &str,
        marker: &str,
    ) -> Result<String> {
        let value = self
            .post(
                &Self::client(proxy)?,
                "playlist/create",
                private_playlist_body(title, marker)?,
                None,
            )
            .await?;
        value["playlistId"]
            .as_str()
            .filter(|id| valid_playlist_id(id))
            .map(str::to_owned)
            .ok_or_else(|| Failure::Unconfirmed.into())
    }
    pub async fn append(&self, proxy: &ProxyConfig, id: &str, videos: &[String]) -> Result<bool> {
        ensure!(
            valid_playlist_id(id)
                && !videos.is_empty()
                && videos.len() <= 50
                && videos.iter().all(|id| crate::addons::valid_video_id(id)),
            "Invalid playlist addition"
        );
        let actions: Vec<_> = videos.iter().map(|id|json!({"action":"ACTION_ADD_VIDEO","addedVideoId":id,"dedupeOption":"DEDUPE_OPTION_SKIP"})).collect();
        let value = self
            .post(
                &Self::client(proxy)?,
                "browse/edit_playlist",
                json!({"playlistId":id,"actions":actions}),
                None,
            )
            .await?;
        ensure!(
            value["status"].as_str() == Some("STATUS_SUCCEEDED"),
            Failure::Unconfirmed
        );
        Ok(append_confirmed(&value, videos))
    }
    pub async fn playlists(&self, proxy: &ProxyConfig) -> Result<Vec<LibraryPlaylist>> {
        let client = Self::client(proxy)?;
        let mut playlists = Vec::new();
        let mut next = None;
        let mut pages = HashSet::new();
        let mut ids = HashSet::new();
        for _ in 0..200 {
            let value = self
                .request(&client, next.as_deref(), "FEmusic_liked_playlists")
                .await?;
            let parsed = parse_saved_playlists_page(&value)?;
            for entry in parsed.entries {
                if entry.kind == crate::music::Kind::Playlist
                    && entry.id.starts_with("VL")
                    && ids.insert(entry.id.clone())
                {
                    playlists.push(LibraryPlaylist {
                        id: entry.id,
                        title: entry.title,
                        image: entry.image,
                    });
                }
            }
            match parsed.continuation {
                Some(token) => {
                    ensure!(
                        pages.insert(token.clone()),
                        "Music returned a repeated library page; please retry"
                    );
                    next = Some(token);
                }
                None => return Ok(playlists),
            }
        }
        anyhow::bail!(
            "The Music library exceeded the supported page limit; no partial library was saved"
        )
    }
    pub async fn membership(
        &self,
        proxy: &ProxyConfig,
        id: &str,
    ) -> Result<std::collections::BTreeMap<String, usize>> {
        ensure!(valid_playlist_id(id), "Invalid Music playlist identity");
        let client = Self::client(proxy)?;
        let mut next = None;
        let mut pages = HashSet::new();
        let mut ids = Vec::new();
        for _ in 0..400 {
            let value = self
                .request(&client, next.as_deref(), &format!("VL{id}"))
                .await?;
            ids.extend(playlist_video_ids(&value)?);
            ensure!(
                ids.len() <= 20_000,
                "This Music playlist exceeds 20,000 entries"
            );
            match parse_playlist_page(&value)?.continuation {
                Some(token) => {
                    ensure!(
                        pages.insert(token.clone()),
                        "Music returned a repeated playlist page"
                    );
                    next = Some(token);
                }
                None => return Ok(crate::playlist_mirror::counts(ids)),
            }
        }
        anyhow::bail!("This Music playlist exceeded the supported page limit")
    }
    pub async fn import(
        &self,
        proxy: &ProxyConfig,
        playlist: &LibraryPlaylist,
    ) -> Result<Collection> {
        ensure!(
            playlist.id.starts_with("VL")
                && playlist.id.len() <= 130
                && playlist
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "Invalid Music playlist identity"
        );
        let client = Self::client(proxy)?;
        let mut tracks = Vec::new();
        let mut next = None;
        let mut pages = HashSet::new();
        for _ in 0..400 {
            let value = self.request(&client, next.as_deref(), &playlist.id).await?;
            let parsed = parse_playlist_page(&value)?;
            tracks.extend(parsed.entries.into_iter().filter_map(|entry| entry.track));
            ensure!(
                tracks.len() <= 20_000,
                "This Music playlist exceeds 20,000 songs"
            );
            match parsed.continuation {
                Some(token) => {
                    ensure!(
                        pages.insert(token.clone()),
                        "Music returned a repeated playlist page; please retry"
                    );
                    next = Some(token);
                }
                None => {
                    return Ok(Collection {
                        title: playlist.title.clone(),
                        source: format!(
                            "https://www.youtube.com/playlist?list={}",
                            &playlist.id[2..]
                        ),
                        tracks,
                        image: playlist.image.clone(),
                        owner: Some("YouTube Music".into()),
                    });
                }
            }
        }
        anyhow::bail!(
            "The Music playlist exceeded the supported page limit; no partial playlist was saved"
        )
    }
}

pub fn valid_playlist_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
