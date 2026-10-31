//! Built-in, opt-in native adaptations. No downloaded JavaScript is executed.
use std::collections::BTreeSet;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::api::models::ArtistRef;
use crate::player::LocalTrack;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Id {
    Nyan,
    WaveLink,
    Oneko,
    CatJam,
    YouTube,
    Vivy,
    Backmusic,
    TrackPeek,
}

pub const ALL: [Id; 8] = [
    Id::YouTube,
    Id::WaveLink,
    Id::TrackPeek,
    Id::Vivy,
    Id::Backmusic,
    Id::Nyan,
    Id::CatJam,
    Id::Oneko,
];

impl Id {
    pub fn name(self) -> &'static str {
        match self {
            Self::Nyan => "Nyan Cat Progress Bar",
            Self::WaveLink => "WaveLink",
            Self::Oneko => "Oneko Cat",
            Self::CatJam => "Cat Jam Synced",
            Self::YouTube => "YouTube Playlist Import",
            Self::Vivy => "vivy",
            Self::Backmusic => "backmusic",
            Self::TrackPeek => "Track Peek",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Nyan => {
                "The original Nyan Cat frames animate along a rainbow progress bar at the track's estimated tempo."
            }
            Self::WaveLink => {
                "Native SoundCloud search and playback. Public, unprotected tracks only; account libraries and private tracks are not supported."
            }
            Self::Oneko => "A tiny cat follows your pointer and sleeps when idle.",
            Self::CatJam => {
                "The original Cat Jam animation changes speed with the track's locally estimated BPM."
            }
            Self::YouTube => {
                "Import a public YouTube playlist into a local Vibefy collection. Plays the exact videos, without changing your Spotify library."
            }
            Self::Vivy => {
                "Native ridgeline and drifting ribbons, driven by live audio. Inspired by vivy; does not use Spotify's advance track analysis."
            }
            Self::Backmusic => {
                "Native audio-reactive rings, spectrum, cover art and particles. An independent adaptation inspired by backmusic."
            }
            Self::TrackPeek => {
                "Peek at the previous and next tracks, and preview audio from a track's context menu without changing your queue. Inspired by Spotify Spice."
            }
        }
    }
    pub fn source(self) -> Option<&'static str> {
        match self {
            Self::Nyan => Some("https://github.com/NoahClemmensen/spicetify-nyan-bar-synced"),
            Self::WaveLink => Some("https://github.com/xoft122/wavelink"),
            Self::Oneko => Some("https://github.com/kyrie25/spicetify-oneko"),
            Self::CatJam => Some("https://github.com/BlafKing/spicetify-cat-jam-synced"),
            Self::Vivy => Some("https://github.com/wairence/vivy"),
            Self::Backmusic => Some("https://github.com/rolandyangg/backmusic_extension"),
            Self::YouTube => None,
            Self::TrackPeek => Some("https://github.com/grasonchan/spotify-spice"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    #[serde(deserialize_with = "read_ids")]
    pub installed: BTreeSet<Id>,
    #[serde(deserialize_with = "read_ids")]
    pub enabled: BTreeSet<Id>,
    pub visualizer: Option<Id>,
    pub visual_style: u8,
    pub intensity: f32,
    pub particles: bool,
    pub vivy: Option<VisualConfig>,
    pub backmusic: Option<VisualConfig>,
    pub cat_jam_scale: f32,
}

fn read_ids<'de, D: serde::Deserializer<'de>>(reader: D) -> Result<BTreeSet<Id>, D::Error> {
    let names = Vec::<String>::deserialize(reader)?;
    Ok(names
        .into_iter()
        .filter_map(|name| serde_json::from_value(serde_json::Value::String(name)).ok())
        .collect())
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            installed: BTreeSet::new(),
            enabled: BTreeSet::new(),
            visualizer: None,
            visual_style: 0,
            intensity: 0.0,
            particles: false,
            vivy: None,
            backmusic: None,
            cat_jam_scale: 0.9,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VisualConfig {
    pub style: u8,
    pub intensity: f32,
    pub particles: bool,
}
impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            style: 0,
            intensity: 1.0,
            particles: false,
        }
    }
}
impl Preferences {
    pub fn cat_scale(&self) -> f32 {
        if self.cat_jam_scale.is_finite() {
            self.cat_jam_scale.clamp(0.5, 2.0)
        } else {
            0.9
        }
    }
    pub fn visual(&self, id: Id) -> VisualConfig {
        let fallback = VisualConfig {
            style: self.visual_style.min(1),
            intensity: if self.intensity.is_finite() && self.intensity > 0.0 {
                self.intensity.clamp(0.2, 3.0)
            } else {
                1.0
            },
            particles: self.particles,
        };
        match id {
            Id::Backmusic => self.backmusic.unwrap_or(fallback),
            _ => self.vivy.unwrap_or(fallback),
        }
    }
    pub fn visual_mut(&mut self, id: Id) -> &mut VisualConfig {
        let old = self.visual(Id::Vivy);
        self.vivy.get_or_insert(old);
        self.backmusic.get_or_insert(old);
        if id == Id::Backmusic {
            self.backmusic.as_mut().unwrap()
        } else {
            self.vivy.as_mut().unwrap()
        }
    }
    pub fn installed_visualizers(&self) -> impl Iterator<Item = Id> + '_ {
        [Id::Vivy, Id::Backmusic]
            .into_iter()
            .filter(|id| self.installed.contains(id))
    }
    pub fn active(&self, id: Id) -> bool {
        self.installed.contains(&id) && self.enabled.contains(&id)
    }
    pub fn set(&mut self, id: Id, enabled: bool) {
        if enabled {
            self.installed.insert(id);
            self.enabled.insert(id);
        } else {
            self.enabled.remove(&id);
        }
    }
    pub fn uninstall(&mut self, id: Id) {
        self.enabled.remove(&id);
        self.installed.remove(&id);
        if self.visualizer == Some(id) {
            self.visualizer = None;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub title: String,
    pub source: String,
    pub tracks: Vec<LocalTrack>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
}

impl Collection {
    /// Older saved imports already contain video artwork; no reimport is needed.
    pub fn cover(&self) -> Option<&str> {
        self.image
            .as_deref()
            .filter(|url| safe_art_url(url))
            .or_else(|| {
                self.tracks.iter().find_map(|track| {
                    track
                        .cover_url
                        .as_deref()
                        .or(track.art_url.as_deref())
                        .or(track.art_small_url.as_deref())
                        .filter(|url| safe_art_url(url))
                })
            })
    }
}

#[derive(Clone, Debug)]
pub enum Request {
    YouTube(String),
    SoundCloud(String),
}

#[derive(Clone, Debug)]
pub enum Command {
    CancelFetch,
    Enable(Id, bool),
    Uninstall(Id),
    Fetch(Request),
    SaveCollection,
    SaveSoundCloud,
    DeleteCollection(usize),
    Play { tracks: Vec<LocalTrack>, index: u32 },
    ChooseVisualizer(Id),
    Fullscreen,
    Style(u8),
    Intensity(f32),
    Particles(bool),
    CatJamScale(f32),
    Host,
    Join(String),
    Leave,
}

/// Accept source identities, never arbitrary URLs or shell fragments.
pub fn direct_url(uri: &str) -> Option<String> {
    if let Some(id) = uri.strip_prefix("vibefy:youtube:") {
        return valid_video_id(id).then(|| format!("https://www.youtube.com/watch?v={id}"));
    }
    let text = uri.strip_prefix("vibefy:soundcloud:")?;
    let url = reqwest::Url::parse(text).ok()?;
    (url.scheme() == "https"
        && url.host_str() == Some("soundcloud.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.path().split('/').filter(|s| !s.is_empty()).count() == 2
        && url.query().is_none()
        && url.fragment().is_none())
    .then(|| url.to_string())
}
pub fn valid_video_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn safe_art_url(text: &str) -> bool {
    reqwest::Url::parse(text).is_ok_and(|url| {
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
            && matches!(
                url.host_str(),
                Some(
                    "i.scdn.co"
                        | "i.ytimg.com"
                        | "img.youtube.com"
                        | "i1.sndcdn.com"
                        | "lh3.googleusercontent.com"
                        | "yt3.ggpht.com"
                        | "yt3.googleusercontent.com"
                )
            )
    })
}

pub fn playlist_url(text: &str) -> Result<String> {
    let url = reqwest::Url::parse(text.trim()).context("Paste a YouTube playlist URL")?;
    ensure!(
        url.scheme() == "https"
            && matches!(
                url.host_str(),
                Some("www.youtube.com" | "youtube.com" | "music.youtube.com")
            )
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none(),
        "Use a youtube.com playlist URL"
    );
    let id = url
        .query_pairs()
        .find(|(k, _)| k == "list")
        .map(|(_, v)| v.into_owned())
        .context("This URL has no playlist ID")?;
    ensure!(
        (10..=100).contains(&id.len())
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
        "Invalid playlist ID"
    );
    Ok(format!("https://www.youtube.com/playlist?list={id}"))
}

pub fn parse_collection(
    value: &serde_json::Value,
    youtube: bool,
    source: String,
) -> Result<Collection> {
    let entries = value
        .get("entries")
        .and_then(|v| v.as_array())
        .context("The source returned no track list")?;
    let mut tracks = Vec::new();
    for entry in entries.iter().take(500) {
        let title = entry["title"].as_str().unwrap_or("");
        if title.is_empty()
            || matches!(title, "[Private video]" | "[Deleted video]")
            || entry["is_live"].as_bool() == Some(true)
            || entry["is_drm"].as_bool() == Some(true)
            || entry["has_drm"].as_bool() == Some(true)
            || matches!(
                entry["availability"].as_str(),
                Some("private" | "premium_only" | "subscriber_only" | "needs_auth")
            )
        {
            continue;
        }
        let uri = if youtube {
            let id = entry["id"].as_str().unwrap_or("");
            if !valid_video_id(id) {
                continue;
            }
            format!("vibefy:youtube:{id}")
        } else {
            format!(
                "vibefy:soundcloud:{}",
                entry["webpage_url"]
                    .as_str()
                    .or_else(|| entry["url"].as_str())
                    .unwrap_or("")
            )
        };
        if direct_url(&uri).is_none() {
            continue;
        }
        let artist = entry["artist"]
            .as_str()
            .or_else(|| entry["uploader"].as_str())
            .or_else(|| entry["channel"].as_str())
            .unwrap_or(if youtube { "YouTube" } else { "SoundCloud" });
        let duration = entry["duration"]
            .as_f64()
            .filter(|d| d.is_finite() && *d > 0.0)
            .unwrap_or(0.0);
        let cover = source_thumbnail(entry, youtube);
        tracks.push(LocalTrack {
            uri,
            name: title.into(),
            title: title.into(),
            duration_ms: (duration * 1000.0).min(u32::MAX as f64) as u32,
            artists: vec![ArtistRef {
                name: artist.into(),
                ..Default::default()
            }],
            cover_url: cover.clone(),
            art_url: cover.clone(),
            art_small_url: cover,
            album: if youtube { "YouTube" } else { "SoundCloud" }.into(),
            ..Default::default()
        });
    }
    ensure!(
        !tracks.is_empty(),
        "No public playable tracks were returned. Private, removed or restricted items cannot be imported."
    );
    Ok(Collection {
        title: value["title"]
            .as_str()
            .unwrap_or(if youtube {
                "YouTube playlist"
            } else {
                "SoundCloud results"
            })
            .chars()
            .take(200)
            .collect(),
        source,
        tracks,
        image: value["thumbnail"]
            .as_str()
            .or_else(|| {
                value["thumbnails"]
                    .as_array()?
                    .iter()
                    .rev()
                    .find_map(|image| image["url"].as_str().filter(|url| safe_art_url(url)))
            })
            .filter(|url| safe_art_url(url))
            .map(str::to_owned),
        owner: value["uploader"]
            .as_str()
            .or_else(|| value["channel"].as_str())
            .filter(|name| !name.trim().is_empty())
            .map(|name| name.chars().take(200).collect()),
    })
}

/// Prefer the uploader's main thumbnail, never yt-dlp's numbered video frames.
pub fn source_thumbnail(entry: &serde_json::Value, youtube: bool) -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(url) = entry["thumbnail"].as_str().filter(|url| safe_art_url(url)) {
        candidates.push((0u64, url));
    }
    if let Some(images) = entry["thumbnails"].as_array() {
        for image in images {
            if let Some(url) = image["url"].as_str().filter(|url| safe_art_url(url)) {
                candidates.push((
                    image["width"].as_u64().unwrap_or(0) * image["height"].as_u64().unwrap_or(0),
                    url,
                ));
            }
        }
    }
    candidates.sort_by_key(|(size, url)| {
        (
            crate::music::is_album_art(url),
            url.contains("default."),
            *size,
        )
    });
    let (_, url) = candidates.last()?;
    if youtube {
        let path = reqwest::Url::parse(url).ok()?.path().to_string();
        if path.rsplit('/').next().is_some_and(|name| {
            matches!(
                name,
                "0.jpg"
                    | "1.jpg"
                    | "2.jpg"
                    | "3.jpg"
                    | "mq1.jpg"
                    | "mq2.jpg"
                    | "mq3.jpg"
                    | "hq1.jpg"
                    | "hq2.jpg"
                    | "hq3.jpg"
            )
        }) {
            let id = entry["id"].as_str()?;
            return valid_video_id(id)
                .then(|| format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"));
        }
    }
    Some(crate::music::artwork_url(url))
}

pub async fn fetch(request: Request, proxy: crate::settings::ProxyConfig) -> Result<Collection> {
    let binary = crate::player::find_yt_dlp_binary().context("yt-dlp is not installed")?;
    let (target, youtube) = match request {
        Request::YouTube(url) => (playlist_url(&url)?, true),
        Request::SoundCloud(query) => {
            let query = query.trim();
            ensure!(
                !query.is_empty() && query.len() <= 200,
                "Enter a SoundCloud search (up to 200 characters)"
            );
            (format!("scsearch20:{query}"), false)
        }
    };
    let mut command = tokio::process::Command::new(binary);
    command.args(crate::player::youtube_proxy_args(&proxy));
    command.args([
        "--ignore-config",
        "--no-warnings",
        "--skip-download",
        "--flat-playlist",
        "--ignore-errors",
        "--playlist-end",
        "500",
        "--socket-timeout",
        "10",
        "--retries",
        "1",
        "-J",
        "--",
        &target,
    ]);
    command.kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = tokio::time::timeout(Duration::from_secs(90), command.output())
        .await
        .context("The source took too long to respond")??;
    ensure!(
        output.status.success(),
        "The source could not be loaded. Check the URL, availability and connection."
    );
    ensure!(
        output.stdout.len() <= 20_000_000,
        "The source returned too much data"
    );
    parse_collection(
        &serde_json::from_slice(&output.stdout).context("Invalid source response")?,
        youtube,
        target,
    )
}

pub fn source_error(stderr: &[u8]) -> &'static str {
    let text = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    if text.contains("drm") {
        "This track is DRM-protected and cannot be played in Vibefy. Choose a public, unprotected upload."
    } else if text.contains("geo") || text.contains("country") || text.contains("region") {
        "This track is not available in your region."
    } else {
        "This source is unavailable or restricted. Try another public, unprotected track."
    }
}

#[derive(Default)]
pub struct Runtime {
    pub artwork_checked: BTreeSet<String>,
    pub artwork_active: Option<String>,
    pub soundcloud_result: Option<Collection>,
    pub soundcloud_message: Option<String>,
    pub soundcloud_focus_requested: bool,
    pub fetching_soundcloud: bool,
    pub tempo_recording: Option<String>,
    pub tempo_bpm: Option<f32>,
    pub tempo_pending: bool,
    pub tempo_cache: std::collections::HashMap<String, Option<f32>>,
    pub filter: String,
    pub collections: Vec<Collection>,
    pub collections_revision: u64,
    pub analyser: crate::vis::WideAnalyser,
    pub ridges: std::collections::VecDeque<Vec<f32>>,
    pub audio_time: f64,
    pub bands: Vec<f32>,
    pub beat: BeatDetector,
    pub active_tracks: Vec<LocalTrack>,
    pub active_collection: Option<String>,
    pub query: String,
    pub youtube_url: String,
    pub busy: bool,
    pub request: u64,
    pub result: Option<Collection>,
    pub message: Option<String>,
    pub fullscreen: bool,
    pub was_fullscreen: bool,
    pub jam_message: Option<String>,
    pub jam_invite: String,
    pub jam_address: String,
    pub jam: Option<crate::jam::Session>,
    pub pending_sync: Option<(crate::jam::Snapshot, std::time::Instant)>,
    pub last_sync: Option<std::time::Instant>,
    pub last_play_attempt: Option<std::time::Instant>,
}

pub fn load_collections(path: &std::path::Path) -> Vec<Collection> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    if bytes.len() > 20_000_000 {
        return Vec::new();
    }
    let Ok(collections) = serde_json::from_slice::<Vec<Collection>>(&bytes) else {
        return Vec::new();
    };
    collections
        .into_iter()
        .take(30)
        .filter(|c| c.tracks.len() <= 500 && c.tracks.iter().all(|t| direct_url(&t.uri).is_some()))
        .collect()
}

/// Adaptive live onset detector; it does not claim a fixed BPM for every song.
#[derive(Default)]
pub struct BeatDetector {
    pub baseline: f32,
    pub pulse: f32,
    last: f64,
}
impl BeatDetector {
    pub fn step(&mut self, energy: f32, time: f64, elapsed: f32) {
        let energy = if energy.is_finite() {
            energy.max(0.0)
        } else {
            0.0
        };
        self.pulse *= (-elapsed.max(0.0) * 8.0).exp();
        if energy > 0.015 && energy > self.baseline.max(0.008) * 1.4 && time - self.last > 0.22 {
            self.pulse = 1.0;
            self.last = time;
        }
        let blend = 1.0 - (-elapsed.clamp(0.0, 0.2) * 2.0).exp();
        self.baseline += blend * (energy - self.baseline);
    }
}
