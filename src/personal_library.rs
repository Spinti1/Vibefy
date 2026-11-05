//! Local playlist metadata and account-scoped YouTube backups. No audio is stored here.
use crate::{
    addons::Collection,
    api::{ApiGateway, Operation, models::PlayableItem},
    http::Http,
    music_accounts,
    music_session::Session,
    player::LocalTrack,
    settings::ProxyConfig,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

pub const MAX_LISTS: usize = 200;
pub const MAX_TRACKS: usize = 20_000;
pub fn owned(source: &str) -> bool {
    source
        .strip_prefix("vibefy:playlist:")
        .is_some_and(|id| id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()))
}
pub fn new_source() -> String {
    format!("vibefy:playlist:{:032x}", rand::random::<u128>())
}
pub fn fingerprint(collection: &Collection) -> String {
    // Artwork enrichment does not change remote membership. No temporary JSON allocation.
    let mut digest = Sha256::new();
    for text in std::iter::once(collection.title.as_str())
        .chain(collection.tracks.iter().map(|t| t.uri.as_str()))
    {
        digest.update((text.len() as u64).to_le_bytes());
        digest.update(text.as_bytes());
    }
    format!("{:x}", digest.finalize())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn valid_track(track: &LocalTrack) -> bool {
    track.title.len() <= 4096
        && track.artists.len() <= 50
        && (crate::addons::direct_url(&track.uri).is_some()
            || track
                .uri
                .strip_prefix("spotify:track:")
                .is_some_and(|id| id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())))
}
pub fn validate(collection: &Collection) -> bool {
    !collection.title.trim().is_empty()
        && collection.title.chars().count() <= 150
        && collection.source.len() <= 1000
        && collection.tracks.len() <= MAX_TRACKS
        && collection.tracks.iter().all(valid_track)
}
pub fn spotify_id(text: &str) -> Option<String> {
    let text = text.trim();
    let id = if let Some(id) = text.strip_prefix("spotify:playlist:") {
        id.to_owned()
    } else {
        let url = reqwest::Url::parse(text).ok()?;
        if url.scheme() != "https"
            || url.host_str() != Some("open.spotify.com")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
        {
            return None;
        }
        url.path().strip_prefix("/playlist/")?.to_owned()
    };
    (id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())).then_some(id)
}
pub fn append(collection: &mut Collection, tracks: Vec<LocalTrack>) {
    let mut seen: HashSet<_> = collection.tracks.iter().map(|t| t.uri.clone()).collect();
    for track in tracks {
        if collection.tracks.len() >= MAX_TRACKS {
            break;
        }
        if valid_track(&track) && seen.insert(track.uri.clone()) {
            collection.tracks.push(track);
        }
    }
}

#[derive(Clone, Debug)]
pub enum Dialog {
    New {
        name: String,
        backup: bool,
    },
    Import {
        youtube: bool,
        url: String,
        backup: bool,
    },
    Rename {
        source: String,
        name: String,
    },
    Delete(String),
}
#[derive(Clone, Debug)]
pub enum Action {
    Show(Dialog),
    CreateFrom(Vec<LocalTrack>),
    Close,
    Continue,
    Create {
        name: String,
        backup: bool,
    },
    Rename {
        source: String,
        name: String,
    },
    Delete(String),
    Add {
        source: String,
        tracks: Vec<LocalTrack>,
    },
    Remove {
        source: String,
        uri: String,
    },
    Import {
        youtube: bool,
        url: String,
        backup: bool,
    },
    Backup(String),
    Export,
    Restore,
}
#[derive(Default)]
pub struct State {
    pub started: bool,
    pub checked_revision: Option<u64>,
    pub current: BTreeMap<String, String>,
    pub seed_tracks: Vec<LocalTrack>,
    pub dialog: Option<Dialog>,
    pub next: u64,
    pub tasks: BTreeMap<u64, Pending>,
    pub scope: Option<String>,
    pub observed: Option<u64>,
    pub backups: BTreeMap<String, Report>,
    pub message: Option<String>,
}
pub struct Pending {
    pub source: Option<String>,
    pub backup: bool,
    pub fingerprint: String,
    pub epoch: u64,
    pub message: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Report {
    #[serde(default)]
    pub retry_at: u64,
    pub remote: String,
    pub fingerprint: String,
    pub missing: usize,
    pub message: Option<String>,
}
pub enum Output {
    Imported(Collection),
    Recovered {
        collection: Collection,
        scope: String,
        report: Report,
    },
    Restored(Vec<Collection>),
    Exported,
    Backed {
        source: String,
        report: Report,
    },
    Account {
        scope: String,
        reports: BTreeMap<String, Report>,
    },
}
#[derive(Clone)]
pub enum Task {
    Import {
        youtube: bool,
        url: String,
    },
    Backup {
        collection: Collection,
        scope: String,
    },
    Account,
}
pub struct ContextData {
    pub progress: Box<dyn Fn(String) + Send + Sync>,
    pub root: PathBuf,
    pub config: PathBuf,
    pub proxy: ProxyConfig,
    pub http: Http,
    pub api: Arc<ApiGateway>,
}

pub async fn execute(task: Task, context: ContextData) -> Result<Output> {
    match task {
        Task::Import { youtube: true, url } => {
            let normalized = crate::addons::playlist_url(&url)?;
            let id = normalized.split("list=").nth(1).unwrap_or_default();
            if let Ok(session) =
                music_accounts::youtube_session(&context.config, &context.proxy).await
            {
                let list = music_accounts::LibraryPlaylist {
                    id: format!("VL{id}"),
                    title: "YouTube playlist".into(),
                    image: None,
                };
                // Public imports obtain the source title/artwork; authenticated import also supports private lists.
                let mut collection = session.import(&context.proxy, &list).await?;
                if let Ok(public) = crate::addons::fetch(
                    crate::addons::Request::YouTube(url),
                    context.proxy.clone(),
                )
                .await
                {
                    collection.title = public.title;
                    collection.image = public.image;
                }
                let (private, description) = session.details(&context.proxy, id).await?;
                if private
                    && let Some(source) = description
                        .lines()
                        .find_map(|line| line.strip_prefix("Vibefy source: "))
                        .filter(|source| owned(source))
                {
                    let marker = format!("Vibefy playlist: {}", hash(source.as_bytes()));
                    if description.contains(&marker) {
                        collection.source = source.into();
                        collection.owner = Some("Your Vibefy playlist".into());
                        let (account, _) = session.identity(&context.proxy).await?;
                        let scope = hash(account.as_bytes());
                        let report = Report {
                            remote: id.into(),
                            fingerprint: fingerprint(&collection),
                            ..Default::default()
                        };
                        let journal = Journal {
                            source: source.into(),
                            remote: id.into(),
                            title: collection.title.clone(),
                            report: report.clone(),
                            ..Default::default()
                        };
                        let path = context
                            .root
                            .join(&scope)
                            .join(format!("{}.json", hash(source.as_bytes())));
                        write(&path, &journal).await?;
                        return Ok(Output::Recovered {
                            collection,
                            scope,
                            report,
                        });
                    }
                }
                Ok(Output::Imported(collection))
            } else {
                Ok(Output::Imported(
                    crate::addons::fetch(
                        crate::addons::Request::YouTube(url),
                        context.proxy.clone(),
                    )
                    .await?,
                ))
            }
        }
        Task::Import {
            youtube: false,
            url,
        } => {
            let id =
                spotify_id(&url).ok_or_else(|| anyhow::anyhow!("Paste a Spotify playlist link"))?;
            let header = context
                .api
                .client_for(Operation::PlaylistMetadata(
                    context.api.playlist_access(&id),
                ))
                .await?
                .playlist(&id)
                .await?;
            context.api.observe_playlist(&header);
            let client = context
                .api
                .client_for(Operation::PlaylistItems(context.api.playlist_access(&id)))
                .await?;
            let mut offset = 0;
            let mut tracks = Vec::new();
            let mut seen = HashSet::new();
            loop {
                (context.progress)(format!("Importing Spotify songs · {} loaded", tracks.len()));
                let page = client.playlist_items(&id, offset, 50).await?;
                for row in &page.items {
                    if let Some(PlayableItem::Track(track)) = row.playable() {
                        if !track.is_local {
                            tracks.push(crate::playlist_mirror::local_track(track));
                        }
                    }
                }
                ensure!(tracks.len() <= MAX_TRACKS, "Playlist exceeds 20,000 songs");
                match page.next_offset() {
                    Some(next) if next > offset && seen.insert(next) => offset = next,
                    Some(_) => anyhow::bail!("Spotify returned a repeated page; retry the import"),
                    None => break,
                }
            }
            let after = context
                .api
                .client_for(Operation::PlaylistMetadata(
                    context.api.playlist_access(&id),
                ))
                .await?
                .playlist(&id)
                .await?;
            ensure!(
                header.snapshot_id.is_none() || after.snapshot_id == header.snapshot_id,
                "Playlist changed during import; please retry"
            );
            Ok(Output::Imported(Collection {
                title: header.name,
                source: format!("https://open.spotify.com/playlist/{id}"),
                tracks,
                image: crate::api::models::pick_image(&header.images, 640).map(str::to_owned),
                owner: Some("Imported from Spotify".into()),
            }))
        }
        Task::Account => {
            let session = music_accounts::youtube_session(&context.config, &context.proxy).await?;
            let (account, _) = session.identity(&context.proxy).await?;
            let scope = hash(account.as_bytes());
            let reports = load_reports(context.root.join(&scope)).await?;
            Ok(Output::Account { scope, reports })
        }
        Task::Backup { collection, scope } => backup(collection, &scope, context).await,
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Journal {
    source: String,
    remote: String,
    creating: bool,
    title: String,
    retry_at: u64,
    matches: BTreeMap<String, LocalTrack>,
    report: Report,
}
async fn read(path: PathBuf) -> Result<Option<Vec<u8>>> {
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        match std::fs::File::open(path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(20_000_001).read_to_end(&mut bytes)?;
                ensure!(
                    bytes.len() <= 20_000_000,
                    "Playlist backup metadata is too large"
                );
                Ok(Some(bytes))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
    .await?
}
async fn write(path: &Path, journal: &Journal) -> Result<()> {
    let bytes = serde_json::to_vec(journal)?;
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || -> Result<()> {
        std::fs::create_dir_all(path.parent().unwrap())?;
        fastframe_file::write_private(&path, bytes)?;
        Ok(())
    })
    .await?
}
async fn load_reports(root: PathBuf) -> Result<BTreeMap<String, Report>> {
    tokio::task::spawn_blocking(move || -> Result<_> {
        let mut result = BTreeMap::new();
        let Ok(entries) = std::fs::read_dir(root) else {
            return Ok(result);
        };
        for entry in entries.take(MAX_LISTS).flatten() {
            if entry.file_type()?.is_symlink() || entry.metadata()?.len() > 20_000_000 {
                continue;
            }
            if let Ok(bytes) = std::fs::read(entry.path())
                && let Ok(saved) = serde_json::from_slice::<Journal>(&bytes)
            {
                result.insert(saved.source, saved.report);
            }
        }
        Ok(result)
    })
    .await?
}
async fn backup(collection: Collection, scope: &str, context: ContextData) -> Result<Output> {
    ensure!(validate(&collection), "Invalid playlist metadata");
    // Serialize remote reconciliation, even when several lists were edited together.
    static LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _guard = LOCK.get_or_init(Default::default).lock().await;
    let session = music_accounts::youtube_session(&context.config, &context.proxy).await?;
    let (account, _) = session.identity(&context.proxy).await?;
    ensure!(
        hash(account.as_bytes()) == scope,
        "YouTube account changed; reconnect before saving this playlist"
    );
    let path = context
        .root
        .join(scope)
        .join(format!("{}.json", hash(collection.source.as_bytes())));
    let mut journal: Journal = match read(path.clone()).await? {
        Some(bytes) => serde_json::from_slice(&bytes)?,
        None => Journal {
            source: collection.source.clone(),
            ..Default::default()
        },
    };
    ensure!(
        journal.source == collection.source,
        "Backup identity does not match"
    );
    ensure!(
        crate::playlist_mirror::now() >= journal.retry_at,
        "YouTube is limiting requests. Your local playlist is safe; try again later."
    );
    let result = sync(&session, &context, &collection, &path, &mut journal).await;
    if let Err(error) = result {
        record_failure(&mut journal, &error, crate::playlist_mirror::now());
        write(&path, &journal).await?;
    }
    Ok(Output::Backed {
        source: collection.source,
        report: journal.report,
    })
}
fn record_failure(journal: &mut Journal, error: &anyhow::Error, now: u64) {
    use crate::music_session::Failure;
    let failure = error.downcast_ref::<Failure>();
    if let Some(Failure::RateLimited(seconds)) = failure {
        journal.retry_at = now.saturating_add(*seconds);
    }
    let rejected = matches!(failure, Some(Failure::RateLimited(_) | Failure::NotSent))
        || matches!(failure,Some(Failure::Rejected(status)) if (400..500).contains(status) && *status!=408);
    if journal.remote.is_empty() && rejected {
        journal.creating = false;
    }
    journal.report.message = Some(error.to_string());
    journal.report.remote = journal.remote.clone();
    journal.report.retry_at = journal.retry_at;
}
async fn sync(
    session: &Session,
    context: &ContextData,
    collection: &Collection,
    path: &Path,
    journal: &mut Journal,
) -> Result<()> {
    let marker = format!("Vibefy playlist: {}", hash(collection.source.as_bytes()));
    let creating = journal.remote.is_empty();
    if creating && !journal.creating {
        // Recover an existing same-name copy after restoring local metadata on another device.
        let candidates = session
            .playlists(&context.proxy)
            .await?
            .into_iter()
            .filter(|p| p.title == collection.title)
            .collect::<Vec<_>>();
        ensure!(
            candidates.len() <= 20,
            "Several playlists share this name. Open the existing YouTube backup instead."
        );
        for list in candidates {
            let id = list.id.strip_prefix("VL").unwrap_or(&list.id);
            let (private, description) = session.details(&context.proxy, id).await?;
            if private && description.contains(&marker) {
                ensure!(
                    journal.remote.is_empty(),
                    "Multiple backup copies found; check YouTube"
                );
                journal.remote = id.into();
            }
        }
        if !journal.remote.is_empty() {
            journal.title = collection.title.clone();
            write(path, journal).await?;
        }
    }
    if journal.remote.is_empty() {
        if journal.creating {
            for list in session.playlists(&context.proxy).await? {
                let id = list.id.strip_prefix("VL").unwrap_or(&list.id);
                let (private, description) = session.details(&context.proxy, id).await?;
                if private && description.contains(&marker) {
                    ensure!(
                        journal.remote.is_empty(),
                        "Multiple backup copies found; check YouTube"
                    );
                    journal.remote = id.into();
                }
            }
            ensure!(
                !journal.remote.is_empty(),
                "YouTube did not confirm playlist creation. Check your YouTube account before trying another backup."
            );
        } else {
            journal.creating = true;
            write(path, journal).await?;
            journal.remote = session
                .create_unlisted(
                    &context.proxy,
                    &collection.title,
                    &format!(
                        "Playlist saved by Vibefy.\n{marker}\nVibefy source: {}",
                        collection.source
                    ),
                )
                .await?;
        }
        journal.creating = false;
        journal.title = collection.title.clone();
        write(path, journal).await?;
    }
    let (mut private, mut description) = session.details(&context.proxy, &journal.remote).await?;
    if creating {
        for seconds in [1, 2, 4] {
            if private && description.contains(&marker) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
            (private, description) = session.details(&context.proxy, &journal.remote).await?;
        }
    }
    ensure!(
        private && description.contains(&marker),
        "The YouTube copy must remain private or unlisted and keep its Vibefy description"
    );
    if journal.title != collection.title {
        session
            .rename(&context.proxy, &journal.remote, &collection.title)
            .await?;
        journal.title = collection.title.clone();
        write(path, journal).await?;
    }
    let search = crate::music::Client::new(context.http.clone());
    let mut desired = Vec::new();
    let mut missing = 0;
    for (index, track) in collection.tracks.iter().enumerate() {
        (context.progress)(format!(
            "Saving {} · song {}/{}",
            collection.title,
            index + 1,
            collection.tracks.len()
        ));
        if let Some(id) = track.uri.strip_prefix("vibefy:youtube:") {
            desired.push(id.to_owned());
            continue;
        }
        if !journal.matches.contains_key(&track.uri) {
            let results = search
                .search(
                    &format!("{} {}", track.title, track.artist_names()),
                    crate::music::Filter::Songs,
                    None,
                )
                .await?;
            let candidates = results
                .entries
                .into_iter()
                .filter_map(|e| e.track)
                .take(8)
                .collect::<Vec<_>>();
            if let Some(chosen) = crate::playlist_mirror::choose_recording(track, &candidates) {
                journal.matches.insert(track.uri.clone(), chosen);
                write(path, journal).await?;
            }
        }
        if let Some(id) = journal
            .matches
            .get(&track.uri)
            .and_then(|t| t.uri.strip_prefix("vibefy:youtube:"))
        {
            desired.push(id.to_owned());
        } else {
            missing += 1;
        }
    }
    let mut unique = HashSet::new();
    desired.retain(|id| unique.insert(id.clone()));
    (context.progress)(format!(
        "Updating YouTube copy · {} matched songs",
        desired.len()
    ));
    session
        .sync_managed(&context.proxy, &journal.remote, &marker, &desired)
        .await?;
    journal.report = Report {
        retry_at: 0,
        remote: journal.remote.clone(),
        fingerprint: fingerprint(collection),
        missing,
        message: None,
    };
    write(path, journal).await
}

pub fn decode_backup(bytes: &[u8]) -> Result<Vec<Collection>> {
    ensure!(bytes.len() <= 20_000_000, "Backup exceeds 20 MB");
    let mut lists: Vec<Collection> =
        serde_json::from_slice(bytes).context("Choose a Vibefy playlist backup JSON file")?;
    ensure!(
        lists.len() <= MAX_LISTS && lists.iter().all(validate),
        "Backup contains unsupported playlist data"
    );
    for list in &mut lists {
        if list
            .image
            .as_deref()
            .is_some_and(|url| !crate::addons::safe_art_url(url))
        {
            list.image = None;
        }
        for track in &mut list.tracks {
            for image in [
                &mut track.cover_url,
                &mut track.art_url,
                &mut track.art_small_url,
            ] {
                if image
                    .as_deref()
                    .is_some_and(|url| !crate::addons::safe_art_url(url))
                {
                    *image = None;
                }
            }
        }
    }
    Ok(lists)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn personal_library_creation_recovery_retains_uncertain_intents_and_cooldowns() {
        use crate::music_session::Failure;
        let mut rejected = Journal {
            source: new_source(),
            creating: true,
            ..Default::default()
        };
        record_failure(&mut rejected, &Failure::RateLimited(120).into(), 1000);
        let recovered: Journal =
            serde_json::from_slice(&serde_json::to_vec(&rejected).unwrap()).unwrap();
        assert!(!recovered.creating);
        assert_eq!(recovered.retry_at, 1120);
        assert_eq!(recovered.report.retry_at, 1120);
        let mut uncertain = Journal {
            creating: true,
            ..Default::default()
        };
        record_failure(&mut uncertain, &Failure::Unconfirmed.into(), 1000);
        assert!(uncertain.creating);
        record_failure(&mut uncertain, &Failure::Rejected(503).into(), 1000);
        assert!(uncertain.creating);
        record_failure(&mut uncertain, &Failure::NotSent.into(), 1000);
        assert!(!uncertain.creating);
    }
    #[test]
    fn personal_library_rejects_foreign_links_and_invalid_backups() {
        assert!(
            spotify_id("https://open.spotify.com/playlist/1234567890123456789012?si=demo")
                .is_some()
        );
        assert!(
            spotify_id("https://open.spotify.com.attacker.test/playlist/1234567890123456789012")
                .is_none()
        );
        assert!(decode_backup(br#"[{"title":"x","source":"vibefy:playlist:bad","tracks":[{"uri":"file:///private"}]}]"#).is_err());
        assert!(owned(&new_source()));
        let mut unsafe_track = LocalTrack {
            uri: "file:///private".into(),
            title: "Song".into(),
            ..Default::default()
        };
        let mut list = Collection {
            source: new_source(),
            title: "Backup".into(),
            tracks: vec![unsafe_track.clone()],
            ..Default::default()
        };
        assert!(decode_backup(&serde_json::to_vec(&vec![list.clone()]).unwrap()).is_err());
        unsafe_track.uri = "vibefy:youtube:abcdefghijk".into();
        unsafe_track.cover_url = Some("file:///private.png".into());
        list.tracks = vec![unsafe_track];
        assert!(
            decode_backup(&serde_json::to_vec(&vec![list]).unwrap()).unwrap()[0].tracks[0]
                .cover_url
                .is_none()
        );
    }
    #[test]
    fn personal_library_backup_round_trip_and_unique_additions() {
        let mut list = Collection {
            title: "My playlist".into(),
            source: new_source(),
            ..Default::default()
        };
        let song = LocalTrack {
            uri: "vibefy:youtube:abcdefghijk".into(),
            title: "Song".into(),
            ..Default::default()
        };
        append(&mut list, vec![song.clone(), song]);
        assert_eq!(list.tracks.len(), 1);
        let before = fingerprint(&list);
        let data = serde_json::to_vec(&vec![list.clone()]).unwrap();
        assert_eq!(decode_backup(&data).unwrap(), vec![list.clone()]);
        list.title = "New name".into();
        assert_ne!(before, fingerprint(&list));
    }
}
