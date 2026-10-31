//! Built-in private listening sessions. No socket opens until Host or Join.
use crate::addons::Command;
use crate::app::App;
use crate::i18n::gettext;
use crate::model::Action;
use crate::theme::{self, Icon};
use egui::{Frame, Margin};

fn button(app: &mut App, ui: &mut egui::Ui, text: &str, command: Command, active: bool) {
    if theme::soft_button(ui, &app.palette, None, text, active).clicked() {
        app.actions.push(Action::Addon(command));
    }
}

pub fn side_panel(app: &mut App, ui: &mut egui::Ui) {
    // A docked panel would squeeze the page's search and navigation into
    // overlapping controls at the main window's minimum width.
    if ui.available_width() < theme::SIDE_PANEL_MIN_WIDTH + 360.0 {
        let ctx = ui.ctx().clone();
        let mut open = true;
        egui::Window::new("Listen Together")
            .id(egui::Id::new("listen-together-compact"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .max_width((ctx.viewport_rect().width() - 32.0).max(200.0))
            .max_height(
                (ctx.viewport_rect().height() - theme::PLAYER_BAR_HEIGHT - 100.0).max(200.0),
            )
            .anchor(
                egui::Align2::RIGHT_TOP,
                egui::vec2(-12.0, theme::top_bar_height() + 16.0),
            )
            .vscroll(true)
            .show(&ctx, |ui| controls(app, ui));
        if !open {
            app.actions.push(Action::ToggleListenTogether);
        }
        return;
    }
    let palette = app.palette;
    let locale = app.locale;
    let fit = super::yielding_panel(
        ui.ctx(),
        "listen-together-panel",
        theme::SIDE_PANEL_MIN_WIDTH..=560.0,
        app.settings.queue_width,
        ui.available_width() - super::topbar::least_width(ui.ctx()),
    );
    let panel = egui::Panel::right("listen-together-panel")
        .resizable(true)
        .default_size(app.settings.queue_width)
        .size_range(fit.range.clone())
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(12, 12)),
        );
    let response = panel.show(ui, |ui| {
        // The panel sits where the Queue would, under the same controls.
        let window_controls =
            super::window_controls_reservation(ui.ctx(), true, false, ui.available_width());
        ui.add_space(window_controls.queue_top);
        let mut close = false;
        egui::Sides::new().shrink_left().show(
            ui,
            |ui| {
                ui.add_space(4.0);
                theme::text(
                    ui,
                    gettext(locale, "Listen Together"),
                    theme::bold(16.0),
                    palette.text,
                );
            },
            |ui| {
                close = theme::icon_button(
                    ui,
                    Icon::X,
                    18.0,
                    palette.secondary,
                    palette.text,
                    &gettext(locale, "Close"),
                )
                .clicked();
            },
        );
        if close {
            app.actions.push(Action::ToggleListenTogether);
        }
        ui.add_space(12.0);
        egui::ScrollArea::vertical()
            .id_salt("listen-together-panel-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| controls(app, ui));
    });
    // The Listen Together panel shares the Queue panel's width, so switching
    // between the two never moves the page.
    let width = response.response.rect.width();
    if (width - app.settings.queue_width).abs() > 1.0
        && super::panel_width_chosen(ui.ctx(), "listen-together-panel", &fit)
    {
        app.settings.queue_width = width;
        app.actions.push(Action::SettingsChanged);
    }
}

fn controls(app: &mut App, ui: &mut egui::Ui) {
    if let Some(message) = &app.addons.jam_message {
        ui.colored_label(app.palette.warning, message);
        ui.add_space(8.0);
    }
    ui.small("Vibefy only · up to 8 listeners · host controls playback · trusted LAN or VPN. Direct sessions are not encrypted; use a VPN outside your home network. Closing the host ends the session.");
    if let Some(session) = &app.addons.jam {
        let state = session.shared.lock().unwrap();
        ui.label(format!(
            "{} · {} listener(s)",
            state.status, state.listeners
        ));
        if session.host && !state.invite.is_empty() {
            let mut invite = state.invite.clone();
            ui.label("Host IPv4 override (optional, for VPN / another network adapter)");
            ui.text_edit_singleline(&mut app.addons.jam_address);
            if !app.addons.jam_address.trim().is_empty() {
                if let (Ok(ip), Ok((address, token))) = (
                    app.addons.jam_address.trim().parse::<std::net::Ipv4Addr>(),
                    crate::jam::parse_invite(&invite),
                ) {
                    invite = format!("vibefy-jam://{}:{}/{token}", ip, address.port());
                } else {
                    ui.small("Enter a valid IPv4 address.");
                }
            }
            ui.label("Share this invitation with your listeners:");
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
            ui.small("If Windows asks, allow Vibefy on private networks. Other users paste this invitation into Listen Together.");
        }
        drop(state);
        button(app, ui, "Leave session", Command::Leave, false);
    } else {
        button(app, ui, "Host a session", Command::Host, true);
        ui.add_space(6.0);
        ui.label("Join a Vibefy host");
        ui.add(
            egui::TextEdit::singleline(&mut app.addons.jam_invite)
                .hint_text("vibefy-jam://IP:port/invitation")
                .desired_width(ui.available_width())
                .char_limit(200),
        );
        let invite = app.addons.jam_invite.clone();
        button(app, ui, "Join session", Command::Join(invite), false);
    }
}
