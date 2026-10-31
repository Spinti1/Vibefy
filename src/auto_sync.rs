//! Local ASR-assisted timing. No microphone, remote transcription or lyric prompting.
//! A correction needs several distinct phrases agreeing on the same recording offset.

use crate::spicy_lyrics::{Calibration, Document, SyncPoint};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub const ALIGNER_VERSION: u8 = 2;

/// Metadata comparison is constant-time. A different total duration may be
/// an intro/outro, a speed change, or a cut, so it is a prior, never an anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TimingComparison {
    pub reference_ms: u32,
    pub source_ms: u32,
}

impl TimingComparison {
    pub fn delta_ms(self) -> i64 {
        i64::from(self.source_ms) - i64::from(self.reference_ms)
    }
    pub fn ratio(self) -> Option<f64> {
        (self.reference_ms >= 30_000 && self.source_ms >= 30_000)
            .then(|| f64::from(self.source_ms) / f64::from(self.reference_ms))
            .filter(|ratio| (0.90..=1.10).contains(ratio))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Alignment {
    pub points: Vec<SyncPoint>,
    pub matched_lines: usize,
    pub median_error_ms: u32,
    #[serde(default)]
    pub version: u8,
    #[serde(default)]
    pub comparison: TimingComparison,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeardWord {
    pub text: String,
    pub at_ms: u32,
    pub probability: f32,
}

fn words(text: &str) -> Vec<String> {
    crate::audio_match::normalized(&text.replace(['\'', '’'], ""))
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// --max-len 1 --split-on-word emits word segments. Do not invent positions
/// inside a multiword segment or treat missing token probabilities as confidence.
pub fn transcript(json: &Value) -> Result<Vec<HeardWord>> {
    let segments = json["transcription"]
        .as_array()
        .context("Missing local transcription")?;
    let mut result = Vec::new();
    for segment in segments.iter().take(20_000) {
        let text = segment["text"].as_str().unwrap_or("");
        if text.contains('[') || text.contains('<') || text.contains('♪') {
            continue;
        }
        let normalized = words(text);
        if normalized.len() != 1 {
            continue;
        }
        let Some(at_ms) = segment["offsets"]["from"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
        else {
            continue;
        };
        let Some(end_ms) = segment["offsets"]["to"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
        else {
            continue;
        };
        if end_ms <= at_ms || end_ms - at_ms > 8000 {
            continue;
        }
        let probabilities: Vec<_> = segment["tokens"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|token| token["id"].as_u64().is_some_and(|id| id < 50_257))
            .filter_map(|token| token["p"].as_f64())
            .filter(|p| p.is_finite() && (0.0..=1.0).contains(p))
            .collect();
        if probabilities.is_empty() {
            continue;
        }
        let probability = (probabilities.iter().sum::<f64>() / probabilities.len() as f64) as f32;
        if result
            .last()
            .is_some_and(|word: &HeardWord| word.at_ms > at_ms)
        {
            continue;
        }
        result.push(HeardWord {
            text: normalized[0].clone(),
            at_ms,
            probability,
        });
    }
    Ok(result)
}

#[derive(Clone)]
struct Match {
    phrase: String,
    lyric_ms: u32,
    audio_ms: u32,
}
impl Match {
    fn offset(&self) -> i64 {
        i64::from(self.audio_ms) - i64::from(self.lyric_ms)
    }
}

fn median(values: &mut [i64]) -> i64 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn phrase_matches(doc: &Document, heard: &[HeardWord]) -> Result<Vec<Match>> {
    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut by_word: std::collections::HashMap<&str, Vec<usize>> = Default::default();
    for (index, word) in heard.iter().enumerate() {
        by_word.entry(&word.text).or_default().push(index);
    }
    for line in doc.lines.iter().filter(|line| !line.background) {
        let Some(lyric_ms) = line.start_ms else {
            continue;
        };
        let text = words(&line.text);
        if text.len() < 3 {
            continue;
        }
        let phrase = text.join(" ");
        // A shorter prefix recovers a line whose later word was misheard.
        // All anchors still start on an independently recognized first word.
        for length in (3..=text.len().min(6)).rev() {
            let prefix = &text[..length];
            if prefix.iter().map(String::len).sum::<usize>() < 12 {
                continue;
            }
            for &index in by_word.get(prefix[0].as_str()).into_iter().flatten() {
                for count in length.saturating_sub(1)..=length + 1 {
                    if index + count > heard.len() || (length < 5 && count != length) {
                        continue;
                    }
                    let group = &heard[index..index + count];
                    if group.iter().any(|w| {
                        !w.probability.is_finite() || !(0.15..=1.0).contains(&w.probability)
                    }) || group.iter().map(|w| w.probability).sum::<f32>() / (count as f32)
                        < 0.60
                        || group
                            .windows(2)
                            .any(|pair| pair[1].at_ms - pair[0].at_ms > 4000)
                        || group[0].at_ms.abs_diff(lyric_ms) > 120_000
                    {
                        continue;
                    }
                    let expected: Vec<_> = prefix.iter().map(String::as_str).collect();
                    let actual: Vec<_> = group.iter().map(|w| w.text.as_str()).collect();
                    if !one_edit(&expected, &actual, length >= 5) {
                        continue;
                    }
                    if seen.insert((lyric_ms, group[0].at_ms)) {
                        candidates.push(Match {
                            phrase: phrase.clone(),
                            lyric_ms,
                            audio_ms: group[0].at_ms,
                        });
                    }
                    if candidates.len() > 1024 {
                        bail!("Too many ambiguous repeated lyric phrases");
                    }
                }
            }
        }
    }
    candidates.sort_by_key(|item| (item.lyric_ms, item.audio_ms));
    Ok(candidates)
}

fn one_edit(expected: &[&str], actual: &[&str], allow: bool) -> bool {
    if expected == actual {
        return true;
    }
    if !allow || expected.len().abs_diff(actual.len()) > 1 {
        return false;
    }
    let (mut left, mut right, mut edits) = (0, 0, 0);
    while left < expected.len() && right < actual.len() {
        if expected[left] == actual[right] {
            left += 1;
            right += 1;
            continue;
        }
        edits += 1;
        if edits > 1 {
            return false;
        }
        if expected.len() >= actual.len() {
            left += 1;
        }
        if expected.len() <= actual.len() {
            right += 1;
        }
    }
    edits + (expected.len() - left) + (actual.len() - right) <= 1
}

fn select_model(candidates: &[Match], slope: f64, intercept: f64, tolerance: f64) -> Vec<Match> {
    let error = |item: &Match| {
        (f64::from(item.audio_ms) - (slope * f64::from(item.lyric_ms) + intercept)).abs()
    };
    let mut selected: std::collections::HashMap<&str, &Match> = Default::default();
    for item in candidates.iter().filter(|item| error(item) <= tolerance) {
        let held = selected.entry(&item.phrase).or_insert(item);
        if error(item) < error(held) {
            *held = item;
        }
    }
    let mut selected: Vec<_> = selected.values().map(|item| (*item).clone()).collect();
    selected.sort_by_key(|item| item.lyric_ms);
    if selected
        .windows(2)
        .any(|pair| pair[0].audio_ms >= pair[1].audio_ms || pair[0].lyric_ms >= pair[1].lyric_ms)
    {
        return Vec::new();
    }
    selected
}

fn model_error(items: &[Match], slope: f64, intercept: f64) -> u32 {
    if items.is_empty() {
        return u32::MAX;
    }
    median(
        &mut items
            .iter()
            .map(|m| {
                (f64::from(m.audio_ms) - slope * f64::from(m.lyric_ms) - intercept)
                    .abs()
                    .round() as i64
            })
            .collect::<Vec<_>>(),
    ) as u32
}

fn fit_recording(
    candidates: &[Match],
    comparison: TimingComparison,
) -> Result<(Vec<Match>, f64, f64, u32)> {
    let mut models = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |slope: f64, intercept: f64| {
        let duration_supported = if slope == 1.0 {
            true
        } else if comparison.ratio().is_some() {
            // Phrase timing alone can fit a repeated chorus at the wrong
            // speed. A whole-song tempo estimate must also reach the real
            // source ending within a modest outro margin after its intro.
            (slope * f64::from(comparison.reference_ms) + intercept
                - f64::from(comparison.source_ms))
            .abs()
                <= 3000.0_f64.max(f64::from(comparison.reference_ms) * 0.01)
        } else {
            (0.98..=1.02).contains(&slope)
        };
        if (0.90..=1.10).contains(&slope)
            && duration_supported
            && seen.insert((
                (slope * 2000.0).round() as i64,
                (intercept / 100.0).round() as i64,
            ))
        {
            models.push((slope, intercept));
        }
    };
    for item in candidates {
        add(1.0, item.offset() as f64);
        if let Some(ratio) = comparison.ratio() {
            add(
                ratio,
                f64::from(item.audio_ms) - ratio * f64::from(item.lyric_ms),
            );
        }
    }
    // Bounded RANSAC hypotheses cover speed differences without presuming
    // that extra duration belongs to the intro or the outro.
    let step = candidates.len().div_ceil(48).max(1);
    for (index, left) in candidates.iter().enumerate().step_by(step) {
        for right in candidates.iter().skip(index + 1).step_by(step) {
            if right.lyric_ms.saturating_sub(left.lyric_ms) < 30_000
                || right.audio_ms <= left.audio_ms
                || right.phrase == left.phrase
            {
                continue;
            }
            let slope = f64::from(right.audio_ms - left.audio_ms)
                / f64::from(right.lyric_ms - left.lyric_ms);
            add(
                slope,
                f64::from(left.audio_ms) - slope * f64::from(left.lyric_ms),
            );
        }
    }
    let mut fits = Vec::new();
    for (slope, intercept) in models {
        let selected = select_model(candidates, slope, intercept, 1000.0);
        let span = selected
            .last()
            .zip(selected.first())
            .map_or(0, |(last, first)| last.lyric_ms - first.lyric_ms);
        if selected.len() < 3
            || span < 12_000
            || (slope != 1.0 && (selected.len() < 5 || span < 30_000))
        {
            continue;
        }
        let intercept = if slope == 1.0 {
            median(&mut selected.iter().map(Match::offset).collect::<Vec<_>>()) as f64
        } else {
            intercept
        };
        let error = model_error(&selected, slope, intercept);
        if error <= 450 {
            fits.push((selected, slope, intercept, error));
        }
    }
    fits.sort_by_key(|(items, _, _, error)| (std::cmp::Reverse(items.len()), *error));
    let Some(mut best) = fits.first().cloned() else {
        bail!("Not enough consistent lyric phrases were recognized. Timing was kept.");
    };
    // Prefer a constant offset when fitting a tempo brings little evidence.
    if let Some(constant) = fits.iter().find(|(items, slope, _, error)| {
        *slope == 1.0 && items.len() == best.0.len() && *error <= best.3.saturating_add(100)
    }) {
        best = constant.clone();
    }
    for other in &fits {
        if other.0.len() * 5 < best.0.len() * 4 || other.3 > best.3.saturating_add(200) {
            continue;
        }
        let common = best
            .0
            .iter()
            .filter(|item| other.0.iter().any(|second| item.phrase == second.phrase))
            .count();
        let time = f64::from(best.0[0].lyric_ms);
        if common * 5 >= best.0.len() * 4
            && ((other.1 - best.1) * time + other.2 - best.2).abs() > 3000.0
        {
            bail!("Repeated lyrics give competing timing matches. Timing was kept.");
        }
    }
    Ok(best)
}

fn local_sections(
    candidates: &[Match],
    slope: f64,
    intercept: f64,
    comparison: TimingComparison,
) -> Option<Vec<SyncPoint>> {
    let tolerance = (comparison.delta_ms().unsigned_abs() as f64 + 2000.0).clamp(5000.0, 15_000.0);
    let mut selected = select_model(candidates, slope, intercept, tolerance);
    if selected.len() < 6 {
        return None;
    }
    selected.sort_by_key(|item| item.lyric_ms);
    let residual = |item: &Match| {
        (f64::from(item.audio_ms) - slope * f64::from(item.lyric_ms) - intercept).round() as i64
    };
    let overall = median(&mut selected.iter().map(residual).collect::<Vec<_>>());
    let mut deviations: Vec<_> = selected
        .iter()
        .map(|item| (residual(item) - overall).abs())
        .collect();
    deviations.sort_unstable();
    let base_error = deviations[deviations.len() * 3 / 4];
    let mut best = None;
    let mut best_error = base_error;
    for split in 3..=selected.len() - 3 {
        let left = &selected[..split];
        let right = &selected[split..];
        if left.last()?.lyric_ms - left[0].lyric_ms < 8000
            || right.last()?.lyric_ms - right[0].lyric_ms < 8000
        {
            continue;
        }
        let l = median(&mut left.iter().map(residual).collect::<Vec<_>>());
        let r = median(&mut right.iter().map(residual).collect::<Vec<_>>());
        if (l - r).abs() < 900 {
            continue;
        }
        let errors: Vec<_> = selected
            .iter()
            .enumerate()
            .map(|(i, item)| (residual(item) - if i < split { l } else { r }).abs())
            .collect();
        let error = median(&mut errors.clone());
        if error > 300 || error >= best_error || errors.iter().any(|e| *e > 900) {
            continue;
        }
        let points: Vec<_> = [
            (left[0].lyric_ms, l),
            (left.last()?.lyric_ms, l),
            (right[0].lyric_ms, r),
            (right.last()?.lyric_ms, r),
        ]
        .into_iter()
        .map(|(lyric_ms, adjustment)| SyncPoint {
            lyric_ms,
            audio_ms: (slope * f64::from(lyric_ms) + intercept + adjustment as f64)
                .round()
                .max(0.0) as u32,
        })
        .collect();
        if points
            .windows(2)
            .any(|pair| pair[0].audio_ms >= pair[1].audio_ms)
        {
            continue;
        }
        best = Some(points);
        best_error = error;
    }
    best
}

/// Use independently recognized phrases, never silence detection or a
/// duration ratio. Ambiguous repeated choruses and weak matches are refused.
pub fn align(doc: &Document, heard: &[HeardWord]) -> Result<Alignment> {
    align_timed(doc, heard, TimingComparison::default())
}

pub fn align_timed(
    doc: &Document,
    heard: &[HeardWord],
    comparison: TimingComparison,
) -> Result<Alignment> {
    if heard.windows(2).any(|pair| pair[0].at_ms > pair[1].at_ms) {
        bail!("Local transcription timestamps are not ordered");
    }
    if heard.len() > 20_000 || doc.lines.len() > 3000 {
        bail!("Recording is too large for auto sync");
    }
    let candidates = phrase_matches(doc, heard)?;
    let (best, slope, intercept, error) = fit_recording(&candidates, comparison)?;
    let mut points = if (slope - 1.0).abs()
        * f64::from(best.last().unwrap().lyric_ms - best[0].lyric_ms)
        >= 400.0
    {
        [best[0].lyric_ms, best.last().unwrap().lyric_ms]
            .into_iter()
            .map(|lyric_ms| SyncPoint {
                lyric_ms,
                audio_ms: (f64::from(lyric_ms) * slope + intercept).max(0.0).round() as u32,
            })
            .collect()
    } else {
        let offset = median(&mut best.iter().map(Match::offset).collect::<Vec<_>>());
        vec![SyncPoint {
            lyric_ms: best[0].lyric_ms,
            audio_ms: (i64::from(best[0].lyric_ms) + offset).max(0) as u32,
        }]
    };
    // Recheck individual verse starts near the established model. Accept a
    // local change only when at least three consecutive, distinct phrases
    // on either side support it. Isolated recognition errors cannot add anchors.
    if let Some(local) = local_sections(&candidates, slope, intercept, comparison) {
        points = local;
    }
    let mut check = Calibration::default();
    for point in &points {
        if !check.add_point(*point) {
            bail!("Automatic timing is not ordered");
        }
    }
    Ok(Alignment {
        points,
        matched_lines: best.len(),
        median_error_ms: error,
        version: ALIGNER_VERSION,
        comparison,
    })
}

/// Native decoder output is averaged to mono and resampled to 16 kHz for ASR.
/// This signal never enters the listening output or equalizer.
pub fn decode_wav(path: &Path) -> Result<Vec<u8>> {
    use symphonia::core::{
        audio::SampleBuffer,
        codecs::{CODEC_TYPE_NULL, DecoderOptions},
        formats::FormatOptions,
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
    };
    let file = std::fs::File::open(path)
        .context("Cached audio is unavailable; wait until playback has loaded")?;
    if file.metadata()?.len() > 128 * 1024 * 1024 {
        bail!("Cached recording exceeds 128 MB");
    }
    let input = MediaSourceStream::new(Box::new(file), Default::default());
    let mut format = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            input,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?
        .format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No decodable audio")?;
    let id = track.id;
    let rate = track
        .codec_params
        .sample_rate
        .context("Unknown audio sample rate")?;
    if !(8000..=192_000).contains(&rate) {
        bail!("Unsupported audio sample rate");
    }
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
    let mut pcm = Mono16k::new(rate);
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(error) => return Err(error.into()),
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(buffer) => buffer,
            Err(error) => {
                return Err(error).context("Cannot reliably decode recording for auto sync");
            }
        };
        let channels = decoded.spec().channels.count();
        if channels == 0 || decoded.spec().rate != rate {
            bail!("Audio format changed during auto sync");
        }
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        samples.copy_interleaved_ref(decoded);
        for frame in samples.samples().chunks_exact(channels) {
            pcm.push(frame.iter().sum::<f32>() / channels as f32);
        }
        if pcm.samples.len() > 16_000 * 15 * 60 {
            bail!("Auto sync supports recordings up to 15 minutes");
        }
    }
    if pcm.samples.len() < 16_000 {
        bail!("Recording is too short for auto sync");
    }
    Ok(wav(&pcm.samples))
}

pub struct Mono16k {
    rate: u32,
    frames: u64,
    previous: f32,
    pub samples: Vec<i16>,
}
impl Mono16k {
    pub fn new(rate: u32) -> Self {
        Self {
            rate: rate.max(1),
            frames: 0,
            previous: 0.0,
            samples: Vec::new(),
        }
    }
    pub fn push(&mut self, value: f32) {
        let value = if value.is_finite() {
            value.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        if self.frames == 0 {
            self.previous = value;
        }
        let end = self.frames * 16_000;
        while self.samples.len() as u64 * u64::from(self.rate) <= end {
            let at = self.samples.len() as u64 * u64::from(self.rate);
            let fraction = if self.frames == 0 {
                1.0
            } else {
                (at.saturating_sub(end - 16_000)) as f32 / 16_000.0
            };
            let sample = self.previous + (value - self.previous) * fraction;
            self.samples
                .push((sample * f32::from(i16::MAX)).round() as i16);
        }
        self.frames += 1;
        self.previous = value;
    }
}

pub fn wav(samples: &[i16]) -> Vec<u8> {
    let bytes = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + bytes as usize);
    wav.extend(b"RIFF");
    wav.extend((36 + bytes).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(16_000u32.to_le_bytes());
    wav.extend(32_000u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(bytes.to_le_bytes());
    for sample in samples {
        wav.extend(sample.to_le_bytes());
    }
    wav
}

pub fn runtime_at(base: &Path) -> Result<(PathBuf, PathBuf)> {
    let name = if cfg!(windows) {
        "whisper-cli.exe"
    } else {
        "whisper-cli"
    };
    let binary = [base.join(name), base.join("Release").join(name)]
        .into_iter()
        .find(|p| p.is_file())
        .context("Local auto-sync engine is missing. Keep the auto-sync folder beside Vibefy.")?;
    let model = base.join("ggml-base.bin");
    if std::fs::metadata(&model).map_or(true, |meta| meta.len() != 147_951_465) {
        bail!("Local multilingual model is missing or incomplete");
    }
    Ok((binary, model))
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub async fn analyze(audio: PathBuf, doc: Document, cache: PathBuf) -> Result<Alignment> {
    analyze_timed(audio, doc, cache, TimingComparison::default()).await
}

pub async fn analyze_timed(
    audio: PathBuf,
    doc: Document,
    cache: PathBuf,
    comparison: TimingComparison,
) -> Result<Alignment> {
    let base = std::env::current_exe()?
        .parent()
        .context("Unknown application folder")?
        .join("auto-sync");
    analyze_with_timing(audio, doc, cache, base, comparison).await
}

/// Explicit runtime location for portable packaging checks and diagnostics.
pub async fn analyze_with_runtime(
    audio: PathBuf,
    doc: Document,
    cache: PathBuf,
    base: PathBuf,
) -> Result<Alignment> {
    analyze_with_timing(audio, doc, cache, base, TimingComparison::default()).await
}

#[derive(Serialize, Deserialize)]
struct RecognitionCache {
    duration_ms: u32,
    heard: Vec<HeardWord>,
}

pub async fn analyze_with_timing(
    audio: PathBuf,
    doc: Document,
    cache: PathBuf,
    base: PathBuf,
    mut comparison: TimingComparison,
) -> Result<Alignment> {
    use sha1::{Digest, Sha1};
    let metadata = tokio::fs::metadata(&audio)
        .await
        .context("Cached audio unavailable; wait for playback to load")?;
    let mut digest = Sha1::new();
    digest.update(
        audio
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .as_bytes(),
    );
    digest.update(metadata.len().to_le_bytes());
    digest.update(
        metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
            .to_le_bytes(),
    );
    let recognition_path = cache.join(format!("recognition-base-v2-{:x}.json", digest.finalize()));
    if let Ok(meta) = tokio::fs::metadata(&recognition_path).await
        && meta.len() <= 16 * 1024 * 1024
        && let Ok(bytes) = tokio::fs::read(&recognition_path).await
        && let Ok(recognized) = serde_json::from_slice::<RecognitionCache>(&bytes)
    {
        comparison.source_ms = recognized.duration_ms;
        return tokio::task::spawn_blocking(move || {
            align_timed(&doc, &recognized.heard, comparison)
        })
        .await?;
    }
    let (binary, model) = tokio::task::spawn_blocking(move || runtime_at(&base)).await??;
    // One recognizer at a time, including launches from separate UI requests.
    static GATE: std::sync::OnceLock<std::sync::Arc<tokio::sync::Semaphore>> =
        std::sync::OnceLock::new();
    let _permit = GATE
        .get_or_init(|| std::sync::Arc::new(tokio::sync::Semaphore::new(1)))
        .clone()
        .acquire_owned()
        .await?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let folder = cache.join(format!("auto-sync-{}-{stamp}", std::process::id()));
    tokio::fs::create_dir_all(&folder).await?;
    let temporary = Temporary(folder.clone());
    let wav_path = folder.join("input.wav");
    let wav_path_worker = wav_path.clone();
    let measured_ms = tokio::task::spawn_blocking(move || -> Result<u32> {
        let encoded = decode_wav(&audio)?;
        let duration_ms = ((encoded.len().saturating_sub(44) as u64 * 1000) / 32_000) as u32;
        std::fs::write(wav_path_worker, encoded)?;
        Ok(duration_ms)
    })
    .await??;
    comparison.source_ms = measured_ms;
    let output = folder.join("recognized");
    let threads = std::thread::available_parallelism()
        .map_or(2, |count| count.get().saturating_sub(2).clamp(1, 6));
    let mut command = tokio::process::Command::new(binary);
    command
        .args(["-ng", "-l", "auto", "-ml", "1", "-sow", "-ojf", "-np", "-t"])
        .arg(threads.to_string())
        .arg("-m")
        .arg(model)
        .arg("-f")
        .arg(&wav_path)
        .arg("-of")
        .arg(&output)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command
        .spawn()
        .context("Cannot start local auto-sync engine")?;
    let status = match tokio::time::timeout(Duration::from_secs(600), child.wait()).await {
        Ok(status) => status?,
        Err(_) => {
            let _ = child.kill().await;
            bail!("Local auto sync timed out; existing timing was kept");
        }
    };
    if !status.success() {
        bail!("Local recognizer could not analyze this recording. Existing timing was kept.");
    }
    let json_path = output.with_extension("json");
    if tokio::fs::metadata(&json_path).await?.len() > 16 * 1024 * 1024 {
        bail!("Local transcription is too large");
    }
    let data = tokio::fs::read(json_path).await?;
    let recognized = tokio::task::spawn_blocking(move || -> Result<RecognitionCache> {
        Ok(RecognitionCache {
            duration_ms: measured_ms,
            heard: transcript(&serde_json::from_slice(&data)?)?,
        })
    })
    .await??;
    // Reuse recognition across lyric refreshes/retries, including unsuccessful
    // alignment. A failed text match must not start another expensive ASR pass.
    let bytes = serde_json::to_vec(&recognized)?;
    let part = recognition_path.with_extension("part");
    tokio::fs::write(&part, bytes).await?;
    let _ = tokio::fs::remove_file(&recognition_path).await;
    tokio::fs::rename(&part, &recognition_path).await?;
    let result =
        tokio::task::spawn_blocking(move || align_timed(&doc, &recognized.heard, comparison))
            .await?;
    drop(temporary);
    result
}
