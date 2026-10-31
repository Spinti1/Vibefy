use spotifast::{
    paths::AppDirs,
    storage::{self, CacheKind},
};
fn setup() -> (std::path::PathBuf, AppDirs) {
    let root = std::env::temp_dir().join(format!("vibefy-cache-{}", rand::random::<u64>()));
    let dirs = AppDirs {
        config: root.join("config"),
        state: root.join("state"),
        cache: root.join("cache"),
    };
    dirs.ensure().unwrap();
    for (folder, name, size) in [
        (dirs.art_cache_dir(), "art.bin", 10),
        (dirs.audio_cache_dir(), "audio.m4a", 20),
        (dirs.lyrics_cache_dir(), "recognition.json", 30),
        (dirs.account_playlist_cache_dir("account"), "page.json", 40),
    ] {
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(name), vec![1; size]).unwrap();
    }
    for path in [
        dirs.settings_file(),
        dirs.history_file(),
        dirs.session_file(),
        dirs.state.join("credential-storage/protected.bin"),
        dirs.config.join("background.png"),
    ] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"keep").unwrap();
    }
    (root, dirs)
}
#[test]
fn individual_and_all_cleanup_preserve_personal_and_protected_data() {
    let (root, dirs) = setup();
    let initial = storage::maintain(&dirs, None);
    assert_eq!(initial.usage.total(), 100);
    assert_eq!(initial.usage.audio, 20);
    let audio = storage::maintain(&dirs, Some(CacheKind::Audio));
    assert_eq!(audio.cleared, 20);
    assert_eq!(audio.usage.total(), 80);
    assert!(dirs.lyrics_cache_dir().join("recognition.json").exists());
    let all = storage::maintain(&dirs, Some(CacheKind::All));
    assert_eq!(all.cleared, 80);
    assert_eq!(all.usage.total(), 0);
    assert!(all.warning.is_none());
    for path in [
        dirs.settings_file(),
        dirs.history_file(),
        dirs.session_file(),
        dirs.state.join("credential-storage/protected.bin"),
        dirs.config.join("background.png"),
    ] {
        assert_eq!(std::fs::read(path).unwrap(), b"keep");
    }
    assert_eq!(storage::maintain(&dirs, Some(CacheKind::All)).cleared, 0);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn advisory_uses_two_gigabytes_and_a_daily_cooldown() {
    assert!(!storage::should_notify(
        storage::ADVISORY_BYTES - 1,
        0,
        100_000
    ));
    assert!(storage::should_notify(storage::ADVISORY_BYTES, 0, 100_000));
    assert!(!storage::should_notify(
        storage::ADVISORY_BYTES,
        100_000,
        100_001
    ));
    assert!(storage::should_notify(
        storage::ADVISORY_BYTES,
        100_000,
        186_400
    ));
    assert!(!storage::should_notify(
        storage::ADVISORY_BYTES,
        200_000,
        100_000
    ));
}
#[test]
fn missing_cache_is_empty_and_does_not_create_folders() {
    let root = std::env::temp_dir().join(format!("vibefy-no-cache-{}", rand::random::<u64>()));
    let dirs = AppDirs {
        config: root.join("config"),
        state: root.join("state"),
        cache: root.join("cache"),
    };
    let result = storage::maintain(&dirs, Some(CacheKind::All));
    assert_eq!(result.usage.total(), 0);
    assert!(result.warning.is_none());
    assert!(!root.exists());
}
#[test]
fn vibefy_version_does_not_enable_the_spotifast_update_feed() {
    assert!(!spotifast::updates::source_allowed(
        &spotifast::updates::Source::github()
    ));
}
#[test]
fn only_catalogue_evidence_with_matching_duration_uses_fast_playback() {
    use spotifast::recording_match::{Match, Recording, Reference, SourceKind};
    let reference = Reference {
        title: "Song".into(),
        artists: vec!["Artist".into()],
        album: "Album".into(),
        duration_secs: 200,
    };
    let mut recording = Recording {
        duration: Some(200.5),
        ..Default::default()
    };
    for source in [
        SourceKind::MetadataMatch,
        SourceKind::VerifiedArtist,
        SourceKind::ArtistChannel,
    ] {
        assert!(!spotifast::player::fast_catalog_match(
            &reference,
            &recording,
            &Match {
                score: 1000,
                source
            }
        ));
    }
    let catalogue = Match {
        score: 1000,
        source: SourceKind::DistributedAudio,
    };
    assert!(spotifast::player::fast_catalog_match(
        &reference, &recording, &catalogue
    ));
    recording.duration = Some(215.0);
    assert!(!spotifast::player::fast_catalog_match(
        &reference, &recording, &catalogue
    ));
}
#[cfg(unix)]
#[test]
fn cleanup_never_follows_links_outside_cache() {
    let (root, dirs) = setup();
    let outside = root.join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("keep"), b"keep").unwrap();
    std::os::unix::fs::symlink(&outside, dirs.art_cache_dir().join("link")).unwrap();
    storage::maintain(&dirs, Some(CacheKind::All));
    assert_eq!(std::fs::read(outside.join("keep")).unwrap(), b"keep");
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn cleanup_never_follows_a_windows_junction_or_a_linked_root() {
    use std::os::windows::process::CommandExt;
    let (root, dirs) = setup();
    let outside = root.join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("keep"), b"keep").unwrap();
    let link = dirs.art_cache_dir().join("junction");
    let output = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&outside)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(output.status.success(), "Could not create test junction");
    let result = storage::maintain(&dirs, Some(CacheKind::All));
    assert!(result.warning.is_none());
    assert_eq!(std::fs::read(outside.join("keep")).unwrap(), b"keep");
    std::fs::remove_dir(&link).unwrap();
    std::fs::remove_dir(dirs.art_cache_dir()).unwrap();
    let output = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(dirs.art_cache_dir())
        .arg(&outside)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        storage::maintain(&dirs, Some(CacheKind::Artwork))
            .warning
            .is_none()
    );
    assert_eq!(std::fs::read(outside.join("keep")).unwrap(), b"keep");
    std::fs::remove_dir(dirs.art_cache_dir()).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
