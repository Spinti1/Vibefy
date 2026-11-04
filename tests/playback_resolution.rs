use spotifast::api::models::ArtistRef;
use spotifast::player::{LocalTrack, MatchCache, resolve_track_audio, youtube_proxy_args};
use spotifast::settings::{ProxyConfig, Settings};

#[test]
fn proxy_off_preserves_an_explicit_empty_argument() {
    let mut command = tokio::process::Command::new("yt-dlp");
    command.args(youtube_proxy_args(&ProxyConfig::Off));
    command.arg("ytsearch6:Artist - Song audio");
    let args: Vec<_> = command.as_std().get_args().collect();
    assert_eq!(args, ["--proxy", "", "ytsearch6:Artist - Song audio"]);
}

#[test]
fn obsolete_saved_proxy_is_ignored() {
    for mode in ["http", "socks", "system"] {
        let settings: Settings = serde_json::from_value(serde_json::json!({"proxy_mode":mode,"proxy_host":"127.0.0.1","proxy_port":"9","proxy_password":"old"})).unwrap();
        assert_eq!(settings.proxy_config().unwrap(), ProxyConfig::Off);
        assert!(settings.proxy_password.is_empty());
        assert!(!settings.encode().unwrap().contains("proxy_host"));
        assert_eq!(
            youtube_proxy_args(&settings.proxy_config().unwrap()),
            ["--proxy", ""]
        );
    }
}

#[tokio::test]
async fn failed_search_is_not_reported_as_an_incompatible_recording() {
    let directory = std::env::temp_dir().join(format!(
        "vibefy-resolution-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cache = MatchCache::new(&directory);
    let track = LocalTrack {
        uri: "spotify:track:test".into(),
        title: "Song".into(),
        artists: vec![ArtistRef {
            name: "Artist".into(),
            ..Default::default()
        }],
        duration_ms: 200_000,
        ..Default::default()
    };
    let result = resolve_track_audio(
        &directory.join("missing-yt-dlp.exe"),
        &cache,
        &track,
        &ProxyConfig::Off,
    )
    .await;
    let _ = std::fs::remove_dir(directory.join("youtube_audio"));
    let _ = std::fs::remove_dir(&directory);
    let error = format!("{:#}", result.unwrap_err());
    assert!(error.contains("Playback cannot search YouTube"), "{error}");
    assert!(error.contains("Cannot start yt-dlp"), "{error}");
    assert!(!error.contains("No matching recording"), "{error}");
}
