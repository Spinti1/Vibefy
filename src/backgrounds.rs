//! Lightweight palette fields shared by the lyrics and visualizer backgrounds.
use crate::spicy_lyrics::Background;

pub fn reactive(style: Background) -> bool {
    matches!(style, Background::BeatGlow | Background::BeatWaves)
}
pub fn animated(style: Background) -> bool {
    reactive(style)
        || matches!(
            style,
            Background::AnimatedColors | Background::Aurora | Background::Fluid | Background::Orbit
        )
}
/// A bounded blend between two artwork colours. Pulse comes from the local audio tap.
pub fn shade(style: Background, u: f32, v: f32, time: f32, pulse: f32) -> f32 {
    let finite = |n: f32| if n.is_finite() { n } else { 0.0 };
    let x = finite(u).clamp(0.0, 1.0);
    let y = finite(v).clamp(0.0, 1.0);
    let t = finite(time).rem_euclid(10000.0);
    let p = finite(pulse).clamp(0.0, 1.0);
    let radius = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();
    let field = match style {
        Background::Aurora => {
            ((x * 9.0 + (y * 3.0 + t).sin() * 2.0 + t * 0.4).sin() * 0.5 + 0.5) * (0.55 + y * 0.45)
        }
        Background::Fluid => {
            0.5 + 0.25 * (x * 8.0 + (y * 5.0 - t).sin() * 2.0 + t).sin()
                + 0.25 * (y * 7.0 + (x * 4.0 + t).cos()).cos()
        }
        Background::Orbit => {
            let dx = x - 0.5 - t.cos() * 0.25;
            let dy = y - 0.5 - t.sin() * 0.25;
            (1.0 - (dx * dx + dy * dy).sqrt() * 1.8).max(0.0)
        }
        Background::BeatGlow => (1.0 - radius * (2.2 - p)).max(0.0) * (0.35 + p * 0.65),
        Background::BeatWaves => {
            0.2 + (0.15 + p * 0.65) * ((radius * 23.0 - t * 3.0).sin() * 0.5 + 0.5)
        }
        _ => 0.5 + 0.25 * (x * 5.0 + t).sin() + 0.25 * (y * 4.0 - t * 0.7).cos(),
    };
    field.clamp(0.0, 1.0)
}
