//! Shared bounded-mesh backgrounds for lyrics and visualizers.
use crate::{
    app::App,
    spicy_lyrics::{Appearance, Background},
};
use egui::{Color32, Rect, pos2, vec2};
pub(super) fn paint(app: &mut App, ui: &mut egui::Ui, rect: Rect, appearance: &Appearance) {
    let mut appearance = appearance.clone();
    appearance.normalize();
    let primary = app.now_playing_tint().unwrap_or(app.palette.accent);
    let secondary = app
        .visualizer_art_palette()
        .and_then(|colors| colors.get(1))
        .map(|rgb| Color32::from_rgb(rgb[0], rgb[1], rgb[2]))
        .unwrap_or(primary.gamma_multiply(0.45));
    let solid = Color32::from_rgb(
        appearance.color[0],
        appearance.color[1],
        appearance.color[2],
    );
    ui.painter().rect_filled(rect, 0.0, solid);
    if matches!(
        appearance.background,
        Background::AnimatedColors
            | Background::Aurora
            | Background::Fluid
            | Background::Orbit
            | Background::BeatGlow
            | Background::BeatWaves
    ) {
        let moving = !crate::motion::reduced(ui.ctx());
        if crate::backgrounds::reactive(appearance.background) {
            let ctx = ui.ctx().clone();
            super::addon_visuals::audio(app, &ctx);
        }
        let sounding = app
            .now_playing()
            .is_some_and(|now| now.local && now.playing && !now.loading);
        let pulse = if sounding && !crate::motion::reduced(ui.ctx()) {
            (app.addons.beat.pulse * 0.7
                + app.addons.bands.iter().take(10).copied().sum::<f32>() / 10.0 * 0.3)
                * appearance.reactivity
        } else {
            0.0
        };
        let time_id = egui::Id::new("spicy-background-time");
        let delta = ui.input(|input| input.stable_dt).clamp(0.0, 0.1);
        let time = ui.ctx().data_mut(|data| {
            let previous = data.get_temp::<f32>(time_id).unwrap_or(0.0);
            let time = if moving {
                previous + delta * 0.45 * appearance.animation_speed
            } else {
                previous
            };
            data.insert_temp(time_id, time);
            time
        });
        if moving {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(
                    1.0 / f64::from(if appearance.limit_fps {
                        appearance.fps.clamp(15, 60)
                    } else {
                        60
                    }),
                ));
        }
        let mut mesh = egui::Mesh::default();
        let (cols, rows) = (32u32, 24u32);
        mesh.vertices.reserve(((cols + 1) * (rows + 1)) as usize);
        mesh.indices.reserve((cols * rows * 6) as usize);
        for y in 0..=rows {
            for x in 0..=cols {
                let u = x as f32 / cols as f32;
                let v = y as f32 / rows as f32;
                let mix = crate::backgrounds::shade(appearance.background, u, v, time, pulse);
                mesh.colored_vertex(
                    pos2(
                        rect.left() + u * rect.width(),
                        rect.top() + v * rect.height(),
                    ),
                    super::blend(primary, secondary, mix),
                );
                if x < cols && y < rows {
                    let index = y * (cols + 1) + x;
                    mesh.add_triangle(index, index + 1, index + cols + 1);
                    mesh.add_triangle(index + 1, index + cols + 2, index + cols + 1);
                }
            }
        }
        ui.painter().add(egui::Shape::mesh(mesh));
    } else if matches!(
        appearance.background,
        Background::BlurredCover | Background::CoverArt | Background::CustomImage
    ) {
        let uri = if appearance.background == Background::CustomImage {
            appearance.custom_image.clone()
        } else {
            app.now_playing()
                .and_then(|now| now.art_url.or(now.art_small))
        };
        let blur = appearance.blur;
        if let Some(texture) =
            app.lyrics_backdrop
                .texture_with_blur(ui.ctx(), app.backend.art(), uri.as_deref(), blur)
        {
            let ratio = (rect.width() / rect.height().max(1.0))
                / (texture.size_vec2().x / texture.size_vec2().y.max(1.0));
            let size = if ratio > 1.0 {
                vec2(1.0, 1.0 / ratio)
            } else {
                vec2(ratio, 1.0)
            };
            let uv = Rect::from_center_size(pos2(0.5, 0.5), size);
            ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
        }
    }
    ui.painter().rect_filled(
        rect,
        0.0,
        Color32::from_black_alpha((appearance.darkness * 255.0) as u8),
    );
}
