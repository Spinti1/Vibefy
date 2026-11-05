//! Welcome screen with optional YouTube connection and local playback.
use crate::{app::App, model::Action, theme};
use egui::{Align, CornerRadius, Frame, Layout, Margin, Rect, Stroke, Vec2, pos2};

pub fn show(app: &mut App, ui: &mut egui::Ui, _connecting: bool) {
    let p = app.palette;
    egui::CentralPanel::default().frame(Frame::new().fill(p.window)).show(ui, |ui| {
        let rect=ui.max_rect();
        fastframe_titlebar::drag(ui, rect);
        super::widgets::paint_corner_gradient(ui,rect,super::blend(p.window,egui::Color32::from_rgb(65,248,170),0.22),super::blend(p.window,egui::Color32::from_rgb(0,210,245),0.25),p.window,p.window);
        let width=rect.width().min(440.0);
        let height=(rect.height()-64.0).clamp(100.0,560.0);
        let card=Rect::from_center_size(rect.center()-egui::vec2(0.0,16.0),Vec2::new(width,height));
        let mut child=ui.new_child(egui::UiBuilder::new().max_rect(card).layout(Layout::top_down(Align::Center)));
        Frame::new().fill(p.panel).stroke(Stroke::new(1.0,p.outline)).corner_radius(CornerRadius::same(theme::RADIUS+8)).inner_margin(Margin::same(30)).show(&mut child,|ui|{
            egui::ScrollArea::vertical().id_salt("welcome-scroll").max_height((height-60.0).max(40.0)).show(ui,|ui|{
                ui.set_width((width-60.0).max(100.0));
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    let (logo,_)=ui.allocate_exact_size(Vec2::splat(132.0),egui::Sense::hover());theme::brand_logo(ui,logo.center(),132.0);
                    ui.add_space(8.0);
                    theme::text(ui,"Your music, your playlists",theme::semibold(20.0),p.text);
                    ui.add_space(8.0);
                    ui.add(egui::Label::new(egui::RichText::new("Connect YouTube to load your library and save playlists to your account, or continue without an account.").color(p.secondary)).wrap());
                    ui.add_space(18.0);
                    let busy=app.music_accounts[0].busy;
                    ui.add_enabled_ui(!busy, |ui| {if big_button(ui,app,"Connect YouTube") {super::music_accounts::send(app,crate::music_accounts::Provider::YouTubeMusic,crate::music_accounts::Action::Connect);}});
                    if busy {ui.spinner();ui.label("Finish signing in in the YouTube window…");}
                    if let Some(error)=&app.music_accounts[0].message {ui.colored_label(p.danger,error);}
                    ui.add_space(10.0);
                    if theme::pill_button(ui,&p,"Skip · Continue as guest",false).clicked(){app.actions.push(Action::Personal(crate::personal_library::Action::Continue));}
                    ui.add_space(14.0);
                    ui.add(egui::Label::new(egui::RichText::new("Import public Spotify playlists from Home using their links. No Spotify login is required. Private playlists and Liked Songs are not supported.").size(12.0).color(p.secondary)).wrap());
                });
            });
        });
        ui.painter().text(pos2(rect.center().x,rect.bottom()-18.0),egui::Align2::CENTER_BOTTOM,format!("Vibefy {}",env!("CARGO_PKG_VERSION")),theme::regular(11.5),p.dim);
    });
}

pub(super) fn big_button(ui: &mut egui::Ui, app: &App, label: &str) -> bool {
    let palette = app.palette;
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), theme::bold(15.0), palette.on_accent);
    let size = Vec2::new(ui.available_width().min(300.0), 46.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if !ui.is_enabled() {
        ui.painter()
            .rect_filled(rect, 23.0, palette.accent.gamma_multiply(0.45));
    } else if palette.dark {
        let (left, right) = if response.hovered() {
            (
                egui::Color32::from_rgb(105, 255, 195),
                egui::Color32::from_rgb(50, 235, 255),
            )
        } else {
            (
                egui::Color32::from_rgb(65, 248, 170),
                egui::Color32::from_rgb(0, 210, 245),
            )
        };
        theme::paint_pill_gradient(ui.painter(), rect, left, right);
    } else {
        let fill = if response.hovered() {
            palette.accent_hover
        } else {
            palette.accent
        };
        ui.painter().rect_filled(rect, 23.0, fill);
    }
    ui.painter().galley(
        rect.center() - galley.size() / 2.0,
        galley,
        palette.on_accent,
    );
    response.clicked()
}
