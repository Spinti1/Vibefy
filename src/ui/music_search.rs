//! Native music search, with a prominent top result and artist shelves.
use super::widgets;
use crate::{
    addons::{Command, Id},
    app::App,
    model::Action,
    music::{Entry, Filter, Kind},
    theme::{self, Icon},
};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    ui.add_space(8.0);
    let browsing = matches!(app.page(), crate::model::Page::MusicBrowse(_));
    if browsing {
        browse_header(app, ui);
    } else {
        theme::text(ui, "Search", theme::bold(28.0), p.text);
    }
    ui.add_space(12.0);
    if !browsing {
        let options: Vec<_> = Filter::ALL.iter().map(|f| (*f, f.label())).collect();
        if let Some(f) = widgets::chips(ui, &p, &options, app.search.music.filter) {
            app.actions.push(Action::MusicFilter(f));
        }
        ui.add_space(12.0);
    }
    if !browsing && app.search.committed.is_empty() {
        recent(app, ui);
        return;
    }
    if app.search.music.loading || app.search.music.loading_more {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(14.0));
            ui.label(if app.search.music.loading_more {
                "Loading more results…"
            } else {
                "Searching…"
            });
        });
    }
    if let Some(error) = app.search.music.error.clone() {
        widgets::error_row(ui, app, &error, None);
        if theme::soft_button(ui, &p, None, "Retry search", false).clicked() {
            if !app.search.music.results.entries.is_empty()
                && app.search.music.results.continuation.is_some()
            {
                app.actions.push(Action::MusicMore);
            } else if let Some(id) = app
                .search
                .music
                .source_key
                .strip_prefix("https://music.youtube.com/browse/")
            {
                app.actions.push(Action::MusicBrowse(id.into()));
            } else {
                app.actions
                    .push(Action::Search(app.search.committed.clone()));
            }
        }
    }
    let state = &app.search.music;
    let entries = &state.results.entries;
    if entries.is_empty() {
        if !state.loading && state.error.is_none() {
            widgets::empty_state(
                ui,
                &p,
                Icon::Search,
                "No music found",
                "Try the song title and artist, or a different spelling.",
            );
        }
        return;
    }
    let mut actions = Vec::new();
    let songs: Vec<_> = entries
        .iter()
        .filter(|e| e.kind == Kind::Song && e.track.is_some())
        .collect();
    ui.horizontal_wrapped(|ui| {
        if !songs.is_empty() {
            if theme::soft_button(ui, &p, Some(Icon::PlayFilled), "Play all", true).clicked() {
                actions.push(Action::Addon(Command::Play {
                    tracks: playable(app),
                    index: 0,
                }));
            }
            if theme::soft_button(ui, &p, Some(Icon::House), "Save on Home", false).clicked() {
                actions.push(Action::SaveMusicResults);
            }
        }
        if state
            .source_key
            .starts_with("https://music.youtube.com/browse/")
            && theme::soft_button(ui, &p, None, "Back", false).clicked()
        {
            actions.push(Action::Back);
        }
    });
    ui.add_space(14.0);
    let all = !browsing && state.filter == Filter::All && state.page_title.is_empty();
    let top = entries
        .iter()
        .find(|e| {
            e.kind == Kind::Artist && e.title.eq_ignore_ascii_case(app.search.committed.trim())
        })
        .or_else(|| entries.first());
    if all && let Some(top) = top {
        let width = ui.available_width();
        if width >= 690.0 && !songs.is_empty() {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((width * 0.40).min(390.0));
                    top_result(app, ui, top, &mut actions);
                });
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.set_width((width * 0.60 - 28.0).max(200.0));
                    song_list(app, ui, &songs[..songs.len().min(4)], &mut actions);
                });
            });
        } else {
            top_result(app, ui, top, &mut actions);
            if !songs.is_empty() {
                ui.add_space(20.0);
                song_list(app, ui, &songs[..songs.len().min(4)], &mut actions);
            }
        }
        ui.add_space(20.0);
    }
    for (kind, label) in [
        (Kind::Artist, "Artists"),
        (Kind::Album, "Albums"),
        (Kind::Playlist, "Playlists"),
    ] {
        let items: Vec<_> = entries.iter().filter(|e| e.kind == kind).collect();
        if items.is_empty() {
            continue;
        }
        theme::section_title(ui, &p, label);
        ui.add_space(8.0);
        egui::ScrollArea::horizontal()
            .id_salt(("music-shelf", label))
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    for e in items {
                        entity_card(app, ui, e, &mut actions);
                    }
                });
            });
        ui.add_space(22.0);
    }
    let remaining = if all {
        songs.get(4..).unwrap_or(&[])
    } else {
        &songs[..]
    };
    if !remaining.is_empty() {
        theme::section_title(ui, &p, if all { "More songs" } else { "Songs" });
        ui.add_space(8.0);
        for entry in remaining {
            song_row(app, ui, entry, &mut actions);
        }
    }
    if state.results.continuation.is_some()
        && !state.loading_more
        && theme::soft_button(ui, &p, None, "Load more", false).clicked()
    {
        actions.push(Action::MusicMore);
    }
    app.actions.extend(actions);
}
fn browse_header(app: &App, ui: &mut egui::Ui) {
    let p = app.palette;
    let profile = app.search.music.results.profile.as_ref();
    let artist = matches!(app.page(),crate::model::Page::MusicBrowse(id) if id.starts_with("UC")||id.starts_with("MPLA"));
    let name = profile
        .map(|p| p.title.as_str())
        .unwrap_or(&app.search.music.page_title);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(
            ui.available_width(),
            if ui.available_width() < 520.0 {
                174.0
            } else {
                206.0
            },
        ),
        egui::Sense::hover(),
    );
    let top = super::blend(p.panel, p.accent, if p.dark { 0.18 } else { 0.08 });
    ui.painter().rect_filled(rect, 16.0, top);
    let side = if rect.width() < 520.0 { 110.0 } else { 146.0 };
    let art = egui::Rect::from_min_size(
        rect.min + egui::vec2(20.0, (rect.height() - side) / 2.0),
        egui::Vec2::splat(side),
    );
    widgets::paint_cover(
        ui,
        &p,
        profile.and_then(|p| p.image.as_deref()),
        art,
        if artist { side / 2.0 } else { 10.0 },
        if artist { Icon::User } else { Icon::Disc },
        Some(app.backend.art()),
    );
    let x = art.right() + 22.0;
    let right = rect.right() - 20.0;
    let painter = ui.painter().with_clip_rect(rect.shrink(16.0));
    crate::bidi::paint_line(
        &painter,
        x,
        right,
        rect.top() + 48.0,
        if artist { "ARTIST" } else { "MUSIC LIBRARY" },
        theme::semibold(11.0),
        p.secondary,
    );
    crate::bidi::paint_line(
        &painter,
        x,
        right,
        rect.top() + 92.0,
        name,
        theme::bold(if rect.width() < 650.0 { 28.0 } else { 44.0 }),
        p.text,
    );
    if let Some(profile) = profile {
        crate::bidi::paint_line(
            &painter,
            x,
            right,
            rect.top() + 129.0,
            &profile.subtitle,
            theme::regular(12.5),
            p.secondary,
        );
        if rect.height() > 180.0 {
            crate::bidi::paint_line(
                &painter,
                x,
                right,
                rect.top() + 163.0,
                &profile.description,
                theme::regular(12.0),
                p.secondary,
            );
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Label,
            true,
            format!("{} {name}", if artist { "Artist" } else { "Music page" }),
        )
    });
}
fn playable(app: &App) -> Vec<crate::player::LocalTrack> {
    app.search
        .music
        .results
        .entries
        .iter()
        .filter(|e| e.kind == Kind::Song)
        .filter_map(|e| e.track.clone())
        .collect()
}
fn activate(app: &App, e: &Entry, actions: &mut Vec<Action>) {
    if let Some(t) = &e.track {
        let tracks = playable(app);
        if let Some(index) = tracks.iter().position(|x| x.uri == t.uri) {
            actions.push(Action::Addon(Command::Play {
                tracks,
                index: index as u32,
            }));
        }
    } else if let Some(id) = &e.browse_id {
        actions.push(Action::MusicBrowse(id.clone()));
    }
}
fn top_result(app: &App, ui: &mut egui::Ui, e: &Entry, actions: &mut Vec<Action>) {
    let p = app.palette;
    theme::section_title(ui, &p, "Top result");
    ui.add_space(8.0);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 224.0),
        egui::Sense::click(),
    );
    ui.painter().rect_filled(
        rect,
        10.0,
        if response.hovered() {
            p.surface_hover
        } else {
            p.surface
        },
    );
    let art =
        egui::Rect::from_min_size(rect.min + egui::vec2(18.0, 18.0), egui::Vec2::splat(100.0));
    widgets::paint_cover(
        ui,
        &p,
        e.image.as_deref(),
        art,
        if e.kind == Kind::Artist { 50.0 } else { 6.0 },
        Icon::User,
        Some(app.backend.art()),
    );
    let paint = ui.painter().with_clip_rect(rect.shrink(18.0));
    crate::bidi::paint_line(
        &paint,
        rect.left() + 18.0,
        rect.right() - 18.0,
        rect.top() + 150.0,
        &e.title,
        theme::bold(29.0),
        p.text,
    );
    crate::bidi::paint_line(
        &paint,
        rect.left() + 18.0,
        rect.right() - 18.0,
        rect.top() + 192.0,
        &e.spotify_note.clone().unwrap_or_else(|| {
            e.track
                .as_ref()
                .map(|t| format!("Song · {}", t.artist_names()))
                .unwrap_or_else(|| e.kind.label().into())
        }),
        theme::regular(13.0),
        p.secondary,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Open top result {}", e.title),
        )
    });
    source_menu(app, &response, e, actions);
    if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        activate(app, e, actions);
    }
}
fn entity_card(app: &App, ui: &mut egui::Ui, e: &Entry, actions: &mut Vec<Action>) {
    let p = app.palette;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(166.0, 216.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(rect, 8.0, p.surface_hover);
        }
        let art =
            egui::Rect::from_min_size(rect.min + egui::vec2(10.0, 8.0), egui::Vec2::splat(146.0));
        widgets::paint_cover(
            ui,
            &p,
            e.image.as_deref(),
            art,
            if e.kind == Kind::Artist { 73.0 } else { 6.0 },
            if e.kind == Kind::Artist {
                Icon::User
            } else {
                Icon::Disc
            },
            Some(app.backend.art()),
        );
        let paint = ui.painter().with_clip_rect(rect.shrink(10.0));
        crate::bidi::paint_line(
            &paint,
            rect.left() + 10.0,
            rect.right() - 10.0,
            rect.top() + 176.0,
            &e.title,
            theme::bold(15.0),
            p.text,
        );
        crate::bidi::paint_line(
            &paint,
            rect.left() + 10.0,
            rect.right() - 10.0,
            rect.top() + 200.0,
            e.spotify_note
                .as_deref()
                .unwrap_or(if e.kind == Kind::Artist {
                    "Artist"
                } else {
                    &e.subtitle
                }),
            theme::regular(12.0),
            p.secondary,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Open {} {}", e.kind.label(), e.title),
        )
    });
    if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        activate(app, e, actions);
    }
}
fn song_list(app: &App, ui: &mut egui::Ui, songs: &[&Entry], actions: &mut Vec<Action>) {
    theme::section_title(ui, &app.palette, "Songs");
    ui.add_space(8.0);
    for e in songs {
        song_row(app, ui, e, actions);
    }
}
fn song_row(app: &App, ui: &mut egui::Ui, e: &Entry, actions: &mut Vec<Action>) {
    let p = app.palette;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(rect, 6.0, p.surface_hover);
        }
        let art =
            egui::Rect::from_min_size(rect.min + egui::vec2(6.0, 6.0), egui::Vec2::splat(40.0));
        widgets::paint_cover(
            ui,
            &p,
            e.image.as_deref(),
            art,
            4.0,
            Icon::Disc,
            Some(app.backend.art()),
        );
        let paint = ui.painter().with_clip_rect(egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.right() - 48.0, rect.bottom()),
        ));
        crate::bidi::paint_line(
            &paint,
            rect.left() + 58.0,
            rect.right() - 48.0,
            rect.top() + 16.0,
            &e.title,
            theme::medium(14.0),
            p.text,
        );
        let mut artist = e
            .track
            .as_ref()
            .map(|t| t.artist_names())
            .unwrap_or_else(|| e.subtitle.clone());
        if artist.is_empty() {
            artist = "Artist unavailable".into();
        }
        if rect.width() >= 520.0
            && let Some(note) = &e.spotify_note
        {
            artist.push_str(" · ");
            artist.push_str(note);
        }
        crate::bidi::paint_line(
            &paint,
            rect.left() + 58.0,
            rect.right() - 48.0,
            rect.top() + 37.0,
            &artist,
            theme::regular(12.0),
            p.secondary,
        );
        if let Some(t) = &e.track {
            let s = t.duration_ms / 1000;
            if s > 0 {
                ui.painter().text(
                    egui::pos2(rect.right() - 6.0, rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{}:{:02}", s / 60, s % 60),
                    theme::regular(12.0),
                    p.secondary,
                );
            }
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Play {}", e.title))
    });
    source_menu(app, &response, e, actions);
    let mut artist_clicked = false;
    if let Some(artist) = e.track.as_ref().and_then(|t| t.artists.first()) {
        let area = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 58.0, rect.top() + 26.0),
            egui::pos2(rect.right() - 48.0, rect.bottom()),
        );
        let link = ui.interact(area, response.id.with("artist"), egui::Sense::click());
        link.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!("Open artist {}", artist.name),
            )
        });
        artist_clicked = link
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked();
        if artist_clicked {
            if let Some(id) = artist
                .id
                .as_ref()
                .filter(|id| id.starts_with("UC") || id.starts_with("MPLA"))
            {
                actions.push(Action::MusicBrowse(id.clone()));
            } else {
                actions.push(Action::Search(artist.name.clone()));
            }
        }
    }
    if !artist_clicked
        && response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    {
        activate(app, e, actions);
    }
}
fn source_menu(app: &App, response: &egui::Response, e: &Entry, actions: &mut Vec<Action>) {
    if let Some(t) = &e.track {
        response.context_menu(|ui| {
            ui.menu_button("Add to Vibefy playlist", |ui| {
                for c in app
                    .addons
                    .collections
                    .iter()
                    .filter(|c| crate::personal_library::owned(&c.source))
                {
                    if ui.button(&c.title).clicked() {
                        actions.push(Action::Personal(crate::personal_library::Action::Add {
                            source: c.source.clone(),
                            tracks: vec![t.clone()],
                        }));
                        ui.close();
                    }
                }
                if ui.button("Create playlist…").clicked() {
                    actions.push(Action::Personal(
                        crate::personal_library::Action::CreateFrom(vec![t.clone()]),
                    ));
                    ui.close();
                }
            });
            for artist in &t.artists {
                if ui
                    .button(format!("View artist · {}", artist.name))
                    .clicked()
                {
                    if let Some(id) = artist
                        .id
                        .as_ref()
                        .filter(|id| id.starts_with("UC") || id.starts_with("MPLA"))
                    {
                        actions.push(Action::MusicBrowse(id.clone()));
                    } else {
                        actions.push(Action::Search(artist.name.clone()));
                    }
                    ui.close();
                }
            }
            if app.settings.addons.active(Id::TrackPeek) && ui.button("Preview audio").clicked() {
                actions.push(Action::PreviewTrack(t.clone()));
                ui.close();
            }
        });
    }
}
fn recent(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    if app.settings.search_history.is_empty() {
        widgets::empty_state(
            ui,
            &p,
            Icon::Search,
            "Search music",
            "Enter a song, album or artist in the search bar above.",
        );
        return;
    }
    theme::section_title(ui, &p, "Recent searches");
    ui.add_space(6.0);
    let history = app.settings.search_history.clone();
    ui.horizontal_wrapped(|ui| {
        for query in &history {
            let (response, forget) = theme::soft_button_dismiss(ui, &p, Icon::Clock, query);
            if forget {
                app.actions.push(Action::ForgetSearch(query.clone()));
            } else if response.clicked() {
                app.actions.push(Action::Search(query.clone()));
            }
        }
    });
}
