//! Public YouTube Music catalogue metadata. No Spotify grants or user cookies.
use crate::{api::models::ArtistRef, http::Http, player::LocalTrack};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Filter {
    #[default]
    All,
    Songs,
    Albums,
    Artists,
    Playlists,
}
impl Filter {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::Songs,
        Self::Albums,
        Self::Artists,
        Self::Playlists,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Songs => "Songs",
            Self::Albums => "Albums",
            Self::Artists => "Artists",
            Self::Playlists => "Playlists",
        }
    }
    fn params(self) -> Option<String> {
        // Public category parameters, documented by sigma67/ytmusicapi (MIT).
        match self {
            Self::All => None,
            Self::Playlists => Some("Eg-KAQwIABAAGAAgACgBMABqChAEEAMQCRAFEAo%3D".into()),
            other => Some(format!(
                "EgWKAQ{}AWoMEA4QChADEAQQCRAF",
                match other {
                    Self::Songs => "II",
                    Self::Albums => "IY",
                    Self::Artists => "Ig",
                    _ => unreachable!(),
                }
            )),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Song,
    Album,
    Artist,
    Playlist,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Song => "Song",
            Self::Album => "Album",
            Self::Artist => "Artist",
            Self::Playlist => "Playlist",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub kind: Kind,
    pub title: String,
    pub subtitle: String,
    pub image: Option<String>,
    pub track: Option<LocalTrack>,
    pub browse_id: Option<String>,
    /// Existing Spotify data only; never triggers an additional request.
    pub spotify_note: Option<String>,
}
#[derive(Clone, Debug, Default)]
pub struct Results {
    pub entries: Vec<Entry>,
    pub continuation: Option<String>,
}

#[derive(Default)]
pub struct State {
    pub filter: Filter,
    pub results: Results,
    pub loading: bool,
    pub loading_more: bool,
    pub error: Option<String>,
    pub source_key: String,
    pub page_title: String,
    cache: VecDeque<(String, Filter, Instant, Results)>,
}
impl State {
    pub fn cached(&mut self, query: &str, filter: Filter) -> Option<Results> {
        self.cache
            .retain(|(_, _, at, _)| at.elapsed() < Duration::from_secs(120));
        let index = self
            .cache
            .iter()
            .position(|(q, f, _, _)| q == query && *f == filter)?;
        let entry = self.cache.remove(index)?;
        let result = entry.3.clone();
        self.cache.push_back(entry);
        Some(result)
    }
    pub fn remember(&mut self, query: &str, filter: Filter, results: &Results) {
        self.cache.retain(|(q, f, _, _)| q != query || *f != filter);
        self.cache
            .push_back((query.into(), filter, Instant::now(), results.clone()));
        while self.cache.len() > 8 {
            self.cache.pop_front();
        }
    }
}

#[derive(Clone)]
pub struct Client {
    http: Http,
    cooldown: Arc<tokio::sync::Mutex<Option<Instant>>>,
    #[cfg(feature = "demo")]
    endpoint: Option<String>,
}
impl Client {
    pub fn new(http: Http) -> Self {
        Self {
            http,
            cooldown: Arc::default(),
            #[cfg(feature = "demo")]
            endpoint: None,
        }
    }
    async fn request(&self, endpoint: &str, mut body: Value) -> Result<Value> {
        ensure!(
            !self
                .cooldown
                .lock()
                .await
                .is_some_and(|until| until > Instant::now()),
            "Search is temporarily limited. Try again later."
        );
        let date = jiff::Timestamp::now()
            .to_zoned(jiff::tz::TimeZone::UTC)
            .strftime("%Y%m%d")
            .to_string();
        let version = format!("1.{date}.01.00");
        body["context"] = json!({"client":{"clientName":"WEB_REMIX","clientVersion":version,"hl":"en"},"user":{}});
        let client = self.http.client().map_err(anyhow::Error::msg)?;
        let base = "https://music.youtube.com/youtubei/v1";
        #[cfg(feature = "demo")]
        let base = self.endpoint.as_deref().unwrap_or(base);
        let mut response = client
            .post(format!("{base}/{endpoint}"))
            .header("Origin", "https://music.youtube.com")
            .header("X-Youtube-Client-Name", "67")
            .header("X-Youtube-Client-Version", &version)
            .timeout(Duration::from_secs(6))
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Search connection failed: {}", e.without_url()))?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let seconds = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(30);
            *self.cooldown.lock().await =
                Instant::now().checked_add(Duration::from_secs(seconds.max(1)));
            anyhow::bail!("Search is temporarily limited. Try again later.");
        }
        ensure!(
            response.status().is_success(),
            "Search is unavailable (HTTP {}). Try again later.",
            response.status().as_u16()
        );
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| anyhow::anyhow!("Incomplete search response: {}", e.without_url()))?
        {
            ensure!(
                bytes.len() + chunk.len() <= 8_000_000,
                "Search returned too much data"
            );
            bytes.extend_from_slice(&chunk);
        }
        let value: Value =
            serde_json::from_slice(&bytes).context("Search returned an invalid response")?;
        ensure!(
            value.get("error").is_none(),
            "Search could not complete this request"
        );
        Ok(value)
    }
    pub async fn search(
        &self,
        query: &str,
        filter: Filter,
        continuation: Option<&str>,
    ) -> Result<Results> {
        ensure!(
            !query.trim().is_empty() && query.len() <= 500,
            "Enter a search up to 500 characters"
        );
        let mut body = json!({"query":query});
        if let Some(params) = filter.params() {
            body["params"] = params.into();
        }
        if let Some(token) = continuation {
            ensure!(token.len() <= 16384, "Invalid result cursor");
            body["continuation"] = token.into();
        }
        tokio::time::timeout(Duration::from_secs(8), async {
            parse(&self.request("search", body).await?, filter)
        })
        .await
        .context("Search timed out. Try again.")?
    }
    #[cfg(feature = "demo")]
    pub fn demo_at(http: Http, endpoint: String) -> Self {
        let url = reqwest::Url::parse(&endpoint).expect("loopback fixture");
        assert!(url.scheme() == "http" && url.host_str() == Some("127.0.0.1"));
        let mut client = Self::new(http);
        client.endpoint = Some(endpoint);
        client
    }
    pub async fn browse(&self, id: &str) -> Result<Results> {
        self.browse_page(id, None).await
    }
    pub async fn browse_page(&self, id: &str, continuation: Option<&str>) -> Result<Results> {
        ensure!(valid_browse_id(id), "Invalid YouTube Music page");
        let mut body = json!({"browseId":id});
        if let Some(token) = continuation {
            ensure!(token.len() <= 16384, "Invalid page cursor");
            body["continuation"] = token.into();
        }
        tokio::time::timeout(Duration::from_secs(8), async {
            parse_browse(&self.request("browse", body).await?)
        })
        .await
        .context("Music page timed out. Try again.")?
    }
}
pub fn valid_browse_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 200
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
pub fn is_album_art(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|u| {
        matches!(
            u.host_str(),
            Some(
                "lh3.googleusercontent.com"
                    | "yt3.googleusercontent.com"
                    | "yt3.ggpht.com"
                    | "i.scdn.co"
            )
        )
    })
}
pub fn artwork_url(url: &str) -> String {
    if is_album_art(url)
        && !url.contains('?')
        && let Some((base, size)) = url.rsplit_once('=')
        && (size.starts_with('w') || size.starts_with('s'))
    {
        return format!("{base}=w600-h600-l90-rj");
    }
    url.into()
}

fn text(value: &Value) -> String {
    value["simpleText"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| {
            value["runs"]
                .as_array()
                .map(|runs| runs.iter().filter_map(|r| r["text"].as_str()).collect())
                .unwrap_or_default()
        })
}
fn find<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    if let Some(v) = value.get(key) {
        return Some(v);
    }
    match value {
        Value::Object(map) => map.values().find_map(|v| find(v, key)),
        Value::Array(list) => list.iter().find_map(|v| find(v, key)),
        _ => None,
    }
}
fn find_str(value: &Value, key: &str) -> Option<String> {
    find(value, key)?.as_str().map(str::to_owned)
}
fn thumbnail(value: &Value) -> Option<String> {
    let mut candidates = Vec::new();
    fn visit(v: &Value, out: &mut Vec<(u64, String)>) {
        if let Some(url) = v["url"].as_str().filter(|u| crate::addons::safe_art_url(u)) {
            out.push((
                v["width"].as_u64().unwrap_or(0) * v["height"].as_u64().unwrap_or(0),
                url.into(),
            ));
        }
        match v {
            Value::Object(map) => {
                for x in map.values() {
                    visit(x, out)
                }
            }
            Value::Array(list) => {
                for x in list {
                    visit(x, out)
                }
            }
            _ => (),
        }
    }
    visit(value, &mut candidates);
    candidates
        .into_iter()
        .max_by_key(|(size, _)| *size)
        .map(|(_, url)| artwork_url(&url))
}
pub fn duration_ms(value: &str) -> u32 {
    let mut seconds = 0u32;
    let mut count = 0;
    for part in value.split(':') {
        let Ok(n) = part.trim().parse::<u32>() else {
            return 0;
        };
        seconds = seconds.saturating_mul(60).saturating_add(n);
        count += 1;
    }
    if (2..=3).contains(&count) {
        seconds.min(86400) * 1000
    } else {
        0
    }
}
fn row(value: &Value, _filter: Filter, library: bool) -> Option<Entry> {
    if value["musicItemRendererDisplayPolicy"].as_str()
        == Some("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
    {
        return None;
    }
    let flex = value["flexColumns"].as_array();
    let first = flex
        .and_then(|x| x.first())
        .and_then(|x| x.get("musicResponsiveListItemFlexColumnRenderer"));
    let title = text(first.map(|x| &x["text"]).unwrap_or(&value["title"]));
    if title.is_empty() {
        return None;
    }
    let details = flex
        .and_then(|x| x.get(1))
        .map(|x| &x["musicResponsiveListItemFlexColumnRenderer"]["text"])
        .unwrap_or(&value["subtitle"]);
    let title_endpoint = first
        .and_then(|x| find(&x["text"], "navigationEndpoint"))
        .or_else(|| find(&value["title"], "navigationEndpoint"))
        .or_else(|| value.get("navigationEndpoint"));
    let browse_id = title_endpoint
        .and_then(|x| find_str(x, "browseId"))
        .or_else(|| {
            value
                .get("navigationEndpoint")
                .and_then(|x| find_str(x, "browseId"))
        });
    let video_id = value["playlistItemData"]["videoId"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| title_endpoint.and_then(|x| find_str(x, "videoId")))
        .or_else(|| find_str(&value["overlay"], "videoId"))
        .or_else(|| find_str(&value["onTap"], "videoId"))
        .filter(|id| crate::addons::valid_video_id(id));
    let video_type = find_str(value, "musicVideoType").unwrap_or_default();
    // Reject official videos, UGC and unclassified clips even in Songs and
    // browse continuations. Category hints must never relabel a video.
    let kind = if let Some(id) = &browse_id {
        if id.starts_with("MPRE") {
            Kind::Album
        } else if id.starts_with("UC") || id.starts_with("MPLA") {
            Kind::Artist
        } else if id.starts_with("VL") || id.starts_with("RD") || id.starts_with("VM") {
            Kind::Playlist
        } else {
            return None;
        }
    } else if video_id.is_some() && (video_type == "MUSIC_VIDEO_TYPE_ATV" || library) {
        Kind::Song
    } else {
        return None;
    };
    let mut artists = Vec::new();
    let mut album = String::new();
    let mut duration = 0;
    if let Some(runs) = details["runs"].as_array() {
        for run in runs {
            let label = run["text"].as_str().unwrap_or("");
            let id = find_str(run, "browseId").unwrap_or_default();
            if id.starts_with("UC") || id.starts_with("MPLA") {
                artists.push(ArtistRef {
                    name: label.chars().take(300).collect(),
                    ..Default::default()
                });
            } else if id.starts_with("MPRE") {
                album = label.into();
            }
            let seconds = duration_ms(label);
            if seconds > 0 {
                duration = seconds;
            }
        }
    }
    if duration == 0
        && let Some(cols) = value["fixedColumns"].as_array()
    {
        for col in cols {
            if let Some(t) = find(col, "text") {
                duration = duration.max(duration_ms(&text(t)));
            }
        }
    }
    let subtitle = text(details);
    if artists.is_empty() && video_id.is_some() {
        let name = subtitle.split(['•', '·']).find(|s| {
            !s.trim().is_empty()
                && !matches!(s.trim(), "Video" | "Song" | "Album" | "Single" | "EP")
                && duration_ms(s.trim()) == 0
                && ![
                    " views",
                    " plays",
                    " monthly audience",
                    " subscribers",
                    " listeners",
                ]
                .iter()
                .any(|suffix| s.trim().to_lowercase().ends_with(suffix))
        });
        if let Some(name) = name {
            artists.push(ArtistRef {
                name: name.trim().chars().take(300).collect(),
                ..Default::default()
            });
        }
    }
    let image = thumbnail(
        value
            .get("thumbnail")
            .or_else(|| value.get("thumbnailRenderer"))
            .unwrap_or(&Value::Null),
    );
    let video_id = if kind == Kind::Song { video_id } else { None };
    let track = video_id.as_ref().map(|id| LocalTrack {
        uri: format!("vibefy:youtube:{id}"),
        title: title.chars().take(500).collect(),
        name: title.chars().take(500).collect(),
        artists: artists.into_iter().take(20).collect(),
        album: album.clone(),
        album_name: album,
        duration_ms: duration,
        cover_url: image.clone(),
        art_url: image.clone(),
        art_small_url: image.clone(),
        ..Default::default()
    });
    Some(Entry {
        id: video_id.or(browse_id.clone())?,
        kind,
        title: title.chars().take(500).collect(),
        subtitle: subtitle
            .strip_prefix(&format!("{} • ", kind.label()))
            .or_else(|| subtitle.strip_prefix(&format!("{} · ", kind.label())))
            .unwrap_or(&subtitle)
            .chars()
            .take(500)
            .collect(),
        image,
        track,
        browse_id,
        spotify_note: None,
    })
}
pub fn parse(value: &Value, filter: Filter) -> Result<Results> {
    parse_rows(value, filter, true, false)
}
pub fn parse_browse(value: &Value) -> Result<Results> {
    parse_rows(value, Filter::All, false, false)
}
/// Explicit account imports preserve user-added recordings and duplicate rows.
/// Public search and browse retain the catalogue-only policy.
pub fn parse_library_browse(value: &Value) -> Result<Results> {
    parse_rows(value, Filter::All, false, true)
}
fn parse_rows(value: &Value, filter: Filter, deduplicate: bool, library: bool) -> Result<Results> {
    ensure!(
        value.get("contents").is_some()
            || value.get("continuationContents").is_some()
            || value.get("onResponseReceivedActions").is_some(),
        "Search returned an unsupported page. Try again later."
    );
    let mut result = Results::default();
    let mut seen = HashSet::new();
    fn walk(
        value: &Value,
        filter: Filter,
        result: &mut Results,
        seen: &mut HashSet<String>,
        depth: usize,
        deduplicate: bool,
        artist: Option<&str>,
        library: bool,
    ) {
        if depth > 60 {
            return;
        }
        if let Some(map) = value.as_object() {
            for (key, v) in map {
                let mut card_artist = None;
                if matches!(
                    key.as_str(),
                    "musicResponsiveListItemRenderer"
                        | "musicTwoRowItemRenderer"
                        | "musicCardShelfRenderer"
                ) && let Some(mut entry) = row(v, filter, library)
                {
                    if key == "musicCardShelfRenderer" && entry.kind == Kind::Artist {
                        card_artist = Some(entry.title.clone());
                    }
                    if let Some(track) = &mut entry.track
                        && track.artists.is_empty()
                        && let Some(artist) = artist
                    {
                        track.artists.push(ArtistRef {
                            name: artist.into(),
                            ..Default::default()
                        });
                    }
                    if (!deduplicate || seen.insert(entry.id.clone()))
                        && result.entries.len() < if library { 20_000 } else { 100 }
                    {
                        result.entries.push(entry);
                    }
                }
                if key == "nextContinuationData" {
                    result.continuation = v["continuation"]
                        .as_str()
                        .filter(|s| s.len() <= 16384)
                        .map(str::to_owned);
                }
                if key == "continuationCommand" && result.continuation.is_none() {
                    result.continuation = v["token"]
                        .as_str()
                        .filter(|s| s.len() <= 16384)
                        .map(str::to_owned);
                }
                walk(
                    v,
                    filter,
                    result,
                    seen,
                    depth + 1,
                    deduplicate,
                    card_artist.as_deref().or(artist),
                    library,
                );
            }
        } else if let Some(list) = value.as_array() {
            for v in list {
                walk(
                    v,
                    filter,
                    result,
                    seen,
                    depth + 1,
                    deduplicate,
                    artist,
                    library,
                );
            }
        }
    }
    walk(
        value,
        filter,
        &mut result,
        &mut seen,
        0,
        deduplicate,
        None,
        library,
    );
    // Bring catalogue songs ahead of release and artist shelves.
    // Preserve the service's top result and every browse page's exact order.
    if deduplicate && filter == Filter::All && result.entries.len() > 1 {
        let first = result.entries.remove(0);
        result.entries.sort_by_key(|e| match e.kind {
            Kind::Song => 0,
            _ => 1,
        });
        result.entries.insert(0, first);
    }
    Ok(result)
}

/// An uploaded video keeps its exact identity. Only a confident music match
/// may supply its album art; duration does not alter playback or lyric timing.
pub fn artwork_reference(track: &LocalTrack) -> (String, String) {
    let mut title = crate::audio_match::clean_title(&track.title);
    let mut artist = track
        .artists
        .first()
        .map(|a| a.name.clone())
        .unwrap_or_default();
    if let Some((left, right)) = title.split_once(" - ") {
        if crate::audio_match::normalized(right) == crate::audio_match::normalized(&artist) {
            title = left.into();
        } else if left.len() <= 100 && !right.to_lowercase().contains("remaster") {
            artist = left.into();
            title = right.into();
        }
    }
    for suffix in [
        " (video)",
        " [video]",
        " (hd)",
        " (4k)",
        " (audio)",
        " [audio]",
        " (lyrics)",
        " [lyrics]",
        " (lyric video)",
        " (visualizer)",
    ] {
        if title.to_ascii_lowercase().ends_with(suffix) {
            title.truncate(title.len() - suffix.len());
        }
    }
    (artist.trim().into(), title.trim().into())
}
pub fn matching_artwork(track: &LocalTrack, results: &Results) -> Option<String> {
    let (artist, title) = artwork_reference(track);
    let norm = crate::audio_match::normalized;
    if artist.is_empty() || title.is_empty() {
        return None;
    }
    results
        .entries
        .iter()
        .filter(|e| e.kind == Kind::Song)
        .find_map(|entry| {
            let found = entry.track.as_ref()?;
            (norm(&crate::audio_match::clean_title(&found.title)) == norm(&title)
                && (found
                    .artists
                    .iter()
                    .any(|a| norm(&a.name).replace(' ', "") == norm(&artist).replace(' ', ""))
                    || norm(&found.artist_names()).replace(' ', "")
                        == norm(&artist).replace(' ', ""))
                && crate::audio_match::compatible_version(&title, &found.title)
                && (track.duration_ms == 0
                    || found.duration_ms == 0
                    || track.duration_ms.abs_diff(found.duration_ms) <= 30000))
                .then(|| entry.image.clone())
                .flatten()
        })
}
