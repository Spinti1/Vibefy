//! Portable playlist metadata. No audio, cache paths, account sessions or credentials.
use crate::{addons::Collection, api::models::ArtistRef, personal_library, player::LocalTrack};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

const FORMAT: &str = "vibefy-playlists";
const VERSION: u32 = 1;
const MAX_BYTES: usize = 20_000_000;

#[derive(Serialize, Deserialize)]
struct Backup {
    format: String,
    version: u32,
    playlists: Vec<Playlist>,
}
#[derive(Serialize, Deserialize)]
struct Playlist {
    source: String,
    title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    tracks: Vec<Track>,
}
#[derive(Serialize, Deserialize)]
struct Track {
    uri: String,
    title: String,
    #[serde(default)]
    artists: Vec<ArtistRef>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    album: String,
    #[serde(default)]
    duration_ms: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    artwork: Option<String>,
}
fn safe_image(image: Option<String>) -> Option<String> {
    image.filter(|s| crate::addons::safe_art_url(s))
}
fn sanitize(lists: &mut [Collection]) {
    for list in lists {
        list.image = safe_image(list.image.take());
        for t in &mut list.tracks {
            t.cover_url = safe_image(t.cover_url.take());
            t.art_url = safe_image(t.art_url.take());
            t.art_small_url = safe_image(t.art_small_url.take());
        }
    }
}
fn validate(lists: &[Collection]) -> Result<()> {
    ensure!(
        lists.len() <= personal_library::MAX_LISTS && lists.iter().all(personal_library::validate),
        "Backup contains unsupported playlist data"
    );
    Ok(())
}
pub fn encode(lists: &[Collection]) -> Result<Vec<u8>> {
    validate(lists)?;
    let backup = Backup {
        format: FORMAT.into(),
        version: VERSION,
        playlists: lists
            .iter()
            .map(|c| Playlist {
                source: c.source.clone(),
                title: c.title.clone(),
                image: safe_image(c.image.clone()),
                tracks: c
                    .tracks
                    .iter()
                    .map(|t| Track {
                        uri: t.uri.clone(),
                        title: if t.title.is_empty() {
                            t.name.clone()
                        } else {
                            t.title.clone()
                        },
                        artists: t.artists.clone(),
                        album: if t.album.is_empty() {
                            t.album_name.clone()
                        } else {
                            t.album.clone()
                        },
                        duration_ms: t.duration_ms,
                        artwork: safe_image(
                            t.cover_url
                                .clone()
                                .or(t.art_url.clone())
                                .or(t.art_small_url.clone()),
                        ),
                    })
                    .collect(),
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&backup)?;
    ensure!(bytes.len() <= MAX_BYTES, "Backup exceeds 20 MB");
    Ok(bytes)
}
pub fn decode(bytes: &[u8]) -> Result<Vec<Collection>> {
    ensure!(bytes.len() <= MAX_BYTES, "Backup exceeds 20 MB");
    let mut lists = if bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'[') {
        // Backups exported before 0.4.52 remain readable.
        serde_json::from_slice::<Vec<Collection>>(bytes)
            .context("Choose a Vibefy playlist backup JSON file")?
    } else {
        let backup: Backup =
            serde_json::from_slice(bytes).context("Choose a Vibefy playlist backup JSON file")?;
        ensure!(
            backup.format == FORMAT && backup.version == VERSION,
            "Unsupported playlist backup version"
        );
        backup
            .playlists
            .into_iter()
            .map(|p| Collection {
                title: p.title,
                source: p.source,
                image: p.image,
                owner: Some("Restored playlist".into()),
                tracks: p
                    .tracks
                    .into_iter()
                    .map(|t| LocalTrack {
                        uri: t.uri,
                        title: t.title.clone(),
                        name: t.title,
                        artists: t.artists,
                        album: t.album.clone(),
                        album_name: t.album,
                        duration_ms: t.duration_ms,
                        cover_url: t.artwork.clone(),
                        art_url: t.artwork.clone(),
                        art_small_url: t.artwork,
                        ..Default::default()
                    })
                    .collect(),
            })
            .collect()
    };
    validate(&lists)?;
    sanitize(&mut lists);
    Ok(lists)
}
/// Stage the entire restore before replacing state. Conflicting snapshots become
/// separate local playlists; an existing playlist is never silently overwritten.
pub fn merge(current: &mut Vec<Collection>, incoming: Vec<Collection>) -> Result<usize> {
    validate(&incoming)?;
    let mut staged = current.clone();
    let mut sources: std::collections::HashMap<_, _> = staged
        .iter()
        .map(|c| (c.source.clone(), personal_library::fingerprint(c)))
        .collect();
    let mut added = 0;
    'incoming: for mut list in incoming {
        let fingerprint = personal_library::fingerprint(&list);
        if sources.get(&list.source) == Some(&fingerprint) {
            continue;
        }
        if sources.contains_key(&list.source) {
            use sha2::{Digest, Sha256};
            let original = list.source.clone();
            for attempt in 0..=personal_library::MAX_LISTS {
                let digest = format!(
                    "{:x}",
                    Sha256::digest(format!("vibefy-backup:{original}:{fingerprint}:{attempt}"))
                );
                list.source = format!("vibefy:playlist:{}", &digest[..32]);
                match sources.get(&list.source) {
                    Some(existing) if existing == &fingerprint => continue 'incoming,
                    Some(_) => continue,
                    None => break,
                }
            }
            ensure!(
                !sources.contains_key(&list.source),
                "No room for another restored snapshot"
            );
        }
        sources.insert(list.source.clone(), fingerprint);
        staged.push(list);
        added += 1;
    }
    validate(&staged)?;
    ensure!(
        serde_json::to_vec(&staged)?.len() <= MAX_BYTES,
        "Restored library would exceed 20 MB. Existing playlists were kept."
    );
    *current = staged;
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn playlist() -> Collection {
        Collection {
            title: "Guest mix".into(),
            source: personal_library::new_source(),
            tracks: vec![LocalTrack {
                uri: "vibefy:youtube:abcdefghijk".into(),
                title: "Example".into(),
                name: "Example".into(),
                artists: vec![ArtistRef {
                    name: "Artist".into(),
                    ..Default::default()
                }],
                duration_ms: 123000,
                ..Default::default()
            }],
            ..Default::default()
        }
    }
    #[test]
    fn guest_backup_preserves_local_ids_metadata_order_and_duplicates_without_audio() {
        let mut p = playlist();
        p.tracks.push(p.tracks[0].clone());
        let bytes = encode(&[p.clone()]).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded[0].source, p.source);
        assert_eq!(decoded[0].tracks, p.tracks);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["format"], FORMAT);
        assert_eq!(
            value["playlists"][0]["tracks"][0]
                .as_object()
                .unwrap()
                .len(),
            4
        );
        assert!(bytes.len() < 1000);
    }
    #[test]
    fn old_backups_load_and_restore_keeps_existing_edits_without_duplicates() {
        let original = playlist();
        let restored = decode(&serde_json::to_vec(&[original.clone()]).unwrap()).unwrap();
        let mut current = vec![original.clone()];
        current[0].tracks.clear();
        assert_eq!(merge(&mut current, restored.clone()).unwrap(), 1);
        assert!(current[0].tracks.is_empty());
        assert_ne!(current[0].source, current[1].source);
        assert_eq!(merge(&mut current, restored).unwrap(), 0);
    }
    #[test]
    fn invalid_restore_is_atomic_and_future_formats_are_rejected() {
        let mut current = vec![playlist()];
        let before = current.clone();
        let mut bad = playlist();
        bad.tracks[0].uri = "file:///private".into();
        assert!(merge(&mut current, vec![playlist(), bad]).is_err());
        assert_eq!(current, before);
        assert!(decode(br#"{"format":"vibefy-playlists","version":99,"playlists":[]}"#).is_err());
    }
    #[test]
    fn equally_named_playlists_with_identical_songs_keep_their_distinct_identities() {
        let first = playlist();
        let mut second = first.clone();
        second.source = personal_library::new_source();
        let mut restored = Vec::new();
        assert_eq!(
            merge(&mut restored, vec![first.clone(), second.clone()]).unwrap(),
            2
        );
        assert_eq!(restored[0].source, first.source);
        assert_eq!(restored[1].source, second.source);
        assert_eq!(merge(&mut restored, vec![first, second]).unwrap(), 0);
    }
}
