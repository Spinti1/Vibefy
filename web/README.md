# Vibefy web

Desktop presentation and phone-only player hosted on Vercel (team use-vibefy, project root web). Jam transport update: 0.4.44.

## Run and check

Create a virtual environment, install requirements.lock, then run uvicorn app:app --host 127.0.0.1 --port 4173. Run python -m unittest discover -s tests -v and node --test tests/*.test.cjs.

## Device behavior

The root shows the desktop download site. Phones automatically enter /app; desktop/laptop browsers visiting /app return to the presentation. Detection uses the mobile user-agent, not viewport width, so resizing a desktop window does not unlock the player. Device detection is a presentation choice, not a security boundary; a spoofed user-agent can change it. Phone users can view desktop information with /?site=1. Tablets receive the presentation. /download and /privacy remain available.

## Mobile capabilities

Audio-only search with cancellation and short caches; official visible YouTube playback; queue, previous/next, shuffle/repeat; local likes and public playlist imports; local backup/restore; LRCLIB lyrics with offset; installable app shell. Optional Nyan Cat, Oneko, Cat Jam and Track Peek use original credited assets and locally stored preferences. Cat Jam has size (90% default) and manual BPM controls. Disabled mods do not run animation loops; hidden pages stop rendering. Import is built in.

Vivy/Backmusic reactive visuals and automatic BPM detection need an audio signal unavailable from the official iframe. WaveLink needs a separate authorized provider integration; it is not included in the mobile build. Native classic mini window and local Whisper Auto Sync remain desktop-only. Direct Web Audio EQ is available only for local direct-audio development, not the public iframe. The iframe pauses when covered or the page is hidden; background audio is not guaranteed.

## Spotify setup and limits

The publisher sets the public VIBEFY_SPOTIFY_CLIENT_ID environment variable in Vercel and registers exactly https://usevibefy.online/app as a redirect URI in the Spotify Developer Dashboard. No Client Secret is needed or accepted. /api/spotify-config exposes only the validated public identifier. No identifier is configured in production at this update, so the connection button is disabled with an explanation; music search remains usable.

Authorization Code with PKCE uses S256, a random one-use state and a ten-minute callback deadline. Tokens stay in sessionStorage for this tab, never in library backups, our API or logs. Spotify requests are read-only and restricted to the exact API origin and approved library paths. Both playlists and saved songs support explicit pagination. Account content stays in memory and clears on disconnect. Selecting a Spotify song starts a search for its public recording, rather than silently pretending Spotify metadata is audio.

Spotify Development Mode currently permits at most five allowlisted users and requires the app owner to have Premium. Public use needs Spotify Extended Quota approval. See https://developer.spotify.com/documentation/web-api/concepts/quota-modes. A Client ID alone does not remove these restrictions. Some playlist contents may be withheld by Spotify. OAuth was unit-tested; live sign-in cannot be verified until the publisher configures an eligible application.

## Shared Jam

Desktop and mobile web use the same Cloudflare Worker / Durable Object relay, with TLS WebSockets. Host a session or join using a 12-character code or https://usevibefy.online/app?jam=CODE. A desktop browser opens the presentation with instructions to paste the code into the native app; phones open the web player.

The host publishes the exact resolved YouTube recording, play/pause, timing and supported upcoming recordings. Participants can suggest YouTube songs for the host queue. Only the separate host credential permits playback control or ending the room; it is never part of the invitation. Rooms expire after six hours, allow 24 listeners, bound messages/queues and remove metadata at end/expiry. Cloudflare processes connection IP addresses; participants never connect directly. No audio files or music-account credentials are relayed. Browser autoplay, visible-player, background playback and recording availability restrictions still apply. Timing is corrected periodically, not sample-accurate. Deployment and limits: [jam-relay](../jam-relay).

## Hosting and privacy

Vercel serves public/ from its CDN and FastAPI handles the bounded public music API. requirements.txt mirrors requirements.lock. CSP restricts external script, image, frame and connection hosts. Spotify API URLs are validated before attaching authorization. Service worker caches only the public app shell, never API responses or OAuth tokens. API burst limits are per-instance, not a distributed abuse defence.

Never deploy .env files, .vercel credentials, desktop profiles, cookies or model files. Public YouTube extraction was refused from Vercel, so production uses the official embedded player. Do not bypass challenges or upload user cookies. Privacy details are in public/privacy.html; asset credits and notices are in the repository root.
