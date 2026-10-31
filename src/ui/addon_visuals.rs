//! Lightweight native animations using the existing post-EQ, pre-volume audio tap.
use crate::addons::{Command, Id};
use crate::app::App;
use crate::model::Action;
use crate::theme::{self, Icon};
use egui::{Color32, Pos2, Rect, Stroke, Vec2, pos2, vec2};

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
    app.addons.bands = levels.to_vec();
    if sounding {
        app.addons.ridges.push_front(levels.to_vec());
        app.addons.ridges.truncate(24);
    }
}

pub fn fullscreen(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    audio(app, &ctx);
    let palette = app.palette;
    let mode = app.settings.addons.visualizer.unwrap_or(Id::Vivy);
    let config = app.settings.addons.visual(mode);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::from_rgb(7, 10, 19)))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter().with_clip_rect(rect);
            let tint = app.now_playing_tint().unwrap_or(palette.accent);
            let intensity = config.intensity;
            let time = app
                .now_playing()
                .map_or(0.0, |n| n.position_ms as f32 / 1000.0);
            let levels = &app.addons.bands;
            if mode == Id::Vivy {
                for (row, band) in app.addons.ridges.iter().enumerate().rev() {
                    let depth = row as f32 / 24.0;
                    let width = rect.width() * (0.9 - depth * 0.45);
                    let base = rect.top() + rect.height() * (0.72 - depth * 0.52);
                    let points: Vec<_> = band
                        .iter()
                        .enumerate()
                        .map(|(i, value)| {
                            let x = rect.center().x - width / 2.0
                                + width * i as f32 / (band.len() - 1).max(1) as f32;
                            let drift = if config.style == 1 {
                                (i as f32 * 0.15 + time * 0.3 + depth * 4.0).sin()
                                    * rect.height()
                                    * 0.05
                            } else {
                                0.0
                            };
                            pos2(
                                x,
                                base - value * intensity * rect.height() * 0.16 * (1.0 - depth)
                                    + drift,
                            )
                        })
                        .collect();
                    painter.add(egui::Shape::line(
                        points,
                        Stroke::new(
                            if row == 0 { 2.5 } else { 1.0 },
                            tint.gamma_multiply(0.95 - depth * 0.7),
                        ),
                    ));
                }
            } else {
                let radius = rect.width().min(rect.height()) * 0.22;
                let center = rect.center() - vec2(0.0, 20.0);
                if config.style == 0 {
                    for ring in 0..5 {
                        let points: Vec<_> = (0..=128)
                            .map(|i| {
                                let angle = i as f32 / 128.0 * std::f32::consts::TAU;
                                let band =
                                    levels.get(i % levels.len().max(1)).copied().unwrap_or(0.0);
                                let r =
                                    radius * (1.0 + ring as f32 * 0.13) + band * intensity * 32.0;
                                center + vec2(angle.cos(), angle.sin()) * r
                            })
                            .collect();
                        painter.add(egui::Shape::line(
                            points,
                            Stroke::new(1.5, tint.gamma_multiply(0.8 - ring as f32 * 0.12)),
                        ));
                    }
                } else {
                    let width = rect.width() * 0.8;
                    let step = width / levels.len().max(1) as f32;
                    for (i, level) in levels.iter().enumerate() {
                        let x = center.x - width / 2.0 + i as f32 * step;
                        let height = level * intensity * rect.height() * 0.3;
                        painter.rect_filled(
                            Rect::from_center_size(
                                pos2(x, center.y),
                                vec2((step - 3.0).max(1.0), height.max(2.0)),
                            ),
                            2.0,
                            tint.gamma_multiply(0.8),
                        );
                    }
                }
                if let Some(now) = app.now_playing() {
                    let cover = Rect::from_center_size(center, Vec2::splat(radius * 1.15));
                    super::widgets::paint_cover(
                        ui,
                        &palette,
                        now.art_url.as_deref(),
                        cover,
                        12.0,
                        Icon::Music,
                        Some(app.backend.art()),
                    );
                }
                if config.particles {
                    for index in 0..48 {
                        let seed = index as f32;
                        let x = rect.left() + ((seed * 0.618 + time * 0.008) % 1.0) * rect.width();
                        let y = rect.top()
                            + ((seed * 0.431 - time * 0.012).rem_euclid(1.0)) * rect.height();
                        painter.circle_filled(
                            pos2(x, y),
                            1.0 + app.addons.beat.pulse * 1.5,
                            tint.gamma_multiply(0.5),
                        );
                    }
                }
            }
            fullscreen_controls(app, ui, mode);
            if !playing(app) {
                ui.label(
                    egui::RichText::new("Play audio on this computer to animate the visualizer.")
                        .color(Color32::from_gray(180)),
                );
            }
            if let Some(now) = app.now_playing() {
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
            } else {
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
        theme::visualiser_frame(&ctx, app.visualiser_fps().map(|v| v.min(30)));
    }
}

fn fullscreen_controls(app: &mut App, ui: &mut egui::Ui, mode: Id) {
    ui.add_space(10.0);
    ui.horizontal_top(|ui| {
        ui.add_space(10.0);
        ui.scope(|ui| {
            ui.set_max_width((ui.available_width() - 10.0).min(440.0));
            *ui.visuals_mut() = egui::Visuals::dark();
            let palette = crate::theme::Palette::dark();
            egui::Frame::new()
                .fill(Color32::from_black_alpha(215))
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            egui::ComboBox::from_label("Visualizer")
                                .selected_text(mode.name())
                                .show_ui(ui, |ui| {
                                    for id in app.settings.addons.installed_visualizers() {
                                        if ui.selectable_label(mode == id, id.name()).clicked() {
                                            app.actions
                                                .push(Action::Addon(Command::ChooseVisualizer(id)));
                                        }
                                    }
                                });
                            if theme::soft_button(
                                ui,
                                &palette,
                                Some(Icon::Shrink),
                                "Exit fullscreen",
                                false,
                            )
                            .clicked()
                            {
                                app.actions.push(Action::Addon(Command::Fullscreen));
                            }
                        });
                        egui::CollapsingHeader::new("Visualizer settings")
                            .default_open(true)
                            .show(ui, |ui| {
                                let config = app.settings.addons.visual(mode);
                                ui.horizontal_wrapped(|ui| {
                                    for (style, label) in if mode == Id::Vivy {
                                        [(0, "Ridgeline"), (1, "Drift")]
                                    } else {
                                        [(0, "Rings"), (1, "Spectrum")]
                                    } {
                                        if ui
                                            .selectable_label(config.style == style, label)
                                            .clicked()
                                        {
                                            app.actions.push(Action::Addon(Command::Style(style)));
                                        }
                                    }
                                });
                                let mut intensity = config.intensity;
                                if ui
                                    .add(
                                        egui::Slider::new(&mut intensity, 0.2..=3.0)
                                            .text("Intensity"),
                                    )
                                    .changed()
                                {
                                    app.actions
                                        .push(Action::Addon(Command::Intensity(intensity)));
                                }
                                if mode == Id::Backmusic {
                                    let mut particles = config.particles;
                                    if ui.checkbox(&mut particles, "Particles").changed() {
                                        app.actions
                                            .push(Action::Addon(Command::Particles(particles)));
                                    }
                                }
                            });
                    });
                });
        });
    });
}

#[derive(Clone)]
struct Neko {
    position: Pos2,
    last: f64,
    idle_since: f64,
}
pub fn overlays(app: &mut App, ui: &egui::Ui) {
    let ctx = ui.ctx();
    let screen = ui.ctx().content_rect();
    if app.settings.addons.active(Id::Oneko) {
        let id = egui::Id::new("vibefy-oneko");
        let time = ctx.input(|i| i.time);
        let mut cat = ctx.data(|d| d.get_temp::<Neko>(id)).unwrap_or(Neko {
            position: pos2(48.0, 64.0),
            last: time,
            idle_since: time,
        });
        let delta = ctx
            .pointer_latest_pos()
            .map_or(Vec2::ZERO, |p| p - cat.position);
        let distance = delta.length();
        let moving = distance > 24.0;
        let frame = (time * 8.0) as usize % 2;
        let (column, row) = if moving {
            let step = ((time - cat.last).clamp(0.0, 0.1) as f32 * 130.0).min(distance - 20.0);
            cat.position += delta.normalized() * step;
            cat.idle_since = time;
            if delta.x.abs() > delta.y.abs() * 2.0 {
                if delta.x > 0.0 {
                    (3, frame)
                } else {
                    (4, 2 + frame)
                }
            } else if delta.y.abs() > delta.x.abs() * 2.0 {
                if delta.y > 0.0 {
                    if frame == 0 { (6, 3) } else { (7, 2) }
                } else {
                    (1, 2 + frame)
                }
            } else if delta.x > 0.0 {
                if delta.y > 0.0 {
                    if frame == 0 { (5, 1) } else { (5, 2) }
                } else {
                    (0, 2 + frame)
                }
            } else if delta.y > 0.0 {
                if frame == 0 { (5, 3) } else { (6, 1) }
            } else {
                (1, frame)
            }
        } else if time - cat.idle_since > 4.0 {
            (2, 0)
        } else {
            (3, 3)
        };
        cat.position.x = cat
            .position
            .x
            .clamp(screen.left() + 16.0, screen.right() - 16.0);
        cat.position.y = cat
            .position
            .y
            .clamp(screen.top() + 16.0, screen.bottom() - 16.0);
        cat.last = time;
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, id));
        let mut overlay = egui::Ui::new(
            ctx.clone(),
            id,
            egui::UiBuilder::new()
                .layer_id(painter.layer_id())
                .max_rect(screen),
        );
        egui::Image::from_bytes(
            "bytes://vibefy-oneko.png",
            include_bytes!("../../assets/mods/oneko.png"),
        )
        .texture_options(egui::TextureOptions::NEAREST)
        .uv(Rect::from_min_max(
            pos2(column as f32 / 8.0, row as f32 / 4.0),
            pos2((column + 1) as f32 / 8.0, (row + 1) as f32 / 4.0),
        ))
        .paint_at(
            &mut overlay,
            Rect::from_center_size(cat.position, Vec2::splat(32.0)),
        );
        ctx.data_mut(|d| d.insert_temp(id, cat));
        if moving {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        } else if time - ctx.data(|d| d.get_temp::<Neko>(id)).unwrap().idle_since < 4.1 {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
    }
    if app.settings.addons.active(Id::CatJam) {
        let id = egui::Id::new("vibefy-cat-jam-original");
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, id));
        let overlay = egui::Ui::new(
            ctx.clone(),
            id,
            egui::UiBuilder::new()
                .layer_id(painter.layer_id())
                .max_rect(screen),
        );
        let position = app.now_playing().map_or(0, |now| now.position_ms);
        let frame = crate::mod_animation::cat_frame(position, app.addons.tempo_bpm);
        let rect =
            crate::mod_animation::cat_rect(ctx.viewport_rect(), app.settings.addons.cat_scale());
        egui::Image::from_bytes(
            "bytes://vibefy-cat-jam-original.png",
            crate::mod_animation::CAT_JAM,
        )
        .uv(crate::mod_animation::cat_uv(frame))
        .paint_at(&overlay, rect);
        if app
            .now_playing()
            .is_some_and(|now| now.playing && !now.loading)
        {
            theme::visualiser_frame(ctx, app.visualiser_fps().map(|fps| fps.min(30)));
        }
    }
}

pub fn progress(app: &App, ui: &egui::Ui, rect: Rect, fraction: f32) {
    if !app.settings.addons.active(Id::Nyan) {
        return;
    }
    let x = rect.left() + rect.width() * fraction.clamp(0.0, 1.0);
    let colors = [
        Color32::RED,
        Color32::from_rgb(255, 153, 0),
        Color32::YELLOW,
        Color32::GREEN,
        Color32::from_rgb(0, 153, 255),
        Color32::from_rgb(102, 51, 255),
    ];
    for (i, color) in colors.iter().enumerate() {
        ui.painter().rect_filled(
            Rect::from_min_max(
                pos2(rect.left(), rect.center().y - 3.0 + i as f32),
                pos2(x, rect.center().y - 2.0 + i as f32),
            ),
            0.0,
            *color,
        );
    }
    let position = app.now_playing().map_or(0, |now| now.position_ms);
    let frame = crate::mod_animation::nyan_frame(position, app.addons.tempo_bpm);
    let center = pos2(
        x.clamp(rect.left() + 17.0, rect.right() - 17.0),
        rect.center().y,
    );
    egui::Image::from_bytes(
        format!("bytes://vibefy-original-nyan-{frame}.png"),
        crate::mod_animation::NYAN_FRAMES[frame],
    )
    .texture_options(egui::TextureOptions::NEAREST)
    .paint_at(ui, Rect::from_center_size(center, vec2(34.0, 21.0)));
    if app
        .now_playing()
        .is_some_and(|now| now.playing && !now.loading)
    {
        theme::visualiser_frame(ui.ctx(), app.visualiser_fps().map(|fps| fps.min(30)));
    }
}
