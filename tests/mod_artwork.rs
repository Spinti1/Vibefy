#[path = "../src/mod_animation.rs"]
mod animation;
use spotifast::mod_tempo;
pub use spotifast::theme;

#[test]
fn original_six_nyan_frames_are_unique_pngs() {
    let mut hashes = std::collections::BTreeSet::new();
    for frame in animation::NYAN_FRAMES {
        let image = image::load_from_memory(frame).unwrap();
        assert_eq!((image.width(), image.height()), (34, 21));
        use sha2::Digest;
        hashes.insert(sha2::Sha256::digest(frame).to_vec());
    }
    assert_eq!(hashes.len(), 6);
}
#[test]
fn original_cat_jam_frames_keep_transparency_and_fit_the_atlas() {
    let image = image::load_from_memory(animation::CAT_JAM)
        .unwrap()
        .into_rgba8();
    assert_eq!(
        (image.width(), image.height()),
        (
            (animation::CAT_COLUMNS * animation::CAT_WIDTH) as u32,
            (animation::CAT_ROWS * animation::CAT_HEIGHT) as u32
        )
    );
    assert!(image.pixels().any(|p| p[3] == 0));
    assert!(image.pixels().any(|p| p[3] > 200));
    for frame in 0..animation::CAT_FRAMES {
        let uv = animation::cat_uv(frame);
        assert!(
            uv.min.x >= 0.0
                && uv.min.y >= 0.0
                && uv.max.x <= 1.0
                && uv.max.y <= 1.0
                && uv.is_positive()
        );
    }
    let first = image.get_pixel(30, 30);
    let other = image.get_pixel(
        (7 * animation::CAT_WIDTH + 30) as u32,
        (6 * animation::CAT_HEIGHT + 30) as u32,
    );
    assert_ne!(first, other);
}
#[test]
fn original_animation_clocks_follow_bpm_pause_seek_and_loop() {
    assert_eq!(animation::nyan_frame(0, Some(120.0)), 0);
    assert_eq!(animation::nyan_frame(100, Some(120.0)), 1);
    assert_eq!(animation::nyan_frame(100, Some(180.0)), 1);
    assert_eq!(animation::nyan_frame(500, Some(120.0)), 0);
    assert_eq!(
        animation::cat_frame(9751, Some(animation::CAT_BASE_BPM as f32)),
        0
    );
    let normal = animation::cat_frame(1000, Some(90.0));
    let faster = animation::cat_frame(1000, Some(180.0));
    assert!(faster >= normal * 2 && faster <= normal * 2 + 1);
    for _ in 0..10 {
        assert_eq!(
            animation::cat_frame(1234, Some(140.0)),
            animation::cat_frame(1234, Some(140.0))
        );
    }
    assert_eq!(
        animation::nyan_frame(100, None),
        animation::nyan_frame(100, Some(f32::NAN))
    );
}

fn envelope(bpm: f32) -> Vec<f32> {
    (0..3000)
        .map(|index| {
            let beat = (index as f32 / 100.0 * bpm / 60.0).fract();
            0.02 + (-beat * 50.0).exp() * 0.8
        })
        .collect()
}
#[test]
fn tempo_estimate_follows_different_pulse_rates_and_rejects_silence() {
    for bpm in [75.0, 90.0, 100.0, 120.0, 135.0, 150.0, 180.0] {
        let estimated = mod_tempo::estimate_envelope(&envelope(bpm), 100.0).unwrap();
        assert!(
            (estimated - bpm).abs() < 2.0,
            "expected {bpm}, got {estimated}"
        );
    }
    assert!(mod_tempo::estimate_envelope(&[0.0; 3000], 100.0).is_none());
    assert!(mod_tempo::estimate_envelope(&[f32::NAN; 3000], 100.0).is_none());
    assert!(mod_tempo::estimate_envelope(&[], f32::INFINITY).is_none());
}
#[test]
fn native_decoder_estimates_a_click_track_without_a_model() {
    let file = std::env::temp_dir().join(format!("vibefy-tempo-{}.flac", rand::random::<u64>()));
    std::fs::write(&file, include_bytes!("../src/testdata/mod-tempo-120.flac")).unwrap();
    let bpm = mod_tempo::estimate_file(&file, &std::sync::atomic::AtomicBool::new(false))
        .unwrap()
        .unwrap();
    assert!((bpm - 120.0).abs() < 2.0, "{bpm}");
    std::fs::remove_file(file).unwrap();
    assert!(
        mod_tempo::estimate_file(
            std::path::Path::new("does-not-exist"),
            &std::sync::atomic::AtomicBool::new(true)
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn visual_settings_migrate_and_remain_independent() {
    use spotifast::addons::{Id, Preferences};
    let mut preferences: Preferences =
        serde_json::from_str(r#"{"visual_style":1,"intensity":1.8,"particles":true}"#).unwrap();
    assert_eq!(preferences.visual(Id::Vivy).style, 1);
    preferences.visual_mut(Id::Vivy).intensity = 0.7;
    assert_eq!(preferences.visual(Id::Backmusic).intensity, 1.8);
    preferences.visual_mut(Id::Backmusic).style = 0;
    assert_eq!(preferences.visual(Id::Vivy).style, 1);
    assert_eq!(preferences.available_visualizers().count(), 3);
    preferences.set(Id::Vivy, true);
    preferences.set(Id::Backmusic, true);
    preferences.set(Id::Backmusic, false);
    assert_eq!(
        preferences.available_visualizers().collect::<Vec<_>>(),
        vec![Id::Vivy, Id::Backmusic, Id::SoundVisualizer]
    );
    let restored: Preferences =
        serde_json::from_str(&serde_json::to_string(&preferences).unwrap()).unwrap();
    assert_eq!(restored, preferences);
}
