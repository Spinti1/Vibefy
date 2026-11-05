include!("onboarding.rs");
#[test]
fn juice_matching_ranks_exact_and_rejects_unrelated_titles() {
    use spotifast::music_accounts::juice_title_score as score;
    assert_eq!(score("Come & Go", "Come & Go"), 1.0);
    assert!(score("Come & Go", "Come & Go (V4)") >= 0.7);
    assert!(score("Lucid Dreams", "Lucid Dreems") >= 0.7);
    assert!(score("Lucid Dreams", "All Girls Are The Same") < 0.7);
}
#[test]
fn juice_private_and_collaborated_lists_are_parsed_once() {
    let value = serde_json::json!({"data":[{"_id":"11111111-1111-1111-1111-111111111111","name":"Private list","isPublic":false}],"collaborated":[{"_id":"11111111-1111-1111-1111-111111111111","name":"Duplicate"},{"_id":"22222222-2222-2222-2222-222222222222","name":"Shared list"}]});
    let lists = spotifast::music_accounts::parse_juice_playlists(&value);
    assert_eq!(lists.len(), 2);
    assert_eq!(lists[0].title, "Private list");
    assert!(
        !format!(
            "{:?}",
            spotifast::music_accounts::Action::Login(spotifast::music_accounts::Password(
                "secret-example".into()
            ))
        )
        .contains("secret-example")
    );
}
#[test]
fn youtube_plain_lyrics_preserve_lines_without_inventing_timing() {
    let response = serde_json::json!({"contents":{"musicDescriptionShelfRenderer":{"description":{"runs":[{"text":"First line\n\nSecond line"}]}}}});
    let lyrics = spotifast::music::parse_music_lyrics(&response).unwrap();
    assert_eq!(lyrics.lines.len(), 2);
    assert!(!lyrics.synced);
    assert!(lyrics.lines.iter().all(|l| l.at_ms.is_none()));
    assert!(
        spotifast::music::parse_music_lyrics(&serde_json::json!({"description":"Not lyrics"}))
            .is_none()
    );
}
#[test]
fn home_provider_shelves_are_ordered_without_linked_section() {
    for (width, dark) in [
        (700.0, true),
        (1280.0, true),
        (700.0, false),
        (1280.0, false),
    ] {
        let ctx = egui::Context::default();
        ctx.set_visuals(if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        ctx.enable_accesskit();
        theme::install(&ctx);
        let mut app = fixture("provider-shelves");
        app.offline = true;
        app.user = Some(spotifast::api::models::User {
            id: "example".into(),
            ..Default::default()
        });
        app.network_reachable = true;
        app.music_accounts[0].connected = true;
        app.settings
            .addons
            .set(spotifast::addons::Id::JuiceVault, true);
        app.library.playlists = spotifast::model::Loadable::Loaded(vec![
            spotifast::api::models::Playlist {
                id: "1111111111111111111111".into(),
                name: "Spotify copy".into(),
                ..Default::default()
            },
            spotifast::api::models::Playlist {
                id: "2222222222222222222222".into(),
                name: "Spotify example".into(),
                ..Default::default()
            },
        ]);
        app.music_accounts[0].playlists = vec![
            spotifast::music_accounts::LibraryPlaylist {
                id: "VLPLexample12345".into(),
                title: "YouTube copy".into(),
                image: None,
            },
            spotifast::music_accounts::LibraryPlaylist {
                id: "VLPLnative123456".into(),
                title: "YouTube example".into(),
                image: None,
            },
        ];
        app.music_accounts[1].playlists = vec![spotifast::music_accounts::LibraryPlaylist {
            id: "33333333-3333-3333-3333-333333333333".into(),
            title: "Juice example".into(),
            image: None,
        }];
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 2400.0),
                )),
                ..Default::default()
            },
            |ui| spotifast::ui::home::show(&mut app, ui),
        );
        output.textures_delta.clear();
        let tree = output.platform_output.accesskit_update.unwrap();
        let labels = [
            "Spotify copy",
            "Spotify example",
            "YouTube example",
            "Juice example",
        ];
        let positions: Vec<_> = labels[1..]
            .iter()
            .map(|label| button(&tree, label).bounds().unwrap().y0)
            .collect();
        assert!(positions.windows(2).all(|p| p[0] < p[1]));
        assert_eq!(
            tree.nodes
                .iter()
                .filter(|(_, n)| n.label() == Some("Spotify copy")
                    && n.role() == egui::accesskit::Role::Button)
                .count(),
            1
        );
        assert!(!tree.nodes.iter().any(|(_, n)| n.label() == Some("Linked")));
        for label in ["Spotify example", "YouTube example", "Juice example"] {
            button(&tree, label);
        }
    }
}
