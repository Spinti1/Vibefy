use super::*;
use std::sync::Arc;
impl App {
    pub fn peek_item(
        &self,
        item: &crate::api::models::PlayableItem,
    ) -> Option<crate::player::LocalTrack> {
        match item {
            crate::api::models::PlayableItem::Track(t) => {
                Some(self.convert_to_local_track(t, None))
            }
            _ => None,
        }
    }
    pub(super) fn start_peek(&mut self, track: crate::player::LocalTrack) {
        if !self.settings.addons.active(crate::addons::Id::TrackPeek) {
            return;
        }
        if !self.local_ready {
            self.toast("Connect local playback before previewing audio.");
            return;
        }
        if !matches!(self.target(), Target::Local) {
            self.toast("Select playback on this computer before previewing audio.");
            return;
        }
        if self.addons.jam.is_some() {
            self.toast("Leave Listen Together before previewing audio privately.");
            return;
        }
        // Retain the original resume point when another preview replaces this one.
        let resume = self.peek.take().and_then(|p| p.resume.clone()).or_else(|| {
            self.now_playing()
                .filter(|n| n.local && n.playing && !n.loading)
                .map(|n| (n.uri, self.local.track_sequence))
        });
        if resume.is_some() && self.believed_playing() {
            self.toggle_play();
        }
        let session = crate::peek::Session::new(track, self.local.volume);
        self.backend.send(Command::TrackPeek(Arc::clone(&session)));
        self.peek = Some(crate::peek::Active { session, resume });
    }
    pub(super) fn stop_peek(&mut self, resume: bool) {
        let Some(active) = self.peek.take() else {
            return;
        };
        active.session.cancel();
        if resume
            && active.resume.as_ref().is_some_and(|(uri, seq)| {
                self.local.track.as_ref().is_some_and(|t| &t.uri == uri)
                    && self.local.track_sequence == *seq
            })
            && !self.believed_playing()
        {
            self.toggle_play();
        }
    }
    pub(super) fn tick_peek(&mut self, ctx: &egui::Context) {
        let Some(active) = &self.peek else { return };
        if self.believed_playing()
            || active.resume.as_ref().is_some_and(|(uri, seq)| {
                self.local.track_sequence != *seq
                    || self.local.track.as_ref().is_none_or(|t| &t.uri != uri)
            })
        {
            self.stop_peek(false);
            return;
        }
        active.session.volume.store(
            u32::from(self.local.volume),
            std::sync::atomic::Ordering::Relaxed,
        );
        match active.session.status() {
            crate::peek::Status::Finished => self.stop_peek(true),
            crate::peek::Status::Failed(error) => {
                self.stop_peek(true);
                self.toast(error);
            }
            _ => {
                ctx.request_repaint_after(Duration::from_millis(150));
            }
        }
    }
}
