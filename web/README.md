# Vibefy web

Desktop landing page and mobile web beta, deployed to Vercel in the `use-vibefy` team. The project root on Vercel must be **web**. No Spotify credentials are deployed.

```sh
python -m venv .venv
# Activate the environment, then:
pip install -r requirements.lock
uvicorn app:app --host 127.0.0.1 --port 4173
python -m unittest discover -s tests -v
```

`/` introduces the desktop app. `/app` opens the responsive web player, also on desktop when requested. `/privacy` documents data handling. Public API endpoints: `/api/search`, `/api/playlist`, `/api/audio/{videoId}`, `/api/lyrics`, `/api/capabilities`.

## Implemented browser features

Audio-only music search with cancellation and short session caches; exact-ID playback through the official visible YouTube player on Vercel (bounded HTTP audio ranges in local development); queue, previous/next, shuffle/repeat, local liked songs and imported public audio playlists; local backup/restore; LRCLIB lyrics with timed lines and offset; five-band Web Audio EQ for direct audio only; Media Session integration; installable app shell. Music needs a connection. Saved data is specific to this browser and is not synced to Spotify or the native app.

## Remaining parity work

The native executable is not compiled to a browser. Native Spotify authorization / account library, local Whisper recording alignment, direct LAN TCP Listen Together, classic mini window and native desktop mods are not exposed as working web features. Browser / OS background playback policy also varies. Full parity requires explicit web implementations and, for shared sessions / account data, durable backend storage and an authorized Spotify application. A Vercel deployment alone does not supply these.

Public YouTube extraction may be refused from cloud IPs or by provider changes. Verify `/api/audio` and actual playback from the deployed environment before advertising reliable mobile playback. Never bypass provider challenges or upload a user's cookies. Configure deployment-wide rate limits before broad promotion; the in-process limits are only a local safeguard. No paid backend has been provisioned.

## Hosting

`vercel.json` configures FastAPI, static rewrites and security headers. `requirements.txt` mirrors the complete pinned `requirements.lock`. `public/` is served by Vercel's CDN; only local preview mounts it through FastAPI. Never include `.env.local`, `.vercel`, native profiles or model files in source uploads. The Vercel CLI can create an ignored local OAuth environment file when linking; do not commit it.

Deployed verification: public search answered successfully from Vercel. YouTube refused server-side extraction with a bot-verification challenge. The public deployment therefore uses the official visible YouTube iframe player without cookies or challenge bypass. It pauses when Now Playing closes or the page becomes hidden. The official player cannot feed the Web Audio equalizer; those controls are disabled for this mode. The browser beta is not feature-equivalent to the native app.

## Native-style browser interface and Jam

The default website opens the player, styled around the native Vibefy sidebar, search toolbar, collection grid and bottom player. The download / credits page remains at /download. Browser Listen Together uses pinned local PeerJS 1.5.5 with public signalling and STUN; no paid relay is provisioned. Host shares a random invitation; up to eight listeners follow recording identity, play/pause and timing. Channels use WebRTC encryption, validate message sizes and types, and open only after Host / Join. Leave closes connections and timers. Some NAT/firewall combinations need the same Wi-Fi or VPN. Desktop LAN sessions and browser sessions are currently separate protocols. Official YouTube playback still requires a visible player and may need a participant to press Play once.
