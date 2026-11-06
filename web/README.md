# Vibefy web

The root is a presentation and Windows download page. `/web` opens the responsive browser player on computers and phones; `/phone` and the existing `/app` route are aliases. Hosting stays on the existing Vercel project and usevibefy.online domain.

## Features

Public YouTube Music song search (submit with Enter or Search), artist songs/albums, official visible YouTube playback, queue, previous/next, shuffle/repeat, LRCLIB lyrics with YouTube Music fallback, and timing offset. Lyrics and queue share Now Playing so the official player stays visible.

Create, rename, hide and remove browser-local playlists. Add songs through their options menu. Import public/unlisted YouTube playlists or public Spotify playlists; Spotify titles/artists/durations are matched to YouTube recordings. Private Spotify playlists and Liked Songs cannot be imported. Public YouTube imports are bounded to 500 entries; account imports to 10,000. Export/restore metadata-only JSON backups; existing collections are preserved. Browser storage is not a cross-device Vibefy account.

Optional browser-local mods: WaveLink (public SoundCloud search and official widget), JuiceVault (search and optional account playlist imports), original Nyan Cat frames and Cat Jam animation. Disabled animation mods do not run loops. Cat tempo is manual because iframe audio is unavailable to the page. Visualizers, Track Peek and Oneko are excluded. Upstream creators and asset notices remain credited.

Jam uses the existing Cloudflare relay shared with the desktop app: link/code, 24 listeners, six-hour rooms, metadata only. YouTube recordings are supported. Autoplay may require a click. Audio and account credentials never pass through the relay.

## Optional Google connection

Set the public `VIBEFY_GOOGLE_WEB_CLIENT_ID` Vercel environment variable to a **Web application** Google OAuth client. Register `https://usevibefy.online` as a JavaScript origin. The native Desktop client cannot replace this web registration. Google Identity Services provides the Continue with Google popup for basic identity first. YouTube read-only access is a separate explicit step for choosing owned playlists; saving an unlisted copy or editing it requests `youtube.force-ssl` incrementally. Google verification, account eligibility and API quotas still apply.

Short-lived Google tokens stay only in page memory and go directly to Google APIs. No client secret, account cookies or refresh token is stored. There is no shared Google account and no native browser-session import on the website. Until configured, the UI explains that Google connection is unavailable; guest playback and public import remain available.

JuiceVault does not allow browser CORS for its API. Its narrow same-origin adapter forwards a user-submitted login to the fixed JuiceVault origin. Passwords are not saved or logged; the access token stays in page memory. Challenges and expired sessions fail visibly without bypasses. Disconnect clears the local session; downloaded playlist metadata remains local.

## Run and verify

Install `requirements.lock`, then run `python -m uvicorn app:app --host 127.0.0.1 --port 4173` from web. Run `python -m unittest discover -s tests -v` and `node --test tests/*.test.cjs`. JavaScript is buildless. The official embedded player is used in local preview too.

Vercel serves public assets from its CDN and FastAPI handles bounded public metadata requests. Source URLs, IDs, response sizes and timeouts are constrained; no arbitrary URL proxy is exposed. Rate limits are per instance, not a distributed abuse defence. Service worker caches the shell only, never API responses, tokens or audio.

No web audio downloads, guaranteed background audio, EQ, native mini window, local Whisper Auto Sync or desktop visualizers are promised. YouTube/SoundCloud embedding restrictions still apply. Never deploy `.env`, `.vercel` credentials, desktop profiles, cookies or models.

The deployment also has the controlled public alias https://vibefy-neon.vercel.app. Jam explicitly allows this alias; arbitrary Vercel sites remain blocked.
