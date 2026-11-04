use super::widgets;
use crate::{
    app::App,
    model::Action,
    theme::{self, Icon},
};

/// Permanent neighbours beside the transport, with no network work during drawing.
pub fn transport_cards(app: &App, ui: &mut egui::Ui, region: egui::Rect, cy: f32, controls: f32) {
    if !app.settings.addons.active(crate::addons::Id::TrackPeek) {
        return;
    }
    super::mod_host::render(
        app,
        ui,
        crate::addons::Id::TrackPeek,
        region,
        &[(23, controls), (24, cy)],
    );
}
pub(super) fn paint_neighbour(app: &App, ui: &egui::Ui, rect: egui::Rect, next: bool) {
    let track = neighbour(app, next);
    let fallback = if next && app.local.shuffle {
        "Shuffle"
    } else if next {
        "End of queue"
    } else {
        "No previous track"
    };
    let title = track.as_ref().map(|t| t.title.as_str()).unwrap_or(fallback);
    let p = app.palette;
    ui.painter().rect_filled(rect, 21.0, p.surface);
    let width = rect.width();
    let cy = rect.center().y;
    let cover_size = if width >= 100.0 {
        34.0
    } else if width >= 52.0 {
        24.0
    } else {
        0.0
    };
    let cover_x = if next {
        rect.right() - cover_size - 4.0
    } else {
        rect.left() + 4.0
    };
    if cover_size > 0.0 {
        let cover = egui::Rect::from_min_size(
            egui::pos2(cover_x, cy - cover_size / 2.0),
            egui::Vec2::splat(cover_size),
        );
        widgets::paint_cover(
            ui,
            &p,
            track
                .as_ref()
                .and_then(|t| t.cover_url.as_deref().or(t.art_url.as_deref())),
            cover,
            cover_size / 2.0,
            Icon::Music,
            Some(app.backend.art()),
        );
    }
    let left = rect.left() + 8.0 + if next { 0.0 } else { cover_size };
    let right = rect.right() - 8.0 - if next { cover_size } else { 0.0 };
    crate::bidi::paint_line(
        &ui.painter().with_clip_rect(egui::Rect::from_min_max(
            egui::pos2(left, rect.top()),
            egui::pos2(right, rect.bottom()),
        )),
        left,
        right,
        cy,
        title,
        theme::medium(13.0),
        p.secondary,
    );
    ui.interact(
        rect,
        ui.id().with(("track-peek-neighbour", next)),
        egui::Sense::hover(),
    )
    .on_hover_text(format!(
        "{}: {}",
        if next { "Up next" } else { "Previously played" },
        title
    ));
}
pub fn neighbour(app: &App, next: bool) -> Option<crate::player::LocalTrack> {
    if app.now_playing().is_some_and(|n| !n.local) {
        return None;
    }
    if !next {
        return app.local.previous_track.clone().or_else(|| {
            app.local
                .queue_index
                .checked_sub(1)
                .and_then(|i| app.local.queue_tracks.get(i).cloned())
        });
    }
    if let Some(uri) = app.manual_queue.first() {
        return app.find_track_metadata(uri).or_else(|| {
            app.local
                .queue_tracks
                .iter()
                .find(|track| &track.uri == uri)
                .cloned()
        });
    }
    if app.local.shuffle {
        return app.local.shuffle_next.clone();
    }
    let index = app.local.queue_index + 1;
    app.local
        .queue_tracks
        .get(index)
        .or_else(|| {
            if app.local.repeat == crate::player::RepeatMode::Context {
                app.local.queue_tracks.first()
            } else {
                None
            }
        })
        .cloned()
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(active) = &app.peek else { return };
    let p = app.palette;
    let mut close = false;
    let mut open = true;
    egui::Window::new("Track Peek")
        .open(&mut open)
        .id(egui::Id::new("track-peek-preview"))
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .anchor(
            egui::Align2::RIGHT_BOTTOM,
            egui::vec2(-20.0, -(theme::PLAYER_BAR_HEIGHT + 20.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(66.0), egui::Sense::hover());
                widgets::paint_cover(
                    ui,
                    &p,
                    active.session.track.cover_url.as_deref(),
                    rect,
                    6.0,
                    Icon::Disc,
                    Some(app.backend.art()),
                );
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&active.session.track.title).strong());
                    ui.label(active.session.track.artist_names());
                    ui.small(match active.session.status() {
                        crate::peek::Status::Loading => "Preparing preview…",
                        _ => "30-second preview",
                    });
                });
            });
            ui.add_space(8.0);
            close = theme::soft_button(ui, &p, Some(Icon::X), "Stop preview", false).clicked();
        });
    if close || !open {
        app.actions.push(Action::StopPreview);
    }
}
