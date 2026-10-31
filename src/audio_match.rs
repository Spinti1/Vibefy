//! Recording selection and an audio clock that retains fractional milliseconds.

pub const MATCHER_VERSION: u8 = 4;

pub fn audible_position(decoded_ms: u32, queued_ms: u32) -> u32 {
    decoded_ms.saturating_sub(queued_ms)
}

/// Container timestamps are not always sample counts (WebM commonly uses
/// milliseconds). Seek in seconds and report where the demuxer actually landed.
pub fn seek_recording(
    format: &mut dyn symphonia::core::formats::FormatReader,
    track_id: u32,
    position_ms: u32,
) -> symphonia::core::errors::Result<u32> {
    use symphonia::core::formats::{SeekMode, SeekTo};
    use symphonia::core::units::Time;
    let time_base = format
        .tracks()
        .iter()
        .find(|track| track.id == track_id)
        .and_then(|track| track.codec_params.time_base);
    let result = format.seek(
        SeekMode::Accurate,
        SeekTo::Time {
            time: Time::new(
                u64::from(position_ms / 1000),
                f64::from(position_ms % 1000) / 1000.0,
            ),
            track_id: Some(track_id),
        },
    )?;
    Ok(time_base.map_or(position_ms, |base| {
        let time = base.calc_time(result.actual_ts);
        (time.seconds as f64 * 1000.0 + time.frac * 1000.0).round() as u32
    }))
}

pub struct SampleClock {
    start_ms: u32,
    frames: u64,
    rate: u32,
}

impl SampleClock {
    pub fn new(start_ms: u32, rate: u32) -> Self {
        Self {
            start_ms,
            frames: 0,
            rate: rate.max(1),
        }
    }

    pub fn seek(&mut self, position_ms: u32) {
        self.start_ms = position_ms;
        self.frames = 0;
    }

    pub fn advance(&mut self, frames: usize) -> u32 {
        self.frames = self.frames.saturating_add(frames as u64);
        let elapsed = self.frames.saturating_mul(1000) / u64::from(self.rate);
        u64::from(self.start_ms)
            .saturating_add(elapsed)
            .min(u64::from(u32::MAX)) as u32
    }
}

pub fn normalized(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'ă' | 'â' | 'á' | 'à' | 'ä' | 'ã' | 'å' => 'a',
            'î' | 'í' | 'ì' | 'ï' => 'i',
            'ș' | 'ş' => 's',
            'ț' | 'ţ' => 't',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn clean_title(title: &str) -> String {
    let mut text = title.to_string();
    for pattern in [
        "(feat.",
        "(ft.",
        "(featuring",
        "(with ",
        "[feat.",
        "[ft.",
        "[featuring",
        "[with ",
        "(prod.",
        "[prod.",
        "(official",
        "[official",
        "(remaster",
        "[remaster",
        "(bonus",
        "[bonus",
        "(deluxe",
        "[deluxe",
        " feat. ",
        " ft. ",
        " featuring ",
        " feat ",
        " - remaster",
        " - original mix",
        " - bonus track",
        " - deluxe",
        " - single version",
        " - album version",
    ] {
        // Lowercasing may expand a character (for example Turkish İ). Find
        // the match on original UTF-8 boundaries before truncating.
        if let Some(index) = text.char_indices().find_map(|(index, _)| {
            text[index..]
                .to_lowercase()
                .starts_with(pattern)
                .then_some(index)
        }) {
            text.truncate(index);
        }
    }
    if text.trim().is_empty() {
        title.to_string()
    } else {
        text.trim().to_string()
    }
}

fn phrase(text: &str, needle: &str) -> bool {
    format!(" {text} ").contains(&format!(" {needle} "))
}

pub fn compatible_version(expected: &str, candidate: &str) -> bool {
    let expected = normalized(expected);
    let candidate = normalized(candidate);
    for version in [
        "live",
        "acoustic",
        "remix",
        "radio edit",
        "slowed",
        "sped up",
        "nightcore",
        "instrumental",
        "karaoke",
        "cover",
        "tribute",
        "remake",
        "reverb",
        "extended",
    ] {
        if phrase(&candidate, version) && !phrase(&expected, version) {
            return false;
        }
        if ["live", "acoustic", "remix", "radio edit", "extended"].contains(&version)
            && phrase(&expected, version)
            && !phrase(&candidate, version)
        {
            return false;
        }
    }
    true
}

pub fn duration_matches(expected: u32, candidate: u32) -> bool {
    expected == 0 || (candidate > 0 && expected.abs_diff(candidate) <= 15)
}

pub fn rank(
    title: &str,
    uploader: &str,
    duration: Option<f64>,
    artist: &str,
    expected_title: &str,
    clean_title: &str,
    expected_duration: u32,
) -> i32 {
    let title_norm = normalized(title);
    let needle = normalized(clean_title);
    if needle.is_empty() || !compatible_version(expected_title, title) {
        return -1000;
    }
    let words: Vec<_> = needle.split_whitespace().collect();
    let candidate_words: Vec<_> = title_norm.split_whitespace().collect();
    let ratio = words
        .iter()
        .filter(|word| candidate_words.contains(word))
        .count() as f32
        / words.len().max(1) as f32;
    let exact_phrase = phrase(&title_norm, &needle);
    if !exact_phrase && ratio < 0.8 {
        return -1000;
    }
    let primary = artist
        .split([',', '&', ';'])
        .next()
        .unwrap_or(artist)
        .trim();
    let artist_norm = normalized(primary);
    let uploader_norm = normalized(uploader);
    let compact = |text: &str| text.replace(' ', "");
    let artist_match = artist_norm.is_empty()
        || artist_norm == "unknown artist"
        || title_norm.contains(&artist_norm)
        || uploader_norm.contains(&artist_norm)
        || compact(&uploader_norm).contains(&compact(&artist_norm));
    if !artist_match {
        return -1000;
    }
    let mut score = if exact_phrase { 150 } else { 100 };
    score += 60;
    if let Some(duration) = duration.filter(|value| value.is_finite() && *value > 0.0) {
        let duration = duration.round() as u32;
        if !duration_matches(expected_duration, duration) {
            return -1000;
        }
        if expected_duration > 0 {
            score += match expected_duration.abs_diff(duration) {
                0..=2 => 100,
                3..=5 => 60,
                6..=8 => 5,
                _ => -140,
            };
        }
    } else {
        score -= 100;
    }
    if uploader_norm.ends_with(" topic") {
        score += 80;
    }
    if phrase(&title_norm, "official audio") {
        score += 60;
    } else if phrase(&title_norm, "audio") {
        score += 25;
    }
    if phrase(&title_norm, "official video")
        || phrase(&title_norm, "music video")
        || phrase(&title_norm, "videoclip oficial")
    {
        score -= 45;
    }
    for unwanted in [
        "hour", "hours", "loop", "preview", "teaser", "snippet", "reaction",
    ] {
        if phrase(&title_norm, unwanted) && !phrase(&needle, unwanted) {
            return -1000;
        }
    }
    score
}
