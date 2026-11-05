use super::*;
use crate::music::{Filter, Results};

impl App {
    pub(super) fn run_music_search(&mut self, query: String, force: bool) {
        let filter = self.search.music.filter;
        if !force
            && query == self.search.committed
            && !self.search.music.loading
            && self.search.music.error.is_none()
            && !self.search.music.results.entries.is_empty()
            && self.search.music.source_key.starts_with("ytmusic:search:")
        {
            return;
        }
        self.search.serial = self.search.serial.wrapping_add(1);
        self.search.committed = query.clone();
        self.search.error = None;
        self.search.playlists_pending = false;
        self.search.catalogue_pending = false;
        self.search.music.source_key = format!("ytmusic:search:{}:{query}", filter.label());
        self.search.music.page_title.clear();
        self.search.music.error = None;
        self.search.music.loading_more = false;
        self.search.music.results = Default::default();
        self.search.music.loading = !query.is_empty();
        let cached = (!query.is_empty())
            .then(|| self.search.music.cached(&query, filter))
            .flatten();
        if let Some(results) = cached {
            self.backend.send(Command::MusicSearch {
                serial: self.search.serial,
                query: String::new(),
                filter,
                continuation: None,
            });
            self.search.music.results = results;
            self.search.music.loading = false;
            self.search.results_serial = self.search.serial;
        } else {
            self.backend.send(Command::MusicSearch {
                serial: self.search.serial,
                query,
                filter,
                continuation: None,
            });
        }
    }
    pub(super) fn receive_music_search(
        &mut self,
        serial: u64,
        query: String,
        filter: Filter,
        more: bool,
        result: Result<Results, String>,
    ) {
        if serial != self.search.serial
            || query != self.search.committed
            || filter != self.search.music.filter
        {
            return;
        }
        self.search.music.loading = false;
        self.search.music.loading_more = false;
        match result {
            Ok(mut results) => {
                self.enrich_music_cached(&mut results);
                if more {
                    let existing: HashSet<_> = self
                        .search
                        .music
                        .results
                        .entries
                        .iter()
                        .map(|e| e.id.clone())
                        .collect();
                    self.search.music.results.entries.extend(
                        results
                            .entries
                            .into_iter()
                            .filter(|e| !existing.contains(&e.id)),
                    );
                    self.search.music.results.entries.truncate(500);
                    self.search.music.results.continuation = results.continuation;
                } else {
                    self.search.music.results = results;
                }
                if self.search.music.results.entries.len() >= 500 {
                    self.search.music.results.continuation = None;
                }
                self.search.music.error = None;
                self.search.results_serial = serial;
                let results = self.search.music.results.clone();
                self.search.music.remember(&query, filter, &results);
            }
            Err(error) => self.search.music.error = Some(error),
        }
    }
    /// Small, cache-only Spotify assist. No catalogue request, grant or waiting.
    fn enrich_music_cached(&self, results: &mut Results) {
        for entry in &mut results.entries {
            entry.spotify_note = None;
            if entry.kind == crate::music::Kind::Artist {
                let artist = self
                    .library
                    .artists
                    .items
                    .iter()
                    .chain(self.artist_pages.values().filter_map(|p| p.artist.get()))
                    .find(|a| a.name.eq_ignore_ascii_case(&entry.title));
                if let Some(artist) = artist {
                    entry.image = crate::api::models::pick_image(&artist.images, 300)
                        .map(str::to_owned)
                        .or_else(|| entry.image.clone());
                    entry.spotify_note = artist
                        .followers
                        .as_ref()
                        .map(|f| format!("{} Spotify followers (cached)", f.total));
                }
                continue;
            }
            let Some(local) = &mut entry.track else {
                continue;
            };
            // A title alone is never evidence of a matching recording.
            let matched: Vec<_> = self
                .track_cache
                .values()
                .filter(|t| {
                    t.name.eq_ignore_ascii_case(&local.title)
                        && t.artist_names().eq_ignore_ascii_case(&local.artist_names())
                        && local.duration_ms > 0
                        && t.duration_ms.abs_diff(local.duration_ms) <= 3000
                        && t.album.as_ref().is_some_and(|a| {
                            !local.album.is_empty() && a.name.eq_ignore_ascii_case(&local.album)
                        })
                })
                .take(2)
                .collect();
            if matched.len() == 1 {
                if let Some(image) = matched[0].image(300) {
                    entry.image = Some(image.into());
                    local.cover_url = Some(image.into());
                    local.art_url = Some(image.into());
                }
                entry.spotify_note = self
                    .play_counts
                    .get(&matched[0].uri)
                    .map(|count| format!("{count} Spotify plays (cached)"));
            }
        }
    }
    pub(super) fn more_music(&mut self) {
        if self.search.music.loading
            || self.search.music.loading_more
            || self.search.music.source_key.is_empty()
        {
            return;
        }
        let Some(continuation) = self.search.music.results.continuation.clone() else {
            return;
        };
        self.search.music.loading_more = true;
        self.search.music.error = None;
        if let Some(id) = self
            .search
            .music
            .source_key
            .strip_prefix("https://music.youtube.com/browse/")
        {
            self.backend.send(Command::MusicBrowse {
                serial: self.search.serial,
                id: id.into(),
                continuation: Some(continuation),
            });
            return;
        }
        self.backend.send(Command::MusicSearch {
            serial: self.search.serial,
            query: self.search.committed.clone(),
            filter: self.search.music.filter,
            continuation: Some(continuation),
        });
    }
    pub(super) fn open_music_page(&mut self, id: String) {
        if !crate::music::valid_browse_id(&id) {
            return;
        }
        let title = self
            .search
            .music
            .results
            .entries
            .iter()
            .find(|e| e.browse_id.as_deref() == Some(id.as_str()))
            .map(|e| e.title.clone())
            .unwrap_or_else(|| id.clone());
        self.search
            .music
            .pages
            .entry(id.clone())
            .or_insert((title, Results::default()));
        self.open(Page::MusicBrowse(id));
    }
    pub(super) fn restore_music_page(&mut self, id: String) {
        let source = format!("https://music.youtube.com/browse/{id}");
        if self.search.music.source_key == source && self.search.music.loading {
            return;
        }
        self.search.serial = self.search.serial.wrapping_add(1);
        let (title, results) = self
            .search
            .music
            .pages
            .get(&id)
            .cloned()
            .unwrap_or_else(|| (id.clone(), Results::default()));
        self.search.music.page_title = title;
        self.search.music.source_key = source;
        self.search.music.loading = results.entries.is_empty() && results.profile.is_none();
        self.search.music.loading_more = false;
        self.search.music.error = None;
        self.search.music.results = results;
        if self.search.music.loading {
            self.backend.send(Command::MusicBrowse {
                serial: self.search.serial,
                id,
                continuation: None,
            });
        }
    }
    pub(super) fn receive_music_browse(
        &mut self,
        serial: u64,
        id: String,
        more: bool,
        result: Result<Results, String>,
    ) {
        if serial != self.search.serial
            || self.search.music.source_key != format!("https://music.youtube.com/browse/{id}")
        {
            return;
        }
        self.search.music.loading = false;
        self.search.music.loading_more = false;
        match result {
            Ok(results) => {
                if let Some(profile) = &results.profile {
                    self.search.music.page_title = profile.title.clone();
                }
                if more {
                    self.search.music.results.entries.extend(results.entries);
                    self.search.music.results.entries.truncate(500);
                    self.search.music.results.continuation = results.continuation;
                } else {
                    self.search.music.results = results;
                }
                // Keep only visited pages. This bounds memory by the navigation history.
                self.search.music.pages.retain(|key, _| {
                    self.history
                        .iter()
                        .any(|p| matches!(p, Page::MusicBrowse(v) if v == key))
                });
                self.search.music.pages.insert(
                    id.clone(),
                    (
                        self.search.music.page_title.clone(),
                        self.search.music.results.clone(),
                    ),
                );
                if self.search.music.results.entries.len() >= 500 {
                    self.search.music.results.continuation = None;
                }
                self.search.music.error = None;
            }
            Err(error) => self.search.music.error = Some(error),
        }
    }
    pub(super) fn save_music_results(&mut self) {
        let tracks = self
            .search
            .music
            .results
            .entries
            .iter()
            .filter_map(|entry| entry.track.clone())
            .take(500)
            .collect::<Vec<_>>();
        if tracks.is_empty() {
            return;
        }
        let collection = crate::addons::Collection {
            title: if self.search.music.page_title.is_empty() {
                self.search.committed.clone()
            } else {
                self.search.music.page_title.clone()
            },
            source: self.search.music.source_key.clone(),
            image: tracks.first().and_then(|t| t.cover_url.clone()),
            owner: Some("YouTube Music".into()),
            tracks,
        };
        if self.save_source_collection(collection) {
            self.toast("Saved on Home.");
        }
    }
    pub(super) fn start_source_artwork(&mut self, source: &str) {
        if self.offline || self.addons.artwork_checked.contains(source) {
            return;
        }
        let Some(collection) = self.addons.collections.iter().find(|c| c.source == source) else {
            return;
        };
        let tracks = collection
            .tracks
            .iter()
            .filter(|track| {
                track.uri.starts_with("vibefy:youtube:")
                    && !track
                        .cover_url
                        .as_deref()
                        .is_some_and(crate::music::is_album_art)
            })
            .cloned()
            .collect::<Vec<_>>();
        if tracks.is_empty() {
            return;
        }
        if let Some(previous) = self.addons.artwork_active.replace(source.into()) {
            self.addons.artwork_checked.remove(&previous);
        }
        self.addons.artwork_checked.insert(source.into());
        self.backend.send(Command::MusicArtwork {
            source: source.into(),
            tracks,
        });
    }
    pub(super) fn receive_music_artwork(&mut self, source: String, uri: String, image: String) {
        if !crate::addons::safe_art_url(&image) {
            return;
        }
        let Some(collection) = self
            .addons
            .collections
            .iter_mut()
            .find(|c| c.source == source)
        else {
            return;
        };
        let mut changed = false;
        for track in &mut collection.tracks {
            if track.uri == uri && track.cover_url.as_deref() != Some(&image) {
                track.cover_url = Some(image.clone());
                track.art_url = Some(image.clone());
                track.art_small_url = Some(image.clone());
                changed = true;
            }
        }
        if !changed {
            return;
        }
        if collection
            .image
            .as_deref()
            .is_none_or(|url| url.contains("/vi/") || url.contains("/vi_webp/"))
        {
            collection.image = Some(image);
        }
        self.addons.collections_revision = self.addons.collections_revision.wrapping_add(1);
        self.save_addon_collections();
    }
    #[cfg(feature = "demo")]
    pub fn demo_music_response(
        &mut self,
        serial: u64,
        query: String,
        filter: Filter,
        more: bool,
        result: Result<Results, String>,
    ) {
        self.receive_music_search(serial, query, filter, more, result);
    }
    #[cfg(feature = "demo")]
    pub fn demo_music_artwork(&mut self, source: String, uri: String, image: String) {
        self.receive_music_artwork(source, uri, image);
    }
    #[cfg(feature = "demo")]
    pub fn demo_music_browse_response(
        &mut self,
        serial: u64,
        id: String,
        more: bool,
        result: Result<Results, String>,
    ) {
        self.receive_music_browse(serial, id, more, result);
    }
}
