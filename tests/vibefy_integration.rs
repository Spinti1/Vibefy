use spotifast::eq::{self, EqSettings, Processor};
use spotifast::paths::{AppDirs, migrate_legacy};
use std::path::{Path, PathBuf};

fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "vibefy-integration-{name}-{}",
        rand::random::<u64>()
    ))
}
fn dirs(root: &Path) -> AppDirs {
    AppDirs {
        config: root.join("config"),
        state: root.join("state"),
        cache: root.join("cache"),
    }
}
#[test]
fn migration_copies_legacy_state_without_overwriting_vibefy_and_keeps_audio_identity() {
    let base = root("migration");
    let old = dirs(&base.join("legacy"));
    let new = dirs(&base.join("vibefy"));
    old.ensure().unwrap();
    new.ensure().unwrap();
    std::fs::create_dir_all(old.state.join("youtube_audio")).unwrap();
    std::fs::write(old.config.join("settings.json"), b"legacy").unwrap();
    std::fs::write(new.config.join("settings.json"), b"vibefy").unwrap();
    std::fs::write(old.state.join("history.json"), b"history").unwrap();
    std::fs::write(old.state.join("spotifast.log"), b"log").unwrap();
    std::fs::write(
        old.state.join("credential-profile"),
        b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )
    .unwrap();
    let audio = old.state.join("youtube_audio/test.m4a");
    std::fs::write(&audio, b"audio").unwrap();
    let modified = std::fs::metadata(&audio).unwrap().modified().unwrap();
    migrate_legacy(&old, &new).unwrap();
    assert_eq!(
        std::fs::read(new.config.join("settings.json")).unwrap(),
        b"vibefy"
    );
    assert_eq!(std::fs::read(new.history_file()).unwrap(), b"history");
    assert_eq!(std::fs::read(old.history_file()).unwrap(), b"history");
    assert!(!new.state.join("spotifast.log").exists());
    assert_eq!(
        std::fs::metadata(new.audio_cache_dir().join("test.m4a"))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
    assert_ne!(
        std::fs::read(new.state.join("credential-profile")).unwrap(),
        b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(
        std::fs::read(new.state.join("credential-profile-source")).unwrap(),
        b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    std::fs::remove_file(new.history_file()).unwrap();
    migrate_legacy(&old, &new).unwrap();
    assert!(
        !new.history_file().exists(),
        "cleared history must never return"
    );
    std::fs::remove_dir_all(base).unwrap();
}
#[test]
fn migration_rejects_a_destination_inside_legacy_data() {
    let base = root("overlap");
    let old = dirs(&base);
    old.ensure().unwrap();
    let new = AppDirs {
        config: old.config.join("child"),
        state: old.state.join("child"),
        cache: old.cache.join("child"),
    };
    assert!(migrate_legacy(&old, &new).is_err());
    std::fs::remove_dir_all(base).unwrap();
}
fn sine() -> Vec<f64> {
    (0..44_100)
        .flat_map(|i| {
            let sample = (std::f64::consts::TAU * 60.0 * i as f64 / 44_100.0).sin() * 0.1;
            [sample, sample]
        })
        .collect()
}
fn rms(samples: &[f64]) -> f64 {
    (samples.iter().map(|s| s * s).sum::<f64>() / samples.len() as f64).sqrt()
}
#[test]
fn shared_equalizer_changes_live_audio_and_bypasses_when_disabled() {
    let shared = eq::shared();
    let mut processor = Processor::new(shared.clone());
    let original = sine();
    let mut flat = original.clone();
    processor.process(&mut flat);
    assert_eq!(flat, original);
    {
        let mut settings = shared.lock().unwrap();
        settings.on = true;
        settings.bands_db[0] = 9.0;
    }
    let mut boosted = original.clone();
    processor.process(&mut boosted);
    let gain = 20.0 * (rms(&boosted[20_000..]) / rms(&original[20_000..])).log10();
    assert!((gain - 9.0).abs() < 0.5, "bass gain {gain}");
    shared.lock().unwrap().on = false;
    let mut bypass = original.clone();
    processor.process(&mut bypass);
    assert_eq!(bypass, original);
    *shared.lock().unwrap() = EqSettings {
        on: true,
        preamp_db: -6.0,
        mono: true,
        balance: 1.0,
        ..Default::default()
    };
    processor.reset();
    let mut channels = vec![0.2, 0.0, 0.2, 0.0];
    processor.process(&mut channels);
    assert_eq!(channels[0], 0.0);
    assert!((channels[1] - 0.1 * 10f64.powf(-6.0 / 20.0)).abs() < 1e-6);
}
#[tokio::test]
async fn unified_provider_uses_available_spotify_lyrics_without_network() {
    let base = root("lyrics");
    let exact = spotifast::lyrics::Lyrics {
        synced: true,
        lines: vec![spotifast::lyrics::Line {
            at_ms: Some(1200),
            text: "Exact recording".into(),
        }],
        ..Default::default()
    };
    let result = spotifast::spicy_lyrics::fetch_with_fallback(
        &reqwest::Client::new(),
        &base,
        "spotify:track:fixture",
        &spotifast::lyrics::Query {
            artist: "Fixture".into(),
            title: "Fixture".into(),
            album: String::new(),
            duration_ms: 100_000,
        },
        "",
        false,
        Some(exact),
    )
    .await
    .unwrap()
    .document
    .unwrap();
    assert_eq!(result.provider, "Spotify");
    assert_eq!(result.lines[0].start_ms, Some(1200));
}
