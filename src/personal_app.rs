use crate::{
    app::App,
    backend::Command,
    model::Page,
    personal_library::{self as library, Action as Edit, Output, Pending, Task},
};

impl App {
    pub fn source_collection(&self, source: &str) -> Option<&crate::addons::Collection> {
        self.addons.collections.iter().find(|c| c.source == source)
    }
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
        self.tick_spotify_copies();
        let revision=self.addons.collections_revision;
        if self.settings.phone_playlist_sharing && self.personal.phone_revision!=Some(revision){self.personal.phone_revision=Some(revision);self.personal.phone_due=Some(std::time::Instant::now()+std::time::Duration::from_secs(3));}
        if self.personal.phone_due.is_some_and(|t|t<=std::time::Instant::now()) && !self.personal.tasks.values().any(|p|p.fingerprint.starts_with("phone-")) {
            self.personal.phone_due=None;
            self.personal_request(Task::PhoneSync(self.addons.collections.clone()),None,false,"phone-sync".into(),"Updating shared playlists…");
        }

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
            // Retired snapshots must not silently repopulate a user's Home.
            self.settings.playlist_snapshot_recovery_v1 = true;
            if !self.settings.legacy_recovery_hidden_v1 {
                for collection in &self.addons.collections {
                    if library::legacy_recovered(collection) {
                        self.actions.push(crate::model::Action::SetPlaylistHidden {
                            source: collection.source.clone(), hidden: true,
                        });
                    }
                }
                self.settings.legacy_recovery_hidden_v1 = true;
            }
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
        if self.personal.spotify_importing
            || self.music_accounts[0].busy
            || self.personal.tasks.values().any(|p| p.backup)
        {
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
                            || (r.retry_at > 0 && crate::playlist_tools::now() >= r.retry_at))
                            && self.personal.current.get(&c.source) != Some(&r.fingerprint)
                    })
            })
            .map(|c| c.source.clone());
        if let Some(source) = next {
            self.apply_personal(Edit::Backup(source));
        }
    }
    pub(super) fn apply_personal(&mut self, action: Edit) {
        let recreate_missing = matches!(&action, Edit::SaveBackup(_));
        match action {
            Edit::PhoneClose => {self.personal.phone_dialog=false;},
            Edit::PhoneShare {source,revoke} => {
                if self.personal.tasks.values().any(|p|p.fingerprint.starts_with("phone-")){return;}
                self.settings.phone_playlist_sharing=true;self.settings_dirty=true;self.personal.phone_dialog=true;self.personal.phone_panel=None;self.personal.phone_status=if revoke {"Disabling sharing…"}else{"Preparing your playlist link…"}.into();
                if let Some(collection)=self.source_collection(&source).cloned(){self.personal_request(Task::PhoneShare{collection,revoke},Some(source),false,"phone-open".into(),"Preparing phone share…");}
                else {self.personal.phone_status="Open this playlist once to load its songs, then choose Share to phone.".into();}
            },

            Edit::SpotifyOpen => {
                self.personal.message = None;
                self.personal.dialog = Some(library::Dialog::Spotify {
                    backup: false,
                    selected: Default::default(),
                    follow: false,
                });
                if self
                    .personal
                    .spotify
                    .as_ref()
                    .is_none_or(|c| c.entries.is_empty())
                {
                    self.apply_personal(Edit::SpotifyLoad { connect: false });
                }
            }
            Edit::SpotifyLoad { connect } => {
                if self.personal.spotify_busy || self.personal.spotify_importing {
                    return;
                }
                self.personal.spotify_busy = true;
                self.personal.message = None;
                self.personal_request(
                    Task::SpotifyLoad { connect },
                    Some("spotify-account".into()),
                    false,
                    "spotify-account".into(),
                    if connect {
                        "Finish signing in in the Spotify window…"
                    } else {
                        "Checking your Spotify connection…"
                    },
                );
            }
            Edit::SpotifyDisconnect => {
                if self.personal.spotify_busy || self.personal.spotify_importing {
                    return;
                }
                self.backend.send(Command::CancelSpotifyImports);
                self.personal
                    .tasks
                    .retain(|_, p| !p.fingerprint.starts_with("spotify-"));
                self.personal.spotify_probe_tried = true;
                self.personal.spotify_resume_account = None;
                self.personal.spotify_busy = true;
                self.personal_request(
                    Task::SpotifyDisconnect,
                    Some("spotify-account".into()),
                    false,
                    "spotify-account".into(),
                    "Disconnecting Spotify…",
                );
            }
            Edit::SpotifyImport {
                selected,
                backup,
                follow,
            } => {
                if self.personal.spotify_busy || self.personal.spotify_importing {
                    return;
                }
                let Some(catalog) = &self.personal.spotify else {
                    return;
                };
                self.personal.spotify_queue = catalog
                    .entries
                    .iter()
                    .filter(|entry| selected.contains(&entry.id))
                    .cloned()
                    .collect();
                if self.personal.spotify_queue.is_empty() {
                    return;
                }
                let new_count = self
                    .personal
                    .spotify_queue
                    .iter()
                    .filter(|entry| {
                        self.source_collection(&entry.source(&catalog.account))
                            .is_none()
                    })
                    .count();
                if self.addons.collections.len() + new_count > library::MAX_LISTS {
                    self.personal.message = Some(format!(
                        "Choose fewer playlists. Vibefy can store {} local playlists; your existing copies are kept.",
                        library::MAX_LISTS
                    ));
                    self.personal.spotify_queue.clear();
                    return;
                }
                self.personal.spotify_importing = true;
                self.personal.spotify_backup = backup;
                self.personal.spotify_follow = follow;
                self.personal.spotify_backup_epoch = self.music_accounts_serial[0];
                self.personal.spotify_completed = 0;
                self.personal.spotify_total = self.personal.spotify_queue.len();
                self.personal.dialog = None;
                self.open(Page::Home);
                self.next_spotify_import();
            }
            Edit::SpotifyToggleSync { source, enabled } => {
                if let Some(binding) = self.settings.spotify_playlists.get_mut(&source) {
                    binding.writes = enabled;
                    self.settings_dirty = true;
                }
            }
            Edit::SpotifySong {
                source,
                local_source,
            } => {
                self.personal.spotify_search_serial =
                    self.personal.spotify_search_serial.wrapping_add(1);
                let selected = crate::spotify_match::valid_uri(&source.uri).then(|| source.clone());
                let destination = local_source
                    .as_ref()
                    .and_then(|s| self.settings.spotify_playlists.get(s))
                    .map(|b| b.id.clone());
                self.personal.dialog = Some(library::Dialog::SpotifyAdd {
                    query: format!("{} {}", source.name, source.artist_names()),
                    source,
                    results: crate::model::Loadable::NotLoaded,
                    selected,
                    destination,
                    local_source,
                    local: true,
                    backup: false,
                });
                if self
                    .personal
                    .spotify
                    .as_ref()
                    .is_none_or(|c| c.entries.is_empty())
                {
                    self.apply_personal(Edit::SpotifyLoad { connect: false });
                }
            }
            Edit::SpotifyFind { source, query } => {
                let Some(account) = self
                    .personal
                    .spotify
                    .as_ref()
                    .map(|c| crate::spotify_sync::account_key(&c.account))
                else {
                    return;
                };
                let query = query.trim().chars().take(200).collect::<String>();
                if query.is_empty() {
                    return;
                }
                self.personal.spotify_search_serial =
                    self.personal.spotify_search_serial.wrapping_add(1);
                let serial = self.personal.spotify_search_serial;
                if let Some(library::Dialog::SpotifyAdd {
                    results, selected, ..
                }) = &mut self.personal.dialog
                {
                    *results = crate::model::Loadable::Loading;
                    *selected = None;
                }
                self.personal_request(
                    Task::SpotifySearch {
                        serial,
                        source,
                        query,
                        account,
                    },
                    None,
                    false,
                    format!("spotify-search:{serial}"),
                    "Searching Spotify recordings…",
                );
            }
            Edit::SpotifyChoose(uri) => {
                if let Some(library::Dialog::SpotifyAdd {
                    results: crate::model::Loadable::Loaded(tracks),
                    selected,
                    ..
                }) = &mut self.personal.dialog
                {
                    if let Some(track) = tracks.iter().find(|t| t.uri == uri) {
                        *selected = Some(track.clone());
                    }
                }
            }
            Edit::SpotifySend => self.send_spotify_song(),
            Edit::SpotifyLocalOnly => {
                if let Some(library::Dialog::SpotifyAdd {
                    source,
                    local_source: Some(target),
                    ..
                }) = self.personal.dialog.clone()
                {
                    if let Some(mut list) = self.source_collection(&target).cloned() {
                        library::append(
                            &mut list,
                            vec![crate::playlist_tools::local_track(&source)],
                        );
                        if self.save_source_collection(list) {
                            self.personal.dialog = None;
                        }
                    }
                }
            }
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
                if !crate::playlist_tools::valid_title(&name) {
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
                if !crate::playlist_tools::valid_title(&name) {
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
                self.settings.spotify_playlists.remove(&source);
                self.settings_dirty = true;
                self.personal_changed();
                self.personal.dialog = None;
                self.open(Page::Home);
            }
            Edit::Add { source, tracks } => {
                if self
                    .settings
                    .spotify_playlists
                    .get(&source)
                    .is_some_and(|b| b.writes)
                {
                    if tracks.len() != 1 {
                        self.personal.message = Some(
                            "Select one song at a time to confirm its Spotify recording.".into(),
                        );
                        return;
                    }
                    let item = tracks[0].to_playable_item();
                    if let crate::api::models::PlayableItem::Track(track) = item {
                        self.apply_personal(Edit::SpotifySong {
                            source: track,
                            local_source: Some(source),
                        });
                    }
                    return;
                }
                if let Some(mut list) = self
                    .addons
                    .collections
                    .iter()
                    .find(|c| {
                        c.source == source
                            && (library::owned(&source)
                                || self.settings.spotify_playlists.contains_key(&source))
                    })
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
            Edit::Backup(source) | Edit::SaveBackup(source) => {
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
                    Task::Backup { collection, scope, recreate_missing },
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
        let spotify_import = pending.fingerprint.starts_with("spotify-import");
        if let Some(serial) = pending
            .fingerprint
            .strip_prefix("spotify-search:")
            .and_then(|s| s.parse::<u64>().ok())
        {
            if serial != self.personal.spotify_search_serial {
                return;
            }
        }
        if pending.fingerprint == "spotify-account" {
            self.personal.spotify_busy = false;
        }
        if pending.fingerprint.starts_with("phone-") {
            match result {
                Ok(Output::PhoneShared(panel))=>{self.personal.phone_panel=panel;self.personal.phone_status=if self.personal.phone_panel.is_some(){"Scan with your phone camera. No account is required."}else{"Sharing disabled. Existing phone copies are kept."}.into();},
                Ok(Output::PhoneSynced)=>{},
                Err(error)=>{self.personal.phone_status=error;self.personal.phone_due=Some(std::time::Instant::now()+std::time::Duration::from_secs(60));},
                _=>{},
            }
            return;
        }
        let mut failed = result.is_err();
        let imported_editable = match &result {
            Ok(Output::SpotifyImported { editable, .. }) => Some(*editable),
            _ => None,
        };
        match result {
            Ok(Output::PhoneShared(_) | Output::PhoneSynced) => {},
            Ok(Output::SpotifyMatches{serial,tracks}) => {
                if serial==self.personal.spotify_search_serial {
                    if let Some(library::Dialog::SpotifyAdd{results,..})=&mut self.personal.dialog {*results=crate::model::Loadable::Loaded(tracks);}
                }
            }
            Ok(Output::SpotifyChanged{source,account,snapshot,collection}) => {
                if self.personal.spotify.as_ref().is_none_or(|c|crate::spotify_sync::account_key(&c.account)!=account) {return;}
                if self.settings.spotify_playlists.get(&source).is_none_or(|b|b.account!=account) {return;}
                if let Some(remote)=collection {
                    if let Some(mut local)=self.source_collection(&source).cloned() {
                        if crate::spotify_sync::append_remote(&mut local,remote.tracks)>0 && !self.save_source_collection(local) {return;}
                    }
                }
                if let Some(binding)=self.settings.spotify_playlists.get_mut(&source) {binding.snapshot=snapshot;self.settings_dirty=true;}
            }
            Ok(Output::SpotifyCatalog(mut catalog)) => {
                if let (Some(next),Some(previous))=(&mut catalog,&self.personal.spotify) {
                    if next.account==previous.account && next.error.is_some() && (previous.loaded || !previous.entries.is_empty()) {
                        next.entries=previous.entries.clone();
                    }
                }
                if let Some(catalog)=&catalog {
                    self.personal.message=catalog.error.clone();
                }
                self.personal.spotify = catalog;
                if let Some(library::Dialog::Spotify { selected, .. }) = &mut self.personal.dialog { selected.clear(); }
                if let Some(account)=self.personal.spotify.as_ref().map(|c|c.account.clone()) {
                    if self.personal.spotify_resume_account.as_deref()!=Some(account.as_str()) {
                        self.backend.send(Command::ResumeSpotifyAdditions(account.clone()));
                        self.personal.spotify_resume_account=Some(account);
                    }
                    if self.personal.spotify.as_ref().is_some_and(|c|!c.loaded) {self.apply_personal(Edit::SpotifyLoad{connect:false});}
                }
            }
            Ok(Output::SpotifyDisconnected) => {
                self.personal.spotify = None;
                self.personal.message = Some("Spotify disconnected. Your imported playlists are kept.".into());
            }
            Ok(Output::LegacyRecovered(collections)) => {
                match crate::playlist_recovery::merge(&mut self.addons.collections, collections) {
                    Ok(count) => {
                        // Queue the library before its completion flag on the same file writer.
                        self.personal_changed();
                        self.settings.playlist_snapshot_recovery_v1 = true;
                        self.settings_dirty = true;
                        if count > 0 { self.personal.message = Some(format!("Recovered {count} saved playlists. Spotify and YouTube playlists are now independent.")); }
                    }
                    Err(error) => self.personal.message = Some(format!("Playlist recovery: {error}. Original files were kept; recovery can be retried after restarting.")),
                }
            }
            Ok(Output::Recovered{collection,scope,report})=>{
                if pending.epoch!=self.music_accounts_serial[0] || !self.music_accounts[0].connected {return;}
                let source=collection.source.clone();
                if self.source_collection(&source).is_none() {self.save_source_collection(collection);}
                self.settings.playlist_backups.insert(source.clone(),scope.clone());self.settings_dirty=true;
                self.personal.scope=Some(scope);self.personal.backups.insert(source.clone(),report);
                self.personal.dialog=None;self.open(Page::LocalCollection(source));
            }
            Ok(Output::Imported(collection) | Output::SpotifyImported{collection,..})=>{
                let source=collection.source.clone();
                if let Some(editable)=imported_editable {
                    if let Some(catalog)=&mut self.personal.spotify {
                        if let Some(entry)=catalog.entries.iter_mut().find(|e|e.source(&catalog.account)==source) {entry.editable=editable;}
                    }
                }
                if self.save_source_collection(collection) {
                    if spotify_import {
                        if let Some(catalog)=&self.personal.spotify {
                            if self.personal.spotify_follow {
                                if let Some(entry)=catalog.entries.iter().find(|e|e.source(&catalog.account)==source) {
                                    self.settings.spotify_playlists.insert(source.clone(),crate::spotify_sync::Binding{account:crate::spotify_sync::account_key(&catalog.account),id:entry.id.clone(),writes:entry.editable,snapshot:String::new()});
                                    self.settings_dirty=true;
                                }
                            } else {self.settings.spotify_playlists.remove(&source);self.settings_dirty=true;}
                        }
                    }
                    self.personal.dialog=None;
                    if !spotify_import {self.open(Page::LocalCollection(source.clone()));}
                    self.personal.message=Some("Playlist imported and saved on this device".into());
                    if matches!(pending.fingerprint.as_str(),"backup"|"spotify-import-backup") && pending.epoch==self.music_accounts_serial[0] {
                        if spotify_import {
                            if let Some(scope)=self.personal.scope.clone() {
                                self.settings.playlist_backups.insert(source,scope);
                                self.personal.checked_revision=None;
                                self.settings_dirty=true;
                            }
                        } else {self.apply_personal(Edit::Backup(source));}
                    }
                } else { failed=true; }
            }
            Ok(Output::Restored(lists))=>{
                match crate::playlist_backup::merge(&mut self.addons.collections, lists) {
                    Ok(count) => { self.personal_changed(); self.personal.message=Some(format!("Restored {count} playlists. Existing playlists were kept; identical backups were skipped.")); self.open(Page::Home); }
                    Err(error) => self.personal.message=Some(error.to_string()),
                }
            }
            Ok(Output::Exported)=>self.personal.message=Some("Playlist backup exported. It contains metadata only, without music or account credentials.".into()),
            Ok(Output::Account{scope,reports})=>{self.personal.scope=Some(scope);self.personal.backups=reports;}
            Ok(Output::Backed{source,report})=>{
                self.personal.message=Some(if let Some(message)=&report.message {format!("Saved locally. YouTube backup: {message}")} else if report.missing>0 {format!("YouTube copy saved; {} songs could not be matched. Your complete local playlist is kept.",report.missing)} else {"Playlist saved to YouTube".into()});
                self.personal.backups.insert(source,report);
            }
            Err(error)=>{
                if pending.fingerprint=="spotify-watch" {
                    let wait=crate::spotify_library::retry_remaining();
                    if wait>0 {self.personal.spotify_poll_due=Some(std::time::Instant::now()+std::time::Duration::from_secs(wait));}
                    else {self.personal.spotify_poll_due=Some(std::time::Instant::now()+std::time::Duration::from_secs(60));}
                }
                if pending.fingerprint.starts_with("spotify-search:") {
                    if let Some(library::Dialog::SpotifyAdd{results,..})=&mut self.personal.dialog {*results=crate::model::Loadable::Failed(error.clone());}
                }
                if pending.backup && let Some(source)=pending.source {self.personal.backups.entry(source).or_default().message=Some(error.clone());}
                if !error.contains("selection cancelled") {self.personal.message=Some(error);}
            }
        }
        if spotify_import {
            if failed {
                self.personal.spotify_queue.clear();
                self.personal.spotify_importing = false;
                if let Some(message) = &mut self.personal.message {
                    message.push_str(" Completed local imports were kept. Select the remaining playlists to retry.");
                }
            } else {
                self.personal.spotify_completed += 1;
                self.next_spotify_import();
            }
        }
    }
    fn next_spotify_import(&mut self) {
        if self.personal.spotify_backup_epoch != self.music_accounts_serial[0] {
            self.personal.spotify_backup = false;
        }
        let Some(entry) = self.personal.spotify_queue.pop_front() else {
            self.personal.spotify_importing = false;
            self.personal.message = Some(format!(
                "Imported {} Spotify collections. Saved locally{}.",
                self.personal.spotify_completed,
                if self.personal.spotify_backup {
                    "; selected YouTube copies are being saved separately"
                } else {
                    ""
                }
            ));
            return;
        };
        let Some(account) = self.personal.spotify.as_ref().map(|c| c.account.clone()) else {
            self.personal.spotify_importing = false;
            self.personal.spotify_queue.clear();
            return;
        };
        let source = entry.source(&account);
        let message = format!(
            "Importing Spotify · {}/{} · {}",
            self.personal.spotify_completed + 1,
            self.personal.spotify_total,
            entry.title
        );
        self.personal_request(
            Task::SpotifyImport { entry, account },
            Some(source),
            false,
            if self.personal.spotify_backup {
                "spotify-import-backup"
            } else {
                "spotify-import"
            }
            .into(),
            &message,
        );
    }
    fn tick_spotify_copies(&mut self) {
        if !self.library_access()
            || self.personal.spotify_busy
            || self.personal.spotify_importing
            || self.personal.tasks.values().any(|p| p.backup)
        {
            return;
        }
        let now = std::time::Instant::now();
        if self.personal.spotify_poll_due.is_some_and(|due| now < due) {
            return;
        }
        if self.personal.tasks.values().any(|p| {
            p.fingerprint.starts_with("spotify-watch") || p.fingerprint == "spotify-account"
        }) {
            return;
        }
        if self.personal.spotify.is_none() {
            if !self.personal.spotify_probe_tried {
                self.personal.spotify_probe_tried = true;
                self.personal.spotify_busy = true;
                self.personal_request(
                    Task::SpotifyProbe,
                    None,
                    false,
                    "spotify-account".into(),
                    "Restoring the selected Spotify playlist connection…",
                );
            }
            return;
        }
        let account =
            crate::spotify_sync::account_key(&self.personal.spotify.as_ref().unwrap().account);
        let next = self
            .settings
            .spotify_playlists
            .iter()
            .filter(|(source, b)| {
                b.account == account
                    && self.source_collection(source).is_some()
                    && self
                        .personal
                        .spotify_checked
                        .get(*source)
                        .is_none_or(|last| {
                            now.duration_since(*last) >= std::time::Duration::from_secs(15 * 60)
                        })
            })
            .min_by_key(|(source, _)| self.personal.spotify_checked.get(*source).copied())
            .map(|(s, b)| (s.clone(), b.clone()));
        if let Some((source, binding)) = next {
            self.personal.spotify_checked.insert(source.clone(), now);
            self.personal.spotify_poll_due = Some(now + std::time::Duration::from_secs(20));
            self.personal_request(
                Task::SpotifyWatch {
                    source: source.clone(),
                    binding,
                },
                Some(source),
                false,
                "spotify-watch".into(),
                "Checking selected Spotify playlist additions…",
            );
        }
    }
    fn send_spotify_song(&mut self) {
        let Some(library::Dialog::SpotifyAdd {
            selected: Some(track),
            destination: Some(id),
            local_source,
            local,
            backup,
            ..
        }) = self.personal.dialog.clone()
        else {
            return;
        };
        let Some(catalog) = self.personal.spotify.clone() else {
            return;
        };
        let Some(entry) = catalog
            .entries
            .iter()
            .find(|e| e.id == id && e.editable)
            .cloned()
        else {
            return;
        };
        if !crate::spotify_sync::valid_addition(&id, &[track.uri.clone()]) {
            return;
        }
        let key = crate::spotify_sync::account_key(&catalog.account);
        let source = local_source.unwrap_or_else(|| entry.source(&catalog.account));
        if self
            .settings
            .spotify_playlists
            .get(&source)
            .is_some_and(|b| b.account != key || b.id != id)
        {
            self.personal.message =
                Some("Reconnect the Spotify account associated with this copy.".into());
            return;
        }
        if local {
            let mut list = self.source_collection(&source).cloned().unwrap_or_else(|| {
                crate::addons::Collection {
                    source: source.clone(),
                    title: entry.title.clone(),
                    image: entry.image.clone(),
                    owner: Some("Spotify copy · Additions only".into()),
                    ..Default::default()
                }
            });
            let mut song = crate::playlist_tools::local_track(&track);
            if track.explicit {
                song.album.push_str(" [Explicit]");
            }
            library::append(&mut list, vec![song]);
            if !self.save_source_collection(list) {
                return;
            }
            self.settings
                .spotify_playlists
                .entry(source.clone())
                .or_insert(crate::spotify_sync::Binding {
                    account: key,
                    id: id.clone(),
                    writes: true,
                    snapshot: String::new(),
                });
            self.settings_dirty = true;
            if backup {
                self.apply_personal(Edit::Backup(source.clone()));
            }
        }
        self.personal
            .spotify_pending
            .insert((source.clone(), track.uri.clone()));
        self.backend
            .send(Command::SpotifyAddition(crate::playlist_outbox::Addition {
                account: catalog.account,
                playlist_id: id,
                playlist_name: entry.title,
                uris: vec![track.uri],
                position: None,
            }));
        self.personal.dialog = None;
        self.personal.message = Some(
            if local {
                "Saved in Vibefy. The Spotify addition is pending; you can keep listening."
            } else {
                "Spotify addition pending. You can keep listening."
            }
            .into(),
        );
    }
    pub(super) fn receive_spotify_addition(
        &mut self,
        account: String,
        playlist: String,
        uris: Vec<String>,
        result: Option<Result<(), String>>,
    ) {
        if self
            .personal
            .spotify
            .as_ref()
            .is_none_or(|c| c.account != account)
        {
            return;
        }
        let entry = crate::spotify_library::Entry {
            editable: true,
            id: playlist,
            title: String::new(),
            image: None,
            count: None,
        };
        let source = entry.source(&account);
        for uri in uris {
            if result.is_none() {
                self.personal.spotify_pending.insert((source.clone(), uri));
            } else {
                self.personal.spotify_pending.remove(&(source.clone(), uri));
            }
        }
        if let Some(result) = result {
            match result {
                Ok(()) => {
                    self.personal.spotify_checked.remove(&source);
                    self.personal.message = Some(
                        if self.source_collection(&source).is_some() {
                            "Song added to Spotify. Your Vibefy copy is saved."
                        } else {
                            "Song added to Spotify."
                        }
                        .into(),
                    );
                }
                Err(error) => {
                    self.personal.message = Some(format!(
                        "{}Spotify: {error}",
                        if self.source_collection(&source).is_some() {
                            "Saved in Vibefy. "
                        } else {
                            ""
                        }
                    ))
                }
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
