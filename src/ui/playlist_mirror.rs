use crate::{app::App, backend::Command, playlist_mirror::Action, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.separator();
    ui.heading("Spotify → YouTube Music");
    ui.label("Create private copies of your Spotify playlists in YouTube Music and show each linked playlist once in Vibefy. Songs added in Vibefy are queued separately for both services. Deletions stay separate.");
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
                "Sync my playlists to private YouTube Music copies",
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
    if !app.mirror.message.is_empty() {
        ui.label(&app.mirror.message);
    }
    let uncertain = app.mirror.lists.iter().any(|list| list.uncertain);
    if uncertain {
        ui.checkbox(
            &mut app.mirror_retry_confirm,
            "I checked YouTube Music and confirm the pending change was NOT applied.",
        );
    }
    egui::CollapsingHeader::new(format!("Linked playlists ({})", app.mirror.lists.len())).show(
        ui,
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt("mirror-lists")
                .max_height(220.0)
                .show_rows(ui, 44.0, app.mirror.lists.len(), |ui, rows| {
                    for index in rows {
                        let list = &app.mirror.lists[index];
                        ui.horizontal(|ui| {
                            super::widgets::cover(
                                ui,
                                &app.palette,
                                list.collection.cover(),
                                34.0,
                                4.0,
                                theme::Icon::Music,
                            );
                            ui.label(format!(
                                "{} · {} songs · {} need matching",
                                list.collection.title,
                                list.collection.tracks.len(),
                                list.missing
                            ))
                            .on_hover_text(
                                list.issue.as_deref().unwrap_or("Private linked playlist"),
                            );
                            if list.uncertain
                                && ui
                                    .add_enabled(
                                        app.mirror_retry_confirm,
                                        egui::Button::new("Allow retry"),
                                    )
                                    .clicked()
                            {
                                action = Some(Action::RetryChecked(list.spotify_id.clone()));
                                app.mirror_retry_confirm = false;
                            }
                        });
                    }
                });
        },
    );
    egui::CollapsingHeader::new(format!("Review recordings ({})",app.mirror.review_total)).show(ui,|ui| {
        ui.small("Uncertain matches are not copied automatically. Choose the recording you want; unmatched rows remain playable through the existing engine.");
        ui.add(egui::TextEdit::singleline(&mut app.mirror_review_query).hint_text("Optional custom search for the song you select below"));
        egui::ScrollArea::vertical().id_salt("mirror-reviews").max_height(320.0).show(ui,|ui| {
            for review in &app.mirror.reviews {
                egui::CollapsingHeader::new(format!("{} · {}",review.source.title,review.source.artist_names())).id_salt(&review.source.uri).show(ui,|ui|{
                    if ui.button("Search recordings").clicked() {action=Some(Action::Search {uri:review.source.uri.clone(),query:if app.mirror_review_query.trim().is_empty(){format!("{} {}",review.source.title,review.source.artist_names())}else{app.mirror_review_query.clone()}});}
                    for candidate in &review.candidates {ui.horizontal(|ui|{
                        super::widgets::cover(ui,&app.palette,candidate.cover_url.as_deref(),32.0,3.0,theme::Icon::Music);
                        ui.label(format!("{} · {} · {}:{:02}",candidate.title,candidate.artist_names(),candidate.duration_ms/60000,candidate.duration_ms/1000%60));
                        if ui.button("Use recording").clicked(){action=Some(Action::Choose {uri:review.source.uri.clone(),recording:candidate.clone()});}
                    });}
                });
            }
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(app.mirror.review_offset>0,egui::Button::new("Previous 100")).clicked(){action=Some(Action::Reviews(app.mirror.review_offset.saturating_sub(100)));}
            ui.label(format!("{}–{} of {}",if app.mirror.review_total==0 {0}else{app.mirror.review_offset+1},(app.mirror.review_offset+app.mirror.reviews.len()).min(app.mirror.review_total),app.mirror.review_total));
            if ui.add_enabled(app.mirror.review_offset+100<app.mirror.review_total,egui::Button::new("Next 100")).clicked(){action=Some(Action::Reviews(app.mirror.review_offset+100));}
        });
    });
    if let Some(action) = action {
        app.backend.send(Command::Mirror {
            serial: app.mirror_serial,
            action,
        });
    }
}
