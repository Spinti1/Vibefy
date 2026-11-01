use crate::{addons::Collection, app::App, backend::Command};

impl App {
    pub(super) fn tick_playlist_mirror(&mut self) {
        let observed = self
            .user_id()
            .filter(|_| self.music_accounts[0].connected && !self.music_accounts[0].busy)
            .map(|id| (id.to_owned(), self.music_accounts_serial[0]));
        if observed == self.mirror_observed {
            return;
        }
        self.mirror_observed = observed;
        self.mirror_serial = self.mirror_serial.wrapping_add(1);
        self.mirror = Default::default();
        self.mirror_retry_confirm = false;
        self.addons.collections_revision = self.addons.collections_revision.wrapping_add(1);
        if self.mirror_observed.is_some() {
            self.backend.send(Command::Mirror {
                serial: self.mirror_serial,
                action: if std::mem::take(&mut self.music_accounts[0].enable_sync_after_connect) {
                    crate::playlist_mirror::Action::Enable(true)
                } else {
                    crate::playlist_mirror::Action::Observe
                },
            });
        } else {
            self.backend.send(Command::StopMirror);
        }
    }
    pub fn source_collection(&self, source: &str) -> Option<&Collection> {
        self.mirror
            .lists
            .iter()
            .map(|list| &list.collection)
            .find(|c| c.source == source)
            .or_else(|| self.addons.collections.iter().find(|c| c.source == source))
    }
    pub(super) fn play_mirror_context(
        &mut self,
        uri: &str,
        offset_uri: Option<&str>,
        offset_index: Option<u32>,
    ) -> bool {
        let Some(id) = uri.strip_prefix("spotify:playlist:") else {
            return false;
        };
        let Some(list) = self
            .mirror
            .lists
            .iter()
            .find(|list| list.spotify_id == id && !list.youtube_id.is_empty())
        else {
            return false;
        };
        if list.collection.tracks.is_empty() {
            return false;
        }
        let index = offset_index
            .or_else(|| {
                offset_uri
                    .and_then(|uri| list.source_uris.iter().position(|source| source == uri))
                    .map(|i| i as u32)
            })
            .unwrap_or(0);
        if index as usize >= list.collection.tracks.len() {
            return false;
        }
        let source = list.collection.source.clone();
        let uris = list
            .collection
            .tracks
            .iter()
            .map(|track| track.uri.clone())
            .collect::<Vec<_>>();
        self.play_source_view(
            &source,
            &uris,
            index,
            offset_uri.is_none() && offset_index.is_none(),
        );
        true
    }
}
