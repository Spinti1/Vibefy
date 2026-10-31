use spotifast::api::models::ArtistRef;
use spotifast::player::{
    LocalState, LocalTrack, MatchCache, Playback, QueueState, receive_track_metadata,
    resolve_track_audio,
};
use spotifast::settings::ProxyConfig;

fn known(uri: &str) -> LocalTrack {
    LocalTrack {
        uri: uri.into(),
        title: "On My Own".into(),
        duration_ms: 185760,
        artists: vec![ArtistRef {
            name: "Three Days Grace".into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn placeholder(uri: &str) -> LocalTrack {
    LocalTrack {
        uri: uri.into(),
        title: "Track unknown".into(),
        ..Default::default()
    }
}

#[test]
fn metadata_arrival_restarts_only_the_current_waiting_track_and_keeps_queue_order() {
    let uri = "spotify:track:current";
    let mut state = LocalState {
        track: Some(placeholder(uri)),
        playback: Playback::Loading,
        position_ms: 1234,
        ..Default::default()
    };
    let mut queue = QueueState {
        tracks: vec![
            placeholder(uri),
            placeholder("spotify:track:next"),
            placeholder(uri),
        ],
        current_index: 0,
        manual_queue_count: 1,
        ..Default::default()
    };
    state.queue_tracks = queue.tracks.clone();
    let metadata = known(uri);
    assert_eq!(
        receive_track_metadata(&mut state, &mut queue, &metadata),
        Some(metadata.clone())
    );
    assert_eq!(state.position_ms, 1234);
    assert_eq!(queue.current_index, 0);
    assert_eq!(queue.manual_queue_count, 1);
    assert_eq!(queue.tracks[1].uri, "spotify:track:next");
    assert_eq!(queue.tracks[2], metadata);
    assert!(receive_track_metadata(&mut state, &mut queue, &metadata).is_none());
}

#[test]
fn paused_or_superseded_metadata_never_starts_playback() {
    let mut state = LocalState {
        track: Some(placeholder("spotify:track:current")),
        playback: Playback::Paused,
        ..Default::default()
    };
    let mut queue = QueueState::default();
    assert!(receive_track_metadata(&mut state, &mut queue, &known("spotify:track:old")).is_none());
    assert!(!state.track.as_ref().unwrap().has_recording_metadata());
    assert!(
        receive_track_metadata(&mut state, &mut queue, &known("spotify:track:current")).is_none()
    );
    assert!(state.track.as_ref().unwrap().has_recording_metadata());
    assert_eq!(state.playback, Playback::Paused);
}

#[tokio::test]
async fn placeholders_do_not_search_youtube_or_destroy_a_previous_match() {
    let root = std::env::temp_dir().join(format!("vibefy-metadata-{}", rand::random::<u64>()));
    let cache = MatchCache::new(&root);
    cache.save_match(
        "current",
        "abcdefghijk",
        "Three Days Grace - On My Own",
        185,
    );
    let result = resolve_track_audio(
        &root.join("missing-yt-dlp.exe"),
        &cache,
        &placeholder("spotify:track:current"),
        &ProxyConfig::Off,
    )
    .await;
    assert!(
        format!("{:#}", result.unwrap_err()).contains("Spotify track details are still loading")
    );
    assert_eq!(
        cache.get_video_id("current").as_deref(),
        Some("abcdefghijk")
    );
    let _ = std::fs::remove_file(root.join("youtube_cache.json"));
    let _ = std::fs::remove_dir(root.join("youtube_audio"));
    let _ = std::fs::remove_dir(root);
}
