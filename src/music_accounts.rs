//! Optional library connections with protected session storage.
use crate::{addons::Collection, api::models::ArtistRef, http::Http, player::LocalTrack};
use anyhow::{Result, bail, ensure};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
#[cfg(not(windows))]
use keyring_core::api::CredentialStoreApi;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    YouTubeMusic,
    JuiceVault,
}
impl Provider {
    pub fn key_id(self) -> &'static str {
        self.key()
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::YouTubeMusic => "YouTube Music",
            Self::JuiceVault => "JuiceVault",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::YouTubeMusic => "youtube-music-library",
            Self::JuiceVault => "juicevault-library",
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub auth_pending: bool,
    pub enable_sync_after_connect: bool,
    pub busy: bool,
    pub deleting: bool,
    pub connected: bool,
    pub name: String,
    pub message: Option<String>,
    pub playlists: Vec<LibraryPlaylist>,
    pub username: String,
    pub password: Password,
}
#[derive(Clone, Debug)]
pub struct LibraryPlaylist {
    pub id: String,
    pub title: String,
    pub image: Option<String>,
}
/// YouTube's podcast-only saved queue is not a music playlist.
pub fn excluded_youtube_playlist(id: &str) -> bool {
    id.strip_prefix("VL").unwrap_or(id) == "SE"
}
pub fn excluded_playlist_source(source: &str) -> bool {
    // Special queues have short IDs and intentionally do not pass the public
    // playlist import validator. Recognize them in old saved metadata too.
    reqwest::Url::parse(source).is_ok_and(|url| {
        url.scheme() == "https"
            && matches!(
                url.host_str(),
                Some("youtube.com" | "www.youtube.com" | "music.youtube.com")
            )
            && url
                .query_pairs()
                .any(|(key, id)| key == "list" && excluded_youtube_playlist(&id))
    })
}
#[derive(Clone, Default)]
pub struct Password(pub String);
impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    Login(Password),
    PublicProfile,
    Connect,
    Refresh,
    Delete(String),
    Disconnect,
    Import(LibraryPlaylist),
}
#[derive(Clone, Debug)]
pub struct Update {
    pub removed_source: Option<String>,
    pub name: String,
    pub playlists: Vec<LibraryPlaylist>,
    pub collection: Option<Collection>,
}
// No Debug: both fields authorize access to the user's library.
#[derive(Serialize, Deserialize)]
struct Tokens {
    access: String,
    refresh: String,
    #[serde(default)]
    username: String,
}
fn random_token() -> String {
    let mut bytes = [0; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

async fn credential(
    provider: Provider,
    config: &Path,
    write: Option<Option<Vec<u8>>>,
) -> Result<Option<Vec<u8>>> {
    // Keep the guard inside the blocking job: cancelling the async caller must
    // not allow a stale save to finish after Disconnect has deleted the entry.
    static LOCK: std::sync::OnceLock<std::sync::Arc<tokio::sync::Mutex<()>>> =
        std::sync::OnceLock::new();
    let guard = LOCK
        .get_or_init(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
        .clone()
        .lock_owned()
        .await;
    let config = config.to_owned();
    tokio::task::spawn_blocking(move || -> Result<Option<Vec<u8>>> {
        let _guard = guard;
        #[cfg(windows)]
        return crate::credentials::music_account_secret(&config, provider.key(), write)
            .map_err(|_| anyhow::anyhow!("Cannot access protected music-account storage"));
        #[cfg(not(windows))]
        {
            #[cfg(target_os = "macos")]
            let store = apple_native_keyring_store::keychain::Store::new();
            #[cfg(target_os = "linux")]
            let store = zbus_secret_service_keyring_store::Store::new();
            let store =
                store.map_err(|_| anyhow::anyhow!("The system credential store is unavailable"))?;
            let entry = store
                .build("Vibefy.MusicAccounts", provider.key(), None)
                .map_err(|_| anyhow::anyhow!("Cannot open the protected music-account entry"))?;
            match write {
                Some(Some(bytes)) => {
                    entry
                        .set_secret(&bytes)
                        .map_err(|_| anyhow::anyhow!("Cannot securely save this connection"))?;
                    Ok(None)
                }
                Some(None) => match entry.delete_credential() {
                    Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(None),
                    Err(_) => bail!("Cannot remove the saved connection"),
                },
                None => match entry.get_secret() {
                    Ok(bytes) => Ok(Some(bytes)),
                    Err(keyring_core::Error::NoEntry) => Ok(None),
                    Err(_) => bail!("Cannot read the saved connection"),
                },
            }
        }
    })
    .await?
}
async fn save(provider: Provider, config: &Path, tokens: &Tokens) -> Result<()> {
    credential(provider, config, Some(Some(serde_json::to_vec(tokens)?))).await?;
    Ok(())
}
pub(crate) async fn json(mut response: reqwest::Response) -> Result<Value> {
    let status = response.status();
    ensure!(
        status.is_success(),
        "The music service returned HTTP {}. Try later or reconnect the account.",
        status.as_u16()
    );
    ensure!(
        response
            .content_length()
            .is_none_or(|n| n <= 8 * 1024 * 1024),
        "The library response is too large"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow::anyhow!("The music service connection was interrupted"))?
    {
        ensure!(
            bytes.len().saturating_add(chunk.len()) <= 8 * 1024 * 1024,
            "The library response is too large"
        );
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("The music service returned an unexpected response"))
}
async fn get(
    http: &Http,
    url: &str,
    token: Option<&str>,
    query: &[(&str, String)],
) -> Result<Value> {
    let mut request = http
        .client()
        .map_err(anyhow::Error::msg)?
        .get(url)
        .query(query)
        .timeout(Duration::from_secs(20));
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    json(
        request
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Cannot reach the music library service"))?,
    )
    .await
}
pub fn juice_song(song: &Value) -> Option<LocalTrack> {
    let id = song["id"].as_str()?;
    if !uuid(id) {
        return None;
    }
    let title = song["title"].as_str()?.to_owned();
    let image = juice_art(song["cover"].as_str());
    let duration = song["duration"]
        .as_f64()
        .filter(|s| s.is_finite() && *s >= 0.0)
        .unwrap_or_else(|| {
            song["length"]
                .as_str()
                .and_then(|s| {
                    let (m, s) = s.split_once(':')?;
                    Some(m.parse::<f64>().ok()? * 60.0 + s.parse::<f64>().ok()?)
                })
                .unwrap_or_default()
        });
    Some(LocalTrack {
        uri: format!("vibefy:juicevault:{id}"),
        name: title.clone(),
        title,
        artists: vec![ArtistRef {
            name: song["artist"].as_str().unwrap_or("Juice WRLD").into(),
            ..Default::default()
        }],
        album: song["album"].as_str().unwrap_or("JuiceVault").into(),
        duration_ms: (duration * 1000.0).min(u32::MAX as f64) as u32,
        cover_url: image.clone(),
        art_url: image.clone(),
        art_small_url: image,
        ..Default::default()
    })
}
pub fn uuid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn juice_art(path: Option<&str>) -> Option<String> {
    let path = path?;
    let url = if path.starts_with("/cdn/") {
        format!("https://api.juicevault.xyz{path}")
    } else {
        path.into()
    };
    crate::addons::safe_art_url(&url).then_some(url)
}
async fn juice_playlists(http: &Http, username: &str) -> Result<(String, Vec<LibraryPlaylist>)> {
    ensure!(
        !username.is_empty() && username.len() <= 100,
        "Enter your JuiceVault username to read its public playlists"
    );
    let encoded = urlencoding::encode(username);
    let value = get(
        http,
        &format!("https://api.juicevault.xyz/user/playlists/user/{encoded}"),
        None,
        &[],
    )
    .await?;
    ensure!(
        value["private"] != true,
        "This profile is private. JuiceVault's public API cannot read its playlists"
    );
    let playlists = value["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let id = p["_id"].as_str()?;
            if !uuid(id) {
                return None;
            }
            Some(LibraryPlaylist {
                id: id.into(),
                title: p["name"].as_str().unwrap_or("JuiceVault playlist").into(),
                image: juice_art(p["coverImage"].as_str()),
            })
        })
        .collect();
    Ok((username.into(), playlists))
}
async fn import_juice(
    http: &Http,
    playlist: &LibraryPlaylist,
    token: Option<&str>,
) -> Result<Collection> {
    ensure!(uuid(&playlist.id), "Invalid JuiceVault playlist identity");
    let value = get(
        http,
        &format!("https://api.juicevault.xyz/user/playlists/{}", playlist.id),
        token,
        &[],
    )
    .await?;
    let p = &value["data"];
    let tracks = p["songs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| juice_song(&s["song"]))
        .collect();
    Ok(Collection {
        title: playlist.title.clone(),
        source: format!("https://api.juicevault.xyz/user/playlists/{}", playlist.id),
        tracks,
        image: playlist.image.clone(),
        owner: Some("JuiceVault".into()),
    })
}
pub async fn execute(
    provider: Provider,
    action: Action,
    _http: &Http,
    config: &Path,
    username: &str,
    proxy: &crate::settings::ProxyConfig,
    session_file: Option<std::path::PathBuf>,
) -> Result<Update> {
    if matches!(action, Action::Disconnect) {
        if provider == Provider::YouTubeMusic {
            if let Some(bytes) = credential(provider, config, None).await? {
                if let Ok(mut session) =
                    serde_json::from_slice::<crate::music_session::Session>(&bytes)
                {
                    session.bind(config, proxy)?;
                    session.disconnect_browser().await?;
                }
            }
        }
        credential(provider, config, Some(None)).await?;
        return Ok(Update {
            removed_source: None,
            name: String::new(),
            playlists: Vec::new(),
            collection: None,
        });
    }
    if provider == Provider::YouTubeMusic {
        let mut session = if matches!(action, Action::Connect) {
            if let Some(file) = session_file {
                crate::music_session::Session::read(&file).await?
            } else {
                crate::music_session::Session::connect_browser(config, proxy).await?
            }
        } else {
            let bytes = credential(provider, config, None)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Connect YouTube Music first"))?;
            serde_json::from_slice::<crate::music_session::Session>(&bytes)
                .map_err(|_| anyhow::anyhow!("Reconnect the YouTube Music session"))?
        };
        session.bind(config, proxy)?;
        let connecting = matches!(action, Action::Connect);
        let result = async {
            let name = session.display_name(proxy).await?;
            let mut playlists = session.playlists(proxy).await?;
            let removed_source = if let Action::Delete(id) = &action {
                let raw = id.strip_prefix("VL").unwrap_or(id);
                session.delete(proxy, raw).await?;
                playlists.retain(|p| p.id.strip_prefix("VL").unwrap_or(&p.id) != raw);
                Some(format!("https://www.youtube.com/playlist?list={raw}"))
            } else {
                None
            };
            let collection = if let Action::Import(playlist) = action {
                Some(session.import(proxy, &playlist).await?)
            } else {
                None
            };
            if connecting {
                credential(provider, config, Some(Some(serde_json::to_vec(&session)?))).await?;
            }
            Ok(Update {
                removed_source,
                name,
                playlists,
                collection,
            })
        }
        .await;
        if connecting && result.is_err() {
            let _ = session.disconnect_browser().await;
        }
        return result;
    }
    // Authentication bodies and bearer credentials must never follow a redirect.
    ensure!(
        !matches!(action, Action::Delete(_)),
        "JuiceVault does not document playlist deletion through its public API. Manage this playlist on JuiceVault."
    );
    let secure =
        Http::new(crate::http::build_collection_client(proxy).map_err(anyhow::Error::msg)?);
    if matches!(action, Action::PublicProfile) {
        let (name, playlists) = juice_playlists(&secure, username.trim()).await?;
        let public = Tokens {
            access: String::new(),
            refresh: String::new(),
            username: name.clone(),
        };
        save(provider, config, &public).await?;
        return Ok(Update {
            removed_source: None,
            name,
            playlists,
            collection: None,
        });
    }
    let mut tokens = match &action {
        Action::Login(password) => {
            juice_password_login(&secure, username.trim(), &password.0).await?
        }
        Action::Connect => juice_login(&secure).await?,
        _ => {
            let bytes = credential(provider, config, None).await?.ok_or_else(|| {
                anyhow::anyhow!("Connect JuiceVault or load a public profile first")
            })?;
            serde_json::from_slice::<Tokens>(&bytes)
                .map_err(|_| anyhow::anyhow!("Reconnect JuiceVault"))?
        }
    };
    let (name, playlists) = if tokens.access.is_empty() {
        juice_playlists(&secure, &tokens.username).await?
    } else {
        let profile = juice_authed(&secure, config, &mut tokens, "/user/auth/me").await?;
        ensure!(
            profile["data"]["isVerified"] != false,
            "Verify your email on JuiceVault before connecting."
        );
        let name = profile["data"]["username"]
            .as_str()
            .unwrap_or("JuiceVault")
            .to_owned();
        tokens.username = name.clone();
        let data = juice_authed(&secure, config, &mut tokens, "/user/playlists").await?;
        (name, parse_juice_playlists(&data))
    };
    let collection = if let Action::Import(p) = &action {
        Some(
            import_juice(
                &secure,
                p,
                (!tokens.access.is_empty()).then_some(tokens.access.as_str()),
            )
            .await?,
        )
    } else {
        None
    };
    save(provider, config, &tokens).await?;
    Ok(Update {
        removed_source: None,
        name,
        playlists,
        collection,
    })
}

pub fn parse_juice_playlists(value: &Value) -> Vec<LibraryPlaylist> {
    let mut seen = std::collections::HashSet::new();
    value["data"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(value["collaborated"].as_array().into_iter().flatten())
        .filter_map(|p| {
            let id = p["_id"].as_str()?;
            if !uuid(id) || !seen.insert(id) {
                return None;
            }
            Some(LibraryPlaylist {
                id: id.into(),
                title: p["name"].as_str().unwrap_or("JuiceVault playlist").into(),
                image: juice_art(p["coverImage"].as_str()),
            })
        })
        .collect()
}
async fn juice_post(http: &Http, path: &str, body: Value, grant: Option<&str>) -> Result<Value> {
    let mut request = http
        .client()
        .map_err(anyhow::Error::msg)?
        .post(format!("https://api.juicevault.xyz{path}"))
        .json(&body)
        .timeout(Duration::from_secs(15));
    if let Some(grant) = grant {
        request = request.header("X-CA", grant);
    }
    let response = request
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("Cannot reach JuiceVault sign-in"))?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        bail!("JuiceVault rejected the login. Check your username and password.");
    }
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        bail!("JuiceVault requires browser verification. Use Sign in through browser.");
    }
    json(response).await
}
async fn juice_password_login(http: &Http, username: &str, password: &str) -> Result<Tokens> {
    ensure!(
        !username.is_empty()
            && username.len() <= 200
            && !password.is_empty()
            && password.len() <= 1024,
        "Enter your JuiceVault username and password"
    );
    let ticket = juice_post(http, "/user/auth/token", serde_json::json!({}), None).await?;
    let token = ticket["data"]["token"]
        .as_str()
        .filter(|s| s.len() < 16384)
        .ok_or_else(|| {
            anyhow::anyhow!("Use browser sign-in to complete JuiceVault verification")
        })?;
    let payload = token
        .split('.')
        .nth(1)
        .and_then(|v| URL_SAFE_NO_PAD.decode(v).ok())
        .and_then(|v| serde_json::from_slice::<Value>(&v).ok());
    let grant = payload
        .as_ref()
        .and_then(|p| p["ca"].as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("Use browser sign-in to complete JuiceVault verification")
        })?;
    let result = juice_post(
        http,
        "/user/auth/login",
        serde_json::json!({"login":username,"password":password,"loginToken":token}),
        Some(grant),
    )
    .await?;
    token_fields(&result["data"], "accessToken", "refreshToken")
}
async fn juice_authed(
    http: &Http,
    config: &Path,
    tokens: &mut Tokens,
    path: &str,
) -> Result<Value> {
    let request = || {
        http.client().map(|c| {
            c.get(format!("https://api.juicevault.xyz{path}"))
                .bearer_auth(&tokens.access)
                .timeout(Duration::from_secs(15))
        })
    };
    let response = request()
        .map_err(anyhow::Error::msg)?
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("Cannot reach JuiceVault"))?;
    if response.status() != reqwest::StatusCode::UNAUTHORIZED {
        return json(response).await;
    }
    ensure!(
        !tokens.refresh.is_empty(),
        "Your JuiceVault session expired. Sign in again."
    );
    let refreshed = juice_post(
        http,
        "/user/auth/refresh",
        serde_json::json!({"refreshToken":tokens.refresh}),
        None,
    )
    .await?;
    let mut next = token_fields(&refreshed["data"], "accessToken", "refreshToken")?;
    if next.refresh.is_empty() {
        next.refresh = tokens.refresh.clone();
    }
    next.username = tokens.username.clone();
    *tokens = next;
    save(Provider::JuiceVault, config, tokens).await?;
    get(
        http,
        &format!("https://api.juicevault.xyz{path}"),
        Some(&tokens.access),
        &[],
    )
    .await
}

pub fn juice_title_score(query: &str, title: &str) -> f32 {
    let clean = |s: &str| {
        s.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let q = clean(query);
    let t = clean(title);
    if q.is_empty() {
        return 0.0;
    }
    if q == t {
        return 1.0;
    }
    if t.starts_with(&(q.clone() + " ")) {
        return 0.92;
    }
    let a: std::collections::BTreeSet<_> = q.split_whitespace().collect();
    let b: std::collections::BTreeSet<_> = t.split_whitespace().collect();
    (2.0 * a.intersection(&b).count() as f32 / (a.len() + b.len()) as f32)
        .max(juice_edit_similarity(&q, &t) * 0.9)
}
pub async fn juice_search(http: &Http, query: &str) -> Result<Collection> {
    let query = query.trim();
    ensure!(
        (2..=200).contains(&query.len()),
        "Enter a song title (2–200 characters)"
    );
    let data = get(
        http,
        "https://api.juicevault.xyz/music/search",
        None,
        &[("q", query.into())],
    )
    .await?;
    let mut tracks: Vec<_> = data["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(juice_song)
        .filter_map(|t| {
            let score = juice_title_score(query, &t.title);
            (score >= 0.70).then_some((score, t))
        })
        .collect();
    tracks.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut seen = std::collections::HashSet::new();
    Ok(Collection {
        title: format!("JuiceVault · {query}"),
        source: format!("juicevault-search:{query}"),
        tracks: tracks
            .into_iter()
            .map(|(_, t)| t)
            .filter(|t| seen.insert(t.uri.clone()))
            .collect(),
        image: None,
        owner: Some("JuiceVault".into()),
    })
}

fn token_fields(v: &Value, access: &str, refresh: &str) -> Result<Tokens> {
    let access = v[access]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("The service did not grant access"))?
        .to_owned();
    let refresh = v[refresh].as_str().unwrap_or_default().to_owned();
    Ok(Tokens {
        access,
        refresh,
        username: String::new(),
    })
}

pub async fn has_saved(provider: Provider, config: &Path) -> bool {
    credential(provider, config, None)
        .await
        .ok()
        .flatten()
        .is_some()
}
async fn juice_login(http: &Http) -> Result<Tokens> {
    let verifier = random_token();
    let mut url = reqwest::Url::parse("https://juicevault.xyz/connect/spotify")?;
    url.query_pairs_mut()
        .append_pair("challenge", &challenge(&verifier))
        .append_pair("client", "vibefy");
    crate::opener::open(url.as_str())?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let response = http
            .client()
            .map_err(anyhow::Error::msg)?
            .post("https://api.juicevault.xyz/user/auth/handoff")
            .timeout(Duration::from_secs(15))
            .json(&serde_json::json!({"verifier":verifier}))
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Cannot reach JuiceVault sign-in"))?;
        if response.status() == reqwest::StatusCode::ACCEPTED {
            continue;
        }
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let seconds = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);
            tokio::time::sleep(Duration::from_secs(seconds)).await;
            continue;
        }
        let value = json(response).await?;
        return token_fields(&value["data"], "accessToken", "refreshToken");
    }
    bail!("JuiceVault sign-in timed out. Try again.")
}

/// The session never leaves the native backend or the protected local store.
pub(crate) async fn youtube_session(
    config: &Path,
    proxy: &crate::settings::ProxyConfig,
) -> Result<crate::music_session::Session> {
    let bytes = credential(Provider::YouTubeMusic, config, None)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Connect YouTube Music first"))?;
    let mut session: crate::music_session::Session =
        serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("Reconnect YouTube"))?;
    session.bind(config, proxy)?;
    Ok(session)
}

fn juice_edit_similarity(a: &str, b: &str) -> f32 {
    let a: Vec<_> = a.chars().take(240).collect();
    let b: Vec<_> = b.chars().take(240).collect();
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j] + 1)
                .min(old + 1)
                .min(previous + usize::from(x != y));
            previous = old;
        }
    }
    1.0 - row[b.len()] as f32 / a.len().max(b.len()) as f32
}
