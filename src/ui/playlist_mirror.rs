use crate::{app::App, backend::Command, playlist_mirror::Action, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.separator();
    ui.heading("Spotify → YouTube Music");
    ui.label("Create unlisted copies of your Spotify playlists in YouTube Music. Anyone with a copy’s link can access it. Existing private copies stay private. Each linked playlist appears once in Vibefy. Songs added in Vibefy are queued separately for both services. Deletions stay separate.");
    if !app.music_accounts[0].connected || app.user_id().is_none() {
        ui.small("Connect Spotify and YouTube Music above to set up this account pair.");
        return;
    }
    if app.music_accounts[0].busy {
        ui.small("Finish connecting or loading YouTube Music before changing sync settings.");
        return;
    }
    let mut action = None;
    if app.mirror.ready {
        ui.label(format!(
            "Spotify: {} · YouTube Music: {}",
            app.mirror.account, app.mirror.youtube_name
        ));
        let mut enabled = app.mirror.enabled;
        if ui
            .checkbox(
                &mut enabled,
                "Sync my playlists to unlisted YouTube Music copies",
            )
            .changed()
        {
            action = Some(Action::Enable(enabled));
        }
        ui.horizontal(|ui| {
            if app.mirror.busy {ui.spinner();}
            if theme::pill_button(ui,&app.palette,"Sync now",false).clicked() {action=Some(Action::SyncNow);}
            ui.small("Changes are checked every 15 minutes while enabled. Rate-limit cooldowns still apply.");
        });
    } else if theme::pill_button(ui, &app.palette, "Check connected accounts", false).clicked() {
        action = Some(Action::Observe);
    }
    if app.mirror.ready
        && theme::pill_button(
            ui,
            &app.palette,
            "Choose playlists / transfer progress",
            false,
        )
        .clicked()
    {
        ui.ctx()
            .data_mut(|d| d.remove::<String>(egui::Id::new("setup-dismissed-pair")));
        app.mirror.setup_complete = false;
        action = Some(Action::ReopenSetup);
    }
    if let Some(action) = action {
        app.backend.send(Command::Mirror {
            serial: app.mirror_serial,
            action,
        });
    }
}
