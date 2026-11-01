//! Explicit YouTube Music browser-session import. Never reads browser databases.
//! Protocol reference: sigma67/ytmusicapi (see CREDITS.md); no Python runtime.
use crate::{addons::Collection, music_accounts::LibraryPlaylist, settings::ProxyConfig};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::{collections::HashSet, path::Path, time::Duration};

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

pub fn parse_playlist_page(value: &Value) -> Result<crate::music::Results> {
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
    crate::music::parse_library_browse(&json!({"contents": section}))
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
        let date = jiff::Timestamp::now()
            .to_zoned(jiff::tz::TimeZone::UTC)
            .strftime("%Y%m%d")
            .to_string();
        let version = format!("1.{date}.01.00");
        let mut body = json!({"browseId":id,"context":{"client":{"clientName":"WEB_REMIX","clientVersion":version,"hl":"en"},"user":{}}});
        if let Some(page) = &self.page {
            body["context"]["user"]["onBehalfOfUser"] = page.clone().into();
        }
        let mut request = client
            .post("https://music.youtube.com/youtubei/v1/browse")
            .query(&[("alt", "json")])
            .header("Origin", ORIGIN)
            .header("X-Origin", ORIGIN)
            .header("X-Youtube-Client-Name", "67")
            .header("X-Youtube-Client-Version", version)
            .header("X-Goog-AuthUser", &self.account)
            .header("Cookie", &self.cookie)
            .header(
                "Authorization",
                self.authorization_at(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                )?,
            )
            .timeout(Duration::from_secs(20));
        if let Some(token) = continuation {
            ensure!(token.len() <= 16384, "Invalid library continuation");
            if id.starts_with("VL") {
                body.as_object_mut().unwrap().remove("browseId");
                body["continuation"] = token.into();
            } else {
                request = request.query(&[("continuation", token), ("ctoken", token)]);
            }
        }
        if let Some(page) = &self.page {
            request = request.header("X-Goog-PageId", page);
        }
        let response = request.json(&body).send().await.map_err(|_| {
            anyhow::anyhow!(
                "Cannot reach YouTube Music; check the connection or reconnect the session"
            )
        })?;
        ensure!(
            !matches!(response.status().as_u16(), 401 | 403),
            "The YouTube Music session expired or was rejected; reconnect it"
        );
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
