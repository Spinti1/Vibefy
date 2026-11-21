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
        ui.label(egui::RichText::new("Create a playlist or import one from Spotify. Your YouTube playlists appear below after connecting.").color(p.secondary));
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing=egui::vec2(10.0,10.0);
            if theme::soft_button(ui,&p,Some(Icon::Plus),"Create playlist",true).clicked(){send(app,Edit::Show(Dialog::New{name:String::new(),backup:app.music_accounts[0].connected}));}
            if theme::soft_button(ui,&p,Some(Icon::ListPlus),"Import Spotify playlist",false).clicked(){send(app,Edit::Show(Dialog::Import{youtube:false,url:String::new(),backup:false}));}
            if theme::soft_button(ui,&p,Some(Icon::User),if app.personal.spotify.is_some(){"Spotify connected"}else if app.personal.spotify_busy{"Checking Spotify…"}else{"Connect Spotify"},false).clicked(){send(app,Edit::SpotifyOpen);}
        });
        ui.add_space(12.0);
        if !app.music_accounts[0].connected {
            ui.label(egui::RichText::new("Saved on this device. Connect YouTube to back up selected playlists. Clearing app data or losing this device may delete local playlists.").size(12.0).color(p.secondary));
            if app.music_accounts[0].busy {ui.horizontal(|ui|{ui.spinner();ui.label("Finish connecting in the YouTube window…");});}
            else if theme::pill_button(ui,&p,"Connect YouTube",false).clicked(){super::music_accounts::send(app,crate::music_accounts::Provider::YouTubeMusic,crate::music_accounts::Action::Connect);}
        } else {ui.label(egui::RichText::new("YouTube connected · Account backups are optional. Music files are never included in playlist backups.").size(12.0).color(p.secondary));}
        ui.add_space(8.0);
        ui.label(egui::RichText::new(format!("File backup · {} playlists · {} songs · Metadata only",app.addons.collections.len(),app.addons.collections.iter().map(|c|c.tracks.len()).sum::<usize>())).size(12.0).color(p.secondary));
        ui.horizontal_wrapped(|ui| {
            if theme::soft_button(ui,&p,Some(Icon::Copy),"Export backup",false).clicked(){send(app,Edit::Export);}
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
        if theme::pill_button(ui, &p, "Share to phone · QR", false).clicked() {send(app,Edit::PhoneShare{source:source.into(),revoke:false});}
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
                if theme::pill_button(ui, &p, if busy { "Saving…" } else { "Save to YouTube" }, false).on_hover_text("Add missing songs to the existing copy, or recreate a removed copy. Existing YouTube songs are kept.").clicked() {
                    send(app, Edit::SaveBackup(source.into()));
                }
            },
        );
        if theme::pill_button(ui, &p, "Remove local copy", false).clicked() {
            send(app, Edit::Show(Dialog::Delete(source.into())));
        }
    });
    if let Some(binding) = app.settings.spotify_playlists.get(source) {
        let mut enabled = binding.writes;
        if checked_option(
            ui,
            &p,
            &mut enabled,
            "Also send new songs from Vibefy to Spotify",
        )
        .changed()
        {
            send(
                app,
                Edit::SpotifyToggleSync {
                    source: source.into(),
                    enabled,
                },
            );
        }
        ui.small("Spotify additions are checked periodically. Only new songs are copied; removals stay independent.");
        let pending = app
            .personal
            .spotify_pending
            .iter()
            .filter(|(s, _)| s == source)
            .count();
        if pending > 0 {
            ui.small(format!(
                "{pending} Spotify additions pending · You can keep listening"
            ));
        }
    }
    if let Some(pending) = app.personal.tasks.values().find(|task| task.backup && task.source.as_deref() == Some(source)) {
        ui.horizontal(|ui| { ui.spinner(); ui.small(&pending.message); });
    }
    if let Some(report) = app.personal.backups.get(source)
        && let Some(error) = &report.message
    {
        ui.small(error);
    }
    ui.add_space(10.0);
}
pub fn menu(app: &mut App, ui: &mut egui::Ui, source: &str, name: &str) {
    if super::widgets::menu_item(ui,&app.palette,Some(Icon::Copy),"Share to phone · QR") {app.actions.push(Action::Personal(crate::personal_library::Action::PhoneShare{source:source.into(),revoke:false}));}

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
        send(app, Edit::SaveBackup(source.into()));
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
        .filter(|c| {
            library::owned(&c.source) || app.settings.spotify_playlists.contains_key(&c.source)
        })
        .map(|c| (c.source.clone(), c.title.clone()))
        .collect();
    let palette = app.palette;
    if let [crate::api::models::PlayableItem::Track(track)] = items {
        if super::widgets::menu_item(
            ui,
            &palette,
            Some(Icon::ListPlus),
            "Add to Spotify playlist…",
        ) {
            send(
                app,
                Edit::SpotifySong {
                    source: track.clone(),
                    local_source: None,
                },
            );
            ui.close();
        }
    }
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
    let width = (ctx.content_rect().width() - 72.0).clamp(220.0, 490.0);
    let frame = egui::Frame::new()
        .fill(p.panel)
        .stroke(egui::Stroke::new(1.0, p.outline))
        .corner_radius(20)
        .inner_margin(24.0);
    let response=egui::Modal::new(egui::Id::new("personal-playlist-dialog")).frame(frame).show(ctx,|ui| {
        ui.set_width(width); ui.set_max_height((ctx.content_rect().height()-120.0).max(180.0)); ui.spacing_mut().item_spacing.y=10.0;
        let (primary,enabled)=egui::ScrollArea::vertical().id_salt("playlist-dialog-scroll").max_height((ctx.content_rect().height()-220.0).max(100.0)).show(ui,|ui| {
            let (title,kicker,icon)=match &dialog {
                Dialog::SpotifyAdd{..}=>("Add to Spotify","CHOOSE A RECORDING AND PLAYLIST",Icon::ListPlus),
                Dialog::Spotify{..}=>("Your Spotify library","IMPORT FROM YOUR ACCOUNT",Icon::User),
                Dialog::New{..}=>("Create playlist","YOUR LIBRARY",Icon::Music),
                Dialog::Import{youtube:true,..}=>("Import YouTube playlist","ADD TO YOUR LIBRARY",Icon::ListPlus),
                Dialog::Import{..}=>("Import Spotify playlist","ADD TO YOUR LIBRARY",Icon::ListPlus),
                Dialog::Rename{..}=>("Rename playlist","PLAYLIST DETAILS",Icon::Pencil),
                Dialog::Delete(..)=>("Remove local playlist?","YOUR LIBRARY",Icon::Trash),
            };
            ui.horizontal(|ui| {
                let (rect,_)=ui.allocate_exact_size(egui::Vec2::splat(46.0),egui::Sense::hover());
                ui.painter().rect_filled(rect,13.0,p.accent.gamma_multiply(0.12));
                theme::paint_icon(ui,icon,rect,23.0,p.accent);
                ui.add_space(5.0);ui.vertical(|ui| {
                    ui.label(egui::RichText::new(kicker).size(10.0).color(p.secondary).strong());
                    theme::text(ui,title,theme::bold(if width<350.0{21.0}else{25.0}),p.text);
                });
            });
            ui.add_space(6.0);
            let (primary,enabled)=match &mut dialog {
                Dialog::Spotify { backup, selected, follow } => {
                    spotify_selector(app,ui,selected,&mut action);
                    checked_option(ui,&p,follow,"Keep new songs in Spotify and Vibefy");
                    ui.add(egui::Label::new(egui::RichText::new("Optional. New songs added directly in Spotify are checked in the background. Additions from Vibefy can also go to playlists you can edit. No automatic deletions.").size(11.5).color(p.secondary)).wrap());
                    backup_option(app,ui,backup);
                    ("Import selected",!selected.is_empty() && !app.personal.spotify_busy && !app.personal.spotify_importing)
                }
                Dialog::SpotifyAdd { source,query,results,selected,destination,local_source,local,backup } => {
                    spotify_song_picker(app,ui,source,query,results,selected,destination,local_source,local,backup,&mut action);
                    ("Add song",selected.is_some() && destination.as_ref().is_some_and(|id|app.personal.spotify.as_ref().is_some_and(|c|c.entries.iter().any(|e|e.id==*id&&e.editable))) && !app.personal.spotify_busy)
                }
                Dialog::New{name,backup}=>{
                    playlist_preview(ui,app,if name.trim().is_empty(){"Your new playlist"}else{name.trim()},"Vibefy playlist · This device");
                    ui.label(egui::RichText::new("PLAYLIST NAME").size(10.5).strong().color(p.secondary));
                    text_field(ui,&p,name,"e.g. Late night drive",100);
                    ui.label(egui::RichText::new("Add songs from search with Add to Vibefy playlist.").size(12.0).color(p.secondary));
                    backup_option(app,ui,backup);
                    ("Create playlist",crate::playlist_tools::valid_title(name))
                }
                Dialog::Import{youtube,url,backup}=>{
                    ui.label(egui::RichText::new(if *youtube{"Add the songs from a YouTube playlist to your library."}else{"Bring a public Spotify playlist into Vibefy. No Spotify login needed."}).color(p.secondary));
                    egui::Frame::new().fill(p.accent.gamma_multiply(0.07)).stroke(egui::Stroke::new(1.0,p.accent.gamma_multiply(0.2))).corner_radius(10).inner_margin(12.0).show(ui,|ui| {
                        ui.label(egui::RichText::new(if *youtube{"Public or unlisted playlist link"}else{"Public Spotify playlists only"}).strong().color(p.text));
                        ui.add(egui::Label::new(egui::RichText::new(if *youtube{"Connect YouTube to import a private playlist."}else{"Set the playlist to public in Spotify, then copy its link. For private playlists and Liked Songs, use Connect Spotify on Home."}).size(12.0).color(p.secondary)).wrap());
                    });
                    ui.label(egui::RichText::new("PLAYLIST LINK").size(10.5).strong().color(p.secondary));
                    text_field(ui,&p,url,if *youtube{"https://www.youtube.com/playlist?list=…"}else{"https://open.spotify.com/playlist/…"},1000);
                    let valid=if *youtube{crate::addons::playlist_url(url).is_ok()}else{library::spotify_id(url).is_some()};
                    if !url.trim().is_empty()&&!valid {ui.label(egui::RichText::new("Paste a complete playlist link from the selected service.").size(12.0).color(p.warning));}
                    backup_option(app,ui,backup);
                    ui.label(egui::RichText::new("The original playlist stays unchanged.").size(11.5).color(p.secondary));
                    let busy=app.personal.tasks.values().any(|p|!p.backup);
                    if busy {ui.horizontal(|ui|{ui.add(egui::Spinner::new().size(16.0));ui.label("Importing playlist… You can close this window.");});}
                    ("Import playlist",valid&&!busy)
                }
                Dialog::Rename{source,name}=>{
                    let subtitle=app.source_collection(source).map(|c|format!("{} songs",c.tracks.len())).unwrap_or_default();
                    playlist_preview(ui,app,if name.trim().is_empty(){"Playlist name"}else{name.trim()},&subtitle);
                    ui.label(egui::RichText::new("PLAYLIST NAME").size(10.5).strong().color(p.secondary));
                    text_field(ui,&p,name,"Playlist name",100);
                    ui.small("A connected YouTube backup is renamed too.");
                    ("Save name",crate::playlist_tools::valid_title(name))
                }
                Dialog::Delete(source)=>{
                    if let Some(c)=app.source_collection(source){playlist_preview(ui,app,&c.title,&format!("{} songs",c.tracks.len()));}
                    ui.label("Remove this playlist from Vibefy on this device? Spotify playlists, YouTube copies and exported backup files are kept.");
                    ("Remove local copy",true)
                }
            };
            if let Some(message)=&app.personal.message {ui.add(egui::Label::new(egui::RichText::new(message).color(p.secondary).size(12.0)).wrap());}
            (primary,enabled)
        }).inner;
            ui.add_space(6.0); ui.separator(); ui.add_space(6.0);
            ui.horizontal(|ui| {
                if dialog_button(ui,&p,"Cancel",false,true,86.0).clicked(){action=Some(Edit::Close);}
                let room=ui.available_width();
                if dialog_button(ui,&p,primary,true,enabled,room).clicked(){
                    action=Some(match &dialog {
                        Dialog::Spotify{backup,selected,follow}=>Edit::SpotifyImport{selected:selected.iter().cloned().collect(),backup:*backup,follow:*follow},
                        Dialog::SpotifyAdd{..}=>Edit::SpotifySend,
                        Dialog::New{name,backup}=>Edit::Create{name:name.clone(),backup:*backup},
                        Dialog::Import{youtube,url,backup}=>Edit::Import{youtube:*youtube,url:url.clone(),backup:*backup},
                        Dialog::Rename{source,name}=>Edit::Rename{source:source.clone(),name:name.clone()},
                        Dialog::Delete(source)=>Edit::Delete(source.clone()),
                    });
                }
            });
    });
    app.personal.dialog = Some(dialog);
    if response.should_close() {
        action = Some(Edit::Close);
    }
    if let Some(action) = action {
        send(app, action);
    }
}
fn spotify_song_picker(
    app: &App,
    ui: &mut egui::Ui,
    source: &crate::api::models::Track,
    query: &mut String,
    results: &crate::model::Loadable<Vec<crate::api::models::Track>>,
    selected: &mut Option<crate::api::models::Track>,
    destination: &mut Option<String>,
    local_source: &Option<String>,
    local: &mut bool,
    backup: &mut bool,
    action: &mut Option<Edit>,
) {
    let p = app.palette;
    playlist_preview(ui, app, &source.name, &source.artist_names());
    ui.add(egui::Label::new(egui::RichText::new("Choose the Spotify recording and destination. Only the selected song is added; the local copy remains available while Spotify processes the request.").color(p.secondary)).wrap());
    if app.personal.spotify_busy {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading your Spotify library…");
        });
        return;
    }
    let Some(catalog) = &app.personal.spotify else {
        if dialog_button(
            ui,
            &p,
            "Sign in with Spotify",
            true,
            cfg!(windows),
            ui.available_width(),
        )
        .clicked()
        {
            *action = Some(Edit::SpotifyLoad { connect: true });
        }
        return;
    };
    ui.small(format!("Connected · {}", catalog.name));
    if let Some(track) = selected {
        egui::Frame::new()
            .fill(p.surface)
            .stroke(egui::Stroke::new(1.0, p.accent.gamma_multiply(0.4)))
            .corner_radius(10)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(44.0), egui::Sense::hover());
                    super::widgets::paint_cover(
                        ui,
                        &p,
                        track.image(96),
                        rect,
                        6.0,
                        Icon::Music,
                        Some(app.backend.art()),
                    );
                    ui.vertical(|ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&track.name).strong()).truncate(),
                        );
                        ui.small(format!(
                            "{} · {}",
                            track.artist_names(),
                            crate::util::format_duration_ms(track.duration_ms)
                        ));
                    });
                });
            });
        if theme::pill_button(ui, &p, "Choose another recording", false).clicked() {
            *selected = None;
        }
    } else {
        let response = ui.add(
            egui::TextEdit::singleline(query)
                .desired_width(f32::INFINITY)
                .hint_text("Song title and artist")
                .char_limit(200),
        );
        if theme::pill_button(ui, &p, "Search Spotify", false).clicked()
            || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            *action = Some(Edit::SpotifyFind {
                source: source.clone(),
                query: query.clone(),
            });
        }
        match results {
            crate::model::Loadable::Loading => {
                ui.spinner();
                ui.small("Searching recordings…");
            }
            crate::model::Loadable::Failed(error) => {
                ui.add(egui::Label::new(egui::RichText::new(error).color(p.warning)).wrap());
            }
            crate::model::Loadable::Loaded(tracks) => {
                if tracks.is_empty() {
                    ui.small("No recording found. Try a shorter title or save only in Vibefy.");
                }
                egui::ScrollArea::vertical()
                    .id_salt("spotify-song-recordings")
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for track in tracks {
                            let response =
                                ui.add_sized([ui.available_width(), 58.0], egui::Button::new(""));
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    true,
                                    &track.name,
                                )
                            });
                            let cover = egui::Rect::from_min_size(
                                response.rect.min + egui::vec2(7.0, 7.0),
                                egui::Vec2::splat(44.0),
                            );
                            super::widgets::paint_cover(
                                ui,
                                &p,
                                track.image(96),
                                cover,
                                5.0,
                                Icon::Music,
                                Some(app.backend.art()),
                            );
                            let rect = egui::Rect::from_min_max(
                                cover.right_top() + egui::vec2(10.0, 0.0),
                                response.rect.max - egui::vec2(8.0, 7.0),
                            );
                            let mut row = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                            row.set_clip_rect(rect.intersect(ui.clip_rect()));
                            row.add(
                                egui::Label::new(egui::RichText::new(&track.name).strong())
                                    .truncate(),
                            );
                            row.add(
                                egui::Label::new(
                                    egui::RichText::new(format!(
                                        "{} · {}",
                                        track.artist_names(),
                                        crate::util::format_duration_ms(track.duration_ms)
                                    ))
                                    .size(11.5)
                                    .color(p.secondary),
                                )
                                .truncate(),
                            );
                            if response.clicked() {
                                *action = Some(Edit::SpotifyChoose(track.uri.clone()));
                            }
                        }
                    });
            }
            _ => {}
        }
    }
    if selected.is_some() {
        ui.label(
            egui::RichText::new("SPOTIFY PLAYLIST")
                .size(10.5)
                .strong()
                .color(p.secondary),
        );
        let locked = local_source
            .as_ref()
            .and_then(|s| app.settings.spotify_playlists.get(s))
            .map(|b| b.id.as_str());
        egui::ScrollArea::vertical()
            .id_salt("spotify-song-destinations")
            .max_height(160.0)
            .show_rows(ui, 34.0, catalog.entries.len(), |ui, range| {
                for index in range {
                    let entry = &catalog.entries[index];
                    ui.add_enabled_ui(
                        entry.editable && locked.is_none_or(|id| id == entry.id),
                        |ui| {
                            if ui
                                .selectable_label(
                                    destination.as_deref() == Some(entry.id.as_str()),
                                    &entry.title,
                                )
                                .clicked()
                            {
                                *destination = Some(entry.id.clone());
                            }
                        },
                    );
                }
            });
        ui.small("Spotify accepts additions only to Liked Songs and playlists you can edit.");
        if local_source.is_some() {
            *local = true;
            ui.small("This song is also saved in the selected Vibefy playlist.");
        } else {
            checked_option(ui, &p, local, "Keep a Vibefy copy of this playlist");
        }
        if *local {
            backup_option(app, ui, backup);
        } else {
            *backup = false;
        }
    }
    if local_source.is_some() && theme::pill_button(ui, &p, "Save only in Vibefy", false).clicked()
    {
        *action = Some(Edit::SpotifyLocalOnly);
    }
}
fn spotify_selector(
    app: &App,
    ui: &mut egui::Ui,
    selected: &mut std::collections::BTreeSet<String>,
    action: &mut Option<Edit>,
) {
    let p = app.palette;
    ui.add(egui::Label::new(egui::RichText::new("Sign in on Spotify's own page, then choose playlists or Liked Songs to copy into Vibefy. Your Spotify library stays unchanged.").color(p.secondary)).wrap());
    let busy = app.personal.spotify_busy || app.personal.spotify_importing;
    if let Some(catalog) = &app.personal.spotify {
        egui::Frame::new()
            .fill(p.surface)
            .corner_radius(10)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("Connected · {}", catalog.name))
                        .strong()
                        .color(p.text),
                );
                ui.add_enabled_ui(!busy, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if theme::pill_button(ui, &p, "Refresh", false).clicked() {
                            *action = Some(Edit::SpotifyLoad { connect: false });
                        }
                        if theme::pill_button(ui, &p, "Disconnect", false).clicked() {
                            *action = Some(Edit::SpotifyDisconnect);
                        }
                    });
                });
            });
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal_wrapped(|ui| {
                if theme::pill_button(ui, &p, "Select all", false).clicked() {
                    selected.extend(catalog.entries.iter().map(|e| e.id.clone()));
                }
                if theme::pill_button(ui, &p, "Clear selection", false).clicked() {
                    selected.clear();
                }
                ui.small(format!("{} selected", selected.len()));
            });
            egui::ScrollArea::vertical()
                .id_salt("spotify-account-playlists")
                .max_height(230.0)
                .show_rows(ui, 60.0, catalog.entries.len(), |ui, range| {
                    for index in range {
                        let entry = &catalog.entries[index];
                        let chosen = selected.contains(&entry.id);
                        egui::Frame::new()
                            .fill(if chosen {
                                super::blend(p.surface, p.accent, 0.1)
                            } else {
                                p.surface
                            })
                            .stroke(egui::Stroke::new(
                                1.0,
                                if chosen {
                                    p.accent.gamma_multiply(0.5)
                                } else {
                                    p.outline
                                },
                            ))
                            .corner_radius(10)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.set_min_width((ui.available_width() - 2.0).max(100.0));
                                ui.horizontal(|ui| {
                                    let mut checked = chosen;
                                    ui.scope(|ui| {
                                        ui.spacing_mut().icon_width = 20.0;
                                        let visuals = &mut ui.style_mut().visuals;
                                        visuals.widgets.inactive.bg_stroke =
                                            egui::Stroke::new(1.5, p.secondary);
                                        visuals.widgets.inactive.bg_fill = p.window;
                                        ui.checkbox(&mut checked, "").on_hover_text(&entry.title);
                                    });
                                    if checked {
                                        selected.insert(entry.id.clone());
                                    } else {
                                        selected.remove(&entry.id);
                                    }
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::Vec2::splat(38.0),
                                        egui::Sense::hover(),
                                    );
                                    if entry.id == "liked" {
                                        super::sidebar::liked_cover(ui, rect, 6.0);
                                    } else {
                                        super::widgets::paint_cover(
                                            ui,
                                            &p,
                                            entry.image.as_deref(),
                                            rect,
                                            6.0,
                                            Icon::Music,
                                            Some(app.backend.art()),
                                        );
                                    }
                                    ui.vertical(|ui| {
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&entry.title)
                                                    .strong()
                                                    .color(p.text),
                                            )
                                            .truncate(),
                                        );
                                        ui.label(
                                            egui::RichText::new(
                                                entry
                                                    .count
                                                    .map(|n| format!("{n} songs"))
                                                    .unwrap_or_else(|| "Spotify playlist".into()),
                                            )
                                            .size(11.0)
                                            .color(p.secondary),
                                        );
                                    });
                                });
                            });
                    }
                });
        });
    } else {
        ui.add_enabled_ui(!busy && cfg!(windows), |ui| {
            if dialog_button(
                ui,
                &p,
                "Sign in with Spotify",
                true,
                true,
                ui.available_width(),
            )
            .clicked()
            {
                *action = Some(Edit::SpotifyLoad { connect: true });
            }
        });
        if !cfg!(windows) {
            ui.small("Spotify browser sign-in is currently available on Windows.");
        }
    }
    if busy {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(if app.personal.spotify_importing {
                "Importing your selection…"
            } else if app.personal.spotify.is_some() {
                "Loading Spotify library…"
            } else {
                "Checking Spotify connection…"
            });
        });
        for pending in app
            .personal
            .tasks
            .values()
            .filter(|p| p.fingerprint == "spotify-account")
        {
            ui.small(&pending.message);
        }
    }
    ui.add(egui::Label::new(egui::RichText::new("Uses a separate local browser session. Spotify may expire the session or limit imports. Only playlist information is copied; no music files are stored.").size(11.5).color(p.secondary)).wrap());
}
fn playlist_preview(ui: &mut egui::Ui, app: &App, title: &str, subtitle: &str) {
    let p = app.palette;
    egui::Frame::new()
        .fill(p.surface)
        .corner_radius(12)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(58.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 9.0, super::blend(p.surface, p.accent, 0.16));
                theme::paint_icon(ui, Icon::Music, rect, 26.0, p.accent);
                ui.add_space(5.0);
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(title)
                                .font(theme::semibold(16.0))
                                .color(p.text),
                        )
                        .truncate(),
                    );
                    ui.label(egui::RichText::new(subtitle).size(11.5).color(p.secondary));
                });
            });
        });
}
fn text_field(ui: &mut egui::Ui, p: &theme::Palette, value: &mut String, hint: &str, limit: usize) {
    egui::Frame::new()
        .fill(p.surface)
        .stroke(egui::Stroke::new(1.0, p.outline))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .hint_text(hint)
                    .char_limit(limit)
                    .font(theme::regular(15.0))
                    .frame(egui::Frame::NONE)
                    .desired_width(ui.available_width()),
            );
        });
}
fn dialog_button(
    ui: &mut egui::Ui,
    p: &theme::Palette,
    label: &str,
    primary: bool,
    enabled: bool,
    width: f32,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, r) =
            ui.allocate_exact_size(egui::vec2(width.max(60.0), 42.0), egui::Sense::click());
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
        let text = if !enabled {
            p.dim
        } else if primary {
            p.on_accent
        } else {
            p.text
        };
        if primary && enabled {
            if p.dark {
                theme::paint_pill_gradient(
                    ui.painter(),
                    rect,
                    egui::Color32::from_rgb(65, 248, 170),
                    egui::Color32::from_rgb(0, 210, 245),
                );
            } else {
                ui.painter().rect_filled(
                    rect,
                    21.0,
                    if r.hovered() {
                        p.accent_hover
                    } else {
                        p.accent
                    },
                );
            }
        } else {
            ui.painter().rect_filled(
                rect,
                21.0,
                if r.hovered() {
                    p.surface_hover
                } else {
                    p.surface
                },
            );
            ui.painter().rect_stroke(
                rect,
                21.0,
                egui::Stroke::new(1.0, p.outline),
                egui::StrokeKind::Inside,
            );
        }
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            theme::semibold(13.0),
            text,
        );
        theme::focus_ring(ui, &r);
        r
    })
    .inner
}
fn backup_option(app: &App, ui: &mut egui::Ui, backup: &mut bool) {
    let p = app.palette;
    ui.add_space(4.0);
    if app.music_accounts[0].connected && app.personal.scope.is_some() {
        egui::Frame::new()
            .fill(if *backup {
                p.accent.gamma_multiply(0.06)
            } else {
                p.surface
            })
            .stroke(egui::Stroke::new(
                1.0,
                if *backup {
                    p.accent.gamma_multiply(0.35)
                } else {
                    p.outline
                },
            ))
            .corner_radius(10)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.scope(|ui| {
                    let selected = *backup;
                    let style = ui.style_mut();
                    style.spacing.icon_width = 20.0;
                    style.spacing.icon_width_inner = 12.0;
                    style.visuals.override_text_color = Some(p.text);
                    for (visuals, border) in [
                        (&mut style.visuals.widgets.inactive, p.secondary),
                        (&mut style.visuals.widgets.hovered, p.accent),
                        (&mut style.visuals.widgets.active, p.accent),
                    ] {
                        visuals.corner_radius = egui::CornerRadius::same(4);
                        visuals.bg_fill = if selected { p.accent } else { p.window };
                        visuals.bg_stroke = egui::Stroke::new(1.5, border);
                        visuals.fg_stroke =
                            egui::Stroke::new(2.0, if selected { p.on_accent } else { p.text });
                    }
                    let response = ui.checkbox(backup, "Also save to my YouTube account");
                    theme::focus_ring(ui, &response);
                });
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(if *backup {
                            "Creates an unlisted copy. Anyone with its link can view it."
                        } else {
                            "Optional. Your complete playlist is always saved on this device."
                        })
                        .size(11.5)
                        .color(p.secondary),
                    )
                    .wrap(),
                );
            });
    } else {
        *backup = false;
    }
    ui.add(egui::Label::new(egui::RichText::new("Export backup on Home saves your playlists as a small file. Music files and account credentials are never included.").size(11.5).color(p.secondary)).wrap());
}

fn checked_option(
    ui: &mut egui::Ui,
    p: &theme::Palette,
    value: &mut bool,
    label: &str,
) -> egui::Response {
    ui.scope(|ui| {
        let selected = *value;
        let style = ui.style_mut();
        style.spacing.icon_width = 20.0;
        style.spacing.icon_width_inner = 12.0;
        for (visuals, border) in [
            (&mut style.visuals.widgets.inactive, p.secondary),
            (&mut style.visuals.widgets.hovered, p.accent),
            (&mut style.visuals.widgets.active, p.accent),
        ] {
            visuals.corner_radius = egui::CornerRadius::same(4);
            visuals.bg_fill = if selected { p.accent } else { p.window };
            visuals.bg_stroke = egui::Stroke::new(1.5, border);
            visuals.fg_stroke = egui::Stroke::new(2.0, if selected { p.on_accent } else { p.text });
        }
        let response = ui.checkbox(value, label);
        theme::focus_ring(ui, &response);
        response
    })
    .inner
}

pub fn phone_dialog(app:&mut App,ctx:&egui::Context){
 if !app.personal.phone_dialog{return;}
 let p=app.palette;let panel=app.personal.phone_panel.clone();let busy=app.personal.tasks.values().any(|p|p.fingerprint.starts_with("phone-"));let mut action=None;
 let response=egui::Modal::new(egui::Id::new("playlist-phone-share")).frame(egui::Frame::new().fill(p.panel).stroke(egui::Stroke::new(1.0,p.outline)).corner_radius(20).inner_margin(24.0)).show(ctx,|ui|{
  ui.set_width(350.0);theme::text(ui,"Your playlist, on your phone",theme::semibold(22.0),p.text);ui.add_space(8.0);
  ui.label(&app.personal.phone_status);
  if busy{ui.spinner();}
  if let Some(panel)=&panel {
   ui.label(egui::RichText::new(&panel.title).strong());
   {
    let n=panel.qr_width;let scale=5.0;let size=(n+8)as f32*scale;ui.vertical_centered(|ui|{let(rect,_)=ui.allocate_exact_size(egui::vec2(size,size),egui::Sense::hover());ui.painter().rect_filled(rect,8.0,egui::Color32::WHITE);for y in 0..n{for x in 0..n{if panel.qr[y*n+x] {ui.painter().rect_filled(egui::Rect::from_min_size(rect.min+egui::vec2((x+4)as f32*scale,(y+4)as f32*scale),egui::Vec2::splat(scale)),0.0,egui::Color32::BLACK);}}}});
   }
   ui.small(format!("{:.1} KiB · playlist information only · no audio",panel.bytes as f32/1024.0));
   ui.label("Anyone with this link can view the playlist. Changes from this desktop appear when the phone opens it. Inactive links expire after 90 days.");
   if theme::pill_button(ui,&p,"Copy playlist link",false).clicked(){ctx.copy_text(panel.url.clone());}
   ui.add_enabled_ui(!busy,|ui|{if theme::pill_button(ui,&p,"Disable sharing",false).clicked(){action=Some(Edit::PhoneShare{source:panel.source.clone(),revoke:true});}});
  }
  ui.add_space(8.0);if theme::pill_button(ui,&p,"Done",true).clicked(){action=Some(Edit::PhoneClose);}
 });
 if response.should_close(){action=Some(Edit::PhoneClose);}if let Some(action)=action{send(app,action);}
}
