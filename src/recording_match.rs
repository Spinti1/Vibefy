//! Spotify-to-YouTube identity checks. A title containing an artist's name is
//! not proof of an artist upload; search results must be inspected in full.
use serde::{Deserialize, Serialize};

use crate::audio_match::{clean_title, compatible_version, normalized};

#[derive(Clone, Debug)]
pub struct Reference {
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_secs: u32,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Recording {
    pub id: String,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub channel: Option<String>,
    pub channel_id: Option<String>,
    pub channel_is_verified: Option<bool>,
    pub duration: Option<f64>,
    pub track: Option<String>,
    pub artists: Option<Vec<String>>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub description: Option<String>,
    pub is_live: Option<bool>,
    pub live_status: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    DistributedAudio,
    VerifiedArtist,
    ArtistChannel,
    VerifiedPublisher,
    CreditedPublisher,
    /// Identity matches, but the upload does not supply publisher evidence.
    MetadataMatch,
}

fn official_video(title: &str) -> bool {
    let title = normalized(title);
    [
        "official video",
        "official music video",
        "videoclip oficial",
        "video oficial",
    ]
    .iter()
    .any(|marker| title.contains(marker))
}

/// Longer official videos may contain a cinematic intro/outro. This allowance
/// never authorizes a short cut, a fan upload, or an arbitrarily longer mix.
pub fn duration_compatible(expected: u32, source: u32, title: &str, verified_video: bool) -> bool {
    crate::audio_match::duration_matches(expected, source)
        || (expected >= 90
            && source > expected
            && verified_video
            && official_video(title)
            && source - expected <= 45.min(expected / 4))
}

#[derive(Clone, Debug)]
pub struct Match {
    pub score: i32,
    pub source: SourceKind,
}

fn contains_name(text: &str, name: &str) -> bool {
    let name = normalized(name);
    !name.is_empty() && format!(" {} ", normalized(text)).contains(&format!(" {name} "))
}

fn artist_channel(channel: &str, artist: &str) -> bool {
    let name = normalized(artist).replace(' ', "");
    let channel = normalized(channel).replace(' ', "");
    !name.is_empty()
        && ["", "topic", "vevo", "official", "officialmusic", "music"]
            .iter()
            .any(|suffix| channel == format!("{name}{suffix}"))
}

fn presentation(tag: &str) -> bool {
    let tag = normalized(tag);
    !tag.is_empty()
        && tag.split_whitespace().all(|word| {
            [
                "official",
                "audio",
                "video",
                "music",
                "lyric",
                "lyrics",
                "hd",
                "hq",
                "4k",
                "visualizer",
                "visualiser",
                "versuri",
                "videoclip",
                "oficial",
                "explicit",
                "uncensored",
                "version",
            ]
            .contains(&word)
        })
}

fn strip_presentation(text: &str) -> String {
    let mut text = text.trim().to_string();
    loop {
        let before = text.clone();
        for (open, close) in [('(', ')'), ('[', ']')] {
            if text.ends_with(close)
                && let Some(index) = text.rfind(open)
                && presentation(&text[index + 1..text.len() - 1])
            {
                text.truncate(index);
            }
        }
        for separator in [" | ", " - "] {
            if let Some((base, tag)) = text.rsplit_once(separator)
                && presentation(tag)
                && !base.trim().is_empty()
            {
                text = base.to_string();
            }
        }
        for suffix in [
            " official music video",
            " official audio",
            " official video",
            " lyrics",
            " lyric video",
            " (audio)",
            " hq",
            " hd",
            " versuri",
            " visualizer",
        ] {
            if text.to_lowercase().ends_with(suffix) && text.len() > suffix.len() {
                text.truncate(text.len() - suffix.len());
            }
        }
        text = text.trim().to_string();
        if text == before {
            return text;
        }
    }
}

fn search_title(title: &str) -> String {
    let mut title = strip_presentation(&clean_title(title));
    for suffix in [
        " - us version",
        " - uk version",
        " - international version",
        " (us version)",
        " (uk version)",
        " - explicit version",
        " - album version",
        " - single version",
        " - original version",
    ] {
        if title.to_lowercase().ends_with(suffix) && title.len() > suffix.len() {
            title.truncate(title.len() - suffix.len());
        }
    }
    title.trim().to_string()
}

/// Remove presentation and an identifiable artist prefix, never arbitrary song
/// words. In particular, "Love" != "Love Story" and "Song 2" != "Song".
pub fn title_key(title: &str, artists: &[String]) -> String {
    let mut title = search_title(title);
    for (index, character) in title.char_indices() {
        if matches!(character, '-' | '–' | '—' | ':') {
            let prefix = &title[..index];
            if artists.iter().any(|artist| contains_name(prefix, artist)) {
                title = title[index + character.len_utf8()..].trim().to_string();
                break;
            }
        }
    }
    let mut key = normalized(&search_title(&title));
    // Also recognize "Song - Artist" and "Artist Song" without requiring
    // publishers to use one punctuation convention.
    for artist in artists {
        let artist = normalized(artist);
        if artist.is_empty() {
            continue;
        }
        if let Some(rest) = key.strip_prefix(&format!("{artist} ")) {
            key = rest.trim().to_string();
        } else if let Some(rest) = key.strip_suffix(&format!(" {artist}")) {
            key = rest.trim().to_string();
        }
    }
    key
}

fn artist_in_title(recording: &Recording, artist: &str) -> bool {
    let title = recording.title.as_deref().unwrap_or("");
    // Only credit-like prefixes count. A song title containing "Ian" must not
    // accidentally identify the unrelated performer "Cristian".
    title.char_indices().any(|(index, c)| {
        matches!(c, '-' | '–' | '—' | ':') && contains_name(&title[..index], artist)
    }) || {
        let title = normalized(title);
        let artist = normalized(artist);
        !artist.is_empty()
            && (title.starts_with(&format!("{artist} ")) || title.ends_with(&format!(" {artist}")))
    } || [" feat ", " ft ", " featuring ", " with "]
        .iter()
        .any(|marker| {
            format!(" {} ", normalized(title))
                .split_once(marker)
                .is_some_and(|(_, credits)| contains_name(credits, artist))
        })
}

fn candidate_title_key(recording: &Recording, reference: &Reference) -> String {
    let bare = title_key(recording.title.as_deref().unwrap_or(""), &[]);
    if crate::title_similarity::score(&title_key(&reference.title, &[]), &bare).is_some() {
        return bare;
    }
    let mut prefixes = reference.artists.clone();
    if let Some(channel) = recording.channel.as_ref().or(recording.uploader.as_ref()) {
        prefixes.push(channel.clone());
    }
    if reference
        .artists
        .first()
        .is_some_and(|artist| artist_in_title(recording, artist))
    {
        let title = recording.title.as_deref().unwrap_or("");
        for (index, c) in title.char_indices() {
            if matches!(c, '-' | '–' | '—' | ':') {
                let tail = title_key(&title[index + c.len_utf8()..], &prefixes);
                if crate::title_similarity::score(&title_key(&reference.title, &[]), &tail)
                    .is_some()
                {
                    return tail;
                }
            }
        }
    }
    title_key(recording.title.as_deref().unwrap_or(""), &prefixes)
}

fn publisher_credits(reference: &Reference, recording: &Recording) -> bool {
    let channel = recording
        .channel
        .as_deref()
        .or(recording.uploader.as_deref())
        .unwrap_or("");
    let title = recording.title.as_deref().unwrap_or("");
    let prefix_matches = title.char_indices().any(|(index, c)| {
        matches!(c, '-' | '–' | '—' | ':') && normalized(&title[..index]) == normalized(channel)
    });
    let description = recording.description.as_deref().unwrap_or("");
    let rights_match = description.lines().any(|line| {
        (line.contains('©') || line.contains('℗') || line.contains("(C)") || line.contains("(P)"))
            && contains_name(line, channel)
    });
    let performers_match = description.lines().any(|line| {
        let line_norm = normalized(line);
        [
            "written by",
            "performed by",
            "voce",
            "voice",
            "artist",
            "artists",
        ]
        .iter()
        .any(|marker| line_norm.starts_with(marker))
            && reference
                .artists
                .iter()
                .all(|artist| contains_name(line, artist))
    });
    !channel.trim().is_empty()
        && prefix_matches
        && rights_match
        && performers_match
        && reference
            .artists
            .iter()
            .all(|artist| artist_in_title(recording, artist))
}

fn metadata_artists(recording: &Recording) -> Vec<&str> {
    if let Some(artists) = &recording.artists {
        artists.iter().map(String::as_str).collect()
    } else {
        recording.artist.as_deref().into_iter().collect()
    }
}

fn base_score(reference: &Reference, recording: &Recording, full_metadata: bool) -> Option<i32> {
    let title = recording.title.as_deref()?;
    let key = title_key(&reference.title, &[]);
    if key.is_empty()
        || !compatible_version(&reference.title, title)
        || !crate::audio_match::audio_edition_allowed(
            &format!("{} {}", reference.title, reference.album),
            &format!(
                "{} {}",
                recording.track.as_deref().unwrap_or(""),
                recording.album.as_deref().unwrap_or("")
            ),
        )
    {
        return None;
    }
    // Explicitly modified audio is distinct even if its duration happens to fit.
    let title_norm = format!(" {} ", normalized(title));
    let expected_norm = format!(" {} ", normalized(&reference.title));
    for tag in [
        "bass boosted",
        "boosted",
        "8d",
        "tiktok",
        "tik tok",
        "mashup",
        "edit",
        "remastered",
        "super slowed",
        "speed up",
        "slowed down",
        "remix",
        "cover",
        "karaoke",
        "instrumental",
        "reaction",
        "snippet",
        "teaser",
        "preview",
        "loop",
        "1 hour",
        "10 hours",
    ] {
        if title_norm.contains(&format!(" {tag} ")) && !expected_norm.contains(&format!(" {tag} "))
        {
            return None;
        }
    }
    if recording.is_live == Some(true)
        || matches!(
            recording.live_status.as_deref(),
            Some("is_live" | "is_upcoming" | "post_live")
        )
    {
        return None;
    }
    let title_score =
        crate::title_similarity::score(&key, &candidate_title_key(recording, reference))?;
    // Conflicting music credits override an attractive video title.
    if let Some(track) = recording.track.as_deref()
        && !track.is_empty()
        && crate::title_similarity::score(&key, &title_key(track, &[])).is_none()
    {
        return None;
    }
    let primary = reference
        .artists
        .first()
        .filter(|name| !name.trim().is_empty())?;
    if normalized(primary) == "unknown artist" {
        return None;
    }
    let metadata = metadata_artists(recording);
    if !metadata.is_empty() && !metadata.iter().any(|name| contains_name(name, primary)) {
        return None;
    }
    let channel = recording
        .channel
        .as_deref()
        .or(recording.uploader.as_deref())
        .unwrap_or("");
    let description_credit = recording
        .description
        .as_deref()
        .unwrap_or("")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(8)
        .any(|line| {
            contains_name(line, primary)
                && (contains_name(line, &key)
                    || ["artist", "performed by", "performer"]
                        .iter()
                        .any(|prefix| normalized(line).starts_with(prefix)))
        });
    let identified = artist_in_title(recording, primary)
        || artist_channel(channel, primary)
        || metadata.iter().any(|name| contains_name(name, primary))
        || description_credit;
    if full_metadata && !identified {
        return None;
    }
    let duration = recording.duration.filter(|v| v.is_finite() && *v > 0.0);
    if full_metadata && duration.is_none() {
        return None;
    }
    let duration = duration.unwrap_or(f64::from(reference.duration_secs));
    let difference = (duration - f64::from(reference.duration_secs)).abs();
    if !duration_compatible(
        reference.duration_secs,
        duration.round() as u32,
        title,
        recording.channel_is_verified == Some(true),
    ) {
        return None;
    }
    let mut score = 200
        + title_score
        + match difference as u32 {
            0..=2 => 100,
            3..=5 => 70,
            6..=8 => 35,
            _ => 0,
        };
    // Prefer an explicitly labelled original recording over an otherwise
    // equivalent upload. Unlabelled songs remain eligible.
    if !crate::audio_match::clean_edition(&reference.title)
        && ["explicit", "uncensored", "necenzurat"].iter().any(|tag| {
            contains_name(title, tag)
                || recording
                    .album
                    .as_deref()
                    .is_some_and(|a| contains_name(a, tag))
        })
    {
        score += 180;
    }
    if !identified {
        score -= 80;
    }
    if !reference.album.is_empty()
        && recording
            .album
            .as_ref()
            .is_some_and(|album| normalized(album) == normalized(&reference.album))
    {
        score += 45;
    }
    for artist in reference.artists.iter().skip(1) {
        if artist_in_title(recording, artist)
            || metadata.iter().any(|name| contains_name(name, artist))
        {
            score += 15;
        } else if !metadata.is_empty() {
            score -= 25;
        }
    }
    Some(score)
}

/// Cheap shortlist only. Never use this alone to authorize playback.
pub fn shortlist_score(reference: &Reference, recording: &Recording) -> Option<i32> {
    let mut score = base_score(reference, recording, false)?;
    let channel = recording
        .channel
        .as_deref()
        .or(recording.uploader.as_deref())
        .unwrap_or("");
    if reference
        .artists
        .iter()
        .any(|artist| artist_channel(channel, artist))
    {
        score += 500;
        if normalized(channel).ends_with(" topic") {
            score += 150;
        }
    }
    if recording.channel_is_verified == Some(true) {
        score += 250;
    }
    if recording.title.as_ref().is_some_and(|title| {
        title.char_indices().any(|(index, c)| {
            matches!(c, '-' | '–' | '—' | ':') && normalized(&title[..index]) == normalized(channel)
        })
    }) {
        score += 150;
    }
    Some(score)
}

/// Called only on full watch-page metadata, never on flat search results.
pub fn verified_match(reference: &Reference, recording: &Recording) -> Option<Match> {
    let mut score = base_score(reference, recording, true)?;
    let channel = recording
        .channel
        .as_deref()
        .or(recording.uploader.as_deref())
        .unwrap_or("");
    let own_channel = reference
        .artists
        .iter()
        .any(|artist| artist_channel(channel, artist));
    let verified = recording.channel_is_verified == Some(true);
    let description = recording.description.as_deref().unwrap_or("");
    let distributed = description.starts_with("Provided to YouTube by ")
        && description.contains("Auto-generated by YouTube.")
        && recording
            .track
            .as_ref()
            .is_some_and(|track| !track.is_empty())
        && !metadata_artists(recording).is_empty();
    let source = if distributed {
        score += 900;
        SourceKind::DistributedAudio
    } else if own_channel && verified {
        score += 750;
        SourceKind::VerifiedArtist
    } else if own_channel {
        score += 500;
        SourceKind::ArtistChannel
    } else if verified
        && reference
            .artists
            .first()
            .is_some_and(|artist| artist_in_title(recording, artist))
        && [
            "official audio",
            "official video",
            "official music video",
            "videoclip oficial",
            "video oficial",
        ]
        .iter()
        .any(|tag| normalized(recording.title.as_deref().unwrap_or("")).contains(tag))
    {
        score += 600;
        SourceKind::VerifiedPublisher
    } else if publisher_credits(reference, recording) {
        score += 550;
        SourceKind::CreditedPublisher
    } else {
        // Publisher evidence is a preference. A correctly identified song on
        // an independent upload is still playable; never label it official.
        SourceKind::MetadataMatch
    };
    if normalized(recording.title.as_deref().unwrap_or("")).contains("official video") {
        score -= 40;
    }
    Some(Match { score, source })
}

pub fn search_queries(reference: &Reference) -> Vec<String> {
    let title = search_title(&reference.title);
    let primary = reference.artists.first().map(String::as_str).unwrap_or("");
    let all = reference.artists.join(" ");
    let mut queries = vec![
        format!("{all} {title} official audio"),
        format!("{primary} {title} topic"),
        format!("{primary} {title}"),
    ];
    if !reference.album.is_empty() && normalized(&reference.album) != normalized(&title) {
        queries.push(format!("{primary} {title} {}", reference.album));
    }
    queries.push(format!("{all} {title}"));
    queries.dedup();
    queries
}

#[cfg(test)]
mod release_update_tests {
    use super::*;
    fn reference() -> Reference {
        Reference {
            title: "Song".into(),
            artists: vec!["Artist".into()],
            album: "Album".into(),
            duration_secs: 200,
        }
    }
    fn recording() -> Recording {
        Recording {
            id: "abcdefghijk".into(),
            title: Some("Artist - Song (Official Audio)".into()),
            channel: Some("Artist".into()),
            channel_is_verified: Some(true),
            duration: Some(200.),
            ..Default::default()
        }
    }
    #[test]
    fn censored_audio_editions_in_title_credits_and_album_are_rejected() {
        let r = reference();
        for label in ["[Clean]", "(Censored)", "(Radio Edit)", "Clean Version"] {
            let mut c = recording();
            c.title = Some(format!("Artist - Song {label}"));
            assert!(verified_match(&r, &c).is_none(), "{label}");
            let mut c = recording();
            c.album = Some(format!("Album {label}"));
            assert!(verified_match(&r, &c).is_none(), "album {label}");
        }
        assert!(!crate::audio_match::clean_edition(
            "Clean Bandit - Symphony"
        ));
        assert!(!crate::audio_match::clean_edition("Song (Uncensored)"));
        assert!(verified_match(&r, &recording()).is_some());
    }
    #[test]
    fn explicit_recording_is_preferred_and_verified_audio_can_start_without_extra_queries() {
        let r = reference();
        let ordinary = recording();
        let mut explicit = ordinary.clone();
        explicit.title = Some("Artist - Song (Explicit) (Official Audio)".into());
        assert!(
            verified_match(&r, &explicit).unwrap().score
                > verified_match(&r, &ordinary).unwrap().score
        );
        let matched = verified_match(&r, &ordinary).unwrap();
        assert!(crate::player::fast_catalog_match(&r, &ordinary, &matched));
        let mut video = ordinary;
        video.title = Some("Artist - Song (Official Video)".into());
        assert!(!crate::player::fast_catalog_match(&r, &video, &matched));
    }
    #[test]
    fn catalogue_mirroring_does_not_choose_a_clean_album_or_reject_ordinary_song_words() {
        let source = crate::player::LocalTrack {
            title: "Song".into(),
            album: "Album".into(),
            artists: vec![crate::api::models::ArtistRef {
                name: "Artist".into(),
                ..Default::default()
            }],
            duration_ms: 200000,
            ..Default::default()
        };
        let clean = crate::player::LocalTrack {
            uri: "vibefy:youtube:abcdefghijk".into(),
            album: "Album (Clean)".into(),
            ..source.clone()
        };
        let explicit = crate::player::LocalTrack {
            uri: "vibefy:youtube:bbbbbbbbbbb".into(),
            album: "Album (Explicit)".into(),
            ..source.clone()
        };
        assert_eq!(
            crate::playlist_tools::choose_recording(&source, &[clean, explicit.clone()])
                .unwrap()
                .uri,
            explicit.uri
        );
    }
}
