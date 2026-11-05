//! Shared painter for built-in visualizers and optional Wasm drawing commands.
use crate::{
    addons::Id,
    app::App,
    theme::{self, Icon},
};
use egui::{Color32, Rect, Stroke, Vec2, pos2, vec2};
pub fn render(app: &App, ui: &egui::Ui, id: Id, rect: Rect, extra: &[(usize, f32)]) {
    let visual = id.is_visualizer();
    if !visual && !app.settings.addons.active(id) {
        return;
    }
    let guest = app.addons.modules.get(&id);
    if !visual && guest.is_none() {
        return;
    }
    let config = app.settings.addons.visual(id);
    let mut input = vec![0.; if visual { 512 + 24 * 128 } else { 32 }];
    input[..4].copy_from_slice(&[rect.left(), rect.top(), rect.width(), rect.height()]);
    if let Some(now) = app.now_playing() {
        input[4] = now.position_ms as f32 / 1000.;
        input[6] = if now.playing && !now.loading { 1. } else { 0. };
    }
    input[5] = ui.ctx().input(|i| i.time) as f32;
    input[7] = app
        .addons
        .tempo_bpm
        .filter(|v| v.is_finite() && *v > 0.)
        .unwrap_or(if id == Id::CatJam { 135.48 } else { 116. });
    let pointer = ui.ctx().pointer_latest_pos().unwrap_or(pos2(48., 64.));
    input[8] = pointer.x;
    input[9] = pointer.y;
    input[10] = app.settings.addons.cat_scale();
    input[12] = config.intensity;
    input[13] = config.style as f32;
    input[15] = if config.particles { 1. } else { 0. };
    input[16] = if config.show_cover { 1. } else { 0. };
    input[17] = app.addons.beat.pulse;
    input[18] = app.addons.bands.len().min(128) as f32;
    input[19] = app.addons.ridges.len().min(24) as f32;
    input[25] = theme::PLAYER_BAR_HEIGHT;
    input[26] = config.fps as f32;
    if visual {
        for (i, v) in app.addons.bands.iter().take(128).enumerate() {
            input[64 + i] = *v;
        }
        for (i, sample) in app.addons.waveform.iter().take(256).enumerate() {
            input[3328 + i] = *sample;
        }
        for (r, row) in app.addons.ridges.iter().take(24).enumerate() {
            for (i, v) in row.iter().take(128).enumerate() {
                input[256 + r * 128 + i] = *v;
            }
        }
    }
    for &(i, v) in extra {
        if i < input.len() {
            input[i] = v;
        }
    }
    let commands = if visual {
        crate::visualizers::render(id, &input)
    } else {
        let Ok(commands) = guest.unwrap().render(input) else {
            return;
        };
        commands
    };
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
    let tint = app
        .now_playing_tint()
        .unwrap_or(app.palette.accent)
        .gamma_multiply(config.brightness.clamp(0.2, 1.));
    let mut points = Vec::new();
    let mut stroke = Stroke::new(1., tint);
    for cmd in commands {
        let [a, b, c, d, e, f, g, _] = cmd.values;
        match cmd.kind {
            0 => {
                points.clear();
                stroke = Stroke::new(a.clamp(0.1, 10.), tint.gamma_multiply(b.clamp(0., 1.)));
            }
            1 => points.push(pos2(a, b)),
            2 => {
                if points.len() > 1 {
                    painter.add(egui::Shape::line(std::mem::take(&mut points), stroke));
                }
            }
            3 => {
                let color = if g >= 0. {
                    [
                        Color32::RED,
                        Color32::from_rgb(255, 153, 0),
                        Color32::YELLOW,
                        Color32::GREEN,
                        Color32::from_rgb(0, 153, 255),
                        Color32::from_rgb(102, 51, 255),
                    ][(g as usize).min(5)]
                } else {
                    tint
                };
                painter.rect_filled(
                    Rect::from_min_size(pos2(a, b), vec2(c.max(0.), d.max(0.))),
                    e.clamp(0., 100.),
                    color.gamma_multiply(f.clamp(0., 1.)),
                );
            }
            4 => {
                painter.circle_filled(
                    pos2(a, b),
                    c.clamp(0., 100.),
                    tint.gamma_multiply(d.clamp(0., 1.)),
                );
            }
            5 => {
                // Each artwork mod owns one sprite set; asset IDs are semantic ABI values.
                let Some(sprite) = guest.and_then(|g| g.sprites.first()) else {
                    continue;
                };
                let frame = e.max(0.) as usize % sprite.frames as usize;
                let cells = usize::from(sprite.columns) * usize::from(sprite.rows);
                let file = frame / cells;
                let cell = frame % cells;
                let Some(bytes) = sprite.files.get(file) else {
                    continue;
                };
                let cols = sprite.columns as f32;
                let rows = sprite.rows as f32;
                let uv = Rect::from_min_max(
                    pos2(
                        (cell % usize::from(sprite.columns)) as f32 / cols,
                        (cell / usize::from(sprite.columns)) as f32 / rows,
                    ),
                    pos2(
                        (cell % usize::from(sprite.columns) + 1) as f32 / cols,
                        (cell / usize::from(sprite.columns) + 1) as f32 / rows,
                    ),
                );
                egui::Image::from_bytes(
                    format!(
                        "bytes://mod-{}-v1-{file}.png",
                        crate::mod_packages::slug(id)
                    ),
                    bytes.clone(),
                )
                .uv(uv)
                .texture_options(if id == Id::CatJam {
                    egui::TextureOptions::LINEAR
                } else {
                    egui::TextureOptions::NEAREST
                })
                .paint_at(
                    ui,
                    Rect::from_min_size(pos2(a, b), vec2(c.max(0.), d.max(0.))),
                );
            }
            6 => super::track_peek::paint_neighbour(
                app,
                ui,
                Rect::from_min_size(pos2(a, b), vec2(c.max(0.), d.max(0.))),
                e > 0.,
            ),
            7 => {
                if let Some(now) = app.now_playing() {
                    super::widgets::paint_cover(
                        ui,
                        &app.palette,
                        now.art_url.as_deref(),
                        Rect::from_center_size(pos2(a, b), Vec2::splat(c.clamp(0., 4096.))),
                        d.clamp(0., 100.),
                        Icon::Music,
                        Some(app.backend.art()),
                    );
                }
            }
            8 => {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f32(
                        1. / a.clamp(1., 60.),
                    ));
            }
            _ => {}
        }
    }
}
