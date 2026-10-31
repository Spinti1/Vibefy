//! Winamp window, skin, and display state. Drawing lives in `ui::winamp`.
//!
//! The built-in Vibefy pixel skin is immediately available.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::settings::Settings;
use crate::skin::{Sheet, Skin};
use crate::vis::{Analyser, AudioTap};

/// The most screen pixels a skin pixel may take.
pub const MAX_SCALE: u32 = 4;
/// How many characters the marquee shows: thirty whole ones and the edge
/// of a thirty-first.
pub const MARQUEE_CHARS: usize = 31;
/// Text this long fits the marquee without scrolling.
const MARQUEE_FITS: usize = 30;
/// How long the marquee waits between one-character steps.
const MARQUEE_STEP: Duration = Duration::from_millis(220);
/// What separates the end of a scrolling title from its start again.
const MARQUEE_GAP: &str = "  ***  ";

/// The text with the gap it scrolls through, for drawing it as pixels.
pub fn marquee_strip(text: &str) -> String {
    format!("{text}{MARQUEE_GAP}")
}
pub struct WinampState {
    /// The skin on screen.
    pub skin: Arc<Skin>,
    /// The skin's sheets on the graphics card, made on demand and dropped
    /// with the window, since textures belong to a window's context.
    textures: HashMap<Sheet, egui::TextureHandle>,
    /// Count down instead of up; clicking the time toggles it.
    pub time_remaining: bool,
    /// The balance while its thumb is held, for the marquee to report.
    pub balance_preview: Option<f32>,
    /// Open the equalizer's presets menu on the next frame; the demo's
    /// `presets` surface asks for it.
    pub open_presets: bool,
    marquee_text: String,
    marquee_offset: usize,
    marquee_moved: Option<Instant>,
    /// Where the window is, or was last: from the session, then followed
    /// while it is open. Only the position is given to the next window,
    /// which takes its size from the skin.
    pub geometry: fastframe_shell::window::Geometry,
    /// The sound on its way out, for the visualiser.
    pub tap: Arc<AudioTap>,
    pub analyser: Analyser,
    /// The equalizer as the player's thread reads it.
    pub eq: crate::eq::SharedEq,
    /// The playlist window: its first visible row, the row clicked, the
    /// wheel's leftover, and the corner drag's leftover.
    pub playlist_scroll: usize,
    /// The rows selected, by URI; Ctrl-click adds and removes, SEL has
    /// the rest.
    pub playlist_selection: std::collections::HashSet<usize>,
    pub playlist_wheel: f32,
    pub playlist_resize: f32,
    /// The playlist's rows as Winamp drew them, kept once drawn.
    pub playlist_text: crate::ui::winamp::PixelText,
}

impl WinampState {
    pub fn new(restore_pos: Option<[f32; 2]>, tap: Arc<AudioTap>, eq: crate::eq::SharedEq) -> Self {
        Self {
            skin: Skin::builtin(),
            textures: HashMap::new(),
            time_remaining: false,
            balance_preview: None,
            open_presets: false,
            marquee_text: String::new(),
            marquee_offset: 0,
            marquee_moved: None,
            geometry: fastframe_shell::window::Geometry {
                position: restore_pos,
                ..Default::default()
            },
            tap,
            analyser: Analyser::default(),
            eq,
            playlist_scroll: 0,
            playlist_selection: std::collections::HashSet::new(),
            playlist_wheel: 0.0,
            playlist_resize: 0.0,
            playlist_text: crate::ui::winamp::PixelText::default(),
        }
    }

    /// Screen pixels per skin pixel: the setting, or else double size on
    /// this display, which is the size people remember Winamp at.
    pub fn scale(settings: &Settings, pixels_per_point: f32) -> u32 {
        let chosen = settings
            .skin_scale
            .map(u32::from)
            .unwrap_or_else(|| (2.0 * pixels_per_point).round() as u32);
        chosen.clamp(1, MAX_SCALE)
    }

    /// The window is gone (or about to be), and with it the textures.
    pub fn forget_textures(&mut self) {
        self.textures.clear();
        self.playlist_text.clear();
    }

    /// The skin's sheets as textures, made now if they are not yet.
    pub fn textures(&mut self, ctx: &egui::Context) -> HashMap<Sheet, egui::TextureId> {
        for sheet in Sheet::ALL {
            if self.textures.contains_key(&sheet) {
                continue;
            }
            let bitmap = self.skin.sheet(sheet);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [bitmap.width as usize, bitmap.height as usize],
                &bitmap.rgba,
            );
            let handle = ctx.load_texture(
                format!("winamp-{}", sheet.file_stem()),
                image,
                egui::TextureOptions::NEAREST,
            );
            self.textures.insert(sheet, handle);
        }
        self.textures
            .iter()
            .map(|(sheet, handle)| (*sheet, handle.id()))
            .collect()
    }

    /// The characters the marquee shows now: the text itself when it
    /// fits, otherwise a window onto it that moves a character at a time.
    pub fn marquee(&mut self, text: &str, now: Instant) -> (String, usize) {
        if text != self.marquee_text {
            self.marquee_text = text.to_string();
            self.marquee_offset = 0;
            self.marquee_moved = Some(now);
        }
        let mut chars: Vec<char> = self.marquee_text.chars().collect();
        if chars.len() <= MARQUEE_FITS {
            return (self.marquee_text.clone(), 0);
        }
        chars.extend(MARQUEE_GAP.chars());
        let moved = self.marquee_moved.get_or_insert(now);
        let steps =
            (now.saturating_duration_since(*moved).as_millis() / MARQUEE_STEP.as_millis()) as usize;
        if steps > 0 {
            self.marquee_offset = self.marquee_offset.wrapping_add(steps);
            *moved += MARQUEE_STEP * steps as u32;
        }
        let offset = self.marquee_offset;
        let shown = (0..MARQUEE_CHARS)
            .map(|index| chars[(offset + index) % chars.len()])
            .collect();
        (shown, offset)
    }

    /// Whether the marquee is on the move, and so wants frames.
    pub fn marquee_scrolling(&self) -> bool {
        self.marquee_text.chars().count() > MARQUEE_FITS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_size_defaults_to_double_on_this_display() {
        let mut settings = Settings::default();
        assert_eq!(WinampState::scale(&settings, 1.0), 2);
        assert_eq!(WinampState::scale(&settings, 2.0), 4);
        assert_eq!(WinampState::scale(&settings, 1.2), 2);
        assert_eq!(WinampState::scale(&settings, 1.5), 3);
        settings.skin_scale = Some(1);
        assert_eq!(WinampState::scale(&settings, 2.0), 1);
        settings.skin_scale = Some(9);
        assert_eq!(WinampState::scale(&settings, 1.0), MAX_SCALE);
    }

    #[test]
    fn a_short_title_sits_still_and_a_long_one_scrolls() {
        let mut state = WinampState::new(None, AudioTap::new(), crate::eq::shared());
        let start = Instant::now();
        assert_eq!(state.marquee("Spotifast", start).0, "Spotifast");
        assert!(!state.marquee_scrolling());

        let long = "Radiohead - Everything In Its Right Place (4:11)";
        let (first, offset) = state.marquee(long, start);
        assert_eq!(first.chars().count(), MARQUEE_CHARS);
        assert_eq!(offset, 0);
        assert!(long.starts_with(&first));
        assert!(state.marquee_scrolling());
        // Not yet time to move.
        assert_eq!(
            state.marquee(long, start + Duration::from_millis(100)).0,
            first
        );
        let (later, stepped) = state.marquee(long, start + MARQUEE_STEP);
        assert!(long[1..].starts_with(&later));
        assert_eq!(stepped, 1);
        // Eventually the start comes round again, after the gap.
        let round = long.chars().count() + MARQUEE_GAP.len();
        let again = state.marquee(long, start + MARQUEE_STEP * round as u32).0;
        assert_eq!(again, first);
        // A new title starts from its beginning.
        let other = "Something else entirely, and just as long as before";
        assert!(other.starts_with(&state.marquee(other, start + Duration::from_secs(9)).0));
    }
}
