//! Cloudflare relay sessions. Only track metadata and timing leave the player.
//! No inbound ports, peer IP addresses, audio, or account credentials are shared.
use crate::player::LocalTrack;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
const RELAY: &str = "https://vibefy-jam.pex-am.workers.dev";
const SOCKET: &str = "wss://vibefy-jam.pex-am.workers.dev/rooms/";
pub const PROTOCOL: u8 = 2;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub track: LocalTrack,
    pub position_ms: u32,
    pub playing: bool,
}
impl Snapshot {
    pub fn valid(&self) -> bool {
        let spotify = self
            .track
            .uri
            .strip_prefix("spotify:track:")
            .is_some_and(|id| id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric()));
        (spotify || crate::addons::direct_url(&self.track.uri).is_some())
            && self.track.title.len() <= 500
            && self.track.artists.len() <= 20
            && self.track.artists.iter().all(|a| a.name.len() <= 300)
            && self.track.duration_ms <= 86_400_000
            && self.position_ms <= 86_400_000
            && [
                &self.track.cover_url,
                &self.track.art_url,
                &self.track.art_small_url,
            ]
            .iter()
            .all(|url| {
                url.as_ref()
                    .is_none_or(|url| crate::addons::safe_art_url(url))
            })
    }
}

pub fn parse_invite(text: &str) -> Result<String> {
    let text = text.trim();
    let code = if text.contains("://") {
        let url = reqwest::Url::parse(text)?;
        ensure!(
            url.scheme() == "https"
                && matches!(
                    url.host_str(),
                    Some("usevibefy.online" | "www.usevibefy.online" | "vibefy-neon.vercel.app")
                )
                && url.username().is_empty()
                && url.password().is_none()
                && url.port().is_none()
                && matches!(url.path(), "/app" | "/web" | "/phone"),
            "Paste a Vibefy Jam link or code"
        );
        url.query_pairs()
            .find(|(k, _)| k == "jam")
            .map(|(_, v)| v.into_owned())
            .context("Missing Jam code")?
    } else {
        text.to_owned()
    };
    let code = code.replace('-', "").replace(' ', "").to_ascii_uppercase();
    ensure!(
        code.len() == 12
            && code
                .bytes()
                .all(|c| b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(&c)),
        "Enter a valid 12-character Jam code"
    );
    Ok(code)
}
/// Public album art only. Never relay arbitrary hosts, query tokens or user artwork.
fn public_cover_url(value: &str) -> Option<String> {
    let url = reqwest::Url::parse(value).ok()?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || value.len() > 2000
    {
        return None;
    }
    let host = url.host_str()?;
    let path = url.path();
    let allowed = match host {
        "api.juicevault.xyz" => {
            path.starts_with("/cdn/")
                && path
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/-_.".contains(&b))
        }
        "i.ytimg.com" => path.starts_with("/vi/") || path.starts_with("/vi_webp/"),
        "i.scdn.co" => path.starts_with("/image/"),
        _ => host.ends_with(".googleusercontent.com") || host.ends_with(".ggpht.com"),
    };
    allowed.then(|| url.to_string())
}
fn relay_track(mut t: LocalTrack) -> Option<LocalTrack> {
    let youtube = t
        .uri
        .strip_prefix("vibefy:youtube:")
        .is_some_and(crate::addons::valid_video_id);
    let juice = t
        .uri
        .strip_prefix("vibefy:juicevault:")
        .is_some_and(crate::music_accounts::uuid);
    if !(youtube || juice)
        || t.title.trim().is_empty()
        || t.title.chars().count() > 300
        || t.artists.len() > 12
    {
        return None;
    }
    let cover = [&t.cover_url, &t.art_url, &t.art_small_url]
        .into_iter()
        .filter_map(|v| v.as_deref())
        .find_map(public_cover_url);
    t.cover_url = cover;
    t.art_url = None;
    t.art_small_url = None;
    Some(t)
}
#[derive(Default)]
pub struct Shared {
    pub connected: bool,
    pub host_waiting: bool,
    pub status: String,
    pub invite: String,
    pub listeners: usize,
    pub queue: Vec<LocalTrack>,
    published: Option<(Snapshot, Instant)>,
    received: Option<(Snapshot, Instant)>,
    incoming: Vec<LocalTrack>,
    outgoing: Vec<LocalTrack>,
    published_queue: Vec<LocalTrack>,
    notifications: Vec<String>,
    pub latency_ms: u32,
    pub last_seen: Option<Instant>,
    pub controls_available: bool,
    live: Option<std::sync::Weak<Mutex<crate::player::LocalState>>>,
    sender: Option<tokio::sync::mpsc::UnboundedSender<crate::backend::Command>>,
    controls: Vec<Value>,
}
impl Shared {
    pub fn playback_notice(&self) -> &'static str {
        let Some(live) = self.live.as_ref().and_then(|s| s.upgrade()) else {
            return "Audio engine is connecting…";
        };
        let state = live.lock().unwrap();
        if state.error.is_some() {
            return "Audio unavailable. Choose another recording.";
        }
        if state.loading {
            return "Loading recording…";
        }
        if state.track.is_none() {
            return "Choose a song to begin.";
        }
        if Self::from_player(&state).is_none() {
            return "Waiting for a YouTube or JuiceVault recording.";
        }
        "Audio ready · YouTube & JuiceVault"
    }
    pub(crate) fn attach(
        &mut self,
        state: std::sync::Weak<Mutex<crate::player::LocalState>>,
        sender: tokio::sync::mpsc::UnboundedSender<crate::backend::Command>,
    ) {
        self.live = Some(state);
        self.sender = Some(sender);
    }
    fn from_player(state: &crate::player::LocalState) -> Option<Snapshot> {
        if !state.is_active() || state.loading {
            return None;
        }
        let mut track = state.track.clone()?;
        if let Some(duration) = state
            .source
            .as_ref()
            .map(|s| s.duration_ms)
            .filter(|d| *d > 0)
        {
            track.duration_ms = duration;
        }
        if !track.uri.starts_with("vibefy:juicevault:") {
            if let Some(source) = &state.source {
                if crate::addons::valid_video_id(&source.youtube_id) {
                    track.uri = format!("vibefy:youtube:{}", source.youtube_id);
                }
            }
        }
        Some(Snapshot {
            track: relay_track(track)?,
            position_ms: state.position_now(),
            playing: state.playback == crate::player::Playback::Playing,
        })
    }
    fn live_queue(state: &crate::player::LocalState) -> Vec<(usize, LocalTrack)> {
        state
            .queue_tracks
            .iter()
            .enumerate()
            .skip(state.queue_index.saturating_add(1))
            .filter_map(|(index, t)| relay_track(t.clone()).map(|t| (index, t)))
            .take(40)
            .collect()
    }
    fn host_packet(&self) -> (Option<Snapshot>, Vec<LocalTrack>) {
        if let Some(live) = &self.live {
            let Some(live) = live.upgrade() else {
                return (None, vec![]);
            };
            let state = live.lock().unwrap();
            return (
                Self::from_player(&state),
                Self::live_queue(&state)
                    .into_iter()
                    .map(|(_, t)| t)
                    .collect(),
            );
        }
        let snapshot = self.published.as_ref().map(|(v, at)| {
            let mut v = v.clone();
            if v.playing {
                v.position_ms = v
                    .position_ms
                    .saturating_add(at.elapsed().as_millis().min(u32::MAX as u128) as u32)
                    .min(if v.track.duration_ms > 0 {
                        v.track.duration_ms
                    } else {
                        86_400_000
                    });
            }
            v
        });
        (snapshot, self.published_queue.clone())
    }
    fn apply_control(&self, packet: &Value) -> bool {
        let Some(live) = self.live.as_ref().and_then(|s| s.upgrade()) else {
            return false;
        };
        let state = live.lock().unwrap();
        let command = match packet["action"].as_str() {
            Some("set_playing") => {
                let Some(current) = Self::from_player(&state) else {
                    return false;
                };
                if packet["uri"].as_str() != Some(&current.track.uri) {
                    return false;
                }
                let Some(playing) = packet["playing"].as_bool() else {
                    return false;
                };
                crate::player::PlayerCommand::SetPlaying(playing)
            }
            Some("seek") => {
                let Some(current) = Self::from_player(&state) else {
                    return false;
                };
                if packet["uri"].as_str() != Some(&current.track.uri) {
                    return false;
                }
                let Some(position) = packet["position_ms"].as_u64().filter(|p| *p <= 86_400_000)
                else {
                    return false;
                };
                crate::player::PlayerCommand::Seek((position as u32).min(
                    if current.track.duration_ms > 0 {
                        current.track.duration_ms
                    } else {
                        86_400_000
                    },
                ))
            }
            Some("play_queue") => {
                let Some(index) = packet["index"].as_u64().filter(|i| *i < 40) else {
                    return false;
                };
                let queue = Self::live_queue(&state);
                let Some((absolute, track)) = queue.get(index as usize) else {
                    return false;
                };
                if packet["uri"].as_str() != Some(&track.uri) {
                    return false;
                }
                crate::player::PlayerCommand::SkipTo(*absolute)
            }
            _ => return false,
        };
        drop(state);
        self.sender.as_ref().is_some_and(|sender| {
            sender
                .send(crate::backend::Command::Player(command))
                .is_ok()
        })
    }
    fn update_listeners(&mut self, listeners: usize) {
        let listeners = listeners.min(24);
        if listeners > self.listeners && self.notifications.len() < 8 {
            let joined = listeners - self.listeners;
            self.notifications.push(if joined == 1 {
                "A listener joined your Jam.".into()
            } else {
                format!("{joined} listeners joined your Jam.")
            });
        }
        self.listeners = listeners;
    }
}
pub struct Session {
    pub host: bool,
    pub shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Session {
    pub fn start_host() -> Self {
        Self::start(None)
    }
    pub fn join(invite: &str) -> Result<Self> {
        Ok(Self::start(Some(parse_invite(invite)?)))
    }
    fn start(remote: Option<String>) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            status: "Connecting to Jam…".into(),
            ..Default::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let session = Self {
            host: remote.is_none(),
            shared: shared.clone(),
            stop: stop.clone(),
        };
        std::thread::spawn(move || {
            if let Err(e) = run(remote, &shared, &stop) {
                let mut s = shared.lock().unwrap();
                s.status = e.to_string();
                s.connected = false;
            }
        });
        session
    }
    pub fn publish(&self, snapshot: Option<Snapshot>) {
        self.shared.lock().unwrap().published = snapshot.and_then(|mut s| {
            s.track = relay_track(s.track)?;
            Some((s, Instant::now()))
        });
    }
    pub fn publish_queue(&self, tracks: &[LocalTrack]) {
        self.shared.lock().unwrap().published_queue = tracks
            .iter()
            .take(40)
            .cloned()
            .filter_map(relay_track)
            .collect();
    }
    pub fn take(&self) -> Option<(Snapshot, Instant)> {
        self.shared.lock().unwrap().received.take()
    }
    pub fn take_additions(&self) -> Vec<LocalTrack> {
        std::mem::take(&mut self.shared.lock().unwrap().incoming)
    }
    pub fn take_notifications(&self) -> Vec<String> {
        std::mem::take(&mut self.shared.lock().unwrap().notifications)
    }
    fn request_control(&self, packet: Value) -> bool {
        let mut state = self.shared.lock().unwrap();
        if !state.connected {
            return false;
        }
        if self.host {
            return state.apply_control(&packet);
        }
        if !state.controls_available || state.controls.len() >= 16 {
            return false;
        }
        state.controls.push(packet);
        true
    }
    pub fn seek(&self, uri: &str, position_ms: u32) -> bool {
        self.request_control(
            json!({"type":"control","action":"seek","uri":uri,"position_ms":position_ms}),
        )
    }
    pub fn play_queue(&self, index: usize, uri: &str) -> bool {
        self.request_control(
            json!({"type":"control","action":"play_queue","uri":uri,"index":index}),
        )
    }
    pub fn add(&self, track: LocalTrack) -> bool {
        let Some(t) = relay_track(track) else {
            return false;
        };
        let mut s = self.shared.lock().unwrap();
        if !s.connected || s.outgoing.len() >= 20 {
            return false;
        }
        s.outgoing.push(t);
        true
    }
}
fn run(remote: Option<String>, shared: &Arc<Mutex<Shared>>, stop: &AtomicBool) -> Result<()> {
    let hosting = remote.is_none();
    let (code, token) = if let Some(code) = remote {
        (code, None)
    } else {
        let response = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(12))
            .redirect(reqwest::redirect::Policy::none())
            .build()?
            .post(format!("{RELAY}/rooms"))
            .send()
            .context("Jam service is unavailable. Try again.")?;
        ensure!(
            response.status().is_success(),
            "Cannot create Jam right now. Try again later."
        );
        let v: Value = response.json()?;
        let code = parse_invite(v["code"].as_str().context("Invalid Jam response")?)?;
        let token = v["host_token"]
            .as_str()
            .context("Invalid Jam credentials")?
            .to_owned();
        (code, Some(token))
    };
    shared.lock().unwrap().invite = format!("https://vibefy-neon.vercel.app/web?jam={code}");
    let mut failures = 0;
    while !stop.load(Ordering::Relaxed) {
        match connected(&code, token.as_deref(), shared, stop) {
            Ok(()) => return Ok(()),
            Err(e) => {
                failures += 1;
                let mut s = shared.lock().unwrap();
                s.connected = false;
                s.host_waiting = true;
                if failures >= 5 {
                    return Err(e.context("Jam disconnected. Leave and rejoin to retry"));
                }
                s.status = "Connection interrupted · reconnecting…".into();
                drop(s);
                for _ in 0..(10 * failures) {
                    if stop.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
    if hosting {
        shared.lock().unwrap().status = "Session ended".into();
    }
    Ok(())
}
fn connected(
    code: &str,
    token: Option<&str>,
    shared: &Arc<Mutex<Shared>>,
    stop: &AtomicBool,
) -> Result<()> {
    use tungstenite::{Error, Message, stream::MaybeTlsStream};
    let config = tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(64_000))
        .max_frame_size(Some(64_000));
    let (mut ws, _) =
        tungstenite::client::connect_with_config(format!("{SOCKET}{code}"), Some(config), 0)
            .context("Jam service is unreachable")?;
    let socket = match ws.get_mut() {
        MaybeTlsStream::Plain(s) => s,
        MaybeTlsStream::Rustls(s) => &mut s.sock,
        _ => anyhow::bail!("Unsupported Jam transport"),
    };
    socket.set_read_timeout(Some(Duration::from_millis(150)))?;
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;
    ws.send(Message::Text(
        json!({"type":"hello","protocol":PROTOCOL,"host_token":token,"name":"Vibefy","shared_controls":true})
            .to_string()
            .into(),
    ))?;
    let mut ready = false;
    let mut last = Instant::now() - Duration::from_secs(2);
    let mut ping = Instant::now();
    let mut seen = Instant::now();
    let mut rtt = 0u32;
    while !stop.load(Ordering::Relaxed) {
        if ready && last.elapsed() >= Duration::from_secs(1) {
            let (host_packet, addition, control) = {
                let mut s = shared.lock().unwrap();
                (
                    token.is_some().then(|| s.host_packet()),
                    if s.outgoing.is_empty() {
                        None
                    } else {
                        Some(s.outgoing.remove(0))
                    },
                    if s.controls.is_empty() {
                        None
                    } else {
                        Some(s.controls.remove(0))
                    },
                )
            };
            // Slow socket writes must never hold a lock used by the UI.
            if let Some((snapshot, mut queue)) = host_packet {
                let data = loop {
                    let data =
                        json!({"type":"state","snapshot":snapshot,"queue":queue}).to_string();
                    if data.len() <= 23_000 {
                        break data;
                    }
                    if queue.pop().is_none() {
                        anyhow::bail!("Jam metadata is too large");
                    }
                };
                ws.send(Message::Text(data.into()))?;
            }
            if let Some(track) = addition {
                ws.send(Message::Text(
                    json!({"type":"add","track":track}).to_string().into(),
                ))?;
            }
            if let Some(packet) = control {
                ws.send(Message::Text(packet.to_string().into()))?;
            }
            last = Instant::now();
        }
        if ready && ping.elapsed() >= Duration::from_secs(10) {
            ws.send(Message::Text(
                json!({"type":"ping","echo":1}).to_string().into(),
            ))?;
            ping = Instant::now();
        }
        match ws.read() {
            Ok(Message::Text(text)) => {
                seen = Instant::now();
                let p: Value = serde_json::from_str(&text)?;
                let mut s = shared.lock().unwrap();
                s.last_seen = Some(Instant::now());
                match p["type"].as_str() {
                    Some("welcome") => {
                        ready = true;
                        s.connected = true;
                        s.status = if token.is_some() {
                            "Hosting"
                        } else {
                            "Following host"
                        }
                        .into();
                    }
                    Some("pong") => {
                        rtt = (ping.elapsed().as_millis() / 2).min(1500) as u32;
                        s.latency_ms = rtt.saturating_mul(2);
                    }
                    Some("state") => {
                        s.update_listeners(p["listeners"].as_u64().unwrap_or(0) as usize);
                        s.controls_available = p["shared_controls"].as_bool().unwrap_or(false);
                        s.queue = serde_json::from_value::<Vec<LocalTrack>>(p["queue"].clone())
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(relay_track)
                            .collect();
                        if token.is_none() {
                            s.host_waiting = p["snapshot"].is_null();
                            if !s.host_waiting {
                                let mut v: Snapshot =
                                    serde_json::from_value(p["snapshot"].clone())?;
                                ensure!(v.valid(), "Invalid Jam track");
                                v.track = relay_track(v.track).context("Unsupported Jam track")?;
                                if v.playing {
                                    v.position_ms = v.position_ms.saturating_add(rtt);
                                }
                                s.received = Some((v, Instant::now()));
                            }
                        }
                    }
                    Some("add") if token.is_some() => {
                        if let Some(sender) = &s.sender {
                            if let Ok(t) = serde_json::from_value(p["track"].clone()) {
                                if let Some(t) = relay_track(t) {
                                    let _ = sender.send(crate::backend::Command::Player(
                                        crate::player::PlayerCommand::AddResolvedToQueue(t),
                                    ));
                                }
                            }
                            continue;
                        }
                        if s.incoming.len() < 100 {
                            if let Ok(t) = serde_json::from_value(p["track"].clone()) {
                                if let Some(t) = relay_track(t) {
                                    s.incoming.push(t);
                                }
                            }
                        }
                    }
                    Some("control") if token.is_some() => {
                        if s.apply_control(&p) {
                            last = Instant::now() - Duration::from_secs(1);
                        }
                    }
                    Some("error") | Some("notice") => {
                        s.status = p["message"]
                            .as_str()
                            .unwrap_or("Jam notification")
                            .chars()
                            .take(200)
                            .collect()
                    }
                    _ => {}
                }
            }
            Ok(Message::Close(_)) => {
                let mut s = shared.lock().unwrap();
                s.connected = false;
                s.host_waiting = true;
                s.status = "Jam ended or expired".into();
                return Ok(());
            }
            Ok(_) => {}
            Err(Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(e.into()),
        }
        ensure!(seen.elapsed() < Duration::from_secs(30), "Jam timed out");
    }
    if token.is_some() {
        let _ = ws.send(Message::Text(json!({"type":"end"}).to_string().into()));
    }
    let _ = ws.close(None);
    Ok(())
}

#[cfg(test)]
mod release_update_tests {
    use super::*;
    fn song(uri: &str) -> LocalTrack {
        LocalTrack {
            uri: uri.into(),
            title: "Synthetic Jam recording".into(),
            duration_ms: 240_000,
            ..Default::default()
        }
    }
    #[test]
    fn host_clock_and_track_changes_follow_the_engine_without_ui_frames() {
        let state = Arc::new(Mutex::new(crate::player::LocalState {
            connected: true,
            playback: crate::player::Playback::Playing,
            track: Some(song("vibefy:youtube:abcdefghijk")),
            position_ms: 1000,
            position_at: Some(Instant::now() - Duration::from_secs(75)),
            ..Default::default()
        }));
        let (sender, _) = tokio::sync::mpsc::unbounded_channel();
        let mut shared = Shared::default();
        shared.attach(Arc::downgrade(&state), sender);
        let snapshot = shared.host_packet().0.unwrap();
        assert!(snapshot.position_ms >= 76_000 && snapshot.position_ms < 78_000);
        {
            let mut local = state.lock().unwrap();
            local.track = Some(song(
                "vibefy:juicevault:12345678-1234-1234-1234-123456789abc",
            ));
            local.playback = crate::player::Playback::Paused;
            local.position_ms = 42_000;
        }
        let snapshot = shared.host_packet().0.unwrap();
        assert!(!snapshot.playing);
        assert_eq!(snapshot.position_ms, 42_000);
        assert!(snapshot.track.uri.starts_with("vibefy:juicevault:"));
        drop(state);
        assert!(shared.host_packet().0.is_none());
    }
    #[test]
    fn shared_controls_validate_recording_and_filtered_queue_before_dispatch() {
        let current = song("vibefy:youtube:abcdefghijk");
        let queued = song("vibefy:juicevault:12345678-1234-1234-1234-123456789abc");
        let state = Arc::new(Mutex::new(crate::player::LocalState {
            connected: true,
            playback: crate::player::Playback::Paused,
            track: Some(current.clone()),
            queue_tracks: vec![current.clone(), song("file:///private"), queued.clone()],
            ..Default::default()
        }));
        let (sender, mut received) = tokio::sync::mpsc::unbounded_channel();
        let mut shared = Shared::default();
        shared.attach(Arc::downgrade(&state), sender);
        assert!(
            !shared.apply_control(&json!({"action":"seek","uri":queued.uri,"position_ms":100}))
        );
        assert!(
            !shared.apply_control(&json!({"action":"seek","uri":current.uri,"position_ms":-1}))
        );
        assert!(
            shared.apply_control(&json!({"action":"seek","uri":current.uri,"position_ms":999999}))
        );
        assert!(matches!(
            received.try_recv().unwrap(),
            crate::backend::Command::Player(crate::player::PlayerCommand::Seek(240_000))
        ));
        assert!(!shared.apply_control(&json!({"action":"play_queue","index":0,"uri":current.uri})));
        assert!(shared.apply_control(&json!({"action":"play_queue","index":0,"uri":queued.uri})));
        assert!(matches!(
            received.try_recv().unwrap(),
            crate::backend::Command::Player(crate::player::PlayerCommand::SkipTo(2))
        ));
        assert!(received.try_recv().is_err());
    }
    #[test]
    fn shared_pause_is_an_explicit_boolean_and_rejects_stale_recordings() {
        let current = song("vibefy:youtube:abcdefghijk");
        let live = Arc::new(Mutex::new(crate::player::LocalState {
            connected: true,
            track: Some(current.clone()),
            playback: crate::player::Playback::Playing,
            ..Default::default()
        }));
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut shared = Shared::default();
        shared.attach(Arc::downgrade(&live), sender);
        for playing in [false, false, true] {
            assert!(shared.apply_control(
                &json!({"action":"set_playing","uri":current.uri,"playing":playing})
            ));
            assert!(
                matches!(receiver.try_recv().unwrap(), crate::backend::Command::Player(crate::player::PlayerCommand::SetPlaying(value)) if value == playing)
            );
        }
        assert!(!shared.apply_control(
            &json!({"action":"set_playing","uri":"vibefy:youtube:bbbbbbbbbbb","playing":false})
        ));
        assert!(
            !shared.apply_control(
                &json!({"action":"set_playing","uri":current.uri,"playing":"false"})
            )
        );
        assert!(receiver.try_recv().is_err());
        assert_eq!(
            crate::music_accounts::juice_search_query("Juice WRLD - Robbery"),
            "Robbery"
        );
        assert_eq!(
            crate::music_accounts::juice_search_query("Juice Man"),
            "Juice Man"
        );
    }
    #[test]
    fn participant_notifications_are_bounded_and_only_report_new_listeners() {
        let mut state = Shared::default();
        state.update_listeners(0);
        state.update_listeners(1);
        state.update_listeners(1);
        state.update_listeners(0);
        assert_eq!(state.notifications, ["A listener joined your Jam."]);
        for _ in 0..30 {
            state.update_listeners(0);
            state.update_listeners(usize::MAX);
        }
        assert_eq!(state.listeners, 24);
        assert_eq!(state.notifications.len(), 8);
    }
    #[test]
    fn codes_and_links_never_accept_peer_addresses_or_other_websites() {
        assert_eq!(parse_invite("abcd-efgh-jklm").unwrap(), "ABCDEFGHJKLM");
        assert_eq!(
            parse_invite("https://usevibefy.online/app?jam=ABCDEFGHJKLM").unwrap(),
            "ABCDEFGHJKLM"
        );
        assert_eq!(
            parse_invite("https://vibefy-neon.vercel.app/web?jam=ABCDEFGHJKLM").unwrap(),
            "ABCDEFGHJKLM"
        );
        for bad in [
            "https://usevibefy.online.evil/app?jam=ABCDEFGHJKLM",
            "https://evil@usevibefy.online/app?jam=ABCDEFGHJKLM",
            "http://usevibefy.online/app?jam=ABCDEFGHJKLM",
            "https://vibefy-neon.vercel.app.evil/web?jam=ABCDEFGHJKLM",
            "https://vibefy-neon.vercel.app:1234/web?jam=ABCDEFGHJKLM",
            "vibefy-jam://127.0.0.1:123/secret",
            "ABCD-EFGH-IJKL",
        ] {
            assert!(parse_invite(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn shared_tracks_drop_external_artwork_and_reject_unresolvable_sources() {
        let track = LocalTrack {
            uri: "vibefy:youtube:abcdefghijk".into(),
            title: "Example".into(),
            cover_url: Some("https://attacker.invalid/pixel".into()),
            ..Default::default()
        };
        assert!(relay_track(track.clone()).unwrap().cover_url.is_none());
        let cover = "https://api.juicevault.xyz/cdn/covers/example.webp";
        assert_eq!(
            relay_track(LocalTrack {
                cover_url: Some(cover.into()),
                ..track.clone()
            })
            .unwrap()
            .cover_url
            .as_deref(),
            Some(cover)
        );
        for bad in [
            "https://api.juicevault.xyz/cdn/cover.webp?token=private",
            "https://api.juicevault.xyz.evil/cdn/pixel.jpg",
            "http://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg",
            "https://user:secret@i.ytimg.com/vi/abcdefghijk/hqdefault.jpg",
        ] {
            assert!(public_cover_url(bad).is_none());
        }
        assert!(
            relay_track(LocalTrack {
                uri: "file:///secret".into(),
                ..track
            })
            .is_none()
        );
    }
    #[test]
    #[ignore = "Creates a temporary public relay room with synthetic metadata; requires network"]
    fn native_tls_relay_preserves_play_pause_and_shared_queue_between_clients() {
        fn wait(mut predicate: impl FnMut() -> bool) {
            let end = Instant::now() + Duration::from_secs(20);
            while !predicate() {
                assert!(Instant::now() < end, "Relay timed out");
                std::thread::sleep(Duration::from_millis(30));
            }
        }
        let host = Session::start_host();
        wait(|| host.shared.lock().unwrap().connected);
        let invite = host.shared.lock().unwrap().invite.clone();
        let track = LocalTrack {
            uri: "vibefy:youtube:abcdefghijk".into(),
            title: "Synthetic Jam test".into(),
            duration_ms: 200000,
            ..Default::default()
        };
        host.publish(Some(Snapshot {
            track: track.clone(),
            position_ms: 12000,
            playing: true,
        }));
        host.publish_queue(std::slice::from_ref(&track));
        let guest = Session::join(&invite).unwrap();
        let mut got = None;
        wait(|| {
            got = guest.take();
            got.as_ref()
                .is_some_and(|(s, _)| s.playing && s.track.uri == track.uri)
        });
        assert!(got.unwrap().0.position_ms >= 12000);
        wait(|| guest.shared.lock().unwrap().queue.len() == 1);
        assert!(guest.add(track.clone()));
        let mut additions = Vec::new();
        wait(|| {
            additions = host.take_additions();
            !additions.is_empty()
        });
        assert_eq!(additions[0].title, track.title);
        host.publish(Some(Snapshot {
            track,
            position_ms: 24000,
            playing: false,
        }));
        wait(|| {
            guest
                .take()
                .is_some_and(|(s, _)| !s.playing && s.position_ms == 24000)
        });
        drop(host);
        wait(|| !guest.shared.lock().unwrap().connected);
    }
}
