//! The Home page.

use egui::{CornerRadius, Rect, Sense, Vec2, pos2, vec2};

use crate::api::models::{Album, Episode, PlayableItem, Playlist, Show, pick_image};
use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Loadable, Page};
use crate::theme::{self, Icon};

use super::widgets;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    if !app.network_reachable {
        ui.label(
            "You're offline. Local playlists are still available; streaming needs a connection.",
        );
    }
    ui.add_space(6.0);
    let greeting = crate::util::greeting(app.locale);
    theme::text(ui, greeting.as_ref(), theme::bold(30.0), palette.text);
    ui.add_space(12.0);
    super::personal_library::home(app, ui);
    let all = quick_access_tiles(app);
    for (group, title) in [(4, "Vibefy playlists"), (0, "Linked"), (1, "Spotify")] {
        if group == 1 && app.user_id().is_none() {
            continue;
        }
        if group == 0 && app.mirror.lists.is_empty() {
            continue;
        }
        ui.label(
            egui::RichText::new(title)
                .font(theme::semibold(20.0))
                .color(palette.text),
        );
        ui.add_space(10.0);
        let tiles: Vec<_> = all
            .iter()
            .filter(|tile| {
                tile_group(app, tile) == group
                    && !tile.uri.as_deref().is_some_and(|s| app.playlist_hidden(s))
            })
            .cloned()
            .collect();
        if tiles.is_empty() {
            ui.small(if group == 0 {
                "Your linked Spotify and YouTube playlists appear here."
            } else if group == 4 {
                "Create a playlist above, then add songs from search using their right-click menu."
            } else {
                "Your Spotify playlists appear here."
            });
        }
        quick_access(app, ui, &tiles);
        ui.add_space(20.0);
    }
    youtube_playlists(app, ui);
    if app.settings.addons.active(crate::addons::Id::JuiceVault) {
        juice_playlists(app, ui);
    }
    ui.add_space(16.0);

    if app.is_connected() {
        if app.settings.home.made_for_you.visible {
            made_for_you(app, ui);
            top_mixes(app, ui);
        }
        new_releases(app, ui);
        recently_played(app, ui);
        podcasts(app, ui);
        top_artists(app, ui);
    }
    hidden_playlists(app, ui);
}

#[derive(Clone)]
struct Tile {
    image: Option<String>,
    name: String,
    page: Page,
    uri: Option<String>,
    liked: bool,
    owned_playlist: Option<Playlist>,
    /// A song played on its own, which the tile plays and whose menu it
    /// offers, rather than a playlist, album or artist.
    song: Option<crate::api::models::Track>,
}

/// Linked copies remain in the Spotify shelf; native YouTube lists get their own shelf.
fn youtube_playlists(app: &mut App, ui: &mut egui::Ui) {
    let linked: std::collections::HashSet<_> = app
        .mirror
        .lists
        .iter()
        .filter(|l| {
            !l.youtube_id.is_empty()
                && app.is_saved(&format!("spotify:playlist:{}", l.spotify_id)) != Some(false)
        })
        .map(|l| l.youtube_id.as_str())
        .collect();
    let mut seen = std::collections::HashSet::new();
    let playlists: Vec<_> = app.music_accounts[0]
        .playlists
        .iter()
        .filter(|p| {
            let id = p.id.strip_prefix("VL").unwrap_or(&p.id);
            !linked.contains(id)
                && !app.personal.backups.values().any(|r| {
                    r.remote == id
                        && app.addons.collections.iter().any(|c| {
                            app.personal
                                .backups
                                .get(&c.source)
                                .is_some_and(|b| b.remote == id)
                        })
                })
                && !app.playlist_hidden(&format!("https://www.youtube.com/playlist?list={id}"))
                && seen.insert(id.to_owned())
        })
        .cloned()
        .collect();

    ui.add_space(16.0);
    ui.label(
        egui::RichText::new("YouTube")
            .font(theme::semibold(20.0))
            .color(app.palette.text),
    );
    ui.add_space(10.0);
    let saved: Vec<_> = quick_access_tiles(app)
        .into_iter()
        .filter(|t| {
            tile_group(app, t) == 2 && !t.uri.as_deref().is_some_and(|s| app.playlist_hidden(s))
        })
        .collect();
    quick_access(app, ui, &saved);
    if playlists.is_empty() && saved.is_empty() {
        ui.small("Your YouTube playlists appear here.");
    }
    let columns = ((ui.available_width() / 230.0).floor() as usize).clamp(1, 5);
    let width = (ui.available_width() - 12.0 * (columns - 1) as f32) / columns as f32;
    let mut selected = None;
    widgets::virtual_rows(ui, playlists.len().div_ceil(columns), 92.0, |ui, row| {
        ui.horizontal(|ui| {
            for playlist in playlists.iter().skip(row * columns).take(columns) {
                let (rect, response) = ui.allocate_exact_size(vec2(width, 80.0), Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &playlist.title)
                });
                if ui.is_rect_visible(rect) {
                    ui.painter().rect_filled(
                        rect,
                        8.0,
                        if response.hovered() {
                            app.palette.surface_hover
                        } else {
                            app.palette.surface
                        },
                    );
                    let cover = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(64.0, 64.0));
                    widgets::paint_cover(
                        ui,
                        &app.palette,
                        playlist.image.as_deref(),
                        cover,
                        5.0,
                        Icon::Music,
                        Some(app.backend.art()),
                    );
                    let text = Rect::from_min_max(
                        pos2(cover.right() + 10.0, rect.top()),
                        rect.max - vec2(8.0, 0.0),
                    );
                    crate::bidi::paint_line(
                        &ui.painter().with_clip_rect(text),
                        text.left(),
                        text.right(),
                        rect.center().y,
                        &playlist.title,
                        theme::medium(14.0),
                        app.palette.text,
                    );
                }
                if response.clicked() && !app.music_accounts[0].busy {
                    selected = Some(playlist.clone());
                }
                let source = format!(
                    "https://www.youtube.com/playlist?list={}",
                    playlist.id.strip_prefix("VL").unwrap_or(&playlist.id)
                );
                egui::Popup::context_menu(&response)
                    .frame(widgets::menu_frame(&app.palette))
                    .show(|ui| source_menu(app, ui, &source, &playlist.title));
                response.on_hover_text(&playlist.title);
            }
        });
        ui.add_space(12.0);
    });
    if app.music_accounts[0].busy {
        ui.spinner();
        ui.label("Loading YouTube playlist…");
    }
    if let Some(playlist) = selected {
        let id = playlist.id.strip_prefix("VL").unwrap_or(&playlist.id);
        let source = format!("https://www.youtube.com/playlist?list={id}");
        if app.source_collection(&source).is_some() {
            app.actions
                .push(Action::Open(Page::LocalCollection(source)));
        } else {
            app.actions
                .push(Action::Personal(crate::personal_library::Action::Import {
                    youtube: true,
                    url: source,
                    backup: false,
                }));
        }
    }
}

fn tile_group(app: &App, tile: &Tile) -> u8 {
    match &tile.page {
        Page::LocalCollection(source)
            if app
                .mirror
                .lists
                .iter()
                .any(|p| !p.youtube_id.is_empty() && &p.collection.source == source) =>
        {
            0
        }
        Page::Playlist(_) | Page::LikedSongs => 1,
        Page::LocalCollection(source) if source.starts_with("https://api.juicevault.xyz/") => 3,
        Page::LocalCollection(source) if crate::addons::playlist_url(source).is_ok() => 2,
        _ => 4,
    }
}

fn juice_playlists(app: &mut App, ui: &mut egui::Ui) {
    use crate::music_accounts::{Action as AccountAction, Provider};
    ui.add_space(20.0);
    ui.label(
        egui::RichText::new("JuiceVault")
            .font(theme::semibold(20.0))
            .color(app.palette.text),
    );
    ui.add_space(10.0);
    let native: Vec<_> = app.music_accounts[1]
        .playlists
        .iter()
        .filter(|p| {
            !app.playlist_hidden(&format!(
                "https://api.juicevault.xyz/user/playlists/{}",
                p.id
            ))
        })
        .cloned()
        .collect();
    let saved: Vec<_> = quick_access_tiles(app)
        .into_iter()
        .filter(|t| {
            tile_group(app, t) == 3
                && !t.uri.as_deref().is_some_and(|s| app.playlist_hidden(s))
                && !native.iter().any(|p| {
                    t.page
                        == Page::LocalCollection(format!(
                            "https://api.juicevault.xyz/user/playlists/{}",
                            p.id
                        ))
                })
        })
        .collect();
    quick_access(app, ui, &saved);
    if native.is_empty() && saved.is_empty() {
        ui.small("Connect JuiceVault or load a public profile from Mods.");
    }
    let columns = ((ui.available_width() / 260.0).floor() as usize).clamp(1, 4);
    let width = (ui.available_width() - 10.0 * (columns - 1) as f32) / columns as f32;
    let mut open = None;
    widgets::virtual_rows(ui, native.len().div_ceil(columns), 74.0, |ui, row| {
        ui.horizontal(|ui| {
            for p in native.iter().skip(row * columns).take(columns) {
                let (rect, response) = ui.allocate_exact_size(vec2(width, 64.0), Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &p.title)
                });
                if ui.is_rect_visible(rect) {
                    ui.painter().rect_filled(
                        rect,
                        8.0,
                        if response.hovered() {
                            app.palette.surface_hover
                        } else {
                            app.palette.surface
                        },
                    );
                    let cover = Rect::from_min_size(rect.min + vec2(6.0, 6.0), Vec2::splat(52.0));
                    widgets::paint_cover(
                        ui,
                        &app.palette,
                        p.image.as_deref(),
                        cover,
                        5.0,
                        Icon::Music,
                        Some(app.backend.art()),
                    );
                    let text = Rect::from_min_max(
                        pos2(cover.right() + 10.0, rect.top()),
                        rect.max - vec2(8.0, 0.0),
                    );
                    crate::bidi::paint_line(
                        &ui.painter().with_clip_rect(text),
                        text.left(),
                        text.right(),
                        text.center().y,
                        &p.title,
                        theme::medium(14.0),
                        app.palette.text,
                    );
                }
                let source = format!("https://api.juicevault.xyz/user/playlists/{}", p.id);
                egui::Popup::context_menu(&response)
                    .frame(widgets::menu_frame(&app.palette))
                    .show(|ui| source_menu(app, ui, &source, &p.title));
                if response.clicked() && !app.music_accounts[1].busy {
                    open = Some(p.clone());
                }
            }
        });
        ui.add_space(10.0);
    });
    if let Some(p) = open {
        super::music_accounts::send(app, Provider::JuiceVault, AccountAction::Import(p));
    }
    if app.music_accounts[1].busy {
        ui.spinner();
    }
}

/// The height of a quick-access tile, and so the side of its cover.
const TILE_HEIGHT: f32 = 60.0;
/// All saved playlists, plus a bounded recent-history preview. The grid
/// virtualizes rows so large libraries remain available without drawing every tile.
fn quick_access_tiles(app: &App) -> Vec<Tile> {
    let tile_art = app.art_px(TILE_HEIGHT);
    let collection = app
        .user
        .as_ref()
        .map(|user| format!("spotify:user:{}:collection", user.id));
    let liked_tile = || Tile {
        image: None,
        name: gettext(app.locale, "Liked Songs").into_owned(),
        page: Page::LikedSongs,
        uri: collection.clone(),
        liked: true,
        owned_playlist: None,
        song: None,
    };
    let playlist_tile = |playlist: &Playlist| {
        let linked = app
            .mirror
            .lists
            .iter()
            .find(|list| list.spotify_id == playlist.id && !list.youtube_id.is_empty());
        let imported = app
            .addons
            .collections
            .iter()
            .find(|c| c.source == format!("https://open.spotify.com/playlist/{}", playlist.id));
        let local_source = linked
            .map(|l| l.collection.source.clone())
            .or_else(|| imported.map(|c| c.source.clone()));
        Tile {
            image: pick_image(&playlist.images, tile_art).map(str::to_string),
            name: playlist.name.clone(),
            page: local_source
                .clone()
                .map(Page::LocalCollection)
                .unwrap_or_else(|| Page::Playlist(playlist.id.clone())),
            uri: Some(local_source.unwrap_or_else(|| playlist.uri.clone())),
            liked: false,
            owned_playlist: app
                .user_id()
                .is_some_and(|id| playlist.owned_by(id))
                .then(|| playlist.clone()),
            song: None,
        }
    };
    let mut tiles: Vec<Tile> = if app.user_id().is_some() {
        vec![liked_tile()]
    } else {
        Vec::new()
    };
    let mut seen = std::collections::HashSet::from([Page::LikedSongs]);
    for saved in app
        .mirror
        .lists
        .iter()
        .filter(|list| {
            !list.youtube_id.is_empty()
                && app.is_saved(&format!("spotify:playlist:{}", list.spotify_id)) != Some(false)
        })
        .map(|list| &list.collection)
        .chain(app.addons.collections.iter().rev().filter(|saved| {
            !app.music_accounts[0].connected
                || !app.music_accounts[0].playlists.iter().any(|p| {
                    let id = p.id.strip_prefix("VL").unwrap_or(&p.id);
                    saved.source == format!("https://www.youtube.com/playlist?list={id}")
                })
        }))
    {
        let page = Page::LocalCollection(saved.source.clone());
        if seen.insert(page.clone()) {
            tiles.push(Tile {
                image: saved.cover().map(str::to_owned),
                name: saved.title.clone(),
                page,
                uri: Some(saved.source.clone()),
                liked: false,
                owned_playlist: None,
                song: None,
            });
        }
    }
    // Library playlists are never truncated, regardless of history availability.
    for playlist in app.library.playlists.get().into_iter().flatten() {
        let tile = playlist_tile(playlist);
        if seen.insert(tile.page.clone()) {
            tiles.push(tile);
        }
    }
    for play in app.home.recently_played.get().into_iter().flatten().take(8) {
        let track = &play.track;
        let context = play.context.as_ref().map(|context| context.uri.as_str());
        let tile = match context.map(|uri| (uri, uri.rsplit(':').next().unwrap_or(""))) {
            Some((uri, _)) if uri.ends_with(":collection") => Some(liked_tile()),
            Some((uri, id)) if uri.starts_with("spotify:playlist:") => (app.is_saved(uri)
                != Some(false))
            .then(|| app.known_playlist(id))
            .flatten()
            .map(playlist_tile),
            Some((uri, id)) if uri.starts_with("spotify:album:") => app
                .known_album(id)
                .or(track.album.as_ref().filter(|album| album.id == id))
                .map(|album| Tile {
                    image: pick_image(&album.images, tile_art).map(str::to_string),
                    name: album.name.clone(),
                    page: Page::Album(album.id.clone()),
                    uri: Some(uri.to_string()),
                    liked: false,
                    owned_playlist: None,
                    song: None,
                }),
            Some((uri, id)) if uri.starts_with("spotify:artist:") => {
                app.known_artist(id).map(|artist| Tile {
                    image: pick_image(&artist.images, tile_art).map(str::to_string),
                    name: artist.name.clone(),
                    page: Page::Artist(artist.id.clone()),
                    uri: Some(uri.to_string()),
                    liked: false,
                    owned_playlist: None,
                    song: None,
                })
            }
            Some((uri, id)) if uri.starts_with("spotify:show:") => {
                app.known_show(id).map(|show| Tile {
                    image: pick_image(&show.images, tile_art).map(str::to_string),
                    name: show.name.clone(),
                    page: Page::Show(show.id.clone()),
                    uri: Some(uri.to_string()),
                    liked: false,
                    owned_playlist: None,
                    song: None,
                })
            }
            // A song played on its own, or from somewhere Home cannot name.
            _ => track
                .album
                .as_ref()
                .filter(|album| !album.id.is_empty())
                .map(|album| Tile {
                    image: track.image(tile_art).map(str::to_string),
                    name: track.name.clone(),
                    page: Page::Album(album.id.clone()),
                    uri: Some(track.uri.clone()),
                    liked: false,
                    owned_playlist: None,
                    song: Some(track.clone()),
                }),
        };
        if let Some(tile) = tile
            && seen.insert(tile.page.clone())
        {
            tiles.push(tile);
        }
    }
    tiles
}

fn quick_access(app: &mut App, ui: &mut egui::Ui, tiles: &[Tile]) {
    let palette = app.palette;
    let available = ui.available_width();
    let columns = ((available / 300.0).floor() as usize).clamp(2, 4);
    let gap = 10.0;
    let tile_width = (available - gap * (columns as f32 - 1.0)) / columns as f32;
    let rows = tiles.len().div_ceil(columns);
    widgets::virtual_rows(ui, rows, TILE_HEIGHT + gap, |ui, row| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for column in 0..columns {
                let Some(Tile {
                    image,
                    name,
                    page,
                    uri,
                    liked,
                    owned_playlist,
                    song,
                }) = tiles.get(row * columns + column)
                else {
                    break;
                };
                let (rect, response) =
                    ui.allocate_exact_size(vec2(tile_width, TILE_HEIGHT), Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
                });
                if ui.is_rect_visible(rect) {
                    let hovered = ui.rect_contains_pointer(rect);
                    let fill = if hovered {
                        palette.surface_hover
                    } else {
                        palette.surface
                    };
                    ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
                    let cover = Rect::from_min_size(rect.min, Vec2::splat(TILE_HEIGHT));
                    if *liked {
                        super::sidebar::liked_cover(ui, cover, 6.0);
                    } else {
                        widgets::paint_cover(
                            ui,
                            &palette,
                            image.as_deref(),
                            cover,
                            6.0,
                            Icon::Music,
                            Some(app.backend.art()),
                        );
                    }
                    let play_room = if hovered && uri.is_some() { 52.0 } else { 12.0 };
                    let text_rect = Rect::from_min_max(
                        pos2(cover.right() + 12.0, rect.top()),
                        pos2(rect.right() - play_room, rect.bottom()),
                    );
                    crate::bidi::paint_line(
                        &ui.painter().with_clip_rect(text_rect),
                        text_rect.left(),
                        text_rect.right(),
                        rect.center().y,
                        name,
                        theme::bold(14.5),
                        palette.text,
                    );
                    if hovered && let Some(uri) = uri {
                        let context_here =
                            app.playing_context_uri().as_deref() == Some(uri.as_str());
                        let playing_here = context_here && app.believed_playing();
                        let button = Rect::from_center_size(
                            pos2(rect.right() - 28.0, rect.center().y),
                            Vec2::splat(40.0),
                        );
                        let mut child =
                            ui.new_child(egui::UiBuilder::new().max_rect(button).layout(
                                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            ));
                        if theme::circle_button(
                            &mut child,
                            if playing_here {
                                Icon::PauseFilled
                            } else {
                                Icon::PlayFilled
                            },
                            40.0,
                            palette.accent,
                            palette.accent_hover,
                            palette.on_accent,
                            &gettext(app.locale, if playing_here { "Pause" } else { "Play" }),
                        )
                        .clicked()
                        {
                            if context_here {
                                app.actions.push(Action::TogglePlay);
                            } else if let Page::LocalCollection(source) = page {
                                if let Some(saved) = app.source_collection(source).cloned() {
                                    app.actions
                                        .push(Action::Addon(crate::addons::Command::Play {
                                            tracks: saved.tracks.clone(),
                                            index: 0,
                                        }));
                                }
                            } else if song.is_some() {
                                app.actions.push(Action::PlayUris {
                                    uris: vec![uri.clone()],
                                    index: 0,
                                });
                            } else {
                                app.actions.push(Action::PlayContext {
                                    uri: uri.clone(),
                                    offset_uri: None,
                                    offset_index: None,
                                });
                            }
                        }
                    }
                }
                if response.clicked() {
                    open_tile(app, page);
                }
                if !liked && let Some(uri) = uri {
                    egui::Popup::context_menu(&response)
                        .id(ui.make_persistent_id(("quick-access-menu", uri)))
                        .frame(widgets::menu_frame(&palette))
                        .show(|ui| match song {
                            Some(track) => widgets::item_menu(
                                ui,
                                app,
                                &PlayableItem::Track(track.clone()),
                                None,
                                None,
                            ),
                            None if matches!(page, Page::LocalCollection(_)) => {
                                source_menu(app, ui, uri, name)
                            }
                            None => widgets::context_menu_items(
                                ui,
                                app,
                                uri,
                                name,
                                owned_playlist.as_ref(),
                            ),
                        });
                }
            }
        });
        ui.add_space(gap);
    });
    if let Loadable::Failed(error) = &app.library.playlists {
        widgets::error_row(ui, app, &error.clone(), Some(Page::Home));
    } else if app.library.playlists_loading_since.is_some() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading your playlists…");
        });
    }
}

/// Renders session Home cards or approximate fallback cards in the same shelf.
fn made_for_you(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let (playlists, loading, failure) = match &app.home.made_for_you {
        Loadable::Loaded(playlists) => (playlists.clone(), false, None),
        Loadable::Loading | Loadable::NotLoaded => (Vec::new(), true, None),
        Loadable::Failed(error) => (Vec::new(), false, Some(error.clone())),
    };
    if playlists.is_empty() && !loading && failure.is_none() {
        return;
    }
    widgets::shelf(
        ui,
        &palette,
        "made-for-you",
        &gettext(app.locale, "Made for you"),
        |ui| {
            if playlists.is_empty() && loading {
                widgets::skeleton_cards(ui, &palette, app.locale, 7, false);
            } else if playlists.is_empty()
                && let Some(message) = &failure
            {
                widgets::error_row(ui, app, message, Some(Page::Home));
            }
            playlist_cards(app, ui, "home-made_for_you-menu", &playlists);
        },
    );
}

fn top_mixes(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let Some(playlists) = app
        .home
        .top_mixes
        .get()
        .filter(|playlists| !playlists.is_empty())
        .cloned()
    else {
        return;
    };
    widgets::shelf(
        ui,
        &palette,
        "top-mixes",
        &gettext(app.locale, "Your top mixes"),
        |ui| playlist_cards(app, ui, "home-top_mixes-menu", &playlists),
    );
}

/// The latest releases from followed artists. A card opens its release;
/// the title opens the page that plays all their songs, newest first.
fn new_releases(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let (albums, loading, failure) = match &app.home.new_releases {
        Loadable::Loaded(albums) => (albums.clone(), false, None),
        Loadable::Loading | Loadable::NotLoaded => (Vec::new(), true, None),
        Loadable::Failed(error) => (Vec::new(), false, Some(error.clone())),
    };
    if albums.is_empty() && !loading && failure.is_none() {
        return;
    }
    let open = widgets::linked_shelf(
        ui,
        &palette,
        "new-releases",
        &gettext(app.locale, "New releases"),
        |ui| {
            if albums.is_empty() && loading {
                widgets::loading_row(ui, &palette, app.locale);
            } else if albums.is_empty()
                && let Some(message) = &failure
            {
                widgets::error_row(ui, app, message, Some(Page::Home));
            }
            for album in &albums {
                release_card(app, ui, album);
            }
        },
    );
    if open {
        app.actions.push(Action::Open(Page::NewReleases));
    }
}

fn release_card(app: &mut App, ui: &mut egui::Ui, album: &Album) {
    let palette = app.palette;
    let artists =
        crate::api::models::join_names(album.artists.iter().map(|artist| artist.name.as_str()));
    let subtitle = if artists.is_empty() {
        album.kind_label(app.locale).into_owned()
    } else {
        format!("{} • {artists}", album.kind_label(app.locale))
    };
    let context_here = app.playing_context_uri().as_deref() == Some(album.uri.as_str());
    let playing_here = context_here && app.believed_playing();
    let card_art = app.art_px(widgets::CARD_ART);
    let card = widgets::card(
        ui,
        app,
        pick_image(&album.images, card_art),
        &album.name,
        &subtitle,
        widgets::CardCover::square(playing_here),
    );
    if card.play {
        if context_here {
            app.actions.push(Action::TogglePlay);
        } else {
            app.actions.push(Action::PlayContext {
                uri: album.uri.clone(),
                offset_uri: None,
                offset_index: None,
            });
        }
    }
    if card.clicked {
        app.actions
            .push(Action::Open(Page::Album(album.id.clone())));
    }
    egui::Popup::context_menu(&card.response)
        .id(ui.make_persistent_id(("home-new-releases-menu", &album.uri)))
        .frame(widgets::menu_frame(&palette))
        .show(|ui| {
            widgets::context_menu_items(ui, app, &album.uri, &album.name, None);
        });
}

fn playlist_cards(app: &mut App, ui: &mut egui::Ui, menu: &str, playlists: &[Playlist]) {
    let palette = app.palette;
    for playlist in playlists {
        let subtitle = playlist
            .description
            .as_deref()
            .map(crate::util::strip_html)
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| {
                // Translators: {owner} is the name of the playlist's owner.
                gettext(app.locale, "By {owner}").replace("{owner}", playlist.owner_name())
            });
        let context_here = app.playing_context_uri().as_deref() == Some(playlist.uri.as_str());
        let playing_here = context_here && app.believed_playing();
        let card_art = app.art_px(widgets::CARD_ART);
        let card = widgets::card(
            ui,
            app,
            pick_image(&playlist.images, card_art),
            &playlist.name,
            &subtitle,
            widgets::CardCover::square(playing_here),
        );
        if card.play {
            if context_here {
                app.actions.push(Action::TogglePlay);
            } else {
                app.actions.push(Action::PlayContext {
                    uri: playlist.uri.clone(),
                    offset_uri: None,
                    offset_index: None,
                });
            }
        }
        if card.clicked {
            app.actions
                .push(Action::Open(Page::Playlist(playlist.id.clone())));
        }
        egui::Popup::context_menu(&card.response)
            .id(ui.make_persistent_id((menu, &playlist.uri)))
            .frame(widgets::menu_frame(&palette))
            .show(|ui| {
                let owned = app.user_id().is_some_and(|id| playlist.owned_by(id));
                widgets::context_menu_items(
                    ui,
                    app,
                    &playlist.uri,
                    &playlist.name,
                    owned.then_some(playlist),
                );
            });
    }
}

fn recently_played(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let history = match app.home.recently_played.clone() {
        Loadable::Loaded(history) => history,
        Loadable::Loading | Loadable::NotLoaded => {
            widgets::shelf(
                ui,
                &palette,
                "recent",
                &gettext(app.locale, "Recently played"),
                |ui| widgets::skeleton_cards(ui, &palette, app.locale, 7, false),
            );
            return;
        }
        Loadable::Failed(message) => {
            widgets::shelf(
                ui,
                &palette,
                "recent",
                &gettext(app.locale, "Recently played"),
                |ui| {
                    widgets::error_row(ui, app, &message, Some(Page::Home));
                },
            );
            return;
        }
    };
    let mut seen = std::collections::HashSet::new();
    let tracks: Vec<_> = history
        .into_iter()
        .filter(|entry| {
            entry
                .track
                .id
                .as_ref()
                .is_some_and(|id| seen.insert(id.clone()))
        })
        .take(16)
        .collect();
    if tracks.is_empty() {
        return;
    }
    widgets::shelf(
        ui,
        &palette,
        "recent",
        &gettext(app.locale, "Recently played"),
        |ui| {
            for entry in &tracks {
                let track = &entry.track;
                let card_art = app.art_px(widgets::CARD_ART);
                let card = widgets::card(
                    ui,
                    app,
                    track.image(card_art),
                    &track.name,
                    &track.artist_names(),
                    widgets::CardCover::square(false),
                );
                if card.play {
                    app.actions.push(Action::PlayUris {
                        uris: vec![track.uri.clone()],
                        index: 0,
                    });
                }
                if card.clicked
                    && let Some(album) = &track.album
                    && !album.id.is_empty()
                {
                    app.actions
                        .push(Action::Open(Page::Album(album.id.clone())));
                }
                egui::Popup::context_menu(&card.response)
                    .id(ui.make_persistent_id(("home-recently_played-menu", &track.uri)))
                    .frame(widgets::menu_frame(&palette))
                    .show(|ui| {
                        widgets::item_menu(
                            ui,
                            app,
                            &PlayableItem::Track(track.clone()),
                            None,
                            None,
                        );
                    });
            }
        },
    );
}

/// Why an episode is on the podcast shelf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EpisodeReason {
    /// Started and not finished, with this much left.
    Continue { left_ms: u32 },
    /// The show's newest episode, recently released and not yet started.
    New,
}

/// How many days after its release an unplayed episode still counts as new.
const NEW_EPISODE_DAYS: i64 = 30;
/// The podcast shelf's card limit, as for Recently played.
const PODCAST_CARDS: usize = 16;

/// The episodes on the podcast shelf: those to continue first, then each
/// show's newest unstarted episode from the last month, each group newest
/// first. `skip` leaves out shows that are audiobooks or no longer saved.
pub(crate) fn podcast_episodes(
    podcasts: &[(Show, Vec<Episode>)],
    skip: impl Fn(&Show) -> bool,
    today: jiff::civil::Date,
) -> Vec<(Show, Episode, EpisodeReason)> {
    let oldest_new = today
        .checked_sub(jiff::Span::new().days(NEW_EPISODE_DAYS))
        .unwrap_or(today);
    let released = |episode: &Episode| {
        episode
            .release_date
            .as_deref()
            .and_then(|date| date.get(..10))
            .and_then(|date| date.parse::<jiff::civil::Date>().ok())
    };
    let mut continuing = Vec::new();
    let mut new = Vec::new();
    for (show, episodes) in podcasts {
        if skip(show) {
            continue;
        }
        for episode in episodes {
            if let Some(resume) = &episode.resume_point
                && !resume.fully_played
                && resume.resume_position_ms > 0
            {
                let left_ms = episode
                    .duration_ms
                    .saturating_sub(resume.resume_position_ms);
                continuing.push((show, episode, EpisodeReason::Continue { left_ms }));
            }
        }
        // Spotify lists a show's episodes newest first.
        if let Some(newest) = episodes.first()
            && newest
                .resume_point
                .as_ref()
                .is_none_or(|resume| !resume.fully_played && resume.resume_position_ms == 0)
            && released(newest).is_some_and(|date| date >= oldest_new)
        {
            new.push((show, newest, EpisodeReason::New));
        }
    }
    for group in [&mut continuing, &mut new] {
        group.sort_by_key(|(_, episode, _)| std::cmp::Reverse(released(episode)));
    }
    let mut seen = std::collections::HashSet::new();
    continuing
        .into_iter()
        .chain(new)
        .filter(|(_, episode, _)| !episode.uri.is_empty() && seen.insert(&episode.uri))
        .take(PODCAST_CARDS)
        .map(|(show, episode, reason)| {
            let mut episode = episode.clone();
            // A show's episode list leaves out the show; menus need it.
            if episode.show.is_none() {
                episode.show = Some(show.clone());
            }
            (show.clone(), episode, reason)
        })
        .collect()
}

fn podcasts(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let episodes = podcast_episodes(
        &app.home.podcasts,
        |show| app.audiobook_shows.contains(&show.uri) || app.is_saved(&show.uri) == Some(false),
        jiff::Zoned::now().date(),
    );
    if episodes.is_empty() {
        return;
    }
    widgets::shelf(
        ui,
        &palette,
        "podcasts",
        &gettext(app.locale, "Your podcasts"),
        |ui| {
            for (show, episode, reason) in &episodes {
                let subtitle = match reason {
                    EpisodeReason::Continue { left_ms } => {
                        // Translators: {time} is the time left in an episode, such as 12 min; {show} is the podcast's name.
                        gettext(app.locale, "{time} left • {show}")
                            .replace(
                                "{time}",
                                &crate::util::format_episode_ms(app.locale, *left_ms),
                            )
                            .replace("{show}", &show.name)
                    }
                    EpisodeReason::New => {
                        // Translators: {show} is the podcast's name.
                        gettext(app.locale, "New • {show}").replace("{show}", &show.name)
                    }
                };
                let card_art = app.art_px(widgets::CARD_ART);
                let card = widgets::card(
                    ui,
                    app,
                    pick_image(&episode.images, card_art)
                        .or_else(|| pick_image(&show.images, card_art)),
                    &episode.name,
                    &subtitle,
                    widgets::CardCover::square(false),
                );
                if card.play {
                    app.actions.push(Action::PlayEpisode {
                        uri: episode.uri.clone(),
                        resume_ms: episode.resume_ms(),
                    });
                }
                if card.clicked && !show.id.is_empty() {
                    app.actions.push(Action::Open(Page::Show(show.id.clone())));
                }
                egui::Popup::context_menu(&card.response)
                    .id(ui.make_persistent_id(("home-podcasts-menu", &episode.uri)))
                    .frame(widgets::menu_frame(&palette))
                    .show(|ui| {
                        widgets::item_menu(
                            ui,
                            app,
                            &PlayableItem::Episode(episode.clone()),
                            None,
                            None,
                        );
                    });
            }
        },
    );
}

fn top_artists(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let artists = match app.home.top_artists.clone() {
        Loadable::Loaded(artists) => artists,
        Loadable::Loading | Loadable::NotLoaded => {
            widgets::shelf(
                ui,
                &palette,
                "top-artists",
                &gettext(app.locale, "Your top artists"),
                |ui| widgets::skeleton_cards(ui, &palette, app.locale, 7, true),
            );
            return;
        }
        Loadable::Failed(message) => {
            widgets::shelf(
                ui,
                &palette,
                "top-artists",
                &gettext(app.locale, "Your top artists"),
                |ui| {
                    widgets::error_row(ui, app, &message, Some(Page::Home));
                },
            );
            return;
        }
    };
    if artists.is_empty() {
        return;
    }
    widgets::shelf(
        ui,
        &palette,
        "top-artists",
        &gettext(app.locale, "Your top artists"),
        |ui| {
            for artist in &artists {
                let context_here =
                    app.playing_context_uri().as_deref() == Some(artist.uri.as_str());
                let playing_here = context_here && app.believed_playing();
                let card_art = app.art_px(widgets::CARD_ART);
                let card = widgets::card(
                    ui,
                    app,
                    pick_image(&artist.images, card_art),
                    &artist.name,
                    &gettext(app.locale, "Artist"),
                    widgets::CardCover::portrait(playing_here),
                );
                if card.play {
                    if context_here {
                        app.actions.push(Action::TogglePlay);
                    } else {
                        app.actions.push(Action::PlayContext {
                            uri: artist.uri.clone(),
                            offset_uri: None,
                            offset_index: None,
                        });
                    }
                }
                if card.clicked {
                    app.actions
                        .push(Action::Open(Page::Artist(artist.id.clone())));
                }
                egui::Popup::context_menu(&card.response)
                    .id(ui.make_persistent_id(("home-top_artists-menu", &artist.uri)))
                    .frame(widgets::menu_frame(&palette))
                    .show(|ui| {
                        widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None);
                    });
            }
        },
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn personal_library_imported_spotify_playlist_has_one_home_tile() {
        use crate::{
            addons::Collection,
            api::models::Playlist,
            app::{App, AppOptions},
            backend::Waker,
            model::Loadable,
            paths::AppDirs,
        };
        let root =
            std::env::temp_dir().join(format!("vibefy-home-dedup-{}", rand::random::<u64>()));
        let mut app = App::new(
            &Waker::default(),
            AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            Default::default(),
            AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
                autostart: false,
            },
        );
        app.backend.set_offline(true);
        app.offline = true;
        let id = "1234567890123456789012";
        app.library.playlists = Loadable::Loaded(vec![Playlist {
            id: id.into(),
            uri: format!("spotify:playlist:{id}"),
            name: "Imported playlist".into(),
            ..Default::default()
        }]);
        app.addons.collections.push(Collection {
            title: "Imported playlist".into(),
            source: format!("https://open.spotify.com/playlist/{id}"),
            ..Default::default()
        });
        assert_eq!(
            super::quick_access_tiles(&app)
                .iter()
                .filter(|t| t.name == "Imported playlist")
                .count(),
            1
        );
    }
    use super::*;
    use crate::api::models::ResumePoint;

    fn show(id: &str) -> Show {
        Show {
            id: id.into(),
            uri: format!("spotify:show:{id}"),
            name: id.to_uppercase(),
            ..Show::default()
        }
    }

    fn episode(id: &str, date: &str, played: Option<u32>, finished: bool) -> Episode {
        Episode {
            id: id.into(),
            uri: format!("spotify:episode:{id}"),
            duration_ms: 3_600_000,
            release_date: Some(date.into()),
            resume_point: Some(ResumePoint {
                fully_played: finished,
                resume_position_ms: played.unwrap_or(0),
            }),
            ..Episode::default()
        }
    }

    fn ids(shelf: &[(Show, Episode, EpisodeReason)]) -> Vec<&str> {
        shelf
            .iter()
            .map(|(_, episode, _)| episode.id.as_str())
            .collect()
    }

    /// A tile's cover is 60 points. Spotify's 64 pixel rendition would be
    /// blurry even at 100%, so the tile takes the 300 pixel one and scales
    /// it down; at 300% the 300 pixel one still has a quarter more pixels
    /// than the tile shows.
    #[cfg(feature = "demo")]
    #[test]
    fn quick_access_tiles_ask_for_art_the_display_can_show() {
        let root = std::env::temp_dir().join(format!("spotifast-tiles-{}", std::process::id()));
        let mut app = App::new(
            &crate::backend::Waker::default(),
            crate::paths::AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            crate::settings::Settings::default(),
            crate::app::AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
                autostart: false,
            },
        );
        crate::demo::populate(&mut app);
        // Only the Library's playlists, so the second tile is the first one.
        app.home.recently_played = Loadable::Loaded(Vec::new());
        let first = app.library.playlists.get().unwrap()[0].images.clone();
        let width = |url: Option<&str>| {
            first
                .iter()
                .find(|image| Some(image.url.as_str()) == url)
                .and_then(|image| image.width)
        };
        app.pixels_per_point = 1.0;
        assert_eq!(
            width(quick_access_tiles(&app)[1].image.as_deref()),
            Some(300)
        );
        app.pixels_per_point = 3.0;
        assert_eq!(
            width(quick_access_tiles(&app)[1].image.as_deref()),
            Some(300)
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// Every library playlist stays available, independent of the recent preview.
    #[cfg(feature = "demo")]
    #[test]
    fn quick_access_lists_every_saved_playlist_once() {
        let root = std::env::temp_dir().join(format!("vibefy-home-{}", rand::random::<u64>()));
        let mut app = App::new(
            &crate::backend::Waker::default(),
            crate::paths::AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            Default::default(),
            crate::app::AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
                autostart: false,
            },
        );
        crate::demo::populate(&mut app);
        let playlist_count = app.library.playlists.get().unwrap().len();
        let tiles = quick_access_tiles(&app);
        assert_eq!(tiles[0].page, Page::LikedSongs);
        assert!(tiles.len() > 8);
        for playlist in app.library.playlists.get().unwrap() {
            assert_eq!(
                tiles
                    .iter()
                    .filter(|tile| tile.page == Page::Playlist(playlist.id.clone()))
                    .count(),
                1
            );
        }
        let pages: std::collections::HashSet<_> =
            tiles.iter().map(|tile| tile.page.clone()).collect();
        assert_eq!(pages.len(), tiles.len());
        app.home.recently_played = Loadable::Loading;
        assert_eq!(quick_access_tiles(&app).len(), playlist_count + 1);
        app.backend.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }
    fn today() -> jiff::civil::Date {
        "2026-09-23".parse().unwrap()
    }

    #[test]
    fn episodes_to_continue_come_before_new_ones() {
        let podcasts = vec![
            (
                show("a"),
                vec![
                    episode("a-new", "2026-09-20", None, false),
                    episode("a-started", "2026-09-01", Some(600_000), false),
                    episode("a-done", "2026-08-25", Some(0), true),
                ],
            ),
            (
                show("b"),
                vec![
                    episode("b-started", "2026-09-10", Some(60_000), false),
                    episode("b-old", "2026-09-03", None, false),
                ],
            ),
            (show("c"), vec![episode("c-new", "2026-09-22", None, false)]),
        ];
        let shelf = podcast_episodes(&podcasts, |_| false, today());
        assert_eq!(ids(&shelf), ["b-started", "a-started", "c-new", "a-new"]);
        assert_eq!(shelf[1].2, EpisodeReason::Continue { left_ms: 3_000_000 });
        assert_eq!(shelf[2].2, EpisodeReason::New);
        assert_eq!(
            shelf[0].1.show.as_ref().map(|show| show.id.as_str()),
            Some("b"),
            "the episode menu can go to its podcast"
        );
    }

    #[test]
    fn only_a_recent_unstarted_newest_episode_is_new() {
        let podcasts = vec![
            (show("old"), vec![episode("old", "2026-07-01", None, false)]),
            (
                show("finished"),
                vec![episode("finished", "2026-09-20", Some(0), true)],
            ),
            (
                show("undated"),
                vec![episode("undated", "2026", None, false)],
            ),
            (
                show("second"),
                vec![
                    episode("second-done", "2026-09-21", Some(0), true),
                    episode("second-new", "2026-09-20", None, false),
                ],
            ),
        ];
        assert!(podcast_episodes(&podcasts, |_| false, today()).is_empty());
    }

    #[test]
    fn skipped_shows_leave_the_shelf_and_it_stays_bounded() {
        let podcasts: Vec<_> = (0..20)
            .map(|index| {
                let id = format!("s{index}");
                let started = episode(&format!("{id}-e"), "2026-09-01", Some(1), false);
                (show(&id), vec![started])
            })
            .collect();
        let shelf = podcast_episodes(&podcasts, |show| show.id == "s0", today());
        assert_eq!(shelf.len(), PODCAST_CARDS);
        assert!(!ids(&shelf).contains(&"s0-e"));
    }
}

fn open_tile(app: &mut App, page: &Page) {
    if let Page::LocalCollection(source) = page {
        if app.source_collection(source).is_none() {
            for index in 0..2 {
                let playlist = app.music_accounts[index]
                    .playlists
                    .iter()
                    .find(|p| {
                        let link = if index == 0 {
                            format!(
                                "https://www.youtube.com/playlist?list={}",
                                p.id.strip_prefix("VL").unwrap_or(&p.id)
                            )
                        } else {
                            format!("https://api.juicevault.xyz/user/playlists/{}", p.id)
                        };
                        link == *source
                    })
                    .cloned();
                if let Some(playlist) = playlist {
                    super::music_accounts::send(
                        app,
                        if index == 0 {
                            crate::music_accounts::Provider::YouTubeMusic
                        } else {
                            crate::music_accounts::Provider::JuiceVault
                        },
                        crate::music_accounts::Action::Import(playlist),
                    );
                    return;
                }
            }
        }
    }
    app.actions.push(Action::Open(page.clone()));
}
fn source_menu(app: &mut App, ui: &mut egui::Ui, source: &str, name: &str) {
    if crate::personal_library::owned(source)
        || source.starts_with("https://open.spotify.com/playlist/")
    {
        super::personal_library::menu(app, ui, source, name);
        return;
    }
    let linked = app
        .mirror
        .lists
        .iter()
        .find(|l| l.collection.source == source)
        .map(|l| l.spotify_id.clone());
    if let Some(id) = linked {
        let playlist = app.known_playlist(&id).cloned();
        if let Some(playlist) = playlist {
            let owned = app.user_id().is_some_and(|id| playlist.owned_by(id));
            if !owned
                && widgets::menu_item(
                    ui,
                    &app.palette,
                    Some(Icon::Trash),
                    "Delete from Spotify and YouTube",
                )
            {
                app.actions.push(Action::ShowDialog(
                    crate::model::Dialog::ConfirmDeletePlaylist {
                        id: id.clone(),
                        name: name.into(),
                        owned: false,
                    },
                ));
            }
            widgets::context_menu_items(ui, app, &playlist.uri, name, owned.then_some(&playlist));
            ui.small("Linked playlist deletion removes it from both platforms.");
            return;
        }
        widgets::playlist_visibility_menu(ui, app, source);
        if widgets::menu_item(ui, &app.palette, Some(Icon::Play), "Open playlist") {
            open_tile(app, &Page::LocalCollection(source.into()));
        }
        if widgets::menu_item(
            ui,
            &app.palette,
            Some(Icon::Trash),
            "Delete from Spotify and YouTube",
        ) {
            app.actions.push(Action::ShowDialog(
                crate::model::Dialog::ConfirmDeletePlaylist {
                    id,
                    name: name.into(),
                    owned: false,
                },
            ));
        }
        return;
    }
    ui.set_min_width(200.0);
    widgets::playlist_visibility_menu(ui, app, source);
    if widgets::menu_item(ui, &app.palette, Some(Icon::Play), "Open playlist") {
        open_tile(app, &Page::LocalCollection(source.into()));
    }
    if source.starts_with("https://www.youtube.com/playlist?list=PL")
        && widgets::menu_item(ui, &app.palette, Some(Icon::Trash), "Delete from YouTube")
    {
        app.actions.push(Action::ShowDialog(
            crate::model::Dialog::ConfirmSourceDelete {
                source: source.into(),
                name: name.into(),
            },
        ));
    }
    if let Some(index) = app
        .addons
        .collections
        .iter()
        .position(|c| c.source == source)
    {
        if widgets::menu_item(
            ui,
            &app.palette,
            Some(Icon::Trash),
            "Remove saved copy from Vibefy",
        ) {
            app.actions
                .push(Action::Addon(crate::addons::Command::DeleteCollection(
                    index,
                )));
        }
    }
    if widgets::menu_item(ui, &app.palette, Some(Icon::Copy), "Copy link") {
        ui.ctx().copy_text(source.into());
    }
}
fn hidden_playlists(app: &mut App, ui: &mut egui::Ui) {
    let mut tiles: Vec<_> = quick_access_tiles(app)
        .into_iter()
        .filter(|t| t.uri.as_deref().is_some_and(|s| app.playlist_hidden(s)))
        .collect();
    for index in 0..2 {
        if index == 1 && !app.settings.addons.active(crate::addons::Id::JuiceVault) {
            continue;
        }
        for p in &app.music_accounts[index].playlists {
            let source = if index == 0 {
                format!(
                    "https://www.youtube.com/playlist?list={}",
                    p.id.strip_prefix("VL").unwrap_or(&p.id)
                )
            } else {
                format!("https://api.juicevault.xyz/user/playlists/{}", p.id)
            };
            if app.playlist_hidden(&source)
                && !tiles.iter().any(|t| {
                    t.uri.as_deref().is_some_and(|s| {
                        app.playlist_visibility_key(s) == app.playlist_visibility_key(&source)
                    })
                })
            {
                tiles.push(Tile {
                    image: p.image.clone(),
                    name: p.title.clone(),
                    page: Page::LocalCollection(source.clone()),
                    uri: Some(source),
                    liked: false,
                    owned_playlist: None,
                    song: None,
                });
            }
        }
    }
    ui.add_space(24.0);
    let key = ui.make_persistent_id(("hidden-playlists-open", app.user_id().unwrap_or("local")));
    let mut open = ui.ctx().data(|d| d.get_temp::<bool>(key).unwrap_or(false));
    ui.horizontal(|ui| {
        theme::text(
            ui,
            "Hidden playlists",
            theme::semibold(20.0),
            app.palette.text,
        );
        if theme::pill_button(
            ui,
            &app.palette,
            &format!("{} ({})", if open { "Hide" } else { "Show" }, tiles.len()),
            false,
        )
        .clicked()
        {
            open = !open;
            ui.ctx().data_mut(|d| d.insert_temp(key, open));
        }
    });
    if open {
        ui.add_space(10.0);
        if tiles.is_empty() {
            ui.small("Right-click a playlist and choose Hide playlist. This does not change its privacy on Spotify or YouTube.");
        }
        quick_access(app, ui, &tiles);
    }
}
