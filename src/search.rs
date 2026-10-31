//! Search results that can be shown before a network round trip.
use crate::api::models::{Page, SearchResults, Track};

pub const MUSIC_TYPES: &[&str] = &["track", "artist", "album"];
pub const EXTRA_TYPES: &[&str] = &["playlist", "show", "episode"];
pub const DEADLINE: std::time::Duration = std::time::Duration::from_secs(8);

/// Search existing account metadata. No disk reads, downloads or invented IDs.
pub fn local_results<'a>(query: &str, tracks: impl Iterator<Item = &'a Track>) -> SearchResults {
    let query = crate::audio_match::normalized(query);
    if query.is_empty() {
        return SearchResults::default();
    }
    let words: Vec<_> = query.split_whitespace().collect();
    let mut seen = std::collections::HashSet::new();
    let mut matches = Vec::new();
    for track in tracks {
        if !track.uri.starts_with("spotify:track:")
            || track.name.is_empty()
            || !seen.insert(track.uri.as_str())
        {
            continue;
        }
        let title = crate::audio_match::normalized(&track.name);
        let mut text = title.clone();
        for artist in &track.artists {
            text.push(' ');
            text.push_str(&crate::audio_match::normalized(&artist.name));
        }
        if words.iter().all(|word| text.contains(word)) {
            let score = if title == query {
                0
            } else if title.starts_with(&query) {
                1
            } else {
                2
            };
            matches.push((score, track));
        }
    }
    matches.sort_unstable_by(|(sa, a), (sb, b)| {
        sa.cmp(sb)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.uri.cmp(&b.uri))
    });
    let items: Vec<_> = matches
        .into_iter()
        .take(50)
        .map(|(_, track)| track.clone())
        .collect();
    if items.is_empty() {
        return SearchResults::default();
    }
    SearchResults {
        tracks: Some(Page {
            total: items.len() as u32,
            items,
            ..Default::default()
        }),
        ..Default::default()
    }
}
