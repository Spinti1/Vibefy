//! Library for local and optional connected sources. No Spotify account requests.
use crate::{
    app::App,
    model::{Action, Page},
    theme::{self, Icon},
};
use egui::{Align, Layout, Rect, Sense, Vec2, vec2};
use std::collections::HashSet;

#[derive(Clone)]
struct Row {
    source: String,
    title: String,
    image: Option<String>,
    subtitle: String,
    provider: u8,
}
fn rows(app: &App, provider: u8, needle: &str, sort: u8) -> Vec<Row> {
    let mut seen = HashSet::new();
    let mut rows = Vec::new();
    let backups: HashSet<_> = app
        .personal
        .backups
        .values()
        .map(|b| b.remote.as_str())
        .collect();
    for c in app.addons.collections.iter().rev() {
        if crate::music_accounts::excluded_playlist_source(&c.source) {
            continue;
        }
        let group = if c.source.contains("youtube.com/playlist?") {
            2
        } else if c.source.contains("juicevault.xyz/") {
            3
        } else {
            1
        };
        if seen.insert(c.source.clone()) {
            rows.push(Row {
                source: c.source.clone(),
                title: c.title.clone(),
                image: c
                    .image
                    .clone()
                    .or_else(|| c.tracks.first().and_then(|t| t.cover_url.clone())),
                subtitle: format!(
                    "{} songs · {}",
                    c.tracks.len(),
                    if group == 2 {
                        "YouTube"
                    } else if group == 3 {
                        "JuiceVault"
                    } else {
                        "This device"
                    }
                ),
                provider: group,
            });
        }
    }
    for (i, account) in app.music_accounts.iter().enumerate() {
        if !account.connected
            || (i == 1 && !app.settings.addons.active(crate::addons::Id::JuiceVault))
        {
            continue;
        }
        for p in &account.playlists {
            let id = p.id.strip_prefix("VL").unwrap_or(&p.id);
            if i == 0
                && (crate::music_accounts::excluded_youtube_playlist(&p.id) || backups.contains(id))
            {
                continue;
            }
            let source = if i == 0 {
                format!("https://www.youtube.com/playlist?list={id}")
            } else {
                format!("https://api.juicevault.xyz/user/playlists/{}", p.id)
            };
            if seen.insert(source.clone()) {
                rows.push(Row {
                    source,
                    title: p.title.clone(),
                    image: p.image.clone(),
                    subtitle: if i == 0 {
                        "YouTube".into()
                    } else {
                        "JuiceVault".into()
                    },
                    provider: i as u8 + 2,
                });
            }
        }
    }
    let needle = needle.trim().to_lowercase();
    rows.retain(|r| {
        (provider == 0 || r.provider == provider)
            && !app.playlist_hidden(&r.source)
            && (needle.is_empty() || r.title.to_lowercase().contains(&needle))
    });
    if sort == 1 {
        rows.sort_by_cached_key(|r| {
            app.recent_contexts
                .iter()
                .position(|c| {
                    c == &r.source || c == &Page::LocalCollection(r.source.clone()).encode()
                })
                .unwrap_or(usize::MAX)
        });
    }
    if sort == 2 {
        rows.sort_by_cached_key(|r| r.title.to_lowercase());
    }
    let pins = app.settings.library_pins();
    rows.sort_by_key(|r| {
        pins.iter()
            .position(|p| p == &r.source)
            .unwrap_or(usize::MAX)
    });
    rows
}
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let key = ui.make_persistent_id("native-library-controls");
    let (mut provider, mut sort, mut search) =
        ui.data(|d| d.get_temp::<(u8, u8, bool)>(key).unwrap_or_default());
    ui.horizontal(|ui| {
        theme::icon(ui, Icon::Library, 20.0, p.secondary);
        theme::text(ui, "Library", theme::bold(15.0), p.text);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            if theme::icon_button(
                ui,
                Icon::PanelLeft,
                15.0,
                p.secondary,
                p.text,
                "Hide sidebar",
            )
            .clicked()
            {
                app.actions.push(Action::ToggleSidebar);
            }
            if theme::icon_button(
                ui,
                if app.settings.sidebar_grid {
                    Icon::LayoutList
                } else {
                    Icon::LayoutGrid
                },
                15.0,
                p.secondary,
                p.text,
                "Switch library layout",
            )
            .clicked()
            {
                app.actions
                    .push(Action::SetLibraryGrid(!app.settings.sidebar_grid));
            }
            if theme::icon_button(ui, Icon::Plus, 15.0, p.secondary, p.text, "Create playlist")
                .clicked()
            {
                app.actions
                    .push(Action::Personal(crate::personal_library::Action::Show(
                        crate::personal_library::Dialog::New {
                            name: String::new(),
                            backup: app.music_accounts[0].connected,
                        },
                    )));
            }
            if theme::icon_button(
                ui,
                Icon::Search,
                15.0,
                p.secondary,
                p.text,
                "Filter library",
            )
            .clicked()
            {
                search = !search;
                if !search {
                    app.library.filter.clear();
                }
            }
        });
    });
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        for (value, label) in [
            (0, "All"),
            (1, "This device"),
            (2, "YouTube"),
            (3, "JuiceVault"),
        ] {
            if value == 3 && !app.settings.addons.active(crate::addons::Id::JuiceVault) {
                continue;
            }
            if theme::soft_button(ui, &p, None, label, provider == value).clicked() {
                provider = value;
            }
        }
    });
    ui.add_space(6.0);
    let labels = ["Recently added", "Recently played", "Name"];
    egui::ComboBox::from_id_salt("native-library-sort")
        .selected_text(labels[sort.min(2) as usize])
        .show_ui(ui, |ui| {
            for (i, label) in labels.iter().enumerate() {
                ui.selectable_value(&mut sort, i as u8, *label);
            }
        });
    if search {
        ui.add_space(6.0);
        super::widgets::search_field(
            ui,
            &p,
            app.locale,
            key.with("filter"),
            &mut app.library.filter,
            "Filter playlist names",
            ui.available_width(),
        );
    }
    ui.data_mut(|d| d.insert_temp(key, (provider, sort, search)));
    ui.add_space(10.0);
    let entries = rows(app, provider, &app.library.filter, sort);
    egui::ScrollArea::vertical()
        .id_salt("native-library-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for index in 0..2 {
                if provider != 0 && provider != index as u8 + 2 {
                    continue;
                }
                let service = if index == 0 {
                    crate::music_accounts::Provider::YouTubeMusic
                } else {
                    crate::music_accounts::Provider::JuiceVault
                };
                if index == 1 && !app.settings.addons.active(crate::addons::Id::JuiceVault) {
                    continue;
                }
                if app.music_accounts[index].busy {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(13.0));
                        ui.small("Refreshing account library…");
                        if theme::pill_button(ui, &p, "Cancel", false).clicked() {
                            super::music_accounts::cancel(app, service);
                        }
                    });
                }
                if let Some(message) = app.music_accounts[index].message.clone() {
                    ui.add(
                        egui::Label::new(egui::RichText::new(message).size(11.0).color(p.warning))
                            .wrap(),
                    );
                    if !app.music_accounts[index].busy
                        && app.music_accounts[index].connected
                        && theme::pill_button(ui, &p, "Retry refresh", false).clicked()
                    {
                        super::music_accounts::send(
                            app,
                            service,
                            crate::music_accounts::Action::Refresh,
                        );
                    }
                }
            }
            if entries.is_empty() {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(if app.library.filter.trim().is_empty() {
                        "No playlists here yet"
                    } else {
                        "No matching playlists"
                    })
                    .color(p.secondary),
                );
                if provider == 0 || provider == 1 {
                    ui.small("Create a playlist or import a public Spotify playlist from Home.");
                } else if provider == 2 && !app.music_accounts[0].connected {
                    ui.small("Connect YouTube in Settings to load its playlists.");
                }
            }
            let columns = if app.settings.sidebar_grid {
                ((ui.available_width() / 110.0).floor() as usize).max(1)
            } else {
                1
            };
            let width = (ui.available_width() - 8.0 * (columns - 1) as f32) / columns as f32;
            let height = if app.settings.sidebar_grid {
                width + 54.0
            } else {
                62.0
            };
            super::widgets::virtual_rows(
                ui,
                entries.len().div_ceil(columns),
                height + 6.0,
                |ui, index| {
                    ui.horizontal_top(|ui| {
                        for r in entries.iter().skip(index * columns).take(columns) {
                            row(app, ui, r, width, height);
                        }
                    });
                },
            );
        });
}
fn row(app: &mut App, ui: &mut egui::Ui, r: &Row, width: f32, height: f32) {
    let p = app.palette;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let active = app.page() == &Page::LocalCollection(r.source.clone());
    if active || response.hovered() {
        ui.painter().rect_filled(
            rect,
            8.0,
            if active {
                p.surface_active
            } else {
                p.surface_hover
            },
        );
    }
    let grid = app.settings.sidebar_grid;
    let art = Rect::from_min_size(
        rect.min + vec2(6.0, 6.0),
        Vec2::splat(if grid { width - 12.0 } else { 48.0 }),
    );
    super::widgets::paint_cover(
        ui,
        &p,
        r.image.as_deref(),
        art,
        6.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    let x = if grid {
        rect.left() + 6.0
    } else {
        art.right() + 10.0
    };
    let y = if grid {
        art.bottom() + 17.0
    } else {
        rect.top() + 22.0
    };
    let painter = ui.painter().with_clip_rect(rect.shrink(4.0));
    crate::bidi::paint_line(
        &painter,
        x,
        rect.right() - 6.0,
        y,
        &r.title,
        theme::medium(13.5),
        p.text,
    );
    crate::bidi::paint_line(
        &painter,
        x,
        rect.right() - 6.0,
        y + 19.0,
        &r.subtitle,
        theme::regular(11.0),
        p.secondary,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Open playlist {}", r.title),
        )
    });
    if response.double_clicked() {
        if let Some(c) = app
            .source_collection(&r.source)
            .filter(|c| !c.tracks.is_empty())
        {
            app.actions
                .push(Action::Addon(crate::addons::Command::Play {
                    tracks: c.tracks.clone(),
                    index: 0,
                }));
        } else {
            super::home::open_tile(app, &Page::LocalCollection(r.source.clone()));
        }
    } else if response.clicked() {
        super::home::open_tile(app, &Page::LocalCollection(r.source.clone()));
    }
    egui::Popup::context_menu(&response)
        .frame(super::widgets::menu_frame(&p))
        .show(|ui| super::home::source_menu(app, ui, &r.source, &r.title));
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_library_lists_local_and_youtube_without_duplicate_sources_or_episode_queue() {
        let root =
            std::env::temp_dir().join(format!("vibefy-native-library-{}", rand::random::<u64>()));
        let mut app = App::new(
            &Default::default(),
            crate::paths::AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            Default::default(),
            crate::app::AppOptions {
                restore_sign_in: false,
                media_controls: false,
                tray: false,
                autostart: false,
            },
        );
        app.addons.collections = vec![
            crate::addons::Collection {
                title: "Guest list".into(),
                source: crate::personal_library::new_source(),
                ..Default::default()
            },
            crate::addons::Collection {
                title: "Saved YouTube".into(),
                source: "https://www.youtube.com/playlist?list=PLdemo".into(),
                ..Default::default()
            },
        ];
        app.music_accounts[0].connected = true;
        app.music_accounts[0].playlists = vec![
            crate::music_accounts::LibraryPlaylist {
                id: "VLPLdemo".into(),
                title: "Saved YouTube".into(),
                image: None,
            },
            crate::music_accounts::LibraryPlaylist {
                id: "VLSE".into(),
                title: "Episodes for later".into(),
                image: None,
            },
        ];
        assert_eq!(rows(&app, 0, "", 0).len(), 2);
        assert_eq!(rows(&app, 1, "", 0).len(), 1);
        assert_eq!(rows(&app, 2, "youtube", 2).len(), 1);
        app.backend.shutdown();
    }
}
