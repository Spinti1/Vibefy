---
title: How It Connects
description: Independent Spotify grants, what is stored, and how API traffic is routed.
nav_order: 1
---

## Vibefy HTTPS and sign-in transport

## Optional accounts and playlist backups (0.4.48)

The desktop welcome screen offers Spotify, YouTube and Continue as guest. Connecting
either provider is optional; Spotify access remains necessary for private Spotify
imports and operations requiring its account. Guest playback uses the existing
YouTube engine. The mobile web beta is separate.

Home offers Create playlist, Import YouTube playlist and Import Spotify playlist.
Playlist imports copy metadata without modifying the original. Reimporting the
same URL refreshes one local copy. Spotify pagination is complete and its snapshot
is checked again before saving. The public yt-dlp YouTube import is bounded to
500 rows; authenticated Music imports retain the existing 20,000-song/page bounds.

Vibefy playlists are identified by random local IDs. They support naming, adding
songs from context menus and removing songs. The local metadata file is
state/library-playlists.json, atomically written by the existing file writer.
Existing mod-collections.json data is migrated on first use, preserving the old
file. Local storage is bounded to 200 lists and 20 MB of metadata. No audio is
included. An optional JSON export/import provides an account-independent backup.

YouTube backups require explicit selection and use the protected session. New
copies are unlisted; anyone with a link can access them. Account identity hashes
scope local backup journals under state/personal-backups. Before creating a copy,
a durable creation intent is written. Uncertain creation searches the account for
the exact description marker and stops if it cannot recover a unique copy.
Later writes verify the account, editable private/unlisted header and marker.
Song removals use server playlist-row identities. Membership/order is read before
and after reconciliation. An interrupted operation is read back before another
attempt; explicit rate-limit deadlines persist. Other failures remain visible
and require retry. The complete local list is retained when recordings cannot be
matched. Backups deduplicate recording IDs; YouTube cannot recover local-only or
unmatched songs, so file export is the complete independent backup.

On a new installation, open a Vibefy-created backup from the connected YouTube
shelf to recover its local identity and edit it again. Its description must retain
the Vibefy source marker. Existing local edits are kept when reopening that copy.
Deleting a local copy keeps the remote original and backup; the confirmation says
so explicitly. Account-specific remote deletion remains a separate operation.

The old mandatory connection/transfer screens and optional YouTube-import mod
are removed. In 0.4.49 the Linked shelf and old transfer service are also removed,
including its review state, periodic work and dual-platform mutation queues.
Existing local transfer snapshots are recovered once in a background file task,
without connecting either account or changing remote playlists. Finished copies
appear as ordinary YouTube playlists; unfinished snapshots remain local editable
Vibefy playlists. Deleted or pending-deletion entries are not restored, newer local
copies are not overwritten, and original journal files are retained. Their old
pending rename/add/delete operations are no longer executed. Spotify and YouTube
lists can now be managed independently. Explicit YouTube backups retain their
own account-scoped association and recovery safeguards.

## YouTube session window (0.4.29)

Connect YouTube opens a dedicated Windows WebView2 window on Google's first-party
sign-in page, returning to YouTube Music. It uses a separate profile inside the
Vibefy configuration folder, never another browser's cookie database. Cookies
remain in WebView2's OS-protected profile; only its random profile identifier and
account selector are retained in protected music-account storage. Do not share
or upload the profile directory. Disconnect clears cookies and all profile data.

Authenticated requests run inside the music.youtube.com origin with a fixed
endpoint allowlist, random response correlation, size limits and timeouts. No
native command objects are exposed to the page. Window navigation allows only
Google/YouTube HTTPS origins. Browser security and the stock user agent remain
enabled. New sessions require no Vibefy Google Cloud OAuth client or tester list,
but Google can still reject embedded sign-in or expire a session. This is an
unofficial integration, with no guarantee of unlimited service access.

The browser lives on its own STA thread; it is closed after 45 idle seconds.
WebView2 Runtime must be installed on Windows. Browser processes add resource
usage while active. Other platforms currently retain their previous connections;
the new sign-in window is Windows-only. Playlist additions retain explicit
rate-limit waits and conservative recovery for unconfirmed writes. New copies
remain unlisted; existing private copies are not made public.

## Regional Google sign-in redirects (0.4.30)

A local account test identified the post-2FA failure: Google's redirect through
accounts.google.ro was cancelled by the original .com-only navigation allowlist.
The embedded snapshot in assets/google-signin-domains.txt comes from Google's
https://www.google.com/supported_domains endpoint, retrieved 2026-11-02. Each
navigation must match an exact listed base domain or its dot-separated subdomain,
remain HTTPS and contain no credentials or nonstandard port. No google.* pattern
or arbitrary suffix is accepted. Authentication response messages remain limited
to the exact music.youtube.com origin. Google controls the complete 2FA flow;
Vibefy does not skip, complete or replay a challenge.

A bounded local diagnostic at config/youtube-browser/signin-diagnostic.json
records navigation hosts, event types, elapsed seconds, HTTP status and WebView2
error numbers. It excludes complete URLs, paths, queries, fragments, cookies,
passwords and account names. Blocked navigation gives an explicit error.

## Legacy official YouTube connection (0.4.28; retained sessions)

The welcome screen and Home after Spotify sign-in offer Connect YouTube directly.
It opens Google's official account chooser in the default browser. The desktop
flow binds an ephemeral 127.0.0.1 port, uses random state and PKCE S256, validates
its exact callback route, and closes on cancellation or a ten-minute timeout.
Authorization codes are exchanged once. Responses and credentials are never logged.

YouTube playlist access uses youtube.force-ssl and the official YouTube Data API.
The channel identity is read before saving a connection. Only the refresh token
and its application identity are persisted in protected music-account storage;
access tokens are refreshed under a task-local mutex and retained in memory.
Tokens are sent only to fixed Google HTTPS endpoints with redirects disabled.

Connecting from the Spotify-connected Home card explicitly explains unlisted
copies and enables sync for that account pair. Startup restoration never enables
sync for a new pair on its own. Existing browser-session connections are retained
for compatibility; the file-import/DevTools path is removed from the UI. Google
OAuth channel identities form their own account pairs; old browser-session copies
are not deleted or automatically adopted into a different identity.

Official API mutations use one video per durable intent, because the Data API
inserts one playlist item per request. A later quota failure cannot cause a
successful earlier insert in that batch to be retried. Explicit rate/quota errors
are deferred; ambiguous writes retain the existing reconciliation requirements.
Google verification, daily project quota and authorization restrictions remain.
The Data API lists the connected channel's owned playlists; it does not expose
all personalized YouTube Music recommendations or every saved third-party list.

The 0.4.29 build no longer embeds a Google client configuration. Existing
protected 0.4.28 sessions retain their own refresh configuration; the adapter
remains only for those sessions.

## Unlisted playlist copies (0.4.27)

Playlist sync is disabled by default for every Spotify/YouTube Music account pair.
Settings shows the exact pair before the user enables unlisted copies. Anyone
with the link can access a new copy; existing private copies retain their
visibility. The native
service reads YouTube's account menu to identify the active channel and scopes
its journal/inbox to SHA-256 of the two identities. It is cancelled on sign-out,
session replacement, account changes and shutdown. Proxy changes restart it with
the new transport. No passwords, cookies, refresh tokens or cross-user data are placed in the
synchronization journal.

Spotify library pages and complete song pages are read through the existing API
gateway, with the same access requirements and cooldowns. A second snapshot read
rejects a playlist changed during traversal. Unchanged snapshots skip song-page
reads. While enabled, a completed scan runs again after 15 minutes; Sync now can
request a scan but cannot skip a server rate-limit deadline. Cooldown deadlines
are persisted. Users can keep listening during initial imports and matching.

Only high-confidence catalogue audio matches are automatic: title, artist credits,
recording version and duration must agree. Alternate releases with similar scores
remain reviewable. User choices persist. A private append-only recording cache
avoids rewriting the library for each match and recovers complete records after a
torn final append. Cache data and playlist journals remain local; credentials stay
in the separate protected session store.

Music copies retain a deterministic recovery marker in their descriptions.
Before creation or append, a durable intent is saved atomically. Interrupted
creation searches for the marked copy instead of blindly creating again.
Append reconciliation compares occurrence counts, preserving intentional repeats
without replaying an acknowledged batch. The service checks unlisted/private editable
headers before adding songs. Complete per-video acknowledgements settle a batch;
otherwise the resulting playlist is read back. Unconfirmed writes stop for review.
Allow retry requires the user's explicit confirmation that they checked the remote
playlist. No automatic deletions, privacy changes or overwriting of unrelated
playlists are implemented. If a user deletes a linked remote copy or changes its
marker or makes it public, that playlist stops syncing and reports the error.

New additions to an already linked Spotify playlist are stored in an independent,
account-paired YouTube inbox before the actor is notified. Spotify's existing
outbox remains independent. Thus a Spotify rate limit does not prevent a YouTube
addition whose recording metadata is known. Unmatched songs need review. Remote
writes have no transactional guarantee across the two providers: one can finish
before the other, and each has its own status. Original source order is retained
locally; later source deletions/reorders and remote manual extras are not replayed
as destructive changes. Source episodes and local files are not transferred.

Home deduplicates linked copies against Spotify entries and local Music imports.
Opening or playing an associated Spotify playlist uses its local linked view and
resolved YouTube IDs. Rows without a chosen recording retain the previous native
resolver. Creating a remote playlist alone does not eliminate network/extraction
latency, and no instant playback or universal catalogue match is promised.

This service is local to an installation. Run one syncing installation per account
pair at a time: the unofficial API supplies no cross-device idempotency key. The
LEOPARD recovery folder is outside source/release assets; restoring 0.4.25 restores
code and the installed app without deleting remote private copies or user data.

## Optional YouTube Music and JuiceVault connections (0.4.25)

Settings offers independent music-account connections. This section describes
the legacy browser-session connection retained for existing saved sessions.
A connected Music or JuiceVault account opens the source player without Spotify.
Spotify operations still require their own Spotify sign-in and permissions.

YouTube Music uses the user's existing browser session, following the documented
[ytmusicapi browser setup](https://ytmusicapi.readthedocs.io/en/stable/setup/browser.html).
The user explicitly selects a UTF-8 request-header file or browser.json; Vibefy
never scans browser profiles, exports all browser cookies or reads passwords.
Older versions included DevTools setup steps and a Music browser link. The import
file grants session access: keep it private and delete it after connecting.
Only Cookie, the Music account selector and optional channel selector are used.
Origin is fixed to music.youtube.com; imported destinations and Authorization
headers are ignored. The SAPISID signature is renewed for each request. A dedicated
client honors the configured proxy and disables redirects. Response bodies,
session files and page traversal are bounded; network/authentication failures
are redacted and returned to the UI without logging headers or service bodies.

The saved-playlists Music shelf is read with continuation pages. Selected lists,
including private lists accessible to that session, import as Home snapshots,
retaining order and duplicate entries. This is read-only, not two-way sync.
Search remains catalogue-only; explicit personal playlist imports retain their
original recordings. Browser sessions expire or can be rejected by YouTube.
This unofficial interface can change; it is not a guarantee of unlimited access.
That legacy path does not use Google Cloud or an OAuth client. New Windows
connections use the dedicated first-party window described above.

Windows stores music-account data in user-bound DPAPI-encrypted files under
config/music-accounts, using the existing atomic private-file store. This avoids
Credential Manager's blob-size limit for browser headers. Other supported
platforms use the system keychain. Saves/deletions run off the UI thread and are
serialized even through cancellation. Startup restores once, without idle polls.
Disconnect deletes local session storage; Home snapshots are removed separately.

JuiceVault uses its documented browser handoff with a random verifier and SHA-256
challenge. The browser signs in; polling honors Retry-After. The public playlist
API uses the username entered by the listener. Private-library and refresh routes
are undocumented and are not claimed as supported. Archive playback uses validated
UUIDs, the configured proxy, bounded streams and the existing audio cache. Account
tokens are not sent to playback/artwork or to other providers. See the
[JuiceVault API documentation](https://api.juicevault.xyz/).

## Vibefy HTTPS and sign-in transport details

The shared asynchronous and blocking reqwest clients trust the OS certificate
store and bundled Mozilla public roots. This supports fresh Windows installs
with incomplete OS roots while retaining configured enterprise/proxy trust.
Certificate and hostname verification remain enabled. No Windows trust store
settings are changed, and the configured proxy is never bypassed automatically.

Spotify's token POST has a twenty-second request timeout. A connection setup
failure may retry once after 250 ms, using the same client and proxy. Timeouts,
HTTP responses and failures after sending are not explicitly retried: a one-use
OAuth code must not be replayed after an uncertain response. Errors classify
TLS certificate, DNS, proxy, timeout and connection failures without displaying
raw source strings, credentials or response bodies. This does not change OAuth
client identity, callback URI, scopes or Spotify application access rules.

## Vibefy's optional Spicy Lyrics view

The separate Spicy Lyrics button uses the official
`https://api.spicylyrics.org/v1/lyrics/{spotifyTrackId}` endpoint when a
publishable API key is configured. The key must allow native requests without
an Origin header. It is independent of the Spotify grants below. Secret keys
are rejected. Provider attribution and community uploader/maker links are
displayed with the lyrics. Successful responses, including credits, are cached
for seven days. Refresh bypasses this cache.

The native view also uses the existing Spotify transcription. A synchronized
Spicy result takes priority; a plain result does not stop fallback lookup.
Plain Spotify lyrics trigger LRCLIB lookup, including search for a compatible
timed entry when its exact album entry has text only. More Lyrics
(https://lyrics.kamiloo13.me/api/get) is tried when the previous sources
still have no timed result. Timed lyrics replace plain text; failures retain
usable plain text. The LRCLIB search deadline leaves time to return an exact
plain result before the panel timeout. Pre-fix plain caches are rechecked once
on first use; new plain answers and misses expire after six hours. Timed
answers retain their thirty-day cache. A usable timed fallback
waits at most 700 ms for Spicy before returning. No timing is fabricated:
line synchronization and word synchronization require their respective timestamps.
More Lyrics receives artist, title, album and duration only. All providers use
the shared HTTP client and proxy configuration. Requests run on the backend;
track changes and key changes invalidate older results. Local files' embedded
lyrics take precedence.

Custom lyrics backgrounds use a local file picker and backend file reads;
no image is sent to a service. Recording diagnostics use metadata already
returned by yt-dlp. Searching the recording again clears only the current
match and restarts that song, using the existing proxy and player.

The yt-dlp search and download commands share the same proxy arguments.
Off uses `--proxy ""`, System leaves discovery to yt-dlp, and manual HTTP/SOCKS
modes pass their endpoint. If every search request fails, playback reports a
search/network error rather than claiming that no compatible recording exists.

Recording selection sends Spotify title, artist names and optionally album
name as public YouTube search terms. It does not send Spotify account data or
credentials. Search metadata only creates a shortlist; full watch-page music
credits are inspected before audio download. Publisher channel evidence is a
ranking preference; correctly identified independent uploads are not excluded
because a channel badge, popularity count or rights-holder line is absent.
Search and inspection results are handled in completion order; fully inspected catalogue matches can finish before slower sibling requests. Up to twelve candidates are inspected, two at a time, with expansion beyond six
only when needed. The cache is bound to Spotify metadata and matcher evidence.
The audio format is the best available native AAC/M4A, without an artificial
bitrate/sample-rate minimum, with combined compatible MP4 as a last format fallback. Supported ordinary combined MP4 cache files lose their video through background sample copying, without audio recoding. Audio timing/edit tables are preserved. Playback receives the original bytes before compaction; an unchanged cache file is replaced atomically. Unsupported layouts retain the original. No new runtime binary is required.
There is no second provider, transcoding service or additional background model.
Unknown placeholder tracks wait for the existing Spotify Track request before
searching. Metadata arrivals update matching queue entries without changing
order, and only restart the currently waiting load. Pause/skip takes priority.

Automatic lyrics calibration runs the packaged whisper.cpp CPU executable on
already cached audio. It does not access a microphone or a transcription API,
and makes no additional network requests. Recognition and decoding run on
background workers; results are accepted only for the current recording and
request generation. Its model and runtime are distributed alongside the app. Recognized words
are cached locally by recording file identity and reused across lyric sources
and retries. Duration comparison uses existing Spotify metadata and decoded
YouTube audio duration; it does not fetch Spotify audio.

## Source songs and Spotify playlist writes

Vibefy 0.4.24 stores confirmed Spotify playlist additions in the app's state
directory (`playlist-outbox`), separately from disposable caches. The interface
remains usable while the requests run. Explicit HTTP 429 rejections retry in
the background after at least 30 seconds and never before Spotify's Retry-After
deadline, including after a restart. Requests resume only for their original
verified account while Vibefy is running. Repeated pending clicks are suppressed.
Loaded playlists retain duplicate confirmation; an unloaded playlist no longer
requires a remote duplicate scan before accepting an edit.

Success is reported only after Spotify confirms it. Authentication, permission
and exhausted-quota errors stop with an error. A lost response or an interrupted
in-flight write is not replayed automatically, because Spotify may already have
committed it. Check the playlist before deliberately submitting it again.
This queue does not remove Spotify account/application access requirements.

A source song's context menu can open **Add to Spotify playlist…**. This sends
only the displayed title and artist query to the existing signed-in Spotify
Web API session. Search runs on the backend with the normal Spotify deadline
and rate-limit handling. Suggestions are ranked locally by title, artist,
duration and version words. The user must select a returned Spotify recording
and an editable playlist before a write is requested. Query changes start a
new request generation; late answers cannot replace a newer or closed dialog.
No audio is sent to Spotify and source URIs are not used as Spotify track IDs.
Loaded-playlist duplicate checks and optimistic playlist reconciliation apply to
the selected Spotify item. Missing recordings and account
permissions can prevent the operation; the song remains available locally.

## Independent grants, once each

Spotifast uses separate credentials for Web API access and local playback:

1. **Your own Spotify app**, the personal app, carries the Web API: account,
   library, catalogue, search, playlists and playback control. Since 0.14.2
   it is the only way to sign in. The sign-in screen walks through creating
   one, and its Client ID is kept in settings
   ([Getting Started](/getting-started/#make-a-spotify-app)). Before
   opening the browser, Spotifast asks Spotify's token endpoint whether it
   knows the Client ID, with a code that cannot be valid: an unknown client
   is refused as such, a real one for the code, and the sign-in screen says
   which. Spotify's own page shows a redirect URI that does not match and
   never tells the app, so the screen waiting for the browser explains it.
   Home's **Made for you** shelf first reads the account's actual
   personalized section over the playback session. Only if that read fails
   does it try four playlist searches as an approximate fallback.
2. **A shared grant from before 0.14.2.** Earlier versions signed everyone in
   through one app, which the ncspot project registered. It stopped taking
   new sign-ins in October 2026
   ([#1057](https://github.com/crmne/spotifast/issues/1057)), and Spotifast
   no longer carries its Client ID. A grant already saved for it keeps
   working: it is stored with the ID it was issued to and refreshes with
   that. Complete playlist-library views and the playlist half of a search
   stay on it while it lasts, so Spotify-owned results are not filtered out.
   When Spotify refuses it, a personal app carries on alone; without one,
   the sign-in screen says why and shows the setup. Both Web API grants
   must verify as the same Spotify account. Spotify's desktop client
   identity cannot take its place: Spotify answers its Web API requests
   with "too many requests" from the first one.
3. **Local playback** uses
   [librespot](https://github.com/librespot-org/librespot). It needs one more
   browser approval and keeps an independent reusable credential. Spotify Premium
   is required. While it is signed in, its session also reads the playlists
   a personal app cannot: other people's, Spotify's own among them, and the
   account's own when there is no personal app. The personalized Made for you
   shelf needs this active session and a verified Web API grant, preferably
   personal. Spotify's private Home request receives the Web API access token as
   its `sp_t` context and the session's Login5 bearer and client token as
   headers. No token is logged
   or persisted for this shelf. Its response is kept in memory for ten minutes
   per account. Visible cards are also saved in the account-scoped Home cache
   for the next start and removed on sign-out. A manual refresh bypasses the
   in-memory cache but respects any
   `Retry-After` cooldown. A failed refresh keeps previously shown cards.
   If the Home read fails or playback is not set up, a Web API search looks for
   Spotify-owned playlists named Discover Weekly, Release Radar, Daily Mix,
   and daylist. The shelf keeps its **Made for you** heading, although search
   cannot recover the account's Home ordering or every personalized card.
   The same read also asks for the account's **Your top mixes** section, at
   the same time, with the same tokens, cache and cooldown; a `Retry-After`
   from either section pauses both. Search cannot find personal mixes, so
   that shelf has no fallback and keeps any cards it already shows.
   When playback becomes ready after Home, the personalized read is retried
   while existing cards stay visible. Shared rate limits can still prevent
   this fallback from loading. This
   private response can change without notice. Radio pages come only from
   this session: the Web API has no stations. Opening one resolves the
   station and reads its songs' details in one batched request.
   Play counts on album pages and an artist's popular songs come from the
   same Pathfinder queries Spotify's own album and artist pages make, with the
   session's token and an already verified Web API grant. Each album or artist
   is read once per run, and a read that fails leaves the column out.

   Vibefy replaces Listening activity with the built-in Listen Together panel. Opening it does not read a Spotify friend feed. A direct session starts only after Host or Join, exchanges metadata and timing, and closes on Leave, sign-out or exit. See [Listen Together](../VIBEFY_MODS.md#listen-together).

Local playback authorization stays separate from the Web API grants. Its
browser approval requests only the streaming permission and always shows the
consent dialog. The playback session uses the account ID verified by a
Web API grant, so it cannot sign in without one: there is no playback-only
mode. A verified personal app can complete sign-in while a shared grant's
verification is still waiting. That verification keeps retrying in the
background through rate limits without showing the top bar's "Waiting for
Spotify" spinner, since nothing on screen waits for it. A request waiting for
shared access that turns out not to exist goes to the personal app.

Web API sign-in opens the browser only after its local callback port is available. If
port 8989 is occupied, the sign-in screen keeps the error visible. Close the
app using that port, then choose **Try again**. Since 0.13.2, playback setup
does the same with its port, 8898: if another app holds it, Settings shows the
error, the browser stays closed, and a saved playback approval is kept.

Artist pages read popular tracks over the verified account's local playback
session when it is ready. The list uses that account's country, or Spotify's
worldwide (`ZZ`) ranking when its country is absent, and keeps the first ten
tracks in Spotify's order. Track details arrive in one metadata batch. If the
session is unavailable or cannot answer, the Web API remains the fallback,
including its rate limits. It goes to a shared grant from before 0.14.2
while one is signed in, and otherwise to the personal app, which Spotify may
refuse it. A different account's playback
session never supplies this list.
An explicitly empty ranking is kept. If a nonempty ranking resolves to no
tracks, the Web API gets its turn rather than showing a false empty answer.

Related artists also prefer the same-account playback session. The existing
configured HTTP client sends `queryArtistRelated` with the session's Login5
bearer and client token, without waiting on the Web API's authorization,
rate-limit cooldown or request budget. The returned cards retain their order,
names and images. A successful empty list stays empty; a missing or malformed
private response falls back to the Web API, with the same limits as popular
tracks. This uses the
same session-read deadline and does not request another grant or permission.

Background reads have separate four-request budgets for the shared Web API,
the personal Web API, and the playback session. Waiting for shared sign-in or
rate-limit recovery therefore cannot occupy the personal app's or playback
session's slots. A failed session read releases its slot before trying the Web
API. Foreground requests retain their existing transport concurrency limit.

Since 0.8.0, local playback retains the artist IDs
already supplied by librespot. Artist links in the player bar work before the
Web API's track metadata arrives, without an extra request.

Since 0.8.0, local seeks discard audio queued from the previous
position once librespot confirms the seek. This also applies when another
Spotify client controls playback on this computer. Natural track transitions
retain their queue for gapless playback. The seek still waits for librespot to
find and fetch the requested audio, and sound already handed to the device
cannot be recalled. Seeking adds no Web API request or full-track download.

Explicit track replacements and confirmed seeks also clear the equalizer's
filter history and the limiter's eight milliseconds of held audio. This keeps
a fragment of the previous song out of a replacement's silent intro. Natural
track transitions and pause/resume retain that history.

Since 0.8.0, requests that need a grant still being
verified wait for it instead of showing "not signed in". Sign-out cancels
pending requests, and their late results cannot undo a new sign-in. If Spotify
rejects a saved refresh grant, Spotifast removes that grant and asks for a new
browser approval. Upgrading to protected storage does not itself require
signing in again.

Each personal app has its own Development Mode quota; a shared grant shares
its app's quota with everyone still signed in through it. Playback state is
checked every 15
seconds, every 20 while this computer's player is active, every 30 while the
window cannot be seen and nothing plays, every 4 while the window is visible
for ten seconds after a command or a switch to another device, once more
just after a song on another device should end, and as soon as this computer's
player stops. See
[Make It Even Faster](/make-it-even-faster/#api-rate-limits).

Since 0.8.0, a search runs as two requests when a
personal app is ready and a shared grant is still signed in: songs, artists,
albums, podcasts, and episodes on the personal app, and playlists on the
shared app. This moves catalogue search off the quota Spotify divides among every user
of the shared app. Each half is shown the moment it
arrives, so a shared app waiting out a rate limit no longer holds up the songs,
and playlists appear underneath when that wait ends. A half belonging to an
earlier search is discarded rather than shown beside a newer one. A newer or cleared search cancels the previous requests, including waits
for shared access. If either half fails, the successful results remain usable
with an error for the failed part. A new query starts a fresh result set; old
songs never appear under its name. The loading indicator remains until both
parts finish. A personal app answers
with ten results for each type where the shared app answers with twenty, because
Development Mode rejects the larger page. With only one grant, one request
asks for all six types.

Since 0.8.0, verified Premium accounts using a shared grant see a
one-time introduction to their own app. Setup and dismissal are remembered in
settings. The prompt uses the existing account profile and adds no request.
When Spotify answers a shared-app Web API request with "too
many requests" (HTTP 429), those accounts also see a banner offering the same
setup, at most once a day and never again after it was closed three times.
It reads the answer the request already received and adds no request.

Since 0.8.0, explicitly sorting a Library section loads its remaining
pages through the existing Web API grant, one at a time, while loaded entries
stay visible. A failed page stops automatic loading. Spotify custom playlist
order uses the existing account-scoped rootlist from local playback; sorting
and dragging never write that order back to Spotify. When the window regains
focus, or is shown again from the tray, after every playlist page has loaded,
Spotifast reads the rootlist again over the playback session, at most once
every 5 seconds, so an order changed in another Spotify app appears without a
restart. A return sooner than that reads once the 5 seconds have passed. This
read uses no Web API quota.

Since 0.9.0, a playlist-folder and invitation-permission read requested
before local playback connects waits for that connection instead of being lost.
Sign-out discards the waiting read and any result from the previous session.

Since 0.9.0, a dropped active playback session retains its resolved
contexts and queue in memory. The replacement session restores them after
Connect registration instead of loading only the interrupted song. Unresolved
context pages continue through the new session. The snapshot contains no login
credentials, is never written to disk, and cannot be restored by another
account. The replacement engine starts at the volume being heard. This
recovery applies to unexpected disconnects; changing audio settings still
restarts the engine with its current-track pickup.

A song playing or paused when the session drops stays as it is while the
replacement session connects: the player keeps playing the audio it has
already fetched, and the new session takes that same player over and picks
the song up where it has got to, without loading it again.
Pausing or playing during the reconnect works on that song. A song that runs
out of fetched audio, or the next song, which needs a new audio key, waits
for the new session. The song plays on through failed reconnection attempts
that are retried; choosing something else, changing audio settings, signing
out, or a reconnection that gives up stops it.

## What the client stores

- Since 0.8.0, shared and personal Web API grants
  and the reusable playback credential use the platform credential store:
  Secret Service on Linux, Keychain on macOS, and Credential Manager on
  Windows, where a copy encrypted for the Windows account with the Data
  Protection API also stays in the data folder for when Credential Manager
  refuses or loses a grant. Since 0.13.4, a portable Windows copy keeps
  that data folder next to its executable, and the first time it makes
  one from the installed copy's state it saves those grants again under
  entries of its own, never moving or deleting the installed copy's
  ([portable copies](/settings-and-files/#portable-copies)). Librespot retains its reusable credential in
  memory; Spotifast owns persistence. Flatpak can talk to
  `org.freedesktop.secrets` for this.
  Version 0.7.1 still uses the older unencrypted files.
  See [migration, sign-out, and storage protection](/settings-and-files/).
- Downloaded audio and artwork, in the cache directory, within the budget
  you set.
- On Windows and macOS, desktop media controls load the cover themselves and
  are given a file, so the full-size artwork is downloaded into that cache
  when a song starts, even when no view on screen is showing it. Linux MPRIS
  carries the Spotify artwork URL for the desktop to resolve and asks for
  nothing extra.
- Lyrics, in the cache directory, for a month.
- Liked Songs metadata, scoped to the verified account, in the cache directory.
  This behavior is available since 0.8.0.
  Cached pages less than 15 minutes old need no repeat request. Older cached
  prefixes refresh through the existing Web API grant, one page at a time,
  while the saved rows remain visible. Manual refresh starts immediately.
  Like and Unlike are kept over lagging reads until Spotify confirms them.
- Spotifast has no telemetry, analytics, or hosted service. When the lyrics
  panel is open and neither Spotify nor a local file has lyrics, it sends
  the track's artist, title, album, and length to
  [lrclib.net](https://lrclib.net). It also checks
  api.github.com once a day for updates. You can turn off automatic checks in
  Settings, or request one there at any time. On macOS, **Check for Updates**
  is also in the application menu. On a connection the system reports as
  metered (Windows), the daily check, automatic downloads, and the large cover
  the media controls show wait for one that is not; a check by hand still
  runs. On macOS, the daily check and automatic downloads also wait while the
  system reports no internet connection.

  On Windows, macOS, and Linux, downloading an update fetches release metadata and
  `checksums.txt` and `checksums.txt.sig` from the project's GitHub release. It
  verifies the manifest's Ed25519 signature against its embedded publisher key
  before downloading the matching binary archive, Windows installer, or universal
  macOS DMG. Spotifast then checks the published SHA-256 digest and the portable
  executable's reported version before offering a restart.
  Automatic downloads are optional; installation always waits for your click.
  Checks and downloads do not open the update popup. The green update pill opens
  it on request; closing the popup does not cancel a download.
  No Spotify credential is sent. Missing or invalid publisher signatures are
  rejected; the updater does not fall back to trusting unsigned checksums.

  Updates stage their files in a private `.spotifast-update-*` directory beside
  the application so replacement stays on the same filesystem. The directory
  retains the previous executable or Mac app bundle and `result.txt` for recovery and diagnosis.
  Helper startup errors are retained in `helper.log`. On macOS the helper runs
  from a copy of the whole signed bundle in that directory, so replacing or
  restoring the installed app never moves the helper's own code.
  Settings, caches and credential stores are not replaced. Package-manager
  installs keep their package-manager update path. Mac updates verify the bundle
  identifier, version and code signature before replacing the whole app bundle.
  A Developer ID installation also requires the same signing team and a passing
  macOS security assessment. Apps running from a disk image or an App Translocation
  directory must be moved to a writable installation directory first.

## Collection loading and artwork

Since 0.9.0, opening a playlist, album, artist, or podcast keeps any
title, summary, and cover Spotifast already knows while the complete page is
requested. Controls that need unfinished details stay disabled, and a loading
indicator marks the unfinished content. If the request fails, the known header
stays above **Retry**. A direct link with no known details shows only the normal
loading or error state.

Each cover uses the smallest artwork URL already present in Spotify metadata
with at least a quarter more pixels than the cover takes on the display.
Spotify's 64-pixel thumbnail serves only covers of 32 pixels or less, so at
100% scaling library rows, cards and page headers use the 300-pixel URL. Of
the usual 25% scaling steps, page headers use the 640-pixel one from 125% and
cards from 175% (to the whole percent, from 114% and 163%). Page tint
extraction and softened loading previews use 64-pixel thumbnails. These URLs
remain unchanged and follow the configured proxy and normal artwork cache.
Choosing an artwork size and reusing known details add no Web API request.

A ready sharp cover stays visible until its replacement is ready. A 300-pixel
grid cover already loaded for the same item can preview a larger page cover
without another download. When only a small known thumbnail is available, a
softened preview may appear while the larger image loads. If the completed
response has no cover, the page shows its placeholder instead of an old image.

Since 0.8.0, album and playlist scrollbars can request a distant track
page through the existing session or Web API read path, without fetching all
preceding tracks. These reads run one at a time per list and retain the existing
rate-limit handling. Unloaded
rows are placeholders until their page arrives; scrolling never starts playback.

## When Spotify pushes back

[Spotify is being slow](/troubleshooting/#rate-limits) explains rate limits
for listeners.

Each Web API session has separate concurrency and rate limits. A `Retry-After`
response pauses only that session. Spotifast routes each request once and
does not retry it through the other app. A playlist read the librespot session
refuses outright, because the playlist is gone or private, is shown as such. A
dropped connection, a read that takes longer than 30 seconds, or a page whose
song details Spotify did not supply in full, hands the read to the Web API
instead of caching rows without songs. A song Spotify no longer has, or
withholds for legal reasons, is an empty row, as the Web API shows it.

While a shared grant is signed in, the playlist list and fallback Made for you
searches go through it even when a personal app is set up, because a personal
app's list leaves out Spotify's own playlists. Without one, they go to the
personal app, and the playlist list comes from the playback session's
rootlist whenever local playback is signed in, as described below. While the
Web API waits out Spotify's limits, the
sidebar and Home show what they last showed for the account, from the cache
described in [Settings & Files](/settings-and-files/).

While local playback is signed in and no shared grant can answer now,
because there is none, or it is still verifying sign-in or waiting out a
rate limit, the playlist list is read from
the playback session's rootlist instead: every saved and followed playlist,
Spotify's own among them, in one request per 500 playlists. The rootlist names
each owner by ID only, so their display names are read from their profiles
over the same session, four at a time, for at most five seconds; a name that
does not arrive in time shows as the ID. A playlist without a cover of its
own keeps the mosaic the earlier list showed for it. A list still waiting on
the shared app when local playback connects is read again this way. If the
session cannot read the list, the request waits for the shared grant, or
goes to the personal app when there is none. None of these reads use Web API quota.

Spotify can also explicitly refuse the key needed to decrypt a track. When
that happens, Spotifast stops local playback and leaves the rest of the queue
alone instead of treating every following track as unavailable. This refusal
comes from Spotify; trying again later may work. A banner says so and offers
the device picker, since other Spotify Connect devices fetch their own audio
and keys; [Spotify refused the audio key](/troubleshooting/#audio-key)
explains it for listeners. The banner reads the engine's own event and adds no request.

Before adding songs to an existing playlist, Spotifast checks the rows it
already holds. A known duplicate produces an immediate confirmation naming the
song. Only a playlist that has not been fully loaded needs a background scan to
rule out duplicates. Once confirmed, the new rows appear locally at once.
Since 0.8.0,
a drop into an open editable playlist sends its chosen insertion position
through the same Web API grant. Duplicate checks and confirmation retain that
position; partial loaded pages keep the correct continuation offset. A
successful write advances the cached playlist to Spotify's returned snapshot
instead of downloading the playlist again. If Spotify cannot answer the scan,
Spotifast preserves the requested edit and lets the write report its result.

Since 0.8.0, manually reloading an edited playlist waits for all
pending writes and confirmation of the returned Spotify revision before
requesting replacement rows. Current rows, filtering, sorting, and selection
stay visible while it loads. Automatic paging also waits for those edits.
If metadata still reports an older revision after three immediate rechecks,
or the request fails, the page keeps the edits and offers a retry. Refreshing
again retries confirmation without losing the local changes. This uses the
existing playlist requests and adds no periodic polling.

## New releases on Home

Home's **New releases** shelf is read when Home refreshes, and again when
local playback becomes ready. With local playback set up, Spotifast sends
Spotify's own new-releases feed query (`queryWhatsNewFeed`) to Pathfinder
with the session's Login5 bearer and client token, asking for the first 50
albums, EPs and singles. Spotifast never sends the feed's mutation that marks
items as seen, so the bell in Spotify's apps is left as it was. The query
uses no Web API quota.

When local playback is not set up, or the feed cannot be read, the Web API
stands in: the followed artists, 50 to a page, and for each of the first 50
of them its ten newest albums and singles, through the normal personal/shared
app routing, merged and sorted by release date. That answer costs a request
for each artist, so it is kept in memory for an hour. A rate limit, exhausted
quota or expired sign-in stops the remaining artists for that refresh; the
shelf keeps what it showed before.

The **New releases** page lists the songs of the releases on the shelf. Over
the playback session they come from two batched metadata reads, one for the
releases' track lists and one for the songs; without it, from the Web API's
album tracks, one request per release.

## Songs hidden in a playlist

Spotify's own playlists, Discover Weekly and the other mixes made for you,
offer **Hide in this playlist** in a song's menu, as Spotify's apps do. The
hidden songs live in the account's `playcontextbans` collection set on
Spotify's client API, which librespot reads and writes for Spotifast. When
local playback connects, Spotifast reads that set once, 500 items to a
page. Hiding or showing a song greys its row out or brings it back at once,
and sends one write for that song and playlist; if Spotify turns it away,
the row goes back and an error is shown. While a playlist has hidden songs,
playing it plays the songs on screen without them, as a filtered playlist
does, so Shuffle and Repeat follow the shown songs. Hiding needs local
playback; without it the menu entry is not offered.

## Podcasts on Home

Since 0.10.0, Home's **Your podcasts** shelf reads the first page of
saved shows, the same request the library's Podcasts shelf makes, if it has
not been read yet. It then asks for the five newest episodes of each of the
eight most recently saved shows, all at once within the Web API session's
concurrency limit, through the normal personal/shared app routing. Spotify includes each episode's resume point in
those answers. Play uses that position so an in-progress episode continues
rather than starting over. Shows known to be audiobooks are not asked for. These requests
run when Home refreshes: on opening Home, at most once every ten minutes, and
on a manual refresh. A rate limit, exhausted quota or expired sign-in stops the
remaining shows for that refresh; the shelf keeps what it showed before.

## Album queueing

Since 0.9.0, adding an album, single, or EP to the queue resolves
its songs before sending playback commands. A complete album already loaded
in the app needs no catalogue request. Otherwise, the existing album-track
endpoint is read in pages of 50, with the normal personal/shared app routing
and rate limits. The album URI itself is never sent to Spotify's song queue.
Local playback queues the resolved songs through librespot. On another device,
Web API queue writes run one at a time in album order. A later song waits for
the album's writes, and partial queue reads cannot remove the shown album
while those writes are pending. A rate-limited queue write waits for the full
`Retry-After` delay and retries automatically, keeping its place ahead of later
additions. This applies to individual songs too. Sign-out cancels the wait.
A permanent rejection stops the remaining album writes, reports the error, and
removes only the unaccepted additions. Songs already accepted stay queued.
Network failures and server errors are not retried automatically because an
append might already have succeeded.

## Playlist cover uploads

Since 0.9.0, **Edit details → Change cover** opens the native file
picker. Spotifast reads only the selected JPEG or PNG, preserves its aspect ratio, flattens transparent
pixels onto white, and encodes a JPEG preview. Files must be smaller than 20 MB
and no larger than 8192 pixels per side, within a 128 MB decoding budget.
Encoding reduces the image to fit
Spotify's 256 KB Base64 request limit. **Upload cover** sends that preview to
Spotify; **Save** separately saves the name, description, and visibility.

Uploads use the same shared or personal app routing as playlist edits. Requests
are not retried through another app. The uploaded image stays visible while
Spotify propagates its artwork. A changed URL can still contain an earlier
upload, so Spotifast checks the largest returned image through its normal
artwork cache, off the UI thread. Only matching image bytes or decoded pixels
replace the temporary preview. It makes at most three immediate metadata
rechecks; if Spotify is still catching up or the check fails, the preview stays
and a later page refresh can check again. Once confirmed, later cover changes
from other clients can appear. The selected source file is not copied to the
cache or settings.

Image uploads require renewed Web API consent. If Spotifast asks you to sign
in again after updating, approve the image upload permission. Reconnect your
personal app in Settings too, if you use one. Local playback authorization is
unchanged. Spotify can refuse changes to playlists you do not own.

On Linux the file picker uses a desktop portal, with Zenity as a fallback.
Install your desktop's file chooser portal or Zenity if no picker opens.

## Receivers on the local network

Spotify's device list only shows signed-in receivers. A new librespot or
spotifyd receiver is therefore invisible to the Web API.

Receivers announce themselves over mDNS as `_spotify-connect._tcp` and answer
a small HTTP interface. Opening or refreshing the picker first reads
`getInfo` to find each receiver's name and device ID. These probes run off
the UI thread, four at a time, with a two-second limit per receiver and six
seconds overall after discovery. Only responding receivers with a name and
ID are offered. Matching IDs are combined; separate devices can have the
same name. These reads send no account credential.

When a receiver is selected, Spotifast encrypts the stored librespot credential
with a receiver-specific key and a key from a Diffie-Hellman exchange. The
encrypted value only works for that receiver and exchange. Spotifast does not
save another copy of the credential.

The receiver then signs in and appears in Spotify's device list. Spotifast
uses the Web API for subsequent control requests.

### Restricted devices and Sonos

Spotify marks some devices as restricted, Sonos players among them. The Web
API refuses every player command for one with "Restriction violated" and
leaves an idle one out of the device list. Spotifast sends commands for a
restricted device over the playback session's Connect state service instead,
as Spotify's own apps do: a transfer, then player commands (play, pause,
resume, skip, seek, shuffle, repeat, queue) and volume, all sent from this
computer's Connect device to that one. This needs local playback enabled,
because the session belongs to it. Playback state is still read from the Web
API, which reports a restricted device while it plays.

Shuffle Play enables shuffle before starting the context, as it does for
Web API devices. If enabling shuffle fails, playback does not start.

Individual and album queue additions use the same Connect route, one song at
a time. A later addition waits for the whole album, repeated songs keep their
places, and a failed write stops the remaining songs. Signing out cancels
pending commands and forgets the account's restricted-device state. The
playback session must belong to the verified Web API account before it can
control another device.

A Sonos is recognised by its receiver endpoint (`/spotifyzc` on port 1400)
and is offered in the picker from the network even while Spotify does not list
it. Its receiver asks for an OAuth authorization code instead of the librespot
credential. Spotifast does not send a login blob or access token to that HTTP
endpoint. Transfer and player commands go through Spotify using the existing
playback session. Other receivers keep the encrypted login blob described above.

## Google Cast devices

Cast speakers, displays and speaker groups announce `_googlecast._tcp`, not
`_spotify-connect._tcp`. They do not run Spotify until a sender starts it.
Spotifast lists them under their Cast name (the `fn` record) without probing
them, because probing one would start Spotify on it.

The picker shows the first resolved Cast announcement immediately while its
three-second browse continues, batching subsequent changes at most every 100 ms.
It retains at most 128 Cast services and one pending UI update, ignores unchanged
advertisements, and bounds provisional rows during receiver churn.
Cast rows do not wait for the subsequent
Connect receiver probes. Reopening or refreshing the picker during that work
shares the scan. Existing rows remain available during refresh; the completed
scan removes departed devices and reconciles Connect/Cast duplicates as before.
If Connect probe setup fails, current Cast results still complete the scan and
remove stale rows.
No receiver address is saved to disk or assumed reachable on the next launch.

The active output stays first. Other outputs are ordered by how often they
occur among the last 32 explicit picker selections, most recent first when
counts match, then by name. This combines signed-in devices and receivers
waiting on the network. Only preference keys and their account ID are saved in
`session.json`; another account does not use that ranking, and sign-out clears
it. Choosing an output records the selection, including an attempt that fails;
background discovery and polling never increase its count. There is no new
setting or network request for ranking.

Signed-in outputs use their Spotify device IDs. A waiting Cast receiver uses a
separate hash of its transfer ID, address and port, so another endpoint with the
same advertised name does not inherit that ranking. No raw address is stored;
an address or port change resets this preference. Waiting and signed-in Cast
rows have separate histories. This is an ordering hint, not authenticated
device identity; authentication still happens only on selection.

Starting Spotify on a Cast device needs local playback enabled, because the
playback session obtains the token. When a Cast device is selected, Spotifast
follows the steps of Spotify's own apps:

1. It opens a Cast v2 connection to the device, checks that the device is a
   genuine Cast device (see below), and launches Spotify's Cast receiver.
2. The receiver's `getInfo` reply names the Spotify client it runs as.
3. The librespot session asks Spotify for a token issued to that client and
   to the device's ID (`/device-auth/v1/refresh`, an internal endpoint).
4. Spotifast sends that token to the receiver with `addUser`.

The device ID is the MD5 hash of the Cast name, as in Spotify's apps. The
device then appears in Spotify's device list, and the Web API controls it
like any other device. Right after an earlier sign-in, a receiver can
refuse the first attempt, so Spotifast tries once more. A device that does
not answer within 20 seconds is reported as not answering.

A Cast device presents a TLS certificate of its own, which system trust stores
do not hold, so the connection accepts any certificate. Since 0.13.1, before
sending anything else, Spotifast asks the device to prove it is genuine, as
Chrome does. It
sends a random challenge on Cast's `deviceauth` channel. The device signs the
challenge and the certificate of this connection with its device key, and
sends that key's certificate chain. Spotifast checks that the chain leads to
one of Google's two Cast root certificates, which it carries, that the device
certificate is one for signing, and that the signature is valid. Because only
that signature vouches for the connection's certificate, the connection's
certificate must also be valid now and for no more than four more days, as
Chrome requires. If any check fails, Spotifast stops. It does not start
Spotify on the device or ask for a token, and it reports that the device could
not prove it is genuine. As in Chrome, older devices that sign with SHA-1 are
accepted, and the revocation list a device may send with its answer is not
checked (Chrome only checks it when told to require one). Spotifast 0.13.0
made no such check: whatever answered at the address of the selected device
received the token.

The token is issued for the Spotify client and device ID that the receiver
names in its `getInfo` reply, not the reusable playback credential. Spotifast
does not save or log the token.

## The engine

Playback runs on a separate runtime. Librespot maintains the Spotify Connect
session, exposes this computer as a device, receives transfers, and reports
playback state. If the session drops, it reconnects with the stored credential.
On macOS, while the system reports no route to the internet at all, the
reconnection waits for one instead of retrying. Spotifast asks the system
about the internet as a whole rather than about a Spotify host, so the check
looks up no name and sends nothing over the network.

Since 0.13.2, a playback connection that fails while the network is up, for
example with a connection reset after the computer wakes, is tried again with
the saved playback approval: after 5 and 15 seconds, 1, 2 and 5 minutes, and
then every 10 minutes until it connects. These attempts keep the error on
screen instead of flickering through "Connecting", and they stop when
Spotify rejects the approval, playback is set up again, or you sign out.
**Set up playback here** and **Try again** first reconnect with the saved
approval; the browser opens only when there is none or Spotify rejects it.

On macOS, local playback and track loading declare a user-initiated activity
so App Nap does not throttle them when another application covers Spotifast.
Pausing, stopping, or an output error releases the activity; a song playing
on through a dropped session keeps it, and remote playback does not hold it. The activity permits idle system sleep
and does not keep the display awake.

Local playback and track loading also keep the computer from
sleeping when idle, released at the same moments (#962): on macOS an
activity with idle system sleep disabled (`pmset -g assertions`), on Windows
a system power request (`powercfg /requests`), and on Linux the desktop
portal's Inhibit request for suspend, or a systemd-logind `idle` inhibitor
when there is no portal (`systemd-inhibit --list`). None of them keeps the
display on, and a sleep you ask for still happens.

Since 0.9.0, selecting this computer in the device picker asks
librespot to transfer playback from the active Connect device. Spotify supplies
the current song, position, playing or paused state, context, and queue together.
This uses the existing playback session's Connect transfer request and needs
no additional authorization. An old or unavailable Web API playback snapshot
does not cause the previous local song to restart or replace the remote queue.

The engine prepares the next two songs of the queue while one plays, so Next
starts the following song without waiting for it to load. For each, it reads
the song's metadata and fetches the first 128 KB of its file from Spotify's
CDN, about three seconds of audio at 320 kbit/s, and keeps them in memory; for
the next song it also asks for the audio key. A song already in the audio
cache is not fetched again. Nothing is wasted when the song then plays: its
load uses what was fetched. Skipping further than two songs ahead loads the
song as before, and a song skipped past before it has loaded stops loading.
These requests go through the playback session and use no Web API quota.

Audio keys are asked for one at a time, at least 200 ms apart. When Spotify
refuses one, the session waits 3 seconds before asking for any key, then 5,
then 10 after each further refusal in a row, and a key that arrives ends the
wait. The song being loaded asks again after each wait, up to three times,
before Spotifast reports the refusal; the next song's key is not prefetched
during a wait.

Local playback has one output on every platform, fastframe-audio: CoreAudio on
macOS, WASAPI on Windows, and on Linux PipeWire when it is running, otherwise a
PulseAudio server, otherwise ALSA directly. It talks only to the local sound
system. On PipeWire and PulseAudio the stream is named **Spotifast** in system
mixers (`application.name`, with the `spotifast` icon), with **Spotify
playback** as its media name and no media role, so its volume is remembered
for Spotifast rather than shared with other music players.
The same session checks releases that the Web API calls `single`, so confirmed
EPs can carry their precise label. Spotifast deduplicates these checks while
the app session is active. If the engine reconnects, an interrupted check may
be tried again; if metadata is unavailable, its label stays `Single`.

The engine discovers access points through `apresolve.spotify.com` and
connects over TCP in the resolver's preference order: port 4070 first,
falling back to 443 and 80. Only outbound connections are needed; no
inbound ports have to be open.

Each socket setup and handshake shares a five-second deadline. The pinned
librespot version can try up to six access points and retry each connection
once. Spotifast allows 75 seconds for the whole engine connection, giving
stalled endpoints more time to fall back. Resolving access points,
authentication, and other setup also use that outer deadline, so reaching
every fallback endpoint is not guaranteed.

Since 0.8.0, the access-point and Dealer TCP connectors resolve
names off the playback runtime thread and try all returned addresses. If
the preferred IPv4 or IPv6 route stalls, the other family starts after
300 ms. DNS, TCP setup and any HTTP proxy tunnel share a five-second limit.
The access-point handshake still shares its existing five-second budget;
the Dealer's WebSocket TLS verification is unchanged.

When a proxy is configured, only its name is resolved locally. The target
name is sent through CONNECT, and a proxy failure never falls back to a
direct connection. Proxy URLs and credentials are not logged by this
connector. The change adds no destination or background polling.

## Proxy

Since 0.9.0, Settings → Proxy has four modes:

- **Off**: a direct connection. Environment proxy variables are ignored.
- **System**: `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY`, and on macOS and
  Windows the OS proxy. This is the default.
- **HTTP**: a configured HTTP proxy: host, port, and optional login.
- **SOCKS5**: a configured SOCKS5 proxy: host, port, and optional login.
  Spotify hostnames are resolved by that proxy.

The mode selects the protocol. Host and port are separate fields.
These settings apply to Spotifast's requests. The external browser used
for Spotify approval keeps its own network and proxy settings.

The Web API, artwork, lyrics, update checks and downloads follow that mode. Update downloads retain their release-host redirect
restrictions and checksum verification. Local receivers on the LAN are never sent through a proxy.

Local playback can only use an unauthenticated, plaintext HTTP proxy: that is
what librespot's CONNECT client supports. Proxy login still covers catalogue
and control, but the engine then connects to Spotify directly. SOCKS5 behaves
the same way for local playback. System uses the same proxy reqwest would
(environment variables, and the OS proxy on macOS and Windows) only when it
is an unauthenticated `http://` endpoint; authenticated, `https://`, and SOCKS
system proxies are ignored by the engine. Applying a proxy choice restarts
local playback when it changes the endpoint used by the current connection,
including when the system proxy has changed since that connection started.

Off and System apply immediately. HTTP and SOCKS5 apply when you press
**Apply settings**. The same choice is on the sign-in screen, so it can be
set before the first grant. The expanded sign-in form scrolls in short windows.
Manual edits remain drafts until the backend
accepts Apply or Sign in. Saving another preference does not save those drafts.
A configuration that cannot be built produces an error without silently
switching to a direct connection. Applying a valid configuration repairs it.

On startup, network work waits for the protected proxy password to be restored.
That lookup runs on the credential worker and does not block the interface or
shutdown. The password belongs to its host, port, and username; editing any of
these fields clears it. See [password storage and migration](/settings-and-files/).

## Upcoming albums and countdown links

`spotify:prerelease:` and Spotify prerelease share links require the same-account
local playback session. The session's extended metadata kind 138 resolves the
prerelease URI to its real album URI, cover, artist and release instant. They
are different IDs. Pathfinder `getAlbum` reads the ordered track preview and
playability using Login5 and the client token through the existing configured
HTTP client. These reads do not use Web API quota. Hidden positions
remain visible, and unavailable rows never enter playback or shuffle lists.

Collection membership uses `/collection/v2/contains`. Pressing Pre-save or
Pre-saved sends one explicit `/collection/v2/write` for the resolved album URI and
then checks membership. Writes use the configured proxy with transport retries
and redirects disabled. Opening the page never changes the library. Both use
the existing session's authorization, with no new OAuth permissions or stored
credentials. Failed confirmation leaves state unknown and offers a fresh read;
it never automatically repeats a mutation. Requests have a 20-second deadline.
Sign-out cancels work, stale session/page generations are ignored, and a new
link or history navigation cannot be overwritten by an older response.

## Removing one copy of a repeated song

Removing a song from a playlist uses the Web API's `DELETE
/playlists/{id}/items`, which removes every copy of the song. When the
playlist holds the removed song more than once, Spotifast instead reads the
playlist's rows and revision over the session (`/playlist/v2/playlist/{id}`)
and sends one `/playlist/v2/playlist/{id}/changes` request through librespot
that removes the picked rows by index, based on that revision, with the
session's own authorization. Spotify refuses the change if the playlist
changed since. The request is sent once, never retried, and has a 20-second
deadline. A row that is no longer
in the playlist removes nothing; the playlist reloads and shows the error.

## Vibefy 0.4.8 library completeness

Home shows the complete playlist library already read by the existing paginated library loader. Saved source collections also appear there and are stored in mod-collections.json. Successful YouTube imports save automatically; SoundCloud search results require Save. Liked Songs continues the existing /me/tracks requests sequentially after each successful page until Spotify returns no next page. It resumes fresh partial caches and keeps the existing account, generation and rate-limit handling. No additional Spotify scope or endpoint is introduced.

## Vibefy 0.4.10 search and playback latency

Spotify search keeps eight complete successful query responses in session RAM for
two minutes. Account-data reset clears them; partial or failed responses are not
cached. Typing debounce is 180 ms. Each Web API client retains its six-request
transport limit, while ordinary calls use at most four slots so GET /search and
GET /tracks metadata can still proceed during library loading. Existing source
routing, cancellation, authentication and Retry-After cooldowns are retained.

Installed WaveLink exposes a second search beside the original. Its independent
query/results page uses the existing public SoundCloud worker and proxy policy.
YouTube import results cannot replace its retained SoundCloud results.

YouTube audio preparation reuses an inspected watch response through yt-dlp
--load-info-json on stdin. Signed URLs remain in memory; failure retains the
normal watch-URL fallback and identical native AAC format selection. Preparation
for the same URI/reference is shared between Play and prefetch. Superseded loads
are cancelled; a prefetch already resolving the newly selected song can finish
and supply the foreground cache read. This changes neither recording acceptance
rules nor the playback queue's order.

## Vibefy 0.4.11 search requests

Search shows loaded account metadata provisionally, then requests track/artist/album
results over catalogue access. Playlists/shows/episodes use the existing playlist
search route only when their filter/section is viewed, or music returns empty.
Both phases are cancellable and bound to the current query and session generation.
Successful music is cached independently of auxiliary completeness, within the
existing eight-entry/two-minute session limit. Errors/provisional metadata are
not cached as remote answers. Auxiliary failures retain the music already shown.

Interactive /search calls do not retry 429 responses. The full Retry-After is
retained in the same client's cooldown, and later searches return RateLimited
without another HTTP request while it lasts. Search HTTP requests have a six-second
timeout; backend search work has an eight-second deadline covering route, grant
and transport waits as well. Other endpoint retry behaviour is unchanged. No
additional permission, token identity, service or rate-limit bypass is introduced.

## Vibefy 0.4.12 main search and imported artwork

The main Search page now uses YouTube Music's public web metadata protocol,
independently of the Spotify gateway. It uses the configured shared HTTP client
and proxy, without Spotify grants, Google cookies or a Google account. The native
Rust implementation references sigma67/ytmusicapi's MIT-licensed protocol
documentation; no Python runtime or downloaded extension runs in the application.
This is an unofficial integration, so service/schema changes may require updates.

Typing retains the 180 ms debounce. Requests select mixed music results by default; individual
categories are requested only when selected. The current query has a cancellable
worker, six-second HTTP timeout and eight-second overall deadline. Successful
results use an eight-entry, two-minute RAM cache keyed by query and category.
Continuation is explicit and capped at 500 visible rows. YouTube HTTP 429 does
not trigger retries; its Retry-After is shared across catalogue and artwork jobs.

Songs keep their real video IDs under vibefy:youtube:, never fabricated Spotify
IDs. Clicking a result loads that exact source through the existing native queue.
Artists, albums and playlists open native Music browse responses. Saving playable
rows uses the existing mod-collections.json file and Home playlist presentation.
Spotify library/account features retain their existing gateway and limitations.

On import or when opening a saved source collection, one cancellable background
job progressively searches music artwork for public video rows. It requires
matching cleaned song title, artist, compatible version and duration (within
30 seconds when both durations are known). A match changes artwork fields only;
it never replaces the chosen video, duration, order or lyric timing. Approved
Google music CDN URLs request 600 px art. Unmatched tracks keep the uploader's
primary thumbnail. Numbered video frames are excluded from thumbnail selection.
Matched artwork is saved with the collection; already resolved music art is not
searched again. No user cache is automatically cleared.

## Vibefy 0.4.13 Search and Track Peek

Search rejects music videos, user-uploaded clips and unclassified playable rows;
only catalogued audio recordings (ATV) are playable search/browse results.
The five filters are All, Songs, Artists, Albums and Playlists. Artist cards and
song rows use one vertical scroll owner; offscreen rows do not load covers.
Already loaded Spotify artist images and unambiguous cached recording artwork
can improve presentation. This performs no Spotify request or authorization.
Existing unambiguous Spotify play counts and artist followers are shown only when already loaded, explicitly labelled as cached Spotify data. Search does not fetch counters or invent YouTube views.

Track Peek is an optional native adaptation of Grason Chan's Spotify Spice (MIT).
Hovering the player Previous/Next buttons reads only the current in-memory queue.
Preview audio is an explicit track context-menu action. It pauses the current
local track, resolves one recording through the existing engine/cache/proxy and
plays at most 30 seconds through a temporary native sink with the existing EQ.
Preparation has a 25-second timeout and is cancellable; no separate persistent
preview cache, runtime, model, browser or always-on process is added.
Stopping/finishing resumes the same original track when still appropriate;
manual playback/seek, disabling the mod, signing out and exiting cancel it.
Preview is unavailable during a Listen Together session to avoid pausing guests.
The queue, repeat/shuffle preferences and play history are not replaced by it.

The former external preset renderer, downloads, child process, shortcuts,
settings and sources have been removed. Old JSON fields are ignored by serde;
existing files, other preferences, credentials and installed mods still load.

### Vibefy 0.4.33: JuiceVault and provider shelves
Home groups linked copies once, followed by Spotify, YouTube and the enabled JuiceVault add-on library. JuiceVault account controls are in Mods below WaveLink. Native login follows the first-party app contract: a login token with its server-issued grant, username/password submission, then a verified account profile and authenticated playlists. Passwords are not persisted or printed by Debug. Tokens are protected using the existing system credential store; requests carrying credentials disable redirects and retries. Expired access tokens refresh once, without repeated login attempts. Public-profile access stores only the requested username. A private playlist import sends the protected bearer token to the exact JuiceVault API host.
JuiceVault search uses the documented archive-wide music search and ranks close title matches locally. A no-match result remains empty. The additional YouTube Music lyrics adapter uses the existing anonymous catalogue client and the exact video ID, with an eight-second deadline. It supplies plain text; existing timed sources retain priority. YTubic was a provider reference only, and no GPL implementation was copied.

## Historical linked playlist names (0.4.34; removed in 0.4.49)

Rename playlist on a Linked collection, or edit its Spotify name, queues a
per-account-pair name intent in the existing private journal. The backend checks
Spotify ownership and the YouTube copy marker before writing. Only the name is
changed: privacy, description and membership are retained. Each destination has
its own persisted acknowledgement. Retrying a name assignment after an uncertain
response is idempotent. Service cooldowns remain enforced, and pending/errors are
shown on the linked collection. Explicit renames also run when periodic transfer
is paused. Followed playlists cannot be renamed by non-owners.

Lyrics retain exact artist identity and whole-title matching after removing
presentation labels, punctuation and diacritics. Plain/missing LRCLIB answers
expire after 30 minutes; the updated matching uses a new cache namespace.
Spotify and public lyrics requests start concurrently. More Lyrics no longer
waits for both earlier providers to finish before it is polled. Timing is used
only when a provider actually supplies it.

Exact YouTube URI playback reuses the same video-ID audio cache as verified
Spotify matches, retaining compatibility with existing URI-hash cache files.
Fragment concurrency is bounded to four for downloads using fragments. Cold
playback still resolves and downloads audio before playback; this is not streaming
and cannot guarantee instant playback on slow networks.

## Audio startup race fix (0.4.35)

A restored YouTube session can initialize the local engine before Spotify /me
verification completes. EngineConnected is now routed to the same account,
generation and attempt checks used by the engine handler. The dispatcher no
longer discards a valid completion merely because Spotify is still verifying.
Successful and failed completions both release engine_busy and notify the UI.

## Music navigation and presentation (0.4.36)

Public Music artist and album pages have separate history entries. Going Back or
Forward reuses the visited page's metadata; returning to search uses its short-lived
query cache. Artist pages omit related-artist/recommendation shelves and display
catalogue songs and albums. The existing audio-only song policy still applies.

The player-bar fullscreen control and F shortcut use the active lyrics or visualizer
view. Queue remains an independent side panel. Lyrics appearance offers compact
presets; visualizer options are saved per visualizer, including brightness, frame
rate, title visibility, and Backmusic artwork/particles. Old settings files receive
compatible defaults. Mandatory upstream credits and legacy credential identifiers
remain intact.

## Direct connections and player controls (0.4.37)

Vibefy no longer offers manual, system, HTTP or SOCKS proxy settings. Legacy
proxy fields are ignored and omitted when preferences are saved. Native HTTP
clients disable proxy discovery, yt-dlp receives an explicit empty proxy argument,
and the dedicated YouTube sign-in window uses --no-proxy-server. Existing account
credentials and playlists are preserved; old protected proxy records are not read.

Lyrics and Visualizers now have separate adjacent player-bar buttons. Track Peek
shows persistent previous/upcoming song cards beside the transport, including
artwork when space allows. The previous card uses actual played metadata; the
next respects manually queued songs, repeat context and a reserved shuffle choice.
Cards perform no additional network requests during drawing. Track-menu audio
previews remain available independently.

## Cloud relay Jam (0.4.44)

Listen Together is built in and has a fixed Users button in the top bar. The old LAN listener and browser PeerJS/STUN transport are replaced by an independent shared TLS WebSocket protocol through a Cloudflare Worker and SQLite Durable Object per room. Neither app opens an inbound listening port. Participants see no peer IP addresses; Cloudflare processes them. Only exact resolved YouTube recording IDs, bounded song metadata, upcoming supported tracks and play/pause/timing are exchanged. No audio, music tokens, cookies, account names or third-party cover URLs cross the relay.

A random 12-character code grants listener access; a separate random 256-bit host credential grants control. Host credentials remain outside the invite link. Rooms allow 24 listeners and expire after six hours. Reconnection attempts are bounded. Host control, input lengths, connection/message rates, queue lengths and unauthenticated socket lifetimes are checked by the server. Normal clock ticks are checkpointed at most every 30 seconds; track, pause, seek and queue changes persist immediately. Ending/expiring the room deletes its state. This is transport encryption, not end-to-end encryption. Service quotas and browser autoplay/background/availability restrictions remain applicable.

## Confirmed playlist deletion (0.4.44)

Before 0.4.49, deleting a Linked tile queued both removal from the Spotify library (the operation Spotify exposes) and deletion of its marked YouTube copy. A durable per-account intent records each acknowledgement separately and retries after service cooldowns. Tombstones prevent stale Spotify pages from recreating the deleted copy. YouTube deletion checks an editable playlist header and, for linked copies, the pairing marker before writing. Owned ordinary YouTube playlists also offer Delete from YouTube with confirmation. Saved collections can still be removed locally. JuiceVault's public API excludes account mutations; no undocumented delete endpoint is guessed.

## Audio edition selection (0.4.44)

Candidate titles, music credits and album labels are checked for clean/censored/radio-edit editions. Explicit/uncensored labels improve the score among identified recordings, while artist, title and duration checks remain. Old matches with a known clean title and known clean linked catalogue choices are rechecked. Unlabelled censorship cannot be inferred reliably from metadata. Official-audio and Topic searches run first; a fully inspected, verified artist official-audio recording within three seconds of the expected duration can start without the remaining fallback queries. Artist videos and uncertain uploads still require the broader comparison. Queued tracks retain known metadata so the engine avoids an unnecessary metadata fetch.
