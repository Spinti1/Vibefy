//! Built-in private listening sessions. No socket opens until Host or Join.
use crate::addons::Command;
use crate::app::App;
use crate::model::Action;
use crate::theme::{self, Icon};
use egui::{Frame, Stroke};

fn button(app: &mut App, ui: &mut egui::Ui, text: &str, command: Command, active: bool) {
    if theme::soft_button(ui, &app.palette, None, text, active).clicked() {
        app.actions.push(Action::Addon(command));
    }
}

pub fn dialog(app: &mut App, ctx: &egui::Context) {
    if !app.show_listen_together {
        return;
    }
    let p = app.palette;
    let frame = Frame::new()
        .fill(p.overlay)
        .stroke(Stroke::new(1.0, p.outline))
        .corner_radius(20)
        .inner_margin(24.0)
        .shadow(egui::epaint::Shadow {
            offset: [0, 10],
            blur: 40,
            spread: 0,
            color: p.shadow,
        });
    let mut close = false;
    let response = egui::Modal::new(egui::Id::new("listen-together-dialog"))
        .frame(frame)
        .backdrop_color(egui::Color32::from_black_alpha(140))
        .show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 96.0).clamp(220.0, 440.0));
            // Area starts with a short default height. Give the scroll region
            // actual viewport space instead of trapping it in that first size.
            ui.set_max_height((ctx.viewport_rect().height() - 120.0).max(180.0));
            ui.spacing_mut().item_spacing.y = 10.0;
            egui::Sides::new().shrink_left().show(
                ui,
                |ui| {
                    ui.vertical(|ui| {
                        theme::text(ui, "LISTEN TOGETHER", theme::bold(11.0), p.accent);
                        theme::text(ui, "Your music. Shared.", theme::bold(24.0), p.text);
                    });
                },
                |ui| {
                    close = theme::icon_button(ui, Icon::X, 18.0, p.secondary, p.text, "Close")
                        .clicked();
                },
            );
            ui.label("One queue, wherever you are. Invite friends on desktop or in their browser.");
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt("listen-together-dialog-scroll")
                .max_height((ctx.viewport_rect().height() - 280.0).max(100.0))
                .auto_shrink([false, true])
                .show(ui, |ui| controls(app, ui));
        });
    if close || response.should_close() {
        app.actions.push(Action::ToggleListenTogether);
    }
}

fn controls(app: &mut App, ui: &mut egui::Ui) {
    if let Some(message) = &app.addons.jam_message {
        ui.colored_label(app.palette.warning, message);
        ui.add_space(8.0);
    }
    if let Some(session) = &app.addons.jam {
        let state = session.shared.lock().unwrap();
        Frame::new()
            .fill(app.palette.accent.gamma_multiply(0.08))
            .stroke(Stroke::new(1.0, app.palette.accent.gamma_multiply(0.25)))
            .corner_radius(12)
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.colored_label(app.palette.accent, &state.status);
                ui.small(format!(
                    "{} listener(s) · shared queue & seeking",
                    state.listeners
                ));
            });
        if session.host && !state.invite.is_empty() {
            let mut invite = state.invite.clone();
            if let Ok(code) = crate::jam::parse_invite(&invite) {
                ui.label(
                    egui::RichText::new(format!("{}  {}  {}", &code[..4], &code[4..8], &code[8..]))
                        .size(24.0)
                        .strong()
                        .monospace(),
                );
            }
            ui.small("INVITE YOUR FRIENDS");
            ui.add(
                egui::TextEdit::singleline(&mut invite)
                    .desired_width(ui.available_width())
                    .interactive(false),
            );
            if theme::soft_button(ui, &app.palette, Some(Icon::Copy), "Copy invitation", false)
                .clicked()
            {
                ui.ctx().copy_text(invite);
            }
            ui.small("Send the link or code privately. Everyone streams on their own device.");
        }
        if !state.queue.is_empty() {
            ui.add_space(12.0);
            ui.strong("Up next");
            for (i, track) in state.queue.iter().take(12).enumerate() {
                let title = format!("{}. {} · {}", i + 1, track.title, track.artist_names());
                if ui
                    .add_enabled(state.controls_available, egui::Button::new(title).wrap())
                    .on_hover_text("Play this queued song for everyone")
                    .clicked()
                {
                    app.actions.push(Action::Addon(Command::JamPlayQueue {
                        index: i,
                        uri: track.uri.clone(),
                    }));
                }
            }
        }
        if !session.host {
            ui.small("Add a YouTube or JuiceVault song to the shared queue. Select a queued song to play it for everyone.");
        }
        drop(state);
        if session.host && !app.addons.jam_monitor_open {
            button(app, ui, "Show Jam status", Command::JamMonitor(true), false);
        }
        button(app, ui, "Leave session", Command::Leave, false);
    } else {
        let p = app.palette;
        Frame::new()
            .fill(p.accent.gamma_multiply(0.07))
            .stroke(Stroke::new(1.0, p.accent.gamma_multiply(0.22)))
            .corner_radius(12)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.strong("Start a Jam");
                ui.small("Choose the music, then share your invitation.");
                button(app, ui, "Start listening together", Command::Host, true);
            });
        ui.add_space(4.0);
        ui.strong("Have an invitation?");
        ui.add(
            egui::TextEdit::singleline(&mut app.addons.jam_invite)
                .hint_text("Paste a link or 12-character code")
                .desired_width(ui.available_width())
                .char_limit(200),
        );
        let invite = app.addons.jam_invite.clone();
        if ui
            .add_enabled(
                !invite.trim().is_empty(),
                egui::Button::new("Join session").min_size(egui::vec2(ui.available_width(), 36.0)),
            )
            .clicked()
        {
            app.actions.push(Action::Addon(Command::Join(invite)));
        }
    }
    ui.add_space(6.0);
    ui.separator();
    ui.small("Up to 24 listeners · rooms last 6 hours. Cloudflare relays the session; listeners never connect directly to your device.");
}

/// An egui overlay, confined to this application. Never an OS topmost window.
pub fn host_monitor(app: &mut App, ctx: &egui::Context) {
    if !app.addons.jam_monitor_open {
        return;
    }
    let Some(session) = app.addons.jam.as_ref().filter(|s| s.host) else {
        return;
    };
    let state = session.shared.lock().unwrap();
    let connected = state.connected;
    let listeners = state.listeners;
    let latency = state.latency_ms;
    let stale = state.last_seen.is_none_or(|at| at.elapsed().as_secs() > 15);
    let status = state.status.clone();
    let playback = state.playback_notice();
    drop(state);
    let mut open = true;
    egui::Window::new("Jam · Host")
        .id(egui::Id::new("vibefy-jam-host-monitor"))
        .open(&mut open)
        .order(egui::Order::Foreground)
        .resizable(false)
        .collapsible(false)
        .movable(true)
        .frame(
            Frame::new()
                .fill(app.palette.overlay)
                .stroke(Stroke::new(1.0, app.palette.outline))
                .corner_radius(14)
                .inner_margin(12.0)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 18,
                    spread: 0,
                    color: app.palette.shadow,
                }),
        )
        .default_width(224.0)
        .default_pos(egui::pos2(
            (ctx.viewport_rect().right() - 290.0).max(8.0),
            90.0,
        ))
        .constrain_to(ctx.viewport_rect().shrink(8.0))
        .show(ctx, |ui| {
            let healthy = connected && !stale;
            ui.horizontal(|ui| {
                ui.colored_label(
                    if healthy {
                        app.palette.accent
                    } else {
                        app.palette.warning
                    },
                    if healthy {
                        "● Live"
                    } else {
                        "● Reconnecting…"
                    },
                )
                .on_hover_text("Drag the title to move this panel. It stays inside Vibefy.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small(format!("{listeners} listening"));
                });
            });
            ui.separator();
            ui.small(playback);
            ui.horizontal(|ui| {
                ui.small(format!("{latency} ms")).on_hover_text(
                    "Round-trip connection latency, not the difference between players",
                );
                if theme::soft_button(ui, &app.palette, None, "Open Jam", false).clicked()
                    && !app.show_listen_together
                {
                    app.actions.push(Action::Addon(Command::JamDetails));
                }
            });
            if latency > 500 {
                ui.colored_label(app.palette.warning, "Network delay is elevated");
            }
            if !healthy {
                ui.small(status);
            }
        });
    if !open {
        app.actions.push(Action::Addon(Command::JamMonitor(false)));
    }
}
