use serde_json::json;
use spotifast::addons::{self, Id, Preferences};

#[test]
fn saved_collections_beyond_thirty_survive_reload() {
    let path =
        std::env::temp_dir().join(format!("vibefy-collections-{}.json", rand::random::<u64>()));
    let collections: Vec<_> = (0..45)
        .map(|index| addons::Collection {
            title: format!("Playlist {index}"),
            source: format!("https://www.youtube.com/playlist?list=PL{index}"),
            ..Default::default()
        })
        .collect();
    std::fs::write(&path, serde_json::to_vec(&collections).unwrap()).unwrap();
    let loaded = addons::load_collections(&path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(loaded, collections);
}

#[test]
fn mods_are_opt_in_and_uninstall_does_not_enable_other_mods() {
    let mut preferences: Preferences = serde_json::from_str("{}").unwrap();
    assert!(addons::ALL.iter().all(|&id| !preferences.active(id)));
    preferences.set(Id::Oneko, true);
    preferences.set(Id::Oneko, false);
    assert!(preferences.installed.contains(&Id::Oneko));
    assert!(!preferences.active(Id::Oneko));
    preferences.set(Id::Vivy, true);
    preferences.visualizer = Some(Id::Vivy);
    preferences.uninstall(Id::Vivy);
    assert_eq!(preferences.visualizer, None);
    let restored: Preferences =
        serde_json::from_str(&serde_json::to_string(&preferences).unwrap()).unwrap();
    assert_eq!(preferences, restored);
}
#[test]
fn source_addresses_cannot_invoke_arbitrary_extractors_or_local_services() {
    for uri in [
        "vibefy:youtube:../../file",
        "vibefy:youtube:abcdefghijk;cmd",
        "vibefy:soundcloud:http://soundcloud.com/a/b",
        "vibefy:soundcloud:https://soundcloud.com.evil/a/b",
        "vibefy:soundcloud:https://soundcloud.com@localhost/a/b",
        "vibefy:soundcloud:https://soundcloud.com/a/b?url=http://localhost",
        "vibefy:soundcloud:https://soundcloud.com/a/b/c",
    ] {
        assert!(addons::direct_url(uri).is_none(), "{uri}");
    }
    assert_eq!(
        addons::direct_url("vibefy:youtube:8Zx6RXGNISk").unwrap(),
        "https://www.youtube.com/watch?v=8Zx6RXGNISk"
    );
    assert!(addons::direct_url("vibefy:soundcloud:https://soundcloud.com/artist/song").is_some());
}
#[test]
fn youtube_playlist_import_preserves_exact_video_order_and_duplicates() {
    let collection=addons::parse_collection(&json!({"title":"My playlist","entries":[
        {"id":"8Zx6RXGNISk","title":"I Am Machine","uploader":"Three Days Grace","duration":201},
        {"id":"aaaaaaaaaaa","title":"[Private video]"},
        {"id":"8Zx6RXGNISk","title":"I Am Machine","duration":201},
        {"id":"bbbbbbbbbbb","title":"Live","is_live":true},null
    ]}),true,"source".into()).unwrap();
    assert_eq!(collection.tracks.len(), 2);
    assert_eq!(collection.tracks[0].uri, collection.tracks[1].uri);
    assert_eq!(collection.tracks[0].duration_ms, 201_000);
    assert_eq!(collection.tracks[0].artist_names(), "Three Days Grace");
    if let spotifast::api::models::PlayableItem::Track(track) =
        collection.tracks[0].to_playable_item()
    {
        assert!(
            track.id.is_none(),
            "external sources must not become Spotify IDs"
        );
    }
    assert!(addons::parse_collection(&json!({"entries":[]}), true, "x".into()).is_err());
}
#[test]
fn youtube_playlist_input_is_canonicalized_without_extra_arguments() {
    assert_eq!(
        addons::playlist_url("https://music.youtube.com/watch?v=abc&list=PL1234567890&index=7")
            .unwrap(),
        "https://www.youtube.com/playlist?list=PL1234567890"
    );
    for text in [
        "file:///tmp/foo",
        "https://youtube.com.evil/playlist?list=PL1234567890",
        "https://www.youtube.com/playlist",
        "https://user:pass@youtube.com/playlist?list=PL1234567890",
        "https://youtube.com/playlist?list=../../bad",
    ] {
        assert!(addons::playlist_url(text).is_err(), "{text}");
    }
}
#[test]
fn soundcloud_results_keep_source_and_metadata() {
    let collection=addons::parse_collection(&json!({"entries":[{"webpage_url":"https://soundcloud.com/artist/song","title":"Example","uploader":"Artist","duration":123.4},{"webpage_url":"https://example.com/no","title":"Ignored"}]}),false,"search".into()).unwrap();
    assert_eq!(collection.tracks.len(), 1);
    assert_eq!(collection.tracks[0].duration_ms, 123_400);
    assert_eq!(
        addons::direct_url(&collection.tracks[0].uri).unwrap(),
        "https://soundcloud.com/artist/song"
    );
}

#[test]
fn restricted_sources_and_untrusted_artwork_are_not_imported_as_playable() {
    assert!(
        addons::source_error(b"DRM protected https://secret.example/token")
            .contains("DRM-protected")
    );
    assert!(!addons::source_error(b"error https://secret.example/token").contains("secret"));
    assert!(!addons::safe_art_url("https://localhost/private"));
    assert!(!addons::safe_art_url(
        "https://i.ytimg.com@localhost/private"
    ));
    let collection=addons::parse_collection(&json!({"entries":[{"id":"8Zx6RXGNISk","title":"Good","thumbnail":"http://localhost/private"},{"id":"aaaaaaaaaaa","title":"Protected","is_drm":true},{"id":"bbbbbbbbbbb","title":"Premium","availability":"premium_only"}]}),true,"QA".into()).unwrap();
    assert_eq!(collection.tracks.len(), 1);
    assert!(collection.tracks[0].cover_url.is_none());
}
#[test]
fn beat_detector_is_finite_quiet_at_rest_and_reacts_to_onsets() {
    let mut detector = addons::BeatDetector::default();
    for tick in 0..90 {
        detector.step(0.0, tick as f64 / 30.0, 1.0 / 30.0);
    }
    assert_eq!(detector.pulse, 0.0);
    detector.step(0.6, 3.1, 1.0 / 30.0);
    assert_eq!(detector.pulse, 1.0);
    detector.step(f32::NAN, 3.2, 0.1);
    assert!(detector.pulse.is_finite() && detector.pulse < 1.0);
}
#[test]
fn collection_file_is_separate_and_roundtrips_without_audio() {
    let root = std::env::temp_dir().join(format!("vibefy-addons-{}", rand::random::<u64>()));
    std::fs::create_dir_all(&root).unwrap();
    let collection = addons::parse_collection(
        &json!({"title":"Local","entries":[{"id":"8Zx6RXGNISk","title":"Machine"}]}),
        true,
        "YouTube".into(),
    )
    .unwrap();
    let path = root.join("mod-collections.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&vec![collection.clone()]).unwrap(),
    )
    .unwrap();
    assert_eq!(addons::load_collections(&path), vec![collection]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "Requires public source fixtures, yt-dlp and network access"]
fn live_source_audio_is_playable_by_the_native_decoder_and_cached() {
    use symphonia::core::{
        codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
        meta::MetadataOptions, probe::Hint,
    };
    let fixture_dir = std::path::PathBuf::from(
        std::env::var("VIBEFY_QA_FIXTURES").expect("QA fixture directory"),
    );
    let root =
        std::env::temp_dir().join(format!("vibefy-source-decoder-{}", rand::random::<u64>()));
    std::fs::create_dir_all(&root).unwrap();
    let cache = spotifast::player::MatchCache::new(&root);
    let binary = spotifast::player::find_yt_dlp_binary().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for (file, youtube) in [
        ("youtube-import.json", true),
        ("soundcloud-search.json", false),
    ] {
        let value = serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(fixture_dir.join(file)).unwrap(),
        )
        .unwrap();
        let collection = addons::parse_collection(&value, youtube, "QA".into()).unwrap();
        let track = &collection.tracks[if youtube { 0 } else { 1 }];
        let bytes = runtime
            .block_on(spotifast::player::resolve_track_audio(
                &binary,
                &cache,
                track,
                &spotifast::settings::ProxyConfig::Off,
            ))
            .unwrap();
        let cached = runtime
            .block_on(spotifast::player::resolve_track_audio(
                &binary,
                &cache,
                track,
                &spotifast::settings::ProxyConfig::Off,
            ))
            .unwrap();
        assert_eq!(cached, bytes);
        let mss = MediaSourceStream::new(
            Box::new(std::io::Cursor::new(bytes.clone())),
            Default::default(),
        );
        let mut hint = Hint::new();
        hint.with_extension("m4a");
        let probe = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .unwrap();
        let mut format = probe.format;
        let stream = format.default_track().unwrap();
        let mut decoder = symphonia::default::get_codecs()
            .make(&stream.codec_params, &DecoderOptions::default())
            .unwrap();
        let mut frames = 0;
        for _ in 0..10 {
            let packet = format.next_packet().unwrap();
            if let Ok(audio) = decoder.decode(&packet) {
                frames += audio.frames();
            }
        }
        assert!(frames > 0);
        println!(
            "{}: {} bytes, {} decoded frames, cache hit verified",
            track.title,
            bytes.len(),
            frames
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn jam_invites_and_metadata_reject_other_protocols_and_sources() {
    use spotifast::jam::{Snapshot, parse_invite};
    assert!(parse_invite("ABCD-EFGH-JKLM").is_ok());
    assert!(parse_invite("https://usevibefy.online/app?jam=ABCDEFGHJKLM").is_ok());
    for text in [
        "https://foo",
        "vibefy-jam://host:123/short",
        "vibefy-jam://0.0.0.0:123/0123456789abcdef0123456789abcdef",
        "vibefy-jam://127.0.0.1:0/0123456789abcdef0123456789abcdef",
    ] {
        assert!(parse_invite(text).is_err());
    }
    let snapshot = Snapshot {
        track: spotifast::player::LocalTrack {
            uri: "vibefy:youtube:8Zx6RXGNISk".into(),
            title: "Track".into(),
            ..Default::default()
        },
        position_ms: 50,
        playing: true,
    };
    assert!(snapshot.valid());
    let mut bad = snapshot.clone();
    bad.track.uri = "file:///C:/private.mp3".into();
    assert!(!bad.valid());
    bad = snapshot;
    bad.track.cover_url = Some("http://localhost/private".into());
    assert!(!bad.valid());
}

#[cfg(feature = "demo")]
mod native_ui {
    use super::*;
    use spotifast::app::{App, AppOptions};
    use spotifast::backend::Waker;
    use spotifast::model::{Action, Page};
    fn app() -> (egui::Context, App, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("vibefy-addons-ui-{}", rand::random::<u64>()));
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let waker = Waker::default();
        waker.attach(&ctx);
        let mut app = App::new(
            &waker,
            spotifast::paths::AppDirs {
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
        for entry in spotifast::mod_packages::catalog() {
            let bytes = std::fs::read(format!(
                "{}/mods/packages/{}.vmod",
                env!("CARGO_MANIFEST_DIR"),
                entry.slug
            ))
            .unwrap();
            app.addons.modules.insert(
                entry.id,
                spotifast::mod_packages::decode(entry.id, &bytes).unwrap(),
            );
        }
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
    #[test]
    fn visualizer_selector_and_settings_live_only_in_fullscreen() {
        let (ctx, mut app, root) = app();
        app.actions
            .push(Action::Addon(addons::Command::Enable(Id::Vivy, true)));
        app.actions.push(Action::Open(Page::Mods));
        frame(&ctx, &mut app, 760.0);
        let out = frame(&ctx, &mut app, 760.0);
        let tree = out.platform_output.accesskit_update.unwrap();
        assert!(
            !tree
                .nodes
                .iter()
                .any(|(_, n)| n.value().or(n.label()) == Some("Intensity"))
        );
        app.actions.push(Action::Addon(addons::Command::Fullscreen));
        frame(&ctx, &mut app, 760.0);
        let out = frame(&ctx, &mut app, 760.0);
        let tree = out.platform_output.accesskit_update.unwrap();
        assert!(
            tree.nodes
                .iter()
                .any(|(_, n)| n.value().or(n.label()) == Some("Settings"))
        );
        assert!(
            tree.nodes
                .iter()
                .any(|(_, n)| n.value().or(n.label()) == Some("vivy"))
        );
        app.actions
            .push(Action::Addon(addons::Command::ChooseVisualizer(
                Id::Backmusic,
            )));
        frame(&ctx, &mut app, 760.0);
        assert_eq!(app.settings.addons.visualizer, Some(Id::Vivy));
        app.actions
            .push(Action::Addon(addons::Command::Enable(Id::Backmusic, true)));
        frame(&ctx, &mut app, 760.0);
        app.actions
            .push(Action::Addon(addons::Command::ConfigureVisual(
                addons::VisualConfig {
                    style: 1,
                    intensity: 2.0,
                    ..Default::default()
                },
            )));
        frame(&ctx, &mut app, 760.0);
        app.actions
            .push(Action::Addon(addons::Command::ChooseVisualizer(Id::Vivy)));
        frame(&ctx, &mut app, 760.0);
        assert_eq!(app.settings.addons.visual(Id::Vivy).intensity, 1.0);
        assert_eq!(app.settings.addons.visual(Id::Backmusic).intensity, 2.0);
        assert!(app.addons.fullscreen);
        app.backend.shutdown();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_visualizers_fit_next_to_volume_and_disable_restores_view() {
        let (ctx, mut app, root) = app();
        app.actions
            .push(Action::Addon(addons::Command::Enable(Id::Vivy, true)));
        frame(&ctx, &mut app, 1280.0);
        for width in [760.0, 800.0, 1280.0] {
            frame(&ctx, &mut app, width);
            let out = frame(&ctx, &mut app, width);
            let tree = out.platform_output.accesskit_update.unwrap();
            let button = |label: &str| {
                tree.nodes
                    .iter()
                    .find_map(|(_, n)| {
                        (n.role() == egui::accesskit::Role::Button && n.label() == Some(label))
                            .then(|| n.bounds().unwrap())
                    })
                    .unwrap_or_else(|| panic!("Missing {label}"))
            };
            let full = button("Fullscreen");
            let volume = tree
                .nodes
                .iter()
                .find_map(|(_, n)| {
                    (n.role() == egui::accesskit::Role::Slider && n.label() == Some("Volume (%)"))
                        .then(|| n.bounds().unwrap())
                })
                .unwrap();
            assert!(
                full.x0 >= volume.x1 && full.x1 <= width as f64,
                "fullscreen right of volume at {width}"
            );
            let lyrics = button("Lyrics");
            let repeat = tree
                .nodes
                .iter()
                .find_map(|(_, n)| {
                    (n.role() == egui::accesskit::Role::Button
                        && n.label().is_some_and(|s| s.starts_with("Repeat")))
                    .then(|| n.bounds().unwrap())
                })
                .unwrap();
            assert!(
                lyrics.x0 >= repeat.x1,
                "transport overlaps at {width}: {lyrics:?} {repeat:?}"
            );
        }
        app.actions.push(Action::Addon(addons::Command::Fullscreen));
        frame(&ctx, &mut app, 1280.0);
        assert!(app.addons.fullscreen);
        app.actions
            .push(Action::Addon(addons::Command::Enable(Id::Vivy, false)));
        frame(&ctx, &mut app, 1280.0);
        assert!(!app.addons.fullscreen);
        app.actions.push(Action::Open(Page::Mods));
        frame(&ctx, &mut app, 760.0);
        let out = frame(&ctx, &mut app, 760.0);
        let tree = out.platform_output.accesskit_update.unwrap();
        assert!(
            tree.nodes
                .iter()
                .any(|(_, n)| n.value().or(n.label()) == Some("YouTube Playlist Import"))
        );
        assert!(tree.nodes.iter().any(|(_, n)| n.label() == Some("Install")));
        app.backend.shutdown();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn all_visual_modes_render_in_both_themes_and_leave_audio_settings_alone() {
        let (ctx, mut app, root) = app();
        let eq = (
            app.settings.eq_on,
            app.settings.eq_preamp_db,
            app.settings.eq_bands_db,
        );
        for theme in ["light", "dark"] {
            for mode in ["mods-vivy", "mods-backmusic"] {
                spotifast::demo::apply_flags(
                    &mut app,
                    Some("mods"),
                    Some(&format!("{theme},{mode}")),
                );
                for style in [0, 1] {
                    let id = app.settings.addons.visualizer.unwrap();
                    app.settings.addons.visual_mut(id).style = style;
                    let out = frame(&ctx, &mut app, 760.0);
                    assert!(!out.shapes.is_empty());
                    frame(&ctx, &mut app, 760.0);
                    if app.settings.addons.visualizer == Some(Id::Vivy) {
                        assert!(!app.addons.ridges.is_empty(), "live audio must reach Vivy");
                    }
                    assert!(
                        app.addons.bands.iter().any(|level| *level > 0.01),
                        "visualizer must respond to non-silent samples"
                    );
                }
            }
        }
        assert_eq!(
            (
                app.settings.eq_on,
                app.settings.eq_preamp_db,
                app.settings.eq_bands_db
            ),
            eq
        );
        app.backend.shutdown();
        std::fs::remove_dir_all(root).unwrap();
    }
}
