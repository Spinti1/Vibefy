//! Ranking for the user-confirmed source-to-Spotify playlist picker.
//! Scores order suggestions; no score authorizes a playlist write.
pub fn valid_uri(uri: &str) -> bool {
    uri.strip_prefix("spotify:track:")
        .is_some_and(|id| id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric()))
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

fn overlap(a: &str, b: &str) -> i32 {
    let a = words(a);
    let b = words(b);
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    let common = a.iter().filter(|word| b.contains(word)).count();
    (200 * common / (a.len() + b.len())) as i32
}

pub fn score(
    title: &str,
    artist: &str,
    duration: u32,
    candidate: &str,
    candidate_artist: &str,
    candidate_duration: u32,
) -> i32 {
    let mut score = overlap(title, candidate) * 3 + overlap(artist, candidate_artist) * 2;
    if duration > 0 && candidate_duration > 0 {
        score += 100 - (duration.abs_diff(candidate_duration) / 1000).min(180) as i32;
    }
    let title_words = words(title);
    let candidate_words = words(candidate);
    for version in [
        "live",
        "remix",
        "acoustic",
        "instrumental",
        "karaoke",
        "sped",
        "slowed",
    ] {
        if title_words.iter().any(|w| w == version) != candidate_words.iter().any(|w| w == version)
        {
            score -= 200;
        }
    }
    score
}
