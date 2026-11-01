use super::*;
use crate::addons::{Command as AddonCommand, Id};

impl App {
    pub(super) fn receive_addon_result(
        &mut self,
        request: u64,
        result: Result<crate::addons::Collection, String>,
    ) {
        if request != self.addons.request {
            return;
        }
        self.addons.busy = false;
        if self.addons.fetching_soundcloud {
            self.addons.soundcloud_result = result.as_ref().ok().cloned();
            self.addons.soundcloud_message = result.as_ref().err().cloned();
            self.addons.fetching_soundcloud = false;
        }
        match result {
            Ok(collection) => {
                self.addons.message = None;
                if crate::addons::playlist_url(&collection.source).is_ok() {
                    self.save_source_collection(collection.clone());
                    self.addons.artwork_checked.remove(&collection.source);
                    self.start_source_artwork(&collection.source);
                }
                self.addons.result = Some(collection);
            }
            Err(error) => self.addons.message = Some(error),
        }
    }

    pub(super) fn apply_addon(&mut self, command: AddonCommand, ctx: &egui::Context) {
        match command {
            AddonCommand::CancelFetch => {
                self.addons.request = self.addons.request.wrapping_add(1);
                self.addons.busy = false;
                self.addons.fetching_soundcloud = false;
                self.backend.send(Command::AddonCancel);
            }
            AddonCommand::Enable(id, enabled) => {
                self.settings.addons.set(id, enabled);
                if enabled && matches!(id, Id::Vivy | Id::Backmusic) {
                    self.settings.addons.visualizer = Some(id);
                }
                if !enabled {
                    self.stop_addon(id, ctx);
                }
                self.settings_dirty = true;
            }
            AddonCommand::Uninstall(id) => {
                self.stop_addon(id, ctx);
                self.settings.addons.uninstall(id);
                self.settings_dirty = true;
            }
            AddonCommand::Fetch(query) => {
                let id = if matches!(query, crate::addons::Request::YouTube(_)) {
                    Id::YouTube
                } else {
                    Id::WaveLink
                };
                if !self.settings.addons.active(id) || self.addons.busy {
                    return;
                }
                self.addons.busy = true;
                self.addons.fetching_soundcloud =
                    matches!(query, crate::addons::Request::SoundCloud(_));
                if self.addons.fetching_soundcloud {
                    self.addons.soundcloud_result = None;
                    self.addons.soundcloud_message = None;
                }
                self.addons.request = self.addons.request.wrapping_add(1);
                self.addons.result = None;
                self.addons.message = None;
                self.backend.send(Command::AddonFetch {
                    request: self.addons.request,
                    query,
                });
            }
            AddonCommand::SaveCollection => {
                if let Some(collection) = self.addons.result.clone() {
                    self.save_source_collection(collection);
                }
            }
            AddonCommand::SaveSoundCloud => {
                if let Some(collection) = self.addons.soundcloud_result.clone() {
                    self.save_source_collection(collection);
                }
            }
            AddonCommand::DeleteCollection(index) => {
                if index < self.addons.collections.len() {
                    let removed = self.addons.collections.remove(index);
                    self.addons.collections_revision =
                        self.addons.collections_revision.wrapping_add(1);
                    self.save_addon_collections();
                    if self.page() == &Page::LocalCollection(removed.source) {
                        self.open(Page::Home);
                    }
                }
            }
            AddonCommand::Play { tracks, index } => {
                self.play_addon_tracks(tracks, index, 0, true);
            }
            AddonCommand::ChooseVisualizer(id) => {
                if self.settings.addons.installed.contains(&id)
                    && matches!(id, Id::Vivy | Id::Backmusic)
                {
                    self.settings.addons.set(id, true);
                    self.settings.addons.visualizer = Some(id);
                    self.settings_dirty = true;
                }
            }
            AddonCommand::Fullscreen => {
                if self.addons.fullscreen {
                    self.leave_addon_fullscreen(ctx);
                } else if let Some(id) = self
                    .settings
                    .addons
                    .visualizer
                    .filter(|id| self.settings.addons.installed.contains(id))
                    .or_else(|| self.settings.addons.installed_visualizers().next())
                {
                    self.settings.addons.visualizer = Some(id);
                    self.settings.addons.set(id, true);
                    self.settings_dirty = true;
                    self.addons.was_fullscreen =
                        ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    self.addons.fullscreen = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                }
            }
            AddonCommand::Style(value) => {
                let id = self.settings.addons.visualizer.unwrap_or(Id::Vivy);
                self.settings.addons.visual_mut(id).style = value.min(1);
                self.settings_dirty = true;
            }
            AddonCommand::Intensity(value) => {
                let id = self.settings.addons.visualizer.unwrap_or(Id::Vivy);
                self.settings.addons.visual_mut(id).intensity = if value.is_finite() {
                    value.clamp(0.2, 3.0)
                } else {
                    1.0
                };
                self.settings_dirty = true;
            }
            AddonCommand::Particles(value) => {
                let id = self.settings.addons.visualizer.unwrap_or(Id::Vivy);
                self.settings.addons.visual_mut(id).particles = value;
                self.settings_dirty = true;
            }
            AddonCommand::CatJamScale(value) => {
                if self.settings.addons.installed.contains(&Id::CatJam) {
                    self.settings.addons.cat_jam_scale = if value.is_finite() {
                        value.clamp(0.5, 2.0)
                    } else {
                        0.9
                    };
                    self.settings_dirty = true;
                }
            }
            AddonCommand::Host => {
                if self.addons.jam.is_none() {
                    self.stop_peek(true);
                    self.addons.jam_message = None;
                    self.addons.jam = Some(crate::jam::Session::start_host());
                    self.addons.pending_sync = None;
                }
            }
            AddonCommand::Join(invite) => {
                if self.addons.jam.is_some() {
                    return;
                }
                match crate::jam::Session::join(&invite) {
                    Ok(session) => {
                        self.stop_peek(true);
                        self.addons.jam = Some(session);
                        self.addons.pending_sync = None;
                        self.addons.jam_message = None;
                    }
                    Err(e) => self.addons.jam_message = Some(e.to_string()),
                }
            }
            AddonCommand::Leave => {
                self.addons.jam_message = None;
                self.addons.jam = None;
                self.addons.pending_sync = None;
            }
        }
    }
    fn stop_addon(&mut self, id: Id, ctx: &egui::Context) {
        if id == Id::TrackPeek {
            self.stop_peek(true);
        }
        if self.settings.addons.visualizer == Some(id) {
            self.settings.addons.visualizer = None;
            self.leave_addon_fullscreen(ctx);
        }
        if matches!(id, Id::YouTube | Id::WaveLink) {
            self.backend.send(Command::AddonCancel);
            self.addons.request = self.addons.request.wrapping_add(1);
            self.addons.busy = false;
            self.addons.fetching_soundcloud = false;
            if id == Id::WaveLink {
                self.addons.soundcloud_result = None;
                self.addons.soundcloud_message = None;
            }
            self.addons.result = None;
        }
    }
    pub(super) fn save_source_collection(&mut self, collection: crate::addons::Collection) -> bool {
        if let Some(old) = self
            .addons
            .collections
            .iter_mut()
            .find(|saved| saved.source == collection.source)
        {
            *old = collection;
        } else if self.addons.collections.len() >= 30 {
            self.addons.message = Some("Remove a saved playlist before adding another (limit: 30). Your import is still available below.".into());
            return false;
        } else {
            self.addons.collections.push(collection);
        }
        self.addons.collections_revision = self.addons.collections_revision.wrapping_add(1);
        self.save_addon_collections();
        self.addons.message = Some("Playlist saved on Home.".into());
        true
    }
    pub(super) fn save_addon_collections(&mut self) {
        if self.offline {
            return;
        }
        if let Ok(text) = serde_json::to_string(&self.addons.collections) {
            self.files.write(
                &self.dirs.state.join("mod-collections.json"),
                text,
                fastframe_file::Access::Default,
            );
        }
    }
    pub(super) fn leave_addon_fullscreen(&mut self, ctx: &egui::Context) {
        if self.addons.fullscreen {
            self.addons.fullscreen = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                self.addons.was_fullscreen,
            ));
        }
    }
    fn play_addon_tracks(
        &mut self,
        tracks: Vec<crate::player::LocalTrack>,
        index: u32,
        position_ms: u32,
        play: bool,
    ) {
        self.load_addon_tracks(tracks, index, position_ms, play, None);
    }

    /// Native source rows retain their sorted/filtered order and repeated videos.
    pub(crate) fn play_source_view(
        &mut self,
        source: &str,
        uris: &[String],
        index: u32,
        header: bool,
    ) {
        let Some(saved) = self.source_collection(source) else {
            return;
        };
        if uris.is_empty() || uris.len() > 20_000 {
            return;
        }
        let Some(tracks) = uris
            .iter()
            .map(|uri| saved.tracks.iter().find(|track| &track.uri == uri).cloned())
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        let index = if header && self.shuffle_wanted {
            rand::random_range(0..uris.len()) as u32
        } else {
            index
        };
        let Some(uri) = uris.get(index as usize).cloned() else {
            return;
        };
        let context = Page::LocalCollection(source.into()).encode();
        if self.load_addon_tracks(tracks, index, 0, true, Some(context.clone())) {
            self.played_row = Some(PlayedRow {
                context: context.clone(),
                view: true,
                index,
                uri,
            });
            self.assumed_context = Some(AssumedContext {
                uri: context,
                shuffle: Some(self.shuffle_wanted),
                at: Instant::now(),
            });
        } else if let Some(message) = self.addons.message.clone() {
            self.toast(message);
        }
    }

    fn load_addon_tracks(
        &mut self,
        tracks: Vec<crate::player::LocalTrack>,
        index: u32,
        position_ms: u32,
        play: bool,
        context: Option<String>,
    ) -> bool {
        if !self.local_ready {
            self.addons.message =
                Some("Local playback is still connecting. Try again in a moment.".into());
            return false;
        }
        let index = index as usize;
        if index >= tracks.len() || tracks.len() > 20_000 {
            return false;
        }
        if tracks.iter().any(|t| {
            !(crate::jam::Snapshot {
                track: t.clone(),
                position_ms: 0,
                playing: false,
            })
            .valid()
        }) {
            self.addons.message = Some("Unsupported source metadata.".into());
            return false;
        }
        self.supersede_play_search();
        self.queued_play = None;
        self.played_row = None;
        self.addons.active_tracks = tracks.clone();
        self.addons.active_collection = context.clone();
        self.addons.last_play_attempt = Some(Instant::now());
        self.backend.player(crate::player::PlayerCommand::Load(
            crate::player::LoadSpec {
                play,
                autoplay: true,
                position_ms,
                uris: tracks.iter().map(|t| t.uri.clone()).collect(),
                offset_index: Some(index as u32),
                track: Some(tracks[index].clone()),
                queue_tracks: tracks,
                shuffle: Some(context.is_some() && self.shuffle_wanted),
                repeat: Some(if context.is_some() {
                    self.local.repeat
                } else {
                    crate::player::RepeatMode::Off
                }),
                context_uri: context,
                ..Default::default()
            },
        ));
        true
    }
    pub(super) fn tick_addons(&mut self, ctx: &egui::Context) {
        self.tick_mod_tempo();
        if self.addons.fullscreen && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.leave_addon_fullscreen(ctx);
        }
        let Some(session) = &self.addons.jam else {
            return;
        };
        ctx.request_repaint_after(Duration::from_millis(100));
        if session.host {
            let now = self.now_playing().filter(|n| n.local && !n.loading);
            let snapshot = now.and_then(|n| {
                let mut track = self.local.track.clone()?;
                if let Some(source) = &self.local.source {
                    if crate::addons::valid_video_id(&source.youtube_id) {
                        track.uri = format!("vibefy:youtube:{}", source.youtube_id);
                    }
                }
                Some(crate::jam::Snapshot {
                    track,
                    position_ms: n.position_ms,
                    playing: n.playing,
                })
            });
            session.publish(snapshot);
            return;
        }
        if let Some(received) = session.take() {
            self.addons.pending_sync = Some(received);
        }
        let (connected, waiting) = {
            let state = session.shared.lock().unwrap();
            (state.connected, state.host_waiting)
        };
        if !connected || waiting {
            if self.addons.pending_sync.is_some()
                && self
                    .now_playing()
                    .is_some_and(|n| n.local && n.playing && !n.loading)
            {
                self.backend.player(crate::player::PlayerCommand::Toggle);
            }
            if !connected {
                self.addons.pending_sync = None;
            }
            return;
        }
        let Some((snapshot, at)) = self.addons.pending_sync.clone() else {
            return;
        };
        if at.elapsed() > Duration::from_secs(15) {
            self.addons.pending_sync = None;
            return;
        }
        let target = if snapshot.playing {
            snapshot
                .position_ms
                .saturating_add(at.elapsed().as_millis() as u32)
        } else {
            snapshot.position_ms
        };
        let now = self.now_playing();
        if now.as_ref().is_none_or(|n| n.uri != snapshot.track.uri) {
            if !snapshot.playing {
                if now
                    .as_ref()
                    .is_some_and(|n| n.local && n.playing && !n.loading)
                {
                    self.backend.player(crate::player::PlayerCommand::Toggle);
                }
                return;
            }
            if self
                .addons
                .active_tracks
                .first()
                .is_none_or(|t| t.uri != snapshot.track.uri)
                || !self.local.loading
            {
                self.play_addon_tracks(vec![snapshot.track], 0, target, snapshot.playing);
            }
            return;
        }
        let now = now.unwrap();
        if now.loading {
            return;
        }
        if now.playing != snapshot.playing {
            if snapshot.playing
                && self.local.error.is_some()
                && self
                    .addons
                    .last_play_attempt
                    .is_some_and(|at| at.elapsed() < Duration::from_secs(20))
            {
                return;
            }
            self.backend.player(crate::player::PlayerCommand::Toggle);
            self.addons.last_play_attempt = Some(Instant::now());
        }
        if now.position_ms.abs_diff(target) > 350
            && self
                .addons
                .last_sync
                .is_none_or(|t| t.elapsed() > Duration::from_secs(2))
        {
            self.backend
                .player(crate::player::PlayerCommand::Seek(target));
            self.addons.last_sync = Some(Instant::now());
        }
    }
    fn tick_mod_tempo(&mut self) {
        let enabled =
            self.settings.addons.active(Id::Nyan) || self.settings.addons.active(Id::CatJam);
        let source = if enabled {
            self.local.source.as_ref().map(|s| s.youtube_id.clone())
        } else {
            None
        };
        if source == self.addons.tempo_recording {
            return;
        }
        self.backend.send(Command::AddonCancelTempo);
        self.addons.tempo_recording = source.clone();
        self.addons.tempo_bpm = None;
        self.addons.tempo_pending = false;
        let Some(recording) = source else {
            return;
        };
        if let Some(bpm) = self.addons.tempo_cache.get(&recording) {
            self.addons.tempo_bpm = *bpm;
            return;
        }
        if !(crate::addons::valid_video_id(&recording)
            || (recording.len() == 40 && recording.bytes().all(|b| b.is_ascii_hexdigit())))
        {
            return;
        }
        self.addons.tempo_pending = true;
        self.backend.send(Command::AddonTempo {
            file: self.dirs.audio_cache_dir().join(format!("{recording}.m4a")),
            recording,
        });
    }
}
