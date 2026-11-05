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
                        ui.add(egui::Label::new(egui::RichText::new(if *youtube{"Connect YouTube to import a private playlist."}else{"Set the playlist to public in Spotify, then copy its link. Private playlists and Liked Songs are not supported."}).size(12.0).color(p.secondary)).wrap());
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
                ui.checkbox(backup, "Also save to my YouTube account");
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
