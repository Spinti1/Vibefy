# Vibefy

**Your music, your kind of player.**

Vibefy is an independent native music player built on [Spotifast](https://github.com/crmne/spotifast). Enjoy synced lyrics, a classic mini player, an equalizer and optional mods in one desktop app.

[Website](https://usevibefy.online) · [Download for Windows](https://github.com/Spinti1/Vibefy/releases/latest) · [Web player beta](https://usevibefy.online/app) · [Credits](CREDITS.md)

## What's inside

- Native Rust / egui interface and Spotify library integration inherited from Spotifast.
- Audio-only music search, public source playback, SoundCloud search through WaveLink, and saved playlist imports.
- Spicy Lyrics / More Lyrics integrations, multiple lyric sources and local recording alignment where supported.
- Working native EQ, fullscreen audio visuals and a Vibefy classic mini player.
- Optional Nyan Cat, Oneko, Cat Jam, Vivy, Backmusic and Track Peek adaptations.
- Built-in Listen Together for trusted LANs or VPNs. This is an independent protocol, not Spotify Jam.
- Source songs can be matched to a Spotify recording and added to an editable Spotify playlist after confirmation.
- Cache controls, per-user Vibefy storage, and GitHub release updates.

## Install

Get the latest **Windows x64 Setup** from [Releases](https://github.com/Spinti1/Vibefy/releases/latest). It includes the public-source helper and local speech model required for desktop Auto Sync. Model files make the installer larger than the app executable alone.

Installers are **not code-signed**. Checks use GitHub HTTPS. Update downloads must match the release's published size and SHA-256 digest, as well as checksums.txt. Installation saves your layout, closes Vibefy safely, and starts Setup in the current installation folder. First install using Setup to register that folder for future updates. Never install a file from an unrelated mirror.

Open the profile menu and choose **Check for updates**. A newer release opens the download and restart dialog; otherwise Vibefy reports that it is up to date. Spotify recording choices for adding source songs to playlists display square album covers. Main windows restored below the minimum usable size recover their saved size at startup.

Spotify account features remain subject to Spotify authorization, account requirements and service limits. Public sources can be unavailable or change. Lyric timing cannot be guaranteed for every recording, edit or live version. Optional mods can increase CPU and memory usage.

The native engine's title, artist, version, duration and source-ranking rules are documented in [Recording selection](docs/_reference/vibefy-recording-selection.md).

Version 0.4.19 accepts closely similar recording titles, with exact titles ranked higher. The audio format row shows automatic AAC/M4A playback and its MP4 fallback. File size depends mainly on bitrate and duration; MP3 conversion is not used.

Version 0.4.20 shares lyrics data between frames instead of copying timed words, reuses visualizer buffers, and records visual history only for Vivy. Release builds use full LTO and Setup uses stronger compression. The complete Auto Sync model and runtime remain included; network-dependent search and playback times still vary.

Version 0.4.24 queues Spotify playlist additions in the background and saves pending requests per account. Temporary rate limits retry automatically, respecting Spotify's waiting period across restarts. Duplicate pending clicks are suppressed. Uncertain writes are not replayed automatically; permission, authentication and exhausted-quota errors remain visible. Spotify access requirements still apply. Windows installers remain unsigned.

Version 0.4.22 prefers available synchronized lyrics over plain results from another provider, including timed LRCLIB entries on another album. Plain text remains available when no matching timing exists. Older plain caches are rechecked once after this update; new plain answers and misses are retried after six hours. This does not invent word timing for songs with text only.

Version 0.4.21 processes search and inspection results as they arrive, allowing fully verified catalogue recordings to start before slower sibling requests finish. Supported combined AAC MP4 cache files are compacted to audio-only in the background without recoding samples or changing timing. Existing files are handled when accessed. Unsupported layouts keep their original cache file; playback does not wait for compaction.

## Mobile web beta

On desktop and laptop, the [website](https://usevibefy.online) presents the native app and downloads. On phones it opens the mobile player with search, queue, local collections, public playlist import, LRCLIB lyrics and official YouTube playback. Optional browser adaptations include Nyan Cat, Oneko, Cat Jam (manual tempo) and Track Peek. Browser Jam connects phone users through WebRTC; it is separate from native LAN sessions.

Spotify PKCE sign-in and paginated account-library views are implemented but **not activated on the public deployment**. They require Vibefy's registered Spotify Client ID and Spotify access approval. A development app permits only allowlisted users; a single Client ID does not grant public access. No end-user Client ID is requested. Selecting a Spotify song searches for a separate playable recording. The official YouTube player does not expose audio for EQ, automatic BPM detection or reactive visualizers, and cannot guarantee background playback. See [web/README.md](web/README.md).

## Build

Use the pinned Rust toolchain and Cargo.lock. The internal crate and build output are still named spotifast for compatibility; the distributed app is vibefy.exe.

```sh
cargo build --release --locked -j 2
cargo test --release --locked -j 2 --test update_integrity --test update_launch
```

Windows packaging is in [packaging/vibefy](packaging/vibefy). Web setup is in [web](web). Historical upstream packaging documents describe Spotifast and are retained for reference, not as Vibefy download instructions.

## Credits and license

By **Spinti**, based on **Spotifast by Carmine Paolino and contributors**. Full project, mod, service and asset credits are in [CREDITS.md](CREDITS.md); dependency notices are in [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt). The original [MIT license](LICENSE) is preserved. Third-party artwork, fonts, lyrics and bundled dependencies retain their own terms; the MIT license does not relicense them.

Not affiliated with Spotify, YouTube, SoundCloud or Spicetify. Use only content you are permitted to access.

[Report a bug](https://github.com/Spinti1/Vibefy/issues) · [Security](SECURITY.md) · [Support Spinti](https://buymeacoffee.com/spinti)
