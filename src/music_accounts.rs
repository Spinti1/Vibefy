//! Optional library connections. Passwords stay in the provider's browser UI.
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
    pub busy: bool,
    pub connected: bool,
    pub name: String,
    pub message: Option<String>,
    pub playlists: Vec<LibraryPlaylist>,
    pub username: String,
}
#[derive(Clone, Debug)]
pub struct LibraryPlaylist {
    pub id: String,
    pub title: String,
    pub image: Option<String>,
}
#[derive(Clone, Debug)]
pub enum Action {
    Connect,
    Refresh,
    Disconnect,
    Import(LibraryPlaylist),
}
#[derive(Clone, Debug)]
pub struct Update {
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
async fn import_juice(http: &Http, playlist: &LibraryPlaylist) -> Result<Collection> {
    ensure!(uuid(&playlist.id), "Invalid JuiceVault playlist identity");
    let value = get(
        http,
        &format!("https://api.juicevault.xyz/user/playlists/{}", playlist.id),
        None,
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
    http: &Http,
    config: &Path,
    username: &str,
    proxy: &crate::settings::ProxyConfig,
    session_file: Option<std::path::PathBuf>,
) -> Result<Update> {
    if matches!(action, Action::Disconnect) {
        credential(provider, config, Some(None)).await?;
        return Ok(Update {
            name: String::new(),
            playlists: Vec::new(),
            collection: None,
        });
    }
    if provider == Provider::YouTubeMusic {
        let session = if matches!(action, Action::Connect) {
            let file = session_file.ok_or_else(|| anyhow::anyhow!("Session import cancelled"))?;
            crate::music_session::Session::read(&file).await?
        } else {
            let bytes = credential(provider, config, None)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Connect YouTube Music first"))?;
            serde_json::from_slice::<crate::music_session::Session>(&bytes)
                .map_err(|_| anyhow::anyhow!("Reconnect the YouTube Music session"))?
        };
        let connecting = matches!(action, Action::Connect);
        // Validate live access before persisting or showing Connected.
        let playlists = session.playlists(proxy).await?;
        let collection = if let Action::Import(playlist) = action {
            Some(session.import(proxy, &playlist).await?)
        } else {
            None
        };
        if connecting {
            credential(provider, config, Some(Some(serde_json::to_vec(&session)?))).await?;
        }
        return Ok(Update {
            name: "YouTube Music".into(),
            playlists,
            collection,
        });
    }
    let mut tokens = if matches!(action, Action::Connect) {
        let tokens = juice_login(http).await?;
        save(provider, config, &tokens).await?;
        tokens
    } else {
        let bytes = credential(provider, config, None)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Connect this music account first"))?;
        serde_json::from_slice::<Tokens>(&bytes)
            .map_err(|_| anyhow::anyhow!("Reconnect this music account"))?
    };
    let collection = if let Action::Import(playlist) = action {
        Some(import_juice(http, &playlist).await?)
    } else {
        None
    };
    let username = if username.trim().is_empty() {
        tokens.username.clone()
    } else {
        username.trim().to_owned()
    };
    if provider == Provider::JuiceVault && username.is_empty() {
        return Ok(Update {
            name: "JuiceVault session".into(),
            playlists: Vec::new(),
            collection,
        });
    }
    let (name, playlists) = juice_playlists(http, &username).await?;
    tokens.username = username;
    save(provider, config, &tokens).await?;
    // Retire access tokens from this task; only the protected store persists them.
    tokens.access.clear();
    Ok(Update {
        name,
        playlists,
        collection,
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
