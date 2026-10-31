#![cfg(feature = "demo")]
use spotifast::{
    api::{ApiClient, ApiError, models::*},
    app::{App, AppOptions},
    backend::Waker,
    model::Action,
    paths::AppDirs,
};
use std::time::{Duration, Instant};

fn track(name: &str, uri: &str) -> Track {
    Track {
        name: name.into(),
        uri: uri.into(),
        duration_ms: 200_000,
        artists: vec![ArtistRef {
            name: "Artist".into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn local_search_matches_all_words_ranks_titles_and_deduplicates_metadata() {
    let tracks = vec![
        track("Song", "spotify:track:1"),
        track("Song Live", "spotify:track:2"),
        track("Song", "spotify:track:1"),
        track("Unrelated", "spotify:track:3"),
        track("Song", "vibefy:youtube:abcdefghijk"),
    ];
    let result = spotifast::search::local_results("artist song", tracks.iter());
    let items = &result.tracks.unwrap().items;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "Song");
    assert!(spotifast::search::local_results("different", tracks.iter()).is_empty());
    assert!(spotifast::search::local_results("  ", tracks.iter()).is_empty());
}

#[test]
fn local_search_reaches_the_end_of_a_large_library_and_bounds_visible_results() {
    let mut tracks: Vec<_> = (0..20_000)
        .map(|i| track(&format!("Song {i}"), &format!("spotify:track:{i}")))
        .collect();
    tracks.last_mut().unwrap().name = "Unique Last Track".into();
    let start = Instant::now();
    let found = spotifast::search::local_results("Unique Last", tracks.iter());
    println!("20k library lookup: {} us", start.elapsed().as_micros());
    assert_eq!(found.tracks.unwrap().items[0].uri, "spotify:track:19999");
    assert_eq!(
        spotifast::search::local_results("Song", tracks.iter())
            .tracks
            .unwrap()
            .items
            .len(),
        50
    );
}

fn app() -> (egui::Context, App, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("vibefy-search-{}", rand::random::<u64>()));
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let waker = Waker::default();
    waker.attach(&ctx);
    let mut app = App::new(
        &waker,
        AppDirs {
            config: root.join("config"),
            state: root.join("state"),
            cache: root.join("cache"),
        },
        Default::default(),
        AppOptions {
            media_controls: false,
            restore_sign_in: false,
            tray: false,
            autostart: false,
        },
    );
    app.attach(&ctx);
    spotifast::demo::populate(&mut app);
    (ctx, app, root)
}
fn frame(ctx: &egui::Context, app: &mut App) -> Vec<String> {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 1000.0),
            )),
            ..Default::default()
        },
        |ui| app.frame_ui(ui),
    );
    out.textures_delta.clear();
    out.platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, node)| node.label().map(str::to_owned))
        .collect()
}

fn music_result(title: &str) -> spotifast::music::Results {
    use spotifast::music::{Entry, Kind, Results};
    Results {
        entries: vec![Entry {
            id: title.into(),
            kind: Kind::Song,
            title: title.into(),
            subtitle: "Artist".into(),
            image: None,
            spotify_note: None,
            browse_id: None,
            track: None,
        }],
        continuation: None,
    }
}
#[test]
fn primary_search_uses_youtube_music_instead_of_the_spotify_catalogue() {
    use spotifast::music::Filter;
    let (ctx, mut app, root) = app();
    app.library.liked.items.push(SavedTrack {
        track: track("Spotify Song", "spotify:track:local"),
        ..Default::default()
    });
    app.actions.push(Action::Search("Spotify Song".into()));
    frame(&ctx, &mut app);
    assert!(app.search.music.loading);
    assert!(!app.search.catalogue_pending && !app.search.playlists_pending);
    assert_eq!(app.search.music.filter, Filter::All);
    let serial = app.search.serial;
    app.demo_music_response(
        serial,
        "Spotify Song".into(),
        Filter::All,
        false,
        Err("YouTube Music unavailable".into()),
    );
    assert!(!app.search.music.loading);
    assert!(
        app.search
            .music
            .cached("Spotify Song", Filter::All)
            .is_none()
    );
    let labels = frame(&ctx, &mut app);
    assert!(labels.iter().any(|label| label == "Retry search"));
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn category_requests_ignore_late_results_and_do_not_wait_for_other_categories() {
    use spotifast::music::Filter;
    let (ctx, mut app, root) = app();
    app.actions.push(Action::Search("Song".into()));
    frame(&ctx, &mut app);
    let serial = app.search.serial;
    app.demo_music_response(
        serial,
        "Song".into(),
        Filter::All,
        false,
        Ok(music_result("Song")),
    );
    assert!(!app.search.music.loading);
    assert_eq!(app.search.music.results.entries[0].title, "Song");
    app.actions.push(Action::MusicFilter(Filter::Albums));
    frame(&ctx, &mut app);
    let current = app.search.serial;
    assert_ne!(current, serial);
    assert!(app.search.music.loading);
    app.demo_music_response(
        serial,
        "Song".into(),
        Filter::All,
        false,
        Ok(music_result("Stale")),
    );
    assert!(app.search.music.results.entries.is_empty());
    app.demo_music_response(
        current,
        "Song".into(),
        Filter::Albums,
        false,
        Ok(music_result("Album")),
    );
    assert_eq!(app.search.music.results.entries[0].title, "Album");
    assert!(!app.search.music.loading);
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn cached_music_is_immediate_and_failed_more_results_keep_the_first_page() {
    use spotifast::music::Filter;
    let (ctx, mut app, root) = app();
    let mut result = music_result("Instant");
    result.continuation = Some("NEXT".into());
    app.search.music.remember("Instant", Filter::All, &result);
    app.actions.push(Action::Search("Instant".into()));
    frame(&ctx, &mut app);
    assert!(!app.search.music.loading);
    assert_eq!(app.search.music.results.entries[0].title, "Instant");
    app.actions.push(Action::MusicMore);
    frame(&ctx, &mut app);
    assert!(app.search.music.loading_more);
    let serial = app.search.serial;
    app.demo_music_response(
        serial,
        "Instant".into(),
        Filter::All,
        true,
        Err("Temporarily unavailable".into()),
    );
    assert!(!app.search.music.loading_more);
    assert_eq!(app.search.music.results.entries[0].title, "Instant");
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
async fn fixture(
    status: &'static str,
    headers: &'static str,
    body: &'static str,
    delay: Duration,
) -> (ApiClient, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client =
        ApiClient::demo_search_client(&format!("http://{}", listener.local_addr().unwrap()));
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut input = vec![0; 16384];
        let size = stream.read(&mut input).await.unwrap();
        let request = String::from_utf8_lossy(&input[..size])
            .lines()
            .next()
            .unwrap()
            .to_owned();
        tokio::time::sleep(delay).await;
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;
        request
    });
    (client, server)
}

#[tokio::test]
async fn search_does_not_wait_through_retry_after_or_send_again_during_cooldown() {
    let (client, server) = fixture(
        "429 Too Many Requests",
        "Retry-After: 30\r\n",
        "{}",
        Duration::ZERO,
    )
    .await;
    let start = Instant::now();
    assert!(matches!(
        client.search("Song", spotifast::search::MUSIC_TYPES).await,
        Err(ApiError::RateLimited)
    ));
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "interactive search retried instead of returning"
    );
    assert!(client.cooling_down().await);
    let start = Instant::now();
    // The fixture server is gone; a network request here would fail differently.
    assert!(matches!(
        client.search("Other", spotifast::search::MUSIC_TYPES).await,
        Err(ApiError::RateLimited)
    ));
    assert!(start.elapsed() < Duration::from_millis(500));
    assert!(server.await.unwrap().starts_with("GET /search?"));
}

#[tokio::test]
async fn music_request_omits_slow_auxiliary_categories() {
    let (client, server) =
        fixture("200 OK", "", r#"{"tracks":{"items":[]}}"#, Duration::ZERO).await;
    client
        .search("Song", spotifast::search::MUSIC_TYPES)
        .await
        .unwrap();
    let request = server.await.unwrap();
    assert!(request.contains("track%2Cartist%2Calbum"));
    for kind in ["playlist", "show", "episode"] {
        assert!(!request.contains(kind));
    }
}

#[tokio::test]
async fn an_unresponsive_search_has_a_short_deadline() {
    let (client, server) = fixture("200 OK", "", "{}", Duration::from_secs(20)).await;
    let start = Instant::now();
    assert!(
        client
            .search("Song", spotifast::search::MUSIC_TYPES)
            .await
            .is_err()
    );
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "search exceeded its deadline"
    );
    server.abort();
}

#[tokio::test]
#[ignore = "Explicit read-only Spotify account timing probe; never refreshes or prints credentials"]
async fn live_account_search_timing() {
    use spotifast::{
        api::ApiSource,
        credentials::{Grant, Slot, Store},
        settings::{ProxyMode, Settings},
    };
    assert_eq!(
        std::env::var("VIBEFY_LIVE_ACCOUNT_SEARCH").as_deref(),
        Ok("1")
    );
    let dirs = AppDirs::installed();
    let settings = Settings::load(&dirs.settings_file());
    if !matches!(settings.proxy_mode, ProxyMode::Off | ProxyMode::System) {
        println!("skipped: manual proxy requires the running application's secret adapter");
        return;
    }
    let http = spotifast::http::Http::from_proxy(&settings.proxy_config().unwrap())
        .unwrap()
        .client()
        .unwrap();
    let store = Store::new(dirs);
    for (slot, source) in [
        (Slot::Personal, ApiSource::Personal),
        (Slot::Shared, ApiSource::Shared),
    ] {
        let loaded = store.lease(slot).load().await.unwrap();
        let Some(Grant::Web(token)) = loaded.grant else {
            continue;
        };
        if token.needs_refresh() {
            println!("skipped {source}: token needs refresh; sign-in untouched");
            continue;
        }
        for types in [
            spotifast::search::MUSIC_TYPES,
            &["track", "artist", "album", "playlist", "show", "episode"][..],
        ] {
            let start = Instant::now();
            let response = http
                .get("https://api.spotify.com/v1/search")
                .bearer_auth(&token.access_token)
                .query(&[
                    ("q", "Metallica"),
                    ("type", &types.join(",")),
                    ("limit", "10"),
                ])
                .timeout(Duration::from_secs(8))
                .send()
                .await
                .unwrap();
            let status = response.status().as_u16();
            let body: serde_json::Value = response.json().await.unwrap();
            let count = body["tracks"]["items"].as_array().map_or(0, Vec::len);
            println!(
                "source={source}, types={}, status={status}, elapsed_ms={}, tracks={count}",
                types.len(),
                start.elapsed().as_millis()
            );
            if status != 200 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(700)).await;
        }
        return;
    }
    println!("no current stored grant was available; no sign-in changes made");
}
