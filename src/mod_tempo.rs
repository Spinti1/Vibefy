//! Bounded local tempo estimate for mod animation. It never changes playback or lyrics timing.
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Estimate a beat interval from a 100 Hz onset envelope. Octave ambiguity is
/// unavoidable for some recordings; weak or silent signals return no estimate.
pub fn estimate_envelope(energy: &[f32], hz: f32) -> Option<f32> {
    if !hz.is_finite() || hz <= 0.0 || hz > 10000.0 || energy.len() < hz as usize * 8 {
        return None;
    }
    let mut previous = 0.0;
    let mut novelty = Vec::with_capacity(energy.len());
    for &value in energy {
        let value = if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        };
        novelty.push((value - previous).max(0.0));
        previous = previous * 0.8 + value * 0.2;
    }
    let power = novelty.iter().map(|x| x * x).sum::<f32>();
    if power < 0.00001 {
        return None;
    }
    // A stable series of attacks is stronger evidence than an autocorrelation
    // peak at half tempo (common when beat spacing falls between sample bins).
    let gate = (power / novelty.len() as f32).sqrt() * 1.5;
    let mut attacks = Vec::new();
    for i in 1..novelty.len() - 1 {
        if novelty[i] >= gate
            && novelty[i] >= novelty[i - 1]
            && novelty[i] > novelty[i + 1]
            && attacks
                .last()
                .is_none_or(|last: &usize| i - *last >= (hz * 0.2) as usize)
        {
            attacks.push(i);
        }
    }
    let gaps: Vec<f32> = attacks
        .windows(2)
        .map(|pair| (pair[1] - pair[0]) as f32)
        .filter(|gap| *gap >= hz * 0.3 && *gap <= hz * 1.1)
        .collect();
    if gaps.len() >= 8 && gaps.len() * 5 >= attacks.len().saturating_sub(1) * 4 {
        let mean = gaps.iter().sum::<f32>() / gaps.len() as f32;
        let deviation =
            (gaps.iter().map(|gap| (gap - mean).powi(2)).sum::<f32>() / gaps.len() as f32).sqrt();
        if deviation / mean < 0.08 {
            return Some(60.0 * hz / mean);
        }
    }
    let first = (hz * 60.0 / 200.0).floor() as usize;
    let last = (hz * 60.0 / 55.0).ceil() as usize;
    let correlation = |lag: usize| {
        let mut dot = 0.0;
        let mut left = 0.0;
        let mut right = 0.0;
        for i in lag..novelty.len() {
            let a = novelty[i];
            let b = novelty[i - lag];
            dot += a * b;
            left += a * a;
            right += b * b;
        }
        if left * right > 0.0000001 {
            dot / (left * right).sqrt()
        } else {
            0.0
        }
    };
    let mut scores = Vec::new();
    for lag in first.max(1)..=last {
        let bpm = 60.0 * hz / lag as f32;
        let raw = correlation(lag);
        // Prefer a common musical pulse when half/double-time peaks tie.
        let prior = if (80.0..=175.0).contains(&bpm) {
            1.0
        } else {
            0.88
        };
        scores.push((lag, raw, prior * (raw + 0.12 * correlation(lag * 2))));
    }
    let maximum = scores.iter().map(|s| s.2).max_by(f32::total_cmp)?;
    let &(lag, raw, _) = scores
        .iter()
        .enumerate()
        .find(|(index, s)| {
            s.2 >= maximum * 0.96
                && (*index == 0 || s.2 >= scores[*index - 1].2)
                && (*index + 1 == scores.len() || s.2 >= scores[*index + 1].2)
        })
        .map(|(_, s)| s)?;
    if raw < 0.18 {
        return None;
    }
    // Refine an integer-lag peak to avoid quantizing 128 BPM to 127.66.
    let at = scores.iter().position(|s| s.0 == lag)?;
    let shift = if at > 0 && at + 1 < scores.len() {
        let (left, center, right) = (scores[at - 1].1, raw, scores[at + 1].1);
        let curve = left - 2.0 * center + right;
        if curve.abs() > 0.00001 {
            (0.5 * (left - right) / curve).clamp(-0.5, 0.5)
        } else {
            0.0
        }
    } else {
        0.0
    };
    Some(60.0 * hz / (lag as f32 + shift))
}

/// Decode at most 90 seconds with the existing native decoder. Only a small
/// onset envelope is kept, on a background worker; no model or microphone.
pub fn estimate_file(path: &Path, cancel: &AtomicBool) -> Result<Option<f32>> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(None);
    }
    use symphonia::core::{
        audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
        meta::MetadataOptions, probe::Hint,
    };
    let file = std::fs::File::open(path).context("Tempo audio cache unavailable")?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("m4a");
    let mut format = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?
        .format;
    let track = format.default_track().context("No audio track")?;
    let id = track.id;
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
    let mut envelope = Vec::with_capacity(9000);
    let mut total = 0.0;
    let mut count = 0;
    let mut low = 0.0f32;
    while envelope.len() < 9000 && !cancel.load(Ordering::Relaxed) {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(_) => break,
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(audio) => audio,
            Err(_) => continue,
        };
        let spec = *decoded.spec();
        let channels = spec.channels.count();
        if channels == 0 {
            continue;
        }
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        samples.copy_interleaved_ref(decoded);
        let hop = (spec.rate as usize / 100).max(1);
        let alpha = 1.0 - (-std::f32::consts::TAU * 180.0 / spec.rate.max(1) as f32).exp();
        for frame in samples.samples().chunks_exact(channels) {
            let sample = frame.iter().sum::<f32>() / channels as f32;
            low += alpha * (sample - low);
            total += 0.7 * low * low + 0.3 * sample * sample;
            count += 1;
            if count >= hop {
                envelope.push((total / count as f32).sqrt());
                total = 0.0;
                count = 0;
                if envelope.len() >= 9000 {
                    break;
                }
            }
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return Ok(None);
    }
    Ok(estimate_envelope(&envelope, 100.0))
}
