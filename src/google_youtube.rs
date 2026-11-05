//! Official Google desktop OAuth and YouTube Data API. Never imports browser cookies.
use crate::{
    addons::Collection, api::models::ArtistRef, music_accounts::LibraryPlaylist,
    music_session::Failure, player::LocalTrack, settings::ProxyConfig,
};
use anyhow::{Result, bail, ensure};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::Mutex,
};

pub const SCOPE: &str = "https://www.googleapis.com/auth/youtube.force-ssl";
const API: &str = "https://www.googleapis.com/youtube/v3/";
const TOKEN: &str = "https://oauth2.googleapis.com/token";

// Desktop clients are public clients. This application configuration is not a
// user credential; refresh/access tokens are stored only in the protected store.
#[derive(Serialize, Deserialize)]
struct Client {
    client_id: String,
    client_secret: String,
}
fn client_config() -> Result<Client> {
    let value: Value =
        serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/google-oauth.json")))
            .map_err(|_| {
                anyhow::anyhow!("This Vibefy build has no valid Google sign-in configuration")
            })?;
    let value = value.get("installed").unwrap_or(&value);
    let client: Client = serde_json::from_value(value.clone())
        .map_err(|_| anyhow::anyhow!("Google sign-in is not configured in this Vibefy build"))?;
    ensure!(
        client.client_id.ends_with(".apps.googleusercontent.com")
            && !client.client_secret.is_empty(),
        "Invalid Google application configuration"
    );
    Ok(client)
}
#[derive(Default)]
struct Access {
    token: String,
    expires_at: u64,
}
// Deliberately no Debug. Only serialized into protected per-user storage.
#[derive(Serialize, Deserialize)]
pub struct Session {
    client: Client,
    refresh: String,
    channel: String,
    name: String,
    #[serde(skip)]
    access: Mutex<Access>,
}
fn nonce() -> String {
    let mut bytes = [0; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
pub fn authorization_url(
    client_id: &str,
    redirect: &str,
    state: &str,
    verifier: &str,
) -> Result<String> {
    let mut url = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth")?;
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("redirect_uri", redirect),
        ("response_type", "code"),
        ("scope", SCOPE),
        ("state", state),
        (
            "code_challenge",
            &crate::music_accounts::challenge(verifier),
        ),
        ("code_challenge_method", "S256"),
        ("access_type", "offline"),
        ("prompt", "consent select_account"),
    ]);
    Ok(url.to_string())
}
/// None is an unrelated local request. A matching refusal ends the login.
pub fn callback_code(line: &str, expected_state: &str) -> Result<Option<String>> {
    ensure!(line.len() <= 8192, "Invalid Google callback");
    let mut parts = line.split_whitespace();
    if parts.next() != Some("GET") {
        return Ok(None);
    }
    let target = parts.next().unwrap_or("");
    if !target.starts_with("/youtube/callback?") {
        return Ok(None);
    }
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))?;
    let pairs: Vec<_> = url.query_pairs().collect();
    let states: Vec<_> = pairs.iter().filter(|(key, _)| key == "state").collect();
    if states.len() != 1 || states[0].1.as_ref() != expected_state {
        return Ok(None);
    }
    let mut fields = BTreeMap::new();
    for (key, value) in pairs {
        ensure!(
            fields.insert(key.to_string(), value.to_string()).is_none(),
            "Duplicate Google callback field"
        );
    }
    if fields.get("state").map(String::as_str) != Some(expected_state) {
        return Ok(None);
    }
    ensure!(
        !fields.contains_key("error"),
        "Google sign-in was declined. You can try again."
    );
    Ok(Some(
        fields
            .remove("code")
            .filter(|s| !s.is_empty() && s.len() <= 4096)
            .ok_or_else(|| anyhow::anyhow!("Google did not return a sign-in code"))?,
    ))
}
async fn token(client: &reqwest::Client, fields: &[(&str, &str)]) -> Result<Value> {
    let response = client
        .post(TOKEN)
        .form(fields)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|_| {
            anyhow::anyhow!("Cannot reach Google sign-in. Check your connection and try again.")
        })?;
    if !response.status().is_success() {
        bail!(
            "Google did not authorize this connection. Reconnect YouTube or check the application's Google access settings."
        );
    }
    crate::music_accounts::json(response).await
}
impl Session {
    pub async fn connect(proxy: &ProxyConfig) -> Result<Self> {
        let client = client_config()?;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| anyhow::anyhow!("Cannot open the local Google sign-in callback"))?;
        let redirect = format!(
            "http://127.0.0.1:{}/youtube/callback",
            listener.local_addr()?.port()
        );
        let state = nonce();
        let verifier = nonce();
        let url = authorization_url(&client.client_id, &redirect, &state, &verifier)?;
        crate::opener::open(&url)
            .map_err(|_| anyhow::anyhow!("Cannot open your default browser for Google sign-in"))?;
        let code = tokio::time::timeout(Duration::from_secs(600), async {
            loop {
                let (mut stream, peer) = listener.accept().await?;
                if !peer.ip().is_loopback() { continue; }
                let mut line = String::new();
                let read = tokio::time::timeout(Duration::from_secs(3), BufReader::new(&mut stream).take(8193).read_line(&mut line)).await;
                if !matches!(read, Ok(Ok(_))) || !line.ends_with('\n') { continue; }
                let result = callback_code(&line, &state);
                let matched = !matches!(result, Ok(None));
                let body = if matches!(result, Ok(Some(_))) { "<!doctype html><title>Vibefy</title><h2>Return to Vibefy</h2><p>Finish connecting in the app. You can close this tab.</p>" }
                    else { "<!doctype html><title>Vibefy</title><p>Return to Vibefy to check your connection.</p>" };
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; frame-ancestors 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",body.len());
                let _ = tokio::time::timeout(Duration::from_secs(2), stream.write_all(response.as_bytes())).await;
                if matched { return result.map(|v| v.unwrap()); }
            }
        }).await.map_err(|_| anyhow::anyhow!("Google sign-in timed out. Click Connect YouTube to try again."))??;
        let http = crate::http::build_collection_client(proxy).map_err(anyhow::Error::msg)?;
        let granted = token(
            &http,
            &[
                ("client_id", &client.client_id),
                ("client_secret", &client.client_secret),
                ("code", &code),
                ("code_verifier", &verifier),
                ("redirect_uri", &redirect),
                ("grant_type", "authorization_code"),
            ],
        )
        .await?;
        ensure!(
            granted["scope"]
                .as_str()
                .is_none_or(|scopes| scopes.split_whitespace().any(|s| s == SCOPE)),
            "YouTube playlist access was not granted. Reconnect and allow playlist access."
        );
        let refresh = granted["refresh_token"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Google did not grant offline access. Reconnect YouTube and accept access."
                )
            })?
            .to_owned();
        let access = access_from(&granted)?;
        let mut session = Self {
            client,
            refresh,
            channel: String::new(),
            name: String::new(),
            access: Mutex::new(access),
        };
        let identity = session
            .request(
                proxy,
                "channels",
                None,
                &[("part", "snippet".into()), ("mine", "true".into())],
            )
            .await?;
        let channel = identity["items"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Create or select a YouTube channel for this Google account, then reconnect."
                )
            })?;
        session.channel = channel["id"]
            .as_str()
            .filter(|s| s.starts_with("UC") && s.len() == 24)
            .ok_or_else(|| anyhow::anyhow!("YouTube did not identify the connected channel"))?
            .into();
        session.name = channel["snippet"]["title"]
            .as_str()
            .unwrap_or("YouTube")
            .into();
        Ok(session)
    }
    async fn access(&self, client: &reqwest::Client) -> Result<String> {
        let mut access = self.access.lock().await;
        if access.expires_at <= crate::playlist_tools::now().saturating_add(90) {
            let value = token(
                client,
                &[
                    ("client_id", &self.client.client_id),
                    ("client_secret", &self.client.client_secret),
                    ("refresh_token", &self.refresh),
                    ("grant_type", "refresh_token"),
                ],
            )
            .await?;
            *access = access_from(&value)?;
        }
        Ok(access.token.clone())
    }
    async fn request(
        &self,
        proxy: &ProxyConfig,
        endpoint: &str,
        body: Option<Value>,
        query: &[(&str, String)],
    ) -> Result<Value> {
        ensure!(
            matches!(
                endpoint,
                "channels" | "playlists" | "playlistItems" | "videos"
            ),
            "Unsupported YouTube operation"
        );
        let client = crate::http::build_collection_client(proxy).map_err(anyhow::Error::msg)?;
        let mut request = if body.is_some() {
            client.post(format!("{API}{endpoint}"))
        } else {
            client.get(format!("{API}{endpoint}"))
        };
        let mut authorization = reqwest::header::HeaderValue::from_str(&format!(
            "Bearer {}",
            self.access(&client).await?
        ))?;
        authorization.set_sensitive(true);
        request = request
            .header(reqwest::header::AUTHORIZATION, authorization)
            .query(query)
            .timeout(Duration::from_secs(20));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.map_err(|e| {
            if e.is_connect() {
                Failure::NotSent
            } else {
                Failure::Unconfirmed
            }
        })?;
        let status = response.status();
        let retry = crate::music_session::retry_after(
            response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok()),
            crate::playlist_tools::now(),
        );
        if status.as_u16() == 429 {
            return Err(Failure::RateLimited(retry).into());
        }
        if status.as_u16() == 401 {
            self.access.lock().await.expires_at = 0;
            return Err(Failure::Rejected(401).into());
        }
        if status.as_u16() == 403 {
            let value = bounded_json(response).await?;
            let reasons = value["error"]["errors"].as_array();
            if reasons.is_some_and(|a| {
                a.iter().any(|v| {
                    matches!(
                        v["reason"].as_str(),
                        Some("quotaExceeded" | "dailyLimitExceeded")
                    )
                })
            }) {
                return Err(Failure::RateLimited(86_400).into());
            }
            if reasons.is_some_and(|a| {
                a.iter().any(|v| {
                    matches!(
                        v["reason"].as_str(),
                        Some("rateLimitExceeded" | "userRateLimitExceeded")
                    )
                })
            }) {
                return Err(Failure::RateLimited(retry).into());
            }
            return Err(Failure::Rejected(403).into());
        }
        if status.is_client_error() {
            return Err(Failure::Rejected(status.as_u16()).into());
        }
        ensure!(status.is_success(), Failure::Unconfirmed);
        bounded_json(response).await
    }
    pub async fn identity(&self, _proxy: &ProxyConfig) -> Result<(String, String)> {
        ensure!(
            self.channel.starts_with("UC") && self.channel.len() == 24,
            "Reconnect YouTube to identify your channel"
        );
        Ok((format!("google:{}", self.channel), self.name.clone()))
    }
    async fn pages(
        &self,
        proxy: &ProxyConfig,
        endpoint: &str,
        mut query: Vec<(&str, String)>,
        limit: usize,
    ) -> Result<Vec<Value>> {
        query.push(("maxResults", "50".into()));
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        for _ in 0..400 {
            let value = self.request(proxy, endpoint, None, &query).await?;
            items.extend(value["items"].as_array().into_iter().flatten().cloned());
            ensure!(
                items.len() <= limit,
                "The YouTube library exceeds the supported size; no partial library was saved"
            );
            let Some(next) = value["nextPageToken"].as_str() else {
                return Ok(items);
            };
            ensure!(
                next.len() <= 16384 && seen.insert(next.to_owned()),
                "YouTube returned an invalid continuation"
            );
            query.retain(|(k, _)| *k != "pageToken");
            query.push(("pageToken", next.into()));
        }
        bail!("The YouTube library exceeded the supported page limit")
    }
    pub async fn playlists(&self, proxy: &ProxyConfig) -> Result<Vec<LibraryPlaylist>> {
        Ok(self
            .pages(
                proxy,
                "playlists",
                vec![("part", "snippet".into()), ("mine", "true".into())],
                10_000,
            )
            .await?
            .iter()
            .filter_map(|v| {
                let id = v["id"]
                    .as_str()
                    .filter(|id| crate::music_session::valid_playlist_id(id))?;
                Some(LibraryPlaylist {
                    id: format!("VL{id}"),
                    title: v["snippet"]["title"]
                        .as_str()
                        .unwrap_or("YouTube playlist")
                        .into(),
                    image: art(&v["snippet"]),
                })
            })
            .collect())
    }
    pub async fn details(&self, proxy: &ProxyConfig, id: &str) -> Result<(bool, String)> {
        ensure!(
            crate::music_session::valid_playlist_id(id),
            "Invalid YouTube playlist"
        );
        let value = self
            .request(
                proxy,
                "playlists",
                None,
                &[("part", "snippet,status".into()), ("id", id.into())],
            )
            .await?;
        let playlist = value["items"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| anyhow::anyhow!("The linked YouTube playlist is unavailable"))?;
        Ok((
            playlist["snippet"]["channelId"].as_str() == Some(&self.channel)
                && matches!(
                    playlist["status"]["privacyStatus"].as_str(),
                    Some("private" | "unlisted")
                ),
            playlist["snippet"]["description"]
                .as_str()
                .unwrap_or_default()
                .into(),
        ))
    }
    pub async fn create_unlisted(
        &self,
        proxy: &ProxyConfig,
        title: &str,
        marker: &str,
    ) -> Result<String> {
        let normalized = crate::music_session::unlisted_playlist_body(title, marker)?;
        let value = self.request(proxy,"playlists",Some(json!({"snippet":{"title":normalized["title"],"description":marker},"status":{"privacyStatus":"unlisted"}})), &[("part","snippet,status".into())]).await?;
        value["id"]
            .as_str()
            .filter(|id| crate::music_session::valid_playlist_id(id))
            .map(str::to_owned)
            .ok_or_else(|| Failure::Unconfirmed.into())
    }
    async fn items(&self, proxy: &ProxyConfig, id: &str) -> Result<Vec<Value>> {
        ensure!(
            crate::music_session::valid_playlist_id(id),
            "Invalid YouTube playlist"
        );
        self.pages(
            proxy,
            "playlistItems",
            vec![
                ("part", "snippet,contentDetails".into()),
                ("playlistId", id.into()),
            ],
            20_000,
        )
        .await
    }
    /// One mutation per durable intent. A quota/connection failure never hides
    /// earlier successful inserts inside an apparently rejected batch.
    pub async fn append(&self, proxy: &ProxyConfig, id: &str, videos: &[String]) -> Result<bool> {
        ensure!(
            crate::music_session::valid_playlist_id(id)
                && videos.len() == 1
                && crate::addons::valid_video_id(&videos[0]),
            "Invalid YouTube addition"
        );
        let value = self.request(proxy,"playlistItems",Some(json!({"snippet":{"playlistId":id,"resourceId":{"kind":"youtube#video","videoId":videos[0]}}})), &[("part","snippet".into())]).await?;
        Ok(value["id"].as_str().is_some() && video_id(&value) == Some(videos[0].as_str()))
    }
    pub async fn import(
        &self,
        proxy: &ProxyConfig,
        playlist: &LibraryPlaylist,
    ) -> Result<Collection> {
        let id = playlist.id.strip_prefix("VL").unwrap_or(&playlist.id);
        let rows = self.items(proxy, id).await?;
        let mut tracks: Vec<LocalTrack> = rows.iter().filter_map(parse_track).collect();
        let unique: Vec<_> = tracks
            .iter()
            .filter_map(|t| t.uri.strip_prefix("vibefy:youtube:"))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let mut metadata = BTreeMap::new();
        for chunk in unique.chunks(50) {
            let value = self
                .request(
                    proxy,
                    "videos",
                    None,
                    &[("part", "contentDetails".into()), ("id", chunk.join(","))],
                )
                .await?;
            for item in value["items"].as_array().into_iter().flatten() {
                if let Some(id) = item["id"].as_str() {
                    metadata.insert(
                        id.to_owned(),
                        duration_ms(
                            item["contentDetails"]["duration"]
                                .as_str()
                                .unwrap_or_default(),
                        ),
                    );
                }
            }
        }
        for track in &mut tracks {
            track.duration_ms = metadata
                .get(
                    track
                        .uri
                        .strip_prefix("vibefy:youtube:")
                        .unwrap_or_default(),
                )
                .copied()
                .unwrap_or_default();
        }
        Ok(Collection {
            title: playlist.title.clone(),
            source: format!("https://www.youtube.com/playlist?list={id}"),
            tracks,
            image: playlist.image.clone(),
            owner: Some(self.name.clone()),
        })
    }
}
fn access_from(value: &Value) -> Result<Access> {
    let token = value["access_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Google did not return an access token"))?;
    Ok(Access {
        token: token.into(),
        expires_at: crate::playlist_tools::now()
            .saturating_add(value["expires_in"].as_u64().unwrap_or(3600).min(86400)),
    })
}
async fn bounded_json(mut response: reqwest::Response) -> Result<Value> {
    ensure!(
        response
            .content_length()
            .is_none_or(|n| n <= 8 * 1024 * 1024),
        "YouTube response is too large"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Unconfirmed)? {
        ensure!(
            bytes.len().saturating_add(chunk.len()) <= 8 * 1024 * 1024,
            "YouTube response is too large"
        );
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| Failure::Unconfirmed.into())
}
fn art(snippet: &Value) -> Option<String> {
    ["maxres", "standard", "high", "medium", "default"]
        .iter()
        .find_map(|key| snippet["thumbnails"][key]["url"].as_str())
        .filter(|s| {
            reqwest::Url::parse(s).is_ok_and(|u| {
                u.scheme() == "https"
                    && u.host_str()
                        .is_some_and(|h| h == "i.ytimg.com" || h == "yt3.ggpht.com")
            })
        })
        .map(str::to_owned)
}
fn video_id(value: &Value) -> Option<&str> {
    value
        .pointer("/contentDetails/videoId")
        .or_else(|| value.pointer("/snippet/resourceId/videoId"))
        .and_then(Value::as_str)
        .filter(|id| crate::addons::valid_video_id(id))
}
pub fn parse_track(value: &Value) -> Option<LocalTrack> {
    let id = video_id(value)?;
    let snippet = &value["snippet"];
    let title = snippet["title"].as_str()?;
    if matches!(title, "Deleted video" | "Private video") {
        return None;
    }
    let artist = snippet["videoOwnerChannelTitle"]
        .as_str()
        .unwrap_or("YouTube")
        .trim_end_matches(" - Topic");
    Some(LocalTrack {
        uri: format!("vibefy:youtube:{id}"),
        name: title.into(),
        title: title.into(),
        artists: vec![ArtistRef {
            name: artist.into(),
            ..Default::default()
        }],
        cover_url: art(snippet),
        art_url: art(snippet),
        ..Default::default()
    })
}
pub fn duration_ms(value: &str) -> u32 {
    let Some(value) = value.strip_prefix("PT") else {
        return 0;
    };
    let mut number = String::new();
    let mut seconds = 0f64;
    for c in value.chars() {
        if c.is_ascii_digit() || c == '.' {
            number.push(c);
            continue;
        }
        let Ok(n) = number.parse::<f64>() else {
            return 0;
        };
        number.clear();
        seconds += n * match c {
            'H' => 3600.,
            'M' => 60.,
            'S' => 1.,
            _ => return 0,
        };
    }
    if !number.is_empty() || !seconds.is_finite() || seconds < 0. {
        return 0;
    }
    (seconds * 1000.).min(u32::MAX as f64) as u32
}
