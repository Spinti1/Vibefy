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
            theme::apply_local(ui, &theme::Palette::dark());
            let appearance = app.settings.spicy_lyrics_appearance.clone();
            super::music_background::paint(app, ui, bounds, &appearance);
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
            "Lyrics are not available for this track.",
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
    });
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

fn configuration(app: &mut App, ctx: &egui::Context) {
    let id = egui::Id::new(CONFIG_ID);
    let mut open = ctx.data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    if !open {
        return;
    }
    let mut tab = ctx
        .data(|data| data.get_temp::<usize>(id.with("tab")).unwrap_or(0))
        .min(1);
    let mut name = ctx.data(|data| data.get_temp::<String>(id.with("name")).unwrap_or_default());
    let mut appearance = app.settings.spicy_lyrics_appearance.clone();
    appearance.normalize();
    if let Some(uri) = &app.spicy.background_pick {
        appearance.custom_image = Some(uri.clone());
        appearance.background = Background::CustomImage;
        app.actions.push(Action::SpicyBackgroundUsed);
    }
    egui::Window::new("Spicy Lyrics settings")
        .id(id.with("window"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .max_width((ctx.content_rect().width() - 40.0).max(260.0))
        .max_height((ctx.content_rect().height() - 120.0).clamp(220.0, 600.0))
        .vscroll(true)
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (index, title) in ["Background", "Lyrics"].iter().enumerate() {
                    ui.selectable_value(&mut tab, index, *title);
                }
            });
            ui.separator();
            if tab == 0 {
                background_settings(app, ui, &mut appearance, true);
            } else {
                lyrics_settings(ui, &mut appearance);
            }
            saved_styles(app, ui, &mut appearance, &mut name);
        });
    if appearance != app.settings.spicy_lyrics_appearance {
        app.actions.push(Action::SetSpicyAppearance(appearance));
    }
    ctx.data_mut(|data| {
        data.insert_temp(id, open);
        data.insert_temp(id.with("tab"), tab);
        data.insert_temp(id.with("name"), name);
    });
}

pub(super) fn background_settings(
    app: &mut App,
    ui: &mut egui::Ui,
    appearance: &mut Appearance,
    show_cover: bool,
) {
    ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
    ui.label("Style");
    let previous = appearance.background;
    egui::Grid::new(ui.id().with("background-styles"))
        .num_columns(3)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            for (index, mode) in [
                Background::BlurredCover,
                Background::CoverArt,
                Background::CustomImage,
                Background::AnimatedColors,
                Background::Aurora,
                Background::Fluid,
                Background::Orbit,
                Background::BeatGlow,
                Background::BeatWaves,
            ]
            .into_iter()
            .enumerate()
            {
                let selected = appearance.background == mode;
                if ui
                    .add_sized(
                        [104.0, 32.0],
                        egui::Button::new(background_label(mode)).selected(selected),
                    )
                    .clicked()
                {
                    appearance.background = mode;
                }
                if index % 3 == 2 {
                    ui.end_row();
                }
            }
        });
    if previous != appearance.background {
        if appearance.background == Background::CoverArt {
            appearance.blur = 0.0;
        }
        if appearance.background == Background::BlurredCover {
            appearance.blur = 18.0;
        }
    }
    if appearance.background == Background::CustomImage {
        ui.horizontal(|ui| {
            if ui.button("Choose image…").clicked() {
                app.actions.push(Action::ChooseSpicyBackground);
            }
            if appearance.custom_image.is_some() && ui.button("Remove").clicked() {
                appearance.custom_image = None;
                appearance.background = Background::BlurredCover;
            }
        });
    }
    ui.add_space(2.0);
    ui.add(
        egui::Slider::new(&mut appearance.darkness, 0.0..=0.95)
            .text("Darkness")
            .show_value(false),
    );
    if crate::backgrounds::reactive(appearance.background) {
        ui.add(
            egui::Slider::new(&mut appearance.reactivity, 0.0..=2.0)
                .text("Beat strength")
                .show_value(false),
        )
        .on_hover_text("Follows audio playing in Vibefy.");
    }
    if show_cover {
        ui.checkbox(&mut appearance.show_cover, "Show cover");
    }
    if details_toggle(ui, "background-details", "More options") {
        if crate::backgrounds::animated(appearance.background) {
            ui.add(
                egui::Slider::new(&mut appearance.animation_speed, 0.2..=2.0)
                    .text("Speed")
                    .show_value(false),
            );
        }
        if matches!(
            appearance.background,
            Background::BlurredCover | Background::CoverArt | Background::CustomImage
        ) {
            ui.add(
                egui::Slider::new(&mut appearance.blur, 0.0..=48.0)
                    .text("Blur")
                    .show_value(false),
            );
        }
        if ui.small_button("Reset background").clicked() {
            let defaults = Appearance::default();
            appearance.background = defaults.background;
            appearance.animation_speed = defaults.animation_speed;
            appearance.reactivity = defaults.reactivity;
            appearance.blur = defaults.blur;
            appearance.darkness = defaults.darkness;
        }
    }
}

pub(super) fn details_toggle(ui: &mut egui::Ui, salt: &str, label: &str) -> bool {
    let id = ui.id().with(salt);
    let mut open = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    if ui
        .selectable_label(open, if open { "Less options" } else { label })
        .clicked()
    {
        open = !open;
        ui.ctx().data_mut(|data| data.insert_temp(id, open));
    }
    open
}

fn background_label(background: Background) -> &'static str {
    match background {
        Background::BlurredCover => "Blurred cover",
        Background::AnimatedColors => "Dynamic colors",
        Background::Aurora => "Aurora",
        Background::Fluid => "Fluid",
        Background::Orbit => "Orbit",
        Background::BeatGlow => "Beat Glow",
        Background::BeatWaves => "Beat Waves",
        Background::CoverArt => "Cover art",
        Background::CustomImage => "Custom image",
    }
}

fn lyrics_settings(ui: &mut egui::Ui, appearance: &mut Appearance) {
    ui.label("Font style");
    ui.horizontal_wrapped(|ui| {
        for style in [FontStyle::Bold, FontStyle::Regular, FontStyle::Monospace] {
            ui.selectable_value(&mut appearance.font_style, style, format!("{style:?}"));
        }
    });
    ui.add(egui::Slider::new(&mut appearance.font_size, 20.0..=64.0).text("Text size"));
    ui.add(egui::Slider::new(&mut appearance.line_gap, 4.0..=48.0).text("Line spacing"));
    ui.label("Text alignment");
    ui.horizontal_wrapped(|ui| {
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
    ui.add_space(8.0);
    ui.label("Text effects");
    ui.horizontal_wrapped(|ui| {
        toggle(ui, &mut appearance.simple_mode, "Simple text");
        toggle(ui, &mut appearance.minimal_mode, "Hide sung lines");
        toggle(ui, &mut appearance.hover_background, "Hover highlight");
        toggle(ui, &mut appearance.use_accent, "Accent words");
        toggle(ui, &mut appearance.smooth_scroll, "Smooth scrolling");
    });
    ui.label("Additional lines · when supplied by the provider");
    ui.horizontal_wrapped(|ui| {
        toggle(ui, &mut appearance.show_backing_vocals, "Backing vocals");
        toggle(ui, &mut appearance.show_romanization, "Romanization");
        toggle(ui, &mut appearance.show_translation, "Translation");
    });
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
        appearance.animation_speed = background.animation_speed;
        appearance.reactivity = background.reactivity;
        appearance.blur = background.blur;
        appearance.darkness = background.darkness;
        appearance.color = background.color;
        appearance.custom_image = background.custom_image;
        appearance.show_cover = background.show_cover;
    }
}

fn toggle(ui: &mut egui::Ui, enabled: &mut bool, text: &str) {
    if ui.selectable_label(*enabled, text).clicked() {
        *enabled = !*enabled;
    }
}

pub(super) fn saved_styles(
    app: &mut App,
    ui: &mut egui::Ui,
    appearance: &mut Appearance,
    name: &mut String,
) {
    ui.add_space(6.0);
    if !details_toggle(ui, "saved-styles", "Saved styles") {
        return;
    }
    ui.separator();
    ui.strong("Your saved styles");
    ui.small("Save your current look.");
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(name)
                .hint_text("Style name")
                .char_limit(48)
                .desired_width(210.0),
        );
        if ui
            .add_enabled(!name.trim().is_empty(), egui::Button::new("Save style"))
            .clicked()
        {
            app.actions.push(Action::SaveSpicyStyle(
                name.trim().to_owned(),
                appearance.clone(),
            ));
            name.clear();
        }
    });
    for saved in &app.settings.spicy_lyrics_styles {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(saved.appearance == *appearance, &saved.name)
                .clicked()
            {
                *appearance = saved.appearance.clone();
            }
            if ui
                .small_button("Delete")
                .on_hover_text("Delete saved style")
                .clicked()
            {
                app.actions
                    .push(Action::DeleteSpicyStyle(saved.name.clone()));
            }
        });
    }
    ui.small("Changes are saved automatically. Saving an existing name replaces that style.");
}
