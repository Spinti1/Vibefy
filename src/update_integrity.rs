//! Bind downloads to GitHub's SHA-256 digest as well as fastframe's manifest.
//! Kept independent of app state so corruption and redirect cases can be tested.
use anyhow::{Context, Result, ensure};
use fastframe_update::{Request, Response, Transport};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Read},
    sync::Mutex,
};

const API: &str = "https://api.github.com/repos/Spinti1/Vibefy/releases/tags/";
const DOWNLOAD: &str = "https://github.com/Spinti1/Vibefy/releases/download/";
const METADATA_LIMIT: u64 = 2 * 1024 * 1024;

#[derive(Clone)]
struct Expected {
    size: u64,
    digest: String,
}

pub struct VerifiedTransport<T> {
    inner: T,
    assets: Mutex<BTreeMap<String, Expected>>,
}
impl<T> VerifiedTransport<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            assets: Mutex::new(BTreeMap::new()),
        }
    }
}

fn https_release_host(value: &str) -> bool {
    reqwest::Url::parse(value).is_ok_and(|u| {
        u.scheme() == "https"
            && u.username().is_empty()
            && u.password().is_none()
            && u.port().is_none()
            && matches!(
                u.host_str(),
                Some(
                    "github.com"
                        | "release-assets.githubusercontent.com"
                        | "objects.githubusercontent.com"
                )
            )
    })
}

impl<T: Transport> Transport for VerifiedTransport<T> {
    fn get(&self, request: &Request<'_>) -> Result<Response> {
        let mut response = self.inner.get(request)?;
        if request.url.starts_with(API) && response.status == 200 {
            let mut bytes = Vec::new();
            response
                .body
                .take(METADATA_LIMIT + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() as u64 <= METADATA_LIMIT,
                "Release metadata is too large"
            );
            let metadata: serde_json::Value = serde_json::from_slice(&bytes)?;
            let tag = metadata["tag_name"]
                .as_str()
                .context("Missing release tag")?;
            ensure!(
                request.url.strip_prefix(API) == Some(tag),
                "Release tag mismatch"
            );
            let assets = metadata["assets"]
                .as_array()
                .context("Missing release assets")?;
            ensure!(assets.len() <= 128, "Too many release assets");
            let mut expected = BTreeMap::new();
            for asset in assets {
                let name = asset["name"].as_str().context("Missing asset name")?;
                let uri = asset["browser_download_url"]
                    .as_str()
                    .context("Missing asset URL")?;
                ensure!(
                    !name.is_empty()
                        && !name.contains(['/', '\\'])
                        && uri == format!("{DOWNLOAD}{tag}/{name}")
                        && https_release_host(uri),
                    "Unexpected release asset URL"
                );
                let digest = asset["digest"]
                    .as_str()
                    .and_then(|v| v.strip_prefix("sha256:"))
                    .context("GitHub did not publish the asset SHA-256 digest")?;
                ensure!(
                    digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
                    "Invalid asset SHA-256 digest"
                );
                let size = asset["size"].as_u64().context("Missing asset size")?;
                ensure!(
                    size > 0 && size <= 2 * 1024 * 1024 * 1024,
                    "Invalid release asset size"
                );
                ensure!(
                    expected
                        .insert(
                            uri.to_owned(),
                            Expected {
                                size,
                                digest: digest.to_ascii_lowercase()
                            }
                        )
                        .is_none(),
                    "Duplicate release asset"
                );
            }
            *self
                .assets
                .lock()
                .map_err(|_| anyhow::anyhow!("Update integrity state unavailable"))? = expected;
            return Ok(Response::ok(io::Cursor::new(bytes)));
        }
        let mut assets = self
            .assets
            .lock()
            .map_err(|_| anyhow::anyhow!("Update integrity state unavailable"))?;
        let expected = assets.get(request.url).cloned();
        ensure!(
            !request.url.starts_with(DOWNLOAD) || expected.is_some(),
            "Download has no verified GitHub metadata"
        );
        if let Some(expected) = expected {
            if matches!(response.status, 301 | 302 | 303 | 307 | 308) {
                let location = response
                    .location
                    .as_deref()
                    .context("Missing release redirect")?;
                ensure!(
                    https_release_host(location),
                    "Unsafe release download redirect"
                );
                ensure!(assets.len() < 256, "Too many release redirects");
                assets.insert(location.to_owned(), expected);
            } else if response.status == 200 {
                response.body = Box::new(VerifiedReader {
                    inner: response.body,
                    expected,
                    received: 0,
                    hash: Sha256::new(),
                    finished: false,
                });
            }
        }
        Ok(response)
    }
}

struct VerifiedReader {
    inner: Box<dyn Read + Send>,
    expected: Expected,
    received: u64,
    hash: Sha256,
    finished: bool,
}
impl Read for VerifiedReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let n = self.inner.read(buffer)?;
        self.received = self.received.saturating_add(n as u64);
        if self.received > self.expected.size {
            return Err(io::Error::other("Download exceeds GitHub release size"));
        }
        self.hash.update(&buffer[..n]);
        if n == 0 && !self.finished {
            if self.received != self.expected.size
                || format!("{:x}", self.hash.clone().finalize()) != self.expected.digest
            {
                return Err(io::Error::other(
                    "Download does not match GitHub release size and SHA-256 digest",
                ));
            }
            self.finished = true;
        }
        Ok(n)
    }
}
