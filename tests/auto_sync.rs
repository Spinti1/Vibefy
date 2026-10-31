use spotifast::auto_sync::{HeardWord, Mono16k, align, transcript, wav};
use spotifast::spicy_lyrics::{Calibration, Document, Line};

fn recording(offset: u32, stretch: f64) -> (Document, Vec<HeardWord>) {
    let phrases = [
        "Silver lantern lights the mountain",
        "Morning rivers carry distant echoes",
        "Golden windows catch the sunlight",
        "Winter gardens shelter sleeping roses",
        "Quiet footsteps cross the courtyard",
        "Summer thunder shakes the valley",
    ];
    let mut doc = Document::default();
    let mut heard = Vec::new();
    for (index, phrase) in phrases.into_iter().enumerate() {
        let time = 10_000 + index as u32 * 20_000;
        doc.lines.push(Line {
            text: phrase.into(),
            start_ms: Some(time),
            ..Default::default()
        });
        let start = (f64::from(time) * stretch).round() as u32 + offset;
        for (word, text) in phrase.to_lowercase().split_whitespace().enumerate() {
            heard.push(HeardWord {
                text: text.into(),
                at_ms: start + word as u32 * 400,
                probability: 0.95,
            });
        }
    }
    (doc, heard)
}

#[test]
fn recognized_phrases_find_an_intro_offset_without_silence_detection() {
    let (doc, heard) = recording(6400, 1.0);
    let result = align(&doc, &heard).unwrap();
    assert_eq!(result.matched_lines, 6);
    assert_eq!(result.points.len(), 1);
    assert_eq!(result.median_error_ms, 0);
    let cal = Calibration {
        points: result.points,
        ..Default::default()
    };
    assert_eq!(cal.lyric_position(16_400, 0), 10_000);
    assert_eq!(cal.audio_position(50_000, 0), 56_400);
}

#[test]
fn several_phrases_can_establish_gradual_drift() {
    let (doc, heard) = recording(2500, 1.01);
    let result = align(&doc, &heard).unwrap();
    assert_eq!(result.points.len(), 2);
    let cal = Calibration {
        points: result.points,
        ..Default::default()
    };
    assert_eq!(cal.lyric_position(63_100, 0), 60_000);
}

#[test]
fn sparse_unrelated_or_low_confidence_audio_does_not_change_timing() {
    let (doc, mut heard) = recording(2000, 1.0);
    assert!(align(&doc, &heard[..10]).is_err());
    for word in &mut heard {
        word.text = "unrelated".into();
    }
    assert!(align(&doc, &heard).is_err());
    let (_, mut heard) = recording(2000, 1.0);
    for word in &mut heard {
        word.probability = 0.1;
    }
    assert!(align(&doc, &heard).is_err());
    for word in &mut heard {
        word.probability = f32::NAN;
    }
    assert!(align(&doc, &heard).is_err());
}

#[test]
fn repeated_choruses_with_competing_offsets_are_refused() {
    let (mut doc, heard) = recording(2000, 1.0);
    doc.lines.truncate(3);
    let mut repeated = heard[..15].to_vec();
    repeated.extend(heard[..15].iter().cloned().map(|mut word| {
        word.at_ms += 100_000;
        word
    }));
    assert!(align(&doc, &repeated).is_err());
}

#[test]
fn a_single_bad_phrase_does_not_move_an_otherwise_consistent_offset() {
    let (doc, mut heard) = recording(2000, 1.0);
    for word in &mut heard[10..15] {
        word.at_ms += 4000;
    }
    let result = align(&doc, &heard).unwrap();
    assert_eq!(result.matched_lines, 5);
    assert_eq!(result.points[0].audio_ms - result.points[0].lyric_ms, 2000);
}

#[test]
fn invalid_or_out_of_order_timestamps_are_rejected() {
    let (doc, mut heard) = recording(2000, 1.0);
    heard.swap(0, 10);
    assert!(align(&doc, &heard).is_err());
}

#[test]
fn transcription_needs_real_word_times_and_confidence() {
    use serde_json::json;
    let json = json!({"transcription": [
        {"text":" Silver", "offsets":{"from":2400,"to":2700}, "tokens":[{"id":1500,"p":0.9}]},
        {"text":"[Music]", "offsets":{"from":2700,"to":3500}, "tokens":[{"id":1600,"p":0.9}]},
        {"text":"several words here", "offsets":{"from":3500,"to":4500}, "tokens":[{"id":1700,"p":0.9}]},
        {"text":"invented", "tokens":[{"id":1800,"p":0.9}]},
        {"text":"weak", "offsets":{"from":4600,"to":4900}, "tokens":[{"id":50364,"p":0.9}]}
    ]});
    let words = transcript(&json).unwrap();
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].text, "silver");
    assert_eq!(words[0].at_ms, 2400);
}

#[test]
fn native_resampling_keeps_duration_for_common_rates_and_bounds_samples() {
    for rate in [8000, 16_000, 44_100, 48_000, 96_000] {
        let mut mono = Mono16k::new(rate);
        for _ in 0..rate {
            mono.push(0.5);
        }
        assert!(mono.samples.len().abs_diff(16_000) <= 1);
        assert!(
            mono.samples
                .iter()
                .all(|sample| sample.abs_diff(16_384) <= 1)
        );
        let encoded = wav(&mono.samples);
        assert_eq!(&encoded[..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes(encoded[24..28].try_into().unwrap()),
            16_000
        );
        assert_eq!(encoded.len(), 44 + mono.samples.len() * 2);
    }
}

#[test]
fn installation_failures_are_actionable_and_old_settings_remain_readable() {
    assert!(
        spotifast::auto_sync::runtime_at(&std::env::temp_dir().join("vibefy-no-such-runtime"))
            .is_err()
    );
    let settings: spotifast::settings::Settings = serde_json::from_str("{}").unwrap();
    assert!(settings.spicy_lyrics_auto_sync);
    assert!(settings.spicy_lyrics_auto_results.is_empty());
}

#[test]
fn stale_and_cancelled_analysis_cannot_replace_a_new_request() {
    let mut state = spotifast::spicy_lyrics::State::default();
    state.auto_key = Some("recording-A".into());
    state.auto_generation = 7;
    state.auto_running = true;
    assert!(state.accepts_auto("recording-A", 7));
    assert!(!state.accepts_auto("recording-B", 7));
    assert!(!state.accepts_auto("recording-A", 6));
    state.auto_generation = 8;
    assert!(!state.accepts_auto("recording-A", 7));
    state.auto_running = false;
    assert!(!state.accepts_auto("recording-A", 8));
}

#[test]
fn duration_comparison_does_not_turn_an_extra_outro_into_tempo_drift() {
    use spotifast::auto_sync::{TimingComparison, align_timed};
    let (doc, heard) = recording(3200, 1.0);
    let comparison = TimingComparison {
        reference_ms: 180_000,
        source_ms: 192_000,
    };
    assert_eq!(comparison.delta_ms(), 12_000);
    assert!((comparison.ratio().unwrap() - 1.0666667).abs() < 0.00001);
    let found = align_timed(&doc, &heard, comparison).unwrap();
    assert_eq!(found.points.len(), 1);
    assert_eq!(found.points[0].audio_ms - found.points[0].lyric_ms, 3200);
    assert!(align_timed(&doc, &[], comparison).is_err());
}

#[test]
fn duration_prior_and_phrases_recover_a_speed_difference_above_two_percent() {
    use spotifast::auto_sync::{TimingComparison, align_timed};
    let (doc, heard) = recording(1500, 1.04);
    let found = align_timed(
        &doc,
        &heard,
        TimingComparison {
            reference_ms: 180_000,
            source_ms: 188_700,
        },
    )
    .unwrap();
    assert_eq!(found.points.len(), 2);
    let cal = Calibration {
        points: found.points,
        ..Default::default()
    };
    assert_eq!(cal.lyric_position(63_900, 0), 60_000);
}

#[test]
fn a_misheard_later_word_does_not_hide_a_recognized_verse_start() {
    let (doc, mut heard) = recording(2300, 1.0);
    for index in (4..heard.len()).step_by(5) {
        heard[index].text = "misheard".into();
    }
    let found = align(&doc, &heard).unwrap();
    assert_eq!(found.matched_lines, 6);
    assert_eq!(found.points[0].audio_ms - found.points[0].lyric_ms, 2300);
}

#[test]
fn several_verses_on_both_sides_can_confirm_a_local_intro_edit() {
    let (doc, mut heard) = recording(2000, 1.0);
    for word in &mut heard[15..] {
        word.at_ms += 4000;
    }
    let found = spotifast::auto_sync::align_timed(
        &doc,
        &heard,
        spotifast::auto_sync::TimingComparison {
            reference_ms: 180_000,
            source_ms: 184_000,
        },
    )
    .unwrap();
    assert!(
        found.points.len() >= 4,
        "expected local verse anchors: {:?}",
        found.points
    );
    let cal = Calibration {
        points: found.points,
        ..Default::default()
    };
    assert_eq!(cal.lyric_position(12_000, 0), 10_000);
    assert_eq!(cal.lyric_position(116_000, 0), 110_000);
}

#[test]
fn decoded_audio_waiting_in_the_queue_is_not_counted_as_heard() {
    assert_eq!(
        spotifast::audio_match::audible_position(25_000, 185),
        24_815
    );
    assert_eq!(spotifast::audio_match::audible_position(100, 200), 0);
}

#[test]
fn older_automatic_results_load_but_are_scheduled_for_revalidation() {
    let old: spotifast::auto_sync::Alignment =
        serde_json::from_str(r#"{"points":[],"matched_lines":5,"median_error_ms":100}"#).unwrap();
    assert_eq!(old.version, 0);
    assert_ne!(old.version, spotifast::auto_sync::ALIGNER_VERSION);
}

#[test]
fn equal_recording_lengths_do_not_confirm_an_unrelated_speed_fit() {
    let (doc, heard) = recording(1500, 1.06);
    let comparison = spotifast::auto_sync::TimingComparison {
        reference_ms: 180_000,
        source_ms: 180_000,
    };
    assert!(spotifast::auto_sync::align_timed(&doc, &heard, comparison).is_err());
}

#[tokio::test]
async fn cached_recognition_aligns_without_installed_runtime() {
    use sha1::{Digest, Sha1};
    let base = std::env::temp_dir().join(format!(
        "vibefy-recognition-cache-{}",
        rand::random::<u64>()
    ));
    let cache = base.join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    let audio = base.join("fixture.m4a");
    std::fs::write(&audio, b"cached audio identity").unwrap();
    let metadata = std::fs::metadata(&audio).unwrap();
    let mut digest = Sha1::new();
    digest.update(audio.file_name().unwrap().to_string_lossy().as_bytes());
    digest.update(metadata.len().to_le_bytes());
    digest.update(
        metadata
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_le_bytes(),
    );
    let path = cache.join(format!("recognition-base-v2-{:x}.json", digest.finalize()));
    let (doc, heard) = recording(2400, 1.0);
    std::fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({"duration_ms":180_000,"heard":heard})).unwrap(),
    )
    .unwrap();
    let result = spotifast::auto_sync::analyze_with_timing(
        audio,
        doc,
        cache,
        base.join("runtime-is-absent"),
        Default::default(),
    )
    .await
    .unwrap();
    assert_eq!(result.matched_lines, 6);
    assert_eq!(result.points[0].audio_ms - result.points[0].lyric_ms, 2400);
    std::fs::remove_dir_all(base).unwrap();
}
