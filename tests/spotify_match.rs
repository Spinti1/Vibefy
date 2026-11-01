#[path = "../src/spotify_match.rs"]
mod matching;

#[test]
fn only_real_spotify_track_identifiers_can_be_added() {
    assert!(matching::valid_uri("spotify:track:1234567890abcdefghABCD"));
    for uri in [
        "vibefy:youtube:abcdefghijk",
        "spotify:episode:1234567890abcdefghABCD",
        "spotify:track:short",
        "spotify:track:1234567890abcdefghABC/",
    ] {
        assert!(!matching::valid_uri(uri));
    }
}

#[test]
fn original_artist_and_duration_rank_above_a_cover() {
    let original = matching::score(
        "I Am Machine",
        "Three Days Grace",
        201000,
        "I Am Machine",
        "Three Days Grace",
        200000,
    );
    let cover = matching::score(
        "I Am Machine",
        "Three Days Grace",
        201000,
        "I Am Machine",
        "Cover Band",
        240000,
    );
    assert!(original > cover);
}

#[test]
fn version_differences_are_not_hidden_by_a_close_duration() {
    let studio = matching::score("Song", "Artist", 180000, "Song", "Artist", 184000);
    let live = matching::score("Song", "Artist", 180000, "Song Live", "Artist", 180000);
    assert!(studio > live);
    let requested_live =
        matching::score("Song Live", "Artist", 180000, "Song Live", "Artist", 180000);
    assert!(
        requested_live > matching::score("Song Live", "Artist", 180000, "Song", "Artist", 180000)
    );
}
