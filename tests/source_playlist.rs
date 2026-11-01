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
fn manual_update_check_opens_the_installer_flow_and_background_check_does_not() {
    for manual in [false, true] {
        let (ctx, mut app) = fixture();
        app.update_source = spotifast::updates::Source::local("http://127.0.0.1:9").unwrap();
        app.demo_backend_event(spotifast::backend::Event::UpdateChecked {
            manual,
            result: Ok(Some(spotifast::updates::Release {
                version: "9.9.9".into(),
                url: "https://github.com/Spinti1/Vibefy/releases/tag/v9.9.9".into(),
            })),
        });
        frame(&ctx, &mut app);
        assert_eq!(app.show_update, manual);
        assert!(app.update.is_some());
        assert!(!app.update_checking);
    }
}

#[test]
fn update_check_reports_current_version_and_errors_without_opening_installation() {
    let (_, mut app) = fixture();
    app.update_source = spotifast::updates::Source::local("http://127.0.0.1:9").unwrap();
    app.update_checking = true;
    app.demo_backend_event(spotifast::backend::Event::UpdateChecked {
        manual: true,
        result: Ok(None),
    });
    assert!(!app.update_checking);
    assert!(!app.show_update);
    assert_eq!(app.toasts.last().unwrap().message, "Vibefy is up to date");
    app.demo_backend_event(spotifast::backend::Event::UpdateChecked {
        manual: true,
        result: Err("GitHub unavailable".into()),
    });
    assert!(
        app.toasts
            .last()
            .unwrap()
            .message
            .contains("Couldn't check for updates")
    );
    assert!(!app.show_update);
}

#[test]
fn a_tiny_main_viewport_cannot_replace_the_saved_normal_window_size() {
    let (ctx, mut app) = fixture();
    app.window_geometry.size = Some([940.0, 680.0]);
    let mut input = egui::RawInput::default();
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .inner_rect = Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(275.0, 116.0),
    ));
    let _ = ctx.run_ui(input, |_| app.background_frame(&ctx));
    assert_eq!(app.window_geometry.size, Some([940.0, 680.0]));
}

#[test]
fn spotify_choices_keep_separate_rows_and_cover_clicks_select_the_right_recording() {
    let (ctx, mut app) = fixture();
    let choices: Vec<_> = ['D', 'E', 'F']
        .into_iter()
        .enumerate()
        .map(|(index, suffix)| {
            let mut track = result(&format!("spotify:track:1234567890abcdefghABC{suffix}"));
            track.name = format!("Choice {index}");
            track
        })
        .collect();
    app.dialog = Some(Dialog::SpotifyTrackMatch {
        source: source(),
        query: "Example song".into(),
        serial: 99,
        results: Loadable::Loaded(choices.clone()),
        selected: None,
    });
    let draw = |app: &mut App, events| {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ui| app.frame_ui(ui),
        )
    };
    for _ in 0..3 {
        draw(&mut app, vec![]);
    }
    let output = draw(&mut app, vec![]);
    let mut positions = Vec::new();
    for choice in &choices {
        positions.push(
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.job.text == choice.name => {
                        Some(text.pos)
                    }
                    _ => None,
                })
                .expect("the recording title must be drawn"),
        );
    }
    assert!(
        positions
            .windows(2)
            .all(|pair| pair[1].y - pair[0].y >= 62.0),
        "rows and hit areas must not overlap: {positions:?}"
    );
    let cover = positions[1] + egui::vec2(-30.0, 12.0);
    for pressed in [true, false] {
        draw(
            &mut app,
            vec![
                egui::Event::PointerMoved(cover),
                egui::Event::PointerButton {
                    pos: cover,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(
        matches!(&app.dialog, Some(Dialog::SpotifyTrackMatch { selected: Some(track), .. }) if track.uri == choices[1].uri)
    );
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
