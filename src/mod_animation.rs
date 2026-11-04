//! Original mod artwork and deterministic playback clocks.
#[cfg(test)]
pub const NYAN_FRAMES: [&[u8]; 6] = [
    include_bytes!("../assets/mods/nyan_1.png"),
    include_bytes!("../assets/mods/nyan_2.png"),
    include_bytes!("../assets/mods/nyan_3.png"),
    include_bytes!("../assets/mods/nyan_4.png"),
    include_bytes!("../assets/mods/nyan_5.png"),
    include_bytes!("../assets/mods/nyan_6.png"),
];
#[cfg(test)]
pub const CAT_JAM: &[u8] = include_bytes!("../assets/mods/catjam-atlas.png");
pub const CAT_FRAMES: usize = 325;
pub const CAT_COLUMNS: usize = 16;
pub const CAT_ROWS: usize = 21;
pub const CAT_WIDTH: usize = 112;
pub const CAT_HEIGHT: usize = 86;
pub const CAT_DURATION: f64 = 9.75;
pub const CAT_BASE_BPM: f64 = 135.48;

pub fn nyan_frame(position_ms: u32, bpm: Option<f32>) -> usize {
    let bpm = bpm.filter(|v| v.is_finite() && *v > 0.0).unwrap_or(116.0) as f64;
    ((f64::from(position_ms) / 1000.0 * bpm / 60.0 * 6.0).floor() as usize) % 6
}
pub fn cat_frame(position_ms: u32, bpm: Option<f32>) -> usize {
    let bpm = bpm
        .filter(|v| v.is_finite() && *v > 0.0)
        .map(f64::from)
        .unwrap_or(CAT_BASE_BPM);
    ((f64::from(position_ms) / 1000.0 * bpm / CAT_BASE_BPM / CAT_DURATION * CAT_FRAMES as f64)
        .floor() as usize)
        % CAT_FRAMES
}
pub fn cat_uv(frame: usize) -> egui::Rect {
    let frame = frame % CAT_FRAMES;
    let x = frame % CAT_COLUMNS;
    let y = frame / CAT_COLUMNS;
    let width = (CAT_COLUMNS * CAT_WIDTH) as f32;
    let height = (CAT_ROWS * CAT_HEIGHT) as f32;
    egui::Rect::from_min_max(
        egui::pos2(
            (x * CAT_WIDTH) as f32 / width + 0.5 / width,
            (y * CAT_HEIGHT) as f32 / height + 0.5 / height,
        ),
        egui::pos2(
            ((x + 1) * CAT_WIDTH) as f32 / width - 0.5 / width,
            ((y + 1) * CAT_HEIGHT) as f32 / height - 0.5 / height,
        ),
    )
}

/// Touch the left window edge and player bar, independent of page margins.
pub fn cat_rect(viewport: egui::Rect, scale: f32) -> egui::Rect {
    let scale = if scale.is_finite() {
        scale.clamp(0.5, 2.0)
    } else {
        0.9
    };
    let size = egui::vec2(CAT_WIDTH as f32, CAT_HEIGHT as f32) * scale;
    egui::Rect::from_min_size(
        egui::pos2(
            viewport.left(),
            viewport.bottom() - crate::theme::PLAYER_BAR_HEIGHT - size.y,
        ),
        size,
    )
}
