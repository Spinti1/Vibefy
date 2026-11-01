use crate::{
    app::App,
    backend::Command,
    music_accounts::{Action, Provider},
    theme,
};
fn send(app: &mut App, provider: Provider, action: Action) {
    let index = provider as usize;
    let start_sync = provider == Provider::YouTubeMusic
        && matches!(action, Action::Connect)
        && app.user_id().is_some();
    app.music_accounts_serial[index] = app.music_accounts_serial[index].wrapping_add(1);
    let state = &mut app.music_accounts[index];
    if matches!(action, Action::Connect) {
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
                "Finish connecting in your browser…"
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
            "Connect with Google to copy your Spotify playlists as unlisted. Anyone with a copy’s link can view it."
        } else { "Choose your Google account to access your YouTube playlists." });
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
        egui::CollapsingHeader::new(format!(
            "YouTube playlists ({})",
            app.music_accounts[0].playlists.len()
        ))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    for playlist in &app.music_accounts[0].playlists {
                        ui.horizontal(|ui| {
                            super::widgets::cover(
                                ui,
                                &palette,
                                playlist.image.as_deref(),
                                36.0,
                                4.0,
                                theme::Icon::Music,
                            );
                            ui.label(&playlist.title);
                            if ui
                                .add_enabled(
                                    !app.music_accounts[0].busy,
                                    egui::Button::new("Save on Home"),
                                )
                                .clicked()
                            {
                                action = Some(Action::Import(playlist.clone()));
                            }
                        });
                    }
                });
        });
        if let Some(action) = action {
            send(app, Provider::YouTubeMusic, action);
        }
    }
    ui.add_space(12.0);
    egui::CollapsingHeader::new("JuiceVault").show(ui, |ui| {
        let state = &mut app.music_accounts[1];
        let mut action = None;
        let mut cancelled = false;
        ui.add(
            egui::TextEdit::singleline(&mut state.username)
                .hint_text("JuiceVault username (public playlists)"),
        );
        ui.horizontal_wrapped(|ui| {
            if state.busy {
                ui.spinner();
                cancelled = theme::pill_button(ui, &palette, "Cancel", false).clicked();
            } else {
                if theme::pill_button(ui, &palette, "Connect JuiceVault", false).clicked() {
                    action = Some(Action::Connect);
                }
                if theme::pill_button(ui, &palette, "Load playlists", false).clicked() {
                    action = Some(Action::Refresh);
                }
                if state.connected
                    && theme::pill_button(ui, &palette, "Disconnect", false).clicked()
                {
                    action = Some(Action::Disconnect);
                }
            }
        });
        if let Some(message) = &state.message {
            ui.small(message);
        }
        for playlist in &state.playlists {
            ui.horizontal(|ui| {
                super::widgets::cover(
                    ui,
                    &palette,
                    playlist.image.as_deref(),
                    36.0,
                    4.0,
                    theme::Icon::Music,
                );
                ui.label(&playlist.title);
                if ui
                    .add_enabled(!state.busy, egui::Button::new("Save on Home"))
                    .clicked()
                {
                    action = Some(Action::Import(playlist.clone()));
                }
            });
        }
        if cancelled {
            cancel(app, Provider::JuiceVault);
        }
        if let Some(action) = action {
            send(app, Provider::JuiceVault, action);
        }
    });
    super::playlist_mirror::show(app, ui);
}
