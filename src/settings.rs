//! User preferences, stored as one readable JSON file.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Local Library identity only. Never sent to Spotify as a context URI.
pub const LIKED_SONGS_KEY: &str = "spotifast:liked-songs";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LibraryShelf {
    #[default]
    Playlists,
    Albums,
    Artists,
    Podcasts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibrarySort {
    Library,
    RecentlyPlayed,
    Name,
    RecentlyAdded,
    Local,
    Spotify,
}

impl LibrarySort {
    pub fn supports(self, shelf: LibraryShelf) -> bool {
        match self {
            Self::RecentlyPlayed | Self::Name | Self::Library => true,
            Self::RecentlyAdded => matches!(shelf, LibraryShelf::Albums | LibraryShelf::Podcasts),
            Self::Local | Self::Spotify => shelf == LibraryShelf::Playlists,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    Dark,
    Light,
    #[default]
    System,
}

/// How the tray icon is drawn on Linux and Windows: in colour, or white or
/// black to match the panel. The macOS menu bar always gets the template
/// image, which it draws black or white to match itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrayIcon {
    #[default]
    Colour,
    White,
    Black,
}

/// Whether a Home shelf is drawn. Hidden shelves still refresh normally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HomeShelfSettings {
    pub visible: bool,
}

impl Default for HomeShelfSettings {
    fn default() -> Self {
        Self { visible: true }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HomeSettings {
    pub made_for_you: HomeShelfSettings,
}

/// What moves behind the player bar's controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerBarVis {
    #[default]
    Off,
    Spectrum,
    Waveform,
}

impl PlayerBarVis {
    /// The mode a click on the player bar moves to: spectrum, waveform,
    /// then off, as Winamp's visualizer cycles.
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Spectrum,
            Self::Spectrum => Self::Waveform,
            Self::Waveform => Self::Off,
        }
    }
}

/// Mini-player visualizer mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VisMode {
    #[default]
    Bars,
    Scope,
    Off,
}

impl VisMode {
    /// Next mode in the display's click cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Bars => Self::Scope,
            Self::Scope => Self::Off,
            Self::Off => Self::Bars,
        }
    }
}

/// The interface language: the operating system's, or one chosen in Settings.
///
/// Stored as `"system"` or a locale tag such as `"es"` or `"pt-BR"`. A file
/// without the field follows the system; a tag this version does not carry
/// also follows the system, rather than making the whole file unreadable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LanguageChoice {
    #[default]
    System,
    Locale(crate::i18n::Locale),
}

impl LanguageChoice {
    /// The locale the interface is drawn in.
    pub fn resolve(self) -> crate::i18n::Locale {
        match self {
            Self::System => crate::i18n::Locale::from_system(),
            Self::Locale(locale) => locale,
        }
    }
}

impl Serialize for LanguageChoice {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::System => "system",
            Self::Locale(locale) => locale.tag(),
        })
    }
}

impl<'de> Deserialize<'de> for LanguageChoice {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(value
            .as_str()
            .and_then(crate::i18n::Locale::from_tag)
            .map_or(Self::System, Self::Locale))
    }
}

impl ThemeChoice {
    pub const ALL: [ThemeChoice; 3] = [Self::System, Self::Light, Self::Dark];

    pub fn label(self, locale: crate::i18n::Locale) -> std::borrow::Cow<'static, str> {
        use crate::i18n::{gettext, pgettext};
        match self {
            Self::Dark => pgettext(locale, "theme", "Dark"),
            Self::Light => pgettext(locale, "theme", "Light"),
            Self::System => gettext(locale, "Follow system"),
        }
    }
}

/// How outbound HTTP traffic reaches the network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// No proxy, ignoring environment and OS proxy settings.
    #[default]
    Off,
    /// Legacy value, ignored when reading settings.
    System,
    /// A configured HTTP proxy.
    Http,
    /// A configured SOCKS5 proxy.
    Socks,
}

/// An equalizer preset saved by name: the preamp and the ten bands, so a
/// headphone correction curve keeps the headroom it needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserEqPreset {
    pub name: String,
    #[serde(default)]
    pub preamp_db: f32,
    pub bands_db: [f32; 10],
}

/// How often the banner suggesting a personal app has appeared. It shows at
/// most once a day, and never again after the listener closed it
/// [`SlowAppBanner::MAX_DISMISSALS`] times.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SlowAppBanner {
    /// When it last appeared, as an RFC 3339 timestamp.
    pub shown_at: Option<String>,
    /// How many times the listener closed it.
    pub dismissals: u8,
}

impl SlowAppBanner {
    pub const MAX_DISMISSALS: u8 = 3;
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Hidden Home playlists, scoped to the signed-in Spotify account.
    pub hidden_playlists: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    pub addons: crate::addons::Preferences,
    /// The Spotify Connect name other devices see.
    pub device_name: String,
    /// 96, 160, or 320 kbps.
    pub bitrate: u16,
    pub normalisation: bool,
    pub autoplay: bool,
    pub gapless: bool,
    /// The output local playback uses: the id of a device
    /// `fastframe_audio::output_devices` lists, or `None` for the system
    /// default. Files from 0.13.3 and earlier may hold a device's name instead; see
    /// `crate::sink::Saved`. The `audio_backend` they may also hold is
    /// ignored: fastframe-audio is the only output.
    pub audio_device: Option<String>,
    /// Windows output buffer in milliseconds. Smaller values may click under
    /// load; larger values delay playback controls.
    /// See [`crate::sink::DEFAULT_BUFFER_MS`].
    #[serde(default = "default_buffer_ms")]
    pub audio_buffer_ms: u32,
    pub audio_cache: bool,
    pub audio_cache_mb: u64,
    /// The folder, searched with its subfolders, that a playlist's
    /// `spotify:local:` songs play from; `None` plays none.
    pub local_files_folder: Option<String>,
    pub theme: ThemeChoice,
    /// The interface language; older files without it follow the system.
    pub language: LanguageChoice,
    /// Filename selected from the local themes directory.
    pub custom_theme: Option<String>,
    /// Last accepted appearance, retained if its source file becomes unavailable.
    #[serde(
        default,
        deserialize_with = "fastframe_theme::read_cached_theme",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_theme_cache: Option<crate::theme::CustomTheme>,
    /// Last detected system palette, so following Omarchy survives a restart.
    #[serde(
        default,
        deserialize_with = "fastframe_theme::read_cached_theme",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_theme_cache: Option<crate::theme::CustomTheme>,
    /// The palettes Follow system shows in light and in dark mode, each with
    /// its last accepted colours; empty slots are Spotifast's own Light and
    /// Dark, as before this setting existed.
    #[serde(skip_serializing_if = "fastframe_theme::Pair::is_default")]
    pub theme_pair: crate::theme::ThemePair,
    pub home: HomeSettings,
    /// Tint the interface with the colour of the playing album's art.
    pub accent_from_art: bool,
    /// A spectrum or waveform of the playing song behind the player bar.
    pub player_bar_vis: PlayerBarVis,
    /// Allow clicks on the player bar background to cycle visualizer modes.
    pub player_bar_vis_click: bool,
    /// How many frames a second the player bar's visualizer and the mini
    /// player's analyser draw; 0 is uncapped.
    pub visualizer_fps: u32,
    /// Last local volume, 0..=65535.
    pub volume: u16,
    /// Percentage change per mouse scroll notch on the volume slider (1..=20%).
    /// Default is 5%.
    #[serde(default = "default_volume_scroll_step")]
    pub volume_scroll_step: u8,
    /// Whether the library sidebar is visible.
    pub sidebar_visible: bool,
    /// The playing album's art docked large at the sidebar's bottom.
    pub art_expanded: bool,
    /// Use compact single-line rows without cover art in the sidebar.
    pub sidebar_compact: bool,
    /// Show the Library as responsive cover cards instead of rows.
    pub sidebar_grid: bool,
    pub sidebar_width: f32,
    pub lyrics_width: f32,
    /// Public API key for the optional native Spicy Lyrics provider.
    #[serde(default)]
    pub spicy_lyrics_key: String,
    /// Added to playback time when highlighting lyrics, in milliseconds.
    #[serde(default)]
    pub spicy_lyrics_offset_ms: i32,
    #[serde(default)]
    pub spicy_lyrics_appearance: crate::spicy_lyrics::Appearance,
    pub spicy_lyrics_auto_sync: bool,
    pub cache_notice_at: u64,
    pub spicy_lyrics_auto_results: HashMap<String, crate::auto_sync::Alignment>,
    #[serde(default)]
    pub spicy_lyrics_calibrations:
        std::collections::HashMap<String, crate::spicy_lyrics::Calibration>,
    pub queue_width: f32,
    /// Use compact single-line rows without cover art in track lists.
    pub tracklist_compact: bool,
    /// Linux: middle-click a list to autoscroll it. Off by default, because
    /// Linux desktops usually paste the primary selection on middle click.
    /// Windows always autoscrolls and macOS never does.
    pub middle_click_autoscroll: bool,
    pub search_history: Vec<String>,
    pub show_shortcut_hints: bool,
    /// An optional personal Spotify Web API application id. The shared
    /// application remains active for coverage when this is present.
    pub web_client_id: Option<String>,
    /// Legacy reminder time, retained for older Spotifast versions.
    pub personal_app_nudge_at: Option<String>,
    /// The listener has dismissed or followed the personal-app introduction.
    pub personal_app_intro_seen: bool,
    /// When the "Spotify is being slow" banner last appeared and how often
    /// the listener closed it. Kept by the app, not chosen in Settings.
    pub slow_app_banner: SlowAppBanner,
    /// Local playback has been authorized at least once on this machine, so
    /// the app can resume it silently instead of prompting.
    pub playback_authorized: bool,
    /// Closing the window hides to the tray and keeps the music playing.
    pub keep_playing_in_background: bool,
    /// Linux and Windows: the tray icon in colour, white or black (#128).
    pub tray_icon: TrayIcon,
    /// Show the interactive Now Playing widget when hovering over the MacBook notch.
    pub mac_notch_widget: bool,
    /// macOS: keep Spotifast in the Dock and Cmd-Tab. Off makes it a
    /// menu-bar app, whose menu-bar icon is the way back (#837).
    pub mac_show_in_dock: bool,
    /// Ask GitHub once a day whether a newer release exists.
    pub check_for_updates: bool,
    pub download_updates_automatically: bool,
    /// Context URIs and the local Liked Songs key, in pin order.
    pub pinned_contexts: Vec<String>,
    /// Older settings keep Liked Songs first until it is moved or unpinned.
    pub liked_songs_pinned: bool,
    /// The sidebar's own playlist order, set by dragging rows. Kept while
    /// another sort is selected; empty means no saved local arrangement.
    pub sidebar_order: Vec<String>,
    /// Explicit order per Library shelf. Missing shelves keep their previous
    /// behaviour; selecting another order never deletes the local arrangement.
    pub library_sort: std::collections::BTreeMap<LibraryShelf, LibrarySort>,
    /// Show only playlists owned by the signed-in account in the playlist shelf.
    pub library_by_you: bool,
    /// Interface zoom, egui's zoom factor; Ctrl+plus/minus changes it.
    pub zoom: f32,
    /// The Winamp window is open.
    pub winamp_window: bool,
    /// Windows and X11: keep a taskbar button while the Winamp window is visible.
    pub winamp_show_taskbar: bool,
    /// Windows and Linux: draw Spotifast's own title bar and window buttons
    /// instead of the desktop frame.
    pub custom_titlebar: bool,
    /// Screen pixels per skin pixel; `None` picks double size for the
    /// display.
    pub skin_scale: Option<u8>,
    /// The Winamp window stays above other windows.
    pub winamp_on_top: bool,
    /// The mini player's visualiser: bars, scope, or off.
    pub vis: VisMode,
    /// The playlist window is open under the mini player.
    pub playlist_open: bool,
    /// How tall the playlist window is, in skin pixels.
    pub playlist_height: u32,
    /// The equalizer window is open under the mini player.
    pub eq_open: bool,
    /// The equalizer shapes local playback.
    pub eq_on: bool,
    /// The preamp, in decibels, never above zero.
    pub eq_preamp_db: f32,
    /// The ten bands, in decibels, 60 Hz to 16 kHz.
    pub eq_bands_db: [f32; 10],
    /// Equalizer presets saved here, listed after the built-in ones (#702).
    pub eq_user_presets: Vec<UserEqPreset>,
    /// The balance, -1 all left to 1 all right.
    pub balance: f32,
    /// Play both channels the same.
    pub mono: bool,
    /// The playlist window is rolled up to its title bar.
    pub playlist_shaded: bool,
    /// The equalizer window is rolled up to its title bar.
    pub eq_shaded: bool,
    /// The main window is rolled up to its title bar.
    pub winamp_shaded: bool,
    /// Which proxy to use. Older files without this field stay on `system`.
    #[serde(skip)]
    pub proxy_mode: ProxyMode,
    /// Combined address from older settings files. Split into host and port
    /// on load; not written again.
    #[serde(skip)]
    pub proxy: String,
    /// Host of a manual HTTP or SOCKS5 proxy. Ignored when the mode is Off
    /// or System.
    #[serde(skip)]
    pub proxy_host: String,
    /// Port of a manual HTTP or SOCKS5 proxy.
    #[serde(skip)]
    pub proxy_port: String,
    /// Optional proxy login for Web requests.
    #[serde(skip)]
    pub proxy_username: String,
    /// Optional proxy password, in memory and the platform credential store.
    /// Legacy JSON copies remain until protected migration succeeds.
    #[serde(skip)]
    pub proxy_password: String,
    /// Legacy settings and their password must survive until protected migration
    /// succeeds. This flag is only in memory and is never persisted.
    #[serde(skip)]
    pub proxy_password_legacy: bool,
}

impl std::fmt::Debug for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Serialization already omits the password and old combined URL.
        // Keep the proxy username out of diagnostics as well.
        let mut preferences = serde_json::to_value(self).map_err(|_| std::fmt::Error)?;
        if let Some(fields) = preferences.as_object_mut() {
            fields.remove("proxy_username");
        }
        f.debug_struct("Settings")
            .field("preferences", &preferences)
            .finish()
    }
}

impl Default for Settings {
    /// Supply the initial preferences and defaults for fields absent from older
    /// settings files, including an inactive playlist ownership filter.
    fn default() -> Self {
        Self {
            hidden_playlists: Default::default(),
            device_name: "Vibefy".to_string(),
            addons: crate::addons::Preferences::default(),
            bitrate: 320,
            normalisation: false,
            autoplay: true,
            gapless: true,
            audio_device: None,
            local_files_folder: None,
            audio_buffer_ms: default_buffer_ms(),
            audio_cache: true,
            audio_cache_mb: 1024,
            theme: ThemeChoice::System,
            language: LanguageChoice::System,
            custom_theme: None,
            custom_theme_cache: None,
            system_theme_cache: None,
            theme_pair: crate::theme::ThemePair::default(),
            home: HomeSettings::default(),
            accent_from_art: true,
            player_bar_vis: PlayerBarVis::Off,
            player_bar_vis_click: true,
            visualizer_fps: crate::visual_rate::DEFAULT_FPS,
            volume: (u16::MAX as u32 * 70 / 100) as u16,
            volume_scroll_step: default_volume_scroll_step(),
            sidebar_visible: true,
            art_expanded: false,
            sidebar_compact: false,
            sidebar_grid: false,
            sidebar_width: 250.0,
            lyrics_width: 360.0,
            spicy_lyrics_key: String::new(),
            spicy_lyrics_offset_ms: 0,
            spicy_lyrics_appearance: Default::default(),
            spicy_lyrics_auto_sync: true,
            cache_notice_at: 0,
            spicy_lyrics_auto_results: Default::default(),
            spicy_lyrics_calibrations: Default::default(),
            queue_width: 360.0,
            tracklist_compact: false,
            middle_click_autoscroll: false,
            search_history: Vec::new(),
            show_shortcut_hints: true,
            web_client_id: None,
            personal_app_nudge_at: None,
            personal_app_intro_seen: false,
            slow_app_banner: SlowAppBanner::default(),
            playback_authorized: false,
            keep_playing_in_background: true,
            tray_icon: TrayIcon::Colour,
            mac_notch_widget: false,
            mac_show_in_dock: true,
            check_for_updates: true,
            download_updates_automatically: false,
            pinned_contexts: Vec::new(),
            liked_songs_pinned: true,
            sidebar_order: Vec::new(),
            library_sort: std::collections::BTreeMap::new(),
            library_by_you: false,
            zoom: 1.0,
            winamp_window: false,
            winamp_show_taskbar: true,
            custom_titlebar: false,
            skin_scale: None,
            winamp_on_top: false,
            vis: VisMode::default(),
            playlist_open: false,
            playlist_height: 174,
            eq_open: false,
            eq_on: false,
            eq_preamp_db: 0.0,
            eq_bands_db: [0.0; 10],
            eq_user_presets: Vec::new(),
            balance: 0.0,
            mono: false,
            playlist_shaded: false,
            eq_shaded: false,
            winamp_shaded: false,
            proxy_mode: ProxyMode::Off,
            proxy: String::new(),
            proxy_host: String::new(),
            proxy_port: String::new(),
            proxy_username: String::new(),
            proxy_password: String::new(),
            proxy_password_legacy: false,
        }
    }
}

fn default_buffer_ms() -> u32 {
    crate::sink::DEFAULT_BUFFER_MS
}

fn default_volume_scroll_step() -> u8 {
    5
}

impl Settings {
    /// The local files folder, when one is set.
    pub fn local_files_folder(&self) -> Option<std::path::PathBuf> {
        self.local_files_folder
            .as_deref()
            .filter(|folder| !folder.trim().is_empty())
            .map(std::path::PathBuf::from)
    }

    /// The one palette shown whatever the system's appearance: a chosen
    /// palette file, or Omarchy's while Follow system follows it.
    pub(crate) fn fixed_palette(&self) -> Option<crate::theme::Palette> {
        let theme = if self.custom_theme.is_some() {
            self.custom_theme_cache.as_ref()
        } else if self.theme == ThemeChoice::System {
            self.system_theme_cache.as_ref()
        } else {
            None
        };
        theme.map(|theme| theme.palette)
    }

    /// The palette to show while the system is `dark` or light: the fixed
    /// one, else Follow system's choice for that appearance. `None` leaves
    /// Spotifast's own Light or Dark.
    pub(crate) fn cached_palette(&self, dark: bool) -> Option<crate::theme::Palette> {
        if let Some(palette) = self.fixed_palette() {
            return Some(palette);
        }
        let base = fastframe_theme::Base::from_dark(dark);
        (self.theme == ThemeChoice::System && self.theme_pair.slot(base).is_some())
            .then(|| self.theme_pair.palette(base))
    }

    pub(crate) fn proxy_password_record(
        &self,
    ) -> Result<Option<crate::credentials::ProxyPassword>, String> {
        Ok(None)
    }

    pub fn library_pins(&self) -> Vec<String> {
        let mut pins = self.pinned_contexts.clone();
        if !self.liked_songs_pinned {
            pins.retain(|key| key != LIKED_SONGS_KEY);
        } else if !pins.iter().any(|key| key == LIKED_SONGS_KEY) {
            pins.insert(0, LIKED_SONGS_KEY.into());
        }
        pins
    }

    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                // A file edited in Notepad or Windows PowerShell can start with
                // a byte order mark, which JSON does not allow.
                let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
                let mut settings = serde_json::from_str(text).unwrap_or_else(|error| {
                    log::warn!("settings at {} are unreadable: {error}", path.display());
                    Self::default()
                });
                settings.migrate_proxy(text);
                settings.proxy_password_legacy = !settings.proxy_password.is_empty();
                if settings.device_name == "Spotifast" {
                    settings.device_name = "Vibefy".to_string();
                }
                settings
            }
            Err(_) => Self::default(),
        }
    }

    /// The settings as their file holds them.
    pub fn encode(&self) -> Option<String> {
        serde_json::to_string_pretty(self)
            .map_err(|error| log::warn!("unable to encode settings: {error}"))
            .ok()
    }

    /// Writes the settings and waits for the disk. The app saves through
    /// its [`fastframe_file::Writer`] instead.
    pub fn save(&self, path: &Path) {
        let Some(text) = self.encode() else { return };
        if let Err(error) = fastframe_file::write(path, text) {
            log::warn!("unable to save settings to {}: {error}", path.display());
        }
    }

    pub fn remember_search(&mut self, query: &str) {
        let query = query.trim();
        if query.is_empty() {
            return;
        }
        self.search_history.retain(|entry| entry != query);
        self.search_history.insert(0, query.to_string());
        self.search_history.truncate(12);
    }

    pub(crate) fn migrate_proxy(&mut self, _text: &str) {
        self.proxy_mode = ProxyMode::Off;
        self.proxy.clear();
        self.proxy_host.clear();
        self.proxy_port.clear();
        self.proxy_username.clear();
        self.proxy_password.clear();
        self.proxy_password_legacy = false;
    }

    /// The proxy the HTTP client should use. Off and System always succeed.
    /// HTTP and SOCKS5 need a host and port.
    pub fn proxy_config(&self) -> Result<ProxyConfig, String> {
        Ok(ProxyConfig::Off)
    }
}

/// Compatibility token for network worker APIs. Vibefy always connects directly.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProxyConfig {
    #[default]
    Off,
    Invalid(String),
}
impl ProxyConfig {
    pub(crate) fn youtube_browser_arguments(&self) -> Result<Option<String>, String> {
        Ok(Some("--no-proxy-server".into()))
    }
    pub(crate) fn password_record(&self) -> Option<crate::credentials::ProxyPassword> {
        None
    }
    pub(crate) fn restore_password(&mut self, _saved: &crate::credentials::ProxyPassword) -> bool {
        false
    }
    pub fn to_url_string(&self) -> Option<String> {
        None
    }
    pub fn librespot_url(&self) -> Option<reqwest::Url> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;

    #[test]
    fn new_profiles_follow_the_system_and_saved_choices_are_preserved() {
        use super::ThemeChoice;
        assert_eq!(Settings::default().theme, ThemeChoice::System);
        assert_eq!(ThemeChoice::default(), ThemeChoice::System);
        let empty: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.theme, ThemeChoice::System);
        for (json, choice) in [("dark", ThemeChoice::Dark), ("light", ThemeChoice::Light)] {
            let settings: Settings =
                serde_json::from_value(serde_json::json!({"theme": json, "volume": 37})).unwrap();
            assert_eq!(settings.theme, choice);
            assert_eq!(settings.volume, 37);
        }
        let settings: Settings = serde_json::from_value(serde_json::json!({
            "theme": "dark", "system_theme_cache": {"broken": true}, "volume": 37
        }))
        .unwrap();
        assert!(settings.system_theme_cache.is_none());
        assert_eq!(settings.theme, ThemeChoice::Dark);
        assert_eq!(settings.volume, 37);
    }

    /// Files from before fastframe-audio was the only output name a
    /// librespot backend. It is read and ignored, the rest of the file still
    /// loads, and the key is not written back, which older builds read as
    /// their own default.
    #[test]
    fn an_old_audio_backend_is_read_and_ignored() {
        for backend in ["pulseaudio", "rodio", "alsa", "no-such-backend"] {
            let settings: Settings = serde_json::from_value(serde_json::json!({
                "audio_backend": backend,
                "audio_device": "PipeWire Sound Server",
                "volume": 37,
            }))
            .unwrap();
            assert_eq!(settings.volume, 37);
            assert_eq!(
                settings.audio_device.as_deref(),
                Some("PipeWire Sound Server")
            );
            let saved: serde_json::Value =
                serde_json::from_str(&settings.encode().unwrap()).unwrap();
            assert!(saved.get("audio_backend").is_none());
            assert_eq!(saved["audio_device"], "PipeWire Sound Server");
        }
    }

    #[test]
    fn language_follows_the_system_unless_a_known_locale_was_chosen() {
        use super::LanguageChoice;
        use crate::i18n::Locale;
        assert_eq!(Settings::default().language, LanguageChoice::System);
        let older: Settings = serde_json::from_value(serde_json::json!({"volume": 37})).unwrap();
        assert_eq!(older.language, LanguageChoice::System);
        assert_eq!(older.volume, 37);
        for (json, choice) in [
            (serde_json::json!("system"), LanguageChoice::System),
            (
                serde_json::json!("es"),
                LanguageChoice::Locale(Locale::Spanish),
            ),
            (
                serde_json::json!("pt-BR"),
                LanguageChoice::Locale(Locale::PortugueseBrazil),
            ),
            (
                serde_json::json!("zh-Hant"),
                LanguageChoice::Locale(Locale::ChineseTraditional),
            ),
            (
                serde_json::json!("de"),
                LanguageChoice::Locale(Locale::German),
            ),
            // A language a later version added, or a hand-edited typo, keeps
            // the rest of the file and follows the system.
            (serde_json::json!("tlh"), LanguageChoice::System),
            (serde_json::json!(7), LanguageChoice::System),
            (serde_json::Value::Null, LanguageChoice::System),
        ] {
            let settings: Settings =
                serde_json::from_value(serde_json::json!({"language": json, "volume": 37}))
                    .unwrap();
            assert_eq!(settings.language, choice, "{json}");
            assert_eq!(settings.volume, 37);
        }
        for &locale in crate::i18n::LOCALES {
            let settings = Settings {
                language: LanguageChoice::Locale(locale),
                ..Default::default()
            };
            let text = serde_json::to_string(&settings).unwrap();
            assert!(text.contains(&format!("\"language\":\"{}\"", locale.tag())));
            let saved: Settings = serde_json::from_str(&text).unwrap();
            assert_eq!(saved.language, settings.language);
        }
        let text = serde_json::to_string(&Settings::default()).unwrap();
        assert!(text.contains("\"language\":\"system\""));
        assert_eq!(
            LanguageChoice::System.resolve(),
            Locale::English,
            "tests pin English"
        );
    }

    #[test]
    fn queue_width_round_trips_through_saved_settings() {
        let settings = Settings {
            queue_width: 300.0,
            ..Default::default()
        };
        let text = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(restored.queue_width, 300.0);
    }

    /// Older files keep the Dock icon; hiding it survives a save (#837).
    #[test]
    fn the_dock_icon_stays_unless_hidden_and_round_trips() {
        let empty: Settings = serde_json::from_str("{}").unwrap();
        assert!(empty.mac_show_in_dock);
        let hidden: Settings = serde_json::from_str(r#"{"mac_show_in_dock": false}"#).unwrap();
        assert!(!hidden.mac_show_in_dock);
        let encoded = serde_json::to_string(&hidden).unwrap();
        let decoded: Settings = serde_json::from_str(&encoded).unwrap();
        assert!(!decoded.mac_show_in_dock);
    }

    #[test]
    fn mac_notch_widget_defaults_to_false_when_absent_and_round_trips_true() {
        let empty: Settings = serde_json::from_str("{}").unwrap();
        assert!(!empty.mac_notch_widget);
        let enabled: Settings = serde_json::from_str(r#"{"mac_notch_widget": true}"#).unwrap();
        assert!(enabled.mac_notch_widget);
        let encoded = serde_json::to_string(&enabled).unwrap();
        let decoded: Settings = serde_json::from_str(&encoded).unwrap();
        assert!(decoded.mac_notch_widget);
    }

    #[test]
    fn custom_theme_cache_round_trips_and_a_bad_cache_keeps_other_settings() {
        let mut settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(settings.custom_theme.is_none() && settings.custom_theme_cache.is_none());
        settings.custom_theme = Some("gruvbox.json".into());
        let mut palette = crate::theme::Palette::light();
        palette.shadow = egui::Color32::from_rgba_unmultiplied(37, 128, 249, 117);
        settings.custom_theme_cache = Some(crate::theme::CustomTheme {
            filename: "gruvbox.json".into(),
            palette,
        });
        let encoded = serde_json::to_string(&settings).unwrap();
        assert_eq!(
            serde_json::from_str::<Settings>(&encoded).unwrap(),
            settings
        );
        let mut damaged = serde_json::to_value(&settings).unwrap();
        damaged["custom_theme_cache"] = serde_json::json!({"palette": "broken"});
        damaged["audio_cache_mb"] = 777.into();
        let recovered: Settings = serde_json::from_value(damaged).unwrap();
        assert_eq!(recovered.custom_theme.as_deref(), Some("gruvbox.json"));
        assert_eq!(recovered.audio_cache_mb, 777);
        assert!(recovered.custom_theme_cache.is_none());
    }

    /// Settings from before the light and dark pair keep their look, and
    /// write no pair until one is chosen (#619).
    #[test]
    fn follow_system_pairs_load_old_settings_unchanged_and_round_trip() {
        use super::ThemeChoice;
        use crate::theme::{CustomTheme, Palette};
        use fastframe_theme::Base;
        let mut nord = Palette::dark();
        nord.accent = egui::Color32::from_rgb(0x88, 0xc0, 0xd0);
        let cache = serde_json::to_value(CustomTheme {
            filename: "Nord.json".into(),
            palette: nord,
        })
        .unwrap();
        // One custom palette, with Follow system underneath: still Nord in
        // light and dark mode alike.
        let single: Settings = serde_json::from_value(serde_json::json!({
            "theme": "system", "custom_theme": "Nord.json", "custom_theme_cache": cache,
        }))
        .unwrap();
        assert!(single.theme_pair.is_default());
        assert_eq!(single.cached_palette(false), Some(nord));
        assert_eq!(single.cached_palette(true), Some(nord));
        // Plain Follow system: Spotifast's own Light and Dark.
        let follow: Settings = serde_json::from_str(r#"{"theme":"system"}"#).unwrap();
        assert_eq!(follow.cached_palette(false), None);
        assert_eq!(follow.cached_palette(true), None);
        let encoded = serde_json::to_value(&follow).unwrap();
        assert!(encoded.get("theme_pair").is_none(), "no pair, no field");

        let mut latte = Palette::light();
        latte.window = egui::Color32::from_rgb(0xef, 0xf1, 0xf5);
        let mut paired = follow.clone();
        assert!(paired.theme_pair.set(
            Base::Light,
            Some(&CustomTheme {
                filename: "Catppuccin Latte.json".into(),
                palette: latte,
            })
        ));
        assert_eq!(paired.cached_palette(false), Some(latte));
        assert_eq!(paired.cached_palette(true), None, "dark stays Spotifast's");
        let encoded = serde_json::to_string(&paired).unwrap();
        let decoded: Settings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, paired);

        // A pair waits behind a fixed choice and comes back with Follow system.
        let mut fixed = paired.clone();
        fixed.theme = ThemeChoice::Dark;
        assert_eq!(fixed.cached_palette(false), None);
        fixed.theme = ThemeChoice::System;
        assert_eq!(fixed.cached_palette(false), Some(latte));

        // A damaged cache keeps the choice and the other settings.
        let mut damaged = serde_json::to_value(&paired).unwrap();
        damaged["theme_pair"]["light"]["palette"] = "broken".into();
        damaged["audio_cache_mb"] = 777.into();
        let recovered: Settings = serde_json::from_value(damaged).unwrap();
        assert_eq!(
            recovered.theme_pair.filename(Base::Light),
            Some("Catppuccin Latte.json")
        );
        assert_eq!(recovered.cached_palette(false), Some(Palette::light()));
        assert_eq!(recovered.audio_cache_mb, 777);
    }

    #[test]
    fn partial_home_preferences_keep_defaults_and_survive_a_settings_round_trip() {
        for visible in [true, false] {
            let settings: Settings = serde_json::from_value(
                serde_json::json!({"volume":12345,"home":{"made_for_you":{"visible":visible}}}),
            )
            .unwrap();
            assert_eq!(settings.home.made_for_you.visible, visible);
            let restored: Settings = serde_json::from_str(&settings.encode().unwrap()).unwrap();
            assert_eq!(restored, settings);
        }
    }

    /// Notepad and Windows PowerShell can save UTF-8 with a byte order mark.
    /// A file edited that way, as the Home shelves guide asks, is still read
    /// rather than dropped as unreadable and replaced with the defaults.
    #[test]
    fn a_settings_file_saved_with_a_byte_order_mark_keeps_its_preferences() {
        let dir = std::env::temp_dir().join(format!("spotifast-bom-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("settings.json");
        std::fs::write(
            &path,
            "\u{feff}{\"volume\":12345,\"home\":{\"made_for_you\":{\"visible\":false}}}",
        )
        .unwrap();
        let settings = Settings::load(&path);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(settings.volume, 12345);
        assert!(!settings.home.made_for_you.visible);
    }

    #[test]
    fn older_settings_keep_the_sidebar_visible() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(settings.sidebar_visible);
    }

    #[test]
    fn older_library_settings_keep_liked_songs_ahead_of_existing_pins() {
        let settings: Settings = serde_json::from_str(
            r#"{"pinned_contexts":["spotify:playlist:one"],"sidebar_order":["spotify:playlist:two"]}"#,
        ).unwrap();
        assert!(settings.liked_songs_pinned);
        assert_eq!(
            settings.library_pins(),
            [super::LIKED_SONGS_KEY, "spotify:playlist:one"]
        );
        assert_eq!(settings.sidebar_order, ["spotify:playlist:two"]);
    }

    /// Saved equalizer presets read back as written; files from before them
    /// have none, and a preset without a preamp keeps it at zero (#702).
    #[test]
    fn saved_equalizer_presets_round_trip_and_default_to_none() {
        let settings: Settings = serde_json::from_str(r#"{"zoom": 1.2}"#).unwrap();
        assert!(settings.eq_user_presets.is_empty());
        let settings: Settings = serde_json::from_str(
            r#"{"eq_user_presets":[{"name":"AirPods","preamp_db":-3.0,"bands_db":[1.2,0,-1.9,-2,0,1.6,1.6,2.6,0,-4.4]},{"name":"Old","bands_db":[0,0,0,0,0,0,0,0,0,1]}]}"#,
        )
        .unwrap();
        assert_eq!(settings.eq_user_presets[0].preamp_db, -3.0);
        assert_eq!(settings.eq_user_presets[1].preamp_db, 0.0);
        let again: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(again.eq_user_presets, settings.eq_user_presets);
    }

    #[test]
    fn older_settings_keep_the_winamp_window_closed_and_the_built_in_skin() {
        let settings: Settings = serde_json::from_str(r#"{"zoom": 1.2}"#).unwrap();
        assert!(!settings.winamp_window);
        assert!(settings.winamp_show_taskbar);
        assert_eq!(settings.skin_scale, None);
        assert!(!settings.winamp_on_top);
        assert_eq!(settings.vis, super::VisMode::Bars);
        assert!(!settings.playlist_open);
        assert_eq!(settings.playlist_height, 174);
        assert!(!settings.eq_on);
        assert_eq!(settings.eq_bands_db, [0.0; 10]);
        assert_eq!(settings.balance, 0.0);
        assert!(!settings.mono);
        assert!(!settings.playlist_shaded);
        assert!(!settings.eq_shaded);
        assert!(!settings.winamp_shaded);
    }

    #[test]
    fn the_visualiser_cycles_bars_scope_off() {
        use super::VisMode;
        assert_eq!(VisMode::Bars.next(), VisMode::Scope);
        assert_eq!(VisMode::Scope.next(), VisMode::Off);
        assert_eq!(VisMode::Off.next(), VisMode::Bars);
        let settings: Settings = serde_json::from_str(r#"{"vis": "scope"}"#).unwrap();
        assert_eq!(settings.vis, VisMode::Scope);
    }

    #[test]
    fn mini_player_settings_round_trip() {
        let settings = Settings {
            winamp_window: true,
            skin_scale: Some(3),
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, settings);
    }

    #[test]
    fn hidden_sidebar_round_trips() {
        let settings = Settings {
            sidebar_visible: false,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert!(!restored.sidebar_visible);
    }

    #[test]
    fn older_settings_default_to_standard_sidebar() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(!settings.sidebar_compact);
    }

    #[test]
    fn compact_sidebar_round_trips() {
        let settings = Settings {
            sidebar_compact: true,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert!(restored.sidebar_compact);
    }

    #[test]
    fn library_grid_round_trips_and_older_settings_keep_the_list() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert!(!old.sidebar_grid);

        let settings = Settings {
            sidebar_grid: true,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert!(restored.sidebar_grid);
    }

    #[test]
    fn older_settings_default_to_standard_tracklist() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(!settings.tracklist_compact);
    }

    #[test]
    fn compact_tracklist_round_trips() {
        let settings = Settings {
            tracklist_compact: true,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert!(restored.tracklist_compact);
    }

    /// The main window's visualizers draw 60 frames a second until the user
    /// picks another rate, and the rate they pick, uncapped included, is
    /// the rate they get back.
    #[test]
    fn the_visualizer_frame_rate_defaults_to_60_and_round_trips() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.visualizer_fps, 60);
        assert_eq!(Settings::default().visualizer_fps, 60);
        for rate in [30, 144, 0] {
            let chosen = Settings {
                visualizer_fps: rate,
                ..Settings::default()
            };
            let json = serde_json::to_string(&chosen).unwrap();
            let restored: Settings = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.visualizer_fps, rate);
        }
    }

    /// Older files keep today's colour icon, and the choice round-trips as
    /// a lowercase word (#128).
    #[test]
    fn the_tray_icon_stays_in_colour_unless_chosen_and_round_trips() {
        use super::TrayIcon;
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.tray_icon, TrayIcon::Colour);
        for (choice, word) in [
            (TrayIcon::Colour, "colour"),
            (TrayIcon::White, "white"),
            (TrayIcon::Black, "black"),
        ] {
            let json = serde_json::to_value(Settings {
                tray_icon: choice,
                ..Settings::default()
            })
            .unwrap();
            assert_eq!(json["tray_icon"], word);
            let restored: Settings = serde_json::from_value(json).unwrap();
            assert_eq!(restored.tray_icon, choice);
        }
    }

    #[test]
    fn the_player_bar_visualizer_is_opt_in_and_round_trips() {
        use super::PlayerBarVis;
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.player_bar_vis, PlayerBarVis::Off);
        assert!(settings.player_bar_vis_click);
        let disabled = Settings {
            player_bar_vis_click: false,
            ..Settings::default()
        };
        let json = serde_json::to_string(&disabled).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert!(!restored.player_bar_vis_click);
        for mode in [PlayerBarVis::Spectrum, PlayerBarVis::Waveform] {
            let settings = Settings {
                player_bar_vis: mode,
                ..Settings::default()
            };
            let json = serde_json::to_string(&settings).unwrap();
            let restored: Settings = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.player_bar_vis, mode);
        }
        let spectrum: Settings = serde_json::from_str(r#"{"player_bar_vis":"spectrum"}"#).unwrap();
        assert_eq!(spectrum.player_bar_vis, PlayerBarVis::Spectrum);
        assert_eq!(PlayerBarVis::Off.next(), PlayerBarVis::Spectrum);
        assert_eq!(PlayerBarVis::Spectrum.next(), PlayerBarVis::Waveform);
        assert_eq!(PlayerBarVis::Waveform.next(), PlayerBarVis::Off);
    }

    #[test]
    fn middle_click_autoscroll_is_opt_in_and_round_trips() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(!settings.middle_click_autoscroll);
        let settings = Settings {
            middle_click_autoscroll: true,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert!(restored.middle_click_autoscroll);
    }

    #[test]
    fn personal_app_nudge_time_is_backward_compatible_and_round_trips() {
        let older: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(older.personal_app_nudge_at, None);
        assert!(!older.personal_app_intro_seen);

        let settings = Settings {
            personal_app_nudge_at: Some("2026-09-03T15:00:00Z".into()),
            personal_app_intro_seen: true,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored.personal_app_nudge_at,
            settings.personal_app_nudge_at
        );
        assert!(restored.personal_app_intro_seen);
    }

    #[test]
    fn slow_app_banner_state_is_backward_compatible_and_round_trips() {
        let older: Settings = serde_json::from_str(r#"{"personal_app_intro_seen":true}"#).unwrap();
        assert_eq!(older.slow_app_banner, super::SlowAppBanner::default());
        assert_eq!(older.slow_app_banner.dismissals, 0);
        let partial: Settings =
            serde_json::from_str(r#"{"slow_app_banner":{"dismissals":2}}"#).unwrap();
        assert_eq!(partial.slow_app_banner.dismissals, 2);
        assert_eq!(partial.slow_app_banner.shown_at, None);

        let settings = Settings {
            slow_app_banner: super::SlowAppBanner {
                shown_at: Some("2026-10-21T09:00:00Z".into()),
                dismissals: 1,
            },
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.slow_app_banner, settings.slow_app_banner);
    }
}

/// The last playlist tree received for one account.
///
/// Only folder order is cached. Edit grants must always come from the live
/// session because they can be revoked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CachedRootlist {
    pub account_id: String,
    pub entries: Vec<crate::player::RootlistEntry>,
}

/// Restorable UI session: what was open when the app last closed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionState {
    /// Recent explicit output choices, scoped to the verified Spotify account.
    pub device_history: crate::device_history::DeviceHistory,
    pub last_page: Option<String>,
    /// Context URIs most recently played, newest first.
    pub recent_contexts: Vec<String>,
    /// What was playing when the app closed, to resume from a cold start.
    pub last_context: Option<String>,
    pub last_track: Option<String>,
    pub last_position_ms: u32,
    /// Metadata and exact playlist occurrence, available before any network lookup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_playback: Option<PlaybackCheckpoint>,
    /// Manually queued songs to restore with the remembered track.
    ///
    /// Context rows are excluded to prevent duplicates. This replaced the old
    /// `last_queue` field, so sessions using that field restore no added rows.
    pub last_added_queue: Vec<String>,
    /// Queue rows displayed on the next start. Playback restores manual rows
    /// from `last_added_queue`; it does not enqueue this list.
    pub last_queue_rows: Vec<crate::api::models::PlayableItem>,
    /// Sidebar folders rolled up, by their rootlist ids.
    pub collapsed_folders: Vec<String>,
    /// Last good playlist tree, scoped to the account that supplied it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootlist: Option<CachedRootlist>,
    /// Shuffle mode saved across contexts and restarts.
    pub shuffle_on: bool,
    /// Local repeat mode ("off", "context" or "track") saved across
    /// restarts. Sessions saved before it existed read as off.
    pub repeat: String,
    /// Each table's chosen sort, by encoded page, restored at start.
    pub sorts: Vec<(String, crate::model::TableSort)>,
    /// Last window inner size, to restore on next launch. Logical pixels
    /// (points at zoom 1) since fastframe-shell's `Geometry`; earlier
    /// versions saved points at the interface zoom, the same at zoom 1.
    pub window_size: Option<[f32; 2]>,
    /// Last window position, to restore on next launch: the outer frame's
    /// corner, the content's on macOS.
    pub window_pos: Option<[f32; 2]>,
    /// Whether the window was maximized; `window_size` and `window_pos`
    /// stay those of the normal window behind it.
    pub window_maximized: bool,
    /// Whether the queue panel was open.
    pub queue_open: Option<bool>,
    /// Which tab the queue panel showed: `queue` or `recents`.
    pub queue_tab: Option<String>,
    /// Last outer position of the Winamp window.
    pub winamp_pos: Option<[f32; 2]>,
    /// The window mode fullscreen lyrics left, when the app closed while
    /// showing them. eframe restores the window full screen, so the next
    /// start returns it to this mode instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics_fullscreen_from: Option<WindowMode>,
}

/// Whether a window was full screen, and whether it was maximized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowMode {
    pub fullscreen: bool,
    pub maximized: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackCheckpoint {
    pub current: crate::player::LocalTrack,
    pub context: Option<String>,
    pub tracks: Vec<crate::player::LocalTrack>,
    pub index: usize,
    pub queued: Vec<crate::player::LocalTrack>,
}

impl PlaybackCheckpoint {
    pub fn capture(
        local: &crate::player::LocalState,
        context: Option<String>,
        manual: &[String],
    ) -> Option<Self> {
        let current = local.track.clone()?;
        let mut tracks = if local.queue_tracks.len() <= 20_000 {
            local.queue_tracks.clone()
        } else {
            Vec::new()
        };
        let index = if tracks
            .get(local.queue_index)
            .is_some_and(|t| t.uri == current.uri)
        {
            local.queue_index
        } else {
            tracks
                .iter()
                .position(|t| t.uri == current.uri)
                .unwrap_or(0)
        };
        // The existing outbox restores manual rows separately. Remove only the
        // exact manual prefix, leaving repeated playlist occurrences intact.
        let start = index.saturating_add(1);
        let end = start.saturating_add(manual.len());
        let queued = if !manual.is_empty()
            && tracks
                .get(start..end)
                .is_some_and(|rows| rows.iter().map(|t| &t.uri).eq(manual.iter()))
        {
            tracks.drain(start..end).collect()
        } else {
            Vec::new()
        };
        Some(Self {
            current,
            context,
            tracks,
            index,
            queued,
        })
    }

    pub fn matches(&self, uri: &str, context: Option<&str>) -> bool {
        self.current.uri == uri
            && self.context.as_deref() == context
            && !self.current.title.is_empty()
            && self.tracks.len() <= 20_000
            && self.queued.len() <= 20_000
            && (self.tracks.is_empty() || self.tracks.get(self.index).is_some_and(|t| t.uri == uri))
    }
}

impl SessionState {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// The session as its file holds it.
    pub fn encode(&self) -> Option<String> {
        serde_json::to_string(self)
            .map_err(|error| log::warn!("unable to encode session: {error}"))
            .ok()
    }

    /// Writes the session and waits for the disk. The app saves through
    /// its [`fastframe_file::Writer`] instead.
    pub fn save(&self, path: &Path) {
        let Some(text) = self.encode() else { return };
        if let Err(error) = fastframe_file::write(path, text) {
            log::warn!("unable to save session to {}: {error}", path.display());
        }
    }
}

#[cfg(test)]
mod session_tests {
    use super::{CachedRootlist, SessionState};
    use crate::player::RootlistEntry;

    /// A session saved before the maximized state was remembered keeps its
    /// window size and positions, and reads as an ordinary window.
    #[test]
    fn old_sessions_keep_their_window_geometry() {
        let state: SessionState = serde_json::from_str(
            r#"{"window_size":[1100.0,700.0],"window_pos":[100.0,150.0],"winamp_pos":[300.0,200.0]}"#,
        )
        .unwrap();
        assert_eq!(state.window_size, Some([1100.0, 700.0]));
        assert_eq!(state.window_pos, Some([100.0, 150.0]));
        assert!(!state.window_maximized);
        assert_eq!(state.winamp_pos, Some([300.0, 200.0]));
    }

    #[test]
    fn old_sessions_without_a_playlist_tree_remain_readable() {
        let state: SessionState = serde_json::from_str(r#"{"last_page":"home"}"#).unwrap();
        assert_eq!(state.last_page.as_deref(), Some("home"));
        assert_eq!(state.rootlist, None);
        assert_eq!(
            crate::player::RepeatMode::from_api(&state.repeat),
            crate::player::RepeatMode::Off,
            "no saved repeat mode is off"
        );
    }

    #[test]
    fn the_repeat_mode_round_trips() {
        let state = SessionState {
            repeat: crate::player::RepeatMode::Track.api_name().into(),
            ..SessionState::default()
        };
        let json = serde_json::to_string(&state).unwrap();
        let read = serde_json::from_str::<SessionState>(&json).unwrap();
        assert_eq!(
            crate::player::RepeatMode::from_api(&read.repeat),
            crate::player::RepeatMode::Track
        );
    }

    #[test]
    fn the_playlist_tree_round_trips_with_its_account() {
        let state = SessionState {
            rootlist: Some(CachedRootlist {
                account_id: "listener".into(),
                entries: vec![
                    RootlistEntry::FolderStart {
                        id: "folder".into(),
                        name: "Favorites".into(),
                    },
                    RootlistEntry::Playlist("spotify:playlist:one".into()),
                    RootlistEntry::FolderEnd,
                ],
            }),
            ..SessionState::default()
        };

        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(serde_json::from_str::<SessionState>(&json).unwrap(), state);
    }

    #[test]
    fn a_new_session_atomically_replaces_the_previous_one() {
        let root = std::env::temp_dir().join(format!(
            "spotifast-session-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let path = root.join("session.json");
        let state = |page: &str| SessionState {
            last_page: Some(page.into()),
            ..SessionState::default()
        };

        state("home").save(&path);
        state("liked").save(&path);

        assert_eq!(
            SessionState::load(&path).last_page.as_deref(),
            Some("liked")
        );
        assert!(crate::util::temporary_files(path.parent().unwrap()).is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
}
