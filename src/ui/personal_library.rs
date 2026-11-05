//! Home library controls and compact, themed playlist dialogs.
use crate::{
    app::App,
    model::{Action, Page},
    personal_library::{self as library, Action as Edit, Dialog},
    theme::{self, Icon},
};
fn send(app: &mut App, action: Edit) {
    app.actions.push(Action::Personal(action));
}

pub fn home(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    egui::Frame::new().fill(p.panel).stroke(egui::Stroke::new(1.0,p.outline)).corner_radius(14).inner_margin(18.0).show(ui,|ui| {
        theme::text(ui,"Your music, your playlists",theme::semibold(20.0),p.text);
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Create a playlist or bring one from another service.").color(p.secondary));
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing=egui::vec2(10.0,10.0);
            if theme::soft_button(ui,&p,Some(Icon::Plus),"Create playlist",true).clicked(){send(app,Edit::Show(Dialog::New{name:String::new(),backup:app.music_accounts[0].connected}));}
            if theme::soft_button(ui,&p,Some(Icon::ListPlus),"Import YouTube playlist",false).clicked(){send(app,Edit::Show(Dialog::Import{youtube:true,url:String::new(),backup:false}));}
            if theme::soft_button(ui,&p,Some(Icon::ListPlus),"Import Spotify playlist",false).clicked(){send(app,Edit::Show(Dialog::Import{youtube:false,url:String::new(),backup:false}));}
        });
        ui.add_space(12.0);
        if !app.music_accounts[0].connected {
            ui.label(egui::RichText::new("Saved on this device. Connect YouTube to back up selected playlists. Clearing app data or losing this device may delete local playlists.").size(12.0).color(p.secondary));
            if app.music_accounts[0].busy {ui.horizontal(|ui|{ui.spinner();ui.label("Finish connecting in the YouTube window…");});}
            else if theme::pill_button(ui,&p,"Connect YouTube",false).clicked(){super::music_accounts::send(app,crate::music_accounts::Provider::YouTubeMusic,crate::music_accounts::Action::Connect);}
        } else {ui.label(egui::RichText::new("YouTube connected · Account backups are optional. Music files are never included in playlist backups.").size(12.0).color(p.secondary));}
        ui.horizontal_wrapped(|ui| {
            if theme::pill_button(ui,&p,"Export backup",false).clicked(){send(app,Edit::Export);}
            if theme::pill_button(ui,&p,"Import backup",false).clicked(){send(app,Edit::Restore);}
        });
        for pending in app.personal.tasks.values() {ui.horizontal(|ui| {ui.spinner();ui.label(&pending.message);});}
        if let Some(message)=&app.personal.message {ui.add_space(6.0);ui.add(egui::Label::new(egui::RichText::new(message).color(p.secondary)).wrap());}
    });
    ui.add_space(20.0);
}
pub fn controls(app: &mut App, ui: &mut egui::Ui, source: &str) {
    let p = app.palette;
    let Some(list) = app.addons.collections.iter().find(|c| c.source == source) else {
        return;
    };
    let title = list.title.clone();
    ui.horizontal_wrapped(|ui| {
        if theme::pill_button(ui, &p, "Find songs", false).clicked() {
            app.actions.push(Action::Open(Page::Search));
        }
        ui.label(
            egui::RichText::new(app.personal_status(source))
                .color(p.secondary)
                .size(12.0),
        );
        if library::owned(source) && theme::pill_button(ui, &p, "Rename", false).clicked() {
            send(
                app,
                Edit::Show(Dialog::Rename {
                    source: source.into(),
                    name: title,
                }),
            );
        }
        let busy = app
            .personal
            .tasks
            .values()
            .any(|p| p.source.as_deref() == Some(source) && p.backup);
        ui.add_enabled_ui(
            app.music_accounts[0].connected && app.personal.scope.is_some() && !busy,
            |ui| {
                if theme::pill_button(ui, &p, "Save to YouTube", false).clicked() {
                    send(app, Edit::Backup(source.into()));
                }
            },
        );
        if theme::pill_button(ui, &p, "Remove local copy", false).clicked() {
            send(app, Edit::Show(Dialog::Delete(source.into())));
        }
    });
    if let Some(report) = app.personal.backups.get(source)
        && let Some(error) = &report.message
    {
        ui.small(error);
    }
    ui.add_space(10.0);
}
pub fn menu(app: &mut App, ui: &mut egui::Ui, source: &str, name: &str) {
    let p = app.palette;
    if super::widgets::menu_item(ui, &p, Some(Icon::Play), "Open playlist") {
        app.actions
            .push(Action::Open(Page::LocalCollection(source.into())));
    }
    if library::owned(source)
        && super::widgets::menu_item(ui, &p, Some(Icon::Pencil), "Rename playlist")
    {
        send(
            app,
            Edit::Show(Dialog::Rename {
                source: source.into(),
                name: name.into(),
            }),
        );
    }
    if app.music_accounts[0].connected
        && super::widgets::menu_item(ui, &p, Some(Icon::ListPlus), "Save to YouTube")
    {
        send(app, Edit::Backup(source.into()));
    }
    super::widgets::playlist_visibility_menu(ui, app, source);
    if super::widgets::menu_item(ui, &p, Some(Icon::Trash), "Remove local copy") {
        send(app, Edit::Show(Dialog::Delete(source.into())));
    }
}
pub fn song_menu(
    ui: &mut egui::Ui,
    app: &mut App,
    items: &[crate::api::models::PlayableItem],
    context: Option<&crate::model::RowContext>,
) {
    let tracks: Vec<_> = items
        .iter()
        .filter_map(|i| {
            if let crate::api::models::PlayableItem::Track(t) = i {
                Some(crate::playlist_tools::local_track(t))
            } else {
                None
            }
        })
        .collect();
    if tracks.is_empty() {
        return;
    }
    let lists: Vec<_> = app
        .addons
        .collections
        .iter()
        .filter(|c| library::owned(&c.source))
        .map(|c| (c.source.clone(), c.title.clone()))
        .collect();
    let palette = app.palette;
    super::widgets::menu_submenu(
        ui,
        &palette,
        Some(Icon::ListPlus),
        "Add to Vibefy playlist",
        |ui| {
            for (source, title) in lists {
                if super::widgets::menu_item(ui, &app.palette, None, &title) {
                    send(
                        app,
                        Edit::Add {
                            source,
                            tracks: tracks.clone(),
                        },
                    );
                    ui.close();
                }
            }
            if super::widgets::menu_item(ui, &app.palette, Some(Icon::Plus), "Create playlist…") {
                send(app, Edit::CreateFrom(tracks.clone()));
                ui.close();
            }
        },
    );
    if let Some(crate::model::RowContext::Context { uri, .. }) = context
        && let Some(Page::LocalCollection(source)) = Page::decode(uri)
        && library::owned(&source)
    {
        if super::widgets::menu_item(
            ui,
            &app.palette,
            Some(Icon::Minus),
            "Remove from this playlist",
        ) {
            for track in tracks {
                send(
                    app,
                    Edit::Remove {
                        source: source.clone(),
                        uri: track.uri,
                    },
                );
            }
            ui.close();
        }
    }
}
pub fn dialog(app: &mut App, ctx: &egui::Context) {
    let Some(mut dialog) = app.personal.dialog.clone() else {
        return;
    };
    let p = app.palette;
    let mut action = None;
    let frame = egui::Frame::new()
        .fill(p.overlay)
        .stroke(egui::Stroke::new(1.0, p.outline))
        .corner_radius(16)
        .inner_margin(24.0);
    let response=egui::Modal::new(egui::Id::new("personal-playlist-dialog")).frame(frame).show(ctx,|ui| {
        ui.set_width(440.0_f32.min((ctx.content_rect().width()-72.0).max(220.0)));
        ui.spacing_mut().item_spacing.y=12.0;
        match &mut dialog {
            Dialog::New{name,backup}=>{
                theme::text(ui,"Create playlist",theme::bold(24.0),p.text);
                ui.label("Give your playlist a name. You can add songs from their right-click menu.");
                ui.add(egui::TextEdit::singleline(name).hint_text("Playlist name").char_limit(100).desired_width(f32::INFINITY));
                backup_option(app,ui,backup);
                if ui.add_enabled(crate::playlist_tools::valid_title(name),egui::Button::new("Create playlist").min_size(egui::vec2(160.0,36.0))).clicked(){action=Some(Edit::Create{name:name.clone(),backup:*backup});}
            }
            Dialog::Import{youtube,url,backup}=>{
                theme::text(ui,if *youtube {"Import YouTube playlist"} else {"Import Spotify playlist"},theme::bold(24.0),p.text);
                ui.label(if *youtube {"Paste a public or unlisted playlist link. Connect YouTube for private playlists. Public imports support up to 500 songs."} else {"Paste a Spotify playlist link. The playlist is copied into Vibefy; its original stays unchanged. Spotify may require connecting your account."});
                ui.add(egui::TextEdit::singleline(url).hint_text("Playlist link").char_limit(1000).desired_width(f32::INFINITY));
                if !*youtube && !app.is_connected() && theme::pill_button(ui,&p,"Connect Spotify",false).clicked(){app.actions.push(Action::SignIn);}
                backup_option(app,ui,backup);
                let busy=app.personal.tasks.values().any(|p|!p.backup);
                if ui.add_enabled(!busy && !url.trim().is_empty(),egui::Button::new("Import playlist").min_size(egui::vec2(160.0,36.0))).clicked(){action=Some(Edit::Import{youtube:*youtube,url:url.clone(),backup:*backup});}
                if busy {ui.horizontal(|ui|{ui.spinner();ui.label("Importing your playlist…");});}
            }
            Dialog::Rename{source,name}=>{
                theme::text(ui,"Rename playlist",theme::bold(24.0),p.text);
                ui.add(egui::TextEdit::singleline(name).char_limit(100).desired_width(f32::INFINITY));
                ui.small("If this playlist has a YouTube backup, its name will be updated there too.");
                if ui.add_enabled(crate::playlist_tools::valid_title(name),egui::Button::new("Save name")).clicked(){action=Some(Edit::Rename{source:source.clone(),name:name.clone()});}
            }
            Dialog::Delete(source)=>{
                theme::text(ui,"Remove local playlist?",theme::bold(24.0),p.text);
                ui.label("This removes the copy saved in Vibefy on this device. Existing Spotify playlists and YouTube backups are kept in their accounts.");
                if theme::pill_button(ui,&p,"Remove local copy",false).clicked(){action=Some(Edit::Delete(source.clone()));}
            }
        }
        if let Some(message)=&app.personal.message {ui.add(egui::Label::new(egui::RichText::new(message).color(p.secondary)).wrap());}
        if theme::pill_button(ui,&p,"Cancel",false).clicked(){action=Some(Edit::Close);}
    });
    app.personal.dialog = Some(dialog);
    if response.should_close() {
        action = Some(Edit::Close);
    }
    if let Some(action) = action {
        send(app, action);
    }
}
fn backup_option(app: &App, ui: &mut egui::Ui, backup: &mut bool) {
    if app.music_accounts[0].connected && app.personal.scope.is_some() {
        ui.checkbox(backup, "Also save an unlisted copy to my YouTube account");
        if *backup {
            ui.small("Anyone with the copy’s link can view it. Only playlist metadata is saved; unmatched songs stay in your local playlist.");
        } else {
            ui.small("This playlist stays on this device until you save or export a backup.");
        }
    } else {
        *backup = false;
        ui.small("Saved on this device. Connect YouTube later or export a backup to protect your playlist.");
    }
}
