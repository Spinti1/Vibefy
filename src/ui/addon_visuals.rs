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
    app.addons.waveform.clear();
    for i in 0..256 {
        app.addons.waveform.push(
            samples
                .get(i * samples.len().saturating_sub(1) / 255)
                .copied()
                .unwrap_or(0.),
        );
    }
    // Capture history at 30 Hz rather than once per UI repaint.
    let keep_history = sounding
        && matches!(
            app.settings.addons.visualizer.unwrap_or(Id::Vivy),
            Id::Vivy | Id::SoundVisualizer
        )
        && time - app.addons.history_time >= 1. / 30.;
    if keep_history {
        app.addons.history_time = time;
    }
    if !sounding {
        app.addons.ridges.clear();
    }
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

fn effect_selector(
    ui: &mut egui::Ui,
    preferences: &crate::addons::Preferences,
) -> (egui::Response, Option<crate::visualizers::Effect>) {
    let selected = crate::visualizers::selected(preferences);
    let mut choice = None;
    let response = egui::ComboBox::from_id_salt("builtin-visualizer-effect")
        .selected_text(selected.label)
        .width((ui.available_width() - 100.).clamp(110., 240.))
        .height(420.)
        .show_ui(ui, |ui| {
            for effect in crate::visualizers::EFFECTS {
                if ui
                    .selectable_label(effect == selected, effect.label)
                    .clicked()
                {
                    choice = Some(effect);
                }
            }
        })
        .response;
    (response, choice)
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
                    if let (_, Some(effect)) = effect_selector(ui, &app.settings.addons) {
                        app.actions.push(Action::Addon(Command::ChooseEffect {
                            id: effect.id,
                            style: effect.style,
                        }));
                    }
                    ui.menu_button("Settings", |ui| {
                        ui.set_max_width(300.0);
                        let original = app.settings.addons.visual(mode);
                        let mut config = original;
                        ui.strong(mode.name());
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
                            config = crate::addons::VisualConfig {
                                style: config.style,
                                ..Default::default()
                            };
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

#[cfg(test)]
mod release_update_tests {
    use super::*;
    #[test]
    fn builtin_visualizer_arrow_opens_all_effects_and_click_selects_an_effect() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut p = crate::addons::Preferences::default();
        let mut selector = Rect::NOTHING;
        let mut frame = |events: Vec<egui::Event>| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800., 800.),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let (response, effect) = effect_selector(ui, &p);
                    selector = response.rect;
                    if let Some(effect) = effect {
                        assert!(crate::visualizers::select(&mut p, effect.id, effect.style));
                    }
                },
            )
        };
        frame(vec![]);
        // Capture the real arrow bounds in a separate draw; all clicks go through egui.
        let output = frame(vec![]);
        let tree = output.platform_output.accesskit_update.unwrap();
        let combo = tree
            .nodes
            .iter()
            .find_map(|(_, n)| {
                (n.role() == egui::accesskit::Role::ComboBox).then(|| n.bounds().unwrap())
            })
            .expect("main effect selector");
        let arrow = pos2(combo.x1 as f32 - 8., (combo.y0 + combo.y1) as f32 / 2.);
        let click = |pos| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        frame(click(arrow));
        let output = frame(vec![]);
        let tree = output.platform_output.accesskit_update.unwrap();
        for effect in crate::visualizers::EFFECTS {
            assert!(
                tree.nodes
                    .iter()
                    .any(|(_, n)| n.value().or(n.label()) == Some(effect.label)),
                "{} must be in the main list",
                effect.label
            );
        }
        let target = tree
            .nodes
            .iter()
            .find_map(|(_, n)| {
                (n.value().or(n.label()) == Some("SoundVisualizer · Arcs"))
                    .then(|| n.bounds().unwrap())
            })
            .unwrap();
        frame(click(pos2(
            (target.x0 + target.x1) as f32 / 2.,
            (target.y0 + target.y1) as f32 / 2.,
        )));
        drop(frame);
        assert_eq!(p.visualizer, Some(Id::SoundVisualizer));
        assert_eq!(p.visual(Id::SoundVisualizer).style, 11);
    }
}
