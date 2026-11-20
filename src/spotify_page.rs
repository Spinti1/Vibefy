//! Bounded snapshots of the user's rendered Spotify library, without Web API reads.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCRIPT: &str = include_str!("../assets/spotify-page-reader.js");
pub fn target(path: &str) -> Result<(String, String, bool)> {
    if path == "me/playlists" {
        return Ok((
            path.into(),
            "https://open.spotify.com/collection".into(),
            true,
        ));
    }
    if path == "me/tracks" {
        return Ok((
            path.into(),
            "https://open.spotify.com/collection/tracks".into(),
            false,
        ));
    }
    let rest = path.strip_prefix("playlists/").unwrap_or_default();
    let (id, suffix) = rest.split_once('/').unwrap_or((rest, ""));
    ensure!(
        id.len() == 22
            && id.bytes().all(|b| b.is_ascii_alphanumeric())
            && matches!(suffix, "" | "items" | "tracks"),
        "Unsupported Spotify page"
    );
    Ok((
        format!("playlists/{id}"),
        format!("https://open.spotify.com/playlist/{id}"),
        false,
    ))
}
#[derive(Default)]
pub struct Snapshot {
    rows: BTreeMap<usize, Value>,
    playlists: BTreeMap<String, Value>,
    positions: BTreeSet<usize>,
    total: Option<usize>,
    meta: Value,
    restarts: u8,
}
impl Snapshot {
    pub fn len(&self, library: bool) -> usize {
        if library {
            self.playlists.len() + self.positions.len()
        } else {
            self.rows.len()
        }
    }
    pub fn absorb(&mut self, value: Value, library: bool) -> Result<bool> {
        let mut restarted = false;
        ensure!(
            value["error"].is_null(),
            "Spotify changed its page layout. No partial playlist was saved."
        );
        ensure!(
            value["collapsed"].as_u64().unwrap_or(0) == 0,
            "Expand playlist folders in the Spotify window, then refresh the library."
        );
        let rows = value["rows"]
            .as_array()
            .filter(|v| v.len() <= 5000)
            .ok_or_else(|| anyhow::anyhow!("Unsupported Spotify page rows"))?;
        if library {
            if let Some(total) = value["total"].as_u64() {
                ensure!(total <= 20000, "Spotify library exceeds the supported size");
                if self.total.is_some_and(|n| n != total as usize) {
                    self.restarts += 1;
                    ensure!(
                        self.restarts <= 3,
                        "Spotify library keeps changing. Wait for Spotify to finish loading, then refresh."
                    );
                    self.positions.clear();
                    self.playlists.clear();
                    restarted = true;
                }
                self.total = Some(total as usize);
                for index in value["positions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_u64)
                {
                    ensure!(
                        index < total,
                        "Spotify returned an invalid library position"
                    );
                    self.positions.insert(index as usize);
                }
            }
            for row in rows {
                let id = row["id"].as_str().unwrap_or_default();
                ensure!(
                    id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric()),
                    "Invalid Spotify playlist link"
                );
                self.playlists.insert(id.into(), row.clone());
            }
            ensure!(
                self.playlists.len() <= 2000,
                "Choose a smaller Spotify library view (maximum 2,000 playlists)"
            );
        } else {
            let total = value["total"]
                .as_u64()
                .filter(|v| *v <= 20000)
                .ok_or_else(|| anyhow::anyhow!("Spotify did not expose the complete song count"))?
                as usize;
            ensure!(
                self.total.is_none_or(|v| v == total),
                "Spotify playlist changed during reading. The existing local copy was kept."
            );
            self.total = Some(total);
            for row in rows {
                let index = row["index"]
                    .as_u64()
                    .filter(|i| *i < total as u64)
                    .ok_or_else(|| anyhow::anyhow!("Spotify returned an invalid song position"))?
                    as usize;
                if let Some(previous) = self.rows.get(&index) {
                    ensure!(
                        previous["item"]["uri"] == row["item"]["uri"],
                        "Spotify playlist changed during reading. Retry the import."
                    );
                }
                self.rows.insert(index, row.clone());
            }
        }
        self.meta = value;
        Ok(restarted)
    }
    pub fn complete(&self, library: bool) -> bool {
        if library {
            self.meta["atEnd"] == true
                && self.total.is_none_or(|n| {
                    self.positions.len() == n && (0..n).all(|i| self.positions.contains(&i))
                })
        } else {
            self.total
                .is_some_and(|n| n == self.rows.len() && (0..n).all(|i| self.rows.contains_key(&i)))
        }
    }
    pub fn finish(self, library: bool, account: Value, id: &str) -> Result<Value> {
        ensure!(
            self.complete(library),
            "Spotify did not display every song. No partial playlist was saved."
        );
        let items: Vec<_> = if library {
            self.playlists.into_values().collect()
        } else {
            self.rows.into_values().collect()
        };
        let snapshot = format!("{:x}", Sha256::digest(serde_json::to_vec(&items)?));
        let value = json!({"account":account,"id":id,"name":self.meta["title"],"images":self.meta["images"],"owner":self.meta["owner"],"snapshot_id":snapshot,"items":items});
        ensure!(
            serde_json::to_vec(&value)?.len() <= 8 * 1024 * 1024,
            "Spotify playlist metadata exceeds the safe import size"
        );
        Ok(value)
    }
}
pub fn api_value(value: &Value, path: &str, offset: usize, limit: usize) -> Result<Value> {
    if path == "me" {
        return Ok(value.clone());
    }
    let items = value["items"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing Spotify page items"))?;
    if path.starts_with("playlists/") && !path.ends_with("/items") && !path.ends_with("/tracks") {
        let mut header = value.clone();
        header.as_object_mut().unwrap().remove("items");
        header["tracks"] = json!({"total":items.len()});
        return Ok(header);
    }
    ensure!(
        limit > 0 && limit <= 50 && offset <= items.len(),
        "Invalid Spotify page bounds"
    );
    let end = offset.saturating_add(limit).min(items.len());
    Ok(
        json!({"offset":offset,"limit":limit,"total":items.len(),"items":items[offset..end],"next":if end<items.len(){Some("rendered-page")}else{None}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spotify_library_restarts_when_loading_count_changes() {
        let mut s = Snapshot::default();
        assert!(
            !s.absorb(
                json!({"rows":[],"total":0,"positions":[],"atEnd":true}),
                true
            )
            .unwrap()
        );
        assert!(
            s.absorb(
                json!({"rows":[],"total":3,"positions":[2],"atEnd":true}),
                true
            )
            .unwrap()
        );
        assert!(!s.complete(true));
        s.absorb(
            json!({"rows":[],"total":3,"positions":[0,1],"atEnd":false}),
            true,
        )
        .unwrap();
        assert!(!s.complete(true));
        s.absorb(
            json!({"rows":[],"total":3,"positions":[2],"atEnd":true}),
            true,
        )
        .unwrap();
        assert!(s.complete(true));
        assert!(
            s.absorb(
                json!({"rows":[],"total":2,"positions":[1],"atEnd":true}),
                true
            )
            .unwrap()
        );
        assert!(!s.complete(true));
        s.absorb(
            json!({"rows":[],"total":1,"positions":[0],"atEnd":true}),
            true,
        )
        .unwrap();
        assert!(
            s.absorb(
                json!({"rows":[],"total":0,"positions":[],"atEnd":true}),
                true
            )
            .is_err()
        );
    }
    #[test]
    fn spotify_page_requires_complete_positions_and_preserves_repeats() {
        let mut s = Snapshot::default();
        s.absorb(json!({"rows":[{"index":0,"item":{"uri":"a"}},{"index":2,"item":{"uri":"a"}}],"total":3,"atEnd":true}),false).unwrap();
        assert!(!s.complete(false));
        s.absorb(
            json!({"rows":[{"index":1,"item":{"uri":"b"}}],"total":3,"atEnd":true}),
            false,
        )
        .unwrap();
        let v = s.finish(false, json!({"id":"account"}), "list").unwrap();
        assert_eq!(v["items"].as_array().unwrap().len(), 3);
        assert_eq!(
            api_value(&v, "playlists/list/items", 0, 2).unwrap()["next"],
            "rendered-page"
        );
        assert!(api_value(&v, "me/tracks", 4, 50).is_err());
    }
    #[test]
    fn spotify_page_rejects_changes_and_unknown_routes() {
        for path in [
            "me/player",
            "../me",
            "playlists/123?x=1",
            "https://evil.test",
        ] {
            assert!(target(path).is_err());
        }
        assert!(target("playlists/1234567890123456789012/items").is_ok());
        let mut s = Snapshot::default();
        s.absorb(
            json!({"rows":[{"index":0,"item":{"uri":"a"}}],"total":2}),
            false,
        )
        .unwrap();
        assert!(
            s.absorb(
                json!({"rows":[{"index":0,"item":{"uri":"b"}}],"total":2}),
                false
            )
            .is_err()
        );
        assert!(
            Snapshot::default()
                .absorb(json!({"rows":[],"total":1,"collapsed":1}), true)
                .is_err()
        );
    }
    #[test]
    fn spotify_page_library_requires_the_full_virtual_list() {
        let mut s = Snapshot::default();
        let playlist = json!({"id":"1234567890123456789012","name":"Example"});
        s.absorb(
            json!({"rows":[playlist.clone()],"total":3,"positions":[0,2],"atEnd":true}),
            true,
        )
        .unwrap();
        assert!(!s.complete(true));
        s.absorb(
            json!({"rows":[playlist],"total":3,"positions":[1],"atEnd":true}),
            true,
        )
        .unwrap();
        assert!(s.complete(true));
        assert_eq!(
            s.finish(true, json!({"id":"account"}), "").unwrap()["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}
