//! Lightweight native animations using the existing post-EQ, pre-volume audio tap.
use crate::addons::{Command, Id};
use crate::app::App;
use crate::model::Action;
use crate::theme;
use egui::{Color32, Rect, pos2};

fn playing(app: &App) -> bool {
    app.now_playing()
        .is_some_and(|n| n.local && n.playing && !n.loading)
}
fn audio(app: &mut App, ctx: &egui::Context) {
    let time = ctx.input(|i| i.time);
    if time == app.addons.audio_time {
        return;
    }
    let elapsed = (time - app.addons.audio_time).clamp(0.0, 0.2) as f32;
    app.addons.audio_time = time;
    let sounding = playing(app);
    let samples = if sounding {
        app.winamp
            .tap
            .window(crate::vis::WIDE_SAMPLES, crate::vis::LAG)
    } else {
        Vec::new()
    };
    let levels = app
        .addons
        .analyser
        .step(&samples, std::time::Instant::now());
    let bass = levels.iter().take(10).copied().sum::<f32>() / 10.0;
    app.addons
        .beat
        .step(if sounding { bass } else { 0.0 }, time, elapsed);
    // Only Vivy draws history; Backmusic needs current bands alone.
    let keep_history = sounding && app.settings.addons.visualizer.unwrap_or(Id::Vivy) == Id::Vivy;
    app.addons.update_visual_levels(&levels, keep_history);
}

pub fn fullscreen(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    audio(app, &ctx);
    let mode = app.settings.addons.visualizer.unwrap_or(Id::Vivy);
    let config = app.settings.addons.visual(mode);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::from_rgb(7, 10, 19)))
        .show(ui, |ui| {
            if !ctx.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
                super::topbar::show(app, ui);
            }
            let rect = ui.available_rect_before_wrap();
            let painter = ui.painter().with_clip_rect(rect);
            super::mod_host::render(app, ui, mode, rect, &[]);
            fullscreen_controls(app, ui, mode);
            if !playing(app) {
                ui.label(
                    egui::RichText::new("Play audio on this computer to animate the visualizer.")
                        .color(Color32::from_gray(180)),
                );
            }
            if config.show_title
                && let Some(now) = app.now_playing()
            {
                painter.text(
                    pos2(rect.center().x, rect.bottom() - 62.0),
                    egui::Align2::CENTER_CENTER,
                    &now.title,
                    theme::bold(24.0),
                    Color32::WHITE,
                );
                painter.text(
                    pos2(rect.center().x, rect.bottom() - 30.0),
                    egui::Align2::CENTER_CENTER,
                    crate::api::models::join_names(now.artists.iter().map(|a| a.name.as_str())),
                    theme::regular(15.0),
                    Color32::from_gray(180),
                );
            } else if app.now_playing().is_none() {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Play a track to start the visualizer",
                    theme::regular(20.0),
                    Color32::WHITE,
                );
            }
        });
    if playing(app) {
        theme::visualiser_frame(
            &ctx,
            Some(
                app.visualiser_fps()
                    .unwrap_or(60)
                    .min(config.fps.clamp(15, 60) as u32),
            ),
        );
    }
}

fn fullscreen_controls(app: &mut App, ui: &mut egui::Ui, mode: Id) {
    ui.add_space(theme::top_bar_height() + 8.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        egui::Frame::new()
            .fill(Color32::from_black_alpha(215))
            .corner_radius(10.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                *ui.visuals_mut() = egui::Visuals::dark();
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("fullscreen-view")
                        .selected_text(mode.name())
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for id in app.settings.addons.installed_visualizers() {
                                if ui.selectable_label(mode == id, id.name()).clicked() {
                                    app.actions
                                        .push(Action::Addon(Command::ChooseVisualizer(id)));
                                }
                            }
                        });
                    ui.menu_button("Settings", |ui| {
                        ui.set_max_width(300.0);
                        let original = app.settings.addons.visual(mode);
                        let mut config = original;
                        ui.strong(mode.name());
                        ui.horizontal(|ui| {
                            for (style, label) in if mode == Id::Vivy {
                                [(0, "Ridgeline"), (1, "Drift")]
                            } else {
                                [(0, "Rings"), (1, "Spectrum")]
                            } {
                                ui.selectable_value(&mut config.style, style, label);
                            }
                        });
                        ui.add(
                            egui::Slider::new(&mut config.intensity, 0.2..=3.0).text("Intensity"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.brightness, 0.2..=1.0).text("Brightness"),
                        );
                        ui.add(egui::Slider::new(&mut config.fps, 15..=60).text("Animation FPS"));
                        ui.small("Lower FPS reduces graphics work.");
                        ui.checkbox(&mut config.show_title, "Song title and artist");
                        if mode == Id::Backmusic {
                            ui.checkbox(&mut config.show_cover, "Album cover");
                            ui.checkbox(&mut config.particles, "Particles");
                        }
                        ui.separator();
                        if ui.button("Restore defaults").clicked() {
                            config = Default::default();
                        }
                        if config != original {
                            app.actions
                                .push(Action::Addon(Command::ConfigureVisual(config)));
                        }
                    });
                });
            });
    });
}

pub fn overlays(app: &mut App, ui: &egui::Ui) {
    let ctx = ui.ctx();
    let screen = ctx.viewport_rect();
    for id in [Id::Oneko, Id::CatJam] {
        if !app.settings.addons.active(id) || !app.addons.modules.contains_key(&id) {
            continue;
        }
        let overlay = egui::Ui::new(
            ctx.clone(),
            egui::Id::new(("mod-overlay", id)),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new(("mod-layer", id)),
                ))
                .max_rect(screen),
        );
        super::mod_host::render(app, &overlay, id, screen, &[]);
    }
}
pub fn progress(app: &App, ui: &egui::Ui, rect: Rect, fraction: f32) {
    super::mod_host::render(app, ui, Id::Nyan, rect, &[(11, fraction.clamp(0., 1.))]);
}
