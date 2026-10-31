use spotifast::audio_match::{SampleClock, clean_title, compatible_version, rank};
use spotifast::spicy_lyrics::{
    Appearance, Calibration, Document, Line, SyncPoint, calibration_key,
};

#[test]
fn the_audio_clock_keeps_fractional_time_over_ten_minutes() {
    let mut clock = SampleClock::new(0, 44_100);
    let mut frames = 0u64;
    for _ in 0..25_839 {
        frames += 1024;
        let position = clock.advance(1024);
        assert_eq!(u64::from(position), frames * 1000 / 44_100);
    }
    let remainder = 44_100 * 600 - frames;
    assert_eq!(clock.advance(remainder as usize), 600_000);
    clock.seek(1234);
    assert_eq!(clock.advance(44_100), 2234);
}

#[test]
fn topic_audio_beats_a_video_with_an_intro() {
    let audio = rank(
        "Artist - Song",
        "Artist - Topic",
        Some(240.0),
        "Artist",
        "Song",
        "Song",
        240,
    );
    let video = rank(
        "Artist - Song (Official Video)",
        "ArtistVEVO",
        Some(248.0),
        "Artist",
        "Song",
        "Song",
        240,
    );
    assert!(audio > video);
    assert!(audio >= 360);
}

#[test]
fn wrong_artists_edits_and_lengths_are_not_played_as_the_album_recording() {
    for (title, channel, duration) in [
        ("Artist - Song live", "Artist", 240.0),
        ("Artist - Song (Cover)", "Artist", 240.0),
        ("Artist - Song slowed", "Artist", 240.0),
        ("Artist - Song", "Artist", 270.0),
        ("Other - Song", "Other - Topic", 240.0),
    ] {
        assert!(
            rank(
                title,
                channel,
                Some(duration),
                "Artist",
                "Song",
                "Song",
                240
            ) < 0,
            "{title}"
        );
    }
    assert!(!compatible_version("Song - Live", "Artist - Song"));
    assert!(compatible_version(
        "Drivers License",
        "Olivia Rodrigo - Drivers License"
    ));
}

#[test]
fn title_cleanup_keeps_requested_versions_and_unicode_boundaries() {
    assert_eq!(clean_title("İİ Song (feat. Someone)"), "İİ Song");
    assert_eq!(clean_title("Song - Live"), "Song - Live");
    assert_eq!(clean_title("Song - Radio Edit"), "Song - Radio Edit");
    assert_eq!(clean_title("Șoare - Remastered 2020"), "Șoare");
}

#[test]
fn one_anchor_fixes_an_intro_without_stretching_the_song() {
    let mut calibration = Calibration::default();
    assert!(calibration.add_point(SyncPoint {
        audio_ms: 12_000,
        lyric_ms: 10_000
    }));
    assert_eq!(calibration.lyric_position(32_000, 0), 30_000);
    assert_eq!(calibration.audio_position(30_000, 0), 32_000);
    assert_eq!(calibration.lyric_position(32_000, 150), 30_150);
}

#[test]
fn multiple_anchors_correct_drift_and_local_edit_offsets_reversibly() {
    let mut calibration = Calibration::default();
    for (audio_ms, lyric_ms) in [(12_000, 10_000), (112_000, 109_000), (162_000, 159_000)] {
        assert!(calibration.add_point(SyncPoint { audio_ms, lyric_ms }));
    }
    assert_eq!(calibration.lyric_position(62_000, 0), 59_500);
    for time in (12_000..=200_000).step_by(137) {
        let lyric = calibration.lyric_position(time, 0);
        assert!(calibration.audio_position(lyric, 0).abs_diff(time) <= 1);
    }
    assert!(!calibration.add_point(SyncPoint {
        audio_ms: 50_000,
        lyric_ms: 200_000
    }));
}

#[test]
fn damaged_or_duplicate_saved_anchors_do_not_break_playback() {
    let calibration = Calibration {
        offset_ms: 0,
        points: vec![
            SyncPoint {
                audio_ms: 1000,
                lyric_ms: 1000,
            },
            SyncPoint {
                audio_ms: 1000,
                lyric_ms: 2000,
            },
        ],
    };
    assert_eq!(calibration.lyric_position(1500, 0), 1500);
}

#[test]
fn timing_is_bound_to_the_audio_and_the_lyrics_recording() {
    let mut doc = Document {
        provider: "Test".into(),
        lines: vec![Line {
            text: "First line".into(),
            start_ms: Some(1000),
            ..Default::default()
        }],
        ..Default::default()
    };
    let key = calibration_key("spotify:track:one", "youtube-A", Some(&doc));
    assert_ne!(
        key,
        calibration_key("spotify:track:one", "youtube-B", Some(&doc))
    );
    doc.lines[0].start_ms = Some(2000);
    assert_ne!(
        key,
        calibration_key("spotify:track:one", "youtube-A", Some(&doc))
    );
}

#[test]
fn appearance_and_calibration_are_backward_compatible_and_serializable() {
    let mut appearance: Appearance = serde_json::from_str("{}").unwrap();
    appearance.font_size = f32::NAN;
    appearance.fps = 0;
    appearance.normalize();
    assert_eq!(appearance.font_size, 38.0);
    assert_eq!(appearance.fps, 15);
    assert!(serde_json::to_string(&appearance).is_ok());
    let calibration: Calibration = serde_json::from_str("{}").unwrap();
    assert_eq!(calibration.lyric_position(1500, 0), 1500);
}

#[test]
fn old_cached_matches_are_marked_for_revalidation() {
    let cached: spotifast::player::CachedMatch =
        serde_json::from_str(r#"{"youtube_id":"old","title":"Artist - Song","duration_secs":240}"#)
            .unwrap();
    assert_eq!(cached.matcher_version, 0);
}

#[test]
fn seeking_uses_container_time_instead_of_assuming_sample_timestamps() {
    use symphonia::core::{
        codecs::CodecParameters,
        errors::{Error, Result},
        formats::{Cue, FormatOptions, FormatReader, Packet, SeekMode, SeekTo, SeekedTo, Track},
        io::MediaSourceStream,
        meta::{Metadata, MetadataLog},
        units::TimeBase,
    };
    // A demuxer with a millisecond timebase but 48 kHz audio, landing on
    // whole-second packet boundaries. Real containers can use either unit.
    struct MillisecondContainer {
        tracks: Vec<Track>,
        metadata: MetadataLog,
    }
    impl FormatReader for MillisecondContainer {
        fn try_new(_: MediaSourceStream, _: &FormatOptions) -> Result<Self> {
            unreachable!()
        }
        fn cues(&self) -> &[Cue] {
            &[]
        }
        fn metadata(&mut self) -> Metadata<'_> {
            self.metadata.metadata()
        }
        fn tracks(&self) -> &[Track] {
            &self.tracks
        }
        fn next_packet(&mut self) -> Result<Packet> {
            unreachable!()
        }
        fn into_inner(self: Box<Self>) -> MediaSourceStream {
            unreachable!()
        }
        fn seek(&mut self, mode: SeekMode, to: SeekTo) -> Result<SeekedTo> {
            assert_eq!(mode, SeekMode::Accurate);
            let SeekTo::Time {
                time,
                track_id: Some(7),
            } = to
            else {
                panic!("Seeking must use regular time units and the selected track");
            };
            let required = time.seconds * 1000 + (time.frac * 1000.0).round() as u64;
            if required > 3000 {
                return Err(Error::Unsupported("outside recording"));
            }
            Ok(SeekedTo {
                track_id: 7,
                required_ts: required,
                actual_ts: required / 1000 * 1000,
            })
        }
    }
    let mut params = CodecParameters::new();
    params
        .with_sample_rate(48_000)
        .with_time_base(TimeBase::new(1, 1000));
    let mut container = MillisecondContainer {
        tracks: vec![Track::new(7, params)],
        metadata: MetadataLog::default(),
    };
    for (target, landed) in [(1234, 1000), (2500, 2000), (700, 0)] {
        assert_eq!(
            spotifast::audio_match::seek_recording(&mut container, 7, target).unwrap(),
            landed
        );
    }
    assert!(spotifast::audio_match::seek_recording(&mut container, 7, 999_000).is_err());
}

#[test]
fn cached_lyrics_from_the_previous_version_still_load() {
    let line: Line = serde_json::from_str(
        r#"{"text":"Old line","start_ms":1000,"end_ms":2000,"words":[],"opposite":false,"background":false}"#,
    ).unwrap();
    assert!(line.translation.is_none());
    assert!(line.romanization.is_none());
}
