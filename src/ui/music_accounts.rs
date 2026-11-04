use crate::{
    app::App,
    backend::Command,
    music_accounts::{Action, Provider},
    theme,
};
pub(super) fn send(app: &mut App, provider: Provider, action: Action) {
    let index = provider as usize;
    let start_sync = false;
    app.music_accounts_serial[index] = app.music_accounts_serial[index].wrapping_add(1);
    let state = &mut app.music_accounts[index];
    if matches!(action, Action::Connect | Action::Login(_)) {
        state.auth_pending = true;
        state.enable_sync_after_connect = start_sync;
    }
    state.busy = true;
    state.message = None;
    app.backend.send(Command::MusicAccount {
        selected: None,
        provider,
        action,
        username: state.username.clone(),
        serial: app.music_accounts_serial[index],
    });
}
fn cancel(app: &mut App, provider: Provider) {
    let index = provider as usize;
    app.music_accounts_serial[index] = app.music_accounts_serial[index].wrapping_add(1);
    let auth_pending = std::mem::take(&mut app.music_accounts[index].auth_pending);
    app.music_accounts[index].busy = auth_pending;
    app.music_accounts[index].enable_sync_after_connect = false;
    if auth_pending {
        app.backend.send(Command::MusicAccount {
            selected: None,
            provider,
            action: Action::Disconnect,
            username: String::new(),
            serial: app.music_accounts_serial[index],
        });
    } else {
        app.backend.send(Command::CancelMusicAccount(provider));
    }
}
/// Shared entry point on the welcome screen, Home after Spotify sign-in, and Settings.
pub fn youtube_connect(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let state = &app.music_accounts[0];
    let mut action = None;
    let mut cancelled = false;
    ui.horizontal_wrapped(|ui| {
        if state.busy {
            ui.spinner();
            ui.label(if state.connected {
                "Loading YouTube playlists…"
            } else {
                "Finish signing in to the YouTube window…"
            });
            cancelled = theme::pill_button(ui, &palette, "Cancel", false).clicked();
        } else if state.connected {
            ui.label(format!("YouTube connected · {}", state.name));
        } else if theme::pill_button(ui, &palette, "Connect YouTube", true).clicked() {
            action = Some(Action::Connect);
        }
    });
    if !state.connected && !state.busy {
        ui.small(if app.user_id().is_some() {
            "Sign in to YouTube to copy your Spotify playlists as unlisted. Anyone with a copy’s link can view it."
        } else { "Sign in with Google in the dedicated YouTube window to load your playlists." });
    }
    if let Some(message) = &state.message {
        theme::text(ui, message, theme::regular(12.0), palette.danger);
    }
    if cancelled {
        cancel(app, Provider::YouTubeMusic);
    }
    if let Some(action) = action {
        send(app, Provider::YouTubeMusic, action);
    }
}
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    theme::text(ui, "YouTube", theme::semibold(15.0), palette.text);
    youtube_connect(app, ui);
    if app.music_accounts[0].connected {
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !app.music_accounts[0].busy,
                    egui::Button::new("Refresh playlists"),
                )
                .clicked()
            {
                action = Some(Action::Refresh);
            }
            if ui
                .add_enabled(!app.music_accounts[0].busy, egui::Button::new("Disconnect"))
                .clicked()
            {
                action = Some(Action::Disconnect);
            }
        });
        ui.small("Your YouTube Music playlists are on Home. Playlist transfer is managed below.");
        if let Some(action) = action {
            send(app, Provider::YouTubeMusic, action);
        }
    }
    super::playlist_mirror::show(app, ui);
}

pub(super) fn juice_connect(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let state = &mut app.music_accounts[1];
    let mut action = None;
    ui.label("JuiceVault account");
    ui.add(
        egui::TextEdit::singleline(&mut state.username)
            .hint_text("Username or email")
            .char_limit(200),
    );
    ui.add(
        egui::TextEdit::singleline(&mut state.password.0)
            .password(true)
            .hint_text("Password")
            .char_limit(1024),
    );
    ui.add_enabled_ui(!state.busy, |ui| {
        ui.horizontal_wrapped(|ui| {
            if theme::pill_button(ui, &palette, "Sign in", true).clicked() {
                action = Some(Action::Login(std::mem::take(&mut state.password)));
            }
            if theme::pill_button(ui, &palette, "Load public profile", false).clicked() {
                state.password.0.clear();
                action = Some(Action::PublicProfile);
            }
        });
        if state.connected {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Library · {}", state.name));
                if ui.button("Refresh playlists").clicked() {
                    action = Some(Action::Refresh);
                }
                if ui.button("Disconnect").clicked() {
                    state.password.0.clear();
                    action = Some(Action::Disconnect);
                }
            });
        }
    });
    if state.busy {
        ui.spinner();
        ui.label("Connecting to JuiceVault…");
    }
    if let Some(message) = &state.message {
        ui.colored_label(palette.warning, message);
    }
    ui.small("Sign in for your private playlists, or enter a username and load its public profile. Playlists appear on Home.");
    if let Some(action) = action {
        send(app, Provider::JuiceVault, action);
    }
}
