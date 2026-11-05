use spotifast::{
    addons::Collection,
    app::{App, AppOptions},
    backend::{AuthStatus, Waker},
    model::{Action, Loadable, Page},
    music::{Entry, Kind, Profile, Results},
    paths::AppDirs,
    personal_library::Dialog,
    player::LocalTrack,
    theme,
};

fn app() -> App {
    let root =
        std::env::temp_dir().join(format!("vibefy-library-qa-{:032x}", rand::random::<u128>()));
    let mut app = App::new(
        &Waker::default(),
        AppDirs {
            config: root.join("config"),
            state: root.join("state"),
            cache: root.join("cache"),
        },
        Default::default(),
        AppOptions {
            restore_sign_in: false,
            media_controls: false,
            tray: false,
            autostart: false,
        },
    );
    app.auth = AuthStatus::SignedOut;
    app.settings.welcome_completed = true;
    app.offline = true;
    app.backend.set_offline(true);
    app.settings.playlist_snapshot_recovery_v1 = true;
    app
}
fn frame(
    ctx: &egui::Context,
    app: &mut App,
    width: f32,
    events: Vec<egui::Event>,
    apply: bool,
) -> egui::accesskit::TreeUpdate {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            if apply {
                app.frame_ui(ui)
            } else {
                spotifast::ui::show(app, ui)
            }
        },
    );
    output.textures_delta.clear();
    output.platform_output.accesskit_update.unwrap()
}
fn context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    ctx
}
fn collection() -> Collection {
    Collection {
        title: "Night drive".into(),
        source: spotifast::personal_library::new_source(),
        tracks: vec![LocalTrack {
            uri: "vibefy:youtube:abcdefghijk".into(),
            title: "Example song".into(),
            name: "Example song".into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn guest_library_has_real_playlists_and_supported_filters_without_spotify_loading() {
    let ctx = context();
    let mut app = app();
    app.addons.collections.push(collection());
    app.music_accounts[0].connected = true;
    app.music_accounts[0].playlists = vec![spotifast::music_accounts::LibraryPlaylist {
        id: "VLSE".into(),
        title: "Episodes for later".into(),
        image: None,
    }];
    app.library.playlists = Loadable::NotLoaded;
    frame(&ctx, &mut app, 1280.0, vec![], false);
    let tree = frame(&ctx, &mut app, 1280.0, vec![], false);
    let labels: Vec<_> = tree.nodes.iter().filter_map(|(_, n)| n.label()).collect();
    assert!(labels.contains(&"Open playlist Night drive"));
    assert!(labels.contains(&"This device"));
    for old in [
        "Podcasts",
        "Albums",
        "Artists",
        "By You",
        "Loading…",
        "Episodes for later",
    ] {
        assert!(!labels.contains(&old), "obsolete library item {old}");
    }
    app.backend.shutdown();
}
#[test]
fn typing_waits_for_enter_and_only_the_submitted_query_enters_history() {
    let ctx = context();
    let mut app = app();
    app.open(Page::Search);
    frame(&ctx, &mut app, 1280.0, vec![], true);
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("global-search")));
    let serial = app.search.serial;
    frame(
        &ctx,
        &mut app,
        1280.0,
        vec![egui::Event::Text("rock".into())],
        true,
    );
    assert_eq!(app.search.query, "rock");
    assert_eq!(app.search.serial, serial);
    assert!(app.settings.search_history.is_empty());
    frame(
        &ctx,
        &mut app,
        1280.0,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
        true,
    );
    assert_eq!(app.search.committed, "rock");
    assert_eq!(app.settings.search_history, ["rock"]);
    app.search.query = "rock ballads".into();
    frame(&ctx, &mut app, 1280.0, vec![], true);
    assert_eq!(app.settings.search_history, ["rock"]);
    app.backend.shutdown();
}
#[test]
fn guest_can_open_an_artist_and_go_back_without_spotify_connection() {
    let mut app = app();
    let ctx = context();
    app.open(Page::Search);
    let album = Entry {
        id: "MPREexample".into(),
        kind: Kind::Album,
        title: "Example album".into(),
        subtitle: String::new(),
        image: None,
        track: None,
        browse_id: Some("MPREexample".into()),
        spotify_note: None,
    };
    app.search.music.pages.insert(
        "UCexample".into(),
        (
            "Example artist".into(),
            Results {
                profile: Some(Profile {
                    title: "Example artist".into(),
                    ..Default::default()
                }),
                entries: vec![album],
                continuation: None,
            },
        ),
    );
    app.actions.push(Action::MusicBrowse("UCexample".into()));
    app.background_frame(&ctx);
    assert_eq!(app.page(), &Page::MusicBrowse("UCexample".into()));
    assert_eq!(app.search.music.page_title, "Example artist");
    assert!(!app.search.music.loading);
    assert_eq!(app.search.music.results.entries[0].title, "Example album");
    app.actions.push(Action::Back);
    app.background_frame(&ctx);
    assert_eq!(app.page(), &Page::Search);
    app.backend.shutdown();
}
#[test]
fn playlist_dialogs_fit_both_themes_and_window_sizes() {
    for width in [700.0, 1280.0] {
        for dark in [true, false] {
            for import in [false, true] {
                let ctx = context();
                let mut app = app();
                app.palette = if dark {
                    theme::Palette::dark()
                } else {
                    theme::Palette::light()
                };
                ctx.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                app.personal.dialog = Some(if import {
                    Dialog::Import {
                        youtube: false,
                        url: String::new(),
                        backup: false,
                    }
                } else {
                    Dialog::New {
                        name: String::new(),
                        backup: false,
                    }
                });
                // Let the modal's sizing and scroll-area layout settle first.
                for _ in 0..6 {
                    frame(&ctx, &mut app, width, vec![], false);
                }
                let tree = frame(&ctx, &mut app, width, vec![], false);
                let primary = if import {
                    "Import playlist"
                } else {
                    "Create playlist"
                };
                let cancel = tree
                    .nodes
                    .iter()
                    .find(|(_, n)| {
                        n.role() == egui::accesskit::Role::Button && n.label() == Some("Cancel")
                    })
                    .expect("dialog cancel")
                    .1
                    .bounds()
                    .unwrap();
                let button = tree
                    .nodes
                    .iter()
                    .rev()
                    .find(|(_, n)| {
                        n.role() == egui::accesskit::Role::Button
                            && n.label() == Some(primary)
                            && n.bounds().is_some_and(|r| {
                                ((r.y0 + r.y1) - (cancel.y0 + cancel.y1)).abs() < 2.0
                            })
                    })
                    .expect("primary dialog action");
                assert!(button.1.is_disabled());
                let r = button.1.bounds().unwrap();
                assert!(
                    r.x0 >= 0.0 && r.x1 <= width as f64 && r.y0 >= 0.0 && r.y1 <= 900.0,
                    "dialog action outside viewport"
                );
                assert!(tree.nodes.iter().any(|(_, n)| n.label() == Some("Cancel")));
                if import {
                    assert!(
                        tree.nodes.iter().any(|(_, n)| n.label().or(n.value())
                            == Some("Public Spotify playlists only"))
                    );
                }
                app.backend.shutdown();
            }
        }
    }
}
#[test]
fn artist_metadata_keeps_header_art_and_real_artist_ids_without_related_artists() {
    use serde_json::json;
    let song = json!({"musicResponsiveListItemRenderer":{"playlistItemData":{"videoId":"abcdefghijk"},"musicVideoType":"MUSIC_VIDEO_TYPE_ATV","flexColumns":[{"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":"Song"}]}}},{"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":"Example artist","navigationEndpoint":{"browseEndpoint":{"browseId":"UCexample"}}}]}}}]}});
    let parsed=spotifast::music::parse_artist(&json!({"header":{"musicImmersiveHeaderRenderer":{"title":{"runs":[{"text":"Example artist"}]},"subtitle":{"runs":[{"text":"Subscribers"}]},"thumbnail":{"thumbnails":[{"url":"https://yt3.googleusercontent.com/example=w600-h600","width":600,"height":600}]}}},"contents":[song,{"musicCarouselShelfRenderer":{"header":{"musicCarouselShelfBasicHeaderRenderer":{"title":{"runs":[{"text":"Fans might also like"}]}}},"contents":[{"musicTwoRowItemRenderer":{"title":{"runs":[{"text":"Other artist"}]},"navigationEndpoint":{"browseEndpoint":{"browseId":"UCother"}}}}]}}]})).unwrap();
    let profile = parsed.profile.unwrap();
    assert_eq!(profile.title, "Example artist");
    assert!(profile.image.is_some());
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(
        parsed.entries[0].track.as_ref().unwrap().artists[0]
            .id
            .as_deref(),
        Some("UCexample")
    );
}
#[test]
fn guest_backup_roundtrip_contains_local_playlist_and_restore_never_overwrites_it() {
    let p = collection();
    let bytes = spotifast::playlist_backup::encode(&[p.clone()]).unwrap();
    let restored = spotifast::playlist_backup::decode(&bytes).unwrap();
    assert_eq!(restored[0].source, p.source);
    assert_eq!(restored[0].tracks, p.tracks);
    let mut library = vec![p.clone()];
    library[0].tracks.clear();
    assert_eq!(
        spotifast::playlist_backup::merge(&mut library, restored.clone()).unwrap(),
        1
    );
    assert!(library[0].tracks.is_empty());
    assert_eq!(
        spotifast::playlist_backup::merge(&mut library, restored).unwrap(),
        0
    );
    assert!(bytes.len() < 1000);
}
#[test]
fn podcast_queue_is_excluded_from_both_account_ids_and_older_saved_metadata() {
    assert!(spotifast::music_accounts::excluded_youtube_playlist("VLSE"));
    assert!(spotifast::music_accounts::excluded_playlist_source(
        "https://www.youtube.com/playlist?list=SE"
    ));
    assert!(!spotifast::music_accounts::excluded_playlist_source(
        "https://www.youtube.com/playlist?list=LM"
    ));
    assert!(!spotifast::music_accounts::excluded_playlist_source(
        "https://youtube.com.attacker.test/playlist?list=SE"
    ));
}
