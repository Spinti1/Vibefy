//! Spicy-style lyrics rendered directly by egui, following Vibefy's clock.

use egui::{Align, Color32, Frame, Layout, Rect, Sense, UiBuilder, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Loadable};
use crate::spicy_lyrics::{Appearance, Background, Document, FontStyle, Line, TextAlign};
use crate::theme::{self, Icon};

use super::widgets;

pub(super) const CONFIG_ID: &str = "spicy-lyrics-config";

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    egui::CentralPanel::default()
        .frame(Frame::new().fill(theme::Palette::dark().window))
        .show(ui, |ui| {
            let bounds = ui.max_rect();
            background(app, ui, bounds);
            let inset = if bounds.width() >= 1000.0 { 48.0 } else { 24.0 };
            let region = Rect::from_min_max(
                pos2(
                    bounds.left() + inset,
                    bounds.top() + theme::top_bar_height() + 12.0,
                ),
                pos2(bounds.right() - inset, bounds.bottom() - 18.0),
            );
            let mut content = ui.new_child(UiBuilder::new().max_rect(region));
            header(app, &mut content);
            metadata(app, &mut content);
            content.add_space(16.0);
            let Some(now) = app.now_playing() else {
                widgets::empty_state(
                    &mut content,
                    &theme::Palette::dark(),
                    Icon::Mic,
                    &gettext(app.locale, "Nothing playing"),
                    &gettext(app.locale, "Play a song to see its lyrics."),
                );
                return;
            };
            let remaining = content.available_rect_before_wrap();
            let wide =
                remaining.width() >= 940.0 && app.settings.spicy_lyrics_appearance.show_cover;
            let lyrics_rect = if wide {
                let side = (remaining.width() * 0.34)
                    .min(360.0)
                    .min((remaining.height() - 120.0).max(100.0));
                let column = Rect::from_min_size(
                    pos2(remaining.left(), remaining.center().y - (side + 92.0) / 2.0),
                    vec2(side, side + 92.0),
                );
                cover(app, &mut content, column, &now);
                Rect::from_min_max(pos2(column.right() + 64.0, remaining.top()), remaining.max)
            } else {
                content.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
                    widgets::playing_cover(
                        ui,
                        &theme::Palette::dark(),
                        app.backend.art(),
                        "spicy-small-cover",
                        app.small_art(&now, 56.0),
                        rect,
                        8.0,
                    );
                    ui.vertical(|ui| track_text(ui, &now));
                });
                content.add_space(14.0);
                content.available_rect_before_wrap()
            };
            let mut words = content.new_child(UiBuilder::new().max_rect(lyrics_rect));
            lyrics(app, &mut words, &now);
            if now.playing {
                if !app.settings.spicy_lyrics_appearance.limit_fps {
                    ui.ctx().request_repaint();
                } else {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_secs_f64(
                            1.0 / f64::from(
                                app.settings.spicy_lyrics_appearance.fps.clamp(15, 240),
                            ),
                        ));
                }
            }
        });
    configuration(app, ui.ctx());
}

fn header(app: &mut App, ui: &mut egui::Ui) {
    let palette = theme::Palette::dark();
    ui.horizontal(|ui| {
        theme::text(ui, "Spicy Lyrics", theme::bold(22.0), palette.text);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::icon_button(
                ui,
                Icon::X,
                20.0,
                palette.secondary,
                palette.text,
                &gettext(app.locale, "Close"),
            )
            .clicked()
            {
                app.actions.push(Action::ToggleSpicyLyrics);
            }
            if theme::icon_button(
                ui,
                Icon::Settings,
                19.0,
                palette.secondary,
                palette.text,
                "Spicy Lyrics settings",
            )
            .clicked()
            {
                app.actions.push(Action::OpenSpicySettings(0));
            }
            if theme::icon_button(
                ui,
                Icon::Refresh,
                19.0,
                palette.secondary,
                palette.text,
                &gettext(app.locale, "Try again"),
            )
            .clicked()
            {
                app.actions.push(Action::RetrySpicyLyrics);
            }
            if !app.spicy.following && theme::pill_button(ui, &palette, "Follow", false).clicked() {
                app.actions.push(Action::SetSpicyFollow(true));
            }
        });
    });
}

fn track_text(ui: &mut egui::Ui, now: &crate::app::NowPlaying) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(&now.title)
                .font(theme::bold(22.0))
                .color(Color32::WHITE),
        )
        .truncate(),
    );
    ui.add(
        egui::Label::new(
            egui::RichText::new(&now.subtitle)
                .font(theme::regular(14.0))
                .color(Color32::from_gray(200)),
        )
        .truncate(),
    );
}

fn cover(app: &App, ui: &mut egui::Ui, column: Rect, now: &crate::app::NowPlaying) {
    let rect = Rect::from_min_size(column.min, vec2(column.width(), column.width()));
    ui.painter().add(
        egui::epaint::Shadow {
            offset: [0, 16],
            blur: 48,
            spread: 0,
            color: Color32::from_black_alpha(130),
        }
        .as_shape(rect, 16.0),
    );
    widgets::playing_cover(
        ui,
        &theme::Palette::dark(),
        app.backend.art(),
        "spicy-cover",
        now.art_url.as_deref().or(now.art_small.as_deref()),
        rect,
        16.0,
    );
    let mut text = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(
        pos2(column.left(), rect.bottom() + 20.0),
        column.max,
    )));
    track_text(&mut text, now);
}

fn lyrics(app: &mut App, ui: &mut egui::Ui, now: &crate::app::NowPlaying) {
    let palette = theme::Palette::dark();
    // Never show the previous track's text during the frame before the
    // backend update requests the new track.
    if app.spicy.uri.as_deref() != Some(now.uri.as_str()) {
        widgets::loading_row(ui, &palette, app.locale);
        return;
    }
    let found = match &app.spicy.result {
        Loadable::Loading | Loadable::NotLoaded => {
            widgets::loading_row(ui, &palette, app.locale);
            return;
        }
        Loadable::Failed(error) => {
            theme::text(ui, error, theme::regular(14.0), palette.text);
            return;
        }
        Loadable::Loaded(found) => found.clone(),
    };
    let mut scrolling = ui.new_child(UiBuilder::new().max_rect(ui.available_rect_before_wrap()));
    match &found.document {
        Some(doc) if doc.instrumental => widgets::empty_state(
            &mut scrolling,
            &palette,
            Icon::Music,
            &gettext(app.locale, "Instrumental"),
            &gettext(app.locale, "No timed lyrics for this track."),
        ),
        Some(doc) => timed_lines(app, &mut scrolling, doc, now),
        None => widgets::empty_state(
            &mut scrolling,
            &palette,
            Icon::Mic,
            &gettext(app.locale, "No lyrics"),
            &gettext(app.locale, "No lyrics found for this track."),
        ),
    }
}

fn metadata(app: &mut App, ui: &mut egui::Ui) {
    let Some(now) = app.now_playing() else {
        return;
    };
    if app.spicy.uri.as_deref() != Some(now.uri.as_str()) {
        return;
    }
    let Loadable::Loaded(found) = &app.spicy.result else {
        return;
    };
    let mut action = None;
    let palette = theme::Palette::dark();
    ui.add_space(6.0);
    ui.scope(|ui| {
        ui.style_mut().override_font_id = Some(theme::regular(11.0));
        ui.spacing_mut().item_spacing = vec2(8.0, 3.0);
        if let Some(doc) = &found.document {
            ui.horizontal_wrapped(|ui| {
                let mode = if doc.word_synced() {
                    "Word synced"
                } else if doc.synced() {
                    "Line synced"
                } else {
                    "Plain lyrics"
                };
                theme::text(
                    ui,
                    format!("{mode} · {}", doc.provider),
                    theme::regular(11.0),
                    palette.secondary,
                );
                for credit in &doc.credits {
                    if let Some(url) = &credit.url {
                        ui.hyperlink_to(&credit.name, url);
                    } else {
                        ui.label(&credit.name);
                    }
                }
            });
            let points = app
                .settings
                .spicy_lyrics_calibrations
                .get(&app.spicy_calibration_key())
                .map_or(0, |calibration| calibration.points.len());
            let key = app.spicy_calibration_key();
            let manual_offset = app
                .settings
                .spicy_lyrics_calibrations
                .get(&key)
                .map_or(0, |cal| cal.offset_ms);
            let status = if app.spicy.auto_running && app.spicy.auto_key.as_deref() == Some(&key) {
                "Syncing…".to_string()
            } else if points > 0 {
                format!("Sync · {points} manual points")
            } else if manual_offset != 0 {
                format!("Sync · {manual_offset:+} ms manual offset")
            } else if app.settings.spicy_lyrics_auto_sync {
                app.settings
                    .spicy_lyrics_auto_results
                    .get(&key)
                    .filter(|result| result.version == crate::auto_sync::ALIGNER_VERSION)
                    .map_or_else(
                        || "Sync".to_string(),
                        |result| format!("Synced · {} matched phrases", result.matched_lines),
                    )
            } else {
                "Sync · auto off".to_string()
            };
            if ui.small_button(status).clicked() {
                action = Some(Action::OpenSpicySettings(2));
            }
        }
        if let Some(notice) = &found.notice {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(notice)
                        .size(12.0)
                        .color(palette.warning),
                )
                .wrap(),
            );
        }
        if app.settings.spicy_lyrics_key.is_empty()
            && ui
                .link("Connect Spicy Lyrics for additional word-synced lyrics")
                .clicked()
        {
            action = Some(Action::OpenSpicySettings(3));
        }
    });
    if let Some(action) = action {
        app.actions.push(action);
    }
}

fn timed_lines(app: &mut App, ui: &mut egui::Ui, doc: &Document, now: &crate::app::NowPlaying) {
    let mut appearance = app.settings.spicy_lyrics_appearance.clone();
    appearance.normalize();
    let calibration = app.spicy_calibration();
    let audio_position = app.spicy_audio_position(now);
    let position = calibration.lyric_position(audio_position, app.settings.spicy_lyrics_offset_ms);
    let scroll_active = doc.active_line(position.saturating_add(appearance.early_scroll_ms));
    let active = doc.active_line(position);
    let viewport = ui.available_rect_before_wrap();
    let manual = ui.rect_contains_pointer(viewport)
        && ui.input(|input| {
            input.smooth_scroll_delta.y != 0.0
                || (input.pointer.primary_down() && input.pointer.delta().y != 0.0)
        });
    let following = app.spicy.following && !manual;
    let follow = following && app.spicy.shown_line != Some(scroll_active);
    let size = appearance.font_size.min(viewport.width() * 0.085).max(20.0);
    ui.spacing_mut().scroll.fade.strength = 0.0;
    egui::ScrollArea::vertical()
        .id_salt(("spicy-scroll", &now.uri))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space((viewport.height() * appearance.follow_position - size).max(8.0));
            if follow && active.is_none() {
                let next = doc
                    .lines
                    .iter()
                    .position(|line| line.start_ms.is_some_and(|at| at > position));
                // Before the song starts, reset to the beginning. Between
                // lines keep the next words in view instead of jumping up.
                if next == Some(0) {
                    ui.scroll_to_rect(
                        Rect::from_min_size(ui.cursor().min, vec2(1.0, 1.0)),
                        Some(Align::Min),
                    );
                }
            }
            for (index, line) in doc.lines.iter().enumerate() {
                if line.background && !appearance.show_backing_vocals {
                    continue;
                }
                if appearance.minimal_mode && line.end_ms.is_some_and(|end| end <= position) {
                    continue;
                }
                let singing = line.start_ms.is_some_and(|start| start <= position)
                    && line.end_ms.is_none_or(|end| position < end);
                let lit = ui.ctx().animate_bool_with_time(
                    egui::Id::new(("spicy-line", &now.uri, index)),
                    singing,
                    crate::motion::seconds(
                        ui.ctx(),
                        if appearance.simple_mode { 0.0 } else { 0.25 },
                    ),
                );
                let line_size = if line.background { size * 0.65 } else { size };
                let response = paint_line(
                    ui,
                    line,
                    position,
                    line_size,
                    lit,
                    app.palette.accent,
                    &appearance,
                );
                if response.clicked()
                    && let Some(start) = line.start_ms
                {
                    if ui.input(|input| input.modifiers.shift) {
                        app.actions.push(Action::CalibrateSpicyLyrics {
                            audio_ms: audio_position,
                            lyric_ms: start,
                        });
                    } else {
                        let target =
                            calibration.audio_position(start, app.settings.spicy_lyrics_offset_ms);
                        app.actions.push(Action::Seek(target));
                        app.actions.push(Action::SetSpicyFollow(true));
                    }
                }

                if scroll_active == Some(index) && follow {
                    let target = Rect::from_min_max(
                        pos2(
                            response.rect.left(),
                            response.rect.top() - viewport.height() * appearance.follow_position,
                        ),
                        response.rect.max,
                    );
                    ui.scroll_to_rect_animation(
                        target,
                        Some(Align::Min),
                        crate::motion::scroll(
                            ui.ctx(),
                            if appearance.smooth_scroll { 0.4 } else { 0.0 },
                        ),
                    );
                }
                if appearance.show_romanization
                    && let Some(text) = &line.romanization
                {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(text)
                                .size(line_size * 0.48)
                                .color(Color32::from_gray(200)),
                        )
                        .wrap(),
                    );
                }
                if appearance.show_translation
                    && let Some(text) = &line.translation
                {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(text)
                                .size(line_size * 0.48)
                                .color(Color32::from_gray(180)),
                        )
                        .wrap(),
                    );
                }
                ui.add_space(if line.background {
                    appearance.line_gap * 0.5
                } else {
                    appearance.line_gap
                });
            }
            ui.add_space(viewport.height() * 0.7);
        });
    if manual {
        app.actions.push(Action::SetSpicyFollow(false));
    }
    if !app
        .actions
        .iter()
        .any(|action| matches!(action, Action::SetSpicyFollow(true)))
    {
        app.actions.push(Action::SpicyLineShown(scroll_active));
    }
}

/// Lay out the whole line once, keeping its wrapping and font metrics fixed.
/// Highlight actual syllable ranges using the layout's UTF-8 cluster offsets;
/// a clipped second paint wipes across the syllable as it is sung. Glyphs
/// retain logical order through the shared bidi helper.
fn paint_line(
    ui: &mut egui::Ui,
    line: &Line,
    position: u32,
    size: f32,
    lit: f32,
    accent: Color32,
    appearance: &Appearance,
) -> egui::Response {
    let mut job = egui::text::LayoutJob::simple(
        line.text.clone(),
        match appearance.font_style {
            FontStyle::Bold => theme::bold(size),
            FontStyle::Regular => theme::regular(size),
            FontStyle::Monospace => egui::FontId::monospace(size),
        },
        Color32::PLACEHOLDER,
        ui.available_width(),
    );
    job.wrap.break_anywhere = false;
    job.halign = match appearance.alignment {
        TextAlign::Left => Align::LEFT,
        TextAlign::Center => Align::Center,
        TextAlign::Right => Align::RIGHT,
        TextAlign::Automatic if line.opposite => Align::RIGHT,
        TextAlign::Automatic => crate::bidi::halign_for(&line.text),
    };
    let mut galley = ui.painter().layout_job(job);
    crate::bidi::reorder(&mut galley);
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), galley.size().y),
        if line.start_ms.is_some() {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            if line.start_ms.is_some() {
                egui::WidgetType::Button
            } else {
                egui::WidgetType::Label
            },
            ui.is_enabled(),
            &line.text,
        )
    });
    if appearance.hover_background && response.hovered() {
        ui.painter()
            .rect_filled(rect.expand(6.0), 8.0, Color32::from_white_alpha(15));
    }
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let origin = crate::bidi::galley_pos(rect, &galley);
    let lift = if !appearance.simple_mode && crate::motion::seconds(ui.ctx(), 1.0) > 0.0 {
        lit * -2.0
    } else {
        0.0
    };
    let origin = origin + vec2(0.0, lift);
    let quiet = if line.start_ms.is_none() {
        210
    } else {
        (105.0 + 35.0 * lit) as u8
    };
    let base = if line.words.is_empty() && lit > 0.0 {
        super::blend(Color32::from_gray(quiet), Color32::WHITE, lit)
    } else {
        Color32::from_gray(quiet)
    };
    ui.painter().galley(origin, galley.clone(), base);
    let mut offset = 0;
    for word in &line.words {
        let range = offset..offset + word.text.len();
        offset = range.end + usize::from(!word.joined);
        let progress = word.progress(position);
        if progress <= 0.0 {
            continue;
        }
        for row in &galley.rows {
            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut rtl = false;
            for glyph in &row.glyphs {
                if range.contains(&(glyph.cluster as usize)) {
                    min_x = min_x.min(glyph.pos.x);
                    max_x = max_x.max(glyph.pos.x + glyph.advance_width);
                    rtl = glyph.rtl;
                }
            }
            if min_x < max_x {
                let width = (max_x - min_x) * progress;
                let left = if rtl { max_x - width } else { min_x };
                let clip = Rect::from_min_size(
                    origin + row.pos.to_vec2() + vec2(left, 0.0),
                    vec2(width, row.size.y),
                );
                let color = if progress < 1.0 {
                    if appearance.use_accent {
                        accent
                    } else {
                        Color32::WHITE
                    }
                } else {
                    super::blend(Color32::from_gray(105), Color32::WHITE, lit)
                };
                ui.painter()
                    .with_clip_rect(clip.intersect(ui.clip_rect()))
                    .galley(origin, galley.clone(), color);
            }
        }
    }
    response
}

fn background(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    theme::apply_local(ui, &theme::Palette::dark());
    let mut appearance = app.settings.spicy_lyrics_appearance.clone();
    appearance.normalize();
    let primary = app.now_playing_tint().unwrap_or(app.palette.accent);
    let secondary = Color32::from_rgb(primary.b(), primary.r(), primary.g());
    let solid = Color32::from_rgb(
        appearance.color[0],
        appearance.color[1],
        appearance.color[2],
    );
    ui.painter().rect_filled(
        rect,
        0.0,
        match appearance.background {
            Background::Theme => {
                app.palette
                    .window
                    .gamma_multiply(if app.palette.dark { 1.0 } else { 0.15 })
            }
            _ => solid,
        },
    );
    if matches!(
        appearance.background,
        Background::AnimatedColors | Background::CoverColors
    ) {
        let moving = appearance.background == Background::AnimatedColors
            && !appearance.simple_mode
            && crate::motion::seconds(ui.ctx(), 1.0) > 0.0;
        let time_id = egui::Id::new("spicy-background-time");
        let playing = app.now_playing().is_some_and(|now| now.playing);
        let time = if moving && playing {
            let time = ui.input(|input| input.time) as f32 * 0.13;
            ui.ctx().data_mut(|data| data.insert_temp(time_id, time));
            time
        } else if moving {
            ui.ctx()
                .data(|data| data.get_temp::<f32>(time_id).unwrap_or(0.0))
        } else {
            0.0
        };
        let mut mesh = egui::Mesh::default();
        let (cols, rows) = (16u32, 12u32);
        for y in 0..=rows {
            for x in 0..=cols {
                let u = x as f32 / cols as f32;
                let v = y as f32 / rows as f32;
                let mix = ((u * 3.7 + time).sin() + (v * 4.1 - time * 0.8).cos() + 2.0) * 0.25;
                mesh.colored_vertex(
                    pos2(
                        rect.left() + u * rect.width(),
                        rect.top() + v * rect.height(),
                    ),
                    super::blend(primary, secondary, mix),
                );
                if x < cols && y < rows {
                    let index = y * (cols + 1) + x;
                    mesh.add_triangle(index, index + 1, index + cols + 1);
                    mesh.add_triangle(index + 1, index + cols + 2, index + cols + 1);
                }
            }
        }
        ui.painter().add(egui::Shape::mesh(mesh));
    } else if matches!(
        appearance.background,
        Background::BlurredCover | Background::CoverArt | Background::CustomImage
    ) {
        let uri = if appearance.background == Background::CustomImage {
            appearance.custom_image.clone()
        } else {
            app.now_playing()
                .and_then(|now| now.art_url.or(now.art_small))
        };
        let blur = appearance.blur;
        if let Some(texture) =
            app.lyrics_backdrop
                .texture_with_blur(ui.ctx(), app.backend.art(), uri.as_deref(), blur)
        {
            let ratio = (rect.width() / rect.height().max(1.0))
                / (texture.size_vec2().x / texture.size_vec2().y.max(1.0));
            let size = if ratio > 1.0 {
                vec2(1.0, 1.0 / ratio)
            } else {
                vec2(ratio, 1.0)
            };
            let uv = Rect::from_center_size(pos2(0.5, 0.5), size);
            ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
        }
    }
    ui.painter().rect_filled(
        rect,
        0.0,
        Color32::from_black_alpha((appearance.darkness * 255.0) as u8),
    );
}

fn configuration(app: &mut App, ctx: &egui::Context) {
    let id = egui::Id::new(CONFIG_ID);
    let mut open = ctx.data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    if !open {
        return;
    }
    let draft_id = id.with("draft");
    let (mut key, mut offset) = ctx.data_mut(|data| {
        data.get_temp::<(String, i32)>(draft_id).unwrap_or_else(|| {
            (
                app.settings.spicy_lyrics_key.clone(),
                app.settings.spicy_lyrics_offset_ms,
            )
        })
    });
    let mut tab = ctx.data(|data| data.get_temp::<usize>(id.with("tab")).unwrap_or(0));
    let mut appearance = app.settings.spicy_lyrics_appearance.clone();
    appearance.normalize();
    if let Some(uri) = &app.spicy.background_pick {
        appearance.custom_image = Some(uri.clone());
        appearance.background = Background::CustomImage;
        app.actions.push(Action::SpicyBackgroundUsed);
    }
    let mut save_source = false;
    egui::Window::new("Spicy Lyrics settings").id(id.with("window"))
        .open(&mut open).collapsible(false).resizable(false).default_width(420.0)
        .max_width((ctx.content_rect().width() - 40.0).max(260.0))
        .max_height((ctx.content_rect().height() - 120.0).clamp(220.0, 520.0)).vscroll(true).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (index, title) in ["Background", "Lyrics", "Sync", "Sources"].iter().enumerate() {
                    if ui.selectable_label(tab == index, *title).clicked() { tab = index; }
                }
            });
            ui.separator();
            if tab < 2 {
                ui.horizontal_wrapped(|ui| {
                    ui.small("Presets");
                    if ui.button("Balanced").clicked() { appearance = Appearance::default(); }
                    if ui.button("Focus").clicked() {
                        appearance.show_cover = false; appearance.minimal_mode = true;
                        appearance.simple_mode = true; appearance.fps = 30; appearance.limit_fps = true;
                    }
                    if ui.button("Large text").clicked() { appearance.font_size = 48.0; }
                });
                ui.add_space(6.0);
            }
            match tab {
                0 => background_settings(app, ui, &mut appearance),
                1 => lyrics_settings(ui, &mut appearance),
                2 => sync_settings(app, ui, &mut offset),
                _ => {
                    ui.label("Spicy Lyrics catalogue");
                    ui.small("LRCLIB and More Lyrics work without a key. Add a publishable key for the official Spicy Lyrics catalogue.");
                    ui.hyperlink_to("Create a Spicy Lyrics application", "https://developers.spicylyrics.org/dashboard/applications");
                    ui.small("Enable client access and allow requests without an Origin header for native apps.");
                    ui.add(egui::TextEdit::singleline(&mut key).hint_text("sl_pk_…").desired_width(f32::INFINITY));
                    if !crate::spicy_lyrics::valid_key(key.trim()) {
                        ui.colored_label(app.palette.warning, "Use a publishable key starting with sl_pk_.");
                    }
                    if ui.add_enabled(crate::spicy_lyrics::valid_key(key.trim()), egui::Button::new("Save API key")).clicked() { save_source = true; }
                    ui.separator();
                    ui.label("Lyrics cache");
                    if ui.button("Refresh this song's lyrics").clicked() { app.actions.push(Action::RetrySpicyLyrics); }
                    ui.small("Reloads the current track and bypasses the native Spicy Lyrics cache.");
                }
            }
            ui.add_space(12.0);
            ui.separator();
            ui.small("Appearance changes are saved automatically. Timing and source options have their own controls.");
        });
    if appearance != app.settings.spicy_lyrics_appearance {
        app.actions.push(Action::SetSpicyAppearance(appearance));
    }
    if save_source {
        app.actions.push(Action::ConfigureSpicyLyrics {
            key: key.clone(),
            offset_ms: offset,
        });
    }
    ctx.data_mut(|data| {
        data.insert_temp(id, open);
        data.insert_temp(id.with("tab"), tab);
        if open {
            data.insert_temp(draft_id, (key, offset));
        } else {
            data.remove::<(String, i32)>(draft_id);
        }
    });
}

fn background_settings(app: &mut App, ui: &mut egui::Ui, appearance: &mut Appearance) {
    ui.label("Background style");
    let previous_background = appearance.background;
    egui::ComboBox::from_id_salt("spicy-background-style")
        .selected_text(background_label(appearance.background))
        .show_ui(ui, |ui| {
            for mode in [
                Background::BlurredCover,
                Background::AnimatedColors,
                Background::CoverArt,
                Background::CoverColors,
                Background::Solid,
                Background::Theme,
                Background::CustomImage,
            ] {
                ui.selectable_value(&mut appearance.background, mode, background_label(mode));
            }
        });
    if appearance.background != previous_background && appearance.background == Background::CoverArt
    {
        appearance.blur = 0.0;
    }
    if appearance.background == Background::CustomImage {
        ui.horizontal(|ui| {
            if ui.button("Choose image…").clicked() {
                app.actions.push(Action::ChooseSpicyBackground);
            }
            if ui.button("Remove image").clicked() {
                appearance.custom_image = None;
                appearance.background = Background::Solid;
            }
        });
        if let Some(uri) = &appearance.custom_image {
            let name = reqwest::Url::parse(uri)
                .ok()
                .and_then(|url| url.to_file_path().ok())
                .and_then(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                });
            ui.small(name.unwrap_or_else(|| "Selected image".into()));
        }
        ui.small("JPEG, PNG or WebP, up to 8 MB. The image stays on this computer.");
    }
    if matches!(
        appearance.background,
        Background::BlurredCover | Background::CoverArt | Background::CustomImage
    ) {
        ui.add(egui::Slider::new(&mut appearance.blur, 0.0..=48.0).text("Image blur"));
    }
    ui.add(egui::Slider::new(&mut appearance.darkness, 0.0..=0.95).text("Background darkness"));
    if matches!(
        appearance.background,
        Background::Solid | Background::CustomImage
    ) {
        ui.horizontal(|ui| {
            ui.label("Background / fallback color");
            ui.color_edit_button_srgb(&mut appearance.color);
        });
    }
    ui.checkbox(&mut appearance.show_cover, "Show large album cover");
    ui.add_space(12.0);
    ui.small("Dynamic Colors is a native animated gradient inspired by Spicy Lyrics. Simple mode and reduced-motion preferences pause the animation.");
    if ui.button("Reset background defaults").clicked() {
        let defaults = Appearance::default();
        appearance.background = defaults.background;
        appearance.blur = defaults.blur;
        appearance.darkness = defaults.darkness;
        appearance.color = defaults.color;
        appearance.show_cover = defaults.show_cover;
    }
}

fn background_label(background: Background) -> &'static str {
    match background {
        Background::BlurredCover => "Blurred cover",
        Background::AnimatedColors => "Dynamic colors (animated)",
        Background::CoverArt => "Cover art",
        Background::CoverColors => "Cover colors (static)",
        Background::Solid => "Solid color",
        Background::Theme => "App background (darkened)",
        Background::CustomImage => "Custom image",
    }
}

fn lyrics_settings(ui: &mut egui::Ui, appearance: &mut Appearance) {
    egui::ComboBox::from_id_salt("spicy-font-style")
        .selected_text(format!("Font: {:?}", appearance.font_style))
        .show_ui(ui, |ui| {
            for style in [FontStyle::Bold, FontStyle::Regular, FontStyle::Monospace] {
                ui.selectable_value(&mut appearance.font_style, style, format!("{style:?}"));
            }
        });
    ui.add(egui::Slider::new(&mut appearance.font_size, 20.0..=64.0).text("Text size"));
    ui.add(egui::Slider::new(&mut appearance.line_gap, 4.0..=48.0).text("Line spacing"));
    egui::ComboBox::from_id_salt("spicy-alignment")
        .selected_text(format!("{:?}", appearance.alignment))
        .show_ui(ui, |ui| {
            for alignment in [
                TextAlign::Automatic,
                TextAlign::Left,
                TextAlign::Center,
                TextAlign::Right,
            ] {
                ui.selectable_value(
                    &mut appearance.alignment,
                    alignment,
                    format!("{alignment:?}"),
                );
            }
        });
    ui.checkbox(&mut appearance.simple_mode, "Simple mode (fewer effects)");
    ui.checkbox(
        &mut appearance.minimal_mode,
        "Minimal mode (hide sung lines)",
    );
    ui.checkbox(&mut appearance.hover_background, "Highlight line on hover");
    ui.checkbox(
        &mut appearance.show_backing_vocals,
        "Show background vocals",
    );
    ui.checkbox(
        &mut appearance.use_accent,
        "Use accent color for sung words",
    );
    ui.checkbox(
        &mut appearance.show_romanization,
        "Show romanization when supplied",
    );
    ui.checkbox(
        &mut appearance.show_translation,
        "Show translation when supplied",
    );
    ui.checkbox(&mut appearance.smooth_scroll, "Smooth scrolling");
    ui.add(egui::Slider::new(&mut appearance.early_scroll_ms, 0..=800).text("Scroll ahead (ms)"));
    ui.add(
        egui::Slider::new(&mut appearance.follow_position, 0.15..=0.65)
            .text("Active line position"),
    );
    ui.checkbox(&mut appearance.limit_fps, "Limit animation frame rate");
    if appearance.limit_fps {
        ui.add(egui::Slider::new(&mut appearance.fps, 15..=240).text("Animation FPS"));
    }
    if ui.button("Reset lyrics display defaults").clicked() {
        let background = appearance.clone();
        *appearance = Appearance::default();
        appearance.background = background.background;
        appearance.blur = background.blur;
        appearance.darkness = background.darkness;
        appearance.color = background.color;
        appearance.custom_image = background.custom_image;
        appearance.show_cover = background.show_cover;
    }
}

fn sync_settings(app: &mut App, ui: &mut egui::Ui, global_offset: &mut i32) {
    ui.label("Automatic sync (local audio analysis)");
    let mut enabled = app.settings.spicy_lyrics_auto_sync;
    if ui
        .checkbox(
            &mut enabled,
            "Automatically match sung lyrics to this recording",
        )
        .changed()
    {
        app.actions.push(Action::SetSpicyAutoSync(enabled));
    }
    ui.small("Recognizes phrases locally with the bundled multilingual model. Audio stays on this computer. Analysis runs in the background and may take a few minutes on CPU.");
    let key = app.spicy_calibration_key();
    let manual = app
        .settings
        .spicy_lyrics_calibrations
        .get(&key)
        .is_some_and(|cal| !cal.points.is_empty() || cal.offset_ms != 0);
    if manual {
        ui.small("Your manual timing corrections take priority over automatic results.");
    }
    if app.spicy.auto_running && app.spicy.auto_key.as_deref() == Some(&key) {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Listening for matching lyric phrases…");
        });
    } else {
        if let Some(result) = app
            .settings
            .spicy_lyrics_auto_results
            .get(&key)
            .filter(|result| result.version == crate::auto_sync::ALIGNER_VERSION)
        {
            ui.label(format!(
                "{} distinct phrases matched · {} ms median agreement",
                result.matched_lines, result.median_error_ms
            ));
            let calibration = app.spicy_calibration();
            if let Some(now) = app.now_playing() {
                let position = app.spicy_audio_position(&now);
                let lyric_position =
                    calibration.lyric_position(position, app.settings.spicy_lyrics_offset_ms);
                if let Some(next) = app
                    .spicy
                    .result
                    .get()
                    .and_then(|r| r.document.as_ref())
                    .and_then(|doc| {
                        doc.lines.iter().find(|line| {
                            !line.background && line.start_ms.is_some_and(|at| at > lyric_position)
                        })
                    })
                {
                    let audio_at = calibration.audio_position(
                        next.start_ms.unwrap(),
                        app.settings.spicy_lyrics_offset_ms,
                    );
                    ui.small(format!(
                        "Next lyric in {:.1}s · {}",
                        audio_at.saturating_sub(position) as f64 / 1000.0,
                        next.text
                    ));
                }
            }
        }
        if app.spicy.auto_key.as_deref() == Some(&key)
            && let Some(message) = &app.spicy.auto_message
        {
            ui.small(message);
        }
        let available =
            app.now_playing().is_some_and(|now| now.local) && app.local.source.is_some();
        if ui
            .add_enabled(
                enabled && available,
                egui::Button::new("Analyze recording again"),
            )
            .clicked()
        {
            app.actions.push(Action::AnalyzeSpicyAudio);
        }
        if !available {
            ui.small("Available for audio playing locally in Vibefy.");
        }
    }
    ui.separator();
    ui.label("Global timing adjustment");
    ui.add(
        egui::DragValue::new(global_offset)
            .speed(50)
            .range(-30_000..=30_000)
            .suffix(" ms"),
    );
    ui.small("Positive shows lyrics earlier; negative delays them.");
    if ui.button("Save global timing").clicked() {
        app.actions.push(Action::ConfigureSpicyLyrics {
            key: app.settings.spicy_lyrics_key.clone(),
            offset_ms: *global_offset,
        });
    }
    ui.separator();
    let Some(now) = app.now_playing() else {
        ui.label("Play a song to calibrate it.");
        return;
    };
    let calibration_key = app.spicy_calibration_key();
    let calibration = app
        .settings
        .spicy_lyrics_calibrations
        .get(&calibration_key)
        .cloned()
        .unwrap_or_default();
    ui.label("This recording only");
    let mut track_offset = calibration.offset_ms;
    if ui
        .add(
            egui::DragValue::new(&mut track_offset)
                .speed(50)
                .range(-120_000..=120_000)
                .suffix(" ms"),
        )
        .changed()
    {
        app.actions.push(Action::SetSpicyTrackOffset(track_offset));
    }
    if let Some(source) = &app.local.source {
        ui.hyperlink_to(
            &source.title,
            format!("https://www.youtube.com/watch?v={}", source.youtube_id),
        );
        let difference = i64::from(source.duration_ms) - i64::from(now.duration_ms);
        ui.small(format!(
            "Source: {:.1}s · Reference: {:.1}s · Difference: {:+.1}s",
            source.duration_ms as f64 / 1000.0,
            now.duration_ms as f64 / 1000.0,
            difference as f64 / 1000.0
        ));
        let comparison = crate::auto_sync::TimingComparison {
            reference_ms: now.duration_ms,
            source_ms: source.duration_ms,
        };
        if let Some(ratio) = comparison.ratio() {
            ui.small(format!("Duration ratio: {ratio:.5}× · checked immediately; vocal matches decide offset, speed or local verse correction."));
        }
        if difference.abs() > 3000 {
            ui.colored_label(app.palette.warning, "Different durations can indicate a different edit. A fixed offset may not be enough.");
        }
        if ui
            .button("Search recording again (restarts song)")
            .clicked()
        {
            app.actions.push(Action::RematchSpicyAudio);
        }
    }
    ui.separator();
    ui.label("Sync points");
    ui.small("Shift+click a timed lyric as you hear its first word. One point fixes an intro offset; two or more also correct drift between points. Points and offsets are saved for this recording and lyrics version.");
    if let Some(found) = app.spicy.result.get().cloned()
        && let Some(doc) = &found.document
    {
        let audio_ms = app.spicy_audio_position(&now);
        let active = doc
            .active_line(
                app.spicy_calibration()
                    .lyric_position(audio_ms, app.settings.spicy_lyrics_offset_ms),
            )
            .unwrap_or(0);
        let id = egui::Id::new(("spicy-sync-selection", &calibration_key));
        let mut selected = ui
            .ctx()
            .data(|data| data.get_temp::<usize>(id))
            .unwrap_or(active)
            .min(doc.lines.len().saturating_sub(1));
        if !doc.lines.is_empty() {
            egui::ComboBox::from_id_salt("spicy-sync-line")
                .selected_text(&doc.lines[selected].text)
                .width(430.0)
                .show_ui(ui, |ui| {
                    for (index, line) in doc
                        .lines
                        .iter()
                        .enumerate()
                        .filter(|(_, line)| line.start_ms.is_some() && !line.background)
                    {
                        ui.selectable_value(&mut selected, index, &line.text);
                    }
                });
            ui.ctx().data_mut(|data| data.insert_temp(id, selected));
            if ui
                .add_enabled(
                    doc.lines[selected].start_ms.is_some(),
                    egui::Button::new("This line starts now"),
                )
                .clicked()
                && let Some(lyric_ms) = doc.lines[selected].start_ms
            {
                app.actions
                    .push(Action::CalibrateSpicyLyrics { audio_ms, lyric_ms });
            }
        }
    }
    for (index, point) in calibration.points.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.small(format!(
                "{} · Audio {:.2}s → Lyrics {:.2}s",
                index + 1,
                point.audio_ms as f64 / 1000.0,
                point.lyric_ms as f64 / 1000.0
            ));
            if ui.small_button("Remove").clicked() {
                app.actions.push(Action::RemoveSpicySyncPoint(index));
            }
        });
    }
    if ui.button("Reset manual timing").clicked() {
        app.actions.push(Action::ResetSpicyCalibration);
    }
    ui.small("Different cuts, missing verses or incorrect lyric timestamps cannot be repaired reliably by stretching the whole song's duration.");
}
