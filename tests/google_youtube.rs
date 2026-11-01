use serde_json::json;
use spotifast::google_youtube::{self, callback_code};

#[test]
fn google_button_requests_offline_playlist_access_with_pkce() {
    let url = google_youtube::authorization_url(
        "example.apps.googleusercontent.com",
        "http://127.0.0.1:43210/youtube/callback",
        "state-123",
        "verifier-123",
    )
    .unwrap();
    let url = reqwest::Url::parse(&url).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("accounts.google.com"));
    let fields: std::collections::BTreeMap<_, _> = url.query_pairs().collect();
    assert_eq!(fields["state"], "state-123");
    assert_eq!(fields["code_challenge_method"], "S256");
    assert_eq!(
        fields["code_challenge"],
        spotifast::music_accounts::challenge("verifier-123")
    );
    assert_eq!(fields["access_type"], "offline");
    assert_eq!(fields["scope"], google_youtube::SCOPE);
    assert_eq!(fields["prompt"], "consent select_account");
    assert!(!fields.contains_key("client_secret"));
}
#[test]
fn google_callback_requires_the_exact_state_and_route() {
    assert_eq!(
        callback_code(
            "GET /youtube/callback?state=right&code=a%2Bb HTTP/1.1\r\n",
            "right"
        )
        .unwrap(),
        Some("a+b".into())
    );
    for request in [
        "GET /login?state=right&code=abc HTTP/1.1",
        "POST /youtube/callback?state=right&code=abc HTTP/1.1",
        "GET /youtube/callback?state=wrong&code=abc HTTP/1.1",
        "GET /youtube/callback?code=abc HTTP/1.1",
        "GET /youtube/callback?state=wrong&code=a&code=b HTTP/1.1",
    ] {
        assert!(callback_code(request, "right").unwrap().is_none());
    }
}
#[test]
fn matched_google_refusal_finishes_and_duplicate_codes_are_rejected() {
    assert!(
        callback_code(
            "GET /youtube/callback?state=right&error=access_denied HTTP/1.1",
            "right"
        )
        .is_err()
    );
    assert!(
        callback_code(
            "GET /youtube/callback?state=right&code=one&code=two HTTP/1.1",
            "right"
        )
        .is_err()
    );
    assert!(callback_code("GET /youtube/callback?state=right&code= HTTP/1.1", "right").is_err());
    assert!(callback_code(&"x".repeat(8193), "right").is_err());
}
#[test]
fn youtube_library_rows_keep_video_identity_and_use_the_owner_credit() {
    let row = json!({"snippet":{"title":"Example song","videoOwnerChannelTitle":"Example artist - Topic","resourceId":{"videoId":"dQw4w9WgXcQ"},"thumbnails":{"high":{"url":"https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg"}}}});
    let track = google_youtube::parse_track(&row).unwrap();
    assert_eq!(track.uri, "vibefy:youtube:dQw4w9WgXcQ");
    assert_eq!(track.artists[0].name, "Example artist");
    assert!(track.art_url.is_some());
    let mut deleted = row.clone();
    deleted["snippet"]["title"] = "Deleted video".into();
    assert!(google_youtube::parse_track(&deleted).is_none());
    let mut unsafe_art = row.clone();
    unsafe_art["snippet"]["thumbnails"]["high"]["url"] = "http://localhost/private".into();
    assert!(
        google_youtube::parse_track(&unsafe_art)
            .unwrap()
            .art_url
            .is_none()
    );
}
#[test]
fn youtube_iso_durations_are_converted_without_guessing() {
    assert_eq!(google_youtube::duration_ms("PT3M21S"), 201_000);
    assert_eq!(google_youtube::duration_ms("PT1H2M3.5S"), 3_723_500);
    for value in ["", "3:21", "PTNaNS", "PT1M10", "PT1XS"] {
        assert_eq!(google_youtube::duration_ms(value), 0);
    }
}
