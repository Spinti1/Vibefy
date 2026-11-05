# Vibefy optional mods (API 1)

Click Install in Mods, then Reload now or Later. Packages run inside Vibefy; they are not separate applications. Downloads use a pinned catalogue, HTTPS, exact byte length and SHA-256. Guest modules have a memory limit and an instruction budget. They cannot open files, sockets, processes or account credentials.

The host supplies numeric playback state and renders bounded draw commands through egui. Nyan, Oneko, Cat Jam and Track Peek have their visual/animation logic in WebAssembly. WaveLink and JuiceVault register capabilities; credential handling, networking and playback remain native host services. Spicetify JavaScript is not directly compatible. Playlist imports, lyrics providers and all visualizers are included core functionality.

Build with: powershell -File mods/build.ps1, then node mods/package.cjs. The Rust wasm32-unknown-unknown target is required. Packages are published as .vmod assets under the mods-v1 GitHub release. Do not overwrite published packages: create a new catalogue/version for updates. The installer excludes packages.

Credits and license notices are preserved inside every package and in the project CREDITS.md. See assets/mods/README.md for artwork provenance and licensing limitations.

## Built-in visualizers

Visualizers are built into Vibefy and are no longer mod packages. The main selector contains two Vivy effects, two Backmusic effects and 12 SoundVisualizer 2D effects: Bars, Center bars, Circular, Wave, Radial wave, Dots, Blocks, Starburst, Helix, Strings, Lissajous and Arcs. Geometry and audio mappings originate in CaYatur/SoundVisualizer (MIT), pinned to revision c3e7fc6bc3fa826e323486256268fa764a317963. It uses Vibefy's existing post-EQ, pre-volume audio tap; waveform modes receive PCM, and spectrum modes receive FFT bands. The history buffer is limited to 24 frames at 30 Hz. Drawing commands are clipped to the visualization area. GPU/3D modes, Canvas bloom and MilkDrop are not part of this port.
