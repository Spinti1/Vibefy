include!("onboarding.rs");
#[test]
fn linked_playlist_exposes_rename_without_settings() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    theme::install(&ctx);
    let mut app = fixture("rename-linked");
    app.offline = true;
    let source = "https://www.youtube.com/playlist?list=PLexample12345";
    app.mirror.lists = vec![
        spotifast::playlist_mirror::ListView {
            spotify_id: "0123456789012345678901".into(),
            youtube_id: "PLexample12345".into(),
            source_uris: vec![],
            missing: 0,
            copied: 0,
            uncertain: false,
            issue: None,
            collection: spotifast::addons::Collection {
                title: "Example playlist".into(),
                source: source.into(),
                tracks: vec![],
                image: None,
                owner: None,
            },
        }
        .into(),
    ];
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000., 800.),
            )),
            ..Default::default()
        },
        |ui| spotifast::ui::collection::source_playlist(&mut app, ui, source),
    );
    let tree = output.platform_output.accesskit_update.unwrap();
    let bounds = button(&tree, "Rename playlist").bounds().unwrap();
    assert!(bounds.y1 < 800. && bounds.x1 < 1000.);
}
