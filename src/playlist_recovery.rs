//! One-time, read-only recovery of local snapshots from the retired transfer service.
//! No account connection, polling, remote mutations or transfer queues are retained.
use crate::{addons::Collection, personal_library, player::LocalTrack};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

#[derive(Deserialize)]
struct Snapshot {
    version: u32,
    #[serde(default)]
    deleted: HashSet<String>,
    lists: BTreeMap<String, SavedList>,
}
#[derive(Deserialize)]
struct SavedList {
    title: String,
    youtube_id: String,
    tracks: Vec<LocalTrack>,
    #[serde(default)]
    extra: Vec<LocalTrack>,
    image: Option<String>,
    #[serde(default)]
    deleting: Option<serde_json::Value>,
}

pub(super) fn read(root: &Path) -> Result<Vec<Collection>> {
    let dir = root.join("playlist-mirror");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        !std::fs::symlink_metadata(&dir)?.file_type().is_symlink(),
        "Cannot recover a linked storage directory"
    );
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name
            .strip_suffix(".json")
            .is_some_and(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()))
            && entry.file_type()?.is_file()
        {
            paths.push(entry.path());
            ensure!(
                paths.len() <= 200,
                "Too many old playlist snapshots to recover automatically"
            );
        }
    }
    paths.sort();
    let mut recovered = BTreeMap::new();
    let mut read_bytes = 0u64;
    for path in paths {
        let len = std::fs::metadata(&path)?.len();
        read_bytes = read_bytes.saturating_add(len);
        ensure!(
            read_bytes <= 64 * 1024 * 1024,
            "Old playlist snapshots exceed the recovery size limit"
        );
        let snapshot: Snapshot = serde_json::from_slice(&std::fs::read(&path)?)
            .context("An old playlist snapshot could not be read; its original file was kept")?;
        ensure!(
            snapshot.version == 1,
            "Unsupported old playlist snapshot; its original file was kept"
        );
        for (id, saved) in snapshot.lists {
            if snapshot.deleted.contains(&id) || saved.deleting.is_some() {
                continue;
            }
            ensure!(
                id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric()),
                "Invalid old playlist identity"
            );
            let source = if saved.youtube_id.is_empty() {
                // Unfinished copies survive as local editable playlists, without starting a transfer.
                let identity = format!("{}:{id}", path.file_stem().unwrap().to_string_lossy());
                format!(
                    "vibefy:playlist:{}",
                    &format!("{:x}", Sha256::digest(identity.as_bytes()))[..32]
                )
            } else {
                ensure!(
                    crate::music_session::valid_playlist_id(&saved.youtube_id),
                    "Invalid saved YouTube playlist identity"
                );
                format!("https://www.youtube.com/playlist?list={}", saved.youtube_id)
            };
            let mut tracks = saved.tracks;
            tracks.extend(saved.extra);
            // Keep source recording identities, ordering, repetitions and unmatched songs.
            // Playback uses the same shared recording cache as other Spotify imports.
            for track in &mut tracks {
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
            let collection = Collection {
                title: saved.title,
                source: source.clone(),
                tracks,
                image: saved.image.filter(|url| crate::addons::safe_art_url(url)),
                owner: Some("Imported playlist".into()),
            };
            ensure!(
                personal_library::validate(&collection),
                "An old playlist exceeds supported metadata bounds; its original file was kept"
            );
            recovered.entry(source).or_insert(collection);
        }
    }
    Ok(recovered.into_values().collect())
}

/// Merge once without replacing newer local edits. Failed merges leave the entire library intact.
pub(super) fn merge(current: &mut Vec<Collection>, recovered: Vec<Collection>) -> Result<usize> {
    let mut sources: HashSet<_> = current.iter().map(|c| c.source.clone()).collect();
    let additions: Vec<_> = recovered
        .into_iter()
        .filter(|c| sources.insert(c.source.clone()))
        .collect();
    ensure!(
        current.len() + additions.len() <= personal_library::MAX_LISTS,
        "Export your playlists and free a local playlist slot before retrying recovery"
    );
    let bytes = current.iter().chain(&additions).try_fold(2usize, |n, c| {
        serde_json::to_vec(c).map(|b| n.saturating_add(b.len() + 1))
    })?;
    ensure!(
        bytes <= 20_000_000,
        "Export your playlists and free local metadata space before retrying recovery"
    );
    let count = additions.len();
    current.extend(additions);
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retired_transfer_recovery_preserves_songs_and_never_overwrites_or_resurrects_deletions() {
        let root =
            std::env::temp_dir().join(format!("vibefy-recovery-{:032x}", rand::random::<u128>()));
        let dir = root.join("playlist-mirror");
        std::fs::create_dir_all(&dir).unwrap();
        let first = "a".repeat(22);
        let deleted = "b".repeat(22);
        let pending = "c".repeat(22);
        let unfinished = "d".repeat(22);
        let track = serde_json::to_value(LocalTrack {
            uri: "spotify:track:0123456789012345678901".into(),
            title: "Song".into(),
            ..Default::default()
        })
        .unwrap();
        let list = |youtube: &str| serde_json::json!({"title":"Example","youtube_id":youtube,"tracks":[track.clone(),track.clone()],"extra":[track.clone()],"image":null});
        let mut lists = serde_json::Map::new();
        lists.insert(first.clone(), list("PLexample12345"));
        lists.insert(deleted.clone(), list("PLdeleted12345"));
        let mut deleting = list("PLpending12345");
        deleting["deleting"] = serde_json::json!({"youtube_done":true});
        lists.insert(pending, deleting);
        lists.insert(unfinished, list(""));
        let bytes =
            serde_json::to_vec(&serde_json::json!({"version":1,"deleted":[deleted],"lists":lists}))
                .unwrap();
        let path = dir.join(format!("{}.json", "a".repeat(64)));
        std::fs::write(&path, &bytes).unwrap();
        let recovered = read(&root).unwrap();
        assert_eq!(recovered.len(), 2);
        assert!(recovered.iter().all(|c| {
            c.tracks.len() == 3
                && c.tracks
                    .iter()
                    .all(|t| t.uri == "spotify:track:0123456789012345678901")
        }));
        assert!(recovered.iter().any(|c| personal_library::owned(&c.source)));
        let mut current = recovered.clone();
        current[0].title = "New local edit".into();
        assert_eq!(merge(&mut current, recovered).unwrap(), 0);
        assert_eq!(current[0].title, "New local edit");
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(
            root.starts_with(std::env::temp_dir())
                && root
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("vibefy-recovery-")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn retired_transfer_recovery_fails_atomically_when_local_library_is_full() {
        let mut current = vec![
            Collection {
                title: "Existing".into(),
                source: "existing".into(),
                ..Default::default()
            };
            personal_library::MAX_LISTS
        ];
        let extra = Collection {
            title: "Recovered".into(),
            source: "new".into(),
            ..Default::default()
        };
        assert!(merge(&mut current, vec![extra]).is_err());
        assert_eq!(current.len(), personal_library::MAX_LISTS);
        assert_eq!(current[0].title, "Existing");
    }
}
