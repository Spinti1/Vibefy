#![cfg(feature = "demo")]
use spotifast::{
    addons::{self, Id},
    app::{App, AppOptions},
    backend::Waker,
    model::{Action, Page},
    music::{Entry, Filter, Kind, Results},
    paths::AppDirs,
    peek,
    player::{LocalTrack, Playback},
};

fn setup() -> (egui::Context, App, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("vibefy-0413-{}", rand::random::<u64>()));
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let w = Waker::default();
    w.attach(&ctx);
    let mut app = App::new(
        &w,
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
fn frame(ctx: &egui::Context, app: &mut App, width: f32, scroll: f32) -> Vec<String> {
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
    out.textures_delta.clear();
    out.platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
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
fn all_previous_mods_track_peek_and_included_lyrics_are_discoverable_at_both_widths() {
    for width in [760.0, 1280.0] {
        let (ctx, mut app, root) = setup();
        app.open(Page::Mods);
        let mut labels = Vec::new();
        for i in 0..16 {
            labels.extend(frame(
                &ctx,
                &mut app,
                width,
                if i == 0 { 0.0 } else { -550.0 },
            ));
        }
        for id in addons::ALL {
            assert!(
                labels.iter().any(|l| l == id.name()),
                "{} missing at {width}",
                id.name()
            );
        }
        for name in ["Spicy Lyrics", "More Lyrics", "Show all"] {
            assert!(
                labels.iter().any(|l| l == name),
                "{name} missing at {width}"
            );
        }
        assert!(app.settings.addons.installed.is_empty());
        finish(app, root);
    }
}
#[test]
fn search_uses_generic_branding_prominent_artist_and_no_video_filter() {
    let (ctx, mut app, root) = setup();
    app.actions.push(Action::Search("Artist".into()));
    frame(&ctx, &mut app, 1280.0, 0.0);
    app.demo_music_response(
        app.search.serial,
        "Artist".into(),
        Filter::All,
        false,
        Ok(Results {
            entries: vec![Entry {
                id: "UCartist".into(),
                kind: Kind::Artist,
                title: "Artist".into(),
                subtitle: "Artist".into(),
                image: None,
                track: None,
                spotify_note: None,
                browse_id: Some("UCartist".into()),
            }],
            continuation: None,
        }),
    );
    let labels = frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(labels.iter().any(|l| l == "Top result"));
    assert!(labels.iter().any(|l| l == "Artists"));
    assert!(
        labels
            .iter()
            .all(|l| !l.contains("YouTube Music") && l != "Videos")
    );
    finish(app, root);
}
#[test]
fn obsolete_renderer_fields_do_not_break_or_reappear_in_saved_preferences() {
    let old = serde_json::json!({"milkdrop_open":true,"milkdrop_fps":144,"milkdrop_size":[640,480],"volume":12345,"visualizer_fps":30,"addons":{"installed":["Oneko","WaveLink"],"enabled":["Oneko"]}});
    let settings: spotifast::settings::Settings = serde_json::from_value(old).unwrap();
    assert_eq!(settings.volume, 12345);
    assert_eq!(settings.visualizer_fps, 30);
    assert!(settings.addons.active(Id::Oneko));
    assert!(!settings.addons.active(Id::TrackPeek));
    let json = serde_json::to_string(&settings).unwrap();
    assert!(!json.contains("milkdrop"));
}
#[test]
fn preview_preserves_queue_and_position_resumes_original_and_cancels_on_manual_play() {
    let (ctx, mut app, root) = setup();
    let main = LocalTrack {
        uri: "vibefy:youtube:abcdefghijk".into(),
        title: "Original".into(),
        ..Default::default()
    };
    let other = LocalTrack {
        uri: "vibefy:youtube:bbbbbbbbbbb".into(),
        title: "Preview".into(),
        ..Default::default()
    };
    app.local.connected = true;
    app.local.track = Some(main.clone());
    app.local.playback = Playback::Playing;
    app.local.position_ms = 12345;
    app.local.position_at = None;
    app.local.queue_tracks = vec![main.clone(), other.clone()];
    app.local.queue_index = 0;
    let sequence = app.local.track_sequence;
    app.actions.push(Action::PreviewTrack(other.clone()));
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(app.peek.is_none(), "uninstalled mod must do no work");
    app.settings.addons.set(Id::TrackPeek, true);
    app.actions.push(Action::PreviewTrack(other.clone()));
    frame(&ctx, &mut app, 1280.0, 0.0);
    let session = std::sync::Arc::clone(&app.peek.as_ref().unwrap().session);
    assert!(!app.believed_playing());
    assert_eq!(app.local.track.as_ref().unwrap().uri, main.uri);
    assert_eq!(app.local.queue_tracks.len(), 2);
    assert_eq!(app.local.position_ms, 12345);
    assert_eq!(app.local.track_sequence, sequence);
    app.local.playback = Playback::Paused;
    app.actions.push(Action::StopPreview);
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(app.peek.is_none());
    assert!(session.cancelled.load(std::sync::atomic::Ordering::Relaxed));
    assert!(app.believed_playing());
    app.local.playback = Playback::Playing;
    app.actions.push(Action::PreviewTrack(other));
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(app.peek.is_some());
    app.actions.push(Action::Next);
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(app.peek.is_none());
    finish(app, root);
}
#[test]
fn preview_completion_and_uninstall_release_session_and_restore_main_pause_state() {
    let (ctx, mut app, root) = setup();
    app.local.connected = true;
    app.local.track = Some(LocalTrack {
        uri: "vibefy:youtube:abcdefghijk".into(),
        title: "Paused".into(),
        ..Default::default()
    });
    app.local.playback = Playback::Paused;
    app.settings.addons.set(Id::TrackPeek, true);
    let track = app.local.track.clone().unwrap();
    app.actions.push(Action::PreviewTrack(track));
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(app.peek.is_some());
    *app.peek.as_ref().unwrap().session.status.lock().unwrap() = peek::Status::Finished;
    app.background_frame(&ctx);
    assert!(app.peek.is_none());
    assert!(!app.believed_playing());
    app.actions
        .push(Action::PreviewTrack(app.local.track.clone().unwrap()));
    frame(&ctx, &mut app, 1280.0, 0.0);
    let session = std::sync::Arc::clone(&app.peek.as_ref().unwrap().session);
    app.actions
        .push(Action::Addon(addons::Command::Uninstall(Id::TrackPeek)));
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(app.peek.is_none());
    assert!(session.cancelled.load(std::sync::atomic::Ordering::Relaxed));
    finish(app, root);
}

#[test]
fn native_browse_load_more_continues_without_replacing_or_reordering_songs() {
    let (ctx, mut app, root) = setup();
    app.open(Page::Search);
    app.actions.push(Action::Search("Artist".into()));
    frame(&ctx, &mut app, 1280.0, 0.0);
    app.actions.push(Action::MusicBrowse("UCartist".into()));
    frame(&ctx, &mut app, 1280.0, 0.0);
    let serial = app.search.serial;
    let row = |id: &str| Entry {
        id: id.into(),
        kind: Kind::Song,
        title: id.into(),
        subtitle: "Artist".into(),
        image: None,
        spotify_note: None,
        browse_id: None,
        track: Some(LocalTrack {
            uri: format!("vibefy:youtube:{id}"),
            ..Default::default()
        }),
    };
    app.demo_music_browse_response(
        serial,
        "UCartist".into(),
        false,
        Ok(Results {
            entries: vec![row("abcdefghijk")],
            continuation: Some("NEXT".into()),
        }),
    );
    app.actions.push(Action::MusicMore);
    frame(&ctx, &mut app, 1280.0, 0.0);
    assert!(
        app.search.music.loading_more,
        "browse continuations must reach the backend"
    );
    app.demo_music_browse_response(
        serial,
        "UCartist".into(),
        true,
        Ok(Results {
            entries: vec![row("bbbbbbbbbbb"), row("abcdefghijk")],
            continuation: None,
        }),
    );
    assert_eq!(
        app.search
            .music
            .results
            .entries
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        vec!["abcdefghijk", "bbbbbbbbbbb", "abcdefghijk"]
    );
    finish(app, root);
}

#[test]
fn spotify_assistance_reuses_exact_cached_recording_art_without_replacing_source() {
    use spotifast::api::models::{Album, ArtistRef, Image, Track};
    let (ctx, mut app, root) = setup();
    app.track_cache.clear();
    app.play_counts
        .insert("spotify:track:cached".into(), 123456);
    let image = "https://i.scdn.co/image/cached-art";
    let artist = ArtistRef {
        name: "Deftones".into(),
        ..Default::default()
    };
    app.track_cache.insert(
        "cached".into(),
        Track {
            uri: "spotify:track:cached".into(),
            name: "Digital Bath".into(),
            artists: vec![artist.clone()],
            duration_ms: 256000,
            album: Some(Album {
                name: "White Pony".into(),
                images: vec![Image {
                    url: image.into(),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    let local = LocalTrack {
        uri: "vibefy:youtube:iEtJQmv7-LY".into(),
        title: "Digital Bath".into(),
        artists: vec![artist],
        album: "White Pony".into(),
        duration_ms: 256000,
        ..Default::default()
    };
    app.actions.push(Action::Search("Digital Bath".into()));
    frame(&ctx, &mut app, 1280.0, 0.0);
    app.demo_music_response(
        app.search.serial,
        "Digital Bath".into(),
        Filter::All,
        false,
        Ok(Results {
            entries: vec![Entry {
                id: "iEtJQmv7-LY".into(),
                kind: Kind::Song,
                title: local.title.clone(),
                subtitle: String::new(),
                image: None,
                spotify_note: None,
                browse_id: None,
                track: Some(local.clone()),
            }],
            continuation: None,
        }),
    );
    let entry = &app.search.music.results.entries[0];
    assert_eq!(entry.image.as_deref(), Some(image));
    assert_eq!(
        entry.spotify_note.as_deref(),
        Some("123456 Spotify plays (cached)")
    );
    assert_eq!(entry.track.as_ref().unwrap().uri, local.uri);
    assert_eq!(entry.track.as_ref().unwrap().duration_ms, local.duration_ms);
    assert!(!app.search.catalogue_pending);
    finish(app, root);
}
