//! Shared metadata helpers for local playlists and YouTube backups.
use crate::{
    api::models::{Track, pick_image},
    player::LocalTrack,
};
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn valid_title(title: &str) -> bool {
    !title.trim().is_empty() && title.chars().count() <= 100 && !title.chars().any(char::is_control)
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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
pub fn counts(ids: impl IntoIterator<Item = String>) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    for id in ids {
        *result.entry(id).or_default() += 1;
    }
    result
}
/// High-confidence catalogue matches only. Ambiguous versions remain unmatched.
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
                && crate::audio_match::audio_edition_allowed(
                    &format!("{} {}", source.title, source.album),
                    &c.album,
                )
        })
        .map(|c| {
            let album = (!source.album.is_empty() && norm(&c.album) == norm(&source.album)) as i32;
            (
                album * 20
                    + (crate::audio_match::normalized(&c.album).contains("explicit") as i32 * 30)
                    - (source.duration_ms.abs_diff(c.duration_ms) / 1000) as i32,
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
