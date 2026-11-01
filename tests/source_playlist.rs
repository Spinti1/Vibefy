#![cfg(feature = "demo")]
use spotifast::{
    api::models::{Page, PlayableItem, SearchResults, Track},
    app::{App, AppOptions},
    backend::{ApiResponse, Waker},
    model::{Action, Dialog, Loadable},
    paths::AppDirs,
};

fn fixture() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let waker = Waker::default();
    waker.attach(&ctx);
    let root = std::env::temp_dir().join(format!("vibefy-source-match-{}", rand::random::<u64>()));
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
    (ctx, app)
}
fn frame(ctx: &egui::Context, app: &mut App) {
    let _ = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200., 900.),
            )),
            ..Default::default()
        },
        |ui| app.frame_ui(ui),
    );
}
fn source() -> Track {
    Track {
        uri: "vibefy:youtube:abcdefghijk".into(),
        name: "Example song".into(),
        duration_ms: 180000,
        ..Default::default()
    }
}
fn result(uri: &str) -> Track {
    Track {
        uri: uri.into(),
        name: "Example song".into(),
        duration_ms: 180000,
        ..Default::default()
    }
}

#[test]
fn stale_results_and_unlisted_selections_cannot_replace_the_confirmed_recording() {
    let (ctx, mut app) = fixture();
    app.actions.push(Action::FindSpotifyTrack {
        source: source(),
        query: "Example song".into(),
    });
    frame(&ctx, &mut app);
    let serial = match app.dialog.as_ref().unwrap() {
        Dialog::SpotifyTrackMatch { serial, .. } => *serial,
        _ => panic!(),
    };
    app.demo_api_response(ApiResponse::SpotifyTrackMatch {
        serial: serial.wrapping_sub(1),
        result: Ok(SearchResults::default()),
    });
    assert!(matches!(
        app.dialog,
        Some(Dialog::SpotifyTrackMatch {
            results: Loadable::Loading,
            ..
        })
    ));
    let candidate = result("spotify:track:1234567890abcdefghABCD");
    app.demo_api_response(ApiResponse::SpotifyTrackMatch {
        serial,
        result: Ok(SearchResults {
            tracks: Some(Page {
                items: vec![source(), candidate.clone()],
                ..Default::default()
            }),
            ..Default::default()
        }),
    });
    assert!(
        matches!(&app.dialog,Some(Dialog::SpotifyTrackMatch{results:Loadable::Loaded(items),..}) if items.len()==1)
    );
    app.actions.push(Action::SelectSpotifyTrack {
        serial,
        track: result("spotify:track:1234567890abcdefghABCE"),
    });
    frame(&ctx, &mut app);
    assert!(matches!(
        app.dialog,
        Some(Dialog::SpotifyTrackMatch { selected: None, .. })
    ));
    app.actions.push(Action::SelectSpotifyTrack {
        serial,
        track: candidate.clone(),
    });
    frame(&ctx, &mut app);
    assert!(
        matches!(&app.dialog,Some(Dialog::SpotifyTrackMatch{selected:Some(t),..}) if t.uri==candidate.uri)
    );
    app.actions.push(Action::CloseDialog);
    frame(&ctx, &mut app);
    app.demo_api_response(ApiResponse::SpotifyTrackMatch {
        serial,
        result: Ok(SearchResults::default()),
    });
    assert!(app.dialog.is_none());
    app.backend.shutdown();
}

#[test]
fn adding_a_source_uri_opens_matching_instead_of_an_optimistic_spotify_write() {
    let (ctx, mut app) = fixture();
    app.actions.push(Action::AddToPlaylist {
        playlist_id: "pl2".into(),
        playlist_name: "Example".into(),
        items: vec![PlayableItem::Track(source())],
    });
    frame(&ctx, &mut app);
    frame(&ctx, &mut app);
    assert!(matches!(
        app.dialog,
        Some(Dialog::SpotifyTrackMatch { selected: None, .. })
    ));
    assert!(!app.playlist_busy);
    app.backend.shutdown();
}
