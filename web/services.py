"""Bounded public metadata adapters. No account cookies, passwords or user tokens."""
import json
import re

import httpx
import yt_dlp
from fastapi import HTTPException

SPOTIFY_QUERY = "c685ca91fb0019c87ff1225f84e1d85b26cd631fa6187f3f0900da182cb0953d"


async def bounded(response):
    if response.status_code == 429:
        raise HTTPException(429, "The source is limiting imports. Please retry later.", headers={"Retry-After": "60"})
    if response.status_code != 200:
        raise HTTPException(502, "The public playlist could not be read. Check that it is public and try again later.")
    chunks = bytearray()
    async for chunk in response.aiter_bytes():
        chunks.extend(chunk)
        if len(chunks) > 8 * 1024 * 1024:
            raise HTTPException(502, "The source response is too large.")
    return chunks.decode("utf-8")


async def spotify_page(identity, offset):
    if not re.fullmatch(r"[A-Za-z0-9]{22}", identity):
        raise HTTPException(400, "Use a public Spotify playlist link. No Spotify login is needed.")
    async with httpx.AsyncClient(timeout=15, follow_redirects=False) as client:
        async with client.stream("GET", "https://open.spotify.com/embed/playlist/" + identity) as response:
            html = await bounded(response)
        match = re.search(r'<script\b[^>]*\bid="__NEXT_DATA__"[^>]*>(.*?)</script>', html, re.S)
        try:
            state = json.loads(match[1])["props"]["pageProps"]["state"]
            if state["data"]["entity"]["uri"] != "spotify:playlist:" + identity:
                raise ValueError()
            token = state["settings"]["session"]["accessToken"]
            if not isinstance(token, str) or not re.fullmatch(r"[!-~]{1,8192}", token):
                raise ValueError()
        except (ValueError, KeyError, TypeError):
            raise HTTPException(502, "Spotify did not provide this public playlist. Private playlists and Liked Songs are not supported.") from None
        payload = {"operationName": "queryPlaylist", "variables": {"uri": "spotify:playlist:" + identity, "offset": offset, "limit": 100, "includeItemV3": False}, "extensions": {"persistedQuery": {"version": 1, "sha256Hash": SPOTIFY_QUERY}}}
        async with client.stream("POST", "https://api-partner.spotify.com/pathfinder/v2/query", json=payload, headers={"Authorization": "Bearer " + token}) as response:
            value = json.loads(await bounded(response))
    entity = value.get("data", {}).get("playlistV2", {})
    content = entity.get("content", {})
    rows = content.get("items")
    if entity.get("uri") != "spotify:playlist:" + identity or not isinstance(rows, list) or len(rows) > 100:
        raise HTTPException(502, "Spotify could not return this public playlist.")
    tracks = []
    for row in rows:
        t = row.get("itemV2", {}).get("data", {})
        if t.get("__typename") != "Track":
            continue
        artists = t.get("artists", {}).get("items", [])
        tracks.append({"title": str(t.get("name", ""))[:300], "artist": ", ".join(str(a.get("profile", {}).get("name", "")) for a in artists)[:300], "primary_artist": str(artists[0].get("profile", {}).get("name", ""))[:300] if artists else "", "duration": t.get("duration", {}).get("totalMilliseconds", 0) / 1000, "explicit": t.get("contentRating", {}).get("label") == "EXPLICIT"})
    next_offset = content.get("pagingInfo", {}).get("nextOffset")
    total = content.get("totalCount", 0)
    if not isinstance(total, int) or not 0 <= total <= 20000 or (next_offset is not None and (not isinstance(next_offset, int) or next_offset != offset + len(rows) or next_offset <= offset or next_offset >= total)):
        raise HTTPException(502, "Unsupported playlist pagination.")
    return {"title": str(entity.get("name", "Spotify playlist"))[:200], "tracks": tracks, "next": next_offset, "total": total}


def soundcloud_search(query):
    options = {"quiet": True, "no_warnings": True, "skip_download": True, "extract_flat": True, "socket_timeout": 8, "retries": 0, "cachedir": False}
    with yt_dlp.YoutubeDL(options) as api:
        data = api.extract_info("scsearch15:" + query, download=False)
    tracks = []
    for t in (data or {}).get("entries", []):
        identity = str(t.get("id", ""))
        if not re.fullmatch(r"\d{1,20}", identity):
            continue
        tracks.append({"id": "sc:" + identity, "title": str(t.get("title", "Untitled"))[:300], "artist": str(t.get("uploader", "SoundCloud"))[:300], "duration": t.get("duration") or 0, "image": t.get("thumbnail") or "", "album": "SoundCloud"})
    return {"tracks": tracks}


def recording_score(source, candidate):
    def terms(value):
        return set(re.findall(r"\w+", value.casefold()))
    a, b = terms(source["title"]), terms(candidate["title"])
    title = 2 * len(a & b) / max(1, len(a) + len(b))
    primary = terms(source.get("primary_artist") or source["artist"].split(",")[0]) - {"the", "and", "feat", "ft"}
    artist = len(primary & terms(candidate["artist"])) / max(1, len(primary))
    duration = abs(source.get("duration", 0) - candidate.get("duration", 0))
    # Preserve explicit/version identity; no silent clean, live or cover replacements.
    for label in ("clean", "censored", "karaoke", "cover", "live", "remix", "slowed", "sped"):
        if (label in a) != (label in b):
            return 0
    if source.get("explicit") is True and candidate.get("explicit") is False:
        return 0
    if title < .65 or artist < .65 or (source.get("duration") and candidate.get("duration") and duration > max(15, source["duration"] * .08)):
        return 0
    return title * .65 + artist * .35 - min(duration / 300, .1)
