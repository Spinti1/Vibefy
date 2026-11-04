use spotifast::{
    app::{App, AppOptions},
    backend::{AuthStatus, Waker},
    paths::AppDirs,
    settings::Settings,
    theme,
};
fn fixture(suffix: &str) -> App {
    let root =
        std::env::temp_dir().join(format!("vibefy-onboarding-{}-{suffix}", std::process::id()));
    App::new(
        &Waker::default(),
        AppDirs {
            config: root.join("config"),
            state: root.join("state"),
            cache: root.join("cache"),
        },
        Settings::default(),
        AppOptions {
            restore_sign_in: false,
            media_controls: false,
            tray: false,
            autostart: false,
        },
    )
}
fn render(ctx: &egui::Context, app: &mut App) -> egui::accesskit::TreeUpdate {
    render_sized(ctx, app, 1000.0, 1000.0)
}
fn render_sized(
    ctx: &egui::Context,
    app: &mut App,
    width: f32,
    height: f32,
) -> egui::accesskit::TreeUpdate {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            ..Default::default()
        },
        |ui| spotifast::ui::show(app, ui),
    );
    output.textures_delta.clear();
    output.platform_output.accesskit_update.unwrap()
}

#[test]
fn playlist_gallery_has_direct_selection_and_visible_actions_in_small_window() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("gallery-small");
    app.auth = AuthStatus::Connected {
        username: "example".into(),
    };
    app.user = Some(spotifast::api::models::User {
        id: "example".into(),
        ..Default::default()
    });
    app.music_accounts[0].connected = true;
    app.mirror.ready = true;
    app.mirror.pair = "gallery-pair".into();
    app.library.playlists = spotifast::model::Loadable::Loaded(
        (0..18)
            .map(|i| spotifast::api::models::Playlist {
                id: format!("{i:022}"),
                name: format!("Playlist {i}"),
                ..Default::default()
            })
            .collect(),
    );
    let tree = render_sized(&ctx, &mut app, 820.0, 640.0);
    for label in ["Start transfer", "Skip for now — not recommended"] {
        let bounds = button(&tree, label).bounds().unwrap();
        assert!(
            bounds.y1 <= 640.0 && bounds.x1 <= 820.0,
            "Action outside viewport: {label} {bounds:?}"
        );
    }
    let card = &tree
        .nodes
        .iter()
        .find(|(_, n)| {
            n.label() == Some("Playlist 0") && n.role() == egui::accesskit::Role::CheckBox
        })
        .expect("direct selectable card")
        .1;
    let bounds = card.bounds().unwrap();
    assert!(bounds.x1 - bounds.x0 > 140.0 && bounds.y1 - bounds.y0 > 180.0);
    assert!(
        bounds.y1 < button(&tree, "Start transfer").bounds().unwrap().y0,
        "First card must be fully visible above the footer"
    );
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Choose playlists to copy"))
    );
}

#[test]
fn active_transfer_has_a_dedicated_progress_screen_and_edit_action() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("transfer-screen");
    app.auth = AuthStatus::Connected {
        username: "example".into(),
    };
    app.user = Some(spotifast::api::models::User {
        id: "example".into(),
        ..Default::default()
    });
    app.music_accounts[0].connected = true;
    app.mirror.ready = true;
    app.mirror.pair = "transfer-pair".into();
    app.mirror.enabled = true;
    app.mirror.busy = true;
    app.mirror.selected = Some(vec!["0123456789012345678901".into()]);
    let tree = render_sized(&ctx, &mut app, 820.0, 640.0);
    button(&tree, "Change selection");
    let skip = button(&tree, "Skip for now — not recommended")
        .bounds()
        .unwrap();
    assert!(skip.y1 <= 640.0);
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Start transfer"))
    );
    // The decorative title uses the theme painter; interactive actions above are accessible.
}
fn button<'a>(tree: &'a egui::accesskit::TreeUpdate, label: &str) -> &'a egui::accesskit::Node {
    &tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some(label) && n.role() == egui::accesskit::Role::Button)
        .unwrap_or_else(|| panic!("Missing button: {label}"))
        .1
}
#[test]
fn youtube_alone_cannot_skip_spotify_and_account_buttons_share_size_and_center() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("spotify-first");
    app.auth = AuthStatus::SignedOut;
    app.music_accounts[0].connected = true;
    let tree = render(&ctx, &mut app);
    let first = button(&tree, "Step 1 · Sign in with Spotify")
        .bounds()
        .unwrap();
    let second = button(&tree, "Step 2 · Sign in with YouTube")
        .bounds()
        .unwrap();
    assert_eq!(first.x1 - first.x0, second.x1 - second.x0);
    assert_eq!(first.y1 - first.y0, second.y1 - second.y0);
    assert!(((first.x1 + first.x0) - (second.x1 + second.x0)).abs() < 1.0);
    assert!(first.y1 <= second.y0);
}
#[test]
fn spotify_connection_shows_required_youtube_step_before_the_main_player() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("youtube-next");
    app.auth = AuthStatus::Connected {
        username: "example".into(),
    };
    app.user = Some(spotifast::api::models::User {
        id: "example".into(),
        ..Default::default()
    });
    let tree = render(&ctx, &mut app);
    button(&tree, "Step 1 · Spotify connected");
    button(&tree, "Step 2 · Sign in with YouTube");
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Skip for now — not recommended"))
    );
}
#[test]
fn connected_accounts_show_selection_and_an_explicit_skip_before_home() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("selection");
    app.auth = AuthStatus::Connected {
        username: "example".into(),
    };
    app.user = Some(spotifast::api::models::User {
        id: "example".into(),
        ..Default::default()
    });
    app.music_accounts[0].connected = true;
    app.mirror.ready = true;
    app.mirror.pair = "example-pair".into();
    app.library.playlists =
        spotifast::model::Loadable::Loaded(vec![spotifast::api::models::Playlist {
            id: "0123456789012345678901".into(),
            name: "Example playlist".into(),
            ..Default::default()
        }]);
    let tree = render(&ctx, &mut app);
    button(&tree, "Start transfer");
    button(&tree, "Skip for now — not recommended");
    assert!(
        tree.nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Example playlist"))
    );
}

#[test]
fn saved_youtube_collection_and_connected_library_have_one_home_tile() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("home-dedup");
    app.network_reachable = true;
    app.music_accounts[0].connected = true;
    app.music_accounts[0].playlists = vec![spotifast::music_accounts::LibraryPlaylist {
        id: "VLPLexample".into(),
        title: "Native YouTube list".into(),
        image: None,
    }];
    app.addons.collections = vec![spotifast::addons::Collection {
        title: "Native YouTube list".into(),
        source: "https://www.youtube.com/playlist?list=PLexample".into(),
        tracks: vec![],
        image: None,
        owner: Some("YouTube Music".into()),
    }];
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 1000.0),
            )),
            ..Default::default()
        },
        |ui| spotifast::ui::home::show(&mut app, ui),
    );
    output.textures_delta.clear();
    let tree = output.platform_output.accesskit_update.unwrap();
    assert_eq!(
        tree.nodes
            .iter()
            .filter(|(_, n)| n.label() == Some("Native YouTube list")
                && n.role() == egui::accesskit::Role::Button)
            .count(),
        1
    );
}
