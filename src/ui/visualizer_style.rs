//! Cover selection and shared, spatial colours for the native visualizers.
use egui::{Color32, Pos2, Rect};

pub fn artwork<'a>(
    large: Option<&'a str>,
    small: Option<&'a str>,
    source_cover: Option<&'a str>,
) -> (Option<&'a str>, Option<&'a str>) {
    let usable = |url: Option<&'a str>| url.filter(|url| !url.trim().is_empty());
    let small = usable(small);
    let requested = usable(large).or(usable(source_cover)).or(small);
    (requested, small.filter(|url| Some(*url) != requested))
}

#[derive(Clone, Copy)]
pub struct Colors {
    values: [Color32; 8],
    len: usize,
}
impl Colors {
    pub fn from_art(art: Option<&[[u8; 3]]>, fallback: Color32) -> Self {
        let mut colors = Self {
            values: [fallback; 8],
            len: 1,
        };
        if let Some(art) = art.filter(|art| !art.is_empty()) {
            colors.len = art.len().min(8);
            for (target, rgb) in colors.values.iter_mut().zip(art.iter().take(8)) {
                *target = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            }
        }
        colors
    }
}

pub fn color_at(colors: Colors, rect: Rect, pos: Pos2, brightness: f32, alpha: f32) -> Color32 {
    let x = ((pos.x - rect.left()) / rect.width().max(1.)).clamp(0., 1.);
    let y = ((pos.y - rect.top()) / rect.height().max(1.)).clamp(0., 1.);
    let at = (x * 0.75 + y * 0.25) * (colors.len - 1) as f32;
    let i = (at as usize).min(colors.len - 1);
    let mix = (at - i as f32).clamp(0., 1.);
    colors.values[i]
        .lerp_to_gamma(colors.values[(i + 1).min(colors.len - 1)], mix)
        .gamma_multiply(brightness.clamp(0.2, 1.) * alpha.clamp(0., 1.))
}

pub fn stroke(
    colors: Colors,
    rect: Rect,
    width: f32,
    brightness: f32,
    alpha: f32,
) -> egui::epaint::PathStroke {
    egui::epaint::PathStroke::new_uv(width, move |_, pos| {
        color_at(colors, rect, pos, brightness, alpha)
    })
}

#[cfg(test)]
mod release_update_tests {
    use super::*;
    #[test]
    fn cover_and_colour_regression_artwork_falls_back_to_source_and_small_images() {
        assert_eq!(
            artwork(Some("large"), Some("small"), Some("source")),
            (Some("large"), Some("small"))
        );
        assert_eq!(
            artwork(None, Some("small"), Some("source")),
            (Some("source"), Some("small"))
        );
        assert_eq!(
            artwork(Some(" "), Some("small"), None),
            (Some("small"), None)
        );
        assert_eq!(artwork(None, None, Some("source")), (Some("source"), None));
        assert_eq!(artwork(None, None, None), (None, None));
    }
    #[test]
    fn cover_and_colour_regression_waveforms_have_multiple_colours_and_respect_brightness() {
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300., 100.));
        let art = [[180, 40, 30], [40, 80, 160], [200, 180, 100]];
        let colors = Colors::from_art(Some(&art), Color32::GREEN);
        assert_eq!(
            color_at(colors, rect, rect.left_top(), 1., 1.),
            Color32::from_rgb(180, 40, 30)
        );
        assert_eq!(
            color_at(colors, rect, rect.right_bottom(), 1., 1.),
            Color32::from_rgb(200, 180, 100)
        );
        let fallback = Colors::from_art(None, Color32::GRAY);
        assert_eq!(
            color_at(fallback, rect, rect.center(), 1., 1.),
            Color32::GRAY
        );
        let out = ctx.run_ui(Default::default(), |ui| {
            ui.painter().add(egui::Shape::line(
                (0..20).map(|i| egui::pos2(i as f32 * 15., 50.)).collect(),
                stroke(colors, rect, 3., 1., 1.),
            ));
        });
        let meshes = ctx.tessellate(out.shapes, out.pixels_per_point);
        let colours: std::collections::HashSet<_> = meshes
            .iter()
            .flat_map(|p| match &p.primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh
                    .vertices
                    .iter()
                    .filter(|v| v.color.a() > 0)
                    .map(|v| v.color)
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect();
        assert!(
            colours.len() > 3,
            "the waveform itself must contain multiple colours"
        );
        let full = color_at(colors, rect, rect.center(), 1., 1.);
        let dim = color_at(colors, rect, rect.center(), 0.2, 1.);
        assert!(dim.r() < full.r() && dim.g() < full.g() && dim.b() < full.b());
    }
}
