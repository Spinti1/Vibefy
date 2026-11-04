//! Spotify Session reads stub.
//!
//! Librespot session reads are removed in favor of direct Spotify Web API requests.

use crate::api::client::ApiError;
use crate::api::models::{Album, Artist, Playlist, Track, User};
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct Session;

impl Session {
    pub fn username(&self) -> String {
        "user".to_string()
    }
    pub fn country(&self) -> Option<String> {
        None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Failure {
    #[error("definitive: {0}")]
    Definitive(ApiError),
    #[error("retry: {0}")]
    Retry(anyhow::Error),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemovedRow {
    pub uri: String,
    pub position: u32,
    pub copy: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FriendActivity {
    pub user_uri: String,
    pub user_name: String,
    pub image_url: Option<String>,
    pub timestamp_ms: i64,
    pub playing: bool,
    pub track_uri: String,
    pub track_name: String,
    pub artist_name: String,
    pub artist_uri: Option<String>,
    pub album_name: String,
    pub album_uri: Option<String>,
    pub context_name: Option<String>,
    pub context_uri: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct FriendNames {
    pub profiles: HashMap<String, User>,
    pub playlists: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct UserProfile {
    pub name: Option<String>,
    pub image_url: Option<String>,
}

pub fn seed_first<T>(_items: &[T]) -> Option<&T> {
    None
}

pub fn normalize_local_track_labels(track: &mut Track) {
    let Some(parts) = track.uri.strip_prefix("spotify:local:") else {
        return;
    };
    let parts: Vec<_> = parts.split(':').collect();
    if parts.len() != 4 {
        return;
    }
    fn restore(label: &mut String, encoded: &str) {
        // Only repair labels copied verbatim from the local URI. Real tags
        // may legitimately contain a plus or percent sign.
        if label == encoded {
            let spaces = encoded.replace('+', " ");
            if let Ok(decoded) = percent_encoding::percent_decode_str(&spaces).decode_utf8() {
                *label = decoded.into_owned();
            }
        }
    }
    restore(&mut track.name, parts[2]);
    if let Some(artist) = track.artists.first_mut() {
        restore(&mut artist.name, parts[0]);
    }
    if let Some(album) = track.album.as_mut() {
        restore(&mut album.name, parts[1]);
    }
}

#[cfg(test)]
mod local_label_tests {
    use super::*;

    #[test]
    fn normalization_preserves_real_tags_and_invalid_utf8() {
        let mut track = Track {
            uri: "spotify:local:Artist:Album:Encoded+Title:180".into(),
            name: "Actual + title %20".into(),
            ..Default::default()
        };
        normalize_local_track_labels(&mut track);
        assert_eq!(track.name, "Actual + title %20");
        track.uri = "spotify:local:Artist:Album:%FF:180".into();
        track.name = "%FF".into();
        normalize_local_track_labels(&mut track);
        assert_eq!(track.name, "%FF");
    }
}

pub async fn playlist(_: &Session, _: &str) -> Result<Playlist, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn station(_: &Session, _: &str) -> Result<Vec<Track>, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn remove_playlist_rows(
    _: &Session,
    _: &str,
    _: &[RemovedRow],
) -> Result<Option<String>, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn set_prerelease_saved(
    _: &Session,
    _: &reqwest::Client,
    _: &str,
    _: bool,
) -> Result<(), Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn prerelease(
    _: &Session,
    _: &crate::api::client::ApiClient,
    _: &str,
) -> Result<crate::prerelease::Prerelease, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn user_profile(_: &Session, _: &str) -> Option<UserProfile> {
    None
}

pub async fn play_counts(
    _: &Session,
    _: &crate::api::client::ApiClient,
    _: &str,
) -> Result<HashMap<String, u64>, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn friend_activity(
    _: &Session,
    _: &tokio::sync::Mutex<FriendNames>,
) -> Result<Vec<FriendActivity>, Failure> {
    Ok(Vec::new())
}

pub async fn audiobook_shows(_: &Session, _: &[String]) -> Result<Vec<String>, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn album_tracks(_: &Session, _: &[Album]) -> Result<Vec<Track>, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn hidden_songs(_: &Session) -> Result<Vec<(String, String)>, Failure> {
    Ok(Vec::new())
}

pub async fn set_hidden(_: &Session, _: &str, _: &str, _: bool, _: i64) -> Result<(), Failure> {
    Ok(())
}

pub async fn items(_: &Session, _: &str, _: u32, _: u32) -> Result<Vec<Track>, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn sample(_: &Session, _: &str, _: u32, _: u32) -> Result<Vec<Track>, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn artist_top_tracks(_: &Session, _: &str) -> Result<Vec<Track>, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn related_artists(_: &Session, _: (), _: &str) -> Result<Vec<Artist>, Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}

pub async fn library(_: &Session) -> Result<(), Failure> {
    Err(Failure::Retry(anyhow::anyhow!("Session reads disabled")))
}
