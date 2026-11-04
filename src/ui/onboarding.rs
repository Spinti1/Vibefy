//! Ordered account setup and initial playlist transfer. Network work stays in the backend.
use crate::{
    api::models::pick_image,
    app::App,
    backend::Command,
    music_accounts::{Action as AccountAction, Provider},
    playlist_mirror::Action,
    theme::{self, Icon},
};
use egui::{Align, Align2, Layout, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use std::collections::BTreeSet;

pub fn required(app: &App, ctx: &egui::Context) -> bool {
    if !(app.is_connected() && app.user.is_some()) || !app.music_accounts[0].connected {
        return true;
    }
    let dismissed = ctx.data(|d| d.get_temp::<String>(egui::Id::new("setup-dismissed-pair")));
    !app.mirror.setup_complete && dismissed.as_deref() != Some(app.mirror.pair.as_str())
}

fn send(app: &mut App, action: Action) {
    app.backend.send(Command::Mirror {
        serial: app.mirror_serial,
        action,
    });
}

fn finish(app: &mut App, ui: &egui::Ui) {
    ui.ctx().data_mut(|d| {
        d.insert_temp(
            egui::Id::new("setup-dismissed-pair"),
            app.mirror.pair.clone(),
        )
    });
    app.mirror.setup_complete = true;
    send(app, Action::FinishSetup);
}

struct Choice {
    id: String,
    title: String,
    image: Option<String>,
    songs: usize,
}

fn choices(app: &App) -> Vec<Choice> {
    let mut result: Vec<_> = app
        .library
        .playlists
        .get()
        .into_iter()
        .flatten()
        .map(|p| {
            let saved = app.mirror.lists.iter().find(|l| l.spotify_id == p.id);
            Choice {
                id: p.id.clone(),
                title: p.name.clone(),
                image: pick_image(&p.images, 256)
                    .map(str::to_owned)
                    .or_else(|| saved.and_then(|l| l.collection.cover().map(str::to_owned))),
                songs: saved.map_or(p.track_total() as usize, |l| l.collection.tracks.len()),
            }
        })
        .collect();
    for list in &app.mirror.lists {
        if !result.iter().any(|p| p.id == list.spotify_id) {
            result.push(Choice {
                id: list.spotify_id.clone(),
                title: list.collection.title.clone(),
                image: list.collection.cover().map(str::to_owned),
                songs: list.collection.tracks.len(),
            });
        }
    }
    result
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(palette.window).inner_margin(24.0))
        .show(ui, |ui| {
            let width = ui.available_width().min(1020.0);
            let height = ui.available_height();
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                ui.allocate_ui_with_layout(
                    vec2(width, height),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.set_width(width);
                        if !app.music_accounts[0].connected {
                            account_step(app, ui);
                            return;
                        }
                        // Connected accounts become a compact step indicator, leaving room for the library.
                        ui.horizontal(|ui| {
                            let (logo, _) =
                                ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
                            theme::brand_logo(ui, logo.center(), 34.0);
                            theme::text(ui, "VIBEFY", theme::bold(15.0), palette.text);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                theme::text(
                                    ui,
                                    "03  Your library",
                                    theme::semibold(12.0),
                                    palette.accent,
                                );
                                if width > 540.0 {
                                    theme::text(
                                        ui,
                                        "02  YouTube connected",
                                        theme::regular(12.0),
                                        palette.secondary,
                                    );
                                    theme::text(
                                        ui,
                                        "01  Spotify connected",
                                        theme::regular(12.0),
                                        palette.secondary,
                                    );
                                }
                            });
                        });
                        ui.add_space(24.0);
                        if !app.mirror.ready {
                            ui.vertical_centered(|ui| {
                                ui.add_space(60.0);
                                theme::spinner(ui, 48.0, palette.accent);
                                ui.add_space(20.0);
                                theme::text(
                                    ui,
                                    "Preparing your library",
                                    theme::bold(26.0),
                                    palette.text,
                                );
                                ui.label(if app.mirror.message.is_empty() {
                                    "Loading your connected playlists…"
                                } else {
                                    &app.mirror.message
                                });
                            });
                            return;
                        }
                        let edit_id = egui::Id::new(("transfer-edit-selection", &app.mirror.pair));
                        let editing = ui
                            .ctx()
                            .data(|d| d.get_temp::<bool>(edit_id))
                            .unwrap_or(false);
                        let started = app.mirror.enabled && app.mirror.selected.is_some();
                        if started && !editing {
                            transfer(app, ui, edit_id);
                        } else {
                            selection(app, ui, edit_id, started);
                        }
                    },
                );
            });
        });
}

fn account_step(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.set_max_width(560.0);
            ui.add_space(32.0);
            let (logo, _) = ui.allocate_exact_size(Vec2::splat(88.0), Sense::hover());
            theme::brand_logo(ui, logo.center(), 88.0);
            theme::text(ui, "Connect your music", theme::bold(30.0), palette.text);
            ui.add_space(20.0);
            ui.add_enabled_ui(false, |ui| {
                super::login::big_button(ui, app, "Step 1 · Spotify connected");
            });
            ui.add_space(12.0);
            let busy = app.music_accounts[0].busy;
            ui.add_enabled_ui(!busy, |ui| {
                if super::login::big_button(ui, app, "Step 2 · Sign in with YouTube") {
                    super::music_accounts::send(
                        app,
                        Provider::YouTubeMusic,
                        AccountAction::Connect,
                    );
                }
            });
            ui.add_space(20.0);
            ui.label("Connect YouTube, then choose the Spotify playlists you want to copy.");
            ui.small("Sign in with Google in the window that opens.");
            if busy {
                ui.add_space(12.0);
                theme::spinner(ui, 24.0, palette.accent);
                ui.label("Waiting for YouTube sign-in…");
            }
            if let Some(error) = &app.music_accounts[0].message {
                ui.colored_label(palette.danger, error);
            }
        })
    });
}

fn selection(app: &mut App, ui: &mut egui::Ui, edit_id: egui::Id, started: bool) {
    let palette = app.palette;
    theme::text(
        ui,
        "Bring your playlists along",
        theme::bold(30.0),
        palette.text,
    );
    ui.add_space(5.0);
    ui.label(
        egui::RichText::new("Choose what to copy from Spotify to YouTube Music.")
            .color(palette.secondary),
    );
    ui.add_space(18.0);
    let all = choices(app);
    let selection_id = egui::Id::new(("transfer-selection", &app.mirror.pair));
    // Do not initialize an empty selection while the library is still loading.
    let mut selected: BTreeSet<String> = ui
        .ctx()
        .data(|d| d.get_temp(selection_id))
        .unwrap_or_else(|| {
            app.mirror
                .selected
                .clone()
                .unwrap_or_else(|| all.iter().map(|p| p.id.clone()).collect())
                .into_iter()
                .collect()
        });
    let filter_id = egui::Id::new(("transfer-filter", &app.mirror.pair));
    let mut filter: String = ui.ctx().data(|d| d.get_temp(filter_id)).unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut filter)
                .hint_text("Find a playlist")
                .desired_width(210.0),
        );
        if theme::pill_button(ui, &palette, "Select all", false).clicked() {
            selected = all.iter().map(|p| p.id.clone()).collect();
        }
        if theme::pill_button(ui, &palette, "Clear selection", false).clicked() {
            selected.clear();
        }
        ui.label(egui::RichText::new(format!("{} selected", selected.len())).color(palette.accent));
    });
    ui.add_space(14.0);
    let query = filter.trim().to_lowercase();
    let visible: Vec<_> = all
        .iter()
        .filter(|p| p.title.to_lowercase().contains(&query))
        .collect();
    let grid_height = (ui.available_height() - 125.0).max(110.0);
    egui::ScrollArea::vertical()
        .id_salt("setup-playlist-cards")
        .max_height(grid_height)
        .min_scrolled_height(grid_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if all.is_empty() {
                ui.spinner();
                ui.label("Loading Spotify playlists…");
            } else if visible.is_empty() {
                ui.label("No playlists match your search.");
            }
            let columns = ((ui.available_width() / 180.0).floor() as usize).clamp(1, 5);
            let gap = 14.0;
            let width = (ui.available_width() - gap * (columns - 1) as f32) / columns as f32;
            let height = width + 74.0;
            super::widgets::virtual_rows(
                ui,
                visible.len().div_ceil(columns),
                height + gap,
                |ui, row| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for p in visible.iter().skip(row * columns).take(columns) {
                            let checked = selected.contains(&p.id);
                            if playlist_card(app, ui, p, width, height, checked) {
                                if checked {
                                    selected.remove(&p.id);
                                } else {
                                    selected.insert(p.id.clone());
                                }
                            }
                        }
                    });
                    ui.add_space(gap);
                },
            );
        });
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    let songs: usize = all
        .iter()
        .filter(|p| selected.contains(&p.id))
        .map(|p| p.songs)
        .sum();
    ui.horizontal_wrapped(|ui| {
        ui.add_enabled_ui(!selected.is_empty(), |ui| {
            if theme::pill_button(
                ui,
                &palette,
                if started {
                    "Apply selection and resume"
                } else {
                    "Start transfer"
                },
                true,
            )
            .clicked()
            {
                let ids: Vec<_> = selected.iter().cloned().collect();
                send(app, Action::Configure(ids.clone()));
                app.mirror.selected = Some(ids);
                app.mirror.enabled = true;
                app.mirror.scan_finished = false;
                app.mirror.busy = true;
                ui.ctx().data_mut(|d| d.insert_temp(edit_id, false));
            }
        });
        ui.label(format!("{} playlists · {songs} songs", selected.len()));
        if started && theme::pill_button(ui, &palette, "Back to transfer", false).clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(edit_id, false));
        }
        if theme::pill_button(ui, &palette, "Skip for now — not recommended", false).clicked() {
            finish(app, ui);
        }
    });
    ui.add_space(5.0);
    ui.label(
        egui::RichText::new(
            "Copies are unlisted: anyone with the link can view them. Originals stay on Spotify.",
        )
        .size(12.0)
        .color(palette.secondary),
    );
    if !all.is_empty() {
        ui.ctx().data_mut(|d| d.insert_temp(selection_id, selected));
    }
    ui.ctx().data_mut(|d| d.insert_temp(filter_id, filter));
}

fn playlist_card(
    app: &App,
    ui: &mut egui::Ui,
    p: &Choice,
    width: f32,
    height: f32,
    selected: bool,
) -> bool {
    let palette = app.palette;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, selected, &p.title)
    });
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(
            rect,
            12.0,
            if response.hovered() {
                palette.surface_hover
            } else {
                palette.surface
            },
        );
        ui.painter().rect_stroke(
            rect,
            12.0,
            Stroke::new(
                if selected { 2.0 } else { 1.0 },
                if selected {
                    palette.accent
                } else {
                    palette.outline
                },
            ),
            StrokeKind::Inside,
        );
        let cover = Rect::from_min_size(rect.min + vec2(10.0, 10.0), Vec2::splat(width - 20.0));
        super::widgets::paint_cover(
            ui,
            &palette,
            p.image.as_deref(),
            cover,
            7.0,
            Icon::Music,
            Some(app.backend.art()),
        );
        let check = pos2(cover.right() - 15.0, cover.top() + 15.0);
        ui.painter().circle_filled(
            check,
            12.0,
            if selected {
                palette.accent
            } else {
                palette.panel
            },
        );
        if selected {
            ui.painter().add(egui::Shape::line(
                vec![
                    check + vec2(-5.0, 0.0),
                    check + vec2(-1.0, 4.0),
                    check + vec2(6.0, -4.0),
                ],
                Stroke::new(2.0, palette.on_accent),
            ));
        } else {
            ui.painter()
                .circle_stroke(check, 11.0, Stroke::new(1.0, palette.secondary));
        }
        let text_rect = Rect::from_min_max(
            pos2(rect.left() + 12.0, cover.bottom() + 12.0),
            rect.max - vec2(12.0, 8.0),
        );
        let painter = ui.painter().with_clip_rect(text_rect);
        let title = painter.layout(
            p.title.clone(),
            theme::semibold(14.0),
            palette.text,
            text_rect.width(),
        );
        let title_clip = Rect::from_min_size(text_rect.min, vec2(text_rect.width(), 36.0));
        painter
            .with_clip_rect(title_clip)
            .galley(text_rect.min, title, palette.text);
        painter.text(
            pos2(text_rect.left(), rect.bottom() - 17.0),
            Align2::LEFT_CENTER,
            if p.songs > 0 {
                format!("{} songs", p.songs)
            } else {
                "Spotify playlist".into()
            },
            theme::regular(12.0),
            palette.secondary,
        );
    }
    theme::focus_ring(ui, &response);
    let clicked = response.clicked();
    response
        .on_hover_text(&p.title)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked
}

fn transfer(app: &mut App, ui: &mut egui::Ui, edit_id: egui::Id) {
    let palette = app.palette;
    let active: Vec<_> = app
        .mirror
        .lists
        .iter()
        .filter(|p| {
            app.mirror
                .selected
                .as_ref()
                .is_some_and(|s| s.contains(&p.spotify_id))
        })
        .collect();
    let total: usize = active.iter().map(|p| p.collection.tracks.len()).sum();
    let copied: usize = active
        .iter()
        .map(|p| p.copied.min(p.collection.tracks.len()))
        .sum();
    let issues = active
        .iter()
        .filter(|p| p.issue.is_some() || p.uncertain)
        .count();
    let finished = app.mirror.scan_finished && !app.mirror.busy;
    let waiting = app.mirror.retry_at > crate::playlist_mirror::now();
    let progress = if total > 0 {
        copied as f32 / total as f32
    } else {
        0.0
    };
    let needs_review = issues > 0 || app.mirror.review_total > 0 || copied < total;
    ui.vertical_centered(|ui| {
        ui.add_space(8.0);
        let (ring, _) = ui.allocate_exact_size(Vec2::splat(116.0), Sense::hover());
        let center = ring.center();
        ui.painter()
            .circle_stroke(center, 50.0, Stroke::new(6.0, palette.surface_active));
        if progress > 0.0 {
            let points: Vec<_> = (0..=80)
                .map(|i| {
                    let angle = -std::f32::consts::FRAC_PI_2
                        + progress * std::f32::consts::TAU * i as f32 / 80.0;
                    center + vec2(angle.cos(), angle.sin()) * 50.0
                })
                .collect();
            ui.painter()
                .add(egui::Shape::line(points, Stroke::new(6.0, palette.accent)));
        }
        ui.painter().text(
            center - vec2(0.0, 7.0),
            Align2::CENTER_CENTER,
            format!("{:.0}%", progress * 100.0),
            theme::bold(27.0),
            palette.text,
        );
        ui.painter().text(
            center + vec2(0.0, 17.0),
            Align2::CENTER_CENTER,
            "copied",
            theme::regular(12.0),
            palette.secondary,
        );
        ui.add_space(12.0);
        theme::text(
            ui,
            if waiting {
                "Transfer paused"
            } else if finished && needs_review {
                "Your transfer needs a look"
            } else if finished {
                "Your playlists are ready"
            } else {
                "Bringing your music to YouTube"
            },
            theme::bold(28.0),
            palette.text,
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!(
                "{copied} of {total} songs confirmed on YouTube Music"
            ))
            .color(palette.secondary),
        );
        if waiting {
            ui.label("Waiting for the service cooldown. Transfer will resume automatically.");
        } else if !finished {
            ui.horizontal(|ui| {
                theme::spinner(ui, 18.0, palette.accent);
                ui.add(
                    egui::Label::new(if app.mirror.message.is_empty() {
                        "Finding matching recordings…"
                    } else {
                        &app.mirror.message
                    })
                    .truncate(),
                );
            });
        }
        if app.mirror.review_total > 0 {
            ui.colored_label(
                palette.warning,
                format!(
                    "{} songs could not be matched automatically",
                    app.mirror.review_total
                ),
            );
        }
    });
    ui.add_space(20.0);
    let list_height = (ui.available_height() - 105.0).max(90.0);
    egui::ScrollArea::vertical()
        .id_salt("transfer-progress-playlists")
        .max_height(list_height)
        .min_scrolled_height(list_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for p in &active {
                egui::Frame::new()
                    .fill(palette.surface)
                    .corner_radius(10)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_width((ui.available_width() - 2.0).max(0.0));
                        ui.horizontal(|ui| {
                            let (cover, _) =
                                ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
                            super::widgets::paint_cover(
                                ui,
                                &palette,
                                p.collection.cover(),
                                cover,
                                6.0,
                                Icon::Music,
                                Some(app.backend.art()),
                            );
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&p.collection.title).strong(),
                                    )
                                    .truncate(),
                                );
                                let count = p.collection.tracks.len();
                                let status = if p.uncertain || p.issue.is_some() {
                                    "Needs attention"
                                } else if p.copied >= count && count > 0 {
                                    "Copied"
                                } else {
                                    "In progress"
                                };
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {count} songs · {status}",
                                        p.copied.min(count)
                                    ))
                                    .size(12.0)
                                    .color(
                                        if p.issue.is_some() || p.uncertain {
                                            palette.warning
                                        } else {
                                            palette.secondary
                                        },
                                    ),
                                );
                            });
                        });
                    });
                ui.add_space(8.0);
            }
            if active.is_empty() {
                ui.label("Preparing selected playlists…");
            }
        });
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        if finished && theme::pill_button(ui, &palette, "Continue to Home", true).clicked() {
            finish(app, ui);
        }
        if theme::pill_button(ui, &palette, "Change selection", false).clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(edit_id, true));
        }
        if !finished
            && theme::pill_button(ui, &palette, "Skip for now — not recommended", false).clicked()
        {
            finish(app, ui);
        }
    });
    ui.add_space(5.0);
    ui.label(
        egui::RichText::new("You can leave this screen. Transfer continues in the background.")
            .size(12.0)
            .color(palette.secondary),
    );
}
