//! Native lyrics data and provider adapters. The UI is implemented in egui;
//! no Spicetify runtime or extension code is embedded in the application.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::lyrics::{Lyrics, Query};
use crate::model::Loadable;

const SPICY_API: &str = "https://api.spicylyrics.org/v1/lyrics";
const MORE_API: &str = "https://lyrics.kamiloo13.me/api/get";
const TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Background {
    #[default]
    BlurredCover,
    AnimatedColors,
    CoverArt,
    CoverColors,
    Solid,
    Theme,
    CustomImage,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    #[default]
    Automatic,
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontStyle {
    #[default]
    Bold,
    Regular,
    Monospace,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub background: Background,
    pub blur: f32,
    pub darkness: f32,
    pub color: [u8; 3],
    pub custom_image: Option<String>,
    pub font_size: f32,
    pub font_style: FontStyle,
    pub line_gap: f32,
    pub alignment: TextAlign,
    pub show_cover: bool,
    pub simple_mode: bool,
    pub minimal_mode: bool,
    pub hover_background: bool,
    pub show_backing_vocals: bool,
    pub show_translation: bool,
    pub show_romanization: bool,
    pub smooth_scroll: bool,
    pub early_scroll_ms: u32,
    pub follow_position: f32,
    pub fps: u16,
    pub limit_fps: bool,
    pub use_accent: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            background: Background::BlurredCover,
            blur: 18.0,
            darkness: 0.55,
            color: [24, 26, 36],
            custom_image: None,
            font_size: 38.0,
            font_style: FontStyle::Bold,
            line_gap: 26.0,
            alignment: TextAlign::Automatic,
            show_cover: true,
            simple_mode: false,
            minimal_mode: false,
            hover_background: true,
            show_backing_vocals: true,
            show_translation: false,
            show_romanization: false,
            smooth_scroll: true,
            early_scroll_ms: 0,
            follow_position: 0.25,
            fps: 30,
            limit_fps: true,
            use_accent: true,
        }
    }
}

impl Appearance {
    pub fn normalize(&mut self) {
        let finite = |value: f32, fallback: f32, min: f32, max: f32| {
            if value.is_finite() {
                value.clamp(min, max)
            } else {
                fallback
            }
        };
        self.blur = finite(self.blur, 18.0, 0.0, 48.0);
        self.darkness = finite(self.darkness, 0.55, 0.0, 0.95);
        self.font_size = finite(self.font_size, 38.0, 20.0, 64.0);
        self.line_gap = finite(self.line_gap, 26.0, 4.0, 48.0);
        self.follow_position = finite(self.follow_position, 0.25, 0.15, 0.65);
        self.fps = self.fps.clamp(15, 240);
        self.early_scroll_ms = self.early_scroll_ms.min(800);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncPoint {
    pub audio_ms: u32,
    pub lyric_ms: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Calibration {
    pub offset_ms: i32,
    pub points: Vec<SyncPoint>,
}

impl Calibration {
    pub fn add_point(&mut self, point: SyncPoint) -> bool {
        let mut points = self.points.clone();
        points.retain(|held| held.audio_ms.abs_diff(point.audio_ms) > 250);
        points.push(point);
        points.sort_by_key(|point| point.audio_ms);
        if points.len() > 32
            || points
                .windows(2)
                .any(|pair| pair[0].lyric_ms >= pair[1].lyric_ms)
        {
            return false;
        }
        self.points = points;
        true
    }

    fn map(&self, time: u32, reverse: bool) -> u32 {
        let axes = |point: &SyncPoint| {
            if reverse {
                (point.lyric_ms, point.audio_ms)
            } else {
                (point.audio_ms, point.lyric_ms)
            }
        };
        let points: Vec<_> = self.points.iter().map(axes).collect();
        if points.len() > 32
            || points
                .windows(2)
                .any(|pair| pair[0].0 >= pair[1].0 || pair[0].1 >= pair[1].1)
        {
            return time;
        }
        let mapped = match points.as_slice() {
            [] => i64::from(time),
            [(x, y)] => i64::from(time) + i64::from(*y) - i64::from(*x),
            _ => {
                let next = points.partition_point(|(x, _)| *x <= time);
                // Outside known anchors keep a constant offset. Extrapolating
                // a fitted tempo into an intro/outro invents evidence.
                if next == 0 {
                    i64::from(time) + i64::from(points[0].1) - i64::from(points[0].0)
                } else if next == points.len() {
                    let (x, y) = points[points.len() - 1];
                    i64::from(time) + i64::from(y) - i64::from(x)
                } else {
                    let (x0, y0) = points[next - 1];
                    let (x1, y1) = points[next];
                    i64::from(y0)
                        + ((f64::from(time - x0) / f64::from(x1 - x0)) * f64::from(y1 - y0)).round()
                            as i64
                }
            }
        };
        mapped.clamp(0, i64::from(u32::MAX)) as u32
    }

    pub fn lyric_position(&self, audio_ms: u32, global_offset_ms: i32) -> u32 {
        position_with_offset(
            self.map(audio_ms, false),
            global_offset_ms.saturating_add(self.offset_ms),
        )
    }

    pub fn audio_position(&self, lyric_ms: u32, global_offset_ms: i32) -> u32 {
        self.map(
            position_with_offset(
                lyric_ms,
                global_offset_ms
                    .saturating_add(self.offset_ms)
                    .saturating_neg(),
            ),
            true,
        )
    }
}

/// Keep corrections separate for every source recording and lyrics file.
pub fn calibration_key(uri: &str, source_id: &str, document: Option<&Document>) -> String {
    use sha1::{Digest, Sha1};
    let mut digest = Sha1::new();
    if let Some(doc) = document {
        digest.update(doc.provider.as_bytes());
        for line in &doc.lines {
            digest.update(line.text.as_bytes());
            digest.update(line.start_ms.unwrap_or(0).to_le_bytes());
            for word in &line.words {
                digest.update(word.start_ms.to_le_bytes());
                digest.update(word.end_ms.to_le_bytes());
            }
        }
    }
    format!("{uri}|{source_id}|{:x}", digest.finalize())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Word {
    pub text: String,
    pub start_ms: u32,
    pub end_ms: u32,
    /// The next syllable belongs to the same word.
    pub joined: bool,
}

impl Word {
    pub fn progress(&self, position: u32) -> f32 {
        if position < self.start_ms {
            0.0
        } else if position >= self.end_ms {
            1.0
        } else {
            (position - self.start_ms) as f32 / (self.end_ms - self.start_ms).max(1) as f32
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Line {
    pub text: String,
    pub start_ms: Option<u32>,
    pub end_ms: Option<u32>,
    pub words: Vec<Word>,
    pub opposite: bool,
    pub background: bool,
    pub translation: Option<String>,
    pub romanization: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credit {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Document {
    pub lines: Vec<Line>,
    pub provider: String,
    pub credits: Vec<Credit>,
    pub instrumental: bool,
}

impl Document {
    pub fn from_lyrics(lyrics: Lyrics, provider: &str) -> Self {
        let mut doc = Self {
            lines: lyrics
                .lines
                .into_iter()
                .map(|line| Line {
                    text: line.text,
                    start_ms: line.at_ms,
                    ..Default::default()
                })
                .collect(),
            provider: provider.to_string(),
            instrumental: lyrics.instrumental,
            ..Default::default()
        };
        doc.finish();
        doc
    }

    fn finish(&mut self) {
        self.lines.retain(|line| !line.text.trim().is_empty());
        // Static lyrics keep their original order. Timed lines may overlap
        // (duets and backing vocals), so they are sorted without merging.
        if self.lines.iter().all(|line| line.start_ms.is_some()) {
            self.lines.sort_by_key(|line| line.start_ms);
        }
        for index in 0..self.lines.len() {
            if self.lines[index].end_ms.is_none() {
                self.lines[index].end_ms = self.lines[index + 1..]
                    .iter()
                    .filter_map(|line| line.start_ms)
                    .find(|next| Some(*next) > self.lines[index].start_ms);
            }
        }
    }

    pub fn active_line(&self, position: u32) -> Option<usize> {
        self.lines
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, line)| {
                (!line.background
                    && line.start_ms.is_some_and(|start| start <= position)
                    && line.end_ms.is_none_or(|end| position < end))
                .then_some(index)
            })
    }

    pub fn word_synced(&self) -> bool {
        self.lines.iter().any(|line| !line.words.is_empty())
    }

    pub fn synced(&self) -> bool {
        self.lines.iter().any(|line| line.start_ms.is_some())
    }
}

#[derive(Default)]
pub struct State {
    pub auto_key: Option<String>,
    pub auto_generation: u64,
    pub auto_running: bool,
    pub auto_message: Option<String>,
    pub background_pick: Option<String>,
    pub open: bool,
    pub uri: Option<String>,
    pub generation: u64,
    /// Shared with rendering so timed words are not deep-copied every frame.
    pub result: Loadable<Arc<Fetch>>,
    pub following: bool,
    pub shown_line: Option<Option<usize>>,
}

impl State {
    pub fn accepts_auto(&self, key: &str, generation: u64) -> bool {
        self.auto_running
            && self.auto_generation == generation
            && self.auto_key.as_deref() == Some(key)
    }

    pub fn receive(&mut self, uri: &str, generation: u64, result: Result<Fetch, String>) {
        if self.uri.as_deref() == Some(uri) && self.generation == generation {
            self.result = match result {
                Ok(found) => Loadable::Loaded(Arc::new(found)),
                Err(error) => Loadable::Failed(error),
            };
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Fetch {
    pub document: Option<Document>,
    pub notice: Option<String>,
}

pub fn position_with_offset(position_ms: u32, offset_ms: i32) -> u32 {
    i64::from(position_ms)
        .saturating_add(i64::from(offset_ms))
        .clamp(0, i64::from(u32::MAX)) as u32
}

/// Only a publishable key is accepted. A distributed desktop binary cannot
/// keep an application's secret key private.
pub fn valid_key(key: &str) -> bool {
    key.is_empty()
        || (key.starts_with("sl_pk_")
            && key.len() > 6
            && key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'))
}

fn track_id(uri: &str) -> Option<&str> {
    let id = uri.strip_prefix("spotify:track:")?;
    (id.len() == 22 && id.bytes().all(|byte| byte.is_ascii_alphanumeric())).then_some(id)
}

fn seconds(value: &Value) -> Option<u32> {
    let value = value.as_f64()?;
    (value.is_finite() && value >= 0.0 && value <= f64::from(u32::MAX) / 1000.0)
        .then(|| (value * 1000.0).round() as u32)
}

fn millis(value: &Value) -> Option<u32> {
    value
        .as_str()
        .and_then(|value| value.parse().ok())
        .or_else(|| value.as_u64().and_then(|value| value.try_into().ok()))
}

fn vocal_group(value: &Value, opposite: bool, background: bool) -> Option<Line> {
    let syllables = value.get("Syllables")?.as_array()?;
    let words: Vec<Word> = syllables
        .iter()
        .filter_map(|syllable| {
            let start_ms = seconds(&syllable["StartTime"])?;
            let end_ms = seconds(&syllable["EndTime"])?;
            let text = syllable["Text"].as_str()?.trim().to_string();
            (!text.is_empty() && end_ms >= start_ms).then_some(Word {
                text,
                start_ms,
                end_ms,
                joined: syllable["IsPartOfWord"].as_bool().unwrap_or(false),
            })
        })
        .collect();
    if words.is_empty() {
        return None;
    }
    let mut text = String::new();
    for (index, word) in words.iter().enumerate() {
        text.push_str(&word.text);
        if !word.joined && index + 1 < words.len() {
            text.push(' ');
        }
    }
    Some(Line {
        text,
        start_ms: words.iter().map(|word| word.start_ms).min(),
        end_ms: words.iter().map(|word| word.end_ms).max(),
        words,
        opposite,
        background,
        translation: value["TranslatedText"].as_str().map(str::to_string),
        romanization: value["TransliteratedText"].as_str().map(str::to_string),
    })
}

pub fn from_spicy(json: &Value, expected_id: &str) -> Result<Document> {
    let body = json
        .get("Body")
        .context("Spicy Lyrics returned no lyrics body")?;
    if body["id"].as_str() != Some(expected_id) {
        bail!("Spicy Lyrics returned a different recording");
    }
    let source = body["source"].as_str().unwrap_or("unknown");
    let mut doc = Document {
        provider: match source {
            "spicy_lyrics" => "Spicy Lyrics",
            "apple_music" => "Apple Music via Spicy Lyrics",
            "spotify" => "Spotify via Spicy Lyrics",
            _ => "Spicy Lyrics (unknown source)",
        }
        .to_string(),
        ..Default::default()
    };
    if source == "spicy_lyrics" {
        let attribution = &body["UploadAttribution"];
        for (role, prefix) in [("Uploader", "Uploaded by"), ("Maker", "Made by")] {
            if let Some(name) = attribution[role]["username"]
                .as_str()
                .filter(|name| !name.is_empty())
            {
                let url = attribution[role]["url"]
                    .as_str()
                    .filter(|url| url.starts_with("https://spicylyrics.org/"));
                doc.credits.push(Credit {
                    name: format!("{prefix} {name}"),
                    url: url.map(str::to_string),
                });
            } else if role == "Uploader" {
                bail!("Spicy Lyrics community sync is missing uploader attribution");
            }
        }
    }
    match body["Type"].as_str() {
        Some("Syllable") => {
            for line in body["Content"].as_array().context("Missing timed lyrics")? {
                let opposite = line["OppositeAligned"].as_bool().unwrap_or(false);
                if let Some(lead) = vocal_group(&line["Lead"], opposite, false) {
                    doc.lines.push(lead);
                }
                if let Some(groups) = line["Background"].as_array() {
                    doc.lines.extend(
                        groups
                            .iter()
                            .filter_map(|group| vocal_group(group, opposite, true)),
                    );
                }
            }
        }
        Some("Line") | Some("Static") => {
            let timed = body["Type"] == "Line";
            let lines = if timed {
                &body["Content"]
            } else {
                &body["Lines"]
            };
            for line in lines.as_array().context("Missing lyrics lines")? {
                if let Some(text) = line["Text"].as_str() {
                    doc.lines.push(Line {
                        text: text.to_string(),
                        start_ms: timed.then(|| seconds(&line["StartTime"])).flatten(),
                        end_ms: timed.then(|| seconds(&line["EndTime"])).flatten(),
                        opposite: line["OppositeAligned"].as_bool().unwrap_or(false),
                        translation: line["TranslatedText"].as_str().map(str::to_string),
                        romanization: line["TransliteratedText"].as_str().map(str::to_string),
                        ..Default::default()
                    });
                }
            }
        }
        _ => bail!("Unsupported Spicy Lyrics timing format"),
    }
    doc.finish();
    if doc.lines.is_empty() {
        bail!("Spicy Lyrics returned no readable lyrics");
    }
    Ok(doc)
}

pub fn from_more(json: &Value) -> Option<Document> {
    let synced = json["isSynced"].as_bool().unwrap_or(false);
    let mut doc = Document {
        provider: format!(
            "{} via More Lyrics",
            json["providerLyricsDisplayName"]
                .as_str()
                .or_else(|| json["provider"].as_str())
                .unwrap_or("Lyrics")
        ),
        lines: json["lines"]
            .as_array()?
            .iter()
            .filter_map(|line| {
                Some(Line {
                    text: line["words"].as_str()?.to_string(),
                    start_ms: if synced {
                        millis(&line["startTimeMs"])
                    } else {
                        None
                    },
                    end_ms: if synced {
                        millis(&line["endTimeMs"]).filter(|end| *end > 0)
                    } else {
                        None
                    },
                    ..Default::default()
                })
            })
            .collect(),
        ..Default::default()
    };
    doc.finish();
    (!doc.lines.is_empty()).then_some(doc)
}

async fn spicy(
    http: &reqwest::Client,
    cache: &Path,
    id: &str,
    key: &str,
    refresh: bool,
) -> Result<Option<Document>> {
    let path = cache.join(format!("spicy-native-v1-{id}.json"));
    if !refresh
        && let Ok(meta) = tokio::fs::metadata(&path).await
        && meta
            .modified()
            .ok()
            .and_then(|time| time.elapsed().ok())
            .is_some_and(|age| age < Duration::from_secs(7 * 24 * 60 * 60))
        && let Ok(bytes) = tokio::fs::read(&path).await
        && let Ok(doc) = serde_json::from_slice(&bytes)
    {
        return Ok(Some(doc));
    }
    if key.is_empty() {
        return Ok(None);
    }
    if !valid_key(key) {
        bail!("Use a publishable Spicy Lyrics key beginning with sl_pk_");
    }
    let response = http
        .get(format!("{SPICY_API}/{id}"))
        .bearer_auth(key)
        .timeout(TIMEOUT)
        .send()
        .await
        .context("Cannot reach Spicy Lyrics")?;
    match response.status().as_u16() {
        404 => return Ok(None),
        401 | 403 => bail!(
            "Spicy Lyrics key was refused. Check the key and enable access without an Origin header for native clients."
        ),
        429 => bail!("Spicy Lyrics request limit reached. Using other lyrics sources."),
        _ => {}
    }
    let json: Value = response
        .error_for_status()
        .context("Spicy Lyrics is unavailable")?
        .json()
        .await
        .context("Invalid Spicy Lyrics response")?;
    let doc = from_spicy(&json, id)?;
    // Cache writes run on the backend, never on the UI thread. A temporary
    // file and rename prevent interrupted writes from becoming valid cache.
    if let Ok(bytes) = serde_json::to_vec(&doc) {
        let _ = tokio::fs::create_dir_all(cache).await;
        let temporary = path.with_extension(format!("{}.tmp", rand::random::<u64>()));
        if tokio::fs::write(&temporary, bytes).await.is_ok() {
            let _ = tokio::fs::rename(&temporary, &path).await;
        }
        let _ = tokio::fs::remove_file(&temporary).await;
    }
    Ok(Some(doc))
}

async fn more(http: &reqwest::Client, query: &Query) -> Result<Option<Document>> {
    let response = http
        .get(MORE_API)
        .query(&[
            ("artist", query.artist.clone()),
            ("track", query.title.clone()),
            ("album", query.album.clone()),
            ("duration", (query.duration_ms / 1000).to_string()),
        ])
        .timeout(TIMEOUT)
        .send()
        .await
        .context("Cannot reach More Lyrics")?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    let json = response
        .error_for_status()
        .context("More Lyrics is unavailable")?
        .json::<Value>()
        .await
        .context("Invalid More Lyrics response")?;
    Ok(from_more(&json))
}

/// Fallbacks never fabricate word timing. A plain source stays plain and a
/// line source lights up whole lines. Provider failures do not hide a usable
/// answer from another provider.
pub async fn fetch(
    http: &reqwest::Client,
    cache: &Path,
    uri: &str,
    query: &Query,
    key: &str,
    refresh: bool,
) -> Result<Fetch> {
    fetch_with_fallback(http, cache, uri, query, key, refresh, None).await
}

/// Combine the former panel's exact source with the native lyrics providers.
pub async fn fetch_with_fallback(
    http: &reqwest::Client,
    cache: &Path,
    uri: &str,
    query: &Query,
    key: &str,
    refresh: bool,
    exact: Option<Lyrics>,
) -> Result<Fetch> {
    let provider = async {
        match track_id(uri) {
            Some(id) => spicy(http, cache, id, key, refresh).await,
            None => {
                if let Some(video) = uri.strip_prefix("vibefy:youtube:") {
                    crate::music::Client::new(crate::http::Http::new(http.clone()))
                        .lyrics(video)
                        .await
                        .map(|v| v.map(|l| Document::from_lyrics(l, "YouTube Music")))
                } else {
                    Ok(None)
                }
            }
        }
    };
    let fallback_request = async {
        let exact = exact.map(|lyrics| Document::from_lyrics(lyrics, "Spotify"));
        if exact.as_ref().is_some_and(timed_or_instrumental) {
            return Ok(exact);
        }
        match tokio::time::timeout(
            TIMEOUT,
            crate::lyrics::fetch_with_refresh(http, cache, query, refresh),
        )
        .await
        {
            Ok(Ok(found)) => Ok(prefer_timing(
                exact,
                found.map(|lyrics| Document::from_lyrics(lyrics, "LRCLIB")),
            )),
            _ if exact.is_some() => Ok(exact),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(anyhow::anyhow!("LRCLIB timed out")),
        }
    };
    select_sources(provider, fallback_request, more(http, query)).await
}

/// Start the session lookup alongside public providers. A slow Spotify session
/// must not delay available timed lyrics, but remains eligible for missing/plain results.
pub async fn fetch_with_exact_request(
    http: &reqwest::Client,
    cache: &Path,
    uri: &str,
    query: &Query,
    key: &str,
    refresh: bool,
    exact: impl std::future::Future<Output = Option<Lyrics>>,
) -> Result<Fetch> {
    select_sources(
        async {
            fetch(http, cache, uri, query, key, refresh)
                .await
                .map(|f| f.document)
        },
        async {
            Ok(tokio::time::timeout(TIMEOUT, exact)
                .await
                .ok()
                .flatten()
                .map(|lyrics| Document::from_lyrics(lyrics, "Spotify")))
        },
        async { Ok(None) },
    )
    .await
}

fn timed_or_instrumental(doc: &Document) -> bool {
    doc.synced() || doc.instrumental
}

fn prefer_timing(held: Option<Document>, candidate: Option<Document>) -> Option<Document> {
    let quality = |doc: &Document| {
        if doc.word_synced() {
            3
        } else if timed_or_instrumental(doc) {
            2
        } else {
            1
        }
    };
    match (held, candidate) {
        (Some(held), Some(candidate)) if quality(&candidate) > quality(&held) => Some(candidate),
        (Some(held), _) => Some(held),
        (None, candidate) => candidate,
    }
}

async fn select_sources(
    provider: impl std::future::Future<Output = Result<Option<Document>>>,
    fallback_request: impl std::future::Future<Output = Result<Option<Document>>>,
    more_request: impl std::future::Future<Output = Result<Option<Document>>>,
) -> Result<Fetch> {
    // Independent requests start together. A slow provider cannot hide lyrics
    // that are already available; cached Spicy word timing still wins.
    tokio::pin!(provider);
    tokio::pin!(fallback_request);
    let first_sources = async {
        tokio::select! {
            primary = &mut provider => {
                if let Ok(Some(doc)) = &primary
                    && timed_or_instrumental(doc)
                {
                    return (primary, Ok(None));
                }
                (primary, fallback_request.await)
            }
            fallback = &mut fallback_request => {
                let primary = if fallback.as_ref().ok().and_then(Option::as_ref)
                    .is_some_and(timed_or_instrumental) {
                    tokio::time::timeout(Duration::from_millis(700), &mut provider).await
                        .unwrap_or(Ok(None))
                } else { provider.await };
                (primary, fallback)
            }
        }
    };
    tokio::pin!(first_sources);
    tokio::pin!(more_request);
    let mut early_more = None;
    let (primary, lrclib) = tokio::select! {
        biased;
        first = &mut first_sources => first,
        more = &mut more_request => {
            if let Ok(Some(doc)) = &more && timed_or_instrumental(doc) {
                return Ok(Fetch { document: more?, notice: None });
            }
            early_more = Some(more);
            first_sources.await
        }
    };
    let mut notice = None;
    let primary = match primary {
        Ok(document) => document,
        Err(error) => {
            notice = Some(format!("{error:#}"));
            None
        }
    };
    let mut failures = Vec::new();
    let fallback = match lrclib {
        Ok(document) => document,
        Err(_) => {
            failures.push("LRCLIB is unavailable");
            None
        }
    };
    let fallback = prefer_timing(primary, fallback);
    if fallback.as_ref().is_some_and(timed_or_instrumental) {
        return Ok(Fetch {
            document: fallback,
            notice,
        });
    }
    match if let Some(more) = early_more {
        more
    } else {
        more_request.await
    } {
        Ok(Some(doc)) => Ok(Fetch {
            document: prefer_timing(fallback, Some(doc)),
            notice,
        }),
        result => {
            if result.is_err() {
                failures.push("More Lyrics is unavailable");
            }
            if fallback.is_none() && !failures.is_empty() {
                notice.get_or_insert_with(|| failures.join(". "));
            }
            Ok(Fetch {
                document: fallback,
                notice,
            })
        }
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    fn doc(provider: &str, timed: bool) -> Document {
        Document {
            provider: provider.into(),
            lines: vec![Line {
                text: "The words".into(),
                start_ms: timed.then_some(1000),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn plain_primary_never_hides_a_timed_fallback_in_either_response_order() {
        for primary_first in [false, true] {
            let primary = async {
                if !primary_first {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Ok(Some(doc("Spicy", false)))
            };
            let fallback = async {
                if primary_first {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Ok(Some(doc("LRCLIB", true)))
            };
            let found = select_sources(primary, fallback, std::future::pending())
                .await
                .unwrap();
            assert_eq!(found.document.unwrap().provider, "LRCLIB");
        }
    }

    #[tokio::test]
    async fn plain_sources_continue_to_more_lyrics_and_keep_credits() {
        let mut timed = doc("More Lyrics", true);
        timed.credits.push(Credit {
            name: "Contributor".into(),
            url: None,
        });
        let found = select_sources(
            async { Ok(Some(doc("Spicy", false))) },
            async { Ok(Some(doc("Spotify", false))) },
            async { Ok(Some(timed)) },
        )
        .await
        .unwrap()
        .document
        .unwrap();
        assert!(found.synced());
        assert_eq!(found.credits[0].name, "Contributor");
    }

    #[tokio::test]
    async fn plain_text_survives_failed_or_plain_alternatives() {
        for fails in [false, true] {
            let found = select_sources(
                async { Ok(Some(doc("Spicy", false))) },
                async { anyhow::bail!("offline") },
                async {
                    if fails {
                        anyhow::bail!("offline")
                    } else {
                        Ok(Some(doc("More", false)))
                    }
                },
            )
            .await
            .unwrap()
            .document
            .unwrap();
            assert_eq!(found.provider, "Spicy");
            assert!(!found.synced());
            assert!(!found.word_synced());
        }
    }

    #[tokio::test]
    async fn more_timing_is_not_blocked_by_two_slow_sources() {
        let found = tokio::time::timeout(
            Duration::from_millis(200),
            select_sources(std::future::pending(), std::future::pending(), async {
                Ok(Some(doc("More", true)))
            }),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(found.document.unwrap().provider, "More");
    }

    #[tokio::test]
    async fn a_slow_primary_does_not_block_a_timed_fallback() {
        let found = tokio::time::timeout(
            Duration::from_secs(2),
            select_sources(
                std::future::pending(),
                async { Ok(Some(doc("Spotify", true))) },
                std::future::pending(),
            ),
        )
        .await
        .unwrap()
        .unwrap()
        .document
        .unwrap();
        assert_eq!(found.provider, "Spotify");
    }

    #[tokio::test]
    async fn cached_word_timing_still_wins_without_waiting_for_other_sources() {
        let mut words = doc("Spicy", true);
        words.lines[0].words.push(Word {
            text: "The".into(),
            start_ms: 1000,
            end_ms: 1500,
            joined: false,
        });
        let found = tokio::time::timeout(
            Duration::from_secs(1),
            select_sources(
                async { Ok(Some(words)) },
                std::future::pending(),
                std::future::pending(),
            ),
        )
        .await
        .unwrap()
        .unwrap()
        .document
        .unwrap();
        assert!(found.word_synced());
    }
}
