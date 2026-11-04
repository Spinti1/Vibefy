//! Local playlist visibility never changes the remote library.
use crate::app::App;
impl App {
    pub fn playlist_visibility_key(&self, source: &str) -> String {
        for list in &self.mirror.lists {
            if source == list.collection.source
                || source == format!("https://www.youtube.com/playlist?list={}", list.youtube_id)
            {
                return format!("spotify:playlist:{}", list.spotify_id);
            }
        }
        source.to_owned()
    }
    pub fn playlist_hidden(&self, source: &str) -> bool {
        self.settings
            .hidden_playlists
            .get(self.user_id().unwrap_or("local"))
            .is_some_and(|entries| entries.contains(&self.playlist_visibility_key(source)))
    }
}
