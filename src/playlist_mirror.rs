//! Per-account, opt-in Spotify -> private YouTube Music playlist copies.
//! Durable intents make interrupted remote writes recoverable without blind replay.
use crate::{
    addons::Collection,
    api::{
        client::ApiError,
        gateway::{ApiGateway, Operation},
        models::{PlayableItem, Playlist, Track, pick_image},
    },
    http::Http,
    music::{Client as Search, Filter},
    music_accounts::{self},
    music_session::{Failure, Session},
    player::LocalTrack,
    settings::ProxyConfig,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum Action {
    Observe,
    Reviews(usize),
    Enable(bool),
    SyncNow,
    Choose { uri: String, recording: LocalTrack },
    Search { uri: String, query: String },
    RetryChecked(String),
    Wake,
}
#[derive(Clone, Debug, Default)]
pub struct View {
    pub ready: bool,
    pub enabled: bool,
    pub busy: bool,
    pub pair: String,
    pub account: String,
    pub youtube_name: String,
    pub message: String,
    pub lists: Vec<Arc<ListView>>,
    pub reviews: Vec<Decision>,
    pub review_total: usize,
    pub review_offset: usize,
}
#[derive(Clone, Debug)]
pub struct ListView {
    pub spotify_id: String,
    pub youtube_id: String,
    pub source_uris: Vec<String>,
    pub missing: usize,
    pub uncertain: bool,
    pub issue: Option<String>,
    pub collection: Collection,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    pub source: LocalTrack,
    pub chosen: Option<LocalTrack>,
    pub candidates: Vec<LocalTrack>,
    pub checked_at: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Intent {
    pub creating: bool,
    /// Multiset of video IDs expected after this append, including repetitions.
    pub expected: BTreeMap<String, usize>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Linked {
    title: String,
    snapshot: Option<String>,
    youtube_id: String,
    tracks: Vec<LocalTrack>,
    extra: Vec<LocalTrack>,
    image: Option<String>,
    intent: Option<Intent>,
    #[serde(default)]
    issue: Option<String>,
}
#[derive(Default, Serialize, Deserialize)]
struct Journal {
    version: u32,
    spotify: String,
    youtube: String,
    enabled: bool,
    retry_at: u64,
    #[serde(default)]
    rate_limited: bool,
    lists: BTreeMap<String, Linked>,
    #[serde(default, skip_serializing)]
    decisions: BTreeMap<String, Decision>,
}
#[derive(Serialize, Deserialize)]
struct Addition {
    playlist: String,
    tracks: Vec<LocalTrack>,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn pair_key(spotify: &str, youtube: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(spotify, youtube)).unwrap())
    )
}
fn valid_spotify(id: &str) -> bool {
    id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}
pub fn local_track(track: &Track) -> LocalTrack {
    let art = track
        .album
        .as_ref()
        .and_then(|a| pick_image(&a.images, 300))
        .filter(|url| crate::addons::safe_art_url(url))
        .map(str::to_owned);
    LocalTrack {
        uri: track.uri.clone(),
        title: track.name.clone(),
        name: track.name.clone(),
        artists: track.artists.clone(),
        duration_ms: track.duration_ms,
        album: track
            .album
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_default(),
        cover_url: art.clone(),
        art_url: art.clone(),
        art_small_url: art,
        ..Default::default()
    }
}
/// Persist a dual-destination request independently of Spotify's acknowledgement.
pub async fn enqueue(root: &Path, pair: &str, playlist: String, tracks: Vec<Track>) -> Result<()> {
    ensure!(
        pair.len() == 64 && pair.bytes().all(|b| b.is_ascii_hexdigit()) && valid_spotify(&playlist),
        "Invalid sync destination"
    );
    ensure!(
        !tracks.is_empty() && tracks.len() <= 1000,
        "Invalid sync addition size"
    );
    let tracks: Vec<_> = tracks
        .iter()
        .filter(|t| {
            t.uri
                .strip_prefix("spotify:track:")
                .is_some_and(valid_spotify)
                && !t.is_local
        })
        .map(local_track)
        .collect();
    ensure!(!tracks.is_empty(), "Only Spotify songs can be mirrored");
    let bytes = serde_json::to_vec(&Addition { playlist, tracks })?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let path = root.join("inbox").join(pair).join(format!("{hash}.json"));
    tokio::task::spawn_blocking(move || fastframe_file::write_private(&path, bytes)).await??;
    Ok(())
}
pub fn counts(ids: impl IntoIterator<Item = String>) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    for id in ids {
        *result.entry(id).or_default() += 1;
    }
    result
}
pub fn missing_ids(desired: &[String], existing: &BTreeMap<String, usize>) -> Vec<String> {
    let mut remaining = existing.clone();
    desired
        .iter()
        .filter(|id| {
            let n = remaining.entry((*id).clone()).or_default();
            if *n > 0 {
                *n -= 1;
                false
            } else {
                true
            }
        })
        .cloned()
        .collect()
}
pub fn intent_confirmed(intent: &Intent, existing: &BTreeMap<String, usize>) -> bool {
    !intent.creating
        && intent
            .expected
            .iter()
            .all(|(id, n)| existing.get(id).copied().unwrap_or(0) >= *n)
}
/// High-confidence catalogue matches only. Ambiguous versions remain reviewable.
pub fn choose_recording(source: &LocalTrack, candidates: &[LocalTrack]) -> Option<LocalTrack> {
    let norm = crate::audio_match::normalized;
    let title = norm(&source.title);
    let artist = source.artists.first().map(|a| norm(&a.name))?;
    if title.is_empty() || artist.is_empty() || source.duration_ms == 0 {
        return None;
    }
    let mut scored: Vec<_> = candidates
        .iter()
        .filter(|c| {
            c.uri
                .strip_prefix("vibefy:youtube:")
                .is_some_and(crate::addons::valid_video_id)
                && norm(&c.title) == title
                && c.artists.iter().any(|a| norm(&a.name) == artist)
                && source.artists.iter().all(|expected| {
                    c.artists
                        .iter()
                        .any(|found| norm(&found.name) == norm(&expected.name))
                })
                && c.duration_ms > 0
                && source.duration_ms.abs_diff(c.duration_ms) <= 3500
                && crate::audio_match::compatible_version(&source.title, &c.title)
        })
        .map(|c| {
            let album = (!source.album.is_empty() && norm(&c.album) == norm(&source.album)) as i32;
            (
                album * 20 - (source.duration_ms.abs_diff(c.duration_ms) / 1000) as i32,
                c,
            )
        })
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    let (score, best) = scored.first()?;
    if let Some((second, other)) = scored.get(1) {
        let equivalent = norm(&best.album) == norm(&other.album)
            && best.duration_ms.abs_diff(other.duration_ms) <= 1000;
        if score - second < 10 && !equivalent {
            return None;
        }
    }
    Some((*best).clone())
}

#[derive(Debug, thiserror::Error)]
#[error("Prioritizing a new playlist addition")]
struct Interrupted;

struct Service {
    root: PathBuf,
    path: PathBuf,
    pair: String,
    journal: Journal,
    youtube_name: String,
    session: Session,
    proxy: ProxyConfig,
    api: Arc<ApiGateway>,
    search: Search,
    rx: mpsc::UnboundedReceiver<Action>,
    events: std::sync::mpsc::Sender<crate::backend::Event>,
    waker: crate::backend::Waker,
    serial: u64,
    review_offset: usize,
    views: BTreeMap<String, Arc<ListView>>,
    priority: Option<String>,
    last_client: Option<Arc<crate::api::ApiClient>>,
}
// Serialized disk operations remain ordered if a task is cancelled while a
// blocking atomic write finishes. Reads acquire the same lock before loading.
async fn io_guard() -> tokio::sync::OwnedMutexGuard<()> {
    static IO: std::sync::OnceLock<Arc<tokio::sync::Mutex<()>>> = std::sync::OnceLock::new();
    IO.get_or_init(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
        .lock_owned()
        .await
}
async fn file(path: PathBuf, write: Option<Vec<u8>>) -> Result<Option<Vec<u8>>> {
    let guard = io_guard().await;
    tokio::task::spawn_blocking(move || -> Result<_> {
        let _guard = guard;
        if let Some(bytes) = write {
            fastframe_file::write_private(&path, bytes)?;
            return Ok(None);
        }
        if let Ok(metadata) = std::fs::metadata(&path) {
            ensure!(
                metadata.len() <= 64 * 1024 * 1024,
                "Sync journal is too large"
            );
        }
        match std::fs::read(path) {
            Ok(bytes) => {
                ensure!(bytes.len() <= 64 * 1024 * 1024, "Sync journal is too large");
                Ok(Some(bytes))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
    .await?
}
async fn append_decision(path: PathBuf, decision: &Decision) -> Result<()> {
    use std::io::Write;
    let mut bytes = serde_json::to_vec(decision)?;
    bytes.push(b'\n');
    let guard = io_guard().await;
    tokio::task::spawn_blocking(move || -> Result<()> {
        let _guard = guard;
        if !path.exists() {
            fastframe_file::write_private(&path, b"")?;
        }
        ensure!(
            std::fs::metadata(&path)?
                .len()
                .saturating_add(bytes.len() as u64)
                <= 64 * 1024 * 1024,
            "Recording match cache is too large"
        );
        let mut output = std::fs::OpenOptions::new().append(true).open(path)?;
        output.write_all(&bytes)?;
        output.sync_data()?;
        Ok(())
    })
    .await?
}
/// Recover complete records only; a torn last append is discarded on restart.
pub fn recover_decisions(bytes: &[u8]) -> Result<BTreeMap<String, Decision>> {
    ensure!(
        bytes.len() <= 64 * 1024 * 1024,
        "Recording match cache is too large"
    );
    let mut decisions = BTreeMap::new();
    for line in bytes.split_inclusive(|b| *b == b'\n') {
        if !line.ends_with(b"\n") {
            break;
        }
        if line.len() == 1 {
            continue;
        }
        let decision: Decision = serde_json::from_slice(line)?;
        ensure!(
            decision
                .source
                .uri
                .strip_prefix("spotify:track:")
                .is_some_and(valid_spotify),
            "Invalid cached Spotify recording"
        );
        decisions.insert(decision.source.uri.clone(), decision);
    }
    Ok(decisions)
}
impl Service {
    async fn save(&self) -> Result<()> {
        file(self.path.clone(), Some(serde_json::to_vec(&self.journal)?)).await?;
        Ok(())
    }
    fn progress(&self, text: impl Into<String>) {
        let _ = self.events.send(crate::backend::Event::MirrorProgress {
            serial: self.serial,
            message: text.into(),
        });
        self.waker.wake();
    }
    fn publish(&mut self, message: impl Into<String>, busy: bool) {
        let active: HashSet<_> = self
            .journal
            .lists
            .values()
            .flat_map(|list| list.tracks.iter().chain(&list.extra))
            .map(|track| track.uri.as_str())
            .collect();
        let reviews: Vec<_> = self
            .journal
            .decisions
            .values()
            .filter(|d| d.chosen.is_none() && active.contains(d.source.uri.as_str()))
            .collect();
        self.review_offset = self
            .review_offset
            .min(reviews.len().saturating_sub(1) / 100 * 100);
        let view = View {
            ready: true,
            enabled: self.journal.enabled,
            busy,
            pair: self.pair.clone(),
            account: self.journal.spotify.clone(),
            youtube_name: self.youtube_name.clone(),
            message: message.into(),
            review_offset: self.review_offset,
            lists: self.views.values().cloned().collect(),
            review_total: reviews.len(),
            reviews: reviews
                .into_iter()
                .skip(self.review_offset)
                .take(100)
                .cloned()
                .collect(),
        };
        let _ = self.events.send(crate::backend::Event::Mirror {
            serial: self.serial,
            view,
        });
        self.waker.wake();
    }
    fn update_view(&mut self, id: &str) {
        let Some(list) = self.journal.lists.get(id) else {
            return;
        };

        let mut missing = 0;
        let tracks = list
            .tracks
            .iter()
            .chain(&list.extra)
            .map(|source| {
                if let Some(chosen) = self
                    .journal
                    .decisions
                    .get(&source.uri)
                    .and_then(|d| d.chosen.as_ref())
                {
                    let mut track = source.clone();
                    track.uri = chosen.uri.clone();
                    track
                } else {
                    missing += 1;
                    source.clone()
                }
            })
            .collect();
        self.views.insert(
            id.into(),
            Arc::new(ListView {
                spotify_id: id.into(),
                youtube_id: list.youtube_id.clone(),
                source_uris: list
                    .tracks
                    .iter()
                    .chain(&list.extra)
                    .map(|track| track.uri.clone())
                    .collect(),
                missing,
                uncertain: list.intent.is_some(),
                issue: list.issue.clone(),
                collection: Collection {
                    title: list.title.clone(),
                    source: if list.youtube_id.is_empty() {
                        format!("vibefy:mirror-pending:{id}")
                    } else {
                        format!("https://www.youtube.com/playlist?list={}", list.youtube_id)
                    },
                    tracks,
                    image: list.image.clone(),
                    owner: Some("Spotify · YouTube Music".into()),
                },
            }),
        );
    }
    async fn inbox(&mut self) -> Result<bool> {
        let dir = self.root.join("inbox").join(&self.pair);
        let mut entries = match tokio::fs::read_dir(dir).await {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e.into()),
        };
        let mut changed = false;
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_file()
                || entry.path().extension().is_none_or(|s| s != "json")
            {
                continue;
            }
            let bytes = file(entry.path(), None).await?.unwrap_or_default();
            ensure!(
                bytes.len() <= 4 * 1024 * 1024,
                "Pending sync request is too large"
            );
            let addition: Addition = serde_json::from_slice(&bytes)?;
            if let Some(list) = self.journal.lists.get_mut(&addition.playlist) {
                for track in addition.tracks {
                    if !list
                        .tracks
                        .iter()
                        .chain(&list.extra)
                        .any(|t| t.uri == track.uri)
                    {
                        list.extra.push(track);
                        changed = true;
                    }
                }
                self.priority = Some(addition.playlist);
                self.save().await?;
                tokio::fs::remove_file(entry.path()).await?;
            }
        }
        Ok(changed)
    }
    async fn action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::SyncNow => {
                if !self.journal.rate_limited {
                    self.journal.retry_at = 0;
                }
            }
            Action::Reviews(offset) => {
                self.review_offset = offset;
            }
            Action::Enable(enabled) => {
                self.journal.enabled = enabled;
                if enabled && !self.journal.rate_limited {
                    self.journal.retry_at = 0;
                }
                self.save().await?;
            }
            Action::Choose { uri, recording } => {
                ensure!(
                    recording
                        .uri
                        .strip_prefix("vibefy:youtube:")
                        .is_some_and(crate::addons::valid_video_id),
                    "Invalid chosen recording"
                );
                let decision = self
                    .journal
                    .decisions
                    .get_mut(&uri)
                    .ok_or_else(|| anyhow::anyhow!("This recording no longer needs review"))?;
                ensure!(
                    decision.candidates.iter().any(|c| c.uri == recording.uri),
                    "Search for this recording before choosing it"
                );
                decision.chosen = Some(recording);
                decision.checked_at = now();
                self.priority = self
                    .journal
                    .lists
                    .iter()
                    .find(|(_, list)| list.tracks.iter().chain(&list.extra).any(|t| t.uri == uri))
                    .map(|(id, _)| id.clone());
                append_decision(
                    self.root.join(format!("{}-recordings.jsonl", self.pair)),
                    &self.journal.decisions[&uri],
                )
                .await?;
                self.save().await?;
            }
            Action::Search { uri, query } => {
                ensure!(
                    self.journal.decisions.contains_key(&uri),
                    "Unknown source recording"
                );
                let results = self.search.search(&query, Filter::Songs, None).await?;
                self.journal.decisions.get_mut(&uri).unwrap().candidates = results
                    .entries
                    .into_iter()
                    .filter_map(|e| e.track)
                    .take(5)
                    .collect();
                append_decision(
                    self.root.join(format!("{}-recordings.jsonl", self.pair)),
                    &self.journal.decisions[&uri],
                )
                .await?;
            }
            Action::RetryChecked(id) => {
                if let Some(list) = self.journal.lists.get_mut(&id) {
                    list.intent = None;
                    self.save().await?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    async fn checkpoint(&mut self) -> Result<()> {
        for _ in 0..16 {
            match self.rx.try_recv() {
                Ok(action) => {
                    self.action(action).await?;
                    self.publish("Sync request received", true);
                }
                Err(_) => break,
            }
        }
        self.inbox().await?;
        ensure!(self.journal.enabled, "Playlist sync is paused");
        ensure!(
            self.api.account().as_ref().map(|a| a.as_str()) == Some(self.journal.spotify.as_str()),
            "Spotify account changed; sync paused"
        );
        Ok(())
    }
    async fn spotify_read<T>(
        &mut self,
        future: impl std::future::Future<Output = crate::api::client::Result<T>>,
    ) -> Result<T> {
        tokio::pin!(future);
        loop {
            tokio::select! {
                result=&mut future=>return result.map_err(Into::into),
                action=self.rx.recv()=> {
                    let action=action.ok_or_else(||anyhow::anyhow!("Playlist sync stopped"))?;
                    self.action(action).await?;self.publish("Sync settings updated",true);
                    ensure!(self.journal.enabled,"Playlist sync is paused");
                }
            }
        }
    }
    async fn refresh_list(&mut self, playlist: Playlist) -> Result<()> {
        self.api.observe_playlist(&playlist);
        let id = playlist.id.clone();
        if !valid_spotify(&id) {
            return Ok(());
        }
        let unchanged =
            self.journal.lists.get(&id).is_some_and(|l| {
                playlist.snapshot_id.is_some() && playlist.snapshot_id == l.snapshot
            });
        if unchanged {
            let list = self.journal.lists.get_mut(&id).unwrap();
            list.title = playlist.name;
            list.image = pick_image(&playlist.images, 300)
                .filter(|url| crate::addons::safe_art_url(url))
                .map(str::to_owned);
            return Ok(());
        }
        let api = self.api.clone();
        let client = self
            .spotify_read(api.client_for(Operation::PlaylistItems(api.playlist_access(&id))))
            .await?;
        self.last_client = Some(client.clone());
        let mut tracks = Vec::new();
        let mut offset = 0;
        let mut seen = HashSet::new();
        loop {
            self.checkpoint().await?;
            let page = self
                .spotify_read(client.playlist_items(&id, offset, 50))
                .await?;
            for item in &page.items {
                if let Some(PlayableItem::Track(track)) = item.playable() {
                    if !track.is_local
                        && track
                            .uri
                            .strip_prefix("spotify:track:")
                            .is_some_and(valid_spotify)
                    {
                        tracks.push(local_track(track));
                    }
                }
            }
            ensure!(
                tracks.len() <= 20_000,
                "A Spotify playlist exceeds 20,000 supported songs"
            );
            match page.next_offset() {
                Some(next) if next > offset && seen.insert(next) => offset = next,
                Some(_) => {
                    anyhow::bail!("Spotify returned a repeated page; no partial playlist was saved")
                }
                None => break,
            }
        }
        // A source edit during traversal must not become a complete cached snapshot.
        let api = self.api.clone();
        let header_client = self
            .spotify_read(api.client_for(Operation::PlaylistMetadata(api.playlist_access(&id))))
            .await?;
        self.last_client = Some(header_client.clone());
        let header = self.spotify_read(header_client.playlist(&id)).await?;
        ensure!(
            playlist.snapshot_id.is_none() || header.snapshot_id == playlist.snapshot_id,
            "Playlist changed during import; retrying on the next sync"
        );
        let list = self.journal.lists.entry(id).or_default();
        list.title = playlist.name;
        list.image = pick_image(&playlist.images, 300)
            .filter(|s| crate::addons::safe_art_url(s))
            .map(str::to_owned);
        list.snapshot = header.snapshot_id;
        list.tracks = tracks;
        list.extra
            .retain(|extra| !list.tracks.iter().any(|t| t.uri == extra.uri));
        self.save().await
    }
    async fn decisions(&mut self, id: &str) -> Result<()> {
        let tracks: Vec<_> = self.journal.lists[id]
            .tracks
            .iter()
            .chain(&self.journal.lists[id].extra)
            .cloned()
            .collect();
        for (index, track) in tracks.iter().enumerate() {
            self.checkpoint().await?;
            if self.priority.is_some() {
                return Err(Interrupted.into());
            }
            if self.journal.decisions.get(&track.uri).is_some_and(|d| {
                d.source.title == track.title
                    && d.source.artists == track.artists
                    && d.source.duration_ms == track.duration_ms
                    && (d.chosen.is_some() || now().saturating_sub(d.checked_at) < 21600)
            }) {
                continue;
            }
            self.progress(format!(
                "Matching {} · {}/{}",
                self.journal.lists[id].title,
                index + 1,
                tracks.len()
            ));
            let query = format!("{} {}", track.title, track.artist_names());
            let results = self.search.search(&query, Filter::Songs, None).await?;
            let candidates: Vec<_> = results
                .entries
                .into_iter()
                .filter_map(|e| e.track)
                .take(5)
                .collect();
            let chosen = choose_recording(track, &candidates);
            self.journal.decisions.insert(
                track.uri.clone(),
                Decision {
                    source: track.clone(),
                    chosen,
                    candidates,
                    checked_at: now(),
                },
            );
            append_decision(
                self.root.join(format!("{}-recordings.jsonl", self.pair)),
                &self.journal.decisions[&track.uri],
            )
            .await?;
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        Ok(())
    }
    fn marker(&self, id: &str) -> String {
        format!(
            "Private playlist copy managed by Vibefy.\nVibefy mirror: {}:{id}",
            self.pair
        )
    }
    async fn discover(&self, id: &str, title: &str) -> Result<Option<String>> {
        let marker = self.marker(id);
        let mut found = None;
        let title: String = title
            .chars()
            .filter(|c| !matches!(c, '<' | '>'))
            .take(150)
            .collect();
        let candidates: Vec<_> = self
            .session
            .playlists(&self.proxy)
            .await?
            .into_iter()
            .filter(|p| p.title == title)
            .collect();
        ensure!(
            candidates.len() <= 20,
            "Several playlists share this title; check the copies before retrying"
        );
        for candidate in candidates {
            let raw = candidate.id.strip_prefix("VL").unwrap_or(&candidate.id);
            let (private, description) = self.session.details(&self.proxy, raw).await?;
            if private && description.contains(&marker) {
                ensure!(
                    found.is_none(),
                    "Multiple private copies found; check them before retrying"
                );
                found = Some(raw.to_owned());
            }
        }
        Ok(found)
    }
    async fn copy(&mut self, id: &str) -> Result<()> {
        self.checkpoint().await?;
        let list = self.journal.lists[id].clone();
        if list.youtube_id.is_empty() {
            if let Some(existing) = self.discover(id, &list.title).await? {
                let saved = self.journal.lists.get_mut(id).unwrap();
                saved.youtube_id = existing;
                saved.intent = None;
                self.save().await?;
            } else {
                ensure!(
                    list.intent.is_none(),
                    "Playlist creation was not confirmed. Check YouTube Music before allowing another attempt."
                );
                self.checkpoint().await?;
                self.journal.lists.get_mut(id).unwrap().intent = Some(Intent {
                    creating: true,
                    expected: BTreeMap::new(),
                });
                self.save().await?;
                let result = self
                    .session
                    .create_private(&self.proxy, &list.title, &self.marker(id))
                    .await;
                match result {
                    Ok(youtube_id) => {
                        let saved = self.journal.lists.get_mut(id).unwrap();
                        saved.youtube_id = youtube_id;
                        saved.intent = None;
                        self.save().await?;
                    }
                    Err(error) => {
                        self.rejected(id, &error).await?;
                        return Err(error);
                    }
                }
            }
        }
        let list = self.journal.lists[id].clone();
        let (private, description) = self.session.details(&self.proxy, &list.youtube_id).await?;
        ensure!(
            private && description.contains(&self.marker(id)),
            "The linked copy is no longer private or its Vibefy marker changed. No songs were added."
        );
        let mut current = self
            .session
            .membership(&self.proxy, &list.youtube_id)
            .await?;
        if let Some(intent) = &list.intent {
            ensure!(
                intent_confirmed(intent, &current),
                "An earlier write was not confirmed. Check YouTube Music before retrying this playlist."
            );
            self.journal.lists.get_mut(id).unwrap().intent = None;
            self.save().await?;
        }
        let desired: Vec<_> = list
            .tracks
            .iter()
            .chain(&list.extra)
            .filter_map(|t| {
                self.journal
                    .decisions
                    .get(&t.uri)
                    .and_then(|d| d.chosen.as_ref())
            })
            .filter_map(|t| t.uri.strip_prefix("vibefy:youtube:").map(str::to_owned))
            .collect();
        for batch in missing_ids(&desired, &current).chunks(50) {
            self.checkpoint().await?;
            let mut expected = current.clone();
            for video in batch {
                *expected.entry(video.clone()).or_default() += 1;
            }
            self.journal.lists.get_mut(id).unwrap().intent = Some(Intent {
                creating: false,
                expected: expected.clone(),
            });
            self.save().await?;
            let acknowledgement = match self
                .session
                .append(&self.proxy, &list.youtube_id, batch)
                .await
            {
                Ok(confirmed) => confirmed,
                Err(error) => {
                    self.rejected(id, &error).await?;
                    return Err(error);
                }
            };
            if acknowledgement {
                current = expected.clone();
            } else {
                current = self
                    .session
                    .membership(&self.proxy, &list.youtube_id)
                    .await?;
            }
            ensure!(
                intent_confirmed(
                    &Intent {
                        creating: false,
                        expected
                    },
                    &current
                ),
                "Music did not confirm every song in this batch; check the pending write"
            );
            self.journal.lists.get_mut(id).unwrap().intent = None;
            self.save().await?;
        }
        Ok(())
    }
    async fn rejected(&mut self, id: &str, error: &anyhow::Error) -> Result<()> {
        if matches!(
            error.downcast_ref::<Failure>(),
            Some(Failure::RateLimited(_) | Failure::NotSent | Failure::Rejected(_))
        ) {
            self.journal.lists.get_mut(id).unwrap().intent = None;
            self.save().await?;
        }
        Ok(())
    }
    async fn cycle(&mut self) -> Result<()> {
        self.checkpoint().await?;
        self.publish("Reading Spotify playlist changes…", true);
        // New additions have priority over the periodic library scan.
        if let Some(id) = self.priority.take() {
            if self.journal.lists.contains_key(&id) {
                self.decisions(&id).await?;
                self.copy(&id).await?;
                self.update_view(&id);
                self.publish(
                    "YouTube Music copy checked; unmatched recordings need review",
                    true,
                );
            }
        }
        let api = self.api.clone();
        let client = self
            .spotify_read(api.client_for(Operation::PlaylistLibrary))
            .await?;
        self.last_client = Some(client.clone());
        let mut offset = 0;
        let mut seen = HashSet::new();
        let mut listed = Vec::new();
        loop {
            self.checkpoint().await?;
            let page = self.spotify_read(client.my_playlists(offset, 50)).await?;
            let next_page = page.next_offset();
            listed.extend(page.items);
            ensure!(
                listed.len() <= 10_000,
                "This library exceeds 10,000 playlists"
            );
            match next_page {
                Some(next) if next > offset && seen.insert(next) => offset = next,
                Some(_) => anyhow::bail!("Spotify returned a repeated library page"),
                None => break,
            }
        }
        let mut errors = Vec::new();
        for playlist in listed {
            let id = playlist.id.clone();
            if !valid_spotify(&id) {
                continue;
            }
            self.checkpoint().await?;
            let result = async {
                self.refresh_list(playlist).await?;
                self.inbox().await?;
                self.copy(&id).await?;
                self.update_view(&id);
                self.publish("Private copy linked; matching recordings…", true);
                self.decisions(&id).await?;
                self.copy(&id).await
            }
            .await;
            if let Some(list) = self.journal.lists.get_mut(&id) {
                list.issue = result.as_ref().err().map(|e| e.to_string());
            }
            self.update_view(&id);
            if let Err(error) = result {
                if error.downcast_ref::<Interrupted>().is_some()
                    || error
                        .downcast_ref::<Failure>()
                        .is_some_and(|f| !matches!(f, Failure::Rejected(404)))
                    || error
                        .downcast_ref::<ApiError>()
                        .is_some_and(|e| e.status() != Some(404))
                {
                    return Err(error);
                }
                errors.push(error.to_string());
            }
            self.publish(
                format!(
                    "{} playlists linked · {} need attention",
                    self.views.len(),
                    errors.len()
                ),
                true,
            );
        }
        self.save().await?;
        self.publish(
            if errors.is_empty() {
                "Sync finished. Check Review recordings for unmatched songs. Deletions stay separate.".into()
            } else {
                format!("{} playlists need attention. {}", errors.len(), errors[0])
            },
            false,
        );
        Ok(())
    }
    async fn run(mut self) {
        for id in self.journal.lists.keys().cloned().collect::<Vec<_>>() {
            self.update_view(&id);
        }
        self.publish(
            "Ready. Enable sync to create private YouTube Music copies.",
            false,
        );
        let mut due = 0;
        loop {
            if self.journal.enabled && now() >= due.max(self.journal.retry_at) {
                match self.cycle().await {
                    Ok(()) => {
                        due = now() + 900;
                        self.journal.retry_at = 0;
                        self.journal.rate_limited = false;
                        let _ = self.save().await;
                    }
                    Err(error) => {
                        if error.downcast_ref::<Interrupted>().is_some() || !self.journal.enabled {
                            due = 0;
                            continue;
                        }
                        let spotify_wait = if matches!(
                            error.downcast_ref::<ApiError>(),
                            Some(ApiError::RateLimited)
                        ) {
                            if let Some(client) = &self.last_client {
                                client
                                    .cooldown_remaining()
                                    .await
                                    .as_secs()
                                    .saturating_add(1)
                                    .max(1)
                            } else {
                                300
                            }
                        } else {
                            300
                        };
                        let delay = match error.downcast_ref::<Failure>() {
                            Some(Failure::RateLimited(seconds)) => *seconds,
                            Some(Failure::Rejected(401 | 403)) => 3600,
                            _ => spotify_wait,
                        };
                        self.journal.rate_limited = matches!(
                            error.downcast_ref::<Failure>(),
                            Some(Failure::RateLimited(_))
                        ) || matches!(
                            error.downcast_ref::<ApiError>(),
                            Some(ApiError::RateLimited)
                        );
                        self.journal.retry_at = now().saturating_add(delay);
                        let _ = self.save().await;
                        self.publish(format!("Sync paused: {error}"), false);
                        due = self.journal.retry_at;
                    }
                }
            }
            let delay = if self.journal.enabled {
                due.max(self.journal.retry_at).saturating_sub(now()).max(1)
            } else {
                86400
            };
            tokio::select! {
                action=self.rx.recv()=> match action {
                    Some(action)=> {let restart=!matches!(action,Action::Observe | Action::Reviews(_) | Action::Search{..});let review=matches!(action,Action::Search{..});
                        match self.action(action).await {Ok(())=> {if restart {due=0;}self.publish(if review {"Choose a recording below"}else{"Playlist sync settings updated"},false);},Err(error)=>self.publish(format!("Sync: {error}"),false)}
                    },None=>return,
                },
                _=tokio::time::sleep(Duration::from_secs(delay))=>{},
            }
        }
    }
}

pub struct Handle {
    pub serial: u64,
    pub sender: mpsc::UnboundedSender<Action>,
    pub task: tokio::task::JoinHandle<()>,
}
pub struct Context {
    pub root: PathBuf,
    pub config: PathBuf,
    pub proxy: ProxyConfig,
    pub http: Http,
    pub api: Arc<ApiGateway>,
    pub events: std::sync::mpsc::Sender<crate::backend::Event>,
    pub waker: crate::backend::Waker,
}
pub fn start(serial: u64, context: Context) -> Handle {
    let (sender, rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let result = async {
            let account = context
                .api
                .account()
                .ok_or_else(|| anyhow::anyhow!("Connect Spotify first"))?
                .as_str()
                .to_owned();
            let session = music_accounts::youtube_session(&context.config).await?;
            let (youtube, youtube_name) = session.identity(&context.proxy).await?;
            let pair = pair_key(&account, &youtube);
            let path = context.root.join(format!("{pair}.json"));
            let mut journal = match file(path.clone(), None).await? {
                Some(bytes) => {
                    let saved: Journal = serde_json::from_slice(&bytes)?;
                    ensure!(
                        saved.version == 1 && saved.spotify == account && saved.youtube == youtube,
                        "The saved account pairing does not match; sync is stopped"
                    );
                    saved
                }
                None => Journal {
                    version: 1,
                    spotify: account,
                    youtube,
                    ..Default::default()
                },
            };
            let cache = context.root.join(format!("{pair}-recordings.jsonl"));
            if let Some(bytes) = file(cache.clone(), None).await? {
                journal.decisions = recover_decisions(&bytes)?;
                if !bytes.is_empty() && !bytes.ends_with(b"\n") {
                    let mut repaired = Vec::new();
                    for decision in journal.decisions.values() {
                        serde_json::to_writer(&mut repaired, decision)?;
                        repaired.push(b'\n');
                    }
                    file(cache, Some(repaired)).await?;
                }
            }
            let service = Service {
                root: context.root,
                path,
                pair,
                journal,
                youtube_name,
                session,
                proxy: context.proxy,
                api: context.api,
                search: Search::new(context.http),
                rx,
                events: context.events.clone(),
                waker: context.waker.clone(),
                serial,
                review_offset: 0,
                views: BTreeMap::new(),
                priority: None,
                last_client: None,
            };
            service.run().await;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if let Err(error) = result {
            let _ = context.events.send(crate::backend::Event::Mirror {
                serial,
                view: View {
                    message: format!("Sync unavailable: {error}"),
                    ..Default::default()
                },
            });
            context.waker.wake();
        }
    });
    Handle {
        serial,
        sender,
        task,
    }
}
