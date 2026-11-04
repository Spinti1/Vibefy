#![cfg(feature = "demo")]
use serde_json::{Value, json};
use spotifast::{
    api::models::ArtistRef,
    http::Http,
    music::{self, Client, Filter, Kind},
    player::LocalTrack,
};
use std::time::{Duration, Instant};

fn row(id: &str, title: &str) -> Value {
    json!({"musicResponsiveListItemRenderer":{
        "playlistItemData":{"videoId":id},"musicVideoType":"MUSIC_VIDEO_TYPE_ATV",
        "flexColumns":[{"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":title,"navigationEndpoint":{"watchEndpoint":{"videoId":id}}}]}}},
            {"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":"Deftones","navigationEndpoint":{"browseEndpoint":{"browseId":"UCARTIST"}}},{"text":" • "},{"text":"White Pony","navigationEndpoint":{"browseEndpoint":{"browseId":"MPREalbum"}}},{"text":" • "},{"text":"4:16"}]}}}],
        "thumbnail":{"musicThumbnailRenderer":{"thumbnail":{"thumbnails":[{"url":"https://yt3.googleusercontent.com/album=w120-h120-l90-rj","width":120,"height":120}]}}}
    }})
}
fn results() -> music::Results {
    music::parse(
        &json!({"contents":[row("iEtJQmv7-LY","Digital Bath")]}),
        Filter::Songs,
    )
    .unwrap()
}
#[test]
fn artist_card_songs_inherit_artist_but_never_treat_duration_as_artist() {
    let mut song = row("iEtJQmv7-LY", "Drive");
    song["musicResponsiveListItemRenderer"]["flexColumns"][1]["musicResponsiveListItemFlexColumnRenderer"]
        ["text"] = json!({"runs":[{"text":"4:51"}]});
    let value = json!({"contents":[{"musicCardShelfRenderer":{
        "title":{"runs":[{"text":"Deftones","navigationEndpoint":{"browseEndpoint":{"browseId":"UCARTIST"}}}]},
        "contents":[song.clone()]
    }}]});
    let parsed = music::parse(&value, Filter::All).unwrap();
    let track = parsed
        .entries
        .iter()
        .find_map(|e| e.track.as_ref())
        .unwrap();
    assert_eq!(track.artist_names(), "Deftones");
    assert_eq!(track.duration_ms, 291000);
    let isolated = music::parse(&json!({"contents":[song]}), Filter::Songs).unwrap();
    let isolated = isolated.entries[0].track.as_ref().unwrap();
    assert!(isolated.artists.is_empty());
    assert!(
        isolated.has_recording_metadata(),
        "An exact source identity can play without an artist credit"
    );
    let mut spotify = isolated.clone();
    spotify.uri = "spotify:track:metadata-needed".into();
    assert!(
        !spotify.has_recording_metadata(),
        "Spotify matching still requires artist evidence"
    );
}

#[test]
fn music_rows_keep_exact_sources_real_album_metadata_and_large_official_art() {
    let result = results();
    let e = &result.entries[0];
    let track = e.track.as_ref().unwrap();
    assert_eq!(e.kind, Kind::Song);
    assert_eq!(track.uri, "vibefy:youtube:iEtJQmv7-LY");
    assert_eq!(track.duration_ms, 256000);
    assert_eq!(track.album, "White Pony");
    assert_eq!(track.artist_names(), "Deftones");
    assert_eq!(
        e.image.as_deref(),
        Some("https://yt3.googleusercontent.com/album=w600-h600-l90-rj")
    );
    if let spotifast::api::models::PlayableItem::Track(t) = track.to_playable_item() {
        assert!(t.id.is_none());
        assert!(t.artists[0].id.is_none());
    }
}
#[test]
fn parser_deduplicates_tracks_rejects_restricted_rows_and_preserves_continuation() {
    let mut blocked = row("aaaaaaaaaaa", "Unavailable");
    blocked["musicResponsiveListItemRenderer"]["musicItemRendererDisplayPolicy"] =
        "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT".into();
    let result=music::parse(&json!({"contents":[row("iEtJQmv7-LY","Digital Bath"),row("iEtJQmv7-LY","Duplicate"),row("../../bad","Invalid"),blocked],"continuations":[{"nextContinuationData":{"continuation":"NEXT"}}]}),Filter::Songs).unwrap();
    assert_eq!(result.entries.len(), 1);
    let browse = music::parse_browse(
        &json!({"contents":[row("iEtJQmv7-LY","Digital Bath"),row("iEtJQmv7-LY","Digital Bath")]}),
    )
    .unwrap();
    assert_eq!(
        browse.entries.len(),
        2,
        "playlist browse retains repeated occurrences"
    );
    assert_eq!(result.continuation.as_deref(), Some("NEXT"));
    assert!(music::parse(&json!({"error":{"message":"Denied"}}), Filter::Songs).is_err());
    assert_eq!(music::duration_ms("1:02:03"), 3723000);
    assert_eq!(music::duration_ms("3M plays"), 0);
}
#[test]
fn albums_and_artists_remain_browsable_instead_of_becoming_spotify_ids_or_tracks() {
    for (id, kind) in [
        ("MPREalbum", Kind::Album),
        ("UCARTIST", Kind::Artist),
        ("VLPLplaylist", Kind::Playlist),
    ] {
        let result=music::parse(&json!({"contents":[{"musicTwoRowItemRenderer":{"title":{"runs":[{"text":"Release"}]},"navigationEndpoint":{"browseEndpoint":{"browseId":id}},"overlay":{"watchEndpoint":{"videoId":"iEtJQmv7-LY"}}}}]}),Filter::All).unwrap();
        assert_eq!(result.entries[0].kind, kind);
        assert_eq!(result.entries[0].browse_id.as_deref(), Some(id));
        assert!(result.entries[0].track.is_none());
    }
    assert!(!music::valid_browse_id("https://localhost"));
}
#[test]
fn artwork_matching_requires_song_artist_title_and_compatible_duration() {
    let source = LocalTrack {
        uri: "vibefy:youtube:O_IIAYZL1R4".into(),
        title: "Deftones - Digital Bath [Official Music Video]".into(),
        artists: vec![ArtistRef {
            name: "Deftones".into(),
            ..Default::default()
        }],
        duration_ms: 256000,
        ..Default::default()
    };
    let answer = results();
    assert!(music::matching_artwork(&source, &answer).is_some());
    let mut wrong = answer.clone();
    wrong.entries[0].track.as_mut().unwrap().artists[0].name = "Cover Artist".into();
    assert!(music::matching_artwork(&source, &wrong).is_none());
    wrong = answer.clone();
    wrong.entries[0].track.as_mut().unwrap().title = "Digital Bath (Live)".into();
    assert!(music::matching_artwork(&source, &wrong).is_none());
    wrong = answer.clone();
    wrong.entries[0].track.as_mut().unwrap().duration_ms = 999000;
    assert!(music::matching_artwork(&source, &wrong).is_none());
    assert_eq!(source.uri, "vibefy:youtube:O_IIAYZL1R4");
    assert!(!spotifast::addons::safe_art_url(
        "https://yt3.googleusercontent.com.evil/image"
    ));
}
#[test]
fn music_cache_separates_categories_and_is_bounded() {
    let mut state = music::State::default();
    let answer = results();
    state.remember("Song", Filter::Songs, &answer);
    assert!(state.cached("Song", Filter::Albums).is_none());
    for i in 0..8 {
        state.remember(&i.to_string(), Filter::Songs, &answer);
    }
    assert!(state.cached("Song", Filter::Songs).is_none());
    assert!(state.cached("7", Filter::Songs).is_some());
}
#[test]
fn videos_are_rejected_in_every_filter_browse_and_continuation() {
    let mut video = row("abcdefghijk", "Official video");
    video["musicResponsiveListItemRenderer"]["musicVideoType"] = "MUSIC_VIDEO_TYPE_OMV".into();
    let mut unknown = row("bbbbbbbbbbb", "Unclassified clip");
    unknown["musicResponsiveListItemRenderer"]["musicVideoType"] = "MUSIC_VIDEO_TYPE_UGC".into();
    for filter in Filter::ALL {
        let value =
            json!({"contents":[video.clone(),unknown.clone(),row("iEtJQmv7-LY","Digital Bath")]});
        let parsed = music::parse(&value, filter).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].kind, Kind::Song);
        assert_eq!(music::parse_browse(&value).unwrap().entries.len(), 1);
        let page = json!({"continuationContents":{"musicShelfContinuation":{"contents":[video.clone(),unknown.clone()]}}});
        assert!(music::parse(&page, filter).unwrap().entries.is_empty());
    }
    assert!(Filter::ALL.iter().all(|f| f.label() != "Videos"));
}
#[test]
fn video_cards_are_never_relabelled_as_songs() {
    let value = json!({"contents":[{"musicCardShelfRenderer":{"title":{"runs":[{"text":"Official clip"}]},"subtitle":{"runs":[{"text":"Video • Artist • 3:00"}]},"onTap":{"watchEndpoint":{"videoId":"abcdefghijk"}}}}]});
    for filter in Filter::ALL {
        assert!(music::parse(&value, filter).unwrap().entries.is_empty());
    }
}
#[test]
fn imports_prefer_main_thumbnails_over_larger_numbered_video_frames() {
    let source = json!({"id":"O_IIAYZL1R4","thumbnails":[
        {"url":"https://i.ytimg.com/vi/O_IIAYZL1R4/hqdefault.jpg","width":480,"height":360},
        {"url":"https://i.ytimg.com/vi/O_IIAYZL1R4/3.jpg","width":1280,"height":720}
    ]});
    assert_eq!(
        spotifast::addons::source_thumbnail(&source, true).as_deref(),
        Some("https://i.ytimg.com/vi/O_IIAYZL1R4/hqdefault.jpg")
    );
    let source = json!({"id":"O_IIAYZL1R4","thumbnail":"https://i.ytimg.com/vi/O_IIAYZL1R4/2.jpg"});
    assert_eq!(
        spotifast::addons::source_thumbnail(&source, true).as_deref(),
        Some("https://i.ytimg.com/vi/O_IIAYZL1R4/hqdefault.jpg")
    );
}
#[test]
fn artwork_updates_preserve_video_ids_duplicates_order_and_timing() {
    use spotifast::{
        app::{App, AppOptions},
        backend::Waker,
        paths::AppDirs,
    };
    let root = std::env::temp_dir().join(format!("vibefy-art-metadata-{}", rand::random::<u64>()));
    let mut app = App::new(
        &Waker::default(),
        AppDirs {
            config: root.join("config"),
            state: root.join("state"),
            cache: root.join("cache"),
        },
        Default::default(),
        AppOptions {
            media_controls: false,
            restore_sign_in: false,
            tray: false,
            autostart: false,
        },
    );
    spotifast::demo::populate(&mut app);
    let collection=spotifast::addons::parse_collection(&json!({"title":"Import","entries":[{"id":"O_IIAYZL1R4","title":"Digital Bath","uploader":"Deftones","duration":256},{"id":"O_IIAYZL1R4","title":"Digital Bath","duration":256},{"id":"abcdefghijk","title":"Other","duration":180}]}),true,"https://www.youtube.com/playlist?list=PL1234567890".into()).unwrap();
    app.addons.collections = vec![collection.clone()];
    app.demo_music_artwork(
        collection.source.clone(),
        collection.tracks[0].uri.clone(),
        "https://yt3.googleusercontent.com/album=w600-h600-l90-rj".into(),
    );
    let updated = &app.addons.collections[0];
    assert_eq!(updated.tracks.len(), collection.tracks.len());
    for (old, new) in collection.tracks.iter().zip(&updated.tracks) {
        assert_eq!(old.uri, new.uri);
        assert_eq!(old.duration_ms, new.duration_ms);
        assert_eq!(old.title, new.title);
        assert_eq!(old.artists, new.artists);
    }
    assert!(updated.tracks[0].cover_url.is_some());
    assert_eq!(updated.tracks[0].cover_url, updated.tracks[1].cover_url);
    assert!(updated.tracks[2].cover_url.is_none());
    assert_eq!(updated.image, updated.tracks[0].cover_url);
    app.backend.shutdown();
    let _ = std::fs::remove_dir_all(root);
}

async fn fixture(
    status: &str,
    headers: &str,
    body: &str,
) -> (Client, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::demo_at(
        Http::new(reqwest::Client::new()),
        format!("http://{}", listener.local_addr().unwrap()),
    );
    let response = format!(
        "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = vec![0; 16384];
        let mut length = 0;
        loop {
            let n = stream.read(&mut bytes[length..]).await.unwrap();
            length += n;
            if n == 0 {
                break;
            }
            let input = String::from_utf8_lossy(&bytes[..length]);
            if let Some((head, body)) = input.split_once("\r\n\r\n") {
                let size = head
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|s| s.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if body.len() >= size {
                    break;
                }
            }
        }
        stream.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8_lossy(&bytes[..length]).into_owned()
    });
    (client, server)
}
#[tokio::test]
async fn public_music_requests_do_not_use_spotify_credentials_or_transport() {
    let (client, server) = fixture(
        "200 OK",
        "Content-Type: application/json\r\n",
        &json!({"contents":[row("iEtJQmv7-LY","Digital Bath")]}).to_string(),
    )
    .await;
    let found = client
        .search("Digital Bath", Filter::Songs, None)
        .await
        .unwrap();
    assert_eq!(found.entries.len(), 1);
    let request = server.await.unwrap();
    let lower = request.to_lowercase();
    assert!(!lower.contains("authorization:") && !lower.contains("cookie:"));
    assert!(request.starts_with("POST /search "));
    let value: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(value["query"], "Digital Bath");
    assert_eq!(value["context"]["client"]["clientName"], "WEB_REMIX");
    assert_eq!(value["params"], "EgWKAQIIAWoMEA4QChADEAQQCRAF");
}
#[tokio::test]
async fn youtube_limits_return_promptly_and_apply_to_cloned_clients() {
    let (client, server) = fixture("429 Too Many Requests", "Retry-After: 30\r\n", "{}").await;
    let now = Instant::now();
    assert!(client.search("Song", Filter::Songs, None).await.is_err());
    assert!(now.elapsed() < Duration::from_secs(2));
    server.await.unwrap();
    let now = Instant::now();
    let error = client
        .clone()
        .search("Another", Filter::Albums, None)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("limited"));
    assert!(now.elapsed() < Duration::from_millis(100));
}
#[tokio::test]
#[ignore = "Public YouTube Music network validation; no account or cookies"]
async fn live_public_music_queries_and_artwork() {
    let client = Client::new(Http::from_proxy(&spotifast::settings::ProxyConfig::Off).unwrap());
    for (query, title, artist) in [
        ("Deftones Digital Bath", "Digital Bath", "Deftones"),
        ("Ian Azteca Never Recover", "Never Recover", "Ian"),
    ] {
        let now = Instant::now();
        let found = client.search(query, Filter::All, None).await.unwrap();
        assert!(!found.entries.is_empty(), "{query}");
        assert!(found.entries.iter().any(|entry| entry.track.is_some()));
        assert!(
            found
                .entries
                .iter()
                .filter_map(|e| e.track.as_ref())
                .any(|t| music_title(&t.title) == music_title(title)
                    && t.artists
                        .iter()
                        .any(|a| spotifast::audio_match::normalized(&a.name)
                            .split_whitespace()
                            .any(|name| name == artist.to_lowercase()))),
            "Expected recording missing for {query}"
        );
        println!(
            "query={query}; elapsed_ms={}; rows={}; first={}",
            now.elapsed().as_millis(),
            found.entries.len(),
            found.entries[0].title
        );
    }
    let found = client
        .search("Deftones Digital Bath", Filter::Songs, None)
        .await
        .unwrap();
    assert!(
        found.entries[0]
            .image
            .as_deref()
            .is_some_and(music::is_album_art)
    );
    for filter in [
        Filter::Artists,
        Filter::Albums,
        Filter::Playlists,
        Filter::All,
    ] {
        let result = client.search("Deftones", filter, None).await.unwrap();
        assert!(!result.entries.is_empty(), "{filter:?}");
        if filter == Filter::All {
            for track in result
                .entries
                .iter()
                .filter_map(|e| e.track.as_ref())
                .take(3)
            {
                assert!(
                    !track.artists.is_empty(),
                    "Artist-card song lost its artist: {}",
                    track.title
                );
                assert_eq!(
                    music::duration_ms(&track.artist_names()),
                    0,
                    "A duration must not become an artist"
                );
                println!(
                    "artist-card song={}; artists={}",
                    track.title,
                    track.artist_names()
                );
            }
        }
        if let Some(id) = result.entries.iter().find_map(|e| e.browse_id.as_deref()) {
            let page = client.browse(id).await.unwrap();
            assert!(!page.entries.is_empty(), "browse {id}");
        }
    }
}
fn music_title(text: &str) -> String {
    spotifast::audio_match::normalized(&spotifast::audio_match::clean_title(text)).replace(' ', "")
}
#[tokio::test]
#[ignore = "Public Music result to native audio/decode/cache validation"]
async fn live_music_result_plays_through_the_existing_native_engine() {
    use symphonia::core::{
        codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
        meta::MetadataOptions, probe::Hint,
    };
    let client = Client::new(Http::from_proxy(&spotifast::settings::ProxyConfig::Off).unwrap());
    let found = client
        .search("Deftones Digital Bath", Filter::Songs, None)
        .await
        .unwrap();
    let track = found
        .entries
        .iter()
        .find_map(|entry| entry.track.as_ref())
        .unwrap();
    let app_binary = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Vibefy-App/yt-dlp.exe");
    let binary = if app_binary.is_file() {
        app_binary
    } else {
        spotifast::player::find_yt_dlp_binary().unwrap()
    };
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/music-audio-validation");
    std::fs::create_dir_all(&root).unwrap();
    let cache = spotifast::player::MatchCache::new(&root);
    let bytes = spotifast::player::resolve_track_audio(
        &binary,
        &cache,
        track,
        &spotifast::settings::ProxyConfig::Off,
    )
    .await
    .unwrap();
    let cached = spotifast::player::resolve_track_audio(
        &binary,
        &cache,
        track,
        &spotifast::settings::ProxyConfig::Off,
    )
    .await
    .unwrap();
    assert_eq!(bytes, cached);
    let preview_frames = spotifast::peek::demo_decode(bytes.clone(), false).unwrap();
    assert_eq!(preview_frames, spotifast::sink::SAMPLE_RATE as usize * 30);
    let cancelled_frames = spotifast::peek::demo_decode(bytes.clone(), true).unwrap();
    assert!(cancelled_frames > 0 && cancelled_frames < preview_frames);
    println!(
        "Track Peek: {preview_frames} frames capped at 30s; cancellation after {cancelled_frames} frames; no output device opened"
    );
    let size = bytes.len();
    let stream = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("m4a");
    let probe = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .unwrap();
    let mut format = probe.format;
    let media_track = format.default_track().unwrap();
    let mut decoder = symphonia::default::get_codecs()
        .make(&media_track.codec_params, &DecoderOptions::default())
        .unwrap();
    let mut frames = 0;
    for _ in 0..10 {
        let packet = format.next_packet().unwrap();
        if let Ok(audio) = decoder.decode(&packet) {
            frames += audio.frames();
        }
    }
    assert!(frames > 0);
    println!(
        "source={}; bytes={size}; decoded_frames={frames}; cache hit verified",
        track.uri
    );
    std::fs::remove_dir_all(&root).unwrap();
}
#[tokio::test]
#[ignore = "Read-only artwork proposals for a supplied local playlist fixture"]
async fn live_import_artwork_proposals() {
    let path = std::env::var("VIBEFY_ART_FIXTURE").expect("fixture path");
    let collections = spotifast::addons::load_collections(std::path::Path::new(&path));
    let client = Client::new(Http::from_proxy(&spotifast::settings::ProxyConfig::Off).unwrap());
    let mut mapping = serde_json::Map::new();
    let mut total = 0;
    for collection in collections.iter().take(1) {
        for track in collection.tracks.iter().take(50) {
            let (artist, title) = music::artwork_reference(track);
            if artist.is_empty() || title.is_empty() {
                continue;
            }
            let result = client
                .search(&format!("{artist} {title}"), Filter::Songs, None)
                .await
                .unwrap();
            total += 1;
            if let Some(image) = music::matching_artwork(track, &result) {
                mapping.insert(track.uri.clone(), image.into());
            }
        }
    }
    println!(
        "matched_artwork={}; evaluated={total}; original source metadata remained unchanged",
        mapping.len()
    );
    assert!(!mapping.is_empty());
    let output = std::env::var("VIBEFY_ART_OUTPUT").expect("QA mapping output");
    std::fs::write(output, serde_json::to_vec_pretty(&mapping).unwrap()).unwrap();
}
