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
pub(super) fn audio(app: &mut App, ctx: &egui::Context) {
    let time = ctx.input(|i| i.time);
    if time == app.addons.audio_time {
        return;
    }
    let elapsed = (time - app.addons.audio_time).clamp(0.0, 0.2) as f32;
    app.addons.audio_time = time;
    let now = app.now_playing();
    let sounding = now
        .as_ref()
        .is_some_and(|n| n.local && n.playing && !n.loading);
    let track = now.map(|n| n.uri);
    if app.addons.visual_track != track {
        app.addons.visual_track = track;
        app.addons.ridges.clear();
        app.addons.bands.clear();
        app.addons.waveform.clear();
        app.addons.beat = Default::default();
        app.addons.analyser = Default::default();
    }
    // Freeze the last real audio frame when paused, including ridgeline history.
    if !sounding {
        return;
    }
    let mut samples = [0.0; crate::vis::WIDE_SAMPLES];
    app.winamp.tap.window_into(&mut samples, crate::vis::LAG);
    let mut levels = app
        .addons
        .analyser
        .step(&samples, std::time::Instant::now());
    let config = app
        .settings
        .addons
        .visual(app.settings.addons.visualizer.unwrap_or(Id::Vivy));
    let blend = if config.smoothing == 0. {
        1.
    } else {
        1. - (-elapsed * (40. - config.smoothing * 36.)).exp()
    };
    for (target, previous) in levels.iter_mut().zip(&app.addons.bands) {
        *target = *previous + (*target - *previous) * blend;
    }
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
    app.addons.update_visual_levels(&levels, keep_history);
}

pub fn fullscreen(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    audio(app, &ctx);
    if let Some(uri) = &app.spicy.background_pick {
        let mut appearance = app.settings.visualizer_background.clone();
        appearance.custom_image = Some(uri.clone());
        appearance.background = crate::spicy_lyrics::Background::CustomImage;
        app.actions.push(Action::SpicyBackgroundUsed);
        app.actions
            .push(Action::SetVisualizerBackground(appearance));
    }
    if let Some(now) = app.now_playing()
        && let Some(url) = now.art_small.or(now.art_url)
    {
        app.tint_for(Some(&url));
    }
    let mode = app.settings.addons.visualizer.unwrap_or(Id::Vivy);
    let config = app.settings.addons.visual(mode);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::from_rgb(7, 10, 19)))
        .show(ui, |ui| {
            if !ctx.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
                super::topbar::show(app, ui);
            }
            let rect = ui.available_rect_before_wrap();
            let appearance = app.settings.visualizer_background.clone();
            super::music_background::paint(app, ui, rect, &appearance);
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
        .width((ui.available_width() - 210.).clamp(110., 240.))
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
    let ctx = ui.ctx().clone();
    let id = egui::Id::new("visualizer-customize");
    let mut open = ctx.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(12.0);
        if let (_, Some(effect)) = effect_selector(ui, &app.settings.addons) {
            app.actions.push(Action::Addon(Command::ChooseEffect {
                id: effect.id,
                style: effect.style,
            }));
        }
        if ui.button("Customize").clicked() {
            open = !open;
        }
    });
    let mut tab = ctx.data(|d| d.get_temp::<bool>(id.with("background")).unwrap_or(false));
    if open {
        egui::Window::new("Visualizer appearance")
            .id(id.with("window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .default_pos(egui::pos2(24.0, 120.0))
            .max_width((ctx.content_rect().width() - 40.0).max(260.0))
            .max_height((ctx.content_rect().height() - 200.0).clamp(240.0, 600.0))
            .vscroll(true)
            .show(&ctx, |ui| {
                *ui.visuals_mut() = egui::Visuals::dark();
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut tab, false, "Visualizer");
                    ui.selectable_value(&mut tab, true, "Background");
                });
                ui.separator();
                if tab {
                    let mut appearance = app.settings.visualizer_background.clone();
                    super::spicy_lyrics::background_settings(app, ui, &mut appearance, false);
                    let mut name =
                        ctx.data(|d| d.get_temp::<String>(id.with("name")).unwrap_or_default());
                    super::spicy_lyrics::saved_styles(app, ui, &mut appearance, &mut name);
                    ctx.data_mut(|d| d.insert_temp(id.with("name"), name));
                    if appearance != app.settings.visualizer_background {
                        app.actions
                            .push(Action::SetVisualizerBackground(appearance));
                    }
                } else {
                    let original = app.settings.addons.visual(mode);
                    let mut config = original;
                    ui.add(
                        egui::Slider::new(&mut config.intensity, 0.2..=3.0)
                            .text("Strength")
                            .show_value(false),
                    );
                    ui.add(
                        egui::Slider::new(&mut config.speed, 0.25..=2.0)
                            .text("Speed")
                            .show_value(false),
                    );
                    ui.checkbox(&mut config.artwork_colors, "Match cover colors");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut config.show_title, "Song title");
                        ui.checkbox(&mut config.show_cover, "Cover");
                    });
                    if config.show_cover {
                        ui.add(
                            egui::Slider::new(&mut config.cover_scale, 0.5..=1.6)
                                .text("Cover size")
                                .show_value(false),
                        );
                    }
                    if super::spicy_lyrics::details_toggle(ui, "visual-details", "More options") {
                        ui.add(
                            egui::Slider::new(&mut config.brightness, 0.2..=1.0)
                                .text("Brightness")
                                .show_value(false),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.smoothing, 0.0..=1.0)
                                .text("Smoothness")
                                .show_value(false),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.detail, 24..=96)
                                .text("Detail")
                                .show_value(false),
                        );
                        ui.add(egui::Slider::new(&mut config.fps, 15..=60).text("FPS"))
                            .on_hover_text("Lower values reduce graphics work.");
                        if ui.small_button("Reset visualizer").clicked() {
                            config = crate::addons::VisualConfig {
                                style: config.style,
                                ..Default::default()
                            };
                        }
                    }
                    if config != original {
                        app.actions
                            .push(Action::Addon(Command::ConfigureVisual(config)));
                    }
                }
            });
    }
    ctx.data_mut(|d| {
        d.insert_temp(id, open);
        d.insert_temp(id.with("background"), tab);
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
            let mut output = ctx.run_ui(
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
            );
            // This headless UI test has no renderer to upload font textures to.
            output.textures_delta.clear();
            output
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
