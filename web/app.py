"""Vibefy web beta. Public catalogue only; never accepts account cookies or arbitrary URLs."""
import asyncio
import os
import re
import threading
import time
from collections import OrderedDict, defaultdict, deque
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.parse import urlparse

import httpx
import yt_dlp
from fastapi import FastAPI, HTTPException, Query, Request
from fastapi.responses import FileResponse, JSONResponse, Response
from fastapi.staticfiles import StaticFiles
from ytmusicapi import YTMusic

ROOT = Path(__file__).resolve().parent
app = FastAPI(title="Vibefy", docs_url=None, redoc_url=None, openapi_url=None)
VIDEO = re.compile(r"^[A-Za-z0-9_-]{11}$")
PLAYLIST = re.compile(r"^(?:PL|OLAK5uy_|RDCLAK5uy_|RDAMPL|RDAMVM)[A-Za-z0-9_-]{8,180}$")
CACHE = OrderedDict()
LOCK = threading.Lock()
LIMITS = defaultdict(deque)
SLOTS = threading.BoundedSemaphore(4)
EXECUTOR = ThreadPoolExecutor(max_workers=4, thread_name_prefix="vibefy-music")

def cached(key, ttl, work):
    with LOCK:
        found = CACHE.get(key)
        if found and found[0] > time.monotonic():
            CACHE.move_to_end(key)
            return found[1]
    value = work()
    with LOCK:
        CACHE[key] = (time.monotonic() + ttl, value)
        CACHE.move_to_end(key)
        while len(CACHE) > 128:
            CACHE.popitem(last=False)
    return value

def safe_art(value):
    try:
        url = urlparse(value)
        if url.scheme == "https" and url.hostname and (url.hostname == "i.ytimg.com" or url.hostname.endswith((".googleusercontent.com", ".ggpht.com"))) and not url.username and not url.password and not url.port:
            return value
    except (ValueError, TypeError):
        pass
    return ""

def track(row):
    identity = row.get("videoId", "")
    # Never turn a music video, UGC clip or an unclassified browse item into a song.
    if not VIDEO.fullmatch(identity) or row.get("videoType") != "MUSIC_VIDEO_TYPE_ATV":
        return None
    return {"id": identity, "title": str(row.get("title", "Untitled"))[:300],
            "artist": ", ".join(str(a.get("name", "")) for a in row.get("artists", []))[:300],
            "album": str((row.get("album") or {}).get("name", ""))[:300],
            "duration": row.get("duration_seconds") or 0,
            "image": safe_art((row.get("thumbnails") or [{}])[-1].get("url", ""))}

@app.middleware("http")
async def security(request: Request, call_next):
    if request.url.path.startswith("/api/"):
        # Per-instance burst protection, not a substitute for a deployment firewall.
        ip = request.client.host if request.client else "unknown"
        if os.getenv("VERCEL"):
            ip = request.headers.get("x-vercel-forwarded-for", ip).split(",")[0][:80]
        now = time.monotonic()
        with LOCK:
            if len(LIMITS) > 2048:
                LIMITS.clear()
            bucket = LIMITS[ip]
            while bucket and bucket[0] < now - 60:
                bucket.popleft()
            busy = len(bucket) >= 80
            if not busy:
                bucket.append(now)
        if busy:
            return JSONResponse({"detail": "Too many requests. Please try again shortly."}, 429, headers={"Retry-After": "60"})
    try:
        response = await call_next(request)
    except Exception:
        # Never send extraction URLs, cookies, local paths or a stack trace to a client.
        return JSONResponse({"detail": "The music service is temporarily unavailable."}, 502)
    response.headers["X-Content-Type-Options"] = "nosniff"
    response.headers["Referrer-Policy"] = "strict-origin-when-cross-origin"
    response.headers["X-Frame-Options"] = "DENY"
    if request.url.path.startswith("/api/"):
        response.headers["Cache-Control"] = "no-store"
    return response

async def worker(work):
    if not SLOTS.acquire(blocking=False):
        raise HTTPException(503, "The music service is busy. Please retry shortly.")
    def run():
        try:
            return work()
        finally:
            SLOTS.release()
    try:
        # A timed-out request must not release its concurrency slot while its
        # blocking extractor is still running. No unbounded executor queue.
        future = asyncio.get_running_loop().run_in_executor(EXECUTOR, run)
        return await asyncio.wait_for(asyncio.shield(future), timeout=24)
    except HTTPException:
        raise
    except Exception:
        raise HTTPException(502, "The music source could not answer. Please try again shortly.") from None

@app.get("/api/capabilities")
def capabilities():
    return {"version": "0.4.15", "beta": True, "search": True, "playback": "official-embedded-player" if os.getenv("VERCEL") else "source-dependent",
            "nativeOnly": ["Spotify account library", "local Whisper Auto Sync", "LAN Listen Together", "native desktop mods", "classic mini window"],
            "storage": "browser-local", "analytics": False}

@app.get("/api/search")
async def search(q: str = Query(min_length=1, max_length=160)):
    q = " ".join(q.split())
    if not q:
        raise HTTPException(400, "Enter an artist or song.")
    def load():
        def fetch():
            api = YTMusic(requests_session=False)
            rows = api.search(q, filter="songs", limit=25)
            # The songs filter explicitly identifies catalogue audio tracks. Older
            # responses omit videoType, so infer ATV only for this exact category.
            for row in rows:
                if row.get("resultType") == "song":
                    row.setdefault("videoType", "MUSIC_VIDEO_TYPE_ATV")
            return [item for row in rows if (item := track(row))]
        return cached(("search", q.casefold()), 120, fetch)
    return {"tracks": await worker(load)}

@app.get("/api/playlist")
async def playlist(id: str = Query(min_length=10, max_length=200)):
    if not PLAYLIST.fullmatch(id):
        raise HTTPException(400, "Enter a public YouTube Music playlist ID or URL.")
    def load():
        def fetch():
            data = YTMusic(requests_session=False).get_playlist(id, limit=500)
            rows = data.get("tracks", [])
            items = [item for row in rows if (item := track(row))]
            return {"id": id, "title": str(data.get("title", "Imported playlist"))[:200], "tracks": items,
                    "omitted": len(rows) - len(items), "limited": len(rows) >= 500}
        return cached(("playlist", id), 120, fetch)
    return await worker(load)

def audio_url(identity):
    def extract():
        opts = {"quiet": True, "no_warnings": True, "noplaylist": True, "skip_download": True,
                "socket_timeout": 12, "retries": 1, "extractor_retries": 1,
                "format": "bestaudio[ext=m4a]/bestaudio", "cachedir": False}
        with yt_dlp.YoutubeDL(opts) as extractor:
            data = extractor.extract_info("https://www.youtube.com/watch?v=" + identity, download=False)
        if not data or data.get("is_live") or data.get("has_drm"):
            raise ValueError("Unavailable recording")
        uri = data.get("url", "")
        parsed = urlparse(uri)
        if not (parsed.scheme == "https" and parsed.hostname and parsed.hostname.endswith(".googlevideo.com") and not parsed.username and not parsed.password and parsed.port in (None, 443)):
            raise ValueError("Untrusted audio source")
        mime = "audio/mp4" if data.get("ext") == "m4a" else "audio/webm"
        return uri, mime
    return cached(("audio", identity), 180, extract)

@app.get("/api/audio/{identity}")
async def audio(identity: str, request: Request):
    if os.getenv("VERCEL") and os.getenv("VIBEFY_DIRECT_AUDIO") != "1":
        raise HTTPException(501, "Use the official browser player on this deployment.")
    if not VIDEO.fullmatch(identity):
        raise HTTPException(400, "Invalid recording ID.")
    range_header = request.headers.get("range", "bytes=0-")
    match = re.fullmatch(r"bytes=(\d{1,12})-(\d{0,12})", range_header)
    if not match:
        raise HTTPException(416, "Unsupported audio range.")
    start = int(match[1])
    end = min(int(match[2]) if match[2] else start + 2 * 1024 * 1024 - 1, start + 2 * 1024 * 1024 - 1)
    if end < start:
        raise HTTPException(416, "Invalid audio range.")
    uri, mime = await worker(lambda: audio_url(identity))
    async with httpx.AsyncClient(follow_redirects=False, timeout=20) as client:
        async with client.stream("GET", uri, headers={"Range": f"bytes={start}-{end}"}) as upstream:
            if upstream.status_code not in (200, 206):
                raise HTTPException(502, "This recording is unavailable from the web player.")
            content_range = upstream.headers.get("content-range", "")
            if not re.fullmatch(r"bytes \d+-\d+/\d+", content_range):
                raise HTTPException(502, "The audio source did not return a bounded range.")
            body = bytearray()
            async for chunk in upstream.aiter_bytes():
                body.extend(chunk)
                if len(body) > 2 * 1024 * 1024:
                    raise HTTPException(502, "The audio source exceeded the range limit.")
    return Response(bytes(body), status_code=206, media_type=mime,
                    headers={"Accept-Ranges": "bytes", "Content-Range": content_range, "Cache-Control": "private, max-age=60"})

@app.get("/api/lyrics")
async def lyrics(title: str = Query(min_length=1, max_length=300), artist: str = Query(max_length=300), duration: int = Query(default=0, ge=0, le=86400)):
    params = {"track_name": title, "artist_name": artist}
    if duration:
        params["duration"] = str(duration)
    async with httpx.AsyncClient(timeout=8, follow_redirects=False) as client:
        answer = await client.get("https://lrclib.net/api/get", params=params, headers={"User-Agent": "Vibefy/0.4.15 (https://github.com/Spinti1/Vibefy)"})
    if answer.status_code == 404:
        return {"plain": "", "synced": "", "source": "LRCLIB"}
    if answer.status_code != 200 or len(answer.content) > 512 * 1024:
        raise HTTPException(502, "Lyrics are temporarily unavailable.")
    data = answer.json()
    return {"plain": str(data.get("plainLyrics") or "")[:100000], "synced": str(data.get("syncedLyrics") or "")[:100000], "source": "LRCLIB"}

@app.get("/app", include_in_schema=False)
def player_page():
    return FileResponse(ROOT / "public" / "app.html")

@app.get("/privacy", include_in_schema=False)
def privacy_page():
    return FileResponse(ROOT / "public" / "privacy.html")

# Vercel serves public/ at the edge and excludes it from the function bundle.
# Local preview serves the same files through ASGI.
if (ROOT / "public").is_dir():
    app.mount("/", StaticFiles(directory=ROOT / "public", html=True), name="public")
