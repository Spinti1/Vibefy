#![cfg(feature = "demo")]

use spotifast::{
    addons,
    api::models::{Page as ApiPage, SearchResults, Track},
    app::{App, AppOptions},
    backend::{ApiResponse, Waker},
    model::{Action, Page, SearchCache},
    paths::AppDirs,
};
use std::time::{Duration, Instant};

fn results(name: &str) -> SearchResults {
    SearchResults {
        tracks: Some(ApiPage {
            items: vec![Track {
                name: name.into(),
                uri: format!("spotify:track:{name}"),
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn search_cache_expires_without_extending_lifetime_on_a_hit() {
    let mut cache = SearchCache::default();
    let now = Instant::now();
    cache.insert("Song", &results("Song"), now);
    assert!(cache.get("song", now).is_none());
    assert!(cache.get("Song", now + Duration::from_secs(119)).is_some());
    assert!(cache.get("Song", now + Duration::from_secs(120)).is_none());
}

#[test]
fn search_cache_is_bounded_and_keeps_the_recently_used_query() {
    let mut cache = SearchCache::default();
    let now = Instant::now();
    for i in 0..8 {
        cache.insert(&i.to_string(), &results(&i.to_string()), now);
    }
    assert!(cache.get("0", now).is_some());
    cache.insert("8", &results("8"), now);
    assert!(cache.get("0", now).is_some());
    assert!(cache.get("1", now).is_none());
}

fn app() -> (egui::Context, App, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("vibefy-fast-paths-{}", rand::random::<u64>()));
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

fn frame(ctx: &egui::Context, app: &mut App, width: f32) -> egui::FullOutput {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 1000.0),
            )),
            ..Default::default()
        },
        |ui| app.frame_ui(ui),
    );
    out.textures_delta.clear();
    out
}

fn search(ctx: &egui::Context, app: &mut App, query: &str) -> u64 {
    app.actions.push(Action::Search(query.into()));
    frame(ctx, app, 1280.0);
    app.search.serial
}

#[test]
fn complete_search_is_instant_on_return_and_rejects_old_responses() {
    use spotifast::music::{Entry, Filter, Kind, Results};
    let result = |name: &str| Results {
        entries: vec![Entry {
            id: name.into(),
            kind: Kind::Song,
            title: name.into(),
            subtitle: "Artist".into(),
            image: None,
            spotify_note: None,
            browse_id: None,
            track: None,
        }],
        continuation: None,
    };
    let (ctx, mut app, root) = app();
    let old = search(&ctx, &mut app, "one");
    app.demo_music_response(old, "one".into(), Filter::All, false, Ok(result("one")));
    let second = search(&ctx, &mut app, "two");
    app.demo_music_response(second, "two".into(), Filter::All, false, Ok(result("two")));
    assert_eq!(
        app.settings.search_history.first().map(String::as_str),
        Some("two")
    );
    let serial = search(&ctx, &mut app, "one");
    assert_ne!(serial, old);
    assert!(!app.search.music.loading);
    assert_eq!(app.search.music.results.entries[0].title, "one");
    app.demo_music_response(old, "one".into(), Filter::All, false, Ok(result("wrong")));
    assert_eq!(app.search.music.results.entries[0].title, "one");
    assert_eq!(
        app.settings.search_history.first().map(String::as_str),
        Some("one")
    );
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn partial_split_search_does_not_enter_the_cache() {
    let (ctx, mut app, root) = app();
    let serial = search(&ctx, &mut app, "partial");
    app.demo_api_response(ApiResponse::SearchStarted {
        query: "partial".into(),
        serial,
        split: true,
    });
    app.demo_api_response(ApiResponse::Search {
        query: "partial".into(),
        serial,
        result: Ok(results("partial")),
    });
    assert!(app.search.cache.get("partial", Instant::now()).is_none());
    app.demo_api_response(ApiResponse::SearchPlaylists {
        query: "partial".into(),
        serial,
        result: Ok(Default::default()),
    });
    assert!(app.search.cache.get("partial", Instant::now()).is_some());
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn soundcloud_button_requires_installation_and_fits_beside_global_search() {
    let (ctx, mut app, root) = app();
    app.open(Page::Home);
    for installed in [false, true] {
        if installed {
            app.settings.addons.set(addons::Id::WaveLink, true);
        }
        for width in [760.0, 1280.0] {
            let mut out = frame(&ctx, &mut app, width);
            for _ in 0..2 {
                out = frame(&ctx, &mut app, width);
            }
            let nodes = &out.platform_output.accesskit_update.as_ref().unwrap().nodes;
            let button = nodes
                .iter()
                .find(|(_, node)| node.label() == Some("SoundCloud search"));
            assert_eq!(button.is_some(), installed);
            if let Some((_, button)) = button {
                let b = button.bounds().unwrap();
                assert!(b.x1 <= f64::from(width) && b.x0 >= 0.0, "{width}: {b:?}");
                let search = nodes
                    .iter()
                    .find(|(_, node)| {
                        node.role() == egui::accesskit::Role::TextInput
                            && node.label().is_some_and(|s| s.contains("Search music"))
                    })
                    .unwrap()
                    .1
                    .bounds()
                    .unwrap();
                assert!(
                    b.x0 >= search.x1 && (b.y0 - search.y0).abs() < 10.0,
                    "button must remain beside search"
                );
            }
        }
    }
    app.open(Page::SoundCloud);
    app.search.query = "spotify query".into();
    app.addons.query = "soundcloud query".into();
    frame(&ctx, &mut app, 1280.0);
    assert_eq!(app.search.query, "spotify query");
    assert_eq!(app.addons.query, "soundcloud query");
    assert!(ctx.memory(|memory| memory.has_focus(egui::Id::new("soundcloud-query"))));
    assert_eq!(
        Page::decode(&Page::SoundCloud.encode()),
        Some(Page::SoundCloud)
    );
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn youtube_import_cannot_replace_soundcloud_results_or_the_saved_selection() {
    let (ctx, mut app, root) = app();
    app.settings.addons.set(addons::Id::WaveLink, true);
    app.addons.fetching_soundcloud = true;
    let soundcloud = addons::Collection {
        title: "SoundCloud result".into(),
        source: "scsearch20:Artist".into(),
        tracks: vec![spotifast::player::LocalTrack {
            uri: "vibefy:soundcloud:https://soundcloud.com/artist/song".into(),
            title: "Song".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    app.demo_source_response(app.addons.request, Ok(soundcloud.clone()));
    let youtube = addons::Collection {
        title: "YouTube import".into(),
        source: "https://www.youtube.com/playlist?list=PL1234567890".into(),
        tracks: vec![spotifast::player::LocalTrack {
            uri: "vibefy:youtube:abcdefghijk".into(),
            title: "Video".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    app.demo_source_response(app.addons.request, Ok(youtube));
    assert_eq!(app.addons.soundcloud_result.as_ref(), Some(&soundcloud));
    app.actions
        .push(Action::Addon(addons::Command::SaveSoundCloud));
    frame(&ctx, &mut app, 1280.0);
    assert!(app.addons.collections.iter().any(|c| c == &soundcloud));
    app.actions.push(Action::Addon(addons::Command::Uninstall(
        addons::Id::WaveLink,
    )));
    frame(&ctx, &mut app, 1280.0);
    assert!(app.addons.soundcloud_result.is_none());
    assert!(app.addons.collections.iter().any(|c| c == &soundcloud));
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn concurrent_play_and_prefetch_reuse_inspected_metadata_and_one_download() {
    use spotifast::player::{LocalTrack, MatchCache, resolve_track_audio};
    use spotifast::settings::ProxyConfig;
    let root = std::env::temp_dir().join(format!("vibefy-audio-fast-{}", rand::random::<u64>()));
    std::fs::create_dir_all(&root).unwrap();
    let helper = root.join(if cfg!(windows) {
        "mock-ytdlp.exe"
    } else {
        "mock-ytdlp"
    });
    let source = root.join("mock.rs");
    std::fs::write(&source, r###"
        use std::io::{Read, Write};
        fn main() {
            let args: Vec<_> = std::env::args().collect();
            let path = std::env::current_exe().unwrap().with_extension("log");
            let mut log = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
            const INFO: &str = r#"{"id":"abcdefghijk","title":"Song","channel":"Artist - Topic","channel_id":"UC-example","track":"Song","artists":["Artist"],"duration":200,"description":"Provided to YouTube by Distributor\nAuto-generated by YouTube."}"#;
            if args.iter().any(|a| a == "--flat-playlist") {
                writeln!(log, "search").unwrap();
                if std::env::current_exe().unwrap().with_extension("slow-search").exists()
                    && args.iter().any(|a| a.contains("official audio")) {
                    std::thread::sleep(std::time::Duration::from_secs(10));
                }
                if std::env::current_exe().unwrap().with_extension("slow-inspection").exists() {
                    println!("{{\"entries\":[{},{}]}}", INFO, INFO.replace("abcdefghijk", "bbbbbbbbbbb"));
                } else { println!("{{\"entries\":[{}]}}", INFO); }
            } else if args.iter().any(|a| a == "--skip-download") {
                writeln!(log, "inspect").unwrap();
                if std::env::current_exe().unwrap().with_extension("slow-inspection").exists()
                    && args.iter().any(|a| a.contains("bbbbbbbbbbb")) {
                    std::thread::sleep(std::time::Duration::from_secs(10));
                }
                println!("{}", INFO);
            } else if args.iter().any(|a| a == "--load-info-json") {
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input).unwrap();
                assert!(input.contains("abcdefghijk") && input.contains("Distributor"));
                writeln!(log, "download-inspected").unwrap();
                if std::env::current_exe().unwrap().with_extension("fail-reuse").exists() {
                    std::process::exit(1);
                }
                std::thread::sleep(std::time::Duration::from_millis(150));
                std::io::stdout().write_all(b"cached-audio-fixture").unwrap();
            } else if std::env::current_exe().unwrap().with_extension("fail-reuse").exists() {
                assert!(args.iter().any(|a| a == "https://www.youtube.com/watch?v=abcdefghijk"));
                writeln!(log, "download-url-fallback").unwrap();
                std::io::stdout().write_all(b"cached-audio-fixture").unwrap();
            } else { panic!("redundant watch-page extraction"); }
        }
    "###).unwrap();
    let output = std::process::Command::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(&helper)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cache = MatchCache::new(&root);
    let track = LocalTrack {
        uri: "spotify:track:fixture".into(),
        title: "Song".into(),
        artists: vec![spotifast::api::models::ArtistRef {
            name: "Artist".into(),
            ..Default::default()
        }],
        duration_ms: 200_000,
        ..Default::default()
    };
    let (first, second) = tokio::join!(
        resolve_track_audio(&helper, &cache, &track, &ProxyConfig::Off),
        resolve_track_audio(&helper, &cache, &track, &ProxyConfig::Off)
    );
    assert_eq!(first.unwrap(), b"cached-audio-fixture");
    assert_eq!(second.unwrap(), b"cached-audio-fixture");
    let log = std::fs::read_to_string(helper.with_extension("log")).unwrap();
    assert_eq!(
        log.lines().filter(|l| *l == "download-inspected").count(),
        1,
        "{log}"
    );
    assert_eq!(log.lines().filter(|l| *l == "inspect").count(), 1, "{log}");
    assert!(
        (1..=2).contains(&log.lines().filter(|l| *l == "search").count()),
        "{log}"
    );
    assert!(cache.has_cached_audio("abcdefghijk"));
    std::fs::write(root.join("youtube_audio/empty.m4a"), []).unwrap();
    assert!(!cache.has_cached_audio("empty"));
    assert!(cache.get_cached_audio("empty").is_none());
    assert!(!cache.has_cached_audio("missing"));
    // An expired inspected stream still gets the original watch-URL path.
    std::fs::write(helper.with_extension("fail-reuse"), []).unwrap();
    let fallback = MatchCache::new(&root.join("fallback"));
    assert_eq!(
        resolve_track_audio(&helper, &fallback, &track, &ProxyConfig::Off)
            .await
            .unwrap(),
        b"cached-audio-fixture"
    );
    let log = std::fs::read_to_string(helper.with_extension("log")).unwrap();
    assert_eq!(
        log.lines()
            .filter(|l| *l == "download-url-fallback")
            .count(),
        1
    );
    assert!(fallback.get_match("fixture").unwrap().matches_track(&track));
    // A fully inspected catalogue result must not wait for a slow sibling search.
    std::fs::remove_file(helper.with_extension("fail-reuse")).unwrap();
    std::fs::write(helper.with_extension("slow-search"), []).unwrap();
    let fast = MatchCache::new(&root.join("fast"));
    let bytes = tokio::time::timeout(
        Duration::from_secs(3),
        resolve_track_audio(&helper, &fast, &track, &ProxyConfig::Off),
    )
    .await
    .expect("Play waited for an unrelated ten-second search")
    .unwrap();
    assert_eq!(bytes, b"cached-audio-fixture");
    assert!(fast.get_match("fixture").unwrap().matches_track(&track));
    std::fs::remove_file(helper.with_extension("slow-search")).unwrap();
    std::fs::write(helper.with_extension("slow-inspection"), []).unwrap();
    let inspected = MatchCache::new(&root.join("fast-inspection"));
    let bytes = tokio::time::timeout(
        Duration::from_secs(3),
        resolve_track_audio(&helper, &inspected, &track, &ProxyConfig::Off),
    )
    .await
    .expect("Play waited for an unrelated ten-second inspection")
    .unwrap();
    assert_eq!(bytes, b"cached-audio-fixture");
    // Aborted child processes release their executable asynchronously on Windows.
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if std::fs::remove_dir_all(&root).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("Cancelled helper process kept the fixture directory open");
}
