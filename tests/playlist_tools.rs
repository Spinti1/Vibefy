use serde_json::json;
use spotifast::{api::models::ArtistRef, music_session, player::LocalTrack, playlist_tools};

fn song(uri: &str, title: &str, artist: &str, duration: u32, album: &str) -> LocalTrack {
    LocalTrack {
        uri: uri.into(),
        title: title.into(),
        name: title.into(),
        artists: vec![ArtistRef {
            name: artist.into(),
            ..Default::default()
        }],
        duration_ms: duration,
        album: album.into(),
        ..Default::default()
    }
}
#[test]
fn copies_are_always_created_unlisted_and_keep_the_recovery_marker() {
    let body =
        music_session::unlisted_playlist_body("Rock <example>", "Vibefy mirror: pair:playlist")
            .unwrap();
    assert_eq!(body["privacyStatus"], "UNLISTED");
    assert_eq!(body["description"], "Vibefy mirror: pair:playlist");
    assert_eq!(body["title"], "Rock example");
    assert!(music_session::unlisted_playlist_body(" <> ", "marker").is_err());
}
#[test]
fn privacy_and_marker_come_only_from_the_editable_playlist_header() {
    let page = json!({"contents":[
        {"musicDescriptionShelfRenderer":{"description":{"runs":[{"text":"unrelated recommendation"}]}}},
        {"musicEditablePlaylistDetailHeaderRenderer":{"editHeader":{"musicPlaylistEditHeaderRenderer":{"privacy":"PRIVATE","description":{"runs":[{"text":"Vibefy mirror: exact-pair:exact-playlist"}]}}}}}
    ]});
    assert_eq!(
        music_session::playlist_details(&page),
        (true, "Vibefy mirror: exact-pair:exact-playlist".into())
    );
    assert!(
        !music_session::playlist_details(
            &json!({"contents":{"musicPlaylistEditHeaderRenderer":{"privacy":"PRIVATE"}}})
        )
        .0
    );
    let public = json!({"header":{"musicEditablePlaylistDetailHeaderRenderer":{"editHeader":{"musicPlaylistEditHeaderRenderer":{"privacy":"PUBLIC","description":{"runs":[{"text":"Vibefy mirror: x"}]}}}}}});
    assert!(!music_session::playlist_details(&public).0);
    for visibility in ["UNLISTED", "PRIVATE", "PUBLIC", "UNKNOWN", ""] {
        let page = json!({"header":{"musicEditablePlaylistDetailHeaderRenderer":{"editHeader":{"musicPlaylistEditHeaderRenderer":{"privacy":visibility,"description":{"runs":[{"text":"Vibefy mirror: x"}]}}}}}});
        let (allowed, description) = music_session::playlist_details(&page);
        assert_eq!(allowed, matches!(visibility, "UNLISTED" | "PRIVATE"));
        assert_eq!(description, "Vibefy mirror: x");
    }
}
#[test]
fn partial_or_unrelated_server_acknowledgements_do_not_settle_a_batch() {
    let expected = vec!["abcdefghijk".into(), "abcdefghijk".into()];
    let result = |ids: Vec<&str>| json!({"status":"STATUS_SUCCEEDED","playlistEditResults":ids.iter().map(|id|json!({"playlistEditVideoAddedResultData":{"videoId":id,"setVideoId":"server-instance"}})).collect::<Vec<_>>()});
    assert!(!music_session::append_confirmed(
        &result(vec!["abcdefghijk"]),
        &expected
    ));
    assert!(!music_session::append_confirmed(
        &result(vec!["abcdefghijk", "zyxwvutsrqp"]),
        &expected
    ));
    assert!(music_session::append_confirmed(
        &result(vec!["abcdefghijk", "abcdefghijk"]),
        &expected
    ));
    assert!(!music_session::append_confirmed(
        &json!({"status":"STATUS_FAILED"}),
        &expected
    ));
}
#[test]
fn retry_after_respects_both_seconds_and_http_dates() {
    assert_eq!(music_session::retry_after(Some("1800"), 100), 1800);
    assert_eq!(
        music_session::retry_after(Some("Wed, 21 Oct 2015 07:28:00 GMT"), 1445412360),
        120
    );
    assert_eq!(music_session::retry_after(None, 100), 60);
    assert!(music_session::retry_after(Some("0"), 100) > 0);
}
#[test]
fn matching_rejects_other_artists_versions_and_missing_durations() {
    let source = song(
        "spotify:track:0123456789012345678901",
        "Painkiller",
        "Three Days Grace",
        180000,
        "Human",
    );
    let good = song(
        "vibefy:youtube:abcdefghijk",
        "Painkiller",
        "Three Days Grace",
        181000,
        "Human",
    );
    assert_eq!(
        playlist_tools::choose_recording(&source, &[good.clone()])
            .unwrap()
            .uri,
        good.uri
    );
    let wrong_artist = song(&good.uri, "Painkiller", "Karaoke Covers", 180000, "Human");
    let wrong_version = song(
        &good.uri,
        "Painkiller (Live)",
        "Three Days Grace",
        180000,
        "Human",
    );
    let wrong_duration = song(&good.uri, "Painkiller", "Three Days Grace", 190000, "Human");
    let no_duration = song(&good.uri, "Painkiller", "Three Days Grace", 0, "Human");
    for candidate in [wrong_artist, wrong_version, wrong_duration, no_duration] {
        assert!(playlist_tools::choose_recording(&source, &[candidate]).is_none());
    }
}
#[test]
fn ambiguous_releases_require_review_but_equivalent_catalogue_ids_are_usable() {
    let source = song(
        "spotify:track:0123456789012345678901",
        "Example",
        "Artist",
        180000,
        "",
    );
    let first = song(
        "vibefy:youtube:abcdefghijk",
        "Example",
        "Artist",
        180000,
        "Release A",
    );
    let mut second = song(
        "vibefy:youtube:zyxwvutsrqp",
        "Example",
        "Artist",
        180000,
        "Release B",
    );
    assert!(playlist_tools::choose_recording(&source, &[first.clone(), second.clone()]).is_none());
    second.album = first.album.clone();
    assert!(playlist_tools::choose_recording(&source, &[first, second]).is_some());
}
#[test]
fn unavailable_playlist_rows_and_recommendations_are_excluded_from_imports() {
    let row = |id: &str| json!({"musicResponsiveListItemRenderer":{"playlistItemData":{"videoId":id},"musicItemRendererDisplayPolicy":"MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT"}});
    let page = json!({"contents":[{"musicPlaylistShelfRenderer":{"contents":[row("abcdefghijk"),row("abcdefghijk")]}},{"musicShelfRenderer":{"contents":[row("zyxwvutsrqp")]}}]});
    assert!(
        music_session::parse_playlist_page(&page)
            .unwrap()
            .entries
            .is_empty()
    );
}
#[test]
fn missing_featured_artist_is_not_an_automatic_recording_match() {
    let mut source = song(
        "spotify:track:0123456789012345678901",
        "Example",
        "Artist",
        180000,
        "Album",
    );
    source.artists.push(ArtistRef {
        name: "Featured artist".into(),
        ..Default::default()
    });
    let candidate = song(
        "vibefy:youtube:abcdefghijk",
        "Example",
        "Artist",
        180000,
        "Album",
    );
    assert!(playlist_tools::choose_recording(&source, &[candidate]).is_none());
}

#[test]
fn captured_redacted_unlisted_header_retains_the_owner_marker() {
    let value =
        serde_json::from_str(include_str!("fixtures/youtube-editable-unlisted.json")).unwrap();
    assert_eq!(
        music_session::playlist_details(&value),
        (true, "Vibefy mirror: fixture".into())
    );
}

#[test]
fn playlist_names_accept_unicode_but_reject_empty_controls_and_overflow() {
    assert!(playlist_tools::valid_title("Muzică 2026"));
    assert!(!playlist_tools::valid_title("  "));
    assert!(!playlist_tools::valid_title("bad\nname"));
    assert!(!playlist_tools::valid_title(&"a".repeat(101)));
}
