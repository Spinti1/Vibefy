# Vibefy 0.4.14 mods and add-ons

Open the shopping cart beside Search. Optional mods start uninstalled and disabled. Listen Together is built in and opens from the Users icon beside Lyrics.

The main search now uses YouTube Music, independently of optional mods. It offers
Songs, All, Albums, Artists and Playlists, native page browsing, Load more,
and Save on Home for playable results. All is the default. Music videos and unclassified playable clips are excluded from all search and browse results. Search does not need
a Spotify developer Client ID; the Spotify library connection remains separate.
The public metadata integration is unofficial and subject to service changes.

Imported playlists progressively gain matched album artwork when opened or newly
imported. Existing imports do not need reimporting. Only sufficiently confident
title/artist/version/duration matches replace artwork; source videos and timing
remain unchanged. Otherwise Vibefy uses the primary uploader thumbnail. Some
videos have no corresponding album cover available in YouTube Music.
Install enables the bundled native implementation; Disable keeps its preferences,
and Uninstall turns it off. No downloaded JavaScript or browser runtime is run.
The catalogue is one continuous responsive grid, with one, two or three columns. Show all clears the filter. Spicy Lyrics and More Lyrics are also listed as included integrations. Source/import tools come first, then Listen Together, fullscreen visuals and cat animations. There are no category tabs. Each card shows availability/enabled status. Expand an enabled import/search/Jam tool to use it. Use the filter to find a mod. Source and credit links are on every adapted card.

| Mod | Native functionality | Credit / differences |
| --- | --- | --- |
| Nyan Cat Progress Bar | Rainbow seek progress with the six original Nyan frames, one animation cycle per estimated beat; seeking still works normally | Original PNGs from NoahClemmensen/spicetify-nyan-bar-synced. Native playback-position animation. |
| WaveLink | Search public SoundCloud uploads, play and queue exact sources, save results as a local collection | Inspired by xoft122/WaveLink (MIT). Public, unprotected audio only; no account login, likes, private tracks or SoundCloud library API. DRM and unavailable sources are reported explicitly. |
| Oneko Cat | Cat follows the pointer, changes directional sprites and sleeps while idle | kyrie25 / adryd325, MIT. Original classic sprite sheet, converted losslessly to PNG; native movement code. |
| Cat Jam Synced | Original animated cat changes speed with locally estimated song BPM | Original WebM from BlafKing/spicetify-cat-jam-synced, converted to a transparent PNG atlas. Playback rate is track BPM / 135.48, matching the upstream animation baseline. |
| YouTube Playlist Import | Import a public playlist into a local collection, preserve order and duplicates, start at any row | Vibefy implementation using the packaged yt-dlp. Up to 500 entries per import and 30 saved collections; unavailable/private/live entries are omitted when identifiable. No Spotify playlist is created. |
| vivy | Native ridgeline history and drifting ribbons, intensity and fullscreen controls | Inspired by wairence/vivy (MIT). Uses live post-EQ, pre-volume audio; no Spotify advance analysis, 3D engine or mini window. |
| backmusic | Live rings/spectrum, album artwork, particles and intensity controls | Independent implementation inspired by rolandyangg/backmusic_extension. Does not copy its unlicensed implementation. |
| Track Peek | Hover Previous/Next for adjacent-track cards; right-click a song to preview up to 30 seconds without replacing the queue | Native adaptation inspired by Grason Chan / Spotify Spice (MIT). Uses the existing audio engine/cache; temporarily pauses and resumes the main local track. Unavailable during Listen Together. |

## Source collections

YouTube import accepts HTTPS youtube.com playlist URLs. Imported videos play
directly, without Spotify matching. WaveLink searches public SoundCloud uploads.
Public search results can still be DRM-protected or region restricted. Vibefy
does not bypass those restrictions. AAC/MP3-compatible sources use the existing
decoder and equalizer. A cold download is reused from the normal audio cache.
Add-on audio downloads are limited to 128 MB per track and time out after 90 seconds.

A successful YouTube import is saved automatically as a Home playlist. Reimporting the same URL updates its saved entries. SoundCloud results are saved explicitly with Save local collection. Every saved source playlist appears on Home alongside all Spotify library playlists, without the former eight-tile limit. Open a source playlist there to browse all its tracks, play from any row or remove its local copy. Home and source playlist rows are virtualized. Collections
are stored separately in `mod-collections.json` in Vibefy's data directory and
survive Disable, Uninstall and Clear all cache. Remove collection deletes only
the selected local collection. It never edits the source or Spotify library.
Browsing and source downloads use the application's applied proxy policy.
Cancelling a request or disabling the source add-on aborts its metadata worker.

## Fullscreen visualizers

Install vivy or backmusic. The fullscreen button
appears immediately to the right of volume. Esc or Exit fullscreen restores the
previous fullscreen state; playback controls remain along the bottom.
The top-left selector lists installed visualizers. Style, intensity and backmusic
particles are adjusted there under Visualizer settings, with independent preferences
for each visualizer. The Mods catalogue only installs, enables or removes them. Visuals
use the existing audio tap and pause their animation when audio is paused.
Rendering is capped at 30 fps, subject to the existing power policy.

## Listen Together

Both users need Vibefy. Open Listen Together with the Users icon beside Lyrics. It uses a compact floating window when a docked panel would crowd the page. Choose Host a session and copy
the invitation to the other users. The host application opens a temporary TCP
port; it closes on Leave session, sign-out or
exit. On Windows, allow Vibefy on private networks if the firewall asks.

Listeners paste the invitation into Join session. Use the same local network,
or a common VPN. The host can override its detected IPv4 with its VPN/other
adapter address before copying the invitation. Ordinary internet connections
behind NAT need a VPN or explicit router/firewall configuration. No public
server has been deployed or promised. Direct TCP is intended for trusted LANs;
use an encrypted VPN for communication outside the home network.

Only track metadata and playback timing are sent. Every participant downloads
their own audio. For Spotify songs, the host's resolved YouTube recording is
shared so listeners use the same video. The host controls the session; guests
do not control the host queue. Position updates are sent about once a second,
compensate for round-trip latency and correct drift above 350 ms, with a two
second seek cooldown. This is not a sample-accurate audio distribution system.
Cold downloads and unavailable sources can delay an individual listener.

## Resource use

Disabled mods do not start animation workers. Merely opening Listen Together does not start a session. Oneko redraws while moving
and slows down while idle. Nyan and Cat Jam use the original bundled artwork
and animation clocks based on playback position. A cancellable background task
estimates BPM from at most 90 seconds of already cached audio, reusing the native
decoder. Results are cached in memory for up to 256 recordings. No network,
microphone or ASR model is used for this task. Ambiguous music can produce
half/double-time estimates; weak signals keep the original animation speed.
Fullscreen modes use the existing audio tap. Jam starts one bounded session
thread only when hosting/joining. Source work runs off the UI thread. Visible
collection rows are virtualized; whole collections are copied only when a play
is requested. Runtime resource use depends on enabled mods and the computer.

MIT notices are in `assets/mods/`. The distributed `MOD_CREDITS.txt` includes the
Oneko sprite notice and additional source credits. Original cat artwork sources,
Git tree hashes, conversion details and SHA256 hashes are in `assets/mods/README.md`.
The Vibefy About card links
to https://buymeacoffee.com/spinti.

## Liked Songs

Opening Liked Songs resumes a fresh partial cache or refreshes an old cache, then reads every remaining Spotify page in order (up to 50 songs per request). Scrolling and sorting are no longer required to load the rest. The existing account and request generation guards, optimistic edits, API rate-limit handling and account-scoped disk cache are retained. Requests stop after an error; the existing retry action remains available. Cached rows remain usable while loading. Tracks unavailable from Spotify cannot be manufactured by the client.

## Source playlist appearance

Saved YouTube and SoundCloud playlists reuse the same playlist hero, cover rendering,
artwork tint, large Play/Pause and Shuffle controls, filter, sortable headings and
virtualized track table as Spotify playlists. Track rows show the original video
thumbnail, title, uploader/artist and duration. Missing album/date-added metadata is
not fabricated. The More menu opens a public source playlist or removes the local copy.
Source tracks offer native queue/copy-link actions, without Spotify-library save buttons.
New imports retain the playlist's supplied cover and owner. Older imports use an
existing video thumbnail as the hero cover and continue to load without reimporting.
Only existing artwork URLs and the existing asynchronous artwork cache are used.
Table rows are cached until the saved collections change; filtering, sorting and
reimports invalidate their view correctly. Playback uses the exact imported video IDs,
retains duplicates and displayed order, and keeps native shuffle/repeat modes.
## SoundCloud search and faster preparation (0.4.10)

Installing WaveLink reveals **SoundCloud search** immediately to the right of the
main search field. It opens an independent page with its own query, Enter/Search,
Cancel, playback and Save on Home. Disabled WaveLink can be enabled on that page;
uninstalling it removes the top-bar button. In narrow windows both search controls
move together onto a second toolbar row, retaining their order and full labels.
Spotify searches and SoundCloud results never replace each other's query/results.
A YouTube import cannot overwrite the dedicated SoundCloud results.

Spotify searches begin after a 180 ms typing pause. Up to eight complete successful
queries remain in RAM for two minutes, within the current account session only.
Partial results/errors are not cached. Sign-out/account-data reset clears this cache.
The existing six-request transport budget is unchanged; ordinary requests use at
most four slots, leaving capacity for search and selected-track metadata. API
cooldowns and authorization still apply. No extra endpoints or permissions are used.

Verified YouTube recordings reuse their inspected metadata/format list via yt-dlp's
--load-info-json stdin option instead of extracting the same watch page again.
Metadata and signed URLs stay in memory. The normal watch-URL path remains a
fallback, with the same audio format and matching rules. Current playback and
next-track prefetch share one preparation per recording; superseded work is
cancelled, and prefetch checks cached file metadata instead of reading the audio.
These changes reduce avoidable work; uncached tracks still depend on external
search, verification and download speed. No universal start-time guarantee applies.
## Search and Windows cache fixes (0.4.11)

Main search matches already loaded liked songs, playlist/album pages, track metadata
and the previous search while Spotify responds. These provisional rows retain their
original Spotify identity and are labelled Known tracks. A failed remote request
keeps them usable; provisional/error results never enter the query cache.
The first Spotify request asks for tracks, artists and albums. Playlist/podcast
categories load when their filter is selected or their section becomes visible;
an empty music result also requests them. Successful music responses have their
own completeness marker in the same eight-entry, two-minute session cache.
Auxiliary failures cannot discard music results, and stale query generations are
ignored. Choosing Songs avoids requests for unseen auxiliary categories.

An HTTP 429 retains Spotify's full Retry-After cooldown and returns promptly instead
of retrying the interactive query repeatedly. Requests during a known cooldown do
not hit Spotify. Search HTTP calls use a six-second timeout and the whole backend
search operation has an eight-second deadline, including grant/transport waits.
Retry search starts a fresh attempt using the existing credentials and proxy.
External service latency and limits still affect tracks absent from loaded metadata.

Windows can virtualize cache children into an MSIX package's LocalCache while
canonicalizing their logical parent to the original AppData directory. Cache walking
therefore keeps the logical Windows boundary, joins only one validated child name
at a time and rejects symlinks/reparse points at the root and every child. Other
platforms retain their canonical boundary check. This fixes false Cache path leaves
its root warnings and restores size accounting/clear operations. Repeated identical
warnings from genuinely inaccessible files are deduplicated. No user cache is
cleared automatically by this update.

### Spotify application identity and public distribution

The intended public sign-in experience uses a single Vibefy-owned public Client ID with PKCE, while each listener authorizes their own account. Listeners should not have to register developer apps. No Vibefy-owned ID has been configured yet; developer setup remains optional and is not presented as a way to remove Spotify's limits.

Spotify currently limits newly created development apps to five allowlisted users and requires the app owner to have Premium. Wider distribution requires Spotify's extended-quota approval, with separate organization eligibility requirements. Creating an ID alone does not grant public access or eliminate rate limits. See [Spotify quota modes](https://developer.spotify.com/documentation/web-api/concepts/quota-modes).


Cat Jam defaults to 90% of its original 112 × 86 size. After installing it, use Size (50–200%) or Reset size on its card. This setting also remains available while the installed mod is disabled. The original transparent frames touch the left window edge and the top of the playback bar.
