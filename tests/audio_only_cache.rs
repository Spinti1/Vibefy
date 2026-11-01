use spotifast::audio_only_mp4::{contains_video, extract};

fn atom(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut data = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    data.extend_from_slice(kind);
    data.extend_from_slice(body);
    data
}
fn numbers(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|n| n.to_be_bytes()).collect()
}
fn fixture(wide: bool, fixed: bool) -> Vec<u8> {
    let ftyp = atom(b"ftyp", b"isom\0\0\0\0isommp42");
    let mut media = b"VIDEO-NOT-AUDIO".repeat(80);
    let first = ftyp.len() + 8 + media.len();
    media.extend_from_slice(b"AAC1AAC2");
    media.extend_from_slice(b"UNUSED-VIDEO-DATA");
    let second = ftyp.len() + 8 + media.len();
    media.extend_from_slice(b"AAC3");
    let stsd = atom(
        b"stsd",
        &[numbers(&[0, 1]), atom(b"mp4a", &[0; 28])].concat(),
    );
    let stsc = atom(b"stsc", &numbers(&[0, 2, 1, 2, 1, 2, 1, 1]));
    let stsz = atom(
        b"stsz",
        &if fixed {
            numbers(&[0, 4, 3])
        } else {
            numbers(&[0, 0, 3, 4, 4, 4])
        },
    );
    let offsets = if wide {
        atom(
            b"co64",
            &[
                numbers(&[0, 2]),
                (first as u64).to_be_bytes().to_vec(),
                (second as u64).to_be_bytes().to_vec(),
            ]
            .concat(),
        )
    } else {
        atom(b"stco", &numbers(&[0, 2, first as u32, second as u32]))
    };
    let stbl = atom(
        b"stbl",
        &[
            stsd,
            stsc,
            stsz,
            offsets,
            atom(b"stts", &numbers(&[0, 1, 3, 1024])),
        ]
        .concat(),
    );
    let audio = atom(
        b"trak",
        &[
            atom(
                b"edts",
                &atom(b"elst", &numbers(&[0, 1, 1234, 1024, 65536])),
            ),
            atom(
                b"mdia",
                &[
                    atom(b"hdlr", &[numbers(&[0, 0]), b"soun".to_vec()].concat()),
                    atom(b"minf", &stbl),
                ]
                .concat(),
            ),
        ]
        .concat(),
    );
    let video = atom(
        b"trak",
        &atom(
            b"mdia",
            &atom(b"hdlr", &[numbers(&[0, 0]), b"vide".to_vec()].concat()),
        ),
    );
    [
        ftyp,
        atom(b"mdat", &media),
        atom(
            b"moov",
            &[atom(b"mvhd", &numbers(&[0, 1, 2, 3])), audio, video].concat(),
        ),
    ]
    .concat()
}

#[test]
fn compacted_files_keep_exact_audio_chunks_timing_and_edit_lists() {
    for wide in [false, true] {
        for fixed in [false, true] {
            let original = fixture(wide, fixed);
            assert!(contains_video(&original));
            let output = extract(&original).unwrap().unwrap();
            assert!(!contains_video(&output));
            assert!(output.len() < original.len());
            assert_eq!(&output[output.len() - 12..], b"AAC1AAC2AAC3");
            for marker in [b"stts", b"elst"] {
                let original_at = original.windows(4).position(|s| s == marker).unwrap();
                let output_at = output.windows(4).position(|s| s == marker).unwrap();
                assert_eq!(
                    &original[original_at..original_at + 20],
                    &output[output_at..output_at + 20]
                );
            }
            let offset_at = output
                .windows(4)
                .position(|s| s == if wide { b"co64" } else { b"stco" })
                .unwrap()
                + 12;
            let start = if wide {
                u64::from_be_bytes(output[offset_at..offset_at + 8].try_into().unwrap()) as usize
            } else {
                u32::from_be_bytes(output[offset_at..offset_at + 4].try_into().unwrap()) as usize
            };
            assert_eq!(&output[start..start + 8], b"AAC1AAC2");
            assert!(extract(&output).unwrap().is_none());
        }
    }
}

#[test]
fn malformed_offsets_sizes_and_fragmented_files_are_never_rewritten() {
    let fixture = fixture(false, false);
    for at in [4, fixture.len() - 1, fixture.len() / 2] {
        let truncated = &fixture[..at];
        assert!(extract(truncated).ok().flatten().is_none());
    }
    let mut outside = fixture.clone();
    let offset = outside.windows(4).position(|b| b == b"stco").unwrap() + 12;
    outside[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(extract(&outside).is_err());
    let mut overlap = fixture.clone();
    let first = overlap[offset..offset + 4].to_vec();
    overlap[offset + 4..offset + 8].copy_from_slice(&first);
    assert!(extract(&overlap).is_err());
    let mut fragmented = fixture.clone();
    fragmented.extend_from_slice(&atom(b"moof", &[]));
    assert!(extract(&fragmented).unwrap().is_none());
    assert!(extract(b"not an MP4 file").unwrap().is_none());
}

#[test]
fn damaged_container_headers_do_not_panic() {
    let original = fixture(true, false);
    let mut seed = 0x71a5u32;
    for _ in 0..4000 {
        let mut damaged = original.clone();
        for _ in 0..4 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let at = seed as usize % damaged.len();
            damaged[at] ^= (seed >> 16) as u8;
        }
        let _ = contains_video(&damaged);
        let _ = extract(&damaged);
    }
}

#[test]
fn compaction_preserves_the_buffer_in_use_and_does_not_escape_the_cache() {
    let root = std::env::temp_dir().join(format!("vibefy-audio-only-{}", rand::random::<u64>()));
    let cache = spotifast::player::MatchCache::new(&root);
    let original = fixture(false, false);
    cache.save_audio("fixture", &original);
    let playing = cache.get_cached_audio("fixture").unwrap();
    let (before, after) = cache.compact_cached_audio("fixture").unwrap();
    assert!(after < before);
    assert_eq!(playing, original);
    assert!(!contains_video(&cache.get_cached_audio("fixture").unwrap()));
    assert!(cache.compact_cached_audio("../fixture").is_none());
    assert!(cache.compact_cached_audio("missing").is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn playback_does_not_wait_for_background_compaction() {
    let root =
        std::env::temp_dir().join(format!("vibefy-audio-background-{}", rand::random::<u64>()));
    let cache = spotifast::player::MatchCache::new(&root);
    let original = fixture(true, false);
    cache.save_audio("fixture", &original);
    assert_eq!(cache.get_cached_audio("fixture").unwrap(), original);
    tokio::time::timeout(std::time::Duration::from_secs(6), async {
        loop {
            if !contains_video(&cache.get_cached_audio("fixture").unwrap()) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn replaying_a_cached_direct_song_does_not_rewrite_recording_metadata() {
    use sha1::{Digest, Sha1};
    use spotifast::player::{LocalTrack, MatchCache, resolve_track_audio};
    let root = std::env::temp_dir().join(format!("vibefy-cache-replay-{}", rand::random::<u64>()));
    let cache = MatchCache::new(&root);
    let track = LocalTrack {
        uri: "vibefy:youtube:abcdefghijk".into(),
        title: "Fixture".into(),
        ..Default::default()
    };
    let key = format!("{:x}", Sha1::digest(track.uri.as_bytes()));
    cache.save_audio(&key, b"cached-audio");
    let helper = root.join("must-not-be-started");
    for replay in 0..2 {
        let before = std::fs::metadata(root.join("youtube_cache.json"))
            .and_then(|m| m.modified())
            .ok();
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        assert_eq!(
            resolve_track_audio(
                &helper,
                &cache,
                &track,
                &spotifast::settings::ProxyConfig::Off
            )
            .await
            .unwrap(),
            b"cached-audio"
        );
        if replay == 1 {
            assert_eq!(
                before,
                std::fs::metadata(root.join("youtube_cache.json"))
                    .and_then(|m| m.modified())
                    .ok()
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
