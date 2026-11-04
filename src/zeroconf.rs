//! Zeroconf discovery stub for Spotify Connect removal.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receiver {
    pub name: String,
    pub address: std::net::IpAddr,
    pub port: u16,
    pub device_id: Option<String>,
    pub model: Option<String>,
    pub path: Option<String>,
    pub cast: bool,
}

impl Receiver {
    pub fn preference_key(&self) -> Option<String> {
        if self.cast {
            // Cast IDs may be derived from a shared advertised name. Keep
            // preferences tied to the actual endpoint, not another speaker.
            return Some(format!("cast:{}:{}", self.address, self.port));
        }
        self.device_id.clone().or_else(|| Some(self.name.clone()))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ReceiverUpdates {
    pub added: Vec<Receiver>,
    pub removed: Vec<String>,
}

impl ReceiverUpdates {
    pub fn publish(&self, _receivers: Vec<Receiver>) -> bool {
        false
    }

    pub fn take(&self) -> Option<Vec<Receiver>> {
        None
    }
}

pub fn merge_discovered(_receivers: &mut Vec<Receiver>, _updates: Vec<Receiver>) {}

pub fn discover_with_updates<F>(
    _timeout: std::time::Duration,
    mut _cb: F,
) -> anyhow::Result<Vec<Receiver>>
where
    F: FnMut(Vec<Receiver>),
{
    Ok(Vec::new())
}

pub fn resolve_receivers(receivers: Vec<Receiver>) -> anyhow::Result<Vec<Receiver>> {
    Ok(receivers)
}

pub fn get_info(_: &reqwest::blocking::Client, _: &Receiver) -> anyhow::Result<String> {
    Ok(String::new())
}

pub fn add_user(
    _: &reqwest::blocking::Client,
    _: &Receiver,
    _: &str,
    _: &Credentials,
    _: &str,
) -> anyhow::Result<()> {
    Ok(())
}

pub struct Credentials;

impl Credentials {
    pub fn from_playback(_: &crate::credentials::Credentials) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
