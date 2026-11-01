//! YouTube playback engine for Spotifast fork.
//!
//! Replaces librespot with a YouTube audio streaming player.
//! Uses `yt-dlp` to query and resolve audio stream URLs, fetches the stream,
//! decodes with Symphonia, and outputs via the Rodio / fastframe-audio sink.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::io::Cursor;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::api::models::ArtistRef;
use crate::recording_match::{Recording, Reference, SourceKind};
use crate::sink::{
    AudioControl, AudioPacket, Converter, NUM_CHANNELS, RodioSink, SAMPLE_RATE, Sink, VolumeGetter,
};
use crate::vis::AudioTap;

/// Finds the `yt-dlp` binary in the application folder, PATH, or common Windows directories.
pub fn find_yt_dlp_binary() -> Option<PathBuf> {
    // 0. Check alongside the running executable first (portable mode)
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let candidate = parent.join("yt-dlp.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
            let candidate_no_ext = parent.join("yt-dlp");
            if candidate_no_ext.is_file() {
                return Some(candidate_no_ext);
            }
        }
    }

    // 1. Try running yt-dlp from PATH
    let mut cmd = std::process::Command::new("yt-dlp");
    cmd.arg("--version");
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    if let Ok(output) = cmd.output() {
        if output.status.success() {
            return Some(PathBuf::from("yt-dlp"));
        }
    }

    // 2. On Windows, check common Python Scripts and AppData directories
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let local_path = PathBuf::from(&local_app_data);
            // Search in LOCALAPPDATA\Python\*\Scripts\yt-dlp.exe
            let python_dir = local_path.join("Python");
            if let Ok(entries) = std::fs::read_dir(&python_dir) {
                for entry in entries.flatten() {
                    let candidate = entry.path().join("Scripts").join("yt-dlp.exe");
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
            // Search in LOCALAPPDATA\Programs\Python\Python*\Scripts\yt-dlp.exe
            let programs_python = local_path.join("Programs").join("Python");
            if let Ok(entries) = std::fs::read_dir(&programs_python) {
                for entry in entries.flatten() {
                    let candidate = entry.path().join("Scripts").join("yt-dlp.exe");
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let scoop_candidate = PathBuf::from(&user_profile)
                .join("scoop")
                .join("shims")
                .join("yt-dlp.exe");
            if scoop_candidate.is_file() {
                return Some(scoop_candidate);
            }
            let pip_user_candidate = PathBuf::from(&user_profile)
                .join("AppData")
                .join("Roaming")
                .join("Python")
                .join("Scripts")
                .join("yt-dlp.exe");
            if pip_user_candidate.is_file() {
                return Some(pip_user_candidate);
            }
        }
    }

    None
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub device_name: String,
    pub bitrate_kbps: u16,
    pub normalisation: bool,
    pub autoplay: bool,
    pub gapless: bool,
    pub audio_device: Option<String>,
    pub initial_volume: u16,
    pub volume_dir: PathBuf,
    pub audio_cache_dir: Option<PathBuf>,
    pub audio_cache_limit: Option<u64>,
    pub buffer_ms: u32,
    pub tap: Arc<AudioTap>,
    pub eq: crate::eq::SharedEq,
    pub proxy: crate::settings::ProxyConfig,
    pub local_files: Option<PathBuf>,
}

impl EngineConfig {
    pub fn device_id(&self) -> String {
        hex(&Sha1::digest(self.device_name.as_bytes()))
    }

    pub fn open_cache(&self) -> Result<()> {
        Ok(())
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Playback {
    #[default]
    Stopped,
    Loading,
    Playing,
    Paused,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum RepeatMode {
    #[default]
    Off,
    Context,
    Track,
}

impl RepeatMode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Context,
            Self::Context => Self::Track,
            Self::Track => Self::Off,
        }
    }

    pub fn api_name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Context => "context",
            Self::Track => "track",
        }
    }

    pub fn from_api(name: &str) -> Self {
        match name {
            "track" => Self::Track,
            "context" => Self::Context,
            _ => Self::Off,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTrack {
    pub uri: String,
    pub name: String,
    pub title: String,
    pub album: String,
    pub album_name: String,
    pub artists: Vec<ArtistRef>,
    pub duration_ms: u32,
    pub cover_url: Option<String>,
    pub art_url: Option<String>,
    pub art_small_url: Option<String>,
    pub is_episode: bool,
}

impl LocalTrack {
    pub fn has_recording_metadata(&self) -> bool {
        if crate::addons::direct_url(&self.uri).is_some() {
            return !self.title.trim().is_empty();
        }
        !self.title.trim().is_empty()
            && self
                .artists
                .iter()
                .any(|artist| !artist.name.trim().is_empty() && artist.name != "Unknown Artist")
    }

    pub fn artist_names(&self) -> String {
        crate::api::models::join_names(self.artists.iter().map(|a| a.name.as_str()))
    }

    pub fn to_playable_item(&self) -> crate::api::models::PlayableItem {
        let images = if let Some(url) = self.cover_url.as_ref().or(self.art_url.as_ref()) {
            vec![crate::api::models::Image {
                url: url.clone(),
                width: Some(300),
                height: Some(300),
            }]
        } else if let Some(url) = &self.art_small_url {
            vec![crate::api::models::Image {
                url: url.clone(),
                width: Some(64),
                height: Some(64),
            }]
        } else {
            Vec::new()
        };

        let album_name = if !self.album.is_empty() {
            self.album.clone()
        } else if !self.album_name.is_empty() {
            self.album_name.clone()
        } else {
            "YouTube Audio".to_string()
        };

        let album = crate::api::models::Album {
            name: album_name,
            images,
            ..Default::default()
        };

        let raw_id = extract_spotify_id(&self.uri);
        let id = if raw_id.is_empty() || self.uri.starts_with("vibefy:") {
            None
        } else {
            Some(raw_id)
        };
        let track_name = if !self.name.is_empty() {
            self.name.clone()
        } else if !self.title.is_empty() {
            self.title.clone()
        } else {
            "Track".to_string()
        };

        crate::api::models::PlayableItem::Track(crate::api::models::Track {
            id,
            name: track_name,
            uri: self.uri.clone(),
            duration_ms: self.duration_ms,
            artists: self.artists.clone(),
            album: Some(album),
            is_playable: Some(true),
            ..Default::default()
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct LocalState {
    pub source: Option<PlaybackSource>,
    pub playback: Playback,
    pub track: Option<LocalTrack>,
    pub position_ms: u32,
    pub position_at: Option<Instant>,
    pub volume: u16,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub connected: bool,
    pub username: String,
    pub error: Option<String>,
    pub queue: Vec<String>,
    pub queue_tracks: Vec<LocalTrack>,
    pub queue_index: usize,
    pub loading: bool,
    pub track_sequence: u64,
    pub seek_sequence: u64,
}

#[derive(Clone, Debug)]
pub struct PlaybackSource {
    pub youtube_id: String,
    pub title: String,
    pub duration_ms: u32,
}

impl LocalState {
    pub fn is_active(&self) -> bool {
        self.connected && !matches!(self.playback, Playback::Stopped)
    }

    pub fn position_now(&self) -> u32 {
        let Some(at) = self.position_at else {
            return self.position_ms;
        };
        let duration = self
            .source
            .as_ref()
            .map(|source| source.duration_ms)
            .filter(|duration| *duration > 0)
            .unwrap_or_else(|| self.track.as_ref().map_or(0, |t| t.duration_ms));
        let elapsed = at.elapsed().as_millis() as u32;
        if duration > 0 {
            (self.position_ms + elapsed).min(duration)
        } else {
            self.position_ms + elapsed
        }
    }

    pub fn interrupted(&self) -> Option<Interrupted> {
        self.track.as_ref().map(|track| Interrupted {
            track: track.clone(),
            position_ms: self.position_now(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Interrupted {
    pub track: LocalTrack,
    pub position_ms: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadSpec {
    pub play: bool,
    pub autoplay: bool,
    pub position_ms: u32,
    pub uris: Vec<String>,
    pub offset_uri: Option<String>,
    pub offset_index: Option<u32>,
    pub context_uri: Option<String>,
    pub shuffle: Option<bool>,
    pub repeat: Option<RepeatMode>,
    pub track: Option<LocalTrack>,
    pub queue_tracks: Vec<LocalTrack>,
}

#[derive(Clone, Debug)]
pub enum PlaybackResume {
    Context { uri: String, position_ms: u32 },
    Track { uri: String, position_ms: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerCommand {
    UpdateMetadata(LocalTrack),
    MetadataFailed(String),
    Toggle,
    Next,
    Previous,
    Seek(u32),
    Volume(u16),
    VolumePreview(u16),
    Shuffle(bool),
    Repeat(RepeatMode),
    ClearQueue,
    AddToQueue(String),
    SkipTo(usize),
    SkipToUri(String),
    Transfer,
    Load(LoadSpec),
}

#[derive(Clone, Debug)]
pub enum EngineEvent {
    State(LocalState),
    AudioDevice { name: String, id: String },
    SessionEnded,
}

pub type Notify = Arc<dyn Fn(EngineEvent) + Send + Sync>;

#[derive(Clone)]
pub struct KeptPlayer {
    pub state: Arc<Mutex<LocalState>>,
}

impl KeptPlayer {
    pub fn stop(&self) {}
    pub fn toggle(&self) -> bool {
        false
    }
    pub fn has_song(&self) -> bool {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.track.clone())
            .is_some()
    }
}

#[derive(Clone)]
pub struct Heard {
    state: Arc<Mutex<LocalState>>,
}

impl Heard {
    pub fn level(&self) -> u16 {
        self.state.lock().unwrap_or_else(|p| p.into_inner()).volume
    }
}

enum AudioWorkerCmd {
    Play {
        bytes: Vec<u8>,
        start_pos_ms: u32,
        generation: u64,
    },
    Pause,
    Resume,
    Seek(u32),
    Stop,
}

enum WorkerEvent {
    TrackEnded { generation: u64 },
}

#[derive(Clone, Debug, Default)]
pub struct QueueState {
    pub tracks: Vec<LocalTrack>,
    pub current_index: usize,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub manual_queue_count: usize,
}

/// Hydrate placeholders without reordering the queue or resuming a paused or
/// superseded request. Returns a track only when the current load was waiting.
pub fn receive_track_metadata(
    state: &mut LocalState,
    queue: &mut QueueState,
    track: &LocalTrack,
) -> Option<LocalTrack> {
    if !track.has_recording_metadata() {
        return None;
    }
    for entry in queue.tracks.iter_mut().chain(state.queue_tracks.iter_mut()) {
        if entry.uri == track.uri && !entry.has_recording_metadata() {
            *entry = track.clone();
        }
    }
    let current = state.track.as_mut()?;
    if current.uri != track.uri || current.has_recording_metadata() {
        return None;
    }
    *current = track.clone();
    (state.playback == Playback::Loading).then(|| track.clone())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedMatch {
    pub youtube_id: String,
    pub title: String,
    pub duration_secs: u32,
    #[serde(default)]
    pub matcher_version: u8,
    #[serde(default)]
    pub reference_key: String,
    #[serde(default)]
    pub source_kind: Option<SourceKind>,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub channel_id: String,
}

impl CachedMatch {
    pub fn matches_track(&self, track: &LocalTrack) -> bool {
        (3..=crate::audio_match::MATCHER_VERSION).contains(&self.matcher_version)
            && self.source_kind.is_some()
            && (!self.channel_id.is_empty() || self.source_kind == Some(SourceKind::MetadataMatch))
            && self.reference_key == recording_reference_key(track)
            && crate::recording_match::duration_compatible(
                track.duration_ms / 1000,
                self.duration_secs,
                &self.title,
                matches!(
                    self.source_kind,
                    Some(SourceKind::VerifiedArtist | SourceKind::VerifiedPublisher)
                ),
            )
    }
}

fn recording_reference(track: &LocalTrack) -> Reference {
    Reference {
        title: track.title.clone(),
        artists: track
            .artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect(),
        album: if track.album.is_empty() {
            track.album_name.clone()
        } else {
            track.album.clone()
        },
        duration_secs: track.duration_ms / 1000,
    }
}

pub fn recording_reference_key(track: &LocalTrack) -> String {
    let reference = recording_reference(track);
    let bytes = serde_json::to_vec(&(
        reference.title,
        reference.artists,
        reference.album,
        track.duration_ms,
    ))
    .unwrap_or_default();
    hex(&Sha1::digest(&bytes))
}

#[derive(Clone)]
pub struct MatchCache {
    json_path: PathBuf,
    audio_dir: PathBuf,
    entries: Arc<Mutex<HashMap<String, CachedMatch>>>,
    resolving: Arc<Mutex<HashMap<String, std::sync::Weak<tokio::sync::Mutex<()>>>>>,
    audio_writes: Arc<Mutex<()>>,
    compacting: Arc<Mutex<std::collections::HashSet<String>>>,
    compaction_worker: Arc<tokio::sync::Semaphore>,
}

impl MatchCache {
    pub fn new(base_dir: &Path) -> Self {
        let json_path = base_dir.join("youtube_cache.json");
        let audio_dir = base_dir.join("youtube_audio");
        let _ = std::fs::create_dir_all(&audio_dir);

        let entries = if json_path.exists() {
            match std::fs::read_to_string(&json_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

        Self {
            json_path,
            audio_dir,
            entries: Arc::new(Mutex::new(entries)),
            resolving: Default::default(),
            audio_writes: Default::default(),
            compacting: Default::default(),
            compaction_worker: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }

    pub fn get_video_id(&self, spotify_id: &str) -> Option<String> {
        let entries = self.entries.lock().ok()?;
        entries.get(spotify_id).map(|c| c.youtube_id.clone())
    }

    pub fn get_match(&self, spotify_id: &str) -> Option<CachedMatch> {
        let entries = self.entries.lock().ok()?;
        entries.get(spotify_id).cloned()
    }

    pub fn remove_match(&self, spotify_id: &str) {
        let mut entries = match self.entries.lock() {
            Ok(e) => e,
            Err(p) => p.into_inner(),
        };
        if let Some(old) = entries.remove(spotify_id) {
            let old_file = self.audio_dir.join(format!("{}.m4a", old.youtube_id));
            let _ = std::fs::remove_file(old_file);
            if let Ok(json) = serde_json::to_string_pretty(&*entries) {
                let _ = fastframe_file::write(&self.json_path, json);
            }
        }
    }

    pub fn get_cached_audio(&self, youtube_id: &str) -> Option<Vec<u8>> {
        let file_path = self.audio_dir.join(format!("{youtube_id}.m4a"));
        let bytes = std::fs::read(&file_path)
            .ok()
            .filter(|bytes| !bytes.is_empty())?;
        self.schedule_audio_compaction(youtube_id, &bytes);
        Some(bytes)
    }
    /// Speculation only needs file presence, not a full allocation/read.
    pub fn has_cached_audio(&self, youtube_id: &str) -> bool {
        std::fs::metadata(self.audio_dir.join(format!("{youtube_id}.m4a")))
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
    }

    fn resolution_lock(&self, track: &LocalTrack) -> Arc<tokio::sync::Mutex<()>> {
        let key = format!("{}:{}", track.uri, recording_reference_key(track));
        let mut resolving = self.resolving.lock().unwrap_or_else(|e| e.into_inner());
        resolving.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = resolving.get(&key).and_then(std::sync::Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(tokio::sync::Mutex::new(()));
        resolving.insert(key, Arc::downgrade(&lock));
        lock
    }

    pub fn save_match(&self, spotify_id: &str, youtube_id: &str, title: &str, duration_secs: u32) {
        self.store_match(
            spotify_id,
            CachedMatch {
                youtube_id: youtube_id.to_string(),
                title: title.to_string(),
                duration_secs,
                matcher_version: 0,
                reference_key: String::new(),
                source_kind: None,
                channel: String::new(),
                channel_id: String::new(),
            },
        );
    }

    fn save_verified_match(&self, track: &LocalTrack, recording: &Recording, source: SourceKind) {
        self.store_match(
            &extract_spotify_id(&track.uri),
            CachedMatch {
                youtube_id: recording.id.clone(),
                title: recording.title.clone().unwrap_or_default(),
                duration_secs: recording.duration.unwrap_or_default().round() as u32,
                matcher_version: crate::audio_match::MATCHER_VERSION,
                reference_key: recording_reference_key(track),
                source_kind: Some(source),
                channel: recording
                    .channel
                    .clone()
                    .or(recording.uploader.clone())
                    .unwrap_or_default(),
                channel_id: recording.channel_id.clone().unwrap_or_default(),
            },
        );
    }

    fn store_match(&self, spotify_id: &str, matched: CachedMatch) {
        let mut entries = match self.entries.lock() {
            Ok(e) => e,
            Err(p) => p.into_inner(),
        };
        if entries.get(spotify_id) == Some(&matched) {
            return;
        }
        entries.insert(spotify_id.to_string(), matched);

        if let Ok(json) = serde_json::to_string_pretty(&*entries) {
            let _ = fastframe_file::write(&self.json_path, json);
        }
    }

    pub fn save_audio(&self, youtube_id: &str, bytes: &[u8]) {
        let file_path = self.audio_dir.join(format!("{youtube_id}.m4a"));
        let _guard = self.audio_writes.lock().unwrap_or_else(|e| e.into_inner());
        let _ = fastframe_file::write(&file_path, bytes);
        self.schedule_audio_compaction(youtube_id, bytes);
    }

    fn schedule_audio_compaction(&self, key: &str, bytes: &[u8]) {
        if key.is_empty()
            || key.len() > 128
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || !crate::audio_only_mp4::contains_video(bytes)
        {
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        if !self
            .compacting
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.to_owned())
        {
            return;
        }
        let cache = self.clone();
        let key = key.to_owned();
        runtime.spawn(async move {
            // Playback receives the original bytes first. One background job
            // at a time avoids competing remux jobs during rapid track changes.
            tokio::time::sleep(Duration::from_secs(2)).await;
            let Ok(_permit) = Arc::clone(&cache.compaction_worker).acquire_owned().await else {
                return;
            };
            let worker = cache.clone();
            let worker_key = key.clone();
            let _ =
                tokio::task::spawn_blocking(move || worker.compact_cached_audio(&worker_key)).await;
            cache
                .compacting
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&key);
        });
    }

    /// Keep the playing buffer intact; atomically replace only an unchanged cache file.
    pub fn compact_cached_audio(&self, key: &str) -> Option<(usize, usize)> {
        if key.is_empty()
            || key.len() > 128
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return None;
        }
        let path = self.audio_dir.join(format!("{key}.m4a"));
        let before = std::fs::symlink_metadata(&path).ok()?;
        if !before.is_file() || before.len() > 128 * 1024 * 1024 {
            return None;
        }
        let modified = before.modified().ok()?;
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .ok()?
            .take(128 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() as u64 != before.len() {
            return None;
        }
        let audio = crate::audio_only_mp4::extract(&bytes).ok()??;
        let _guard = self.audio_writes.lock().unwrap_or_else(|e| e.into_inner());
        let current = std::fs::symlink_metadata(&path).ok()?;
        if !current.is_file()
            || current.len() != before.len()
            || current.modified().ok()? != modified
        {
            return None;
        }
        fastframe_file::write(&path, &audio).ok()?;
        log::info!(
            "Audio-only cache compaction: {} -> {} bytes",
            bytes.len(),
            audio.len()
        );
        Some((bytes.len(), audio.len()))
    }

    // TODO: Manual search override: Allow user to right-click track in UI and choose
    // "Search YouTube manually", or provide custom YouTube URL / ID to override this cache entry.
    pub fn override_match(
        &self,
        spotify_id: &str,
        youtube_id: &str,
        title: &str,
        duration_secs: u32,
    ) {
        self.save_match(spotify_id, youtube_id, title, duration_secs);
        let old_file = self.audio_dir.join(format!("{youtube_id}.m4a"));
        let _ = std::fs::remove_file(old_file);
    }
}

#[derive(Debug, Deserialize)]
struct YtFlatPlaylist {
    entries: Option<Vec<Recording>>,
}

fn extract_spotify_id(uri: &str) -> String {
    if let Some(id) = uri.strip_prefix("spotify:track:") {
        id.to_string()
    } else {
        uri.to_string()
    }
}

/// yt-dlp disables proxies with an explicitly empty --proxy value. It has no
/// --no-proxy switch. Keep the empty argument intact when spawning directly.
pub fn youtube_proxy_args(proxy: &crate::settings::ProxyConfig) -> Vec<String> {
    if let Some(proxy_url) = proxy.to_url_string() {
        vec!["--proxy".into(), proxy_url]
    } else if matches!(proxy, crate::settings::ProxyConfig::Off) {
        vec!["--proxy".into(), String::new()]
    } else {
        Vec::new()
    }
}

fn apply_yt_proxy(cmd: &mut tokio::process::Command, proxy: &crate::settings::ProxyConfig) {
    cmd.args(youtube_proxy_args(proxy));
}

async fn download_youtube_audio_by_url(
    yt_dlp: &Path,
    url: &str,
    proxy: &crate::settings::ProxyConfig,
) -> Result<Vec<u8>> {
    let mut cmd = tokio::process::Command::new(yt_dlp);
    apply_yt_proxy(&mut cmd, proxy);
    cmd.args([
        "--ignore-config",
        "--no-warnings",
        "--buffer-size",
        "64K",
        "-f",
        YOUTUBE_AUDIO_FORMAT,
        "--no-playlist",
        "-o",
        "-",
        url,
    ]);
    cmd.kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let output = tokio::time::timeout(Duration::from_secs(90), cmd.output())
        .await
        .context("YouTube audio download timed out")?
        .context("Cannot start yt-dlp for audio download")?;
    if !output.status.success() || output.stdout.is_empty() {
        anyhow::bail!(
            "YouTube audio download failed (yt-dlp {}). No standard-quality native audio may be available",
            output.status
        );
    }

    Ok(output.stdout)
}

/// Highest available native AAC/M4A audio. Availability wins over a bitrate
/// floor; a compatible MP4 audio stream is the last native-format fallback.
pub const YOUTUBE_AUDIO_FORMAT: &str = "ba[ext=m4a]/b[ext=mp4][acodec^=mp4a]";

async fn search_candidates_for_query(
    yt_dlp: &Path,
    query: &str,
    proxy: &crate::settings::ProxyConfig,
) -> Result<Vec<Recording>> {
    let mut cmd = tokio::process::Command::new(yt_dlp);
    apply_yt_proxy(&mut cmd, proxy);
    cmd.args([
        "--ignore-config",
        "--no-warnings",
        "--default-search",
        "ytsearch",
        "--flat-playlist",
        "-J",
        &format!("ytsearch20:{query}"),
    ]);
    cmd.kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let output = tokio::time::timeout(Duration::from_secs(25), cmd.output())
        .await
        .context("YouTube search timed out")?
        .context("Cannot start yt-dlp for YouTube search")?;
    if !output.status.success() {
        // Do not include stderr: proxy failures can contain credential URLs.
        anyhow::bail!("YouTube search failed (yt-dlp {})", output.status);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let playlist: YtFlatPlaylist = serde_json::from_str(&stdout)
        .context("yt-dlp returned an invalid YouTube search response")?;
    Ok(playlist.entries.unwrap_or_default())
}

async fn inspect_recording(
    yt_dlp: &Path,
    candidate: &Recording,
    proxy: &crate::settings::ProxyConfig,
) -> Result<(Recording, Vec<u8>)> {
    anyhow::ensure!(
        candidate.id.len() == 11
            && candidate
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-'),
        "Invalid YouTube video identity"
    );
    let mut cmd = tokio::process::Command::new(yt_dlp);
    apply_yt_proxy(&mut cmd, proxy);
    cmd.args([
        "--ignore-config",
        "--no-warnings",
        "--skip-download",
        "--no-playlist",
        "--socket-timeout",
        "10",
        "--retries",
        "1",
        "-J",
        &format!("https://www.youtube.com/watch?v={}", candidate.id),
    ]);
    cmd.kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    let output = tokio::time::timeout(Duration::from_secs(25), cmd.output())
        .await
        .context("YouTube recording verification timed out")?
        .context("Cannot start yt-dlp for recording verification")?;
    anyhow::ensure!(
        output.status.success(),
        "YouTube recording verification failed (yt-dlp {})",
        output.status
    );
    let mut detailed: Recording =
        serde_json::from_slice(&output.stdout).context("Invalid YouTube recording metadata")?;
    anyhow::ensure!(
        detailed.id == candidate.id,
        "YouTube recording identity changed"
    );
    // Watch pages may omit the verified badge that the search result supplies.
    // Reuse it only when the stable channel IDs agree.
    if detailed.channel_is_verified.is_none()
        && detailed.channel_id.is_some()
        && detailed.channel_id == candidate.channel_id
    {
        detailed.channel_is_verified = candidate.channel_is_verified;
    }
    Ok((detailed, output.stdout))
}

/// Reuse the inspected format list through stdin. Signed media URLs stay in
/// memory and yt-dlp can refresh expired URLs using its normal fallback.
async fn download_inspected_audio(
    yt_dlp: &Path,
    info: &[u8],
    proxy: &crate::settings::ProxyConfig,
) -> Result<Vec<u8>> {
    let mut command = tokio::process::Command::new(yt_dlp);
    apply_yt_proxy(&mut command, proxy);
    command.args([
        "--ignore-config",
        "--no-warnings",
        "--no-progress",
        "--no-playlist",
        "--no-simulate",
        "--socket-timeout",
        "10",
        "--retries",
        "1",
        "-f",
        YOUTUBE_AUDIO_FORMAT,
        "-o",
        "-",
        "--load-info-json",
        "-",
    ]);
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    command.kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command
        .spawn()
        .context("Cannot start inspected audio download")?;
    let mut stdin = child
        .stdin
        .take()
        .context("Audio metadata input unavailable")?;
    let (written, output) = tokio::time::timeout(Duration::from_secs(90), async {
        tokio::join!(
            async {
                let result = stdin.write_all(info).await;
                drop(stdin);
                result
            },
            child.wait_with_output()
        )
    })
    .await
    .context("Inspected audio download timed out")?;
    written.context("Could not pass inspected audio metadata")?;
    let output = output.context("Inspected audio download failed")?;
    anyhow::ensure!(
        output.status.success() && !output.stdout.is_empty(),
        "Inspected audio download failed ({})",
        output.status
    );
    Ok(output.stdout)
}

async fn download_recording_audio(
    yt_dlp: &Path,
    id: &str,
    info: Option<&Vec<u8>>,
    proxy: &crate::settings::ProxyConfig,
) -> Result<Vec<u8>> {
    if let Some(info) = info
        && let Ok(bytes) = download_inspected_audio(yt_dlp, info, proxy).await
    {
        return Ok(bytes);
    }
    download_youtube_audio_by_url(
        yt_dlp,
        &format!("https://www.youtube.com/watch?v={id}"),
        proxy,
    )
    .await
}

/// Only complete catalogue evidence can bypass the remaining query variants.
pub fn fast_catalog_match(
    reference: &Reference,
    recording: &Recording,
    matched: &crate::recording_match::Match,
) -> bool {
    matched.source == SourceKind::DistributedAudio
        && recording
            .duration
            .is_some_and(|duration| (duration - f64::from(reference.duration_secs)).abs() <= 3.0)
}

/// Resolve against all Spotify artists, title, album and duration. Flat search
/// is only a shortlist; watch-page credits and publisher identity authorize play.
pub async fn resolve_track_audio(
    yt_dlp: &Path,
    cache: &MatchCache,
    track: &LocalTrack,
    proxy: &crate::settings::ProxyConfig,
) -> Result<Vec<u8>> {
    // If Play catches the next-track prefetch, wait for its cache result
    // instead of starting another search and download for the same recording.
    let lock = cache.resolution_lock(track);
    let _guard = lock.lock().await;
    resolve_track_audio_inner(yt_dlp, cache, track, proxy).await
}

async fn resolve_track_audio_inner(
    yt_dlp: &Path,
    cache: &MatchCache,
    track: &LocalTrack,
    proxy: &crate::settings::ProxyConfig,
) -> Result<Vec<u8>> {
    if track.uri.starts_with("vibefy:") {
        let url =
            crate::addons::direct_url(&track.uri).context("Unsupported external track address")?;
        let key = hex(&Sha1::digest(track.uri.as_bytes()));
        cache.store_match(
            &track.uri,
            CachedMatch {
                youtube_id: key.clone(),
                title: track.title.clone(),
                duration_secs: track.duration_ms / 1000,
                matcher_version: crate::audio_match::MATCHER_VERSION,
                reference_key: recording_reference_key(track),
                source_kind: Some(SourceKind::MetadataMatch),
                channel: track.artist_names(),
                channel_id: String::new(),
            },
        );
        if let Some(bytes) = cache.get_cached_audio(&key) {
            return Ok(bytes);
        }
        let mut command = tokio::process::Command::new(yt_dlp);
        apply_yt_proxy(&mut command, proxy);
        command.args([
            "--ignore-config",
            "--no-warnings",
            "--no-playlist",
            "--max-filesize",
            "128M",
            "--fragment-retries",
            "1",
            "--socket-timeout",
            "15",
            "--retries",
            "1",
            "-f",
            if track.uri.starts_with("vibefy:youtube:") {
                YOUTUBE_AUDIO_FORMAT
            } else {
                "ba[acodec=mp3]/ba[ext=mp3]/ba[ext=m4a]/ba[acodec^=mp4a]/ba[acodec=vorbis]"
            },
            "-o",
            "-",
            "--",
            &url,
        ]);
        command.kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let output = tokio::time::timeout(Duration::from_secs(90), command.output())
            .await
            .context("Source audio download timed out")??;
        if !output.status.success() || output.stdout.is_empty() {
            anyhow::bail!("{}", crate::addons::source_error(&output.stderr));
        }
        anyhow::ensure!(
            output.stdout.len() <= 128 * 1024 * 1024,
            "This track exceeds the add-on audio cache limit (128 MB)."
        );
        cache.save_audio(&key, &output.stdout);
        return Ok(output.stdout);
    }
    anyhow::ensure!(
        track.has_recording_metadata(),
        "Spotify track details are still loading; no YouTube search was made"
    );
    let spotify_id = extract_spotify_id(&track.uri);
    let reference = recording_reference(track);
    let artist = track.artist_names();
    let title = &track.title;
    if let Some(cached) = cache.get_match(&spotify_id) {
        if cached.matches_track(track) {
            if let Some(bytes) = cache.get_cached_audio(&cached.youtube_id) {
                log::info!(
                    "Verified audio cache hit for '{artist} - {title}' ({})",
                    cached.youtube_id
                );
                return Ok(bytes);
            }
            let url = format!("https://www.youtube.com/watch?v={}", cached.youtube_id);
            if let Ok(bytes) = download_youtube_audio_by_url(yt_dlp, &url, proxy).await {
                cache.save_audio(&cached.youtube_id, &bytes);
                return Ok(bytes);
            }
        } else {
            log::info!("Rechecking obsolete recording match for '{artist} - {title}'");
            cache.remove_match(&spotify_id);
        }
    }

    let mut candidates = HashMap::<String, (Recording, i32)>::new();
    let mut inspected = std::collections::HashSet::new();
    let mut inspected_success = std::collections::HashSet::new();
    let mut unavailable = std::collections::HashSet::new();
    let mut verified = Vec::<(Recording, crate::recording_match::Match)>::new();
    let mut inspected_info = HashMap::<String, Vec<u8>>::new();
    let mut completed_searches = 0;
    let mut completed_inspections = 0;
    let mut last_search_error = None;
    let mut last_inspection_error = None;
    let mut last_download_error = None;
    let queries = crate::recording_match::search_queries(&reference);
    for (batch, pair) in queries.chunks(2).enumerate() {
        let final_batch = (batch + 1) * 2 >= queries.len();
        let mut searches = tokio::task::JoinSet::new();
        for query in pair {
            let query = query.clone();
            let binary = yt_dlp.to_owned();
            let proxy = proxy.clone();
            searches
                .spawn(async move { search_candidates_for_query(&binary, &query, &proxy).await });
        }
        while let Some(result) = searches.join_next().await {
            let result =
                result.unwrap_or_else(|error| Err(anyhow::anyhow!("Search task failed: {error}")));

            match result {
                Ok(entries) => {
                    completed_searches += 1;
                    for entry in entries {
                        if let Some(score) =
                            crate::recording_match::shortlist_score(&reference, &entry)
                            && candidates
                                .get(&entry.id)
                                .is_none_or(|(_, old)| score > *old)
                        {
                            candidates.insert(entry.id.clone(), (entry, score));
                        }
                    }
                }
                Err(error) => last_search_error = Some(error),
            }
            let mut shortlist: Vec<_> = candidates.values().cloned().collect();
            shortlist.sort_by(|(a, sa), (b, sb)| sb.cmp(sa).then_with(|| a.id.cmp(&b.id)));
            shortlist.truncate(12);
            let global_order: Vec<_> = shortlist
                .iter()
                .map(|(entry, _)| entry.id.clone())
                .collect();
            shortlist.retain(|(entry, _)| !inspected.contains(&entry.id));
            let final_results = final_batch && searches.is_empty();
            if !final_results {
                // Speculate only on catalogue-like entries. Other uploads retain
                // the full-query ordering and inspection budget.
                shortlist.retain(|(entry, _)| {
                    entry.track.is_some()
                        || entry
                            .channel
                            .as_deref()
                            .is_some_and(|channel| channel.ends_with(" - Topic"))
                });
                shortlist.truncate(2);
            }
            'inspection_pairs: for pair in shortlist.chunks(2) {
                for (entry, _) in pair {
                    inspected.insert(entry.id.clone());
                }
                let mut details = tokio::task::JoinSet::new();
                for (entry, _) in pair {
                    let entry = entry.clone();
                    let binary = yt_dlp.to_owned();
                    let proxy = proxy.clone();
                    details.spawn(async move { inspect_recording(&binary, &entry, &proxy).await });
                }
                while let Some(result) = details.join_next().await {
                    let result = result.unwrap_or_else(|error| {
                        Err(anyhow::anyhow!("Inspection task failed: {error}"))
                    });

                    match result {
                        Ok((recording, info)) => {
                            completed_inspections += 1;
                            inspected_success.insert(recording.id.clone());
                            if let Some(matched) =
                                crate::recording_match::verified_match(&reference, &recording)
                            {
                                inspected_info.insert(recording.id.clone(), info);
                                verified.push((recording, matched));
                            }
                        }
                        Err(error) => last_inspection_error = Some(error),
                    }
                    // A fully inspected catalogue recording with matching duration can
                    // play immediately; fan uploads still wait for every search variant.
                    verified.sort_by(|(a, ma), (b, mb)| {
                        mb.score.cmp(&ma.score).then_with(|| a.id.cmp(&b.id))
                    });
                    // Only the best three download candidates need their larger
                    // format lists retained. Lower-ranked matches keep URL fallback.
                    inspected_info.retain(|id, _| {
                        verified
                            .iter()
                            .take(3)
                            .any(|(recording, _)| &recording.id == id)
                    });
                    for (recording, matched) in verified.iter().filter(|(recording, matched)| {
                        fast_catalog_match(&reference, recording, matched)
                    }) {
                        if unavailable.contains(&recording.id) {
                            continue;
                        }
                        match download_recording_audio(
                            yt_dlp,
                            &recording.id,
                            inspected_info.get(&recording.id),
                            proxy,
                        )
                        .await
                        {
                            Ok(bytes) => {
                                cache.save_audio(&recording.id, &bytes);
                                cache.save_verified_match(track, recording, matched.source);
                                return Ok(bytes);
                            }
                            Err(error) => {
                                unavailable.insert(recording.id.clone());
                                last_download_error = Some(error);
                            }
                        }
                    }
                    let prefix_successes = global_order
                        .iter()
                        .take_while(|id| inspected.contains(*id))
                        .filter(|id| inspected_success.contains(*id))
                        .count();
                    let first_pair_inspected =
                        global_order.iter().take(2).all(|id| inspected.contains(id));
                    if final_results
                        && details.is_empty()
                        && ((first_pair_inspected
                            && verified.iter().any(|(recording, matched)| {
                                global_order.iter().take(2).any(|id| id == &recording.id)
                                    && matches!(matched.source, SourceKind::VerifiedArtist)
                                    && recording.duration.is_some_and(|duration| {
                                        (duration - f64::from(reference.duration_secs)).abs() <= 3.0
                                    })
                            }))
                            || (prefix_successes >= 6 && !verified.is_empty()))
                    {
                        break 'inspection_pairs;
                    }
                }
            }
        }
    }
    if completed_searches == 0
        && let Some(error) = last_search_error
    {
        return Err(error)
            .context("Playback cannot search YouTube. Check network and proxy settings");
    }
    verified.sort_by(|(a, ma), (b, mb)| mb.score.cmp(&ma.score).then_with(|| a.id.cmp(&b.id)));
    for (recording, matched) in verified
        .into_iter()
        .filter(|(recording, _)| !unavailable.contains(&recording.id))
        .take(3)
    {
        log::info!(
            "Verified recording selected: '{}' (ID: {}, source: {:?})",
            recording.title.as_deref().unwrap_or(""),
            recording.id,
            matched.source
        );
        match download_recording_audio(
            yt_dlp,
            &recording.id,
            inspected_info.get(&recording.id),
            proxy,
        )
        .await
        {
            Ok(bytes) => {
                cache.save_audio(&recording.id, &bytes);
                cache.save_verified_match(track, &recording, matched.source);
                return Ok(bytes);
            }
            Err(error) => last_download_error = Some(error),
        }
    }
    if let Some(error) = last_download_error {
        return Err(error)
            .context("Could not download a verified recording at standard audio quality");
    }
    if completed_inspections == 0
        && let Some(error) = last_inspection_error
    {
        return Err(error)
            .context("Could not verify the recording publisher. Check network and proxy settings");
    }
    anyhow::bail!(
        "No matching YouTube recording found after comparing the track title, artist and version. Official channels are preferred but are not required."
    )
}

struct YoutubeVolume(Arc<AtomicU32>);

impl VolumeGetter for YoutubeVolume {
    fn attenuation_factor(&self) -> f64 {
        f64::from(self.0.load(Ordering::Relaxed)) / 65535.0
    }
}

pub struct Engine {
    state: Arc<Mutex<LocalState>>,
    notify: Notify,
    device_id: String,
    device_name: String,
    audio: Arc<AudioControl>,
    active: Arc<AtomicBool>,
    yt_dlp_path: Option<PathBuf>,
    cmd_tx: mpsc::Sender<AudioWorkerCmd>,
    volume: Arc<AtomicU32>,
    generation: Arc<AtomicU64>,
    queue: Arc<Mutex<QueueState>>,
    cache: MatchCache,
    proxy: Arc<Mutex<crate::settings::ProxyConfig>>,
}

impl Engine {
    pub fn match_cache(&self) -> MatchCache {
        self.cache.clone()
    }
    pub fn heard(&self) -> Heard {
        Heard {
            state: Arc::clone(&self.state),
        }
    }

    pub async fn rootlist(&self) -> Result<Rootlist> {
        Ok(Rootlist {
            uris: Vec::new(),
            entries: Vec::new(),
            editable: BTreeSet::new(),
        })
    }

    pub async fn lyrics_json(&self, _track_id: &str) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }

    pub async fn album_is_ep(&self, _album_id: &str) -> Result<bool> {
        Ok(false)
    }

    pub fn session(&self) -> &crate::session_reads::Session {
        static SESSION: crate::session_reads::Session = crate::session_reads::Session;
        &SESSION
    }

    pub async fn connect(
        config: &EngineConfig,
        _proxy: Option<reqwest::Url>,
        _credentials: crate::credentials::Credentials,
        _cache: (),
        notify: Notify,
        _kept: Option<KeptPlayer>,
    ) -> Result<Self> {
        let device_id = config.device_id();
        let device_name = config.device_name.clone();
        let audio = AudioControl::new(config.buffer_ms);

        let yt_dlp_path = find_yt_dlp_binary();
        let yt_error = if yt_dlp_path.is_none() {
            log::warn!(
                "yt-dlp was not found in PATH or known user locations. Audio playback will be unavailable."
            );
            Some("yt-dlp not found in PATH. Install yt-dlp to enable YouTube playback.".to_string())
        } else {
            log::info!("yt-dlp found at: {:?}", yt_dlp_path.as_ref().unwrap());
            None
        };

        let initial_state = LocalState {
            source: None,
            playback: Playback::Stopped,
            track: None,
            position_ms: 0,
            position_at: None,
            volume: config.initial_volume,
            shuffle: false,
            repeat: RepeatMode::Off,
            connected: true,
            username: "Free Spotify User".into(),
            error: yt_error,
            queue: Vec::new(),
            queue_tracks: Vec::new(),
            queue_index: 0,
            loading: false,
            track_sequence: 1,
            seek_sequence: 1,
        };

        let state = Arc::new(Mutex::new(initial_state.clone()));
        notify(EngineEvent::State(initial_state));

        let active = Arc::new(AtomicBool::new(true));
        let volume = Arc::new(AtomicU32::new(config.initial_volume as u32));
        let generation = Arc::new(AtomicU64::new(0));

        let base_dir = config.volume_dir.parent().unwrap_or(&config.volume_dir);
        let cache = MatchCache::new(base_dir);
        let queue = Arc::new(Mutex::new(QueueState::default()));
        let proxy = Arc::new(Mutex::new(config.proxy.clone()));

        let (cmd_tx, cmd_rx) = mpsc::channel::<AudioWorkerCmd>();
        let (event_tx, event_rx) = mpsc::channel::<WorkerEvent>();

        // Spawn background dedicated audio worker thread
        let worker_audio = Arc::clone(&audio);
        let worker_tap = Arc::clone(&config.tap);
        let worker_volume = Arc::clone(&volume);
        let worker_state = Arc::clone(&state);
        let worker_notify = Arc::clone(&notify);
        let worker_device = config.audio_device.clone();
        let worker_buffer_ms = config.buffer_ms;
        let worker_eq = Arc::clone(&config.eq);

        std::thread::spawn(move || {
            let mut equalizer = crate::eq::Processor::new(worker_eq);
            let mut sink = RodioSink::new(
                worker_device,
                Arc::new(|e| log::error!("RodioSink error: {e}")),
                Box::new(YoutubeVolume(worker_volume)),
                worker_buffer_ms,
                worker_audio,
            );
            let mut converter = Converter::new(None);

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    AudioWorkerCmd::Play {
                        bytes,
                        start_pos_ms,
                        generation,
                    } => {
                        equalizer.reset();
                        let cursor = Cursor::new(bytes);
                        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
                        let mut hint = Hint::new();
                        hint.with_extension("m4a");
                        let probed = match symphonia::default::get_probe().format(
                            &hint,
                            mss,
                            &FormatOptions::default(),
                            &MetadataOptions::default(),
                        ) {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("Failed to probe audio stream: {e}");
                                continue;
                            }
                        };

                        let mut format = probed.format;
                        let track = match format
                            .tracks()
                            .iter()
                            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
                        {
                            Some(t) => t,
                            None => {
                                log::error!("No audio track found in stream");
                                continue;
                            }
                        };

                        let track_id = track.id;
                        let track_rate = track.codec_params.sample_rate.unwrap_or(SAMPLE_RATE);

                        if let Some(n_frames) = track.codec_params.n_frames {
                            let dur_ms = ((n_frames as f64 / track_rate as f64) * 1000.0) as u32;
                            if dur_ms > 0 {
                                let mut s = worker_state.lock().unwrap();
                                if let Some(source) = &mut s.source {
                                    source.duration_ms = dur_ms;
                                }
                                if let Some(t) = &mut s.track {
                                    if t.duration_ms == 0 {
                                        t.duration_ms = dur_ms;
                                        worker_notify(EngineEvent::State(s.clone()));
                                    }
                                }
                            }
                        }
                        let mut resampler = crate::resample::Resampler::new(
                            track_rate,
                            SAMPLE_RATE,
                            NUM_CHANNELS as usize,
                        );

                        let mut decoder = match symphonia::default::get_codecs()
                            .make(&track.codec_params, &DecoderOptions::default())
                        {
                            Ok(d) => d,
                            Err(e) => {
                                log::error!("Failed to initialize audio decoder: {e}");
                                continue;
                            }
                        };

                        let actual_start_ms = if start_pos_ms > 0 {
                            crate::audio_match::seek_recording(&mut *format, track_id, start_pos_ms)
                                .unwrap_or(0)
                        } else {
                            0
                        };

                        let _ = sink.start();

                        let mut sample_buf = None;

                        let mut sample_clock =
                            crate::audio_match::SampleClock::new(actual_start_ms, SAMPLE_RATE);
                        let mut last_notify = Instant::now();
                        let mut paused = false;
                        let mut playback_aborted = false;

                        'decode_loop: loop {
                            while let Ok(incoming) = cmd_rx.try_recv() {
                                match incoming {
                                    AudioWorkerCmd::Stop => {
                                        let _ = sink.stop();
                                        playback_aborted = true;
                                        break 'decode_loop;
                                    }
                                    AudioWorkerCmd::Pause => {
                                        let _ = sink.stop();
                                        paused = true;
                                    }
                                    AudioWorkerCmd::Resume => {
                                        let _ = sink.start();
                                        paused = false;
                                    }
                                    AudioWorkerCmd::Seek(pos) => {
                                        if let Ok(actual_ms) = crate::audio_match::seek_recording(
                                            &mut *format,
                                            track_id,
                                            pos,
                                        ) {
                                            sample_clock.seek(actual_ms);
                                            decoder.reset();
                                            equalizer.reset();
                                            resampler = crate::resample::Resampler::new(
                                                track_rate,
                                                SAMPLE_RATE,
                                                NUM_CHANNELS as usize,
                                            );
                                        }
                                    }
                                    AudioWorkerCmd::Play { .. } => {
                                        let _ = sink.stop();
                                        playback_aborted = true;
                                        break 'decode_loop;
                                    }
                                }
                            }

                            while paused {
                                match cmd_rx.recv() {
                                    Ok(AudioWorkerCmd::Resume) => {
                                        let _ = sink.start();
                                        paused = false;
                                    }
                                    Ok(AudioWorkerCmd::Stop) => {
                                        playback_aborted = true;
                                        break 'decode_loop;
                                    }
                                    Ok(AudioWorkerCmd::Seek(pos)) => {
                                        if let Ok(actual_ms) = crate::audio_match::seek_recording(
                                            &mut *format,
                                            track_id,
                                            pos,
                                        ) {
                                            sample_clock.seek(actual_ms);
                                            decoder.reset();
                                            equalizer.reset();
                                            resampler = crate::resample::Resampler::new(
                                                track_rate,
                                                SAMPLE_RATE,
                                                NUM_CHANNELS as usize,
                                            );
                                        }
                                    }
                                    Ok(AudioWorkerCmd::Play { .. }) => {
                                        playback_aborted = true;
                                        break 'decode_loop;
                                    }
                                    _ => {}
                                }
                            }

                            // Read after handling commands: a seek invalidates the old packet.
                            let packet = match format.next_packet() {
                                Ok(packet) => packet,
                                Err(_) => break,
                            };
                            if packet.track_id() != track_id {
                                continue;
                            }

                            match decoder.decode(&packet) {
                                Ok(decoded) => {
                                    if sample_buf.is_none() {
                                        let spec = *decoded.spec();
                                        sample_buf = Some(SampleBuffer::<f32>::new(
                                            decoded.capacity() as u64,
                                            spec,
                                        ));
                                    }

                                    if let Some(buf) = &mut sample_buf {
                                        buf.copy_interleaved_ref(decoded);
                                        let f32_samples = buf.samples();

                                        let samples_44k: std::borrow::Cow<[f32]> =
                                            if let Some(r) = &mut resampler {
                                                std::borrow::Cow::Owned(r.process(f32_samples))
                                            } else {
                                                std::borrow::Cow::Borrowed(f32_samples)
                                            };

                                        let mut f64_samples: Vec<f64> =
                                            samples_44k.iter().map(|&s| s as f64).collect();
                                        equalizer.process(&mut f64_samples);

                                        // Push samples to visualizer
                                        worker_tap.push(&f64_samples, 1.0);

                                        // Push samples to audio output device
                                        let _ = sink.write(
                                            AudioPacket::Samples(f64_samples),
                                            &mut converter,
                                        );

                                        let frames = samples_44k.len() / NUM_CHANNELS as usize;
                                        let current_pos_ms = crate::audio_match::audible_position(
                                            sample_clock.advance(frames),
                                            sink.queued_ms(),
                                        );

                                        if last_notify.elapsed() >= Duration::from_millis(250) {
                                            last_notify = Instant::now();
                                            let mut s = worker_state.lock().unwrap();
                                            s.position_ms = current_pos_ms;
                                            s.position_at = Some(Instant::now());
                                            worker_notify(EngineEvent::State(s.clone()));
                                        }
                                    }
                                }
                                Err(symphonia::core::errors::Error::DecodeError(e)) => {
                                    log::trace!("Symphonia decode warning: {e}");
                                    continue;
                                }
                                Err(_) => break 'decode_loop,
                            }
                        }

                        let _ = sink.stop();

                        if !playback_aborted {
                            let _ = event_tx.send(WorkerEvent::TrackEnded { generation });
                        }
                    }
                    AudioWorkerCmd::Pause => {
                        let _ = sink.stop();
                    }
                    AudioWorkerCmd::Resume => {
                        let _ = sink.start();
                    }
                    AudioWorkerCmd::Seek(_) | AudioWorkerCmd::Stop => {
                        let _ = sink.stop();
                    }
                }
            }
        });

        // Spawn background listener for track completion (auto-advance / next track)
        let state_ev = Arc::clone(&state);
        let notify_ev = Arc::clone(&notify);
        let cmd_tx_ev = cmd_tx.clone();
        let yt_dlp_ev = yt_dlp_path.clone();
        let gen_ev = Arc::clone(&generation);
        let cache_ev = cache.clone();
        let queue_ev = Arc::clone(&queue);
        let proxy_ev = Arc::clone(&proxy);

        let rt_handle = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            let _enter = rt_handle.enter();
            while let Ok(WorkerEvent::TrackEnded {
                generation: end_gen,
            }) = event_rx.recv()
            {
                if gen_ev.load(Ordering::SeqCst) == end_gen {
                    log::info!("Track completed naturally, auto-advancing to next track");
                    Engine::advance_next(
                        &state_ev, &notify_ev, &cmd_tx_ev, &yt_dlp_ev, &gen_ev, &cache_ev,
                        &proxy_ev, &queue_ev, true,
                    );
                }
            }
        });

        Ok(Self {
            state,
            notify,
            device_id,
            device_name,
            audio,
            active,
            yt_dlp_path,
            cmd_tx,
            volume,
            generation,
            queue,
            cache,
            proxy,
        })
    }

    pub fn credentials(&self) -> Option<crate::credentials::Credentials> {
        None
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn shutdown(&self) {
        self.active.store(false, Ordering::Relaxed);
        let _ = self.cmd_tx.send(AudioWorkerCmd::Stop);
        let mut state = self.state.lock().unwrap();
        state.connected = false;
        state.playback = Playback::Stopped;
        (self.notify)(EngineEvent::State(state.clone()));
    }

    pub fn close_session(&self) {
        self.shutdown();
    }

    /// Path construction only; the auto-sync worker opens this file off-thread.
    pub fn cached_recording(&self, youtube_id: &str) -> Option<PathBuf> {
        if youtube_id.is_empty()
            || !youtube_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return None;
        }
        Some(self.cache.audio_dir.join(format!("{youtube_id}.m4a")))
    }

    /// Runs on a backend blocking worker: cache IO never blocks drawing.
    pub fn rematch_current_audio(&self) {
        let track = self.state.lock().unwrap().track.clone();
        if let Some(track) = track {
            self.cache.remove_match(&extract_spotify_id(&track.uri));
            Self::start_track(
                Arc::clone(&self.state),
                Arc::clone(&self.notify),
                self.cmd_tx.clone(),
                self.yt_dlp_path.clone(),
                Arc::clone(&self.generation),
                self.cache.clone(),
                Arc::clone(&self.proxy),
                track,
                0,
            );
        }
    }

    pub fn keep_player(&self) -> KeptPlayer {
        KeptPlayer {
            state: Arc::clone(&self.state),
        }
    }

    pub fn carrying(&self) -> bool {
        false
    }

    pub fn interrupted(&self) -> Option<Interrupted> {
        self.state.lock().unwrap().interrupted()
    }

    pub fn resume_point(&self) -> Option<PlaybackResume> {
        let state = self.state.lock().unwrap();
        state.track.as_ref().map(|track| PlaybackResume::Track {
            uri: track.uri.clone(),
            position_ms: state.position_now(),
        })
    }

    pub fn resume(&self, resume: PlaybackResume) -> Result<()> {
        log::info!("YoutubePlayer: resume {resume:?}");
        Ok(())
    }

    fn start_track(
        state: Arc<Mutex<LocalState>>,
        notify: Notify,
        cmd_tx: mpsc::Sender<AudioWorkerCmd>,
        yt_dlp_path: Option<PathBuf>,
        generation_atomic: Arc<AtomicU64>,
        cache: MatchCache,
        proxy: Arc<Mutex<crate::settings::ProxyConfig>>,
        track: LocalTrack,
        start_pos_ms: u32,
    ) {
        let generation = generation_atomic.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = cmd_tx.send(AudioWorkerCmd::Stop);

        let artist_name = track.artist_names();
        let track_title = track.title.clone();
        let spotify_id = extract_spotify_id(&track.uri);

        {
            let mut s = state.lock().unwrap();
            s.source = None;
            s.track = Some(track.clone());
            s.playback = Playback::Loading;
            s.loading = true;
            s.error = None;
            s.position_ms = start_pos_ms;
            s.position_at = None;
            s.track_sequence += 1;
            notify(EngineEvent::State(s.clone()));
        }

        if !track.has_recording_metadata() {
            log::info!("Waiting for Spotify metadata before resolving audio");
            return;
        }

        let state_clone = Arc::clone(&state);
        let notify_clone = Arc::clone(&notify);
        let cmd_tx_clone = cmd_tx.clone();
        let current_gen = Arc::clone(&generation_atomic);
        let proxy_cfg = proxy.lock().unwrap().clone();

        tokio::spawn(async move {
            if current_gen.load(Ordering::SeqCst) != generation {
                return;
            }

            let yt_dlp = match yt_dlp_path {
                Some(path) => path,
                None => {
                    let mut s = state_clone.lock().unwrap();
                    s.playback = Playback::Stopped;
                    s.loading = false;
                    s.error =
                        Some("yt-dlp is not available. Install yt-dlp for audio playback.".into());
                    notify_clone(EngineEvent::State(s.clone()));
                    return;
                }
            };

            let audio_started = Instant::now();
            let audio_res = tokio::select! {
                result = resolve_track_audio(&yt_dlp, &cache, &track, &proxy_cfg) => result,
                _ = async {
                    while current_gen.load(Ordering::SeqCst) == generation {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                } => return,
            };
            log::info!(
                "Playback audio preparation: {} ms",
                audio_started.elapsed().as_millis()
            );

            if current_gen.load(Ordering::SeqCst) != generation {
                return;
            }

            let bytes = match audio_res {
                Ok(b) => b,
                Err(err) => {
                    log::error!(
                        "Failed to fetch YouTube audio for '{artist_name} - {track_title}': {err}"
                    );
                    let mut s = state_clone.lock().unwrap();
                    s.playback = Playback::Stopped;
                    s.loading = false;
                    s.error = Some(format!("Playback failed: {err}"));
                    notify_clone(EngineEvent::State(s.clone()));
                    return;
                }
            };

            if current_gen.load(Ordering::SeqCst) != generation {
                return;
            }

            {
                let mut state = state_clone.lock().unwrap();
                state.source = cache.get_match(&spotify_id).map(|matched| PlaybackSource {
                    youtube_id: matched.youtube_id,
                    title: matched.title,
                    duration_ms: matched.duration_secs.saturating_mul(1000),
                });
            }

            log::info!(
                "Audio stream ready for '{artist_name} - {track_title}' ({} bytes), starting playback",
                bytes.len()
            );

            {
                let mut s = state_clone.lock().unwrap();
                s.playback = Playback::Playing;
                s.loading = false;
                if let Some(t) = &mut s.track {
                    if t.duration_ms == 0 {
                        t.duration_ms = 210_000;
                    }
                }
                s.position_at = Some(Instant::now());
                notify_clone(EngineEvent::State(s.clone()));
            }

            let _ = cmd_tx_clone.send(AudioWorkerCmd::Play {
                bytes,
                start_pos_ms,
                generation,
            });
        });
    }

    fn prefetch_upcoming_tracks(
        state: Arc<Mutex<LocalState>>,
        queue: Arc<Mutex<QueueState>>,
        cache: MatchCache,
        yt_dlp_path: Option<PathBuf>,
        proxy: Arc<Mutex<crate::settings::ProxyConfig>>,
        count: usize,
    ) {
        let Some(yt_dlp) = yt_dlp_path else { return };
        let proxy_cfg = proxy.lock().unwrap().clone();
        let expected = state.lock().unwrap().track_sequence;
        tokio::spawn(async move {
            // Give the requested song priority over speculative downloads.
            loop {
                let (sequence, playback) = {
                    let current = state.lock().unwrap();
                    (current.track_sequence, current.playback)
                };
                if sequence != expected || matches!(playback, Playback::Stopped | Playback::Paused)
                {
                    return;
                }
                if playback == Playback::Playing {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
            tokio::time::sleep(std::time::Duration::from_millis(750)).await;
            if state.lock().unwrap().track_sequence != expected {
                return;
            }
            let tracks: Vec<LocalTrack> = {
                let q = queue.lock().unwrap();
                let mut list = Vec::new();
                for i in 1..=count {
                    if let Some(track) = q.tracks.get(q.current_index + i) {
                        list.push(track.clone());
                    }
                }
                list
            };

            for track in tracks {
                if state.lock().unwrap().track_sequence != expected {
                    return;
                }
                let id = extract_spotify_id(&track.uri);
                let needs_download = cache.get_match(&id).is_none_or(|matched| {
                    !matched.matches_track(&track) || !cache.has_cached_audio(&matched.youtube_id)
                });
                if needs_download {
                    log::info!("Prefetching verified recording for '{}'", track.title);
                    tokio::select! {
                        _ = resolve_track_audio(&yt_dlp, &cache, &track, &proxy_cfg) => {},
                        _ = async {
                            loop {
                                {
                                    let current = state.lock().unwrap();
                                    if current.track_sequence != expected
                                        && current.track.as_ref().is_none_or(|selected| selected.uri != track.uri)
                                    {
                                        return;
                                    }
                                }
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                        } => return,
                    }
                }
            }
        });
    }

    fn advance_next(
        state: &Arc<Mutex<LocalState>>,
        notify: &Notify,
        cmd_tx: &mpsc::Sender<AudioWorkerCmd>,
        yt_dlp_path: &Option<PathBuf>,
        generation_atomic: &Arc<AtomicU64>,
        cache: &MatchCache,
        proxy: &Arc<Mutex<crate::settings::ProxyConfig>>,
        queue: &Arc<Mutex<QueueState>>,
        autoplay: bool,
    ) {
        let next_track = {
            let mut q = queue.lock().unwrap();
            if q.tracks.is_empty() {
                None
            } else if q.repeat == RepeatMode::Track && autoplay {
                q.tracks.get(q.current_index).cloned()
            } else if q.shuffle && q.tracks.len() > 1 {
                let next_idx = rand::random_range(0..q.tracks.len());
                q.current_index = next_idx;
                q.tracks.get(q.current_index).cloned()
            } else if q.current_index + 1 < q.tracks.len() {
                q.current_index += 1;
                q.tracks.get(q.current_index).cloned()
            } else if q.repeat == RepeatMode::Context {
                q.current_index = 0;
                q.tracks.get(0).cloned()
            } else {
                None
            }
        };

        if let Some(track) = next_track {
            {
                let q = queue.lock().unwrap();
                let mut s = state.lock().unwrap();
                s.queue_tracks = q.tracks.clone();
                s.queue_index = q.current_index;
                s.queue = q
                    .tracks
                    .iter()
                    .skip(q.current_index + 1)
                    .map(|t| t.uri.clone())
                    .collect();
            }
            Self::start_track(
                Arc::clone(state),
                Arc::clone(notify),
                cmd_tx.clone(),
                yt_dlp_path.clone(),
                Arc::clone(generation_atomic),
                cache.clone(),
                Arc::clone(proxy),
                track,
                0,
            );

            // Proactively prefetch next 2 tracks in background
            Self::prefetch_upcoming_tracks(
                Arc::clone(state),
                Arc::clone(queue),
                cache.clone(),
                yt_dlp_path.clone(),
                Arc::clone(proxy),
                2,
            );
        } else {
            let _ = cmd_tx.send(AudioWorkerCmd::Stop);
            let mut s = state.lock().unwrap();
            s.playback = Playback::Stopped;
            s.position_ms = 0;
            s.position_at = None;
            s.track_sequence += 1;
            notify(EngineEvent::State(s.clone()));
        }
    }

    fn advance_previous(
        state: &Arc<Mutex<LocalState>>,
        notify: &Notify,
        cmd_tx: &mpsc::Sender<AudioWorkerCmd>,
        yt_dlp_path: &Option<PathBuf>,
        generation_atomic: &Arc<AtomicU64>,
        cache: &MatchCache,
        proxy: &Arc<Mutex<crate::settings::ProxyConfig>>,
        queue: &Arc<Mutex<QueueState>>,
    ) {
        let current_pos = state.lock().unwrap().position_now();
        if current_pos > 3000 {
            let _ = cmd_tx.send(AudioWorkerCmd::Seek(0));
            let mut s = state.lock().unwrap();
            s.position_ms = 0;
            s.position_at = if s.playback == Playback::Playing {
                Some(Instant::now())
            } else {
                None
            };
            s.seek_sequence += 1;
            notify(EngineEvent::State(s.clone()));
        } else {
            let prev_track = {
                let mut q = queue.lock().unwrap();
                if q.current_index > 0 {
                    q.current_index -= 1;
                    q.tracks.get(q.current_index).cloned()
                } else {
                    q.tracks.first().cloned()
                }
            };

            if let Some(track) = prev_track {
                {
                    let q = queue.lock().unwrap();
                    let mut s = state.lock().unwrap();
                    s.queue_tracks = q.tracks.clone();
                    s.queue_index = q.current_index;
                    s.queue = q
                        .tracks
                        .iter()
                        .skip(q.current_index + 1)
                        .map(|t| t.uri.clone())
                        .collect();
                }
                Self::start_track(
                    Arc::clone(state),
                    Arc::clone(notify),
                    cmd_tx.clone(),
                    yt_dlp_path.clone(),
                    Arc::clone(generation_atomic),
                    cache.clone(),
                    Arc::clone(proxy),
                    track,
                    0,
                );
            } else {
                let _ = cmd_tx.send(AudioWorkerCmd::Stop);
                let mut s = state.lock().unwrap();
                s.position_ms = 0;
                s.position_at = None;
                s.playback = Playback::Stopped;
                s.track_sequence += 1;
                notify(EngineEvent::State(s.clone()));
            }
        }
    }

    pub fn command(&self, command: PlayerCommand) -> Result<()> {
        log::info!("YoutubePlayer command received: {command:?}");
        match command {
            PlayerCommand::UpdateMetadata(track) => {
                let (restart, position) = {
                    let mut queue = self.queue.lock().unwrap();
                    let mut state = self.state.lock().unwrap();
                    let restart = receive_track_metadata(&mut state, &mut queue, &track);
                    (self.notify)(EngineEvent::State(state.clone()));
                    (restart, state.position_ms)
                };
                if let Some(track) = restart {
                    Self::start_track(
                        Arc::clone(&self.state),
                        Arc::clone(&self.notify),
                        self.cmd_tx.clone(),
                        self.yt_dlp_path.clone(),
                        Arc::clone(&self.generation),
                        self.cache.clone(),
                        Arc::clone(&self.proxy),
                        track,
                        position,
                    );
                }
            }
            PlayerCommand::MetadataFailed(uri) => {
                let mut state = self.state.lock().unwrap();
                if state
                    .track
                    .as_ref()
                    .is_some_and(|track| track.uri == uri && !track.has_recording_metadata())
                {
                    state.playback = Playback::Stopped;
                    state.loading = false;
                    state.error = Some("Cannot load Spotify track details yet. Try again after the network or Spotify rate limit recovers.".into());
                    (self.notify)(EngineEvent::State(state.clone()));
                }
            }
            PlayerCommand::Load(spec) => {
                let mut q = self.queue.lock().unwrap();
                q.shuffle = spec.shuffle.unwrap_or(q.shuffle);
                q.repeat = spec.repeat.unwrap_or(q.repeat);
                q.manual_queue_count = 0;

                if !spec.queue_tracks.is_empty() {
                    q.tracks = spec.queue_tracks;
                    if let Some(offset) = &spec.offset_uri {
                        q.current_index =
                            q.tracks.iter().position(|t| t.uri == *offset).unwrap_or(0);
                    } else if let Some(idx) = spec.offset_index {
                        q.current_index = (idx as usize).min(q.tracks.len().saturating_sub(1));
                    } else {
                        q.current_index = 0;
                    }
                } else if let Some(t) = &spec.track {
                    q.tracks = vec![t.clone()];
                    q.current_index = 0;
                }

                let track = q
                    .tracks
                    .get(q.current_index)
                    .cloned()
                    .or(spec.track)
                    .unwrap_or_else(|| {
                        let track_uri = spec
                            .offset_uri
                            .or_else(|| spec.uris.first().cloned())
                            .unwrap_or_else(|| "spotify:track:placeholder".to_string());
                        LocalTrack {
                            uri: track_uri,
                            name: "Loading from YouTube...".into(),
                            title: "Loading from YouTube...".into(),
                            album: "YouTube Audio".into(),
                            album_name: "YouTube Audio".into(),
                            artists: vec![ArtistRef {
                                name: "Unknown Artist".into(),
                                id: None,
                                uri: None,
                            }],
                            duration_ms: 0,
                            cover_url: None,
                            art_url: None,
                            art_small_url: None,
                            is_episode: false,
                        }
                    });
                {
                    let mut s = self.state.lock().unwrap();
                    s.queue_tracks = q.tracks.clone();
                    s.queue_index = q.current_index;
                    s.queue = q
                        .tracks
                        .iter()
                        .skip(q.current_index + 1)
                        .map(|t| t.uri.clone())
                        .collect();
                }
                drop(q);

                Self::start_track(
                    Arc::clone(&self.state),
                    Arc::clone(&self.notify),
                    self.cmd_tx.clone(),
                    self.yt_dlp_path.clone(),
                    Arc::clone(&self.generation),
                    self.cache.clone(),
                    Arc::clone(&self.proxy),
                    track,
                    spec.position_ms,
                );

                // Proactively prefetch next 2 tracks in background
                Self::prefetch_upcoming_tracks(
                    Arc::clone(&self.state),
                    Arc::clone(&self.queue),
                    self.cache.clone(),
                    self.yt_dlp_path.clone(),
                    Arc::clone(&self.proxy),
                    2,
                );
            }
            PlayerCommand::Toggle => {
                let mut state = self.state.lock().unwrap();
                if state.playback == Playback::Loading {
                    self.generation.fetch_add(1, Ordering::SeqCst);
                    state.playback = Playback::Paused;
                    state.loading = false;
                    (self.notify)(EngineEvent::State(state.clone()));
                    return Ok(());
                } else if state.playback == Playback::Playing {
                    state.playback = Playback::Paused;
                    state.position_ms = state.position_now();
                    state.position_at = None;
                    let _ = self.cmd_tx.send(AudioWorkerCmd::Pause);
                } else if state.playback == Playback::Paused || state.playback == Playback::Stopped
                {
                    if state.source.is_none()
                        && let Some(track) = state.track.clone()
                    {
                        let position = state.position_ms;
                        drop(state);
                        Self::start_track(
                            Arc::clone(&self.state),
                            Arc::clone(&self.notify),
                            self.cmd_tx.clone(),
                            self.yt_dlp_path.clone(),
                            Arc::clone(&self.generation),
                            self.cache.clone(),
                            Arc::clone(&self.proxy),
                            track,
                            position,
                        );
                        return Ok(());
                    }
                    state.playback = Playback::Playing;
                    state.position_at = Some(Instant::now());
                    let _ = self.cmd_tx.send(AudioWorkerCmd::Resume);
                }
                (self.notify)(EngineEvent::State(state.clone()));
            }
            PlayerCommand::Next => {
                log::info!("YoutubePlayer: Next track requested");
                Self::advance_next(
                    &self.state,
                    &self.notify,
                    &self.cmd_tx,
                    &self.yt_dlp_path,
                    &self.generation,
                    &self.cache,
                    &self.proxy,
                    &self.queue,
                    false,
                );
            }
            PlayerCommand::Previous => {
                log::info!("YoutubePlayer: Previous track requested");
                Self::advance_previous(
                    &self.state,
                    &self.notify,
                    &self.cmd_tx,
                    &self.yt_dlp_path,
                    &self.generation,
                    &self.cache,
                    &self.proxy,
                    &self.queue,
                );
            }
            PlayerCommand::Seek(pos) => {
                let _ = self.cmd_tx.send(AudioWorkerCmd::Seek(pos));
                let mut state = self.state.lock().unwrap();
                state.position_ms = pos;
                state.position_at = if state.playback == Playback::Playing {
                    Some(Instant::now())
                } else {
                    None
                };
                state.seek_sequence += 1;
                (self.notify)(EngineEvent::State(state.clone()));
            }
            PlayerCommand::Volume(vol) | PlayerCommand::VolumePreview(vol) => {
                self.volume.store(vol as u32, Ordering::Relaxed);
                let mut state = self.state.lock().unwrap();
                state.volume = vol;
                (self.notify)(EngineEvent::State(state.clone()));
            }
            PlayerCommand::Shuffle(enabled) => {
                let mut q = self.queue.lock().unwrap();
                q.shuffle = enabled;
                let mut state = self.state.lock().unwrap();
                state.shuffle = enabled;
                (self.notify)(EngineEvent::State(state.clone()));
            }
            PlayerCommand::Repeat(mode) => {
                let mut q = self.queue.lock().unwrap();
                q.repeat = mode;
                let mut state = self.state.lock().unwrap();
                state.repeat = mode;
                (self.notify)(EngineEvent::State(state.clone()));
            }
            PlayerCommand::ClearQueue => {
                let mut q = self.queue.lock().unwrap();
                if q.manual_queue_count > 0 {
                    let start = (q.current_index + 1).min(q.tracks.len());
                    let end = (start + q.manual_queue_count).min(q.tracks.len());
                    q.tracks.drain(start..end);
                    q.manual_queue_count = 0;
                }
                let mut state = self.state.lock().unwrap();
                state.queue_tracks = q.tracks.clone();
                state.queue_index = q.current_index;
                state.queue = q
                    .tracks
                    .iter()
                    .skip(q.current_index + 1)
                    .map(|t| t.uri.clone())
                    .collect();
                (self.notify)(EngineEvent::State(state.clone()));
            }
            PlayerCommand::AddToQueue(uri) => {
                let mut q = self.queue.lock().unwrap();
                let insert_idx = (q.current_index + 1 + q.manual_queue_count).min(q.tracks.len());
                let new_track = LocalTrack {
                    uri: uri.clone(),
                    name: "Queued track".into(),
                    title: "Queued track".into(),
                    album: String::new(),
                    album_name: String::new(),
                    artists: Vec::new(),
                    duration_ms: 0,
                    cover_url: None,
                    art_url: None,
                    art_small_url: None,
                    is_episode: false,
                };
                q.tracks.insert(insert_idx, new_track);
                q.manual_queue_count += 1;
                let mut state = self.state.lock().unwrap();
                state.queue_tracks = q.tracks.clone();
                state.queue_index = q.current_index;
                state.queue = q
                    .tracks
                    .iter()
                    .skip(q.current_index + 1)
                    .map(|t| t.uri.clone())
                    .collect();
                (self.notify)(EngineEvent::State(state.clone()));
            }
            PlayerCommand::SkipTo(target_idx) => {
                let mut q = self.queue.lock().unwrap();
                if target_idx < q.tracks.len() {
                    q.current_index = target_idx;
                    let track = q.tracks[target_idx].clone();
                    {
                        let mut state = self.state.lock().unwrap();
                        state.queue_tracks = q.tracks.clone();
                        state.queue_index = q.current_index;
                        state.queue = q
                            .tracks
                            .iter()
                            .skip(q.current_index + 1)
                            .map(|t| t.uri.clone())
                            .collect();
                    }
                    drop(q);
                    Self::start_track(
                        Arc::clone(&self.state),
                        Arc::clone(&self.notify),
                        self.cmd_tx.clone(),
                        self.yt_dlp_path.clone(),
                        Arc::clone(&self.generation),
                        self.cache.clone(),
                        Arc::clone(&self.proxy),
                        track,
                        0,
                    );
                    Self::prefetch_upcoming_tracks(
                        Arc::clone(&self.state),
                        Arc::clone(&self.queue),
                        self.cache.clone(),
                        self.yt_dlp_path.clone(),
                        Arc::clone(&self.proxy),
                        2,
                    );
                }
            }
            PlayerCommand::SkipToUri(uri) => {
                let mut q = self.queue.lock().unwrap();
                let found = q
                    .tracks
                    .iter()
                    .enumerate()
                    .skip(q.current_index + 1)
                    .find(|(_, t)| t.uri == uri)
                    .or_else(|| q.tracks.iter().enumerate().find(|(_, t)| t.uri == uri))
                    .map(|(i, _)| i);
                if let Some(target_idx) = found {
                    q.current_index = target_idx;
                    let track = q.tracks[target_idx].clone();
                    {
                        let mut state = self.state.lock().unwrap();
                        state.queue_tracks = q.tracks.clone();
                        state.queue_index = q.current_index;
                        state.queue = q
                            .tracks
                            .iter()
                            .skip(q.current_index + 1)
                            .map(|t| t.uri.clone())
                            .collect();
                    }
                    drop(q);
                    Self::start_track(
                        Arc::clone(&self.state),
                        Arc::clone(&self.notify),
                        self.cmd_tx.clone(),
                        self.yt_dlp_path.clone(),
                        Arc::clone(&self.generation),
                        self.cache.clone(),
                        Arc::clone(&self.proxy),
                        track,
                        0,
                    );
                    Self::prefetch_upcoming_tracks(
                        Arc::clone(&self.state),
                        Arc::clone(&self.queue),
                        self.cache.clone(),
                        self.yt_dlp_path.clone(),
                        Arc::clone(&self.proxy),
                        2,
                    );
                }
            }
            PlayerCommand::Transfer => {}
        }

        Ok(())
    }
}

pub struct Rootlist {
    pub uris: Vec<String>,
    pub entries: Vec<RootlistEntry>,
    pub editable: BTreeSet<String>,
}

pub fn editable_uris<'a>(uris: impl IntoIterator<Item = &'a String>) -> BTreeSet<String> {
    uris.into_iter().cloned().collect()
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RootlistEntry {
    Playlist(String),
    FolderStart { id: String, name: String },
    FolderEnd,
}

pub fn parse_rootlist(uris: &[String]) -> Vec<RootlistEntry> {
    uris.iter()
        .map(|u| RootlistEntry::Playlist(u.clone()))
        .collect()
}

pub const AUDIO_KEY_REFUSED: &str = "Spotify refused an audio key";
