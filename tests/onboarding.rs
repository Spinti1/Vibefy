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
fn home_import_controls_are_visible_without_mandatory_transfer_in_small_window() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("home-import-actions");
    app.backend.set_offline(true);
    app.offline = true;
    app.settings.welcome_completed = true;
    app.auth = AuthStatus::SignedOut;
    let tree = render_sized(&ctx, &mut app, 820.0, 640.0);
    for label in [
        "Create playlist",
        "Import YouTube playlist",
        "Import Spotify playlist",
    ] {
        let bounds = button(&tree, label).bounds().unwrap();
        assert!(
            bounds.y1 <= 640.0 && bounds.x1 <= 820.0,
            "{label} outside viewport: {bounds:?}"
        );
    }
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Start transfer"))
    );
}
#[test]
fn backup_progress_is_inline_on_home_and_does_not_block_import_controls() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("home-backup-progress");
    app.backend.set_offline(true);
    app.offline = true;
    app.settings.welcome_completed = true;
    app.personal.tasks.insert(
        1,
        spotifast::personal_library::Pending {
            source: Some("demo".into()),
            backup: true,
            fingerprint: String::new(),
            epoch: 0,
            message: "Saving playlist to YouTube…".into(),
        },
    );
    let tree = render(&ctx, &mut app);
    button(&tree, "Create playlist");
    button(&tree, "Import Spotify playlist");
    assert!(
        tree.nodes
            .iter()
            .any(|(_, n)| n.value().or(n.label()) == Some("Saving playlist to YouTube…"))
    );
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Change selection"))
    );
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
fn optional_account_buttons_share_size_and_center_and_offer_skip() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("spotify-first");
    app.auth = AuthStatus::SignedOut;
    app.music_accounts[0].connected = false;
    let tree = render(&ctx, &mut app);
    button(&tree, "Skip · Continue as guest");
    let first = button(&tree, "Connect Spotify").bounds().unwrap();
    let second = button(&tree, "Connect YouTube").bounds().unwrap();
    assert_eq!(first.x1 - first.x0, second.x1 - second.x0);
    assert_eq!(first.y1 - first.y0, second.y1 - second.y0);
    assert!(((first.x1 + first.x0) - (second.x1 + second.x0)).abs() < 1.0);
    assert!(first.y1 <= second.y0);
}
#[test]
fn spotify_connection_opens_home_without_requiring_youtube() {
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
    button(&tree, "Create playlist");
    button(&tree, "Import Spotify playlist");
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Skip for now — not recommended"))
    );
}
#[test]
fn connected_accounts_open_home_without_the_old_transfer_wizard() {
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
    app.library.playlists =
        spotifast::model::Loadable::Loaded(vec![spotifast::api::models::Playlist {
            id: "0123456789012345678901".into(),
            name: "Example playlist".into(),
            ..Default::default()
        }]);
    let tree = render(&ctx, &mut app);
    button(&tree, "Create playlist");
    button(&tree, "Import YouTube playlist");
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Start transfer"))
    );
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
