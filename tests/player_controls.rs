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

pub use spotifast::{addons, app, bidi, model, peek, player, theme as app_theme};
mod widgets {
    pub use spotifast::ui::widgets::*;
}
#[path = "../src/ui/track_peek.rs"]
mod track_peek;
mod mod_host {
    pub fn render(
        _: &spotifast::app::App,
        _: &egui::Ui,
        _: spotifast::addons::Id,
        _: egui::Rect,
        _: &[(usize, f32)],
    ) {
    }
}
fn song(id: &str, title: &str) -> player::LocalTrack {
    player::LocalTrack {
        uri: format!("vibefy:youtube:{id}"),
        title: title.into(),
        artists: vec![spotifast::api::models::ArtistRef {
            name: "Example artist".into(),
            ..Default::default()
        }],
        duration_ms: 200_000,
        ..Default::default()
    }
}
fn player_fixture() -> App {
    let mut app = fixture("peek-controls");
    app.offline = true;
    app.auth = AuthStatus::Connected {
        username: "example".into(),
    };
    app.user = Some(spotifast::api::models::User {
        id: "example".into(),
        ..Default::default()
    });
    app.music_accounts[0].connected = true;
    app.local.connected = true;
    app.local.playback = player::Playback::Playing;
    app.local.queue_tracks = vec![
        song("aaaaaaaaaaa", "Angel's Share"),
        song("bbbbbbbbbbb", "Current song"),
        song("ccccccccccc", "Catherine"),
    ];
    app.local.queue_index = 1;
    app.local.track = Some(app.local.queue_tracks[1].clone());
    app.local.previous_track = Some(app.local.queue_tracks[0].clone());
    app.settings.addons.set(addons::Id::TrackPeek, true);
    app.settings.addons.installed.insert(addons::Id::TrackPeek);
    app.settings.addons.installed.insert(addons::Id::Vivy);
    app
}
#[test]
fn track_peek_shows_previous_and_next_and_wraps_at_queue_end() {
    let mut app = player_fixture();
    assert_eq!(
        track_peek::neighbour(&app, false).unwrap().title,
        "Angel's Share"
    );
    assert_eq!(
        track_peek::neighbour(&app, true).unwrap().title,
        "Catherine"
    );
    app.local.queue_index = 2;
    assert!(track_peek::neighbour(&app, true).is_none());
    app.local.repeat = player::RepeatMode::Context;
    assert_eq!(
        track_peek::neighbour(&app, true).unwrap().title,
        "Angel's Share"
    );
}
#[test]
fn track_peek_uses_reserved_shuffle_and_manual_queue_priority() {
    let mut app = player_fixture();
    app.local.shuffle = true;
    app.local.plan_shuffle_next();
    let reserved = app.local.shuffle_next.as_ref().unwrap();
    assert_ne!(reserved.uri, app.local.track.as_ref().unwrap().uri);
    let uri = reserved.uri.clone();
    assert_eq!(track_peek::neighbour(&app, true).unwrap().uri, uri);
    assert_eq!(track_peek::neighbour(&app, true).unwrap().uri, uri);
    app.manual_queue.push(app.local.queue_tracks[2].uri.clone());
    assert_eq!(
        track_peek::neighbour(&app, true).unwrap().title,
        "Catherine"
    );
}
#[test]
fn old_proxy_settings_are_discarded_without_losing_other_preferences() {
    for mode in ["system", "http", "socks"] {
        let settings: Settings=serde_json::from_value(serde_json::json!({"proxy_mode":mode,"proxy_host":"127.0.0.1","proxy_port":"9","proxy_password":"obsolete","volume":42})).unwrap();
        assert_eq!(
            settings.proxy_config().unwrap(),
            spotifast::settings::ProxyConfig::Off
        );
        assert!(settings.proxy_host.is_empty() && settings.proxy_password.is_empty());
        assert_eq!(settings.volume, 42);
        assert!(!settings.encode().unwrap().contains("proxy_mode"));
        assert_eq!(
            player::youtube_proxy_args(&settings.proxy_config().unwrap()),
            vec!["--proxy", ""]
        );
    }
}
#[test]
fn direct_http_ignores_environment_proxy() {
    use std::io::{Read, Write};
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = server.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = server.accept().unwrap();
        let mut buf = [0; 4096];
        stream.read(&mut buf).unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK")
            .unwrap();
    });
    let http = spotifast::http::build_blocking(
        &spotifast::settings::ProxyConfig::Off,
        std::time::Duration::from_secs(3),
    )
    .unwrap();
    assert_eq!(
        http.get(format!("http://{address}/"))
            .send()
            .unwrap()
            .text()
            .unwrap(),
        "OK"
    );
    worker.join().unwrap();
}
#[test]
fn distinct_lyrics_and_visualizer_controls_are_present() {
    let mut app = player_fixture();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280., 800.),
            )),
            ..Default::default()
        },
        |ui| spotifast::ui::player_bar::show(&mut app, ui),
    );
    let tree = output.platform_output.accesskit_update.unwrap();
    for name in ["Lyrics", "Visualizers", "Fullscreen"] {
        assert!(
            tree.nodes.iter().any(|(_, n)| n.label() == Some(name)),
            "Missing {name}"
        );
    }
}

#[test]
fn mod_settings_open_separately_and_juicevault_has_no_browser_login() {
    let mut app = player_fixture();
    app.settings.addons.set(addons::Id::JuiceVault, true);
    app.addons.filter = "JuiceVault".into();
    app.actions.push(model::Action::Open(model::Page::Mods));
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    for expanded in [false, true] {
        app.addons.settings_open = expanded.then_some(addons::Id::JuiceVault);
        let mut labels = Vec::new();
        for _ in 0..3 {
            let out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1100., 1100.),
                    )),
                    ..Default::default()
                },
                |ui| app.frame_ui(ui),
            );
            labels = out
                .platform_output
                .accesskit_update
                .unwrap()
                .nodes
                .into_iter()
                .filter_map(|(_, node)| node.label().or(node.value()).map(str::to_owned))
                .collect();
        }
        assert_eq!(labels.iter().any(|s| s == "JuiceVault account"), expanded);
        assert!(!labels.iter().any(|s| s == "Sign in through browser"));
    }
    app.backend.shutdown();
}
#[test]
fn reload_later_keeps_playback_and_reload_now_requests_a_normal_quit() {
    let mut app = player_fixture();
    let ctx = egui::Context::default();
    theme::install(&ctx);
    app.addons.reload_notice = Some(addons::Id::Oneko);
    app.actions
        .push(model::Action::Addon(addons::Command::ReloadLater));
    let frame = |app: &mut App| {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| app.frame_ui(ui));
    };
    frame(&mut app);
    assert!(app.addons.reload_notice.is_none());
    assert!(!app.quit_requested);
    app.actions
        .push(model::Action::Addon(addons::Command::ReloadNow));
    frame(&mut app);
    assert!(app.quit_requested);
    assert!(
        app.addons
            .reload_requested
            .load(std::sync::atomic::Ordering::Acquire)
    );
    app.backend.shutdown();
}
