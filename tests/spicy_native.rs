mod tests {
    use serde_json::json;
    use spotifast::model::Loadable;
    use spotifast::spicy_lyrics::*;
    const ID: &str = "1QV6tiMFM6fSOKOGLMHYYg";

    #[test]
    fn rendered_lyrics_share_storage_and_survive_track_changes() {
        use std::sync::Arc;
        let mut state = State {
            uri: Some("track".into()),
            generation: 1,
            ..Default::default()
        };
        let mut document = Document::default();
        document.lines.push(Line {
            text: "kept lyrics".into(),
            ..Default::default()
        });
        state.receive(
            "track",
            1,
            Ok(Fetch {
                document: Some(document),
                notice: None,
            }),
        );
        let Loadable::Loaded(original) = &state.result else {
            panic!("missing lyrics")
        };
        let rendered = Arc::clone(original);
        assert!(Arc::ptr_eq(original, &rendered));
        state.uri = Some("next".into());
        state.generation = 2;
        state.receive("next", 2, Ok(Fetch::default()));
        assert_eq!(
            rendered.document.as_ref().unwrap().lines[0].text,
            "kept lyrics"
        );
        let Loadable::Loaded(next) = &state.result else {
            panic!("missing next result")
        };
        assert!(!Arc::ptr_eq(&rendered, next));
    }

    #[test]
    fn syllables_keep_unicode_joining_duets_and_credits() {
        let json = json!({"Body": {
            "id": ID, "Type": "Syllable", "source": "spicy_lyrics",
            "UploadAttribution": {"Uploader": {"username": "maker", "url": "https://spicylyrics.org/uid/1"}},
            "Content": [{"OppositeAligned": true, "Lead": {"Syllables": [
                {"Text": "Șoa", "StartTime": 1.0, "EndTime": 1.4, "IsPartOfWord": true},
                {"Text": "re", "StartTime": 1.4, "EndTime": 2.0},
                {"Text": "луна", "StartTime": 2.0, "EndTime": 3.0}
            ]}, "Background": [{"Syllables": [{"Text": "oh", "StartTime": 1.5, "EndTime": 2.5}]}]}]
        }});
        let doc = from_spicy(&json, ID).unwrap();
        assert_eq!(doc.lines[0].text, "Șoare луна");
        assert!(doc.lines[0].opposite);
        assert!(doc.lines[1].background);
        assert_eq!(doc.credits.len(), 1);
        assert_eq!(doc.active_line(1800), Some(0));
        assert_eq!(doc.active_line(3000), None);
        assert_eq!(doc.lines[0].words[0].progress(1200), 0.5);
        assert!(doc.word_synced());
    }

    #[test]
    fn rejects_another_recording_and_missing_community_credit() {
        let mut json = json!({"Body": {"id": ID, "Type": "Static", "source": "spicy_lyrics", "Lines": [{"Text": "hello"}]}});
        assert!(from_spicy(&json, ID).is_err());
        json["Body"]["source"] = json!("spotify");
        assert!(from_spicy(&json, ID).is_ok());
        assert!(from_spicy(&json, "another-track").is_err());
    }

    #[test]
    fn plain_and_line_sources_never_invent_word_times() {
        for synced in [false, true] {
            let doc = from_more(&json!({"isSynced": synced, "provider": "LRCLIB", "lines": [
                {"words": "first", "startTimeMs": "0", "endTimeMs": "0"},
                {"words": "second", "startTimeMs": "1500", "endTimeMs": "0"}
            ]}))
            .unwrap();
            assert!(!doc.word_synced());
            assert_eq!(doc.synced(), synced);
            assert_eq!(doc.active_line(1500), synced.then_some(1));
        }
    }

    #[test]
    fn retries_and_track_changes_discard_old_answers() {
        let mut state = State {
            uri: Some("new".into()),
            generation: 2,
            ..Default::default()
        };
        state.receive("old", 2, Ok(Fetch::default()));
        state.receive("new", 1, Ok(Fetch::default()));
        assert!(matches!(state.result, Loadable::NotLoaded));
        state.receive("new", 2, Ok(Fetch::default()));
        assert!(matches!(state.result, Loadable::Loaded(_)));
    }

    #[test]
    fn offset_and_key_validation_are_bounded() {
        assert_eq!(position_with_offset(100, -500), 0);
        assert_eq!(position_with_offset(u32::MAX, 500), u32::MAX);
        assert!(valid_key("sl_pk_example"));
        assert!(!valid_key("sl_sk_secret"));
        assert!(!valid_key("sl_pk_bad\r\nheader"));
    }

    #[test]
    fn old_settings_default_to_an_unconfigured_provider() {
        let settings: spotifast::settings::Settings = serde_json::from_value(json!({})).unwrap();
        assert!(settings.spicy_lyrics_key.is_empty());
        assert_eq!(settings.spicy_lyrics_offset_ms, 0);
        let round_trip: spotifast::settings::Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(round_trip.spicy_lyrics_offset_ms, 0);
    }

    #[tokio::test]
    async fn a_fresh_cache_preserves_word_times_and_visible_credits() {
        let cache =
            std::env::temp_dir().join(format!("vibefy-spicy-cache-{}", rand::random::<u64>()));
        tokio::fs::create_dir_all(&cache).await.unwrap();
        let doc = Document {
            provider: "Spicy Lyrics".into(),
            credits: vec![Credit {
                name: "Uploaded by Demo".into(),
                url: Some("https://spicylyrics.org/uid/1".into()),
            }],
            lines: vec![Line {
                text: "Demo".into(),
                start_ms: Some(1000),
                end_ms: Some(2000),
                words: vec![Word {
                    text: "Demo".into(),
                    start_ms: 1000,
                    end_ms: 2000,
                    joined: false,
                }],
                ..Default::default()
            }],
            instrumental: false,
        };
        tokio::fs::write(
            cache.join(format!("spicy-native-v1-{ID}.json")),
            serde_json::to_vec(&doc).unwrap(),
        )
        .await
        .unwrap();
        let found = fetch(
            &reqwest::Client::new(),
            &cache,
            &format!("spotify:track:{ID}"),
            &spotifast::lyrics::Query {
                artist: "Demo".into(),
                title: "Demo".into(),
                album: String::new(),
                duration_ms: 5000,
            },
            "",
            false,
        )
        .await
        .unwrap()
        .document
        .unwrap();
        assert!(found.word_synced());
        assert_eq!(found.lines[0].words[0].progress(1500), 0.5);
        assert_eq!(found.credits[0].name, "Uploaded by Demo");
        tokio::fs::remove_dir_all(cache).await.unwrap();
    }
}

#[cfg(feature = "demo")]
mod ui {
    use spotifast::app::{App, AppOptions};
    use spotifast::backend::Waker;
    use spotifast::model::Action;
    use spotifast::paths::AppDirs;
    use spotifast::settings::Settings;

    fn app(name: &str) -> (egui::Context, App, std::path::PathBuf) {
        let root =
            std::env::temp_dir().join(format!("vibefy-spicy-{name}-{}", rand::random::<u64>()));
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let waker = Waker::default();
        waker.attach(&ctx);
        let mut app = App::new(
            &waker,
            AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            Settings::default(),
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

    fn frame(ctx: &egui::Context, app: &mut App, width: f32) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 800.0),
                )),
                ..Default::default()
            },
            |ui| app.frame_ui(ui),
        );
        output.textures_delta.clear();
        output
    }

    #[test]
    fn mods_button_fits_beside_search_and_opens_catalogue() {
        use spotifast::model::Page;
        let (ctx, mut app, root) = app("mods");
        for width in [760.0, 1280.0] {
            frame(&ctx, &mut app, width);
            let output = frame(&ctx, &mut app, width);
            let tree = output.platform_output.accesskit_update.unwrap();
            let cart = tree
                .nodes
                .iter()
                .find_map(|(_, node)| {
                    (node.role() == egui::accesskit::Role::Button && node.label() == Some("Mods"))
                        .then(|| node.bounds().unwrap())
                })
                .expect("Mods cart button");
            let search = ctx
                .read_response(egui::Id::new("global-search"))
                .unwrap()
                .rect;
            assert!(cart.x0 >= search.right() as f64);
            assert!(cart.x1 < width as f64);
        }
        app.actions.push(Action::Open(Page::Mods));
        frame(&ctx, &mut app, 760.0);
        assert_eq!(app.page(), &Page::Mods);
        let output = frame(&ctx, &mut app, 760.0);
        let tree = output.platform_output.accesskit_update.unwrap();
        assert!(tree.nodes.iter().any(|(_, node)| {
            node.value()
                .or(node.label())
                .is_some_and(|text| text.contains("Some mods may reduce performance"))
        }));
        assert_eq!(Page::decode(&Page::Mods.encode()), Some(Page::Mods));
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn unified_lyrics_button_stays_clear_of_transport_at_narrow_sizes() {
        let (ctx, mut app, root) = app("buttons");
        for width in [760.0, 800.0, 900.0, 1280.0, 1600.0] {
            frame(&ctx, &mut app, width);
            let output = frame(&ctx, &mut app, width);
            let tree = output.platform_output.accesskit_update.unwrap();
            let button = |name: &str| {
                tree.nodes
                    .iter()
                    .find_map(|(_, node)| {
                        (node.role() == egui::accesskit::Role::Button && node.label() == Some(name))
                            .then(|| node.bounds().unwrap())
                    })
                    .unwrap_or_else(|| panic!("Missing {name} at {width}"))
            };
            let normal = button("Lyrics");
            let repeat = tree
                .nodes
                .iter()
                .find_map(|(_, node)| {
                    (node.role() == egui::accesskit::Role::Button
                        && node
                            .label()
                            .is_some_and(|label| label.starts_with("Repeat")))
                    .then(|| node.bounds().unwrap())
                })
                .unwrap();
            assert!(
                !tree
                    .nodes
                    .iter()
                    .any(|(_, node)| node.label() == Some("Spicy Lyrics"))
            );
            assert_eq!(
                tree.nodes
                    .iter()
                    .filter(|(_, node)| node.role() == egui::accesskit::Role::Button
                        && node.label() == Some("Lyrics"))
                    .count(),
                1
            );
            assert!(
                normal.x0 - repeat.x1 >= 10.0,
                "clear of transport at {width}: {normal:?}, {repeat:?}"
            );
        }
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_shortcut_toggles_the_unified_lyrics_view() {
        let (ctx, mut app, root) = app("toggle");
        app.actions.push(Action::ToggleSpicyLyrics);
        frame(&ctx, &mut app, 1280.0);
        assert!(app.spicy.open);
        assert!(!app.show_lyrics_panel);
        assert!(!app.show_queue_panel);
        app.actions.push(Action::SetLyricsFullscreen(true));
        frame(&ctx, &mut app, 1280.0);
        assert!(app.spicy.open && app.lyrics_fullscreen.is_some());
        assert!(!app.show_lyrics_panel);
        app.actions.push(Action::SetLyricsFullscreen(false));
        frame(&ctx, &mut app, 1280.0);
        assert!(app.spicy.open && app.lyrics_fullscreen.is_none());
        app.actions.push(Action::ToggleLyricsPanel);
        frame(&ctx, &mut app, 1280.0);
        assert!(!app.spicy.open);
        assert!(!app.show_lyrics_panel);
        app.actions.push(Action::ToggleSpicyLyrics);
        frame(&ctx, &mut app, 1280.0);
        assert!(app.spicy.open);
        app.actions.push(Action::ToggleSpicyLyrics);
        frame(&ctx, &mut app, 1280.0);
        assert!(!app.spicy.open);
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn about_puts_vibefy_before_upstream_credits_in_both_themes() {
        let (ctx, mut app, root) = app("about");
        for theme in ["light", "dark"] {
            spotifast::demo::apply_flags(&mut app, None, Some(theme));
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("settings-filter"), "About".to_owned())
            });
            for width in [760.0, 1280.0] {
                let mut output = None;
                for _ in 0..2 {
                    let mut rendered = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width, 1000.0),
                            )),
                            ..Default::default()
                        },
                        |ui| spotifast::ui::settings::show(&mut app, ui),
                    );
                    rendered.textures_delta.clear();
                    output = Some(rendered);
                }
                let output = output.unwrap();
                let tree = output.platform_output.accesskit_update.unwrap();
                let labels: Vec<_> = tree
                    .nodes
                    .iter()
                    .filter_map(|(_, node)| node.value().or(node.label()))
                    .collect();
                assert!(
                    labels.iter().any(|text| text.starts_with("Vibefy ")),
                    "{labels:?}"
                );
                assert!(labels.contains(&"Built on Spotifast"), "{labels:?}");
                assert!(labels.contains(&"Carmine Paolino"));
            }
        }
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn credits_are_buttons_and_storage_exposes_independent_clear_actions() {
        let (ctx, mut app, root) = app("credits-storage");
        for (filter, expected) in [
            (
                "About",
                vec![
                    "Spotifast · GitHub",
                    "Carmine Paolino",
                    "MIT license",
                    "Spicy Lyrics · GitHub",
                    "More Lyrics · GitHub",
                ],
            ),
            (
                "Storage",
                vec![
                    "Clear all cache",
                    "Clear audio",
                    "Clear artwork",
                    "Clear lyrics",
                    "Clear library",
                    "Clear history",
                ],
            ),
        ] {
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("settings-filter"), filter.to_owned())
            });
            for _ in 0..2 {
                app.cache_busy = false;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(760.0, 1400.0),
                        )),
                        ..Default::default()
                    },
                    |ui| spotifast::ui::settings::show(&mut app, ui),
                );
                output.textures_delta.clear();
                let tree = output.platform_output.accesskit_update.unwrap();
                for label in &expected {
                    assert!(
                        tree.nodes
                            .iter()
                            .any(|(_, node)| node.role() == egui::accesskit::Role::Button
                                && node.label() == Some(*label)),
                        "Missing button {label}"
                    );
                }
            }
        }
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn credit_button_click_emits_the_expected_browser_action() {
        let (ctx, mut app, root) = app("credit-click");
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("settings-filter"), "About".to_owned()));
        let mut bounds = None;
        for _ in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 1400.0),
                    )),
                    ..Default::default()
                },
                |ui| spotifast::ui::settings::show(&mut app, ui),
            );
            output.textures_delta.clear();
            bounds = output
                .platform_output
                .accesskit_update
                .unwrap()
                .nodes
                .iter()
                .find_map(|(_, node)| {
                    (node.role() == egui::accesskit::Role::Button
                        && node.label() == Some("Spotifast · GitHub"))
                    .then(|| node.bounds().unwrap())
                });
        }
        let bounds = bounds.unwrap();
        let pos = egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        );
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 1400.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    },
                ],
                ..Default::default()
            },
            |ui| spotifast::ui::settings::show(&mut app, ui),
        );
        output.textures_delta.clear();
        assert!(app.actions.iter().any(|action|matches!(action,Action::OpenUrl(url) if url=="https://github.com/crmne/spotifast")));
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn storage_recommends_cleanup_above_two_gigabytes() {
        let (ctx, mut app, root) = app("cache-advisory");
        app.cache_usage = Some(spotifast::storage::Usage {
            audio: 2_100_000_000,
            ..Default::default()
        });
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new("settings-filter"), "Storage".to_owned())
        });
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 1400.0),
                )),
                ..Default::default()
            },
            |ui| spotifast::ui::settings::show(&mut app, ui),
        );
        output.textures_delta.clear();
        let tree = output.platform_output.accesskit_update.unwrap();
        assert!(tree.nodes.iter().any(|(_, node)| {
            node.value()
                .or(node.label())
                .is_some_and(|text| text.contains("Cache exceeds 2 GB"))
        }));
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn redundant_theme_selectors_are_absent() {
        let (ctx, mut app, root) = app("appearance-clean");
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new("settings-filter"), "Appearance".to_owned())
        });
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 1400.0),
                )),
                ..Default::default()
            },
            |ui| spotifast::ui::settings::show(&mut app, ui),
        );
        output.textures_delta.clear();
        let tree = output.platform_output.accesskit_update.unwrap();
        for (_, node) in tree.nodes {
            assert!(!matches!(
                node.label(),
                Some("Light mode theme" | "Dark mode theme")
            ));
        }
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn word_line_plain_and_error_views_render_in_both_themes() {
        let (ctx, mut app, root) = app("states");
        for theme in ["light", "dark"] {
            for surface in ["spicy-words", "spicy-plain", "spicy-empty", "spicy-error"] {
                spotifast::demo::apply_flags(&mut app, None, Some(&format!("{theme},{surface}")));
                for width in [760.0, 1280.0] {
                    frame(&ctx, &mut app, width);
                    let output = frame(&ctx, &mut app, width);
                    assert!(!output.shapes.is_empty());
                    assert!(app.spicy.open);
                }
            }
        }
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn following_offset_and_key_changes_apply_after_drawing() {
        let (ctx, mut app, root) = app("settings");
        spotifast::demo::apply_flags(&mut app, None, Some("spicy"));
        app.spicy.following = false;
        app.spicy.shown_line = Some(Some(2));
        app.actions.push(Action::SetSpicyFollow(true));
        frame(&ctx, &mut app, 1280.0);
        assert!(app.spicy.following);
        assert_eq!(app.spicy.shown_line, None);
        app.actions.push(Action::ConfigureSpicyLyrics {
            key: "sl_sk_do_not_store".into(),
            offset_ms: 123,
        });
        frame(&ctx, &mut app, 1280.0);
        assert!(app.settings.spicy_lyrics_key.is_empty());
        let generation = app.spicy.generation;
        app.actions.push(Action::ConfigureSpicyLyrics {
            key: "sl_pk_example".into(),
            offset_ms: 99999,
        });
        frame(&ctx, &mut app, 1280.0);
        assert_eq!(app.settings.spicy_lyrics_key, "sl_pk_example");
        assert_eq!(app.settings.spicy_lyrics_offset_ms, 30000);
        assert!(app.spicy.generation > generation);
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn expanded_settings_render_all_tabs_and_background_modes() {
        use spotifast::spicy_lyrics::{Appearance, Background, FontStyle, TextAlign};
        let (ctx, mut app, root) = app("expanded-settings");
        spotifast::demo::apply_flags(&mut app, None, Some("spicy"));
        let config = egui::Id::new("spicy-lyrics-config");
        ctx.data_mut(|data| data.insert_temp(config, true));
        for mode in [
            Background::BlurredCover,
            Background::AnimatedColors,
            Background::CoverArt,
            Background::CoverColors,
            Background::Solid,
            Background::Theme,
            Background::CustomImage,
        ] {
            let appearance = Appearance {
                background: mode,
                alignment: TextAlign::Center,
                font_style: FontStyle::Regular,
                ..Default::default()
            };
            app.actions.push(Action::SetSpicyAppearance(appearance));
            frame(&ctx, &mut app, 1280.0);
            for tab in 0..4 {
                ctx.data_mut(|data| data.insert_temp(config.with("tab"), tab as usize));
                let output = frame(&ctx, &mut app, 760.0);
                assert!(!output.shapes.is_empty());
            }
            assert_eq!(app.settings.spicy_lyrics_appearance.background, mode);
        }
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn calibration_actions_persist_without_leaking_to_another_recording() {
        let (ctx, mut app, root) = app("calibration-actions");
        spotifast::demo::apply_flags(&mut app, None, Some("spicy"));
        let key = app.spicy_calibration_key();
        app.actions.push(Action::CalibrateSpicyLyrics {
            audio_ms: 12_000,
            lyric_ms: 10_000,
        });
        frame(&ctx, &mut app, 1280.0);
        let saved = app.settings.spicy_lyrics_calibrations.get(&key).unwrap();
        assert_eq!(saved.lyric_position(32_000, 0), 30_000);
        app.actions.push(Action::SetSpicyTrackOffset(150));
        frame(&ctx, &mut app, 1280.0);
        assert_eq!(
            app.settings.spicy_lyrics_calibrations[&key].lyric_position(32_000, 0),
            30_150
        );
        let json = serde_json::to_string(&app.settings).unwrap();
        let restored: spotifast::settings::Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.spicy_lyrics_calibrations[&key].points.len(), 1);
        app.local.source = Some(spotifast::player::PlaybackSource {
            youtube_id: "different-video".into(),
            title: "Other recording".into(),
            duration_ms: 180_000,
        });
        let mut local_track = spotifast::player::LocalTrack::default();
        local_track.uri = app.spicy.uri.clone().unwrap();
        app.local.track = Some(local_track);
        assert_ne!(key, app.spicy_calibration_key());
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn automatic_timing_is_saved_separately_and_manual_points_take_priority() {
        use spotifast::spicy_lyrics::{Calibration, SyncPoint};
        let (ctx, mut app, root) = app("auto-priority");
        spotifast::demo::apply_flags(&mut app, None, Some("spicy"));
        let key = app.spicy_calibration_key();
        app.settings.spicy_lyrics_auto_results.insert(
            key.clone(),
            spotifast::auto_sync::Alignment {
                points: vec![SyncPoint {
                    audio_ms: 15_000,
                    lyric_ms: 10_000,
                }],
                matched_lines: 5,
                median_error_ms: 100,
                version: spotifast::auto_sync::ALIGNER_VERSION,
                comparison: Default::default(),
            },
        );
        assert_eq!(app.spicy_calibration().lyric_position(25_000, 0), 20_000);
        app.settings.spicy_lyrics_calibrations.insert(
            key.clone(),
            Calibration {
                offset_ms: 150,
                ..Default::default()
            },
        );
        assert_eq!(app.spicy_calibration().lyric_position(25_000, 0), 25_150);
        app.settings.spicy_lyrics_calibrations.insert(
            key.clone(),
            Calibration {
                points: vec![SyncPoint {
                    audio_ms: 12_000,
                    lyric_ms: 10_000,
                }],
                ..Default::default()
            },
        );
        assert_eq!(app.spicy_calibration().lyric_position(25_000, 0), 23_000);
        app.actions.push(Action::ResetSpicyCalibration);
        frame(&ctx, &mut app, 1280.0);
        assert_eq!(app.spicy_calibration().lyric_position(25_000, 0), 20_000);
        app.actions.push(Action::SetSpicyAutoSync(false));
        frame(&ctx, &mut app, 1280.0);
        assert_eq!(app.spicy_calibration().lyric_position(25_000, 0), 25_000);
        let restored: Settings =
            serde_json::from_str(&serde_json::to_string(&app.settings).unwrap()).unwrap();
        assert_eq!(restored.spicy_lyrics_auto_results[&key].matched_lines, 5);
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn custom_background_is_decoded_off_the_ui_thread() {
        use spotifast::spicy_lyrics::Background;
        let (ctx, mut app, root) = app("custom-background");
        std::fs::create_dir_all(&root).unwrap();
        let image_path = root.join("background.png");
        image::RgbImage::from_pixel(128, 96, image::Rgb([40u8, 100, 170]))
            .save(&image_path)
            .unwrap();
        let uri = reqwest::Url::from_file_path(&image_path)
            .unwrap()
            .to_string();
        spotifast::demo::apply_flags(&mut app, None, Some("spicy"));
        app.settings.spicy_lyrics_appearance.background = Background::CustomImage;
        app.settings.spicy_lyrics_appearance.custom_image = Some(uri.clone());
        app.settings.spicy_lyrics_appearance.blur = 0.0;
        let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut loaded = false;
        while std::time::Instant::now() < until {
            frame(&ctx, &mut app, 1280.0);
            if let Some(texture) =
                app.lyrics_backdrop
                    .texture_with_blur(&ctx, app.backend.art(), Some(&uri), 0.0)
            {
                assert_eq!(texture.size(), [128, 96]);
                loaded = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(loaded, "local background was not delivered to the UI");
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }
}
