mod api {
    pub mod client {
        pub use spotifast::api::client::{ApiError, Result};
    }
}
#[path = "../src/playlist_outbox.rs"]
mod playlist_outbox;
