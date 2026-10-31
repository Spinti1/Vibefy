use super::widgets;
use crate::{
    app::App,
    model::Action,
    theme::{self, Icon},
};

pub fn adjacent(response: egui::Response, app: &App, next: bool) -> egui::Response {
    if !app.settings.addons.active(crate::addons::Id::TrackPeek) {
        return response;
    }
    if app.now_playing().is_some_and(|now| !now.local) {
        return response;
    }
    if next && app.local.shuffle {
        return response.on_hover_text("Shuffle selects the next track when you skip.");
    }
    let index = if next {
        app.local.queue_index.checked_add(1).and_then(|index| {
            if index < app.local.queue_tracks.len() {
                Some(index)
            } else if app.local.repeat == crate::player::RepeatMode::Context {
                Some(0)
            } else {
                None
            }
        })
    } else {
        app.local.queue_index.checked_sub(1)
    };
    let track = index.and_then(|i| app.local.queue_tracks.get(i));
    if let Some(t) = track {
        response.on_hover_ui(|ui| {
            ui.set_width(240.0);
            ui.small(if next { "Up next" } else { "Previous track" });
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(60.0), egui::Sense::hover());
                widgets::paint_cover(
                    ui,
                    &app.palette,
                    t.cover_url.as_deref().or(t.art_url.as_deref()),
                    rect,
                    5.0,
                    Icon::Disc,
                    Some(app.backend.art()),
                );
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&t.title).strong());
                    ui.label(t.artist_names());
                });
            });
        })
    } else {
        response
    }
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
