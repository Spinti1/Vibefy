use spotifast::{
    addons,
    api::models::{Playlist, User},
    model::{Action, Loadable},
};
fn fixture() -> spotifast::app::App {
    let root = std::env::temp_dir().join(format!("vibefy-hidden-{}", std::process::id()));
    let mut app = spotifast::app::App::new(
        &spotifast::backend::Waker::default(),
        spotifast::paths::AppDirs {
            config: root.join("config"),
            state: root.join("state"),
            cache: root.join("cache"),
        },
        Default::default(),
        spotifast::app::AppOptions {
            restore_sign_in: false,
            media_controls: false,
            tray: false,
            autostart: false,
        },
    );
    app.offline = true;
    app.auth = spotifast::backend::AuthStatus::Connected {
        username: "example".into(),
    };
    app.user = Some(User {
        id: "example".into(),
        ..Default::default()
    });
    app.music_accounts[0].connected = true;
    app.settings.addons.installed.insert(addons::Id::Vivy);
    app.home.recently_played = Loadable::Loaded(vec![]);
    app.home.top_artists = Loadable::Loaded(vec![]);
    app.settings.home.made_for_you.visible = false;
    app.library.playlists = Loadable::Loaded(vec![Playlist {
        id: "secret".into(),
        uri: "spotify:playlist:secret".into(),
        name: "Hidden example".into(),
        ..Default::default()
    }]);
    app
}
fn frame(app: &mut spotifast::app::App, ctx: &egui::Context) -> egui::FullOutput {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200., 900.),
            )),
            ..Default::default()
        },
        |ui| app.frame_ui(ui),
    )
}
#[test]
fn remove_from_library_updates_immediately_and_recent_history_cannot_restore_card() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    spotifast::theme::install(&ctx);
    let playlist = app.library.playlists.get().unwrap()[0].clone();
    app.playlist_pages
        .entry(playlist.id.clone())
        .or_default()
        .playlist = Loadable::Loaded(playlist.clone());
    app.saved.insert(playlist.uri.clone(), true);
    app.home.recently_played = Loadable::Loaded(vec![spotifast::api::models::PlayHistory {
        context: Some(spotifast::api::models::Context {
            uri: playlist.uri.clone(),
            kind: "playlist".into(),
        }),
        ..Default::default()
    }]);
    app.actions.push(Action::ToggleSaved(playlist.uri.clone()));
    frame(&mut app, &ctx);
    assert_eq!(app.is_saved(&playlist.uri), Some(false));
    assert!(app.library.playlists.get().unwrap().is_empty());
    assert!(
        app.library
            .playlists_edits
            .iter()
            .any(|e| e.uri == playlist.uri && e.added.is_none())
    );
    let out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200., 2400.),
            )),
            ..Default::default()
        },
        |ui| {
            egui::CentralPanel::default().show(ui, |ui| spotifast::ui::home::show(&mut app, ui));
        },
    );
    assert!(
        !out.platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Hidden example"))
    );
}
#[test]
fn visibility_round_trips_and_is_scoped_to_account() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    spotifast::theme::install(&ctx);
    app.actions.push(Action::SetPlaylistHidden {
        source: "spotify:playlist:secret".into(),
        hidden: true,
    });
    frame(&mut app, &ctx);
    assert!(app.playlist_hidden("spotify:playlist:secret"));
    let saved = app.settings.encode().unwrap();
    app.settings = serde_json::from_str(&saved).unwrap();
    assert!(app.playlist_hidden("spotify:playlist:secret"));
    app.user = Some(User {
        id: "another-account".into(),
        ..Default::default()
    });
    assert!(!app.playlist_hidden("spotify:playlist:secret"));
    app.user = Some(User {
        id: "example".into(),
        ..Default::default()
    });
    app.actions.push(Action::SetPlaylistHidden {
        source: "spotify:playlist:secret".into(),
        hidden: false,
    });
    frame(&mut app, &ctx);
    assert!(!app.playlist_hidden("spotify:playlist:secret"));
}
#[test]
fn playlist_visibility_is_independent_between_platforms() {
    let mut app = fixture();
    app.settings
        .hidden_playlists
        .entry("example".into())
        .or_default()
        .insert("spotify:playlist:secret".into());
    assert!(app.playlist_hidden("spotify:playlist:secret"));
    assert!(!app.playlist_hidden("https://www.youtube.com/playlist?list=PLexample"));
}
#[test]
fn visualizer_button_preserves_window_mode_and_fullscreen_is_separate() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    spotifast::theme::install(&ctx);
    app.actions.push(Action::Addon(addons::Command::Fullscreen));
    let out = frame(&mut app, &ctx);
    assert!(app.addons.fullscreen);
    assert!(
        !out.viewport_output
            .values()
            .flat_map(|v| v.commands.iter())
            .any(|c| matches!(
                c,
                egui::ViewportCommand::Fullscreen(_) | egui::ViewportCommand::Maximized(_)
            ))
    );
    app.actions.push(Action::ToggleFullscreen);
    let out = frame(&mut app, &ctx);
    assert!(
        out.viewport_output
            .values()
            .flat_map(|v| v.commands.iter())
            .any(|c| matches!(c, egui::ViewportCommand::Fullscreen(true)))
    );
    app.actions.push(Action::Addon(addons::Command::Fullscreen));
    frame(&mut app, &ctx);
    assert!(!app.addons.fullscreen);
}
#[test]
fn home_hides_titles_until_show_button_is_pressed() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    spotifast::theme::install(&ctx);
    app.settings
        .hidden_playlists
        .entry("example".into())
        .or_default()
        .insert("spotify:playlist:secret".into());
    let draw = |app: &mut spotifast::app::App, events| {
        ctx.run_ui(
            egui::RawInput {
                events,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200., 2400.),
                )),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| spotifast::ui::home::show(app, ui));
            },
        )
    };
    let out = draw(&mut app, vec![]);
    let tree = out.platform_output.accesskit_update.unwrap();
    assert!(!tree.nodes.iter().any(|(_, n)| matches!(
        n.label(),
        Some("Hidden example" | "Your top songs" | "Last 4 weeks" | "Recommended for you")
    )));
    let bounds = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Show (1)"))
        .unwrap()
        .1
        .bounds()
        .unwrap();
    let pos = egui::pos2(
        ((bounds.x0 + bounds.x1) / 2.) as f32,
        ((bounds.y0 + bounds.y1) / 2.) as f32,
    );
    for pressed in [true, false] {
        draw(
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
    let out = draw(&mut app, vec![]);
    assert!(
        out.platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Hidden example"))
    );
}
#[test]
fn owned_playlist_menu_offers_only_supported_actions() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    spotifast::theme::install(&ctx);
    let mut playlist = Playlist {
        id: "secret".into(),
        uri: "spotify:playlist:secret".into(),
        name: "Example".into(),
        ..Default::default()
    };
    playlist.owner.id = Some("example".into());
    let out = ctx.run_ui(Default::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            spotifast::ui::widgets::context_menu_items(
                ui,
                &mut app,
                &playlist.uri,
                &playlist.name,
                Some(&playlist),
            )
        });
    });
    let tree = out.platform_output.accesskit_update.unwrap();
    for label in [
        "Hide playlist",
        "Edit details",
        "Delete from Spotify",
        "Copy link",
    ] {
        assert!(
            tree.nodes.iter().any(|(_, n)| n.label() == Some(label)),
            "Missing {label}"
        );
    }
    for label in ["Download", "Create folder", "Invite collaborators"] {
        assert!(!tree.nodes.iter().any(|(_, n)| n.label() == Some(label)));
    }
}
