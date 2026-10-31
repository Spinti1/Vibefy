//! Diagnostic: audio file, LRCLIB JSON or native document, runtime folder, scratch folder.
//! Writes the resulting calibration only; no audio or transcript is uploaded.
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(
        args.len() == 4,
        "Usage: auto_sync_probe AUDIO LYRICS_JSON RUNTIME SCRATCH"
    );
    let bytes = std::fs::read(&args[1])?;
    let json: serde_json::Value = serde_json::from_slice(&bytes)?;
    let doc = if let Some(lrc) = json["syncedLyrics"].as_str() {
        spotifast::spicy_lyrics::Document::from_lyrics(
            spotifast::lyrics::Lyrics {
                lines: spotifast::lyrics::parse_lrc(lrc),
                synced: true,
                instrumental: false,
            },
            "LRCLIB diagnostic",
        )
    } else {
        serde_json::from_slice(&bytes).context("Expected native lyrics document or LRCLIB JSON")?
    };
    let comparison = spotifast::auto_sync::TimingComparison {
        reference_ms: (json["duration"].as_f64().unwrap_or(0.0) * 1000.0).round() as u32,
        source_ms: 0,
    };
    let start = std::time::Instant::now();
    let result = spotifast::auto_sync::analyze_with_timing(
        (&args[0]).into(),
        doc,
        (&args[3]).into(),
        (&args[2]).into(),
        comparison,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    eprintln!("Analysis wall time: {:.3}s", start.elapsed().as_secs_f64());
    Ok(())
}
