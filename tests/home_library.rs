#![cfg(feature = "demo")]
use spotifast::{
    addons,
    api::models::{Page as ApiPage, SavedTrack, Track},
    app::{App, AppOptions},
    backend::{ApiResponse, Waker},
    model::{Action, Loadable, Page},
    paths::AppDirs,
};

fn app() -> (egui::Context, App, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("vibefy-home-{}", rand::random::<u64>()));
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
fn frame(
    ctx: &egui::Context,
    app: &mut App,
    width: f32,
    height: f32,
    scroll: f32,
) -> egui::FullOutput {
    let events = if scroll == 0.0 {
        vec![]
    } else {
        vec![
            egui::Event::PointerMoved(egui::pos2(width - 80.0, 300.0)),
            egui::Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, scroll),
                modifiers: Default::default(),
            },
        ]
    };
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            events,
            ..Default::default()
        },
        |ui| app.frame_ui(ui),
    );
    out.textures_delta.clear();
    out
}
fn labels(out: &egui::FullOutput) -> Vec<String> {
    out.platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, n)| n.label().or(n.value()).map(str::to_owned))
        .collect()
}
fn source() -> addons::Collection {
    addons::Collection {
        title: "Imported Rock".into(),
        image: None,
        owner: None,
        source: "https://www.youtube.com/playlist?list=PL1234567890".into(),
        tracks: vec![spotifast::player::LocalTrack {
            uri: "vibefy:youtube:8Zx6RXGNISk".into(),
            title: "Exact imported video".into(),
            name: "Exact imported video".into(),
            duration_ms: 201_000,
            ..Default::default()
        }],
    }
}
fn page(offset: u32, total: u32) -> ApiPage<SavedTrack> {
    let count = 50.min(total.saturating_sub(offset));
    ApiPage {
        offset,
        limit: 50,
        total,
        next: (offset + count < total).then(|| "next".into()),
        items: (offset..offset + count)
            .map(|i| SavedTrack {
                track: Track {
                    uri: format!("spotify:track:{i}"),
                    name: format!("Saved song {i}"),
                    ..Default::default()
                },
                ..Default::default()
            })
            .collect(),
    }
}
#[test]
fn liked_loads_all_pages_without_sorting_or_scrolling_and_stops_on_failure() {
    let (_ctx, mut app, root) = app();
    let generation = app.demo_liked_cache(None);
    let account = app.user.as_ref().unwrap().id.clone();
    assert!(app.library.liked.loading);
    for offset in [0, 50, 100, 150, 200] {
        app.demo_api_response(ApiResponse::SavedTracks {
            offset,
            generation,
            account_id: Some(account.clone()),
            result: Ok(page(offset, 237)),
        });
        assert_eq!(
            app.library.liked.items.len(),
            ((offset + 50).min(237)) as usize
        );
        assert_eq!(
            app.library.liked.loading,
            offset < 200,
            "automatically continues next page"
        );
    }
    assert!(app.library.liked.is_complete());
    assert_eq!(
        app.library.liked.items.last().unwrap().track.name,
        "Saved song 236"
    );
    // Late responses from an older request or a different account cannot replace rows.
    app.demo_api_response(ApiResponse::SavedTracks {
        offset: 0,
        generation: generation.wrapping_sub(1),
        account_id: Some(account.clone()),
        result: Ok(page(0, 1)),
    });
    app.demo_api_response(ApiResponse::SavedTracks {
        offset: 0,
        generation,
        account_id: Some("different-account".into()),
        result: Ok(page(0, 1)),
    });
    assert_eq!(app.library.liked.items.len(), 237);
    let generation = app.demo_liked_cache(None);
    app.demo_api_response(ApiResponse::SavedTracks {
        offset: 0,
        generation,
        account_id: Some(account),
        result: Err(spotifast::api::ApiError::Network(
            "temporary failure".into(),
        )),
    });
    assert!(!app.library.liked.loading);
    assert!(app.library.liked.error.is_some());
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn fresh_partial_liked_cache_continues_and_complete_cache_needs_no_request() {
    let (_ctx, mut app, root) = app();
    let account = app.user.as_ref().unwrap().id.clone();
    for total in [137, 50] {
        let mut songs = spotifast::liked::LikedSongs::default();
        songs.cache_checked = true;
        songs.start_refresh(1);
        songs.absorb(0, page(0, total), jiff::Timestamp::now().as_second());
        let cache = songs.checkpoint(account.clone(), true).unwrap();
        let generation = app.demo_liked_cache(Some(cache));
        assert_eq!(app.library.liked.items.len(), 50);
        assert_eq!(app.library.liked.loading, total > 50);
        if total > 50 {
            for offset in [50, 100] {
                app.demo_api_response(ApiResponse::SavedTracks {
                    offset,
                    generation,
                    account_id: Some(account.clone()),
                    result: Ok(page(offset, total)),
                });
            }
        }
        assert!(app.library.liked.is_complete());
        assert_eq!(app.library.liked.items.len(), total as usize);
    }
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn youtube_import_saves_once_opens_on_home_and_survives_disabling_the_addon() {
    let (ctx, mut app, root) = app();
    // Keep transport in demo mode, but exercise the real asynchronous file writer.
    app.offline = false;
    app.home.recently_played = Loadable::Loaded(vec![]);
    app.demo_source_response(app.addons.request.wrapping_add(1), Ok(source()));
    assert!(app.addons.collections.is_empty(), "stale import rejected");
    app.demo_source_response(app.addons.request, Ok(source()));
    app.demo_source_response(app.addons.request, Ok(source()));
    assert_eq!(
        app.addons.collections.len(),
        1,
        "same source updates existing import"
    );
    assert!(app.flush_files());
    let restored = addons::load_collections(&root.join("state/mod-collections.json"));
    assert_eq!(restored, app.addons.collections);
    app.settings.addons.uninstall(addons::Id::YouTube);
    app.actions.push(Action::Open(Page::Home));
    frame(&ctx, &mut app, 1280.0, 1000.0, 0.0);
    let out = frame(&ctx, &mut app, 1280.0, 1000.0, 0.0);
    assert!(labels(&out).iter().any(|s| s == "Imported Rock"));
    let local_page = Page::LocalCollection(source().source);
    assert_eq!(Page::decode(&local_page.encode()), Some(local_page.clone()));
    app.actions.push(Action::Open(local_page.clone()));
    frame(&ctx, &mut app, 760.0, 1000.0, 0.0);
    let out = frame(&ctx, &mut app, 760.0, 1000.0, 0.0);
    let text = labels(&out);
    assert!(text.iter().any(|s| s == "Play"));
    assert!(text.iter().any(|s| s.contains("Exact imported video")));
    assert_eq!(app.page(), &local_page);
    app.actions
        .push(Action::Addon(addons::Command::DeleteCollection(0)));
    frame(&ctx, &mut app, 760.0, 1000.0, 0.0);
    assert!(app.addons.collections.is_empty());
    assert_eq!(app.page(), &Page::Home);
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn an_existing_import_can_be_updated_at_the_saved_collection_limit() {
    let (ctx, mut app, root) = app();
    app.addons.collections = (0..30)
        .map(|i| addons::Collection {
            source: format!("local-source-{i}"),
            ..source()
        })
        .collect();
    let existing = app.addons.collections[7].source.clone();
    app.addons.result = Some(addons::Collection {
        source: existing,
        title: "Updated playlist".into(),
        ..source()
    });
    app.actions
        .push(Action::Addon(addons::Command::SaveCollection));
    frame(&ctx, &mut app, 1280.0, 1000.0, 0.0);
    assert_eq!(app.addons.collections.len(), 30);
    assert_eq!(app.addons.collections[7].title, "Updated playlist");
    app.demo_source_response(app.addons.request, Ok(source()));
    assert_eq!(app.addons.collections.len(), 30);
    assert!(app.addons.message.as_ref().unwrap().contains("limit"));
    assert!(app.addons.result.is_some());
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn home_reaches_playlists_beyond_seven_and_virtualizes_large_libraries() {
    let (ctx, mut app, root) = app();
    app.home.recently_played = Loadable::Loaded(vec![]);
    let template = app.library.playlists.get().unwrap()[0].clone();
    app.library.playlists = Loadable::Loaded(
        (0..120)
            .map(|i| spotifast::api::models::Playlist {
                id: format!("home-{i}"),
                uri: format!("spotify:playlist:home-{i}"),
                name: format!("Home playlist {i}"),
                ..template.clone()
            })
            .collect(),
    );
    app.actions.push(Action::Open(Page::Home));
    frame(&ctx, &mut app, 760.0, 1000.0, 0.0);
    let mut found = std::collections::HashSet::new();
    for _ in 0..32 {
        let out = frame(&ctx, &mut app, 760.0, 1000.0, -450.0);
        let text = labels(&out);
        let playlist_labels: Vec<_> = text
            .iter()
            .filter(|s| s.starts_with("Home playlist "))
            .collect();
        assert!(
            playlist_labels.len() < 60,
            "only visible grid rows are drawn"
        );
        for label in playlist_labels {
            found.insert(label.to_string());
        }
    }
    assert!(
        found.contains("Home playlist 119"),
        "last library playlist reachable by scrolling: {found:?}"
    );
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn mods_catalogue_has_one_ordered_grid_and_collapses_optional_tools() {
    let (ctx, mut app, root) = app();
    for width in [760.0, 1280.0] {
        app.actions.push(Action::Open(Page::Mods));
        frame(&ctx, &mut app, width, 1000.0, 0.0);
        let out = frame(&ctx, &mut app, width, 1000.0, 0.0);
        let text = labels(&out);
        let youtube = text
            .iter()
            .position(|s| s == "YouTube Playlist Import")
            .unwrap();
        let wave = text.iter().position(|s| s == "WaveLink").unwrap();
        assert!(youtube < wave);
        assert!(text.iter().any(|s| s == "Listen Together"));
        assert!(!text.iter().any(|s| s == "Intensity"));
    }
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_imports_keep_artwork_and_new_imports_keep_playlist_metadata() {
    let old = serde_json::json!({"title":"Old import","source":"https://www.youtube.com/playlist?list=PL1234567890","tracks":[{"uri":"vibefy:youtube:8Zx6RXGNISk","name":"Video","title":"Video","album":"","album_name":"","artists":[],"is_episode":false,"duration_ms":201000,"cover_url":"https://i.ytimg.com/vi/8Zx6RXGNISk/hqdefault.jpg"}]});
    let restored: addons::Collection = serde_json::from_value(old).unwrap();
    assert_eq!(
        restored.cover(),
        Some("https://i.ytimg.com/vi/8Zx6RXGNISk/hqdefault.jpg")
    );
    assert!(restored.owner.is_none());
    let value = serde_json::json!({"title":"Rock import","uploader":"Playlist owner","thumbnails":[{"url":"http://localhost/private"},{"url":"https://i.ytimg.com/vi/8Zx6RXGNISk/hqdefault.jpg"}],"entries":[{"id":"8Zx6RXGNISk","title":"Video","duration":201,"uploader":"Artist","thumbnail":"https://i.ytimg.com/vi/8Zx6RXGNISk/hqdefault.jpg"}]});
    let parsed = addons::parse_collection(
        &value,
        true,
        "https://www.youtube.com/playlist?list=PL1234567890".into(),
    )
    .unwrap();
    assert_eq!(parsed.owner.as_deref(), Some("Playlist owner"));
    assert_eq!(parsed.image.as_deref(), restored.cover());
    let roundtrip: addons::Collection =
        serde_json::from_slice(&serde_json::to_vec(&parsed).unwrap()).unwrap();
    assert_eq!(roundtrip, parsed);
}

#[test]
fn source_playlist_uses_shared_hero_controls_and_table_in_both_themes() {
    let (ctx, mut app, root) = app();
    app.demo_source_response(app.addons.request, Ok(source()));
    let page = Page::LocalCollection(source().source);
    for light in [false, true] {
        spotifast::demo::apply_flags(&mut app, None, Some(if light { "light" } else { "dark" }));
        for width in [760.0, 1280.0] {
            app.actions.push(Action::Open(page.clone()));
            frame(&ctx, &mut app, width, 1000.0, 0.0);
            let out = frame(&ctx, &mut app, width, 1000.0, 0.0);
            let text = labels(&out);
            assert!(text.iter().any(|s| s == "Playlist"));
            assert!(text.iter().any(|s| s == "Imported Rock"));
            assert!(text.iter().any(|s| s == "Play"));
            assert!(text.iter().any(|s| s == "Shuffle" || s == "Shuffle off"));
            assert!(text.iter().any(|s| s == "More"));
            assert!(text.iter().any(|s| s == "Filter"));
            assert!(text.iter().any(|s| s.contains("Exact imported video")));
            assert!(
                !text.iter().any(|s| s == "Play all"),
                "old centered button list is gone"
            );
            let tree = out.platform_output.accesskit_update.as_ref().unwrap();
            let play = tree
                .nodes
                .iter()
                .find(|(_, n)| n.label() == Some("Play"))
                .unwrap()
                .1
                .bounds()
                .unwrap();
            let row = tree
                .nodes
                .iter()
                .find(|(_, n)| {
                    n.label()
                        .is_some_and(|s| s.starts_with("Play Exact imported video"))
                })
                .unwrap()
                .1
                .bounds()
                .unwrap();
            assert!(
                row.y0 > play.y1 && row.x0 >= 250.0 && row.x1 <= width as f64,
                "table below hero/actions at {width}"
            );
        }
    }
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_table_playback_keeps_filtered_order_and_the_chosen_duplicate() {
    use spotifast::model::RowContext;
    use std::sync::Arc;
    let (ctx, mut app, root) = app();
    let mut collection = source();
    let mut second = collection.tracks[0].clone();
    second.uri = "vibefy:youtube:abcdefghijk".into();
    second.title = "Second video".into();
    second.name = "Second video".into();
    collection.tracks.push(second);
    collection.tracks.push(collection.tracks[0].clone());
    let source = collection.source.clone();
    app.demo_source_response(app.addons.request, Ok(collection));
    app.local_ready = true;
    let uris: Arc<[String]> = Arc::from(vec![
        "vibefy:youtube:abcdefghijk".into(),
        "vibefy:youtube:8Zx6RXGNISk".into(),
        "vibefy:youtube:8Zx6RXGNISk".into(),
    ]);
    let context = RowContext::View {
        uris: uris.clone(),
        context_uri: Page::LocalCollection(source.clone()).encode(),
        editable_playlist: None,
    };
    app.actions.push(Action::PlayFromRow {
        context: context.clone(),
        uri: uris[2].clone(),
        index: 2,
    });
    frame(&ctx, &mut app, 1280.0, 1000.0, 0.0);
    assert_eq!(
        app.addons
            .active_tracks
            .iter()
            .map(|t| t.uri.clone())
            .collect::<Vec<_>>(),
        uris.to_vec()
    );
    assert!(app.row_is_played_copy(&context, 2, &uris[2]));
    assert!(!app.row_is_played_copy(&context, 1, &uris[1]));
    assert_eq!(
        app.addons.active_collection,
        Some(Page::LocalCollection(source.clone()).encode())
    );
    // An injected row not present in this source must not change the active queue.
    app.actions.push(Action::PlayFromRow {
        context: RowContext::View {
            uris: Arc::from(vec!["vibefy:youtube:zzzzzzzzzzz".into()]),
            context_uri: Page::LocalCollection(source).encode(),
            editable_playlist: None,
        },
        uri: "vibefy:youtube:zzzzzzzzzzz".into(),
        index: 0,
    });
    frame(&ctx, &mut app, 1280.0, 1000.0, 0.0);
    assert_eq!(app.addons.active_tracks.len(), 3);
    app.backend.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
