//! Lossless cache compaction for ordinary, self-contained AAC MP4 files.
//! Copy sample bytes and timing/edit tables; never decode or encode audio.
//! Unsupported layouts are left intact. This is not a general MP4 writer.

use std::ops::Range;

type Parse<T> = Result<T, &'static str>;
const LIMIT: usize = 128 * 1024 * 1024;

#[derive(Clone)]
struct Atom {
    kind: [u8; 4],
    whole: Range<usize>,
    body: Range<usize>,
}

fn u32_at(data: &[u8], at: usize) -> Parse<usize> {
    Ok(u32::from_be_bytes(
        data.get(at..at.checked_add(4).ok_or("offset")?)
            .ok_or("truncated integer")?
            .try_into()
            .map_err(|_| "integer")?,
    ) as usize)
}

fn atoms(data: &[u8], range: Range<usize>) -> Parse<Vec<Atom>> {
    if range.end > data.len() {
        return Err("atom bounds");
    }
    let mut at = range.start;
    let mut found = Vec::new();
    while at < range.end {
        if range.end - at < 8 || found.len() >= 100_000 {
            return Err("atom header");
        }
        let size = u32_at(data, at)?;
        let (size, header) = match size {
            0 => (range.end - at, 8),
            1 => {
                let bytes = data.get(at + 8..at + 16).ok_or("extended size")?;
                (
                    usize::try_from(u64::from_be_bytes(bytes.try_into().map_err(|_| "size")?))
                        .map_err(|_| "size overflow")?,
                    16,
                )
            }
            size => (size, 8),
        };
        let end = at
            .checked_add(size)
            .filter(|end| size >= header && *end <= range.end)
            .ok_or("atom size")?;
        found.push(Atom {
            kind: data[at + 4..at + 8].try_into().map_err(|_| "type")?,
            whole: at..end,
            body: at + header..end,
        });
        at = end;
    }
    Ok(found)
}

fn one(items: &[Atom], kind: &[u8; 4]) -> Parse<Atom> {
    let mut found = items.iter().filter(|a| &a.kind == kind);
    let first = found.next().ok_or("missing atom")?.clone();
    if found.next().is_some() {
        return Err("duplicate atom");
    }
    Ok(first)
}

fn boxed(kind: &[u8; 4], body: &[u8]) -> Parse<Vec<u8>> {
    let size = body
        .len()
        .checked_add(8)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("output size")?;
    let mut result = Vec::with_capacity(size as usize);
    result.extend_from_slice(&size.to_be_bytes());
    result.extend_from_slice(kind);
    result.extend_from_slice(body);
    Ok(result)
}

/// Inspect container headers only; audio payload is not scanned or copied.
pub fn contains_video(input: &[u8]) -> bool {
    if input.len() > LIMIT || input.get(4..8) != Some(b"ftyp") {
        return false;
    }
    let inspect = || -> Parse<bool> {
        let top = atoms(input, 0..input.len())?;
        let movie = atoms(input, one(&top, b"moov")?.body)?;
        for track in movie.iter().filter(|a| &a.kind == b"trak") {
            let children = atoms(input, track.body.clone())?;
            let media = atoms(input, one(&children, b"mdia")?.body)?;
            let handler = one(&media, b"hdlr")?;
            if handler.body.len() >= 12
                && &input[handler.body.start + 8..handler.body.start + 12] == b"vide"
            {
                return Ok(true);
            }
        }
        Ok(false)
    };
    inspect().unwrap_or(false)
}

/// `None` means no compaction is needed or the layout is unsupported.
/// Errors also preserve the original file: callers must not replace on failure.
pub fn extract(input: &[u8]) -> Parse<Option<Vec<u8>>> {
    if input.len() > LIMIT || input.get(4..8) != Some(b"ftyp") {
        return Ok(None);
    }
    let top = atoms(input, 0..input.len())?;
    if top.iter().any(|a| &a.kind == b"moof") {
        return Ok(None);
    }
    let ftyp = one(&top, b"ftyp")?;
    let moov = one(&top, b"moov")?;
    let movie = atoms(input, moov.body)?;
    if movie.iter().any(|a| &a.kind == b"mvex") {
        return Ok(None);
    }
    let mut audio = Vec::new();
    let mut video = false;
    for track in movie.iter().filter(|a| &a.kind == b"trak") {
        let track_children = atoms(input, track.body.clone())?;
        let mdia = one(&track_children, b"mdia")?;
        let media = atoms(input, mdia.body)?;
        let hdlr = one(&media, b"hdlr")?;
        let handler = input
            .get(hdlr.body.start + 8..hdlr.body.start + 12)
            .filter(|_| hdlr.body.len() >= 12)
            .ok_or("handler")?;
        video |= handler == b"vide";
        if handler == b"soun" {
            audio.push((track.clone(), media));
        }
    }
    if !video || audio.len() != 1 {
        return Ok(None);
    }
    let (track, media) = audio.pop().ok_or("audio track")?;
    let minf = one(&media, b"minf")?;
    let info = atoms(input, minf.body)?;
    let stbl = one(&info, b"stbl")?;
    let table = atoms(input, stbl.body)?;
    // Tables with auxiliary offsets, encryption or packed sample sizes require
    // a broader remuxer. Keep their original bytes instead of guessing.
    if table
        .iter()
        .any(|a| [*b"saio", *b"saiz", *b"senc", *b"stz2"].contains(&a.kind))
    {
        return Ok(None);
    }
    let stsd = one(&table, b"stsd")?;
    let descriptions = input.get(stsd.body.clone()).ok_or("sample descriptions")?;
    if u32_at(descriptions, 4)? != 1 {
        return Ok(None);
    }
    let entries = atoms(descriptions, 8..descriptions.len())?;
    if entries.len() != 1 || &entries[0].kind != b"mp4a" {
        return Ok(None);
    }
    let stsz = one(&table, b"stsz")?;
    let sizes = &input[stsz.body];
    let fixed = u32_at(sizes, 4)?;
    let samples = u32_at(sizes, 8)?;
    if samples == 0
        || samples > 2_000_000
        || (fixed == 0 && samples > sizes.len().saturating_sub(12) / 4)
    {
        return Err("sample count");
    }
    let stsc = one(&table, b"stsc")?;
    let mapping = &input[stsc.body];
    let mappings = u32_at(mapping, 4)?;
    if mappings == 0 || mappings > 1_000_000 || mappings > mapping.len().saturating_sub(8) / 12 {
        return Err("chunk map");
    }
    let mut runs = Vec::with_capacity(mappings);
    for n in 0..mappings {
        let at = 8 + n * 12;
        let first = u32_at(mapping, at)?;
        let count = u32_at(mapping, at + 4)?;
        if count == 0
            || u32_at(mapping, at + 8)? != 1
            || runs.last().is_some_and(|(prev, _)| *prev >= first)
        {
            return Err("chunk run");
        }
        runs.push((first, count));
    }
    if runs[0].0 != 1 {
        return Err("first chunk");
    }
    let offsets: Vec<_> = table
        .iter()
        .filter(|a| &a.kind == b"stco" || &a.kind == b"co64")
        .collect();
    if offsets.len() != 1 {
        return Err("chunk offsets");
    }
    let offsets = offsets[0];
    let wide = &offsets.kind == b"co64";
    let width = if wide { 8 } else { 4 };
    let offset_data = &input[offsets.body.clone()];
    let count = u32_at(offset_data, 4)?;
    if count == 0
        || count > 1_000_000
        || count > offset_data.len().saturating_sub(8) / width
        || runs.last().ok_or("runs")?.0 > count
    {
        return Err("chunk count");
    }
    let media_ranges: Vec<_> = top
        .iter()
        .filter(|a| &a.kind == b"mdat")
        .map(|a| a.body.clone())
        .collect();
    let mut chunks = Vec::with_capacity(count);
    let mut sample: usize = 0;
    let mut run = 0;
    let mut total = 0usize;
    for chunk in 0..count {
        while run + 1 < runs.len() && runs[run + 1].0 <= chunk + 1 {
            run += 1;
        }
        let end_sample = sample
            .checked_add(runs[run].1)
            .filter(|end| *end <= samples)
            .ok_or("sample bounds")?;
        let mut length = 0usize;
        for n in sample..end_sample {
            length = length
                .checked_add(if fixed != 0 {
                    fixed
                } else {
                    u32_at(sizes, 12 + n * 4)?
                })
                .filter(|n| *n <= input.len())
                .ok_or("chunk size")?;
        }
        sample = end_sample;
        let at = 8 + chunk * width;
        let start = if wide {
            usize::try_from(u64::from_be_bytes(
                offset_data[at..at + 8].try_into().map_err(|_| "offset")?,
            ))
            .map_err(|_| "offset overflow")?
        } else {
            u32_at(offset_data, at)?
        };
        let end = start.checked_add(length).ok_or("chunk overflow")?;
        // Top-level media ranges are ordered and disjoint. Avoid a linear
        // scan per chunk in files with many mdat atoms.
        let media_index = media_ranges.partition_point(|range| range.end <= start);
        if length == 0
            || !media_ranges
                .get(media_index)
                .is_some_and(|r| start >= r.start && end <= r.end)
        {
            return Err("chunk outside media");
        }
        total = total
            .checked_add(length)
            .filter(|n| *n <= input.len())
            .ok_or("media size")?;
        chunks.push(start..end);
    }
    if sample != samples {
        return Err("unused samples");
    }
    let mut sorted = chunks.clone();
    sorted.sort_by_key(|r| r.start);
    if sorted.windows(2).any(|p| p[0].end > p[1].start) {
        return Err("overlapping chunks");
    }
    let mvhd = one(&movie, b"mvhd")?;
    // Keep audio timing, edit lists, codec setup and sample tables byte-for-byte.
    // Track references may point at the video, so leave those layouts intact.
    if atoms(input, track.body.clone())?
        .iter()
        .any(|a| &a.kind == b"tref")
    {
        return Ok(None);
    }
    let mut audio_track = input[track.whole.clone()].to_vec();
    let data_start = ftyp.whole.len() + 8 + mvhd.whole.len() + audio_track.len() + 8;
    let mut next = data_start;
    for (n, chunk) in chunks.iter().enumerate() {
        let at = offsets.body.start - track.whole.start + 8 + n * width;
        if wide {
            audio_track[at..at + 8].copy_from_slice(&(next as u64).to_be_bytes());
        } else {
            audio_track[at..at + 4].copy_from_slice(
                &u32::try_from(next)
                    .map_err(|_| "offset limit")?
                    .to_be_bytes(),
            );
        }
        next += chunk.len();
    }
    if next >= input.len() {
        return Ok(None);
    }
    let mut movie_body = input[mvhd.whole].to_vec();
    movie_body.extend_from_slice(&audio_track);
    let movie = boxed(b"moov", &movie_body)?;
    let mut output = Vec::with_capacity(next);
    output.extend_from_slice(&input[ftyp.whole]);
    output.extend_from_slice(&movie);
    output.extend_from_slice(
        &u32::try_from(total + 8)
            .map_err(|_| "mdat limit")?
            .to_be_bytes(),
    );
    output.extend_from_slice(b"mdat");
    for range in chunks {
        output.extend_from_slice(&input[range]);
    }
    Ok(Some(output))
}
