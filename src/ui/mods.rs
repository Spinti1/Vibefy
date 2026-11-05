//! Native, opt-in mod catalogue and source collections.
use crate::addons::{self, Command, Id, Request};
use crate::app::App;
use crate::model::{Action, Page};
use crate::theme::{self, Icon};
use egui::{CornerRadius, Frame, Margin, Stroke};

fn button(app: &mut App, ui: &mut egui::Ui, text: &str, command: Command, active: bool) {
    if theme::soft_button(ui, &app.palette, None, text, active).clicked() {
        app.actions.push(Action::Addon(command));
    }
}
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(28.0), egui::Sense::hover());
        theme::paint_icon(ui, Icon::ShoppingCart, rect, 26.0, palette.accent);
        theme::text(ui, "Mods", theme::bold(28.0), palette.text);
    });
    ui.label(
        egui::RichText::new("Make Vibefy yours. Install only the add-ons you want to use.")
            .color(palette.secondary),
    );
    ui.add_space(8.0);
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(16.0), egui::Sense::hover());
        theme::paint_icon(ui, Icon::CircleAlert, rect, 15.0, palette.secondary);
        ui.add(egui::Label::new(egui::RichText::new("Some mods may reduce performance and increase CPU, memory, or GPU usage. System requirements may vary.").size(12.0).color(palette.secondary)).wrap());
    });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.addons.filter)
                .hint_text("Find an add-on…")
                .desired_width((ui.available_width() - 95.0).max(100.0))
                .char_limit(80),
        );
        if theme::soft_button(ui, &palette, None, "Show all", false).clicked() {
            app.addons.filter.clear();
        }
    });
    let filter = app.addons.filter.trim().to_lowercase();
    let visible: Vec<_> = addons::ALL
        .into_iter()
        .filter(|id| {
            filter.is_empty()
                || id.name().to_lowercase().contains(&filter)
                || id.description().to_lowercase().contains(&filter)
        })
        .collect();
    ui.add_space(12.0);
    ui.small(format!(
        "{} of {} optional add-ons · 2 included lyrics mods · scroll to explore",
        visible.len(),
        addons::ALL.len()
    ));
    let columns = if ui.available_width() >= 1000.0 {
        3
    } else if ui.available_width() >= 620.0 {
        2
    } else {
        1
    };
    let gap = 14.0;
    let width = (ui.available_width() - gap * (columns - 1) as f32) / columns as f32;
    // Equal-width catalogue cards; settings live in a separate detail panel.
    for row in visible.chunks(columns) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for &id in row {
                ui.push_id(id, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        card(app, ui, id, width);
                    });
                });
            }
        });
        ui.add_space(gap);
    }
    if let Some(id) = app.addons.settings_open {
        if app.settings.addons.installed.contains(&id) {
            let mut open = true;
            egui::Window::new(format!("{} settings", id.name()))
                .id(egui::Id::new("mod-settings"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(400.0)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    ui.set_max_width(
                        400.0_f32.min((ui.ctx().content_rect().width() - 48.0).max(200.0)),
                    );
                    egui::ScrollArea::vertical()
                        .max_height((ui.ctx().content_rect().height() - 140.0).max(160.0))
                        .show(ui, |ui| controls(app, ui, id));
                });
            if !open {
                app.addons.settings_open = None;
            }
        } else {
            app.addons.settings_open = None;
        }
    }
    // Lyrics integrations predate the optional catalogue. Keep them available
    // here as included mods without changing existing listeners' preferences.
    let mut included_matches = 0;
    for (name, description, source) in [
        (
            "Spicy Lyrics",
            "Included · synchronized lyrics, backgrounds and display settings.",
            "https://github.com/Spikerko/spicy-lyrics",
        ),
        (
            "More Lyrics",
            "Included · an additional provider in the unified lyrics view.",
            "https://github.com/Kamiloo13/spicetify-extensions/tree/main/extensions/more-lyrics",
        ),
    ] {
        if !filter.is_empty()
            && !format!("{name} {description}")
                .to_lowercase()
                .contains(&filter)
        {
            continue;
        }
        included_matches += 1;
        Frame::new()
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(14)
            .inner_margin(16)
            .show(ui, |ui| {
                ui.set_width((ui.available_width() - 2.0).max(100.0));
                theme::text(ui, name, theme::bold(17.0), palette.text);
                ui.label(description);
                ui.horizontal_wrapped(|ui| {
                    if theme::soft_button(
                        ui,
                        &palette,
                        Some(Icon::AudioLines),
                        "Open lyrics",
                        false,
                    )
                    .clicked()
                    {
                        app.actions.push(Action::ToggleSpicyLyrics);
                    }
                    if theme::soft_button(
                        ui,
                        &palette,
                        Some(Icon::ExternalLink),
                        "Credits & source",
                        false,
                    )
                    .clicked()
                    {
                        app.actions.push(Action::OpenUrl(source.into()));
                    }
                });
            });
        ui.add_space(8.0);
    }
    if visible.is_empty() && included_matches == 0 {
        ui.label("No add-ons match your search.");
    }
    if app.addons.busy {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading source…");
        });
        button(app, ui, "Cancel", Command::CancelFetch, false);
    }
    if let Some(message) = &app.addons.message {
        ui.label(message);
    }
    if let Some(collection) = &app.addons.result {
        ui.separator();
        ui.heading(&collection.title);
        let saved = app
            .addons
            .collections
            .iter()
            .position(|saved| saved.source == collection.source);
        collection_controls(ui, &palette, &mut app.actions, collection, saved);
        tracks(&mut app.actions, ui, &collection.tracks, "source-results");
    }
    if !app.addons.collections.is_empty() {
        ui.add_space(12.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading("Saved playlists");
            if theme::soft_button(ui, &palette, Some(Icon::House), "Open Home", false).clicked() {
                app.actions.push(Action::Open(Page::Home));
            }
        });
        ui.small("Your saved source playlists also appear on Home.");
        for collection in &app.addons.collections {
            if theme::soft_button(
                ui,
                &palette,
                Some(Icon::ListMusic),
                &format!("{} · {} tracks", collection.title, collection.tracks.len()),
                false,
            )
            .clicked()
            {
                app.actions.push(Action::Open(Page::LocalCollection(
                    collection.source.clone(),
                )));
            }
        }
    }
}

fn card(app: &mut App, ui: &mut egui::Ui, id: Id, width: f32) {
    let palette = app.palette;
    let installed = app.settings.addons.installed.contains(&id);
    let active = app.settings.addons.active(id) && app.addons.modules.contains_key(&id);
    let pending = app.addons.package_pending.contains(&id);
    let busy = app.addons.package_busy.contains(&id);
    let missing = installed && !app.addons.modules.contains_key(&id) && !pending;
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width((width - 34.0).max(100.0));
            ui.set_min_height(220.0);
            ui.allocate_ui(egui::vec2(ui.available_width(), 48.0), |ui| {
                ui.set_min_height(48.0);
                ui.horizontal_top(|ui| {
                    let icon = match id {
                        Id::YouTube => Icon::ListVideo,
                        Id::WaveLink => Icon::AudioLines,
                        Id::Vivy => Icon::AudioLines,
                        Id::Backmusic => Icon::Disc,
                        _ => Icon::Sparkles,
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(32.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, 8.0, palette.accent.gamma_multiply(0.12));
                    theme::paint_icon(ui, icon, rect.shrink(6.0), 20.0, palette.accent);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(id.name())
                                .font(theme::bold(17.0))
                                .color(palette.text),
                        );
                        ui.label(
                            egui::RichText::new(if busy {
                                "Downloading…"
                            } else if pending {
                                "Installed · reload to activate"
                            } else if missing {
                                "Download required"
                            } else if active {
                                "Enabled"
                            } else if installed {
                                "Installed · disabled"
                            } else {
                                "Available to install"
                            })
                            .size(11.0)
                            .color(if active {
                                palette.accent
                            } else {
                                palette.secondary
                            }),
                        );
                    });
                });
            });
            ui.add_space(6.0);
            ui.allocate_ui(egui::vec2(ui.available_width(), 64.0), |ui| {
                ui.set_min_height(64.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(id.description())
                            .size(13.0)
                            .color(palette.secondary),
                    )
                    .wrap(),
                );
            });
            ui.add_space(10.0);
            ui.add_enabled_ui(!busy, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if missing {
                        button(app, ui, "Download", Command::Enable(id, true), true);
                        button(app, ui, "Uninstall", Command::Uninstall(id), false);
                    } else if pending {
                        button(app, ui, "Reload now", Command::ReloadNow, true);
                        button(app, ui, "Uninstall", Command::Uninstall(id), false);
                    } else if installed {
                        button(
                            app,
                            ui,
                            if active { "Disable" } else { "Enable" },
                            Command::Enable(id, !active),
                            active,
                        );
                        button(app, ui, "Uninstall", Command::Uninstall(id), false);
                    } else {
                        button(app, ui, "Install", Command::Enable(id, true), true);
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.add_enabled_ui(active, |ui| {
                    if theme::soft_button(
                        ui,
                        &palette,
                        Some(Icon::Settings),
                        "Settings",
                        app.addons.settings_open == Some(id),
                    )
                    .clicked()
                    {
                        app.addons.settings_open = if app.addons.settings_open == Some(id) {
                            None
                        } else {
                            Some(id)
                        };
                    }
                });
                if let Some(source) = id.source() {
                    if theme::soft_button(
                        ui,
                        &palette,
                        Some(Icon::ExternalLink),
                        "Credits & source",
                        false,
                    )
                    .clicked()
                    {
                        app.actions.push(Action::OpenUrl(source.into()));
                    }
                }
            });
            if let Some(error) = app.addons.package_errors.get(&id) {
                ui.add(
                    egui::Label::new(egui::RichText::new(error).small().color(palette.warning))
                        .wrap(),
                );
            }
        });
}

fn controls(app: &mut App, ui: &mut egui::Ui, id: Id) {
    match id {
        Id::JuiceVault => {
            if theme::soft_button(
                ui,
                &app.palette,
                Some(Icon::Search),
                "JuiceVault search",
                true,
            )
            .clicked()
            {
                app.actions.push(Action::Open(Page::JuiceVault));
            }
            super::music_accounts::juice_connect(app, ui);
        }
        Id::WaveLink => {
            if theme::soft_button(
                ui,
                &app.palette,
                Some(Icon::Search),
                "SoundCloud search",
                true,
            )
            .clicked()
            {
                app.actions.push(Action::Open(Page::SoundCloud));
            }
            ui.small(
                "WaveLink by xoft122 (MIT). Public, unprotected tracks; no account token stored.",
            );
        }
        Id::YouTube => {
            ui.label("YouTube playlist URL (up to 500 entries)");
            ui.add(
                egui::TextEdit::singleline(&mut app.addons.youtube_url)
                    .hint_text("https://www.youtube.com/playlist?list=…")
                    .desired_width(ui.available_width())
                    .char_limit(500),
            );
            let url = app.addons.youtube_url.clone();
            ui.add_enabled_ui(!app.addons.busy, |ui| {
                button(
                    app,
                    ui,
                    "Import playlist",
                    Command::Fetch(Request::YouTube(url)),
                    false,
                )
            });
            ui.small("Imported playlists are saved on Home automatically. Unavailable videos may be omitted.");
        }
        Id::Vivy | Id::Backmusic | Id::SoundVisualizer => {
            ui.small("Open Visualizers beside Lyrics. Display settings are available inside the visualizer; fullscreen is optional.");
        }
        Id::Oneko => {
            ui.small("Original Oneko sprite: kyrie25 / adryd325 (MIT).");
        }
        Id::CatJam => {
            let mut percent = (app.settings.addons.cat_scale() * 100.0).round();
            ui.horizontal(|ui| {
                ui.label("Size");
                if ui
                    .add(
                        egui::Slider::new(&mut percent, 50.0..=200.0)
                            .suffix("%")
                            .step_by(1.0),
                    )
                    .changed()
                {
                    app.actions
                        .push(Action::Addon(Command::CatJamScale(percent / 100.0)));
                }
            });
            if theme::soft_button(ui, &app.palette, Some(Icon::Refresh), "Reset size", false)
                .clicked()
            {
                app.actions.push(Action::Addon(Command::CatJamScale(0.9)));
            }
            ui.small("Original Cat Jam clip: BlafKing. Speed follows the estimated track BPM.");
        }
        Id::TrackPeek => {
            ui.small("Previous and upcoming songs stay visible beside the playback controls. Right-click a song and choose Preview audio. Stop or close the preview to return to your music.");
        }
        Id::Nyan => {
            ui.small("Original six Nyan frames: NoahClemmensen.");
        }
    }
    if matches!(id, Id::Nyan | Id::CatJam) {
        if let Some(bpm) = app.addons.tempo_bpm {
            ui.small(format!("Estimated track tempo: {bpm:.0} BPM"));
        } else if app.addons.tempo_pending {
            ui.small("Estimating track tempo…");
        }
    }
}

/// WaveLink keeps its query and results independent of Spotify search.
pub fn soundcloud(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    ui.add_space(12.0);
    theme::text(ui, "SoundCloud search", theme::bold(28.0), palette.text);
    ui.label(
        egui::RichText::new("Find public tracks and artists with WaveLink.")
            .color(palette.secondary),
    );
    if !app.settings.addons.installed.contains(&Id::WaveLink) {
        ui.label("Install WaveLink from Mods to use SoundCloud search.");
        if theme::soft_button(ui, &palette, Some(Icon::ShoppingCart), "Open Mods", true).clicked() {
            app.actions.push(Action::Open(Page::Mods));
        }
        return;
    }
    if !app.settings.addons.active(Id::WaveLink) {
        ui.label("WaveLink is disabled.");
        button(
            app,
            ui,
            "Enable WaveLink",
            Command::Enable(Id::WaveLink, true),
            true,
        );
        return;
    }
    ui.add_space(16.0);
    let searching = app.addons.busy && app.addons.fetching_soundcloud;
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 105.0).max(100.0);
        let response = super::widgets::search_field(
            ui,
            &palette,
            app.locale,
            egui::Id::new("soundcloud-query"),
            &mut app.addons.query,
            "Track or artist on SoundCloud",
            width,
        );
        let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if app.addons.soundcloud_focus_requested {
            app.addons.soundcloud_focus_requested = false;
            response.request_focus();
        }
        let clicked = ui
            .add_enabled_ui(
                !app.addons.busy && !app.addons.query.trim().is_empty(),
                |ui| theme::soft_button(ui, &palette, Some(Icon::Search), "Search", true).clicked(),
            )
            .inner;
        if (submitted || clicked) && !app.addons.busy && !app.addons.query.trim().is_empty() {
            app.actions
                .push(Action::Addon(Command::Fetch(Request::SoundCloud(
                    app.addons.query.clone(),
                ))));
        }
    });
    if app.addons.busy {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(if searching {
                "Searching SoundCloud…"
            } else {
                "A playlist import is running…"
            });
            button(app, ui, "Cancel", Command::CancelFetch, false);
        });
    }
    if let Some(message) = &app.addons.soundcloud_message {
        ui.label(egui::RichText::new(message).color(palette.secondary));
    }
    ui.add_space(12.0);
    if let Some(collection) = app.addons.soundcloud_result.clone() {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "{} · {} tracks",
                collection.title,
                collection.tracks.len()
            ));
            if theme::soft_button(ui, &palette, Some(Icon::Play), "Play all", true).clicked() {
                app.actions.push(Action::Addon(Command::Play {
                    tracks: collection.tracks.clone(),
                    index: 0,
                }));
            }
            let saved = app
                .addons
                .collections
                .iter()
                .any(|c| c.source == collection.source);
            if !saved
                && theme::soft_button(ui, &palette, Some(Icon::Plus), "Save on Home", false)
                    .clicked()
            {
                app.actions.push(Action::Addon(Command::SaveSoundCloud));
            }
        });
        soundcloud_tracks(app, ui, &collection.tracks);
    } else if !searching {
        ui.label("Enter a track or artist above to search SoundCloud.");
    }
    ui.add_space(12.0);
    ui.small("Public, unprotected uploads only. Availability and audio quality depend on the upload. WaveLink by xoft122 (MIT).");
}

fn soundcloud_tracks(app: &mut App, ui: &mut egui::Ui, tracks: &[crate::player::LocalTrack]) {
    let palette = app.palette;
    let playing_uri = app.now_playing().filter(|now| now.local).map(|now| now.uri);
    let height = (ui.available_height() - 56.0).max(180.0);
    egui::ScrollArea::vertical()
        .id_salt("soundcloud-results")
        .max_height(height)
        .show_rows(ui, 60.0, tracks.len(), |ui, range| {
            for index in range {
                let track = &tracks[index];
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 54.0),
                    egui::Sense::click(),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!("Play {} · {}", track.title, track.artist_names()),
                    )
                });
                if response.hovered() {
                    ui.painter().rect_filled(rect, 6.0, palette.surface_hover);
                }
                let cover = egui::Rect::from_min_size(
                    rect.min + egui::vec2(8.0, 7.0),
                    egui::vec2(40.0, 40.0),
                );
                super::widgets::paint_cover(
                    ui,
                    &palette,
                    track.cover_url.as_deref().or(track.art_url.as_deref()),
                    cover,
                    4.0,
                    Icon::Disc,
                    Some(app.backend.art()),
                );
                let left = rect.left() + 62.0;
                let clip = egui::Rect::from_min_max(
                    egui::pos2(left, rect.top()),
                    egui::pos2(rect.right() - 66.0, rect.bottom()),
                );
                let painter = ui.painter().with_clip_rect(clip);
                let current = playing_uri.as_deref() == Some(track.uri.as_str());
                painter.text(
                    egui::pos2(left, rect.top() + 9.0),
                    egui::Align2::LEFT_TOP,
                    &track.title,
                    theme::medium(14.0),
                    if current {
                        palette.accent
                    } else {
                        palette.text
                    },
                );
                painter.text(
                    egui::pos2(left, rect.top() + 29.0),
                    egui::Align2::LEFT_TOP,
                    track.artist_names(),
                    theme::regular(12.0),
                    palette.secondary,
                );
                let seconds = track.duration_ms / 1000;
                ui.painter().text(
                    egui::pos2(rect.right() - 10.0, rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    if seconds == 0 {
                        "—".into()
                    } else {
                        format!("{}:{:02}", seconds / 60, seconds % 60)
                    },
                    theme::regular(13.0),
                    palette.secondary,
                );
                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Play from this track")
                    .clicked()
                {
                    app.actions.push(Action::Addon(Command::Play {
                        tracks: tracks.to_vec(),
                        index: index as u32,
                    }));
                }
            }
        });
}

fn collection_controls(
    ui: &mut egui::Ui,
    palette: &theme::Palette,
    actions: &mut Vec<Action>,
    collection: &addons::Collection,
    saved: Option<usize>,
) {
    ui.horizontal_wrapped(|ui| {
        if theme::soft_button(ui, palette, Some(Icon::Play), "Play all", true).clicked() {
            actions.push(Action::Addon(Command::Play {
                tracks: collection.tracks.clone(),
                index: 0,
            }));
        }
        let (label, command) = match saved {
            Some(index) => ("Remove collection", Command::DeleteCollection(index)),
            None => ("Save local collection", Command::SaveCollection),
        };
        if theme::soft_button(ui, palette, None, label, false).clicked() {
            actions.push(Action::Addon(command));
        }
    });
}
fn tracks(
    actions: &mut Vec<Action>,
    ui: &mut egui::Ui,
    tracks: &[crate::player::LocalTrack],
    id: &str,
) {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(260.0)
        .show_rows(ui, 32.0, tracks.len(), |ui, range| {
            for index in range {
                let track = &tracks[index];
                if ui
                    .add_sized(
                        [ui.available_width(), 30.0],
                        egui::Button::new(format!(
                            "{}   ·   {}",
                            track.title,
                            track.artist_names()
                        ))
                        .truncate(),
                    )
                    .on_hover_text("Play from this track")
                    .clicked()
                {
                    actions.push(Action::Addon(Command::Play {
                        tracks: tracks.to_vec(),
                        index: index as u32,
                    }));
                }
            }
        });
}

/// Isolated query/results; the provider API searches the entire public music catalogue.
pub fn juicevault(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    theme::text(ui, "JuiceVault search", theme::bold(28.0), palette.text);
    ui.label("Search the archive by song title. Exact and close matches appear first.");
    if !app.settings.addons.active(Id::JuiceVault) {
        button(
            app,
            ui,
            "Enable JuiceVault",
            Command::Enable(Id::JuiceVault, true),
            true,
        );
        return;
    }
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        let field = super::widgets::search_field(
            ui,
            &palette,
            app.locale,
            egui::Id::new("juicevault-query"),
            &mut app.addons.juice_query,
            "Song title on JuiceVault",
            (ui.available_width() - 110.0).max(100.0),
        );
        let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let click = ui
            .add_enabled_ui(!app.addons.busy, |ui| {
                theme::pill_button(ui, &palette, "Search", true).clicked()
            })
            .inner;
        if (enter || click) && !app.addons.busy && !app.addons.juice_query.trim().is_empty() {
            app.actions
                .push(Action::Addon(Command::Fetch(Request::JuiceVault(
                    app.addons.juice_query.clone(),
                ))));
        }
    });
    if app.addons.busy && app.addons.fetching_juice {
        ui.spinner();
        button(app, ui, "Cancel", Command::CancelFetch, false);
    }
    if let Some(message) = &app.addons.juice_message {
        ui.colored_label(palette.warning, message);
    }
    if let Some(collection) = app.addons.juice_result.clone() {
        ui.label(format!("{} close title matches", collection.tracks.len()));
        if collection.tracks.is_empty() {
            ui.small("No close title matches. Try another spelling or an alternate title.");
        }
        ui.push_id("juicevault-results", |ui| {
            soundcloud_tracks(app, ui, &collection.tracks)
        });
    }
}

/// Reload is explicit; closing the prompt always leaves playback running.
pub fn reload_prompt(app: &mut App, ctx: &egui::Context) {
    let Some(id) = app.addons.reload_notice else {
        return;
    };
    let response = egui::Modal::new(egui::Id::new("addon-reload")).show(ctx, |ui| {
        ui.set_max_width(380.0);
        ui.heading(format!("{} installed", id.name()));
        ui.add_space(8.0);
        ui.label("Restart Vibefy to reload your add-ons. Playback will stop; your accounts and layout will be kept.");
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            button(app, ui, "Reload now", Command::ReloadNow, true);
            button(app, ui, "Later", Command::ReloadLater, false);
        });
    });
    if response.should_close() {
        app.actions.push(Action::Addon(Command::ReloadLater));
    }
}
