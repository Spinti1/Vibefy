//! Short, on-demand LAN discovery. Playback still uses the authenticated Jam relay.
//! Discovery never sends account credentials or audio to a LAN peer.
use anyhow::{Result, ensure};
use std::{
    io::{Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
const SERVICE: &str = "_vibefy._tcp.local.";
#[derive(Clone, Debug)]
pub struct Device {
    pub name: String,
    pub address: SocketAddr,
    token: String,
}
#[derive(Clone, Default)]
pub struct State {
    pub devices: Vec<Device>,
    pub searching: bool,
    pub pairing: bool,
    pub message: String,
    checked: Option<Instant>,
}
fn state() -> &'static Arc<Mutex<State>> {
    static STATE: OnceLock<Arc<Mutex<State>>> = OnceLock::new();
    STATE.get_or_init(|| Arc::new(Mutex::new(State::default())))
}
fn local(ip: IpAddr) -> bool {
    matches!(ip, IpAddr::V4(v) if v.is_private() || v.is_link_local())
}
fn valid_token(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|c| c.is_ascii_hexdigit())
}
pub fn snapshot() -> State {
    state().lock().unwrap().clone()
}
pub fn discover() {
    let mut s = state().lock().unwrap();
    if s.searching
        || s.checked
            .is_some_and(|t| t.elapsed() < Duration::from_secs(20))
    {
        return;
    }
    s.searching = true;
    s.checked = Some(Instant::now());
    s.devices.clear();
    drop(s);
    std::thread::spawn(|| {
        let result = (|| -> Result<()> {
            let mdns = mdns_sd::ServiceDaemon::new()?;
            let result = (|| -> Result<()> {
                let receiver = mdns.browse(SERVICE)?;
                let start = Instant::now();
                while start.elapsed() < Duration::from_secs(6) {
                    if let Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) =
                        receiver.recv_timeout(Duration::from_millis(250))
                    {
                        let token = info.get_property_val_str("token").unwrap_or_default();
                        if info.get_property_val_str("v") != Some("1")
                            || !valid_token(token)
                            || info.port == 0
                        {
                            continue;
                        }
                        let Some(ip) = info
                            .addresses
                            .iter()
                            .map(|a| a.to_ip_addr())
                            .find(|ip| local(*ip))
                        else {
                            continue;
                        };
                        let name = match info.get_property_val_str("platform") {
                            Some("ios") => "Vibefy iPhone / iPad",
                            Some("android") => "Vibefy Android",
                            _ => continue,
                        };
                        let mut s = state().lock().unwrap();
                        if s.devices.len() < 24 && !s.devices.iter().any(|d| d.token == token) {
                            s.devices.push(Device {
                                name: name.into(),
                                address: SocketAddr::new(ip, info.port),
                                token: token.into(),
                            });
                        }
                    }
                }
                Ok(())
            })();
            let _ = mdns.stop_browse(SERVICE);
            let _ = mdns.shutdown();
            result
        })();
        let mut s = state().lock().unwrap();
        s.searching = false;
        if result.is_err() {
            s.message = "Local discovery unavailable. Use a Jam invitation instead.".into();
        }
    });
}
pub fn pair(device: Device, shared: Arc<Mutex<crate::jam::Shared>>) {
    let mut s = state().lock().unwrap();
    if s.pairing {
        return;
    }
    s.pairing = true;
    s.message = "Preparing connection…".into();
    drop(s);
    std::thread::spawn(move || {
        let result = (|| -> Result<()> {
            ensure!(
                local(device.address.ip()) && valid_token(&device.token),
                "Invalid nearby device"
            );
            let start = Instant::now();
            let code = loop {
                let s = shared.lock().unwrap();
                if s.connected
                    && let Ok(code) = crate::jam::parse_invite(&s.invite)
                {
                    break code;
                }
                drop(s);
                ensure!(
                    start.elapsed() < Duration::from_secs(15),
                    "Jam did not connect. Try again."
                );
                std::thread::sleep(Duration::from_millis(200));
            };
            let mut stream = TcpStream::connect_timeout(&device.address, Duration::from_secs(3))?;
            stream.set_read_timeout(Some(Duration::from_secs(3)))?;
            stream.set_write_timeout(Some(Duration::from_secs(3)))?;
            stream.write_all(format!("VIBEFY_PAIR_V1 {} {}\n", device.token, code).as_bytes())?;
            let mut response = [0u8; 6];
            stream.read_exact(&mut response)?;
            ensure!(
                &response == b"READY\n",
                "Phone could not accept the invitation"
            );
            Ok(())
        })();
        let mut s = state().lock().unwrap();
        s.pairing = false;
        s.message = if result.is_ok() {
            "Confirm Connect on your phone to start shared playback.".into()
        } else {
            "Phone unavailable. Keep Vibefy open on the same Wi-Fi and try again.".into()
        };
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restrict_discovery_to_private_lan_addresses() {
        for ip in ["127.0.0.1", "0.0.0.0", "8.8.8.8", "224.0.0.251", "::1"] {
            assert!(!local(ip.parse().unwrap()));
        }
        assert!(local("192.168.1.5".parse().unwrap()));
        assert!(local("10.0.0.8".parse().unwrap()));
    }
    #[test]
    fn strict_pairing_token() {
        assert!(valid_token("0123456789abcdef0123456789abcdef"));
        for token in [
            "",
            "short",
            "0123456789abcdef0123456789abcde\n",
            "0123456789abcdef0123456789abcdeg",
        ] {
            assert!(!valid_token(token));
        }
    }
}
