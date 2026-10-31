//! Sonos stub for Spotify Connect removal.

use crate::api::ApiError;
use crate::api::models::Device;
use crate::backend::RemoteAction;
use crate::zeroconf::Receiver;

pub struct PreviousDevice;

impl PreviousDevice {
    pub fn new() -> Self {
        Self
    }
    pub fn remember(&self, _: &[Receiver]) {}
}

pub fn is_sonos(_: &Receiver) -> bool {
    false
}

pub fn device_row(_: &Receiver, _: bool) -> Option<Device> {
    None
}

#[derive(Clone, Debug, Default)]
pub struct Controller;

impl Controller {
    pub fn new() -> Self {
        Self
    }
    pub fn restricted_target(&self, _: Option<&str>) -> Option<String> {
        None
    }
    pub fn is_restricted(&self, _: &str) -> bool {
        false
    }
    pub fn observe_devices(&self, _: &[Device]) {}
    pub fn observe_playing(&self, _: u64, _: Option<&Device>) {}
    pub fn remember(&self, _: &[Receiver]) {}
    pub async fn transfer(
        &self,
        _: &crate::session_reads::Session,
        _: &str,
    ) -> Result<(), ApiError> {
        Ok(())
    }
    pub async fn add_to_queue(
        &self,
        _: &crate::session_reads::Session,
        _: &str,
        uris: &[String],
    ) -> (usize, Result<(), ApiError>) {
        (uris.len(), Ok(()))
    }
}

pub fn command_body(
    _: RemoteAction,
    _: Option<&crate::api::PlayRequest>,
    _: u32,
    _: bool,
    _: &str,
) -> Option<serde_json::Value> {
    None
}

pub async fn command(
    _: &crate::session_reads::Session,
    _: &str,
    _: &serde_json::Value,
) -> Result<(), ApiError> {
    Ok(())
}

pub async fn set_volume(_: &crate::session_reads::Session, _: &str, _: u8) -> Result<(), ApiError> {
    Ok(())
}
