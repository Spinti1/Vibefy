# Vibefy credits

Vibefy is developed by **Spinti** as an independent fork of [Spotifast](https://github.com/crmne/spotifast) by **Carmine Paolino and contributors**. The native interface, architecture and player foundation originate in Spotifast. The original MIT copyright notice is preserved in [LICENSE](LICENSE). Thank you to the upstream project and every contributor.

## Foundation and services

| Project | Contribution / attribution |
| --- | --- |
| [Rust](https://www.rust-lang.org/), [egui / eframe](https://github.com/emilk/egui) | Native application and immediate-mode interface; includes the pinned crmne app forks |
| [fastframe](https://github.com/crmne/fastframe) | Native platform, storage, UI and safe update installation foundation |
| [librespot](https://github.com/librespot-org/librespot) | Spotify session and playback integration |
| [Symphonia](https://github.com/pdeljanov/Symphonia), [CPAL](https://github.com/RustAudio/cpal) | Audio decoding and device output |
| [yt-dlp](https://github.com/yt-dlp/yt-dlp) | Public source resolution and playlist extraction; upstream license and bundled component exceptions apply |
| [whisper.cpp / ggml](https://github.com/ggml-org/whisper.cpp), [OpenAI Whisper](https://github.com/openai/whisper) | Local desktop recognition and model used for recording alignment; included MIT notices |
| [ytmusicapi](https://github.com/sigma67/ytmusicapi) by sigma67 | Reference for native public music catalogue requests; Python library in the web API, MIT |
| [Spicy Lyrics](https://github.com/Spikerko/spicy-lyrics) by Spikerko | Lyrics experience and provider integration adapted for native Vibefy; not a complete Spicetify runtime |
| [More Lyrics](https://github.com/Kamiloo13/spicetify-extensions/tree/main/extensions/more-lyrics) by Kamiloo13 | Additional lyric provider integration / inspiration |
| [LRCLIB](https://lrclib.net/) and credited lyric providers | Timed and plain lyrics where available; lyrics remain the property of their rights holders |
| [FastAPI](https://github.com/fastapi/fastapi), [Starlette](https://github.com/Kludex/starlette), [HTTPX](https://github.com/encode/httpx), [Uvicorn](https://github.com/Kludex/uvicorn) | Mobile web API |
| [Inno Setup](https://jrsoftware.org/isinfo.php) | Windows setup creation, not an app runtime dependency |

## Mods and inspirations

These are native adaptations, not the original extensions running inside Spicetify. Features can differ from their originals. Listen Together is included in Vibefy.

| Creator / project | Vibefy use |
| --- | --- |
| [NoahClemmensen / Nyan Bar Synced](https://github.com/NoahClemmensen/spicetify-nyan-bar-synced) | Six original Nyan PNG frames, preserved byte for byte; native BPM-based progress animation |
| [kyrie25 / Oneko](https://github.com/kyrie25/spicetify-oneko), adryd325 | Original classic sprite sheet, MIT notice retained |
| [BlafKing / Cat Jam Synced](https://github.com/BlafKing/spicetify-cat-jam-synced) | Original transparent animation converted to a native PNG atlas, estimated BPM playback |
| [xoft122 / WaveLink](https://github.com/xoft122/wavelink) | Native public SoundCloud search / playback adaptation; MIT notice retained |
| [wairence / Vivy](https://github.com/wairence/vivy) | Independent live audio visual adaptation; MIT notice retained |
| [rolandyangg / Backmusic](https://github.com/rolandyangg/backmusic_extension) | Visual inspiration for independent spectrum / rings / particles implementation; unlicensed extension code was not copied |
| [Grason Chan / Spotify Spice](https://github.com/grasonchan/spotify-spice) | Track Peek inspiration / native adaptation only, no Vinyl; MIT notice retained |
| [Kyzenkms / Spicetify Jam](https://github.com/Kyzenkms/spicetify-jam) | Inspiration for independent direct LAN/VPN metadata-and-timing protocol; no copied PeerJS service or Spotify Jam implementation |
| [Spicetify contributors](https://github.com/spicetify) | Original extension ecosystem and marketplace references |

The inspected Nyan and Cat Jam repositories do not contain explicit artwork license notices. **This project's MIT license does not relicense that artwork.** Source revisions, original URLs, conversion details and hashes are recorded in [assets/mods/README.md](assets/mods/README.md). Other mod notices are retained under `assets/mods/LICENSE_*.txt` and in the distribution's third-party notices.

## Fonts, icons and other dependencies

The website uses **DM Sans** (Colophon Foundry and contributors) and **Space Grotesk** (Florian Karsten), under the SIL Open Font License, served by Google Fonts. Native fonts, icon sets and additional asset notices are retained under `assets/` and the dependency notices. Vibefy's V logo and application branding are separate from the original Spotifast branding.

[THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt) collects the source checkout's dependency and bundled asset notices. It is not a claim that every dependency is used on every target. Dependency versions and source revisions are pinned in Cargo.lock and the web requirements lockfile. External service trademarks, cover art and lyrics belong to their respective owners.

Vibefy is not affiliated with, endorsed by, or an official client of Spotify, YouTube, Google, SoundCloud or Spicetify. Only access content you are permitted to use; availability remains subject to each provider.

## Browser Listen Together

The web implementation uses [PeerJS 1.5.5](https://github.com/peers/peerjs) under MIT, with the original notice retained in web/public/vendor/peerjs-LICENSE.txt. PeerJS Cloud provides signalling and Google STUN assists direct encrypted WebRTC connections. This is a Vibefy web implementation, separate from the native LAN protocol. Audio is played independently by each participant.
