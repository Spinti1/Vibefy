//! Verify real search, download and native audio decoding using a scratch cache.
//! Does not access the user's account, playback device or normal cache.
use anyhow::{Context, Result, ensure};
use spotifast::api::models::ArtistRef;
use spotifast::player::{LocalTrack, MatchCache, resolve_track_audio};
use spotifast::settings::ProxyConfig;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 6 || args.len() == 7,
        "Usage: playback_probe YTDLP SCRATCH SPOTIFY_ID ARTIST[;ARTIST] TITLE DURATION_MS [ALBUM]"
    );
    let scratch = PathBuf::from(&args[1]);
    let cache = MatchCache::new(&scratch);
    let track = LocalTrack {
        uri: format!("spotify:track:{}", args[2]),
        artists: args[3]
            .split(';')
            .map(|artist| ArtistRef {
                name: artist.to_string(),
                ..Default::default()
            })
            .collect(),
        title: args[4].clone(),
        album: args.get(6).cloned().unwrap_or_default(),
        duration_ms: args[5].parse()?,
        ..Default::default()
    };
    let start = std::time::Instant::now();
    let bytes = resolve_track_audio(args[0].as_ref(), &cache, &track, &ProxyConfig::Off).await?;
    let selected = cache
        .get_match(&args[2])
        .context("Missing selected recording")?;
    let audio = scratch
        .join("youtube_audio")
        .join(format!("{}.m4a", selected.youtube_id));
    let wav = spotifast::auto_sync::decode_wav(&audio)?;
    // Decoder produces a 44-byte header and mono 16-bit samples at 16 kHz.
    let decoded_ms = wav.len().saturating_sub(44) * 1000 / 32_000;
    ensure!(decoded_ms > 10_000, "Audio must decode beyond its intro");
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "artist": args[3], "title": args[4],
            "source_title": selected.title, "youtube_id": selected.youtube_id,
            "channel": selected.channel, "channel_id": selected.channel_id,
            "source_kind": selected.source_kind,
            "reference_ms": track.duration_ms, "decoded_ms": decoded_ms,
            "audio_bytes": bytes.len(), "proxy": "off",
            "elapsed_seconds": start.elapsed().as_secs_f64()
        }))?
    );
    Ok(())
}
