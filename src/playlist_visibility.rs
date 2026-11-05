//! Local playlist visibility never changes the remote library.
use crate::app::App;
impl App {
    pub fn playlist_hidden(&self, source: &str) -> bool {
        self.settings
            .hidden_playlists
            .get(self.user_id().unwrap_or("local"))
            .is_some_and(|entries| entries.contains(source))
    }
}
