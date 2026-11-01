use spotifast::{
    addons::{direct_url, safe_art_url},
    music_accounts::{challenge, juice_song},
    music_session::Session,
};

#[test]
fn browser_session_parses_json_and_raw_headers_without_reusing_authorization() {
    let json = r#"{"Cookie":"__Secure-3PAPISID=example-only; SID=example","X-Goog-AuthUser":"0","Origin":"https://music.youtube.com","Authorization":"expired","Host":"evil.example"}"#;
    let raw = "cookie: __Secure-3PAPISID=example-only; SID=example\nx-goog-authuser: 0\nx-origin: https://music.youtube.com";
    let chrome = "cookie\n__Secure-3PAPISID=example-only; SID=example\nx-goog-authuser\n0";
    let expected = Session::parse(json)
        .unwrap()
        .authorization_at(123456)
        .unwrap();
    assert_eq!(
        expected,
        "SAPISIDHASH 123456_f7a74149cba4ee2a2d175e68041b5403cf11d855"
    );
    assert_ne!(expected, "expired");
    assert_eq!(
        Session::parse(raw)
            .unwrap()
            .authorization_at(123456)
            .unwrap(),
        expected
    );
    assert_eq!(
        Session::parse(chrome)
            .unwrap()
            .authorization_at(123456)
            .unwrap(),
        expected
    );
    assert_ne!(
        Session::parse(json)
            .unwrap()
            .authorization_at(123457)
            .unwrap(),
        expected
    );
}

#[test]
fn browser_session_rejects_signed_out_foreign_and_malformed_imports() {
    for input in [
        r#"{"Cookie":"SID=not-signed-in"}"#,
        r#"{"Cookie":"SAPISID=example","Origin":"https://evil.example"}"#,
        r#"{"Cookie":"SAPISID=example","X-Goog-AuthUser":"0\r\nHost: evil"}"#,
        r#"{"Cookie":"SAPISID=example\r\nHost: evil"}"#,
        r#"{"Cookie":"SAPISID=example","X-Goog-PageId":"bad/header"}"#,
        "{broken",
        "",
        "[]",
    ] {
        assert!(Session::parse(input).is_err(), "invalid import accepted");
    }
    assert!(Session::parse(&"x".repeat(65537)).is_err());
}

#[test]
fn explicit_library_import_keeps_user_recordings_duplicates_and_large_pages() {
    let row = serde_json::json!({"musicResponsiveListItemRenderer":{
        "flexColumns":[{"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":"Example recording"}]}}}],
        "playlistItemData":{"videoId":"abcdefghijk"},
        "musicVideoType":"MUSIC_VIDEO_TYPE_OMV"
    }});
    let response = serde_json::json!({"contents":[row.clone(),row.clone()]});
    assert!(
        spotifast::music::parse_browse(&response)
            .unwrap()
            .entries
            .is_empty()
    );
    let rows = vec![row; 150];
    let response = serde_json::json!({"continuationContents":{"musicPlaylistShelfContinuation":{"contents":rows,"continuations":[{"nextContinuationData":{"continuation":"next-page"}}]}}});
    let imported = spotifast::music::parse_library_browse(&response).unwrap();
    assert_eq!(imported.entries.len(), 150);
    assert_eq!(imported.continuation.as_deref(), Some("next-page"));
    assert!(
        imported
            .entries
            .iter()
            .all(|entry| entry.track.as_ref().unwrap().uri == "vibefy:youtube:abcdefghijk")
    );
}

#[test]
fn pkce_uses_the_rfc7636_sha256_example() {
    assert_eq!(
        challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn juicevault_metadata_preserves_identity_and_safe_artwork() {
    let song = serde_json::json!({"id":"88738dac-ec83-4e5e-a9c3-95bfb180b7f5","title":"Example","artist":"Example artist","duration":165,"cover":"/cdn/music/covers/88738dac-ec83-4e5e-a9c3-95bfb180b7f5"});
    let track = juice_song(&song).unwrap();
    assert_eq!(track.duration_ms, 165000);
    assert_eq!(track.artist_names(), "Example artist");
    assert_eq!(
        direct_url(&track.uri).as_deref(),
        Some("https://api.juicevault.xyz/music/stream/88738dac-ec83-4e5e-a9c3-95bfb180b7f5")
    );
    assert!(safe_art_url(track.cover_url.as_ref().unwrap()));
}

#[test]
fn source_addresses_cannot_escape_the_provider() {
    for id in [
        "../private",
        "88738dac-ec83-4e5e-a9c3-95bfb180b7f5?token=x",
        "https://localhost",
        "88738dac-ec83-4e5e-a9c3-95bfb180b7fg",
    ] {
        assert!(direct_url(&format!("vibefy:juicevault:{id}")).is_none());
        assert!(juice_song(&serde_json::json!({"id":id,"title":"Example"})).is_none());
    }
    for url in [
        "http://api.juicevault.xyz/cdn/test",
        "https://api.juicevault.xyz.evil.example/test",
        "https://user:password@api.juicevault.xyz/cdn/test",
    ] {
        assert!(!safe_art_url(url));
    }
}

#[test]
fn juicevault_unsafe_art_is_not_saved() {
    let track=juice_song(&serde_json::json!({"id":"88738dac-ec83-4e5e-a9c3-95bfb180b7f5","title":"Example","length":"2:45","cover":"https://localhost/secret"})).unwrap();
    assert_eq!(track.duration_ms, 165000);
    assert!(track.cover_url.is_none());
}

#[test]
fn music_playlist_parser_ignores_recommendations_and_uses_shelf_continuation() {
    let row = serde_json::json!({"musicResponsiveListItemRenderer":{
        "flexColumns":[{"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":"Example recording"}]}}}],
        "playlistItemData":{"videoId":"abcdefghijk"},"musicVideoType":"MUSIC_VIDEO_TYPE_ATV"
    }});
    let page = serde_json::json!({"contents":[
        {"musicPlaylistShelfRenderer":{"contents":[row.clone()],"continuations":[{"nextContinuationData":{"continuation":"playlist-page"}}]}},
        {"musicShelfRenderer":{"contents":[row.clone()],"continuations":[{"reloadContinuationData":{"continuation":"suggestions-page"}}]}}
    ]});
    let parsed = spotifast::music_session::parse_playlist_page(&page).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.continuation.as_deref(), Some("playlist-page"));
    let appended = serde_json::json!({"onResponseReceivedActions":[{"appendContinuationItemsAction":{"continuationItems":[row]}}]});
    assert_eq!(
        spotifast::music_session::parse_playlist_page(&appended)
            .unwrap()
            .entries
            .len(),
        1
    );
    assert!(
        spotifast::music_session::parse_playlist_page(&serde_json::json!({"contents":[]})).is_err()
    );
}
