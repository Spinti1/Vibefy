//! Durable playlist additions. Only explicit rejections are safe to retry.
use crate::api::client::{ApiError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Addition {
    pub account: String,
    pub playlist_id: String,
    pub playlist_name: String,
    pub uris: Vec<String>,
    pub position: Option<u32>,
}

impl Addition {
    fn key(&self) -> String {
        // Names may change; identity is the account, destination and exact edit.
        let bytes =
            serde_json::to_vec(&(&self.account, &self.playlist_id, &self.uris, self.position))
                .unwrap();
        format!("{:x}", Sha256::digest(bytes))
    }
}

#[derive(Serialize, Deserialize)]
struct Record {
    addition: Addition,
    #[serde(default)]
    sending: bool,
    #[serde(default)]
    retry_at: u64,
}

#[derive(Clone, Default)]
pub struct Outbox {
    active: Arc<Mutex<HashSet<String>>>,
    // Serialize writes, without holding a slot while Spotify asks us to wait.
    writing: Arc<tokio::sync::Mutex<()>>,
}

struct Active {
    key: String,
    active: Arc<Mutex<HashSet<String>>>,
}
impl Drop for Active {
    fn drop(&mut self) {
        self.active.lock().unwrap().remove(&self.key);
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn storage_error() -> ApiError {
    ApiError::Network("Could not save the pending playlist addition. Nothing was sent.".into())
}
fn uncertain() -> ApiError {
    ApiError::Network("Spotify did not confirm this addition. Check the playlist before trying again; it was not automatically sent twice.".into())
}

async fn save(path: &Path, record: &Record) -> Result<()> {
    let bytes = serde_json::to_vec(record).map_err(|_| storage_error())?;
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || fastframe_file::write(&path, bytes))
        .await
        .map_err(|_| storage_error())?
        .map_err(|_| storage_error())
}

/// Read only this account's requests. Sending records require manual review:
/// a crash may have happened after Spotify committed the POST.
pub async fn pending(root: &Path, account: &str) -> std::io::Result<Vec<Addition>> {
    let mut entries = match tokio::fs::read_dir(root).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut result = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if entry.path().extension().is_none_or(|v| v != "json")
            || !entry.file_type().await?.is_file()
        {
            continue;
        }
        let bytes = tokio::fs::read(entry.path()).await?;
        let record: Record = serde_json::from_slice(&bytes).map_err(std::io::Error::other)?;
        if record.addition.account == account {
            result.push(record.addition);
        }
    }
    Ok(result)
}

impl Outbox {
    /// None means the exact same operation already has a worker.
    pub async fn run<F, Fut, N>(
        &self,
        root: PathBuf,
        addition: Addition,
        mut send: F,
        queued: N,
    ) -> Option<Result<Option<String>>>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<Option<String>>>,
        N: FnOnce(),
    {
        let key = addition.key();
        if !self.active.lock().unwrap().insert(key.clone()) {
            return None;
        }
        let _active = Active {
            key: key.clone(),
            active: self.active.clone(),
        };
        Some(
            self.run_inner(
                root.join(format!("{key}.json")),
                addition,
                &mut send,
                queued,
            )
            .await,
        )
    }

    async fn run_inner<F, Fut, N>(
        &self,
        path: PathBuf,
        addition: Addition,
        send: &mut F,
        queued: N,
    ) -> Result<Option<String>>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<Option<String>>>,
        N: FnOnce(),
    {
        let mut record = match tokio::fs::read(&path).await {
            Ok(bytes) => serde_json::from_slice::<Record>(&bytes).map_err(|_| storage_error())?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Record {
                addition: addition.clone(),
                sending: false,
                retry_at: 0,
            },
            Err(_) => return Err(storage_error()),
        };
        if record.addition.key() != addition.key() {
            return Err(storage_error());
        }
        if record.sending {
            // Retire the uncertain operation, allowing a deliberate new action
            // after the user has checked Spotify. Never resume it automatically.
            let _ = tokio::fs::remove_file(&path).await;
            return Err(uncertain());
        }
        save(&path, &record).await?;
        queued();
        loop {
            tokio::time::sleep(Duration::from_secs(record.retry_at.saturating_sub(now()))).await;
            let write = self.writing.lock().await;
            record.sending = true;
            save(&path, &record).await?;
            let result = send().await;
            drop(write);
            match result {
                Err(
                    ApiError::RateLimited
                    | ApiError::PlaylistRetryAfter(_)
                    | ApiError::PlaylistNotSent,
                ) => {
                    let wait = match result {
                        Err(ApiError::PlaylistRetryAfter(seconds)) => seconds.max(30),
                        Err(ApiError::PlaylistNotSent) => 60,
                        _ => 30,
                    };
                    record.sending = false;
                    record.retry_at = now().saturating_add(wait);
                    save(&path, &record).await?;
                }
                Err(ApiError::Network(_)) | Err(ApiError::Decode(_)) => {
                    // Report the uncertain result and retire this automatic job.
                    // A later deliberate user action may try again after review.
                    let _ = tokio::fs::remove_file(&path).await;
                    return Err(uncertain());
                }
                result => {
                    // If deleting fails, leave the Sending marker so a restart
                    // never replays a successful or definitively failed write.
                    if tokio::fs::remove_file(&path).await.is_err() {
                        log::warn!(
                            "Could not retire a completed playlist addition; automatic replay is disabled"
                        );
                    }
                    return result;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn fixture() -> (PathBuf, Addition) {
        static SERIAL: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "vibefy-outbox-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let addition = Addition {
            account: "listener".into(),
            playlist_id: "list".into(),
            playlist_name: "My music".into(),
            uris: vec!["spotify:track:one".into()],
            position: None,
        };
        (root, addition)
    }

    #[tokio::test]
    async fn rate_limit_is_durable_and_resumes_only_for_its_account() {
        let (root, addition) = fixture();
        let calls = AtomicUsize::new(0);
        let outbox = Outbox::default();
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            outbox.run(
                root.clone(),
                addition.clone(),
                || async {
                    calls.fetch_add(1, Ordering::Relaxed);
                    Err(ApiError::PlaylistRetryAfter(3600))
                },
                || {},
            ),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(pending(&root, "other-account").await.unwrap().is_empty());
        assert_eq!(
            pending(&root, "listener").await.unwrap(),
            vec![addition.clone()]
        );
        let path = root.join(format!("{}.json", addition.key()));
        let mut record: Record =
            serde_json::from_slice(&tokio::fs::read(&path).await.unwrap()).unwrap();
        assert!(!record.sending);
        assert!(record.retry_at >= now() + 3598);
        // Simulate reaching the server's deadline after a restart.
        record.retry_at = 0;
        save(&path, &record).await.unwrap();
        let result = Outbox::default()
            .run(
                root.clone(),
                addition,
                || async { Ok(Some("snapshot".into())) },
                || {},
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.as_deref(), Some("snapshot"));
        assert!(pending(&root, "listener").await.unwrap().is_empty());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn uncertain_post_is_never_replayed_even_after_restart() {
        let (root, addition) = fixture();
        let result = Outbox::default()
            .run(
                root.clone(),
                addition.clone(),
                || async { Err(ApiError::Network("response lost".into())) },
                || {},
            )
            .await
            .unwrap();
        assert!(result.is_err());
        assert!(pending(&root, "listener").await.unwrap().is_empty());
        // Separately simulate a process exit after sending, before handling
        // the response. The persisted Sending marker must not be replayed.
        save(
            &root.join(format!("{}.json", addition.key())),
            &Record {
                addition: addition.clone(),
                sending: true,
                retry_at: 0,
            },
        )
        .await
        .unwrap();
        let result = Outbox::default()
            .run(
                root.clone(),
                addition,
                || async { panic!("must not replay an ambiguous POST") },
                || {},
            )
            .await
            .unwrap();
        assert!(result.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn duplicate_clicks_share_one_active_request() {
        let (root, addition) = fixture();
        let outbox = Outbox::default();
        let ready = Arc::new(tokio::sync::Notify::new());
        let task = tokio::spawn({
            let (outbox, root, addition, ready) = (
                outbox.clone(),
                root.clone(),
                addition.clone(),
                ready.clone(),
            );
            async move {
                outbox
                    .run(
                        root,
                        addition,
                        || async {
                            ready.notify_one();
                            std::future::pending().await
                        },
                        || {},
                    )
                    .await
            }
        });
        ready.notified().await;
        assert!(
            outbox
                .run(
                    root.clone(),
                    addition,
                    || async { panic!("duplicate sent") },
                    || {}
                )
                .await
                .is_none()
        );
        task.abort();
        let _ = task.await;
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn permission_and_quota_failures_do_not_loop() {
        for error in [
            ApiError::Status {
                status: 403,
                message: "permission denied".into(),
            },
            ApiError::QuotaExhausted,
            ApiError::NotSignedIn,
        ] {
            let (root, addition) = fixture();
            let calls = AtomicUsize::new(0);
            let result = Outbox::default()
                .run(
                    root.clone(),
                    addition,
                    || async {
                        calls.fetch_add(1, Ordering::Relaxed);
                        Err(error.clone())
                    },
                    || {},
                )
                .await
                .unwrap();
            assert!(result.is_err());
            assert_eq!(calls.load(Ordering::Relaxed), 1);
            assert!(pending(&root, "listener").await.unwrap().is_empty());
            tokio::fs::remove_dir_all(root).await.unwrap();
        }
    }
}
