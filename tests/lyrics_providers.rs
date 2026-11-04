// Run provider unit tests separately from the desktop application's test harness.
pub use spotifast::{addons, api, audio_match, http, player};
#[path = "../src/lyrics.rs"]
mod lyrics;
#[path = "../src/music.rs"]
mod music;
mod model {
    pub use spotifast::model::Loadable;
}
#[path = "../src/spicy_lyrics.rs"]
mod spicy_lyrics;
