use spotifast::{
    app::{App, AppOptions},
    backend::{AuthStatus, Command, Event, LocalPlayback, Waker},
    paths::AppDirs,
    player::PlayerCommand,
    settings::Settings,
};
#[test]
fn youtube_engine_ready_while_spotify_is_still_verifying() {
    let root = std::env::temp_dir().join(format!("vibefy-engine-race-{}", std::process::id()));
    let dirs = AppDirs {
        config: root.join("config"),
        state: root.join("state"),
        cache: root.join("cache"),
    };
    let mut app = App::new(
        &Waker::default(),
        dirs,
        Settings::default(),
        AppOptions {
            restore_sign_in: false,
            media_controls: false,
            tray: false,
            autostart: false,
        },
    );
    app.backend.send(Command::MusicAccountReady {
        provider: spotifast::music_accounts::Provider::YouTubeMusic,
        connected: true,
        serial: 0,
    });
    let start = std::time::Instant::now();
    let mut ready = false;
    while start.elapsed() < std::time::Duration::from_secs(8) {
        for event in app.backend.poll() {
            match event {
                Event::Playback(LocalPlayback::Ready { .. }) => ready = true,
                Event::Auth(AuthStatus::Connected { .. }) => panic!("Test must not verify Spotify"),
                Event::Playback(LocalPlayback::Failed(error)) => panic!("Engine failed: {error}"),
                _ => {}
            }
        }
        if ready {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        ready,
        "A restored YouTube session must reach Ready before Spotify verifies"
    );
    app.backend.player(PlayerCommand::Volume(12345));
    let responding = std::time::Instant::now();
    let mut acknowledged = false;
    while responding.elapsed() < std::time::Duration::from_secs(2) {
        acknowledged |= app
            .backend
            .poll()
            .iter()
            .any(|e| matches!(e,Event::Local(state) if state.connected && state.volume==12345));
        if acknowledged {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(acknowledged, "The ready engine must accept player commands");
    app.backend.shutdown();
}
