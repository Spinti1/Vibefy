// Run provider unit tests separately from the desktop application's test harness.
#[path = "../src/lyrics.rs"]
mod lyrics;
mod model {
    pub use spotifast::model::Loadable;
}
#[path = "../src/spicy_lyrics.rs"]
mod spicy_lyrics;
