//! Private, direct Vibefy sessions. Metadata and timing only, never audio or account tokens.
//! TCP is intended for a trusted LAN or VPN. There is no public relay or telemetry.
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::player::LocalTrack;

const MAX_PACKET: usize = 24_000;
pub const PROTOCOL: &str = "vibefy-jam-1";

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

#[derive(Serialize, Deserialize)]
enum Packet {
    Hello {
        protocol: String,
        token: String,
    },
    Poll(u64),
    State {
        echo: u64,
        snapshot: Option<Snapshot>,
    },
    Welcome,
}

pub fn parse_invite(text: &str) -> Result<(SocketAddr, String)> {
    let value = text
        .trim()
        .strip_prefix("vibefy-jam://")
        .context("Paste a Vibefy invitation")?;
    let (address, token) = value.split_once('/').context("Incomplete invitation")?;
    let address: SocketAddr = address
        .parse()
        .context("Invitation must contain an IP address and port")?;
    ensure!(
        address.port() > 0 && !address.ip().is_unspecified() && !address.ip().is_multicast(),
        "Invalid host address"
    );
    ensure!(
        token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid session token"
    );
    Ok((address, token.to_owned()))
}

struct Peer {
    stream: TcpStream,
    input: Vec<u8>,
    output: Vec<u8>,
    cursor: usize,
    authenticated: bool,
    last: Instant,
}
impl Peer {
    fn new(stream: TcpStream) -> std::io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream,
            input: Vec::new(),
            output: Vec::new(),
            cursor: 0,
            authenticated: false,
            last: Instant::now(),
        })
    }
    fn queue(&mut self, packet: &Packet) -> Result<()> {
        if self.cursor > 0 {
            self.output.drain(..self.cursor);
            self.cursor = 0;
        }
        ensure!(
            self.output.len().saturating_sub(self.cursor) < MAX_PACKET * 2,
            "Slow listener"
        );
        let mut bytes = serde_json::to_vec(packet)?;
        ensure!(bytes.len() < MAX_PACKET, "Packet too large");
        bytes.push(b'\n');
        self.output.extend(bytes);
        Ok(())
    }
    fn step(&mut self) -> Result<Vec<Packet>> {
        while self.cursor < self.output.len() {
            match self.stream.write(&self.output[self.cursor..]) {
                Ok(0) => anyhow::bail!("Connection closed"),
                Ok(n) => self.cursor += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        if self.cursor == self.output.len() {
            self.output.clear();
            self.cursor = 0;
        }
        let mut buffer = [0u8; 4096];
        loop {
            match self.stream.read(&mut buffer) {
                Ok(0) => anyhow::bail!("Connection closed"),
                Ok(n) => {
                    self.input.extend_from_slice(&buffer[..n]);
                    ensure!(self.input.len() <= MAX_PACKET * 2, "Packet too large");
                    self.last = Instant::now();
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        let mut packets = Vec::new();
        while let Some(end) = self.input.iter().position(|b| *b == b'\n') {
            ensure!(end < MAX_PACKET && packets.len() < 12, "Too many messages");
            let packet = serde_json::from_slice(&self.input[..end])?;
            self.input.drain(..=end);
            packets.push(packet);
        }
        ensure!(
            self.last.elapsed() < Duration::from_secs(if self.authenticated { 15 } else { 5 }),
            "Session timed out"
        );
        Ok(packets)
    }
}

#[derive(Default)]
pub struct Shared {
    pub connected: bool,
    pub host_waiting: bool,
    pub status: String,
    pub invite: String,
    pub listeners: usize,
    published: Option<(Snapshot, Instant)>,
    received: Option<(Snapshot, Instant)>,
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
        Self::start(None, "0.0.0.0:0".parse().unwrap())
    }
    #[cfg(feature = "demo")]
    pub fn loopback_host() -> Self {
        Self::start(None, "127.0.0.1:0".parse().unwrap())
    }
    pub fn join(invite: &str) -> Result<Self> {
        let peer = parse_invite(invite)?;
        Ok(Self::start(Some(peer), "0.0.0.0:0".parse().unwrap()))
    }
    fn start(remote: Option<(SocketAddr, String)>, bind: SocketAddr) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            status: "Connecting…".into(),
            ..Default::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let session = Self {
            host: remote.is_none(),
            shared: shared.clone(),
            stop: stop.clone(),
        };
        std::thread::spawn(move || {
            let result = if let Some((address, token)) = remote {
                guest(address, token, &shared, &stop)
            } else {
                host(&shared, &stop, bind)
            };
            if let Err(error) = result {
                let mut state = shared.lock().unwrap();
                state.status = error.to_string();
                state.connected = false;
            }
        });
        session
    }
    pub fn publish(&self, snapshot: Option<Snapshot>) {
        self.shared.lock().unwrap().published = snapshot.map(|s| (s, Instant::now()));
    }
    pub fn take(&self) -> Option<(Snapshot, Instant)> {
        self.shared.lock().unwrap().received.take()
    }
}

fn host(shared: &Arc<Mutex<Shared>>, stop: &AtomicBool, bind: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(bind).context("Could not open a session port")?;
    listener.set_nonblocking(true)?;
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
    let ip = socket
        .connect("192.0.2.1:9")
        .and_then(|_| socket.local_addr())
        .map(|s| s.ip())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    let token = rand::random::<[u8; 16]>()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    {
        let mut state = shared.lock().unwrap();
        state.invite = format!(
            "vibefy-jam://{ip}:{}/{token}",
            listener.local_addr()?.port()
        );
        state.status = "Hosting on this computer".into();
    }
    let mut peers = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        // Bound accepts so a busy port cannot starve existing listeners.
        for _ in 0..8 {
            match listener.accept() {
                Ok((stream, _)) if peers.len() < 8 => {
                    if let Ok(peer) = Peer::new(stream) {
                        peers.push(peer);
                    }
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        peers.retain_mut(|peer| {
            let mut process = || -> Result<()> {
                for packet in peer.step()? {
                    match packet {
                        Packet::Hello {
                            protocol,
                            token: supplied,
                        } if !peer.authenticated => {
                            ensure!(
                                protocol == PROTOCOL && supplied == token,
                                "Invalid invitation"
                            );
                            peer.authenticated = true;
                            peer.queue(&Packet::Welcome)?;
                        }
                        Packet::Poll(echo) if peer.authenticated => {
                            let snapshot =
                                shared.lock().unwrap().published.as_ref().map(|(s, at)| {
                                    let mut s = s.clone();
                                    if s.playing {
                                        s.position_ms = s.position_ms.saturating_add(
                                            at.elapsed().as_millis().min(10_000) as u32,
                                        );
                                    }
                                    s
                                });
                            peer.queue(&Packet::State { echo, snapshot })?;
                        }
                        _ => anyhow::bail!("Unsupported message"),
                    }
                }
                Ok(())
            };
            process().is_ok()
        });
        shared.lock().unwrap().listeners = peers.iter().filter(|p| p.authenticated).count();
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn guest(
    address: SocketAddr,
    token: String,
    shared: &Arc<Mutex<Shared>>,
    stop: &AtomicBool,
) -> Result<()> {
    let stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
        .context("Host unreachable. Check LAN/VPN and the host firewall")?;
    let mut peer = Peer::new(stream)?;
    peer.queue(&Packet::Hello {
        protocol: PROTOCOL.into(),
        token,
    })?;
    let started = Instant::now();
    let mut last_poll = Instant::now() - Duration::from_secs(1);
    let mut serial = 0;
    while !stop.load(Ordering::Relaxed) {
        if peer.authenticated && last_poll.elapsed() >= Duration::from_secs(1) {
            serial = started.elapsed().as_millis() as u64;
            last_poll = Instant::now();
            peer.queue(&Packet::Poll(serial))?;
        }
        for packet in peer.step()? {
            match packet {
                Packet::Welcome => {
                    peer.authenticated = true;
                    let mut state = shared.lock().unwrap();
                    state.status = "Connected to host".into();
                    state.connected = true;
                }
                Packet::State {
                    echo,
                    snapshot: Some(mut snapshot),
                } if peer.authenticated && echo == serial => {
                    ensure!(snapshot.valid(), "Host sent unsupported track metadata");
                    if snapshot.playing {
                        snapshot.position_ms = snapshot
                            .position_ms
                            .saturating_add((last_poll.elapsed().as_millis() / 2).min(1500) as u32);
                    }
                    let mut state = shared.lock().unwrap();
                    state.received = Some((snapshot, Instant::now()));
                    state.host_waiting = false;
                }
                Packet::State { snapshot: None, .. } if peer.authenticated => {
                    shared.lock().unwrap().host_waiting = true;
                }
                Packet::State { .. } if peer.authenticated => {}
                _ => anyhow::bail!("Unsupported host protocol"),
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}
