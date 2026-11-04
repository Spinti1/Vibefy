#![cfg(feature = "demo")]
pub use spotifast::theme;
use spotifast::{
    addons::{self, Command, Id},
    app::{App, AppOptions},
    backend::Waker,
    model::{Action, Page},
    paths::AppDirs,
    settings::{Settings, ThemeChoice},
};

fn setup() -> (egui::Context, App, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("vibefy-0414-{}", rand::random::<u64>()));
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
    scroll: f32,
) -> egui::accesskit::TreeUpdate {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 1000.0),
            )),
            events: if scroll == 0.0 {
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
            },
            ..Default::default()
        },
        |ui| app.frame_ui(ui),
    );
    fn collect(shape: &egui::epaint::Shape, text: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::epaint::Shape::Text(shape) => text.push((
                shape.galley.job.text.clone(),
                shape.galley.rect.translate(shape.pos.to_vec2()),
            )),
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, text);
                }
            }
            _ => {}
        }
    }
    let mut painted = Vec::new();
    for shape in &out.shapes {
        collect(&shape.shape, &mut painted);
    }
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("test-painted-labels"), painted));
    out.textures_delta.clear();
    out.platform_output.accesskit_update.take().unwrap()
}
fn labels(tree: &egui::accesskit::TreeUpdate) -> Vec<String> {
    tree.nodes
        .iter()
        .filter_map(|(_, n)| n.label().or(n.value()).map(str::to_owned))
        .collect()
}
fn finish(mut app: App, root: std::path::PathBuf) {
    app.backend.shutdown();
    drop(app);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn old_settings_drop_removed_skin_and_mod_choices_without_resetting_other_preferences() {
    let settings: Settings = serde_json::from_value(serde_json::json!({
        "volume": 63, "skin": "Other.wsz", "random_skin": true, "skin_scale": 3,
        "addons": {"installed": ["ListenTogether", "Oneko", "TrackPeek"], "enabled": ["ListenTogether", "Oneko"]}
    })).unwrap();
    assert_eq!(settings.volume, 63);
    assert_eq!(settings.skin_scale, Some(3));
    assert!(settings.addons.active(Id::Oneko));
    assert!(settings.addons.installed.contains(&Id::TrackPeek));
    assert!(!settings.addons.active(Id::TrackPeek));
    assert_eq!(settings.addons.cat_scale(), 0.9);
    let json = serde_json::to_value(&settings).unwrap();
    assert!(json.get("skin").is_none() && json.get("random_skin").is_none());
    assert!(!json["addons"].to_string().contains("ListenTogether"));
    assert!(addons::ALL.iter().all(|id| id.name() != "Listen Together"));
}

#[test]
fn listen_together_is_available_without_installation_and_does_not_open_a_session_on_view() {
    for width in [760.0, 1280.0] {
        for theme in [ThemeChoice::Dark, ThemeChoice::Light] {
            let (ctx, mut app, root) = setup();
            app.settings.theme = theme;
            app.actions.push(Action::SettingsChanged);
            app.show_queue_panel = true;
            app.actions.push(Action::ToggleListenTogether);
            // Floating egui windows first measure their content invisibly.
            for _ in 0..3 {
                frame(&ctx, &mut app, width, 0.0);
            }
            let tree = frame(&ctx, &mut app, width, 0.0);
            let text = labels(&tree);
            assert!(app.show_listen_together && !app.show_queue_panel);
            for label in ["Listen Together", "Host a session", "Join session"] {
                assert!(
                    text.iter().any(|s| s == label),
                    "Missing {label} at {width}: {text:?}"
                );
            }
            assert!(
                text.iter()
                    .all(|s| s != "Listening activity" && s != "Install")
            );
            assert!(app.settings.addons.installed.is_empty());
            assert!(app.addons.jam.is_none());
            app.actions
                .push(Action::Addon(Command::Join("invalid invitation".into())));
            frame(&ctx, &mut app, width, 0.0);
            assert!(app.addons.jam.is_none());
            assert!(app.addons.jam_message.is_some());
            assert!(app.addons.message.is_none());
            app.actions.push(Action::ToggleQueuePanel);
            frame(&ctx, &mut app, width, 0.0);
            assert!(app.show_queue_panel && !app.show_listen_together);
            finish(app, root);
        }
    }
}

#[test]
fn cat_size_is_persisted_bounded_and_only_exposed_for_installed_cat_jam() {
    let (ctx, mut app, root) = setup();
    app.open(Page::Mods);
    app.addons.filter = "Cat Jam".into();
    let text = labels(&frame(&ctx, &mut app, 760.0, 0.0));
    assert!(!text.iter().any(|s| s == "Reset size"));
    app.actions.push(Action::Addon(Command::CatJamScale(1.5)));
    frame(&ctx, &mut app, 760.0, 0.0);
    assert_eq!(app.settings.addons.cat_scale(), 0.9);
    app.actions
        .push(Action::Addon(Command::Enable(Id::CatJam, true)));
    frame(&ctx, &mut app, 760.0, 0.0);
    let text = labels(&frame(&ctx, &mut app, 760.0, 0.0));
    assert!(text.iter().any(|s| s == "Size") && text.iter().any(|s| s == "Reset size"));
    for (input, expected) in [(1.5, 1.5), (100.0, 2.0), (-2.0, 0.5), (f32::NAN, 0.9)] {
        app.actions.push(Action::Addon(Command::CatJamScale(input)));
        frame(&ctx, &mut app, 760.0, 0.0);
        assert_eq!(app.settings.addons.cat_scale(), expected);
    }
    app.actions
        .push(Action::Addon(Command::Enable(Id::CatJam, false)));
    frame(&ctx, &mut app, 760.0, 0.0);
    assert!(
        labels(&frame(&ctx, &mut app, 760.0, 0.0))
            .iter()
            .any(|s| s == "Reset size")
    );
    let again: Settings =
        serde_json::from_str(&serde_json::to_string(&app.settings).unwrap()).unwrap();
    assert_eq!(again.addons.cat_scale(), 0.9);
    finish(app, root);
}

#[test]
fn original_cat_art_reaches_the_window_edges_at_every_supported_size() {
    #[path = "../src/mod_animation.rs"]
    mod cat;
    let image = image::load_from_memory(cat::CAT_JAM).unwrap().into_rgba8();
    for scale in [0.5, 0.9, 1.0, 1.5, 2.0] {
        for size in [egui::vec2(760.0, 1000.0), egui::vec2(1280.0, 720.0)] {
            let viewport = egui::Rect::from_min_size(egui::pos2(11.0, 7.0), size);
            let rect = cat::cat_rect(viewport, scale);
            assert_eq!(rect.left(), viewport.left());
            assert_eq!(
                rect.bottom(),
                viewport.bottom() - spotifast::theme::PLAYER_BAR_HEIGHT
            );
            assert!((rect.width() - cat::CAT_WIDTH as f32 * scale).abs() < 0.001);
        }
    }
    for frame in 0..cat::CAT_FRAMES {
        let x = (frame % cat::CAT_COLUMNS * cat::CAT_WIDTH) as u32;
        let y = (frame / cat::CAT_COLUMNS * cat::CAT_HEIGHT + cat::CAT_HEIGHT - 1) as u32;
        assert!(
            (x..x + cat::CAT_WIDTH as u32).any(|x| image.get_pixel(x, y)[3] > 16),
            "Transparent bottom gap in frame {frame}"
        );
    }
}

#[test]
fn lyrics_provider_sync_and_connect_are_compact_at_the_top_left() {
    for width in [760.0, 1280.0] {
        let (ctx, mut app, root) = setup();
        spotifast::demo::apply_flags(&mut app, None, Some("spicy-plain"));
        for _ in 0..2 {
            frame(&ctx, &mut app, width, 0.0);
        }
        let tree = frame(&ctx, &mut app, width, 0.0);
        for label in [
            "Plain lyrics · Demo lyrics",
            "Sync",
            "Connect Spicy Lyrics for additional word-synced lyrics",
        ] {
            let painted = ctx
                .data(|data| {
                    data.get_temp::<Vec<(String, egui::Rect)>>(egui::Id::new("test-painted-labels"))
                })
                .unwrap();
            let bounds = painted
                .iter()
                .find(|(text, _)| text == label)
                .map(|(_, rect)| *rect)
                .unwrap_or_else(|| panic!("Missing {label}: {:?}", labels(&tree)));
            assert!(
                bounds.top() < 200.0 && bounds.left() < 80.0,
                "Misplaced {label}: {bounds:?}"
            );
            assert!(bounds.height() < 28.0, "Oversized info: {bounds:?}");
        }
        finish(app, root);
    }
}

#[test]
fn mini_player_is_branded_and_settings_do_not_advertise_removed_skin_features() {
    let skin = spotifast::skin::Skin::builtin();
    assert_eq!(skin.name, "Vibefy");
    assert_eq!(skin.playlist.current, [0, 210, 224]);
    for (_, sprite) in spotifast::skin::sprites::ALL {
        assert!(skin.sprite(*sprite).is_some(), "Missing sprite {sprite:?}");
    }
    for width in [760.0, 1280.0] {
        let (ctx, mut app, root) = setup();
        app.open(Page::Settings);
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new("settings-filter"), "Mini player".to_owned())
        });
        let mut text = vec![];
        for step in 0..6 {
            text.extend(labels(&frame(
                &ctx,
                &mut app,
                width,
                if step == 0 { 0.0 } else { -350.0 },
            )));
        }
        assert!(text.iter().any(|s| s == "Switch to it"));
        for removed in [
            "Skin Museum",
            "Installed skins",
            "Random",
            "Spotifast",
            "Drop a skin",
        ] {
            assert!(
                text.iter().all(|s| !s.contains(removed)),
                "Removed control {removed} still present"
            );
        }
        finish(app, root);
    }
}

#[test]
#[ignore = "opens a local TCP session; run explicitly for network validation"]
fn built_in_host_works_without_mod_flags_and_panel_closure_preserves_session() {
    use std::time::{Duration, Instant};
    let (ctx, mut app, root) = setup();
    app.actions.push(Action::Addon(Command::Host));
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(app.settings.addons.installed.is_empty());
    let deadline = Instant::now() + Duration::from_secs(4);
    while app
        .addons
        .jam
        .as_ref()
        .unwrap()
        .shared
        .lock()
        .unwrap()
        .invite
        .is_empty()
    {
        assert!(Instant::now() < deadline, "Host did not become ready");
        std::thread::sleep(Duration::from_millis(10));
    }
    app.actions.push(Action::ToggleListenTogether);
    frame(&ctx, &mut app, 1280.0, 0.0);
    app.actions.push(Action::ToggleListenTogether);
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(app.addons.jam.is_some() && !app.show_listen_together);
    app.actions.push(Action::Addon(Command::Leave));
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(app.addons.jam.is_none() && app.addons.pending_sync.is_none());
    finish(app, root);
}
