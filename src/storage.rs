//! Disposable cache accounting and cleanup. Called only on backend file workers.
use crate::paths::AppDirs;
use std::{
    io,
    path::{Path, PathBuf},
};

pub const ADVISORY_BYTES: u64 = 2_000_000_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheKind {
    Artwork,
    Audio,
    Lyrics,
    Library,
    All,
}
#[derive(Clone, Debug, Default)]
pub struct Usage {
    pub artwork: u64,
    pub audio: u64,
    pub lyrics: u64,
    pub library: u64,
}
impl Usage {
    pub fn total(&self) -> u64 {
        self.artwork
            .saturating_add(self.audio)
            .saturating_add(self.lyrics)
            .saturating_add(self.library)
    }
    pub fn bytes(&self, kind: CacheKind) -> u64 {
        match kind {
            CacheKind::Artwork => self.artwork,
            CacheKind::Audio => self.audio,
            CacheKind::Lyrics => self.lyrics,
            CacheKind::Library => self.library,
            CacheKind::All => self.total(),
        }
    }
}
#[derive(Debug)]
pub struct Outcome {
    pub usage: Usage,
    pub cleared: u64,
    pub warning: Option<String>,
}
pub fn size_label(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} GB", bytes as f64 / 1_000_000_000.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    }
}
pub fn should_notify(bytes: u64, last: u64, now: u64) -> bool {
    bytes >= ADVISORY_BYTES && (last == 0 || now.saturating_sub(last) >= 86_400)
}
fn roots(dirs: &AppDirs, kind: CacheKind) -> Vec<PathBuf> {
    match kind {
        CacheKind::Artwork => vec![dirs.art_cache_dir()],
        CacheKind::Audio => vec![dirs.audio_cache_dir()],
        CacheKind::Lyrics => vec![dirs.lyrics_cache_dir()],
        CacheKind::Library => dirs.account_cache_dirs().to_vec(),
        CacheKind::All => [
            CacheKind::Artwork,
            CacheKind::Audio,
            CacheKind::Lyrics,
            CacheKind::Library,
        ]
        .into_iter()
        .flat_map(|kind| roots(dirs, kind))
        .collect(),
    }
}
// Never follow symlinks or Windows junctions, including at the root.
fn linked(meta: &std::fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}
fn visit(root: &Path, clear: bool, bytes: &mut u64) -> io::Result<()> {
    let meta = match std::fs::symlink_metadata(root) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if linked(&meta) {
        return Ok(());
    }
    #[cfg(windows)]
    let boundary = root.to_path_buf();
    #[cfg(not(windows))]
    let boundary = std::fs::canonicalize(root)?;
    fn walk(path: &Path, boundary: &Path, clear: bool, bytes: &mut u64) -> io::Result<()> {
        let meta = std::fs::symlink_metadata(path)?;
        if linked(&meta) {
            return Ok(());
        }
        #[cfg(windows)]
        let contained = path.starts_with(boundary);
        #[cfg(not(windows))]
        let contained = std::fs::canonicalize(path)?.starts_with(boundary);
        if !contained {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Cache path leaves its root",
            ));
        }
        if meta.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                // Descend only by one name returned by this directory. Never
                // trust an absolute entry path or follow a reparse point.
                // MSIX legitimately redirects individual children to LocalCache
                // while canonicalize(root) still names the unvirtualized root.
                let name = entry.file_name();
                let mut components = Path::new(&name).components();
                if !matches!(components.next(), Some(std::path::Component::Normal(_)))
                    || components.next().is_some()
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid cache entry name",
                    ));
                }
                match walk(&path.join(name), boundary, clear, bytes) {
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    result => result?,
                }
            }
            // Retain directories so concurrent cache writers can keep working.
        } else if meta.is_file() {
            if clear {
                std::fs::remove_file(path)?;
            }
            *bytes = bytes.saturating_add(meta.len());
        }
        Ok(())
    }
    walk(root, &boundary, clear, bytes)
}
pub fn maintain(dirs: &AppDirs, clear: Option<CacheKind>) -> Outcome {
    let mut freed = 0;
    let mut warnings = Vec::new();
    if let Some(kind) = clear {
        for root in roots(dirs, kind) {
            if let Err(error) = visit(&root, true, &mut freed) {
                warnings.push(error.to_string());
            }
        }
    }
    let mut usage = Usage::default();
    for (kind, total) in [
        (CacheKind::Artwork, &mut usage.artwork),
        (CacheKind::Audio, &mut usage.audio),
        (CacheKind::Lyrics, &mut usage.lyrics),
        (CacheKind::Library, &mut usage.library),
    ] {
        for root in roots(dirs, kind) {
            if let Err(error) = visit(&root, false, total) {
                warnings.push(error.to_string());
            }
        }
    }
    warnings.sort();
    warnings.dedup();
    Outcome {
        usage,
        cleared: freed,
        warning: (!warnings.is_empty()).then(|| {
            format!(
                "Some cache files could not be accessed: {}",
                warnings.join("; ")
            )
        }),
    }
}
