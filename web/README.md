# Vibefy web

The root is a presentation and Windows download page. `/web` opens the responsive browser player on computers and phones; `/phone` and the existing `/app` route are aliases. Hosting stays on the existing Vercel project and usevibefy.online domain.

## Features

Mods use compact cards with creator credits and inline Cat Jam settings. Spotify and YouTube have separate import forms. The top bar is the only account connection entry on Home; no synthetic Liked Songs collection is added. Previously saved likes remain compatible with metadata backups.

The player uses the desktop app's library layout adapted for touch: a searchable playlist sidebar on larger screens, cover and song-count header, numbered tracks with album and duration, playlist filtering/sorting, right-click options and browser back/forward controls. Sorting and filtering affect the displayed playback order without rewriting saved playlists. The compact player includes previous/next and direct lyrics/queue controls. The phone layout keeps touch targets and collapses secondary columns. Existing guest libraries and metadata backups are preserved.

Web update 0.4.60 uses the native Lucide/Spotifast SVG artwork, compact toolbar, soft buttons and Home backup controls. The desktop-sized browser player keeps its transport visible, including idle state, seek and volume/mute. Volume changes go through the official YouTube/SoundCloud player API or the browser audio element; the official visible source player remains required. Mobile devices can also use their system volume controls. No desktop binary rebuild is needed for this update.

Public YouTube Music song search (submit with Enter or Search), artist songs/albums, official visible YouTube playback, queue, previous/next, shuffle/repeat, LRCLIB lyrics with YouTube Music fallback, and timing offset. Lyrics and queue share Now Playing so the official player stays visible.

Create, rename, hide and remove browser-local playlists. Add songs through their options menu. Import public/unlisted YouTube playlists or public Spotify playlists; Spotify titles/artists/durations are matched to YouTube recordings. Private Spotify playlists and Liked Songs cannot be imported. Public YouTube imports are bounded to 500 entries; account imports to 10,000. Export/restore metadata-only JSON backups; existing collections are preserved. Browser storage is not a cross-device Vibefy account.

Optional browser-local mods: WaveLink (public SoundCloud search and official widget), JuiceVault (search and optional account playlist imports), original Nyan Cat frames and Cat Jam animation. Disabled animation mods do not run loops. Cat tempo is manual because iframe audio is unavailable to the page. Visualizers, Track Peek and Oneko are excluded. Upstream creators and asset notices remain credited.

Jam uses the existing Cloudflare relay shared with the desktop app: link/code, 24 listeners, six-hour rooms, metadata only. YouTube and public JuiceVault recordings are supported. Participants can add songs, start a queued recording and seek the current song when the host supports shared controls. The host validates recording IDs and queue positions; requests are rate limited. Autoplay may require a click. Audio and account credentials never pass through the relay.

0.4.65 places active Jam inside the normal library/search page on both phone and desktop, with a shared queue, bounded connection-label roster, exact-second seek and public song covers. The source player is nonmodal within the room; official YouTube video remains visible. Jam song menus offer shared queue additions instead of playlist writes. JuiceVault cover paths derive from validated public UUIDs when a safe source cover is absent. Lyric buttons disappear only after a confirmed miss; temporary errors keep them available.

## Optional Google connection

Set the public `VIBEFY_GOOGLE_WEB_CLIENT_ID` Vercel environment variable to a **Web application** Google OAuth client. Register each served origin, including `https://usevibefy.online` and the public `https://vibefy-neon.vercel.app` alias. The native Desktop client cannot replace this web registration. Connect YouTube in Home or the toolbar opens Google Identity Services from the click, requests basic profile and read-only YouTube access, then loads the available playlists automatically. Connected buttons open account details without requesting access again. The user chooses playlists to import. Saving a copy or editing it requests `youtube.force-ssl` separately. Popup closure, load failure and timeouts restore retry controls. Google verification, account eligibility and API quotas still apply.

Ordinary browser access tokens stay in page memory and go directly to Google APIs. Mobile 0.2.5 uses the authorization-code model: a same-origin HTTPS endpoint exchanges/renews grants with Google using VIBEFY_GOOGLE_WEB_CLIENT_SECRET in protected Vercel configuration. User tokens are never persisted on the server; mobile refresh credentials stay in iOS Keychain or Android Keystore-encrypted storage. The public google-config endpoint exposes only the client ID and renewal availability. There is no shared Google account and no native browser-session import on the website. Until configured, the UI explains that Google connection is unavailable; guest playback and public import remain available.

JuiceVault does not allow browser CORS for its API. Its narrow same-origin adapter forwards a user-submitted login to the fixed JuiceVault origin. Passwords are not saved or logged; the access token stays in page memory. Challenges and expired sessions fail visibly without bypasses. Disconnect clears the local session; downloaded playlist metadata remains local.

## Run and verify

Install `requirements.lock`, then run `python -m uvicorn app:app --host 127.0.0.1 --port 4173` from web. Run `python -m unittest discover -s tests -v` and `node --test tests/*.test.cjs`. JavaScript is buildless. The official embedded player is used in local preview too.

Vercel serves public assets from its CDN and FastAPI handles bounded public metadata requests. Source URLs, IDs, response sizes and timeouts are constrained; no arbitrary URL proxy is exposed. Rate limits are per instance, not a distributed abuse defence. Service worker caches the shell only, never API responses, tokens or audio.

No web audio downloads, guaranteed background audio, EQ, native mini window, local Whisper Auto Sync or desktop visualizers are promised. YouTube/SoundCloud embedding restrictions still apply. Never deploy `.env`, `.vercel` credentials, desktop profiles, cookies or models.

The deployment also has the controlled public alias https://vibefy-neon.vercel.app. Jam explicitly allows this alias; arbitrary Vercel sites remain blocked.


Web update 0.4.63 shows the current public, unverified Google status and remaining 100-user cap before opening consent. The existing Google project is In production, so manual tester invitations are no longer required. The site ownership marker is verified in Search Console; Google branding/sensitive-scope approval remains pending, with ownership propagation and a real consent/feature demo required. Read access loads account playlists; the broader YouTube management grant remains a separate request for explicit playlist writes. This UI disclosure does not remove Google’s warning or complete Google verification.

0.4.62 adds the optional source buttons to the shared toolbar, direct JuiceVault account controls, fixed-height aligned mod cards and a separate Cat Jam settings dialog. New browser preferences cover reduced motion, compact rows, lyric size, queue restoration and automatic next-song playback. Clear cookies & site data requires an explicit confirmation warning that local playlists will be lost, with metadata export nearby. Browser support for Clear-Site-Data varies; local storage/cache/service-worker fallbacks are also attempted, and external provider cookies/playlists are untouched.

YouTube account playlists render only on Home; successful account loading closes the small connection dialog. The browser Jam endpoint is now /api/jam on this Vercel project. The fixed-upstream WebSocket gateway uses websockets 17.4 and shares existing Cloudflare room state with desktop clients; no in-memory room database or user audio uploads are introduced. Function connections rotate before the 300-second limit and reconnect. The direct relay remains a bounded fallback. Invitations use https://vibefy-neon.vercel.app/web while the custom domain is unavailable. Updated desktop builds accept this alias and still use the shared relay directly.

SoundCloud remains public search by the user's explicit choice; it is not presented as a connected account. JuiceVault login still requires that provider to accept the account credentials. No real account credential was used in these UI checks.

0.4.63 repairs Jam actions after icon replacement, preserves the connection indicator, confirms joins and host waiting/paused states, bounds handshake attempts, and shows participant-joined toasts on desktop and browser. Verification included actual browser Host + phone-size invite/Join, shared recording and pause, plus focused state-machine regression tests.

0.4.64 adds shared queue selection/seeking, public JuiceVault audio, an engine-driven desktop host clock and an application-only draggable status window. Browser lyrics have readable timed lines, track artwork, a source badge and follow scrolling that pauses during manual scrolling. Phone layouts remove auxiliary YouTube account/import/copy buttons. YouTube still uses its official visible player; no hidden video or audio extraction is implemented. Audio-only JuiceVault playback can still be limited by the provider or browser background/autoplay policy.

During Jam, the room is the only visible playback surface. On wide screens it sits beside search and library; smaller screens stack it above the content. Volume and mute reuse the existing controls and affect this device only. Leaving a room restores the standard player controls.

0.4.67 routes JuiceVault public catalogue and optional account operations through a fixed Cloudflare HTTPS adapter, shared by the browser and legacy Vercel routes. It allows only search, username playlists, UUID playlist reading and login, with the existing origin allowlist, 30 requests/minute per connecting IP, bounded bodies and responses, 15-second upstream deadlines and no redirects. Public search metadata is cached for 60 seconds (128 keys); credentials, sessions, private playlists and audio are never cached there. Audio still loads directly from JuiceVault. Provider rate limits are respected, without an automatic retry loop. Browser search strips an optional Juice WRLD artist prefix and ranks exact/prefix title matches before other catalogue alias results.

Jam now sends explicit set_playing booleans rather than toggles. Guests show a seek immediately, protect pending playback intents from stale 1 Hz snapshots for at most six seconds, and release them on confirmation, track change, error or leaving. Progress dragging is local until commit; incoming clock updates cannot overwrite the dragged value. Cat mods select the visible progress bar and actual modal top layer, and recover after dialog changes.


## Native mobile audio (0.2.7)

Updated APK/IPA builds resolve public YouTube audio on the device, then play only a progressive M4A/AAC stream through the existing audio controls. The source recording ID is unchanged, including music-video sources. No YouTube iframe is created for these builds, no video track is requested, and no audio file or signed stream URL is saved in playlist backups. Resolution caches at most eight URLs for four minutes in native memory. Seek, pause, queue, Media Session and Jam use the same audio element. Provider restrictions still produce an error; there is no hidden-video fallback.

Android uses NewPipeExtractor v0.27.1 (GPL-3.0-or-later); the Android distribution is GPL-3.0-or-later, with corresponding wrapper source and dependency pin published alongside the release. iOS uses Alexander Eichhorn's YouTubeKit, MIT, pinned to commit 7f75a2299dc329c0b7855afb25d47663a02855cb in the project-owned fork. Original license notices ship in both mobile bundles. Neither resolver receives the user's Google tokens or browser cookies. Only the existing account bridge handles sign-in.

This native capability requires the new mobile binary. Older mobile wrappers and ordinary browsers continue to use the prior embedded provider. Pure browser audio-only YouTube playback is not activated on Vercel: its datacenter requests currently receive YouTube's bot sign-in challenge. No account cookies are exported to the server to work around this.

Phone controls (November 2026): native Play/Pause paths do not depend on the external icon sprite. The mini player exposes a seek control; drag previews stay local until release, and Nyan anchors to the visible slider in the correct dialog coordinate space. Hidden queue contents are rendered only when needed. Cat Jam is desktop-browser-only; WaveLink is no longer offered. Saved playlist metadata and account sessions are preserved.

Lyrics use LRCLIB and YouTube Music with a metadata-verified More Lyrics fallback at https://lyrics.kamiloo13.me/api/get. Provider attribution is retained and a failed or blocked provider never replaces another usable result. More Lyrics availability is external; direct validation returned HTTP 403 on 20 November 2026.

Resource update: visual mods skip snapshots/layout work while disabled or hidden; lyrics style rows only on active-line changes. Native-approved nearby invitations enter the existing validated Jam join path and fixed relay. Native apps advertise over LAN; ordinary browsers continue to join with a link or code.

## Home Screen installation

Open `/phone` (or `/web`) on a phone. The install guide appears once per seven days after dismissal, with a retry button in Settings. Android browsers exposing `beforeinstallprompt` show the real install prompt only after a user click; iOS shows Safari Share > Add to Home Screen steps. Native shells and already-standalone apps do not show the offer. Installation uses the existing manifest and mobile UI, with standalone safe-area styling. It does not provide the native audio resolver or native LAN discovery; browser playback and background limits still apply. Some browsers isolate installed-app storage, so users are told to sign in and save playlists from the Home Screen app. No account or local playlist data is cleared or migrated by the installer guide.

Search parity update: web and APK share All, Songs, Videos and Artists filters. All combines a bounded mixed catalogue request with a clips request; exact title matches rank first without rejecting other recordings. Artist cards open songs/albums through the existing artist dialog. Each category has a two-minute cache and strict submission-only search. Superseded requests cannot overwrite current results or abort a newer request through an old timeout. The phone search focuses its visible field instead of the hidden desktop field. This does not claim full native channel-search parity: arbitrary non-music YouTube channels are not yet covered by the music catalogue.

Home Screen header padding uses a higher-specificity installed-app selector plus the status-bar inset, so later responsive padding shorthands cannot erase it. The standalone flag and display-mode query are both recognized. Safari instructions identify the Share icon; Safari does not expose a programmatic Add to Home Screen prompt.
