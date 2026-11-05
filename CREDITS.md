# Vibefy credits

Vibefy is developed by **Spinti** as an independent fork of [Spotifast](https://github.com/crmne/spotifast) by **Carmine Paolino and contributors**. The native interface, architecture and player foundation originate in Spotifast. The original MIT copyright notice is preserved in [LICENSE](LICENSE). Thank you to the upstream project and every contributor.

## Foundation and services

| Project | Contribution / attribution |
| --- | --- |
| [Google Identity / OAuth](https://developers.google.com/identity/protocols/oauth2/native-app) and [YouTube Data API](https://developers.google.com/youtube/v3) | Legacy 0.4.28 browser authorization and playlist metadata/writes, retained for existing protected sessions. Google and YouTube services remain subject to their own terms, verification and quotas. |
| [sigma67 / ytmusicapi](https://github.com/sigma67/ytmusicapi) | Reference for browser-session signing and YouTube Music library browse/continuation requests; independent native Rust implementation. MIT notice retained under assets/licenses. No Python runtime is bundled. New Windows connections use a dedicated first-party WebView2 profile; older stored sessions remain readable. |
| [JuiceVault](https://juicevault.xyz/) and its [documented API](https://api.juicevault.xyz/) | Browser login handoff, public playlist metadata, artwork and archive audio endpoints; service content remains with its respective rights holders. No JuiceVault application source code is bundled. |
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
| [CaYatur / SoundVisualizer](https://github.com/CaYatur/SoundVisualizer) | 12 adapted 2D effect geometries and audio mappings by Çağan Turgut / CaYaDev, revision c3e7fc6bc3fa826e323486256268fa764a317963. MIT notice preserved in assets/mods/LICENSE-SoundVisualizer.txt and the optional package. Canvas bloom, GPU and MilkDrop modes are not included. |
| [Kyzenkms / Spicetify Jam](https://github.com/Kyzenkms/spicetify-jam) | Inspiration for independent shared listening; current transport uses a Vibefy Cloudflare relay, not Spotify Jam |
| [Spicetify contributors](https://github.com/spicetify) | Original extension ecosystem and marketplace references |

The inspected Nyan and Cat Jam repositories do not contain explicit artwork license notices. **This project's MIT license does not relicense that artwork.** Source revisions, original URLs, conversion details and hashes are recorded in [assets/mods/README.md](assets/mods/README.md). Other mod notices are retained under `assets/mods/LICENSE_*.txt` and in the distribution's third-party notices.

## Fonts, icons and other dependencies

The website uses **DM Sans** (Colophon Foundry and contributors) and **Space Grotesk** (Florian Karsten), under the SIL Open Font License, served by Google Fonts. Native fonts, icon sets and additional asset notices are retained under `assets/` and the dependency notices. Vibefy's V logo and application branding are separate from the original Spotifast branding.

[THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt) collects the source checkout's dependency and bundled asset notices. It is not a claim that every dependency is used on every target. Dependency versions and source revisions are pinned in Cargo.lock and the web requirements lockfile. External service trademarks, cover art and lyrics belong to their respective owners.

Vibefy is not affiliated with, endorsed by, or an official client of Spotify, YouTube, Google, SoundCloud or Spicetify. Only access content you are permitted to use; availability remains subject to each provider.

## Listen Together

[Cloudflare Workers and Durable Objects](https://developers.cloudflare.com/durable-objects/) host the independently implemented room relay. [tungstenite](https://github.com/snapview/tungstenite-rs) provides the native TLS WebSocket client (MIT / Apache-2.0). Desktop and mobile web share one metadata-and-timing protocol. Audio is played independently by each participant. Cloudflare processes connection IP addresses; participants receive no peer IP addresses. No Spotify or YouTube credentials are sent to the relay.

## Mobile mod adaptations

The mobile player reuses the original credited Nyan frames, Oneko sprite sheet and Cat Jam animation atlas already included in the native distribution. Browser motion and controls are independently implemented. Track Peek adapts the next-track concept credited above. Cat Jam tempo is manually adjustable in the browser because official embedded playback does not expose an audio signal. Original asset terms remain unchanged.

## Classic mini player and website demo

The classic mini player is inherited from Spotifast by Carmine Paolino and contributors. Vibefy changes its branding, colors and return control; it does not claim the original mini-player implementation as new work. The desktop website screenshot is rendered from the actual application in isolated demo mode, with sample account and playlist metadata. Demo artwork is served by [Lorem Picsum](https://picsum.photos/). No personal profile or private playlist is used.

## HTTPS trust roots

[webpki-roots](https://github.com/rustls/webpki-roots) supplies bundled Mozilla public CA roots under CDLA-Permissive-2.0, alongside the operating system trust store. Its original license is retained in THIRD_PARTY_LICENSES.txt. HTTPS certificate and hostname verification remain enabled.

## YouTube connection window

[YTubic by NUber-dev and contributors](https://github.com/NUber-dev/YTubic) was reviewed as a functional reference for first-party YouTube sign-in. Its GPL-3.0 application code was not copied or translated into Vibefy. Vibefy independently uses [Microsoft Edge WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) and the MIT-licensed [webview2-com bindings](https://github.com/wravery/webview2-rs). The WebView2 Runtime is supplied by Microsoft under its own terms. Existing dependency and asset licenses continue to apply.

### JuiceVault and additional lyrics
JuiceVault (https://juicevault.xyz/) provides archive metadata, playback and account libraries through its API (https://api.juicevault.xyz/). Vibefy implements an independent Rust adapter; no JuiceVault extension code is bundled. YTubic (https://github.com/NUber-dev/YTubic, GPL-3.0) was consulted to identify its lyrics providers; no YTubic implementation is copied or translated. The additional YouTube Music lookup uses Vibefy’s existing catalogue client. Lyrics retain their provider attribution.

### Wasmi

Optional in-app WebAssembly mod runtime: [Wasmi](https://github.com/wasmi-labs/wasmi), dual MIT/Apache-2.0. Modules receive bounded playback/drawing interfaces, not account credentials.
