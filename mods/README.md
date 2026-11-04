# Vibefy optional mods (API 1)

Click Install in Mods, then Reload now or Later. Packages run inside Vibefy; they are not separate applications. Downloads use a pinned catalogue, HTTPS, exact byte length and SHA-256. Guest modules have a memory limit and an instruction budget. They cannot open files, sockets, processes or account credentials.

The host supplies numeric playback state and renders bounded draw commands through egui. Nyan, Oneko, Cat Jam, Vivy, Backmusic and Track Peek have their visual/animation logic in WebAssembly. WaveLink, JuiceVault and playlist import register capabilities; credential handling, networking and playback remain native host services. Spicetify JavaScript is not directly compatible. Lyrics providers remain included core functionality.

Build with: powershell -File mods/build.ps1, then node mods/package.cjs. The Rust wasm32-unknown-unknown target is required. Packages are published as .vmod assets under the mods-v1 GitHub release. Do not overwrite published packages: create a new catalogue/version for updates. The installer excludes packages.

Credits and license notices are preserved inside every package and in the project CREDITS.md. See assets/mods/README.md for artwork provenance and licensing limitations.
