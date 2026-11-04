use spotifast::{
    addons::Id,
    mod_packages::{self, Guest},
};
use std::sync::Arc;
fn bytes(id: Id) -> Vec<u8> {
    std::fs::read(format!(
        "{}/mods/packages/{}.vmod",
        env!("CARGO_MANIFEST_DIR"),
        mod_packages::slug(id)
    ))
    .unwrap()
}
fn guest(id: Id) -> Arc<Guest> {
    mod_packages::decode(id, &bytes(id)).unwrap()
}
fn input() -> Vec<f32> {
    let mut v = vec![0.; 3328];
    v[2] = 1000.;
    v[3] = 700.;
    v[4] = 1.;
    v[5] = 1.;
    v[6] = 1.;
    v[7] = 120.;
    v[10] = 0.9;
    v[11] = 0.5;
    v[12] = 1.;
    v[18] = 64.;
    v[19] = 24.;
    v[23] = 220.;
    v[24] = 350.;
    v[25] = 90.;
    v[26] = 30.;
    for n in &mut v[64..128] {
        *n = 0.2;
    }
    v
}
#[test]
fn every_published_module_validates_and_runs_with_bounded_output() {
    for entry in mod_packages::catalog() {
        let module = guest(entry.id);
        let commands = module.render(input()).unwrap();
        assert!(commands.len() <= 4096);
        for c in commands {
            assert!(c.values.iter().all(|v| v.is_finite()));
        }
    }
}
#[test]
fn tampered_truncated_and_wrong_identity_packages_are_rejected() {
    let original = bytes(Id::Nyan);
    let mut corrupt = original.clone();
    corrupt[20] ^= 1;
    assert!(mod_packages::decode(Id::Nyan, &corrupt).is_err());
    assert!(mod_packages::decode(Id::Nyan, &original[..original.len() - 1]).is_err());
    assert!(mod_packages::decode(Id::Oneko, &original).is_err());
}
#[test]
fn mod_artwork_is_loaded_from_the_package() {
    let cat = guest(Id::CatJam);
    assert_eq!(cat.sprites[0].frames, 325);
    assert_eq!(cat.sprites[0].columns, 16);
    assert_eq!(guest(Id::Nyan).sprites[0].files.len(), 6);
    assert_eq!(guest(Id::Oneko).sprites[0].frames, 32);
}
#[test]
fn track_peek_is_bounded_and_keeps_the_controls_clear() {
    let commands = guest(Id::TrackPeek).render(input()).unwrap();
    let cards: Vec<_> = commands.iter().filter(|v| v.kind == 6).collect();
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].values[2], 180.);
    assert!(cards[0].values[0] + 180. < 390.);
    assert!(cards[1].values[0] > 610.);
}
#[test]
fn animation_frames_follow_the_song_clock() {
    let g = guest(Id::Nyan);
    let mut v = input();
    v[4] = 0.;
    let a = g
        .render(v.clone())
        .unwrap()
        .into_iter()
        .find(|c| c.kind == 5)
        .unwrap();
    v[4] = 0.1;
    let b = g
        .render(v)
        .unwrap()
        .into_iter()
        .find(|c| c.kind == 5)
        .unwrap();
    assert_eq!(a.values[4], 0.);
    assert_eq!(b.values[4], 1.);
}
#[test]
fn disabled_or_unloaded_packages_cannot_activate() {
    let mut p = spotifast::addons::Preferences::default();
    p.require_packages = true;
    p.set(Id::WaveLink, true);
    assert!(!p.active(Id::WaveLink));
    p.ready_packages.insert(Id::WaveLink);
    assert!(p.active(Id::WaveLink));
    p.set(Id::WaveLink, false);
    assert!(!p.active(Id::WaveLink));
}
#[test]
#[ignore = "downloads public release assets; run explicitly during release validation"]
fn published_packages_download_validate_and_reopen() {
    let root = std::env::temp_dir().join(format!("vibefy-mod-download-{}", rand::random::<u64>()));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        for entry in mod_packages::catalog() {
            mod_packages::install(root.clone(), entry.id).await.unwrap();
            assert!(mod_packages::load(&root, entry.id).is_ok());
        }
    });
    std::fs::remove_dir_all(root).unwrap();
}
