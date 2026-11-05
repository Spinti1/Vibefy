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

use spotifast::{
    addons::{Command as ModCommand, Id},
    model::{Action, Page},
    music::{self, Entry, Kind, Results},
};
fn connected() -> App {
    let mut app = fixture("navigation-0436");
    app.auth = AuthStatus::Connected {
        username: "example".into(),
    };
    app.user = Some(spotifast::api::models::User {
        id: "example".into(),
        ..Default::default()
    });
    app.music_accounts[0].connected = true;
    app.offline = true;
    app
}
fn entry(id: &str, title: &str, kind: Kind) -> Entry {
    Entry {
        id: id.into(),
        title: title.into(),
        kind,
        subtitle: String::new(),
        image: None,
        browse_id: Some(id.into()),
        track: None,
        spotify_note: None,
    }
}
fn act(app: &mut App, ctx: &egui::Context, action: Action) {
    app.actions.push(action);
    app.background_frame(ctx);
}
#[test]
fn music_artist_album_back_forward_preserves_real_pages() {
    let ctx = egui::Context::default();
    let mut app = connected();
    app.open(Page::Search);
    app.search.committed = "Example artist".into();
    let album = entry("MPREexample", "Example album", Kind::Album);
    app.search.music.results = Results {
        profile: None,
        entries: vec![entry("UCexample", "Example artist", Kind::Artist)],
        continuation: None,
    };
    app.search.music.pages.insert(
        "UCexample".into(),
        (
            "Example artist".into(),
            Results {
                profile: None,
                entries: vec![album.clone()],
                continuation: None,
            },
        ),
    );
    app.search.music.pages.insert(
        "MPREexample".into(),
        (
            "Example album".into(),
            Results {
                profile: None,
                entries: vec![album],
                continuation: None,
            },
        ),
    );
    act(&mut app, &ctx, Action::MusicBrowse("UCexample".into()));
    assert_eq!(app.page(), &Page::MusicBrowse("UCexample".into()));
    act(&mut app, &ctx, Action::MusicBrowse("MPREexample".into()));
    assert_eq!(app.search.music.page_title, "Example album");
    act(&mut app, &ctx, Action::Back);
    assert_eq!(app.page(), &Page::MusicBrowse("UCexample".into()));
    assert_eq!(app.search.music.page_title, "Example artist");
    assert!(!app.search.music.loading);
    act(&mut app, &ctx, Action::Forward);
    assert_eq!(app.page(), &Page::MusicBrowse("MPREexample".into()));
    act(&mut app, &ctx, Action::Back);
    act(&mut app, &ctx, Action::Back);
    assert_eq!(app.page(), &Page::Search);
    assert!(app.search.music.page_title.is_empty());
}
#[test]
fn queue_and_fullscreen_keep_lyrics_and_visualizer_context() {
    let ctx = egui::Context::default();
    let mut app = connected();
    app.open(Page::Search);
    app.spicy.open = true;
    act(&mut app, &ctx, Action::ToggleQueuePanel);
    assert!(app.show_queue_panel && app.spicy.open);
    assert_eq!(app.page(), &Page::Search);
    act(&mut app, &ctx, Action::ToggleFullscreen);
    assert!(app.lyrics_fullscreen.is_some() && app.spicy.open && app.show_queue_panel);
    act(&mut app, &ctx, Action::ToggleFullscreen);
    assert!(app.lyrics_fullscreen.is_none() && app.spicy.open);
    app.settings.addons.installed.insert(Id::Vivy);
    act(&mut app, &ctx, Action::Addon(ModCommand::Fullscreen));
    assert!(app.addons.fullscreen && !app.spicy.open);
    act(&mut app, &ctx, Action::ToggleQueuePanel);
    act(&mut app, &ctx, Action::ToggleQueuePanel);
    assert!(app.addons.fullscreen && app.show_queue_panel);
    act(&mut app, &ctx, Action::ToggleFullscreen);
    assert!(
        app.addons.fullscreen,
        "Fullscreen toggles the window while preserving the visualizer"
    );
    assert_eq!(app.page(), &Page::Search);
}
#[test]
fn artist_discography_excludes_related_artists_and_recommended_albums() {
    use serde_json::json;
    let item = |id: &str, title: &str| json!({"musicTwoRowItemRenderer":{"title":{"runs":[{"text":title}]},"navigationEndpoint":{"browseEndpoint":{"browseId":id}}}});
    let page = json!({"contents":[item("MPREown", "Own album"),item("UCother", "Other artist"),
        {"musicCarouselShelfRenderer":{"header":{"musicCarouselShelfBasicHeaderRenderer":{"title":{"runs":[{"text":"Fans might also like"}]}}},"contents":[item("MPREother","Other album")]}}]});
    let parsed = music::parse_artist(&page).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.entries[0].title, "Own album");
}
#[test]
fn existing_visualizer_settings_receive_compatible_defaults() {
    let visual: spotifast::addons::VisualConfig =
        serde_json::from_str(r#"{"style":1,"intensity":1.5,"particles":true}"#).unwrap();
    assert!(visual.show_cover && visual.show_title);
    assert_eq!(visual.fps, 30);
}
