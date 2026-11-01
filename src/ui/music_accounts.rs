use crate::{
    app::App,
    backend::Command,
    music_accounts::{Action, Provider},
    theme,
};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    for provider in [Provider::YouTubeMusic, Provider::JuiceVault] {
        let index = provider as usize;
        let mut action = None;
        ui.push_id(provider.key_id(),|ui|{
            theme::text(ui,provider.name(),theme::semibold(15.0),palette.text);
            let state=&mut app.music_accounts[index];
            if provider==Provider::JuiceVault{
                ui.label("JuiceVault username (for public playlists)");
                ui.add(egui::TextEdit::singleline(&mut state.username).hint_text("Your JuiceVault username"));
            }
            ui.horizontal_wrapped(|ui|{
                if state.busy{
                    ui.spinner();ui.label("Connecting…");
                    if theme::pill_button(ui,&palette,"Cancel",false).clicked(){
                        state.busy=false;app.music_accounts_serial[index]=app.music_accounts_serial[index].wrapping_add(1);
                        app.backend.send(Command::CancelMusicAccount(provider));
                    }
                }else{
                    if theme::pill_button(ui,&palette,if provider == Provider::YouTubeMusic {"Import browser session"} else if state.connected {"Reconnect"} else {"Connect"},false).clicked(){action=Some(Action::Connect);}
                    if theme::pill_button(ui,&palette,"Load playlists",false).clicked(){action=Some(Action::Refresh);}
                    if state.connected&&theme::pill_button(ui,&palette,"Disconnect",false).clicked(){action=Some(Action::Disconnect);}
                }
            });
            if !state.name.is_empty(){ui.label(format!("Connected: {}",state.name));}
            if provider == Provider::YouTubeMusic {
                if theme::pill_button(ui, &palette, "Open YouTube Music", false).clicked() {
                    let _ = crate::opener::open("https://music.youtube.com/library/playlists");
                }
                egui::CollapsingHeader::new("How to connect the browser session").default_open(!state.connected).show(ui, |ui| {
                    ui.label("1. Sign in on music.youtube.com in your browser.");
                    ui.label("2. Open Developer Tools > Network, filter by browse, then open Library > Playlists.");
                    ui.label("3. Copy the request headers of a POST request to music.youtube.com/youtubei/v1/browse into a local UTF-8 .txt file (or use ytmusicapi browser.json).");
                    ui.label("4. Click Connect here and select that file. Vibefy checks the session and saves a protected local copy.");
                    ui.label("Session headers grant account access. Keep the import file private, delete it after connecting, and never send it in chat or upload it.");
                });
            }
            let detail=match provider{
                Provider::YouTubeMusic=>"Uses your YouTube Music browser session to load saved playlists. No Google Cloud project or OAuth Client ID is required. Reconnect if the session expires.",
                Provider::JuiceVault=>"Sign in on juicevault.xyz. The documented library API currently exposes public playlists; private playlist access is not available here.",
            };
            theme::text(ui,detail,theme::regular(12.0),palette.secondary);
            if let Some(error)=&state.message{theme::text(ui,error,theme::regular(12.0),palette.secondary);}
            egui::CollapsingHeader::new(format!("Playlists ({})",state.playlists.len())).show(ui,|ui|{
                egui::ScrollArea::vertical().max_height(240.0).show(ui,|ui|{
                    for playlist in &state.playlists{
                        ui.horizontal(|ui|{
                            super::widgets::cover(ui,&palette,playlist.image.as_deref(),36.0,4.0,theme::Icon::Music);
                            ui.label(&playlist.title);
                            if ui.add_enabled(!state.busy,egui::Button::new("Save on Home")).clicked(){action=Some(Action::Import(playlist.clone()));}
                        });
                    }
                });
            });
            ui.add_space(12.0);
        });
        if let Some(action) = action {
            app.music_accounts_serial[index] = app.music_accounts_serial[index].wrapping_add(1);
            let state = &mut app.music_accounts[index];
            state.busy = true;
            state.message = None;
            if provider == Provider::YouTubeMusic && matches!(action, Action::Connect) {
                app.backend
                    .choose_music_session(app.music_accounts_serial[index]);
                continue;
            }
            app.backend.send(Command::MusicAccount {
                selected: None,
                provider,
                action,
                username: state.username.clone(),
                serial: app.music_accounts_serial[index],
            });
        }
    }
    super::playlist_mirror::show(app, ui);
}
