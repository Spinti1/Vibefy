//! Self-update from GitHub releases, through fastframe-update.
//!
//! The previous release's helper installs the first release built on the
//! crate, so what it relies on stays the same: `--version` prints
//! `<command> <version>`, the new app accepts `--update-receipt` and
//! `--update-error` (stripped by [`fastframe_update::intercept`] before the
//! argument parser), and the handoff and receipt files keep their format.

pub use fastframe_update::{
    CHECK_INTERVAL, DownloadState, Installation, Kind, Prepared, Release, Source, Unsupported,
    Updater,
};
use fastframe_update::{ReqwestTransport, UpdateConfig};

#[path = "update_integrity.rs"]
mod integrity;

pub const CONFIG: UpdateConfig = UpdateConfig {
    // GitHub HTTPS, asset digests and checksums.txt establish download integrity.
    // Vibefy installers are not code-signed. Never trust Spotifast's signing key.
    publisher_key: None,
    ..UpdateConfig::new(
        "Spinti1/Vibefy",
        "Vibefy",
        "vibefy",
        env!("CARGO_PKG_VERSION"),
    )
};

/// This fork must never install an upstream Spotifast binary.
pub fn source_allowed(source: &Source) -> bool {
    source.is_github()
}

/// An updater on Spotifast's HTTP client, through the configured proxy.
pub fn updater(proxy: &crate::settings::ProxyConfig) -> anyhow::Result<Updater> {
    let builder = crate::http::blocking_builder(proxy).map_err(anyhow::Error::msg)?;
    Ok(Updater::new(
        CONFIG,
        integrity::VerifiedTransport::new(ReqwestTransport::new(builder)?),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_config_is_valid() {
        CONFIG.validate().unwrap();
        assert_eq!(CONFIG.current_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn updates_belong_to_vibefy() {
        assert_eq!(CONFIG.repository, "Spinti1/Vibefy");
        assert_eq!(CONFIG.slug, "vibefy");
        assert!(source_allowed(&Source::github()));
        assert!(!source_allowed(
            &Source::local("http://127.0.0.1:9999").unwrap()
        ));
        CONFIG.validate().unwrap();
    }

    /// An AppImage may replace itself only because the release workflow
    /// builds the AppImages into the signed checksums.txt;
    /// packaging/test-release-names.py checks that side.
    #[test]
    fn unshipped_appimages_do_not_update_themselves() {
        const { assert!(!CONFIG.appimage) };
    }
}
