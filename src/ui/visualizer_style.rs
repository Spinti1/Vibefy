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

pub fn color_at(rect: Rect, pos: Pos2, brightness: f32, alpha: f32) -> Color32 {
    const COLORS: [[u8; 3]; 5] = [
        [25, 240, 197],
        [65, 152, 255],
        [156, 100, 255],
        [255, 91, 180],
        [255, 193, 78],
    ];
    let x = ((pos.x - rect.left()) / rect.width().max(1.)).clamp(0., 1.);
    let y = ((pos.y - rect.top()) / rect.height().max(1.)).clamp(0., 1.);
    let at = (x * 0.75 + y * 0.25) * 4.;
    let i = (at as usize).min(3);
    let mix = (at - i as f32).clamp(0., 1.);
    let channel =
        |j| (COLORS[i][j] as f32 * (1. - mix) + COLORS[i + 1][j] as f32 * mix).round() as u8;
    Color32::from_rgb(channel(0), channel(1), channel(2))
        .gamma_multiply(brightness.clamp(0.2, 1.) * alpha.clamp(0., 1.))
}

pub fn stroke(rect: Rect, width: f32, brightness: f32, alpha: f32) -> egui::epaint::PathStroke {
    egui::epaint::PathStroke::new_uv(width, move |_, pos| color_at(rect, pos, brightness, alpha))
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
        let out = ctx.run_ui(Default::default(), |ui| {
            ui.painter().add(egui::Shape::line(
                (0..20).map(|i| egui::pos2(i as f32 * 15., 50.)).collect(),
                stroke(rect, 3., 1., 1.),
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
        let full = color_at(rect, rect.center(), 1., 1.);
        let dim = color_at(rect, rect.center(), 0.2, 1.);
        assert!(dim.r() < full.r() && dim.g() < full.g() && dim.b() < full.b());
    }
}
