//! Built-in visual effects, compiled natively and available offline.
//! SoundVisualizer geometries: CaYatur / Çağan Turgut / CaYaDev, MIT,
//! revision c3e7fc6bc3fa826e323486256268fa764a317963. Notice in assets/mods.
use crate::{
    addons::{Id, Preferences},
    mod_packages::Draw,
};
use std::f32::consts::{PI, TAU};

#[cfg(test)]
mod release_update_tests {
    use super::*;
    fn input(width: f32, height: f32) -> Vec<f32> {
        let mut input = vec![0.; 3584];
        input[2] = width;
        input[3] = height;
        input[4] = 12.;
        input[6] = 1.;
        input[12] = 1.;
        input[18] = 128.;
        input[19] = 24.;
        input[26] = 30.;
        for (i, value) in input[64..192].iter_mut().enumerate() {
            *value = (i as f32 * 0.1).sin().abs();
        }
        for (i, value) in input[256..3328].iter_mut().enumerate() {
            *value = (i as f32 * 0.1).cos().abs();
        }
        input
    }
    #[test]
    fn builtin_visualizers_render_all_16_effects_without_mod_packages() {
        assert_eq!(EFFECTS.len(), 16);
        for (width, height) in [(1200., 760.), (320., 240.)] {
            for effect in EFFECTS {
                let mut input = input(width, height);
                input[13] = effect.style as f32;
                let commands = render(effect.id, &input);
                assert!(!commands.is_empty(), "{}", effect.label);
                assert!(commands.len() <= 4096, "{}", effect.label);
                assert!(
                    commands
                        .iter()
                        .all(|c| c.values.iter().all(|v| v.is_finite()))
                );
                input[6] = 0.;
                assert!(render(effect.id, &input).iter().all(|c| c.kind != 8));
            }
        }
    }
    #[test]
    fn builtin_visualizer_wave_modes_follow_pcm() {
        for style in [3., 4., 10.] {
            let mut input = input(800., 600.);
            input[13] = style;
            let points = |input: &[f32]| {
                render(Id::SoundVisualizer, input)
                    .into_iter()
                    .filter(|c| c.kind == 1)
                    .map(|c| c.values)
                    .collect::<Vec<_>>()
            };
            let silent = points(&input);
            input[3328..].fill(0.5);
            assert_ne!(silent, points(&input));
        }
    }
    #[test]
    fn builtin_visualizers_migrate_old_mod_settings_and_keep_optional_mods() {
        let mut p = Preferences::default();
        p.require_packages = true;
        p.set(Id::Oneko, true);
        p.visualizer = Some(Id::Backmusic);
        p.visual_mut(Id::Backmusic).intensity = 2.;
        for id in p.available_visualizers() {
            p.installed.insert(id);
            p.enabled.insert(id);
            p.ready_packages.insert(id);
        }
        p.migrate_builtin_visualizers();
        assert_eq!(p.visualizer, Some(Id::Backmusic));
        assert_eq!(p.visual(Id::Backmusic).intensity, 2.);
        assert!(p.installed.contains(&Id::Oneko));
        for effect in EFFECTS {
            assert!(p.active(effect.id));
            assert!(!p.installed.contains(&effect.id));
            assert!(
                !crate::mod_packages::catalog()
                    .iter()
                    .any(|e| e.id == effect.id)
            );
            assert!(select(&mut p, effect.id, effect.style));
            assert_eq!(selected(&p), effect);
        }
        assert_eq!(p.visual(Id::Backmusic).intensity, 2.);
        let before = p.clone();
        assert!(!select(&mut p, Id::SoundVisualizer, 12));
        assert!(!select(&mut p, Id::Oneko, 0));
        assert_eq!(p, before);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effect {
    pub id: Id,
    pub style: u8,
    pub label: &'static str,
}
pub const EFFECTS: [Effect; 16] = [
    Effect {
        id: Id::Vivy,
        style: 0,
        label: "Vivy · Ridgeline",
    },
    Effect {
        id: Id::Vivy,
        style: 1,
        label: "Vivy · Drift",
    },
    Effect {
        id: Id::Backmusic,
        style: 0,
        label: "Backmusic · Rings",
    },
    Effect {
        id: Id::Backmusic,
        style: 1,
        label: "Backmusic · Spectrum",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 0,
        label: "SoundVisualizer · Bars",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 1,
        label: "SoundVisualizer · Center bars",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 2,
        label: "SoundVisualizer · Circular",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 3,
        label: "SoundVisualizer · Wave",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 4,
        label: "SoundVisualizer · Radial wave",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 5,
        label: "SoundVisualizer · Dots",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 6,
        label: "SoundVisualizer · Blocks",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 7,
        label: "SoundVisualizer · Starburst",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 8,
        label: "SoundVisualizer · Helix",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 9,
        label: "SoundVisualizer · Strings",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 10,
        label: "SoundVisualizer · Lissajous",
    },
    Effect {
        id: Id::SoundVisualizer,
        style: 11,
        label: "SoundVisualizer · Arcs",
    },
];
pub fn selected(preferences: &Preferences) -> Effect {
    let id = preferences.visualizer.unwrap_or(Id::Vivy);
    let style = preferences.visual(id).style;
    EFFECTS
        .iter()
        .find(|v| v.id == id && v.style == style)
        .copied()
        .unwrap_or(EFFECTS[0])
}
pub fn select(preferences: &mut Preferences, id: Id, style: u8) -> bool {
    if !EFFECTS.iter().any(|v| v.id == id && v.style == style) {
        return false;
    }
    preferences.visualizer = Some(id);
    preferences.visual_mut(id).style = style;
    true
}
pub fn render(id: Id, input: &[f32]) -> Vec<Draw> {
    let mut frame = Frame {
        input,
        output: Vec::new(),
    };
    let (x, y, w, h) = (frame.n(0), frame.n(1), frame.n(2), frame.n(3));
    if w < 1. || h < 1. {
        return frame.output;
    }
    match id {
        Id::Vivy => frame.render_vivy(x, y, w, h),
        Id::Backmusic => frame.render_backmusic(x, y, w, h),
        Id::SoundVisualizer => frame.render_sound(x, y, w, h),
        _ => {}
    }
    frame.output
}
struct Frame<'a> {
    input: &'a [f32],
    output: Vec<Draw>,
}
impl Frame<'_> {
    fn n(&self, i: i32) -> f32 {
        self.input
            .get(i as usize)
            .copied()
            .filter(|v| v.is_finite())
            .unwrap_or(0.)
    }
    fn draw(&mut self, kind: i32, values: [f32; 8]) {
        if self.output.len() < 4096 && values.iter().all(|v| v.is_finite()) {
            self.output.push(Draw { kind, values });
        }
    }
    fn line(&mut self, width: f32, alpha: f32) {
        self.draw(0, [width, alpha, 0., 0., 0., 0., 0., 0.]);
    }
    fn point(&mut self, x: f32, y: f32) {
        self.draw(1, [x, y, 0., 0., 0., 0., 0., 0.]);
    }
    fn end(&mut self) {
        self.draw(2, [0.; 8]);
    }
    fn repaint(&mut self, fps: f32) {
        self.draw(8, [fps, 0., 0., 0., 0., 0., 0., 0.]);
    }
    fn render_vivy(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let count = self.n(18) as i32;
        let rows = self.n(19) as i32;
        for row in (0..rows.min(24)).rev() {
            let depth = row as f32 / 24.;
            let width = w * (0.9 - depth * 0.45);
            let base = y + h * (0.72 - depth * 0.52);
            self.line(if row == 0 { 2.5 } else { 1. }, 0.95 - depth * 0.7);
            for i in 0..count.min(128) {
                let drift = if self.n(13) > 0. {
                    (i as f32 * 0.15 + self.n(4) * 0.3 + depth * 4.).sin() * h * 0.05
                } else {
                    0.
                };
                self.point(
                    x + w / 2. - width / 2. + width * i as f32 / (count - 1).max(1) as f32,
                    base - self.n(256 + row * 128 + i) * self.n(12) * h * 0.16 * (1. - depth)
                        + drift,
                );
            }
            self.end();
        }
        if self.n(6) > 0. {
            self.repaint(self.n(26));
        }
    }
    fn render_backmusic(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let radius = w.min(h) * 0.22;
        let cx = x + w / 2.;
        let cy = y + h / 2. - 20.;
        let count = (self.n(18) as i32).min(128);
        if self.n(13) < 1. {
            for ring in 0..5 {
                self.line(1.5, 0.8 - ring as f32 * 0.12);
                for i in 0..=128 {
                    let angle = i as f32 / 128. * std::f32::consts::TAU;
                    let band = if count > 0 {
                        self.n(64 + i % count)
                    } else {
                        0.
                    };
                    let r = radius * (1. + ring as f32 * 0.13) + band * self.n(12) * 32.;
                    self.point(cx + angle.cos() * r, cy + angle.sin() * r);
                }
                self.end();
            }
        } else {
            let width = w * 0.8;
            let step = width / count.max(1) as f32;
            for i in 0..count {
                let bh = (self.n(64 + i) * self.n(12) * h * 0.3).max(2.);
                let bw = (step - 3.).max(1.);
                self.draw(
                    3,
                    [
                        cx - width / 2. + i as f32 * step - bw / 2.,
                        cy - bh / 2.,
                        bw,
                        bh,
                        2.,
                        0.8,
                        -1.,
                        0.,
                    ],
                );
            }
        }
        if self.n(16) > 0. {
            self.draw(7, [cx, cy, radius * 1.15, 12., 0., 0., 0., 0.]);
        }
        if self.n(15) > 0. {
            for i in 0..48 {
                let seed = i as f32;
                self.draw(
                    4,
                    [
                        x + (seed * 0.618 + self.n(4) * 0.008).rem_euclid(1.) * w,
                        y + (seed * 0.431 - self.n(4) * 0.012).rem_euclid(1.) * h,
                        1. + self.n(17) * 1.5,
                        0.5,
                        0.,
                        0.,
                        0.,
                        0.,
                    ],
                );
            }
        }
        if self.n(6) > 0. {
            self.repaint(self.n(26));
        }
    }
    fn band(&self, i: usize, count: usize) -> f32 {
        let len = (self.n(18) as usize).clamp(1, 128);
        (self.n(64 + (i * len / count.max(1)).min(len - 1) as i32) * self.n(12)).clamp(0., 1.)
    }
    fn wave(&self, i: usize) -> f32 {
        (self.n(3328 + (i % 256) as i32) * self.n(12)).clamp(-1., 1.)
    }
    fn segment(&mut self, x: f32, y: f32, xx: f32, yy: f32, width: f32, alpha: f32) {
        self.line(width, alpha);
        self.point(x, y);
        self.point(xx, yy);
        self.end();
    }
    fn dot(&mut self, x: f32, y: f32, r: f32, alpha: f32) {
        self.draw(4, [x, y, r, alpha, 0., 0., 0., 0.]);
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
        self.draw(3, [x, y, w, h, 2., alpha, -1., 0.]);
    }
    fn render_sound(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let mode = self.n(13) as u8;
        let t = self.n(4); // Playback time freezes while paused, independent of UI repaints.
        let (cx, cy) = (x + w / 2., y + h / 2.);
        let dim = w.min(h);
        let bass = (0..8).map(|i| self.band(i, 64)).sum::<f32>() / 8.;
        match mode {
            0 | 1 => {
                // bars.js / centerbars.js
                let count = 64;
                let slot = w / count as f32;
                for i in 0..count {
                    let k = if mode == 1 {
                        (((i as f32 - 31.5).abs() / 32.) * 63.).round() as usize
                    } else {
                        i
                    };
                    let bh =
                        (self.band(k, count) * h * if mode == 1 { 0.44 } else { 0.80 }).max(1.);
                    self.rect(
                        x + i as f32 * slot + slot * 0.1,
                        if mode == 1 { cy - bh } else { y + h * 0.9 - bh },
                        slot * 0.8,
                        if mode == 1 { bh * 2. } else { bh },
                        0.9,
                    );
                }
            }
            2 | 7 => {
                // circular.js / starburst.js
                let count = 64;
                let base = dim
                    * if mode == 2 {
                        0.22 * (1. + bass * 0.28)
                    } else {
                        0.06
                    };
                for i in 0..count {
                    let a = i as f32 / count as f32 * TAU + t * 0.1;
                    let v = self.band(i, count);
                    let len = v * if mode == 2 {
                        dim * 0.22
                    } else {
                        (w * w + h * h).sqrt() * 0.46 * (1. + bass * 0.35)
                    };
                    // A few tapered sections approximate the original radial mask.
                    for j in 0..4 {
                        let r0 = base + len * j as f32 / 4.;
                        let r1 = base + len * (j + 1) as f32 / 4.;
                        self.segment(
                            cx + a.cos() * r0,
                            cy + a.sin() * r0,
                            cx + a.cos() * r1,
                            cy + a.sin() * r1,
                            if mode == 2 {
                                3.
                            } else {
                                dim * 0.006 * (0.35 + v)
                            },
                            1. - j as f32 * 0.2,
                        );
                    }
                }
            }
            3 => {
                // wave.js: actual PCM, not spectrum magnitudes.
                self.line(2., 0.95);
                for i in 0..256 {
                    self.point(x + w * i as f32 / 255., cy + self.wave(i) * h * 0.3528);
                }
                self.end();
            }
            4 => {
                // radialwave.js
                let radius = dim * 0.22 * (1. + bass * 0.28);
                self.line(2., 0.95);
                for i in 0..=256 {
                    let a = (i % 256) as f32 / 256. * TAU + t * 0.12;
                    let r = radius + self.wave(i) * dim * 0.1092;
                    self.point(cx + a.cos() * r, cy + a.sin() * r);
                }
                self.end();
            }
            5 | 6 => {
                // dots.js / blocks.js: bounded 48 x 24 grid.
                let cols = 48;
                let rows = (h / 30.).round().clamp(5., 24.) as usize;
                let sx = w / cols as f32;
                let sy = h * 0.84 / rows as f32;
                for i in 0..cols {
                    let lit = self.band(i, cols) * rows as f32;
                    for row in 0..rows {
                        let alpha = if (row as f32) < lit { 0.9 } else { 0.055 };
                        let px = x + (i as f32 + 0.5) * sx;
                        let py = y + h * 0.92 - (row as f32 + 0.5) * sy;
                        if mode == 5 {
                            self.dot(px, py, sx.min(sy) * 0.34, alpha);
                        } else {
                            self.rect(px - sx * 0.4, py - sy * 0.38, sx * 0.8, sy * 0.76, alpha);
                        }
                    }
                }
            }
            8 => {
                // extras.js Helix
                for side in 0..2 {
                    self.line(2., 0.85);
                    for i in 0..96 {
                        let f = i as f32 / 95.;
                        let amp = h * 0.276 * (0.35 + self.band(i, 96) * 0.9);
                        self.point(
                            x + f * w,
                            cy + (f * TAU * 2.4 + t * 1.1 + side as f32 * PI).sin() * amp,
                        );
                    }
                    self.end();
                }
                for i in (0..96).step_by(2) {
                    let f = i as f32 / 95.;
                    let v = self.band(i, 96);
                    let dy = (f * TAU * 2.4 + t * 1.1).sin() * h * 0.276 * (0.35 + v * 0.9);
                    self.segment(x + f * w, cy - dy, x + f * w, cy + dy, 1., 0.25 + v * 0.6);
                }
            }
            9 => {
                // extras.js Strings
                let count = 18;
                let slot = h / (count + 1) as f32;
                for i in 0..count {
                    let v = self.band(i, count);
                    self.line(1. + v, 0.35 + v * 0.65);
                    for j in 0..=90 {
                        let f = j as f32 / 90.;
                        let dy = (f * PI * (1 + i % 3) as f32 * 2. + t * (7. + i as f32 * 1.7))
                            .sin()
                            * (f * PI).sin()
                            * slot
                            * 0.4232
                            * v;
                        self.point(x + f * w, y + slot * (i + 1) as f32 + dy);
                    }
                    self.end();
                }
            }
            10 => {
                // extras.js Lissajous (mono phase-space, not stereo balance).
                let r = dim * 0.3864;
                let a = t * 0.15;
                self.line(1.7, 0.9);
                for i in 0..256 {
                    let u = self.wave(i);
                    let v = self.wave(i + 15);
                    self.point(
                        cx + (u * a.cos() - v * a.sin()) * r,
                        cy + (u * a.sin() + v * a.cos()) * r,
                    );
                }
                self.end();
            }
            _ => {
                // extras.js Arcs
                let count = 24;
                let r = dim * 0.46;
                let inner = r * 0.12;
                let step = (r - inner) / count as f32;
                for i in 0..count {
                    let v = self.band(i, count);
                    let radius = inner + step * (i as f32 + 0.5);
                    let sweep = TAU * (0.06 + v * 0.9);
                    let start = t * (0.3 + i as f32 * 0.045) * if i % 2 == 0 { 1. } else { -1. }
                        - sweep / 2.;
                    self.line((step * 0.7).max(1.), 0.28 + v * 0.72);
                    for j in 0..=48 {
                        let a = start + sweep * j as f32 / 48.;
                        self.point(cx + a.cos() * radius, cy + a.sin() * radius);
                    }
                    self.end();
                }
            }
        }
        if self.n(6) > 0. {
            self.repaint(self.n(26));
        }
    }
}
