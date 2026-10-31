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
- Cache controls, per-user Vibefy storage, and GitHub release updates.

## Install

Get the latest **Windows x64 Setup** from [Releases](https://github.com/Spinti1/Vibefy/releases/latest). It includes the public-source helper and local speech model required for desktop Auto Sync. Model files make the installer larger than the app executable alone.

Installers are **not code-signed**. Checks use GitHub HTTPS. Update downloads must match the release's published size and SHA-256 digest, as well as checksums.txt. Installation saves your layout, closes Vibefy safely, and starts Setup in the current installation folder. First install using Setup to register that folder for future updates. Never install a file from an unrelated mirror.

Spotify account features remain subject to Spotify authorization, account requirements and service limits. Public sources can be unavailable or change. Lyric timing cannot be guaranteed for every recording, edit or live version. Optional mods can increase CPU and memory usage.

## Mobile web beta

Open the [web player](https://usevibefy.online/app) and optionally add it to your Home Screen. The browser implementation provides music search, queue, local saved collections, public playlist import, LRCLIB lyrics and official YouTube playback controls. It is a **beta with a separate browser-local library**, not full native feature parity. Web Listen Together supports direct encrypted sessions between browser users on PC and phone. These sessions are separate from the native desktop LAN protocol. Native Spotify sign-in, local Whisper Auto Sync and desktop mods still require further browser/server adaptation. Vercel playback uses the official visible YouTube player; EQ and background playback are unavailable in this mode. See [web/README.md](web/README.md) for capabilities and deployment limits.

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
