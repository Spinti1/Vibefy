# Vibefy releases

Current source version: **0.4.22**. The imported Spotifast base was 0.14.2; its version is not the Vibefy version.

For each future user-requested functional update, increment the Vibefy patch version once (next: 0.4.23), update Cargo.toml and the root package in Cargo.lock, and record the changes here. Do not auto-update this fork from the Spotifast release feed.

0.4.4: About and Mods credits, clearer buttons, cache management and 2 GB advisory, performance improvements.

0.4.5: Shopping cart beside Search opens the optional Mods catalogue with a performance advisory.

0.4.6: Optional native mod catalogue, cats and rainbow progress, live fullscreen visualizers, public SoundCloud playback, local YouTube playlist imports, direct LAN/VPN Listen Together, and Vibefy support link.

0.4.7: Original Nyan PNG frames and Cat Jam animation at track-tempo speed; fullscreen visualizer selector and per-visualizer settings; generated build files cleaned after installation.

0.4.8: Responsive, ordered Mods catalogue without categories; YouTube imports saved automatically on Home; all library playlists shown in the virtualized Home grid; complete sequential Liked Songs loading and partial-cache continuation.

0.4.9: Saved YouTube/source playlists use the shared Spotify playlist hero, artwork tint, large playback controls, filtering/sorting and native track table with covers and durations. Original imports retain compatibility; new imports preserve playlist cover/owner metadata. Source view playback retains video identity, repeated entries and the displayed order.

0.4.10: Installed WaveLink adds SoundCloud search beside the main search, with an independent query/results page. Spotify search starts after 180 ms, reuses eight complete queries for two minutes per account session, and reserves transport capacity for search/track metadata. Playback reuses inspected yt-dlp metadata, shares in-flight preparation with prefetch, cancels superseded work and avoids reading whole cached files just to check prefetch eligibility.

0.4.11: Correct cache accounting/cleanup under Windows MSIX file virtualization while rejecting linked roots and child junctions. Search shows loaded account metadata immediately, requests music first and auxiliary categories on demand, caches music independently, returns promptly on rate limits, and bounds search network work to eight seconds.

0.4.12: Main search moves to public YouTube Music, with category filters, native browse/continuation, exact source playback and Save on Home. Imported playlists progressively acquire confidently matched 600 px music artwork while retaining source IDs, order and timing. Primary uploader thumbnails replace numbered frame selection.

0.4.13: Strict audio-only search, generic Search label, prominent artist/top-result cards and shelves, cached Spotify artwork assist, native browse pagination fix, complete Mods catalogue including built-in lyrics, optional native Track Peek, removed obsolete external preset renderer and media/build support.

0.4.14: Built-in Listen Together replaces Listening Activity; compact top-left lyrics information; edge-anchored Cat Jam with persistent size control and 90% default; removed custom skin catalogue/import controls; branded Vibefy classic mini player.

0.4.15: Public Vibefy project and website preparation, dedicated Windows installer, and GitHub HTTPS updates bound to release asset size and SHA-256 digests. Mobile web capabilities are documented separately from native desktop features.

0.4.16: Desktop presentation-only website, phone-specific player, optional read-only Spotify PKCE integration awaiting publisher configuration/approval, browser adaptations of Nyan Cat, Oneko, Cat Jam and Track Peek. No new Windows installer for this web-only update.

0.4.17: Website uses an isolated desktop demo screenshot, factual copy and visible project/mod credits, including the Spotifast mini-player origin. Native source-track menus offer Spotify recording search, ranked suggestions and user-confirmed playlist insertion.

0.4.18: Profile-menu GitHub update checks open the verified installer flow; square artwork in Spotify recording choices; undersized framework-restored windows recover the saved main size without changing the mini player.

0.4.19: Bounded similarity replaces exact cleaned-title equality for Spotify-to-YouTube playback; exact matches rank higher, with artist/version/duration and sequel guards preserved. Settings show the actual automatic AAC/M4A format in place of inactive legacy bitrate buttons.

0.4.20: Shared immutable lyrics for rendering and sync settings; recycled visualizer buffers and history only for Vivy; removed unused constants; full release LTO and stronger installer compression, with all source helpers and sync models retained.

0.4.21: Native lossless background AAC/MP4 cache compaction; process search/inspection responses as they arrive for fully verified catalogue playback; skip identical recording-index writes. No new runtime executable, bitrate reduction or changed artist/version/duration rules.

0.4.22: Plain lyrics no longer stop synchronized-provider lookup; plain Spotify lyrics can fall back to timed LRCLIB entries, and timed More Lyrics results replace plain text. Pre-fix plain caches are rechecked once; new plain caches and misses expire after six hours. Existing word timing, credits and plain-text fallback are retained.
