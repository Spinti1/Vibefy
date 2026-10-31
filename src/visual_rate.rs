//! Frame-rate choices shared by native audio visualizers.
/// How many frames a second the window draws by default.
pub const DEFAULT_FPS: u32 = 60;
/// Configurable frame-rate range. Zero is handled separately as uncapped.
pub const FPS_RANGE: std::ops::RangeInclusive<u32> = 10..=360;
/// Whether a refresh rate a screen reports is a rate at all: Windows reports
/// 0 or 1 Hz for "the hardware's default", which says nothing. Screens
/// faster than the dial goes still count.
pub fn is_screen_rate(hz: u32) -> bool {
    hz >= *FPS_RANGE.start()
}
/// Frame-rate slider stops: 30, 60, screen rate, current rate, and uncapped.
pub fn fps_stops(screen: u32, current: u32) -> Vec<u32> {
    let mut rates = vec![30, 60];
    let screen = if is_screen_rate(screen) { screen } else { 0 };
    for extra in [screen, current] {
        if extra > 0 && !rates.contains(&extra) {
            rates.push(extra);
        }
    }
    rates.sort_unstable();
    rates.push(0);
    rates
}

/// Label for a frame-rate stop, marking the screen's refresh rate.
pub fn fps_label(locale: crate::i18n::Locale, rate: u32, screen: u32) -> String {
    use crate::i18n::gettext;
    match rate {
        0 => gettext(locale, "Uncapped").into_owned(),
        rate if rate == screen => gettext(
            locale,
            // Translators: A frame rate that matches the display. Keep {rate}.
            "{rate} fps, your screen",
        )
        .replace("{rate}", &rate.to_string()),
        rate => format!("{rate} fps"),
    }
}

/// A screen's refresh rate in whole hertz, from the millihertz a monitor
/// reports, when it is a rate at all (see [`is_screen_rate`]).
pub fn screen_hz(millihertz: u32) -> Option<u32> {
    let hz = (f64::from(millihertz) / 1000.0).round() as u32;
    is_screen_rate(hz).then_some(hz)
}
