"""Vibefy web beta. Public catalogue only; never accepts account cookies or arbitrary URLs."""
import asyncio
import base64
import json
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
from services import spotify_page, soundcloud_search, recording_score
from pydantic import BaseModel, Field

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
            "artistId": next((a["id"] for a in row.get("artists", []) if re.fullmatch(r"UC[A-Za-z0-9_-]{22}", a.get("id") or "")), ""),
            "explicit": row.get("isExplicit") if isinstance(row.get("isExplicit"), bool) else None,
            "album": str((row.get("album") or {}).get("name", ""))[:300],
            "duration": row.get("duration_seconds") or 0,
            "image": safe_art((row.get("thumbnails") or [{}])[-1].get("url", ""))}

@app.middleware("http")
async def security(request: Request, call_next):
    if request.method == "POST":
        try:
            if int(request.headers.get("content-length", "0")) > 64 * 1024:
                return JSONResponse({"detail": "Request is too large."}, 413)
        except ValueError:
            return JSONResponse({"detail": "Invalid request size."}, 400)
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
    return {"version": "0.4.53", "beta": True, "search": True, "playback": "official-embedded-player",
            "nativeOnly": ["local Whisper Auto Sync", "audio-reactive desktop visualizers", "classic mini window"],
            "storage": "browser-local", "analytics": False, "jam": "shared Cloudflare relay; desktop and web"}

@app.get("/api/google-config")
def google_config():
    identity = os.getenv("VIBEFY_GOOGLE_WEB_CLIENT_ID", "")
    if not re.fullmatch(r"[0-9]+-[A-Za-z0-9_-]+\.apps\.googleusercontent\.com", identity):
        identity = ""
    return {"clientId": identity}

@app.get("/api/import/spotify")
async def import_spotify(id: str = Query(max_length=22), offset: int = Query(default=0, ge=0, le=20000)):
    return await spotify_page(id, offset)

class MatchSong(BaseModel):
    title: str = Field(min_length=1, max_length=300)
    artist: str = Field(max_length=300)
    duration: float = Field(default=0, ge=0, le=86400)
    primary_artist: str = Field(default="", max_length=300)
    explicit: bool | None = None

@app.post("/api/match")
async def match_recording(song: MatchSong):
    data = await search((song.title + " " + song.artist)[:160])
    scored = sorted(((recording_score(song.model_dump(), t), t) for t in data["tracks"]), key=lambda item: item[0], reverse=True)
    return {"track": scored[0][1] if scored and scored[0][0] >= .7 else None}

@app.get("/api/soundcloud/search")
async def search_soundcloud(q: str = Query(min_length=2, max_length=160)):
    return await worker(lambda: cached(("soundcloud", q.casefold()), 120, lambda: soundcloud_search(q)))

async def juice_request(path, token="", body=None, grant=""):
    headers = {"Authorization": "Bearer " + token} if token else {}
    if grant:
        headers["X-CA"] = grant
    async with httpx.AsyncClient(timeout=15, follow_redirects=False) as client:
        async with client.stream("POST" if body is not None else "GET", "https://api.juicevault.xyz" + path, json=body, headers=headers) as response:
            if response.status_code == 403 or (response.status_code == 401 and not token and body is None):
                raise HTTPException(503, "JuiceVault is blocking requests from this web host. Open JuiceVault directly or use the desktop add-on.")
            if response.status_code == 401:
                raise HTTPException(401, "JuiceVault could not authorize this request. Check your login or try again later.")
            if response.status_code != 200:
                raise HTTPException(502, "JuiceVault is temporarily unavailable.")
            data = bytearray()
            async for chunk in response.aiter_bytes():
                data.extend(chunk)
                if len(data) > 2 * 1024 * 1024:
                    raise HTTPException(502, "JuiceVault returned too much data.")
            return json.loads(data)

def juice_token(request):
    value = request.headers.get("authorization", "")
    if not value:
        return ""
    if not re.fullmatch(r"Bearer [A-Za-z0-9_.-]{1,16384}", value):
        raise HTTPException(400, "Invalid session.")
    return value[7:]

class JuiceLogin(BaseModel):
    username: str = Field(min_length=1, max_length=100)
    password: str = Field(min_length=1, max_length=1024)

@app.post("/api/juice/login")
async def juice_login(value: JuiceLogin):
    ticket = await juice_request("/user/auth/token", body={})
    token = ticket.get("data", {}).get("token", "")
    try:
        payload = token.split(".")[1]
        grant = json.loads(base64.urlsafe_b64decode(payload + "=" * (-len(payload) % 4)))["ca"]
        if not isinstance(grant, str) or not re.fullmatch(r"[!-~]{1,16384}", grant):
            raise ValueError()
    except (ValueError, KeyError, IndexError, TypeError):
        raise HTTPException(502, "JuiceVault sign-in is unavailable in this browser. Public search still works.") from None
    result = await juice_request("/user/auth/login", body={"login": value.username, "password": value.password, "loginToken": token}, grant=grant)
    access = result.get("data", {}).get("accessToken", "")
    if not isinstance(access, str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,16384}", access):
        raise HTTPException(401, "JuiceVault did not grant a session.")
    return {"accessToken": access}

@app.get("/api/juice/search")
async def juice_search(q: str = Query(min_length=2, max_length=160)):
    from urllib.parse import urlencode
    return await juice_request("/music/search?" + urlencode({"q": q}))

@app.get("/api/juice/playlists")
async def juice_playlists(request: Request, username: str = Query(min_length=1, max_length=100)):
    from urllib.parse import quote
    return await juice_request("/user/playlists/user/" + quote(username, safe=""), token=juice_token(request))

@app.get("/api/juice/playlist")
async def juice_playlist(request: Request, id: str = Query(max_length=36)):
    if not re.fullmatch(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}", id):
        raise HTTPException(400, "Invalid playlist ID.")
    return await juice_request("/user/playlists/" + id, token=juice_token(request))

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

@app.get("/api/artist")
async def artist(id: str = Query(max_length=24)):
    if not re.fullmatch(r"UC[A-Za-z0-9_-]{22}", id):
        raise HTTPException(400, "Invalid artist.")
    def load():
        data = YTMusic(requests_session=False).get_artist(id)
        songs = data.get("songs", {}).get("results", [])
        for song in songs:
            song.setdefault("videoType", "MUSIC_VIDEO_TYPE_ATV")
        albums = [{"id": a.get("browseId", ""), "title": str(a.get("title", "Album"))[:200], "image": safe_art((a.get("thumbnails") or [{}])[-1].get("url", ""))} for a in data.get("albums", {}).get("results", []) if re.fullmatch(r"MPREb_[A-Za-z0-9_-]{5,100}", a.get("browseId", ""))]
        return {"name": str(data.get("name", "Artist"))[:200], "image": safe_art((data.get("thumbnails") or [{}])[-1].get("url", "")), "description": str(data.get("description", ""))[:2000], "tracks": [t for row in songs if (t := track(row))], "albums": albums}
    return await worker(lambda: cached(("artist", id), 300, load))

@app.get("/api/album")
async def album(id: str = Query(max_length=110)):
    if not re.fullmatch(r"MPREb_[A-Za-z0-9_-]{5,100}", id):
        raise HTTPException(400, "Invalid album.")
    def load():
        data = YTMusic(requests_session=False).get_album(id)
        rows = data.get("tracks", [])
        for row in rows:
            row.setdefault("videoType", "MUSIC_VIDEO_TYPE_ATV")
            row.setdefault("artists", data.get("artists", []))
            row.setdefault("thumbnails", data.get("thumbnails", []))
        return {"title": str(data.get("title", "Album"))[:200], "tracks": [t for row in rows if (t := track(row))]}
    return await worker(lambda: cached(("album", id), 300, load))

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
async def lyrics(title: str = Query(min_length=1, max_length=300), artist: str = Query(max_length=300), duration: int = Query(default=0, ge=0, le=86400), id: str = Query(default="", max_length=11)):
    if id and not VIDEO.fullmatch(id):
        raise HTTPException(400, "Invalid recording.")
    params = {"track_name": title, "artist_name": artist}
    if duration:
        params["duration"] = str(duration)
    async with httpx.AsyncClient(timeout=8, follow_redirects=False) as client:
        answer = await client.get("https://lrclib.net/api/get", params=params, headers={"User-Agent": "Vibefy/0.4.15 (https://github.com/Spinti1/Vibefy)"})
    if answer.status_code not in (200, 404) or len(answer.content) > 512 * 1024:
        raise HTTPException(502, "Lyrics are temporarily unavailable.")
    data = answer.json() if answer.status_code == 200 else {}
    result = {"plain": str(data.get("plainLyrics") or "")[:100000], "synced": str(data.get("syncedLyrics") or "")[:100000], "source": "LRCLIB"}
    if result["synced"] or not id:
        return result
    def music_lyrics():
        api = YTMusic(requests_session=False)
        reference = api.get_watch_playlist(videoId=id).get("lyrics")
        if not reference:
            return None
        data = api.get_lyrics(reference, timestamps=True)
        if not data:
            return None
        if data.get("hasTimestamps"):
            synced = "\n".join(f"[{line.start_time // 60000:02d}:{line.start_time % 60000 / 1000:05.2f}]{line.text}" for line in data["lyrics"][:2000])
            return {"plain": "", "synced": synced[:100000], "source": "YouTube Music · " + str(data.get("source") or "lyrics")[:100]}
        return {"plain": str(data.get("lyrics") or "")[:100000], "synced": "", "source": "YouTube Music · " + str(data.get("source") or "lyrics")[:100]}
    try:
        extra = await worker(lambda: cached(("lyrics", id), 3600, music_lyrics))
        if extra and (extra["synced"] or not result["plain"]):
            return extra
    except HTTPException:
        pass
    return result

@app.get("/phone", include_in_schema=False)
@app.get("/web", include_in_schema=False)
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
