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
                && url.host_str() == Some("usevibefy.online")
                && url.username().is_empty()
                && url.password().is_none()
                && url.port().is_none()
                && url.path() == "/app",
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
fn relay_track(mut t: LocalTrack) -> Option<LocalTrack> {
    if !t
        .uri
        .strip_prefix("vibefy:youtube:")
        .is_some_and(crate::addons::valid_video_id)
        || t.title.trim().is_empty()
        || t.title.chars().count() > 300
        || t.artists.len() > 12
    {
        return None;
    }
    t.cover_url = None;
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
    shared.lock().unwrap().invite = format!("https://usevibefy.online/app?jam={code}");
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
        json!({"type":"hello","protocol":PROTOCOL,"host_token":token,"name":"Vibefy"})
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
            let mut s = shared.lock().unwrap();
            if token.is_some() {
                let snapshot = s.published.as_ref().map(|(v, at)| {
                    let mut v = v.clone();
                    if v.playing {
                        v.position_ms = v
                            .position_ms
                            .saturating_add(at.elapsed().as_millis().min(10_000) as u32)
                            .min(86_400_000);
                    }
                    v
                });
                let mut queue = s.published_queue.clone();
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
            if !s.outgoing.is_empty() {
                let track = s.outgoing.remove(0);
                ws.send(Message::Text(
                    json!({"type":"add","track":track}).to_string().into(),
                ))?;
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
                    Some("pong") => rtt = (ping.elapsed().as_millis() / 2).min(1500) as u32,
                    Some("state") => {
                        s.listeners = p["listeners"].as_u64().unwrap_or(0) as usize;
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
                        if s.incoming.len() < 100 {
                            if let Ok(t) = serde_json::from_value(p["track"].clone()) {
                                if let Some(t) = relay_track(t) {
                                    s.incoming.push(t);
                                }
                            }
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
    #[test]
    fn codes_and_links_never_accept_peer_addresses_or_other_websites() {
        assert_eq!(parse_invite("abcd-efgh-jklm").unwrap(), "ABCDEFGHJKLM");
        assert_eq!(
            parse_invite("https://usevibefy.online/app?jam=ABCDEFGHJKLM").unwrap(),
            "ABCDEFGHJKLM"
        );
        for bad in [
            "https://usevibefy.online.evil/app?jam=ABCDEFGHJKLM",
            "https://evil@usevibefy.online/app?jam=ABCDEFGHJKLM",
            "http://usevibefy.online/app?jam=ABCDEFGHJKLM",
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
