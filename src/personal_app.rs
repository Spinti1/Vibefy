use crate::{
    app::App,
    backend::Command,
    model::Page,
    personal_library::{self as library, Action as Edit, Output, Pending, Task},
};

impl App {
    pub fn library_access(&self) -> bool {
        self.settings.welcome_completed
            || (self.is_connected() && self.user.is_some())
            || self.music_accounts[0].connected
    }
    fn personal_request(
        &mut self,
        task: Task,
        source: Option<String>,
        backup: bool,
        fingerprint: String,
        message: &str,
    ) {
        self.personal.next = self.personal.next.wrapping_add(1);
        let request = self.personal.next;
        self.personal.tasks.insert(
            request,
            Pending {
                source,
                backup,
                fingerprint,
                epoch: self.music_accounts_serial[0],
                message: message.into(),
            },
        );
        self.backend.send(Command::Personal { request, task });
    }
    pub(super) fn tick_personal_library(&mut self) {
        if self.personal.checked_revision != Some(self.addons.collections_revision) {
            self.personal.current = self
                .addons
                .collections
                .iter()
                .filter(|c| {
                    self.settings.playlist_backups.contains_key(&c.source)
                        || self.personal.backups.contains_key(&c.source)
                })
                .map(|c| (c.source.clone(), library::fingerprint(c)))
                .collect();
            self.personal.checked_revision = Some(self.addons.collections_revision);
        }
        if self.library_access() && !self.personal.started {
            self.personal.started = true;
            self.settings.welcome_completed = true;
            self.settings_dirty = true;
            self.backend.send(Command::GuestPlayback(true));
        }
        let observed = self.music_accounts[0]
            .connected
            .then_some(self.music_accounts_serial[0]);
        if self.personal.observed != observed {
            self.personal.observed = observed;
            self.personal.scope = None;
            self.personal.backups.clear();
            self.personal.checked_revision = None;
            self.personal.tasks.retain(|_, p| !p.backup);
            if observed.is_some() && !self.music_accounts[0].busy {
                self.personal_request(
                    Task::Account,
                    None,
                    true,
                    String::new(),
                    "Checking YouTube backups…",
                );
            } else if observed.is_some() {
                self.personal.observed = None;
            }
        }
        if self.music_accounts[0].busy || self.personal.tasks.values().any(|p| p.backup) {
            return;
        }
        let Some(scope) = self.personal.scope.clone() else {
            return;
        };
        let next = self
            .addons
            .collections
            .iter()
            .find(|c| {
                self.settings.playlist_backups.get(&c.source) == Some(&scope)
                    && self.personal.backups.get(&c.source).is_none_or(|r| {
                        (r.message.is_none()
                            || (r.retry_at > 0 && crate::playlist_mirror::now() >= r.retry_at))
                            && self.personal.current.get(&c.source) != Some(&r.fingerprint)
                    })
            })
            .map(|c| c.source.clone());
        if let Some(source) = next {
            self.apply_personal(Edit::Backup(source));
        }
    }
    pub(super) fn apply_personal(&mut self, action: Edit) {
        match action {
            Edit::Show(dialog) => {
                self.personal.seed_tracks.clear();
                self.personal.message = None;
                self.personal.dialog = Some(dialog);
            }
            Edit::CreateFrom(tracks) => {
                self.personal.seed_tracks = tracks;
                self.personal.message = None;
                self.personal.dialog = Some(library::Dialog::New {
                    name: String::new(),
                    backup: self.music_accounts[0].connected,
                });
            }
            Edit::Close => self.personal.dialog = None,
            Edit::Continue => {
                self.settings.welcome_completed = true;
                self.settings_dirty = true;
                self.backend.send(Command::GuestPlayback(true));
                self.open(Page::Home);
            }
            Edit::Create { name, backup } => {
                if !crate::playlist_mirror::valid_title(&name) {
                    self.personal.message = Some("Enter a playlist name (1–100 characters)".into());
                    return;
                }
                let source = library::new_source();
                let mut collection = crate::addons::Collection {
                    title: name.trim().into(),
                    source: source.clone(),
                    owner: Some("Your Vibefy playlist".into()),
                    ..Default::default()
                };
                library::append(
                    &mut collection,
                    std::mem::take(&mut self.personal.seed_tracks),
                );
                if self.save_source_collection(collection) {
                    self.personal.dialog = None;
                    self.open(Page::LocalCollection(source.clone()));
                    if backup {
                        self.apply_personal(Edit::Backup(source));
                    }
                }
            }
            Edit::Rename { source, name } => {
                if !crate::playlist_mirror::valid_title(&name) {
                    return;
                }
                if let Some(list) = self
                    .addons
                    .collections
                    .iter_mut()
                    .find(|c| c.source == source && library::owned(&source))
                {
                    list.title = name.trim().into();
                    self.personal_changed();
                    self.personal.dialog = None;
                }
            }
            Edit::Delete(source) => {
                self.addons.collections.retain(|c| c.source != source);
                self.settings.playlist_backups.remove(&source);
                self.settings_dirty = true;
                self.personal_changed();
                self.personal.dialog = None;
                self.open(Page::Home);
            }
            Edit::Add { source, tracks } => {
                if let Some(mut list) = self
                    .addons
                    .collections
                    .iter()
                    .find(|c| c.source == source && library::owned(&source))
                    .cloned()
                {
                    library::append(&mut list, tracks);
                    if self.save_source_collection(list) {
                        self.toast("Added to your Vibefy playlist");
                    }
                }
            }
            Edit::Remove { source, uri } => {
                if let Some(list) = self
                    .addons
                    .collections
                    .iter_mut()
                    .find(|c| c.source == source && library::owned(&source))
                {
                    list.tracks.retain(|t| t.uri != uri);
                    self.personal_changed();
                }
            }
            Edit::Import {
                youtube,
                url,
                backup,
            } => {
                if self.personal.tasks.values().any(|p| !p.backup) {
                    return;
                }
                if (youtube && crate::addons::playlist_url(&url).is_err())
                    || (!youtube && library::spotify_id(&url).is_none())
                {
                    self.personal.message =
                        Some("Paste a valid playlist link from the selected service".into());
                    return;
                }
                let source = if youtube {
                    crate::addons::playlist_url(&url).unwrap()
                } else {
                    format!(
                        "https://open.spotify.com/playlist/{}",
                        library::spotify_id(&url).unwrap()
                    )
                };
                // An existing import is refreshed in place, never duplicated.
                let fingerprint = if backup {
                    "backup".into()
                } else {
                    String::new()
                };
                self.personal_request(
                    Task::Import { youtube, url },
                    Some(source),
                    false,
                    fingerprint,
                    "Importing playlist…",
                );
            }
            Edit::Backup(source) => {
                let Some(scope) = self.personal.scope.clone() else {
                    self.personal.message = Some(
                        "Connect YouTube in Settings, then save this playlist to your account"
                            .into(),
                    );
                    return;
                };
                if self
                    .personal
                    .tasks
                    .values()
                    .any(|p| p.backup && p.source.as_deref() == Some(&source))
                {
                    return;
                }
                let Some(collection) = self.source_collection(&source).cloned() else {
                    return;
                };
                let fingerprint = library::fingerprint(&collection);
                self.settings
                    .playlist_backups
                    .insert(source.clone(), scope.clone());
                self.personal.checked_revision = None;
                self.settings_dirty = true;
                self.personal_request(
                    Task::Backup { collection, scope },
                    Some(source),
                    true,
                    fingerprint,
                    "Saving playlist to YouTube…",
                );
            }
            Edit::Export | Edit::Restore => {
                let export =
                    matches!(action, Edit::Export).then(|| self.addons.collections.clone());
                let dialog =
                    rfd::AsyncFileDialog::new().add_filter("Vibefy playlist backup", &["json"]);
                let selected = if export.is_some() {
                    Box::pin(dialog.set_file_name("vibefy-playlists.json").save_file())
                        as std::pin::Pin<
                            Box<dyn std::future::Future<Output = Option<rfd::FileHandle>> + Send>,
                        >
                } else {
                    Box::pin(dialog.pick_file())
                };
                self.personal.next = self.personal.next.wrapping_add(1);
                let request = self.personal.next;
                self.personal.tasks.insert(
                    request,
                    Pending {
                        source: None,
                        backup: false,
                        fingerprint: String::new(),
                        epoch: 0,
                        message: "Playlist backup…".into(),
                    },
                );
                self.backend.send(Command::PersonalFiles {
                    request,
                    export,
                    selected,
                });
            }
        }
    }
    fn personal_changed(&mut self) {
        self.addons.collections_revision = self.addons.collections_revision.wrapping_add(1);
        self.save_addon_collections();
    }
    pub(super) fn receive_personal(&mut self, request: u64, result: Result<Output, String>) {
        let Some(pending) = self.personal.tasks.remove(&request) else {
            return;
        };
        if pending.backup
            && (pending.epoch != self.music_accounts_serial[0] || !self.music_accounts[0].connected)
        {
            return;
        }
        match result {
            Ok(Output::Recovered{collection,scope,report})=>{
                if pending.epoch!=self.music_accounts_serial[0] || !self.music_accounts[0].connected {return;}
                let source=collection.source.clone();
                if self.source_collection(&source).is_none() {self.save_source_collection(collection);}
                self.settings.playlist_backups.insert(source.clone(),scope.clone());self.settings_dirty=true;
                self.personal.scope=Some(scope);self.personal.backups.insert(source.clone(),report);
                self.personal.dialog=None;self.open(Page::LocalCollection(source));
            }
            Ok(Output::Imported(collection))=>{
                let source=collection.source.clone();
                if self.save_source_collection(collection) {self.personal.dialog=None;self.open(Page::LocalCollection(source.clone()));self.personal.message=Some("Playlist imported and saved on this device".into());if pending.fingerprint=="backup" && pending.epoch==self.music_accounts_serial[0] {self.apply_personal(Edit::Backup(source));}}
            }
            Ok(Output::Restored(lists))=>{
                let mut count=0;for collection in lists {if self.save_source_collection(collection){count+=1;}}
                self.personal.message=Some(format!("Restored {count} playlists"));self.open(Page::Home);
            }
            Ok(Output::Exported)=>self.personal.message=Some("Playlist backup exported. It contains metadata only, without music or account credentials.".into()),
            Ok(Output::Account{scope,reports})=>{self.personal.scope=Some(scope);self.personal.backups=reports;}
            Ok(Output::Backed{source,report})=>{
                self.personal.message=Some(if let Some(message)=&report.message {format!("Saved locally. YouTube backup: {message}")} else if report.missing>0 {format!("YouTube copy saved; {} songs could not be matched. Your complete local playlist is kept.",report.missing)} else {"Playlist saved to YouTube".into()});
                self.personal.backups.insert(source,report);
            }
            Err(error)=>{
                if pending.backup && let Some(source)=pending.source {self.personal.backups.entry(source).or_default().message=Some(error.clone());}
                if !error.contains("selection cancelled") {self.personal.message=Some(error);}
            }
        }
    }
    pub fn personal_status(&self, source: &str) -> String {
        if self
            .personal
            .tasks
            .values()
            .any(|p| p.source.as_deref() == Some(source) && p.backup)
        {
            return "Saved locally · Syncing to YouTube…".into();
        }
        if let Some(report) = self.personal.backups.get(source) {
            if report.message.is_some() {
                return "Saved locally · YouTube backup needs attention".into();
            }
            if self.personal.current.get(source) == Some(&report.fingerprint) {
                return if report.missing == 0 {
                    "Saved locally · Saved to YouTube".into()
                } else {
                    format!(
                        "Saved locally · {} songs not matched on YouTube",
                        report.missing
                    )
                };
            }
        }
        "Saved locally · This device".into()
    }
}
