# Vibefy releases

Current source version: **0.4.44**. The imported Spotifast base was 0.14.2; its version is not the Vibefy version.

For each future user-requested functional update, increment the Vibefy patch version once (next: 0.4.45), update Cargo.toml and the root package in Cargo.lock, and record the changes here. Do not auto-update this fork from the Spotifast release feed.

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

0.4.23: Bundle Mozilla HTTPS trust roots alongside native OS roots; retry OAuth connection-establishment failures once without replaying sent grants; provide redacted certificate/DNS/proxy/timeout diagnostics. Document unsigned installer/SmartScreen limits; no code-signing identity is configured.

0.4.24: Account-scoped durable Spotify playlist additions retry explicit rate limits in the background, preserving Retry-After across restarts and suppressing duplicate pending clicks. Uncertain writes are not automatically replayed; authorization and quota errors remain visible.

0.4.25: YouTube Music browser-session import, protected local session storage, private/saved Music playlist reading and Home snapshots without Google OAuth. Optional JuiceVault browser handoff, public playlists and direct archive playback. Independent music-account entry points; Spotify remains separate.

0.4.26: Opt-in account-paired Spotify → private YouTube Music playlist copies, cached recording matches and manual review, durable dual-destination song additions, source snapshot checks, interrupted-write reconciliation and one linked playlist on Home. LEOPARD rollback backup preserves the complete 0.4.25 source and installer outside the repository.

0.4.27: New YouTube Music playlist copies are unlisted, with clear link-access wording. Existing private copies retain their visibility and recovery markers.

0.4.28: Direct Connect YouTube button below Spotify and on Home; official Google browser sign-in with PKCE and protected refresh tokens; YouTube Data API playlist reading and unlisted copying with single-write recovery. Browser-header import removed from the connection UI; existing stored sessions remain readable.

0.4.29: Independent Windows first-party YouTube sign-in window with a dedicated WebView2 profile, same-origin authenticated requests and local session removal. Direct Connect YouTube no longer opens the Google Cloud OAuth/testing flow. Native Spotify, playback and unlisted-copy journals remain in place; existing stored OAuth connections remain readable. No YTubic application code is incorporated.

0.4.30: Fix the confirmed post-2FA block of accounts.google.ro by accepting exact Google regional domains from the official supported_domains list, with hostname boundary checks. Keep bounded host/status-only local navigation diagnostics, explicit blocked-redirect errors and the canonical Music return URL. Remove the speculative cookie-based handoff recovery; Google controls the full authentication sequence.

0.4.31: Ordered Spotify then YouTube setup with matching centered account buttons, playlist selection, transfer progress and an explicit not-recommended skip. Account-paired setup state and selection survive restarts. Match before creating new copies; upload confirmed recordings progressively and reuse existing copies and complete source snapshots. Home exposes the connected YouTube library separately and deduplicates linked copies; Settings retains account/transfer controls rather than a second library list.

0.4.32: Dedicated playlist-selection gallery with cached artwork, searchable selectable cards and persistent visible actions. Separate transfer screen with confirmed-song progress, per-playlist status, service cooldowns and selection editing; remove collapsible onboarding menus.

0.4.33: Home shelves ordered Linked, Spotify, YouTube and optional JuiceVault. JuiceVault mod beside WaveLink includes title-ranked archive search, protected account sessions, username/password login and public/private library access. Remove linked/review dropdowns from Settings. Additional exact-video YouTube Music plain-lyrics fallback; timed sources remain preferred.

0.4.34: Durable linked-playlist renaming, stricter artist identity for lyrics with metadata cleanup and bounded exact lookups, concurrent More Lyrics fallback, and shared YouTube recording cache across linked and Spotify tracks.

0.4.35: Handle audio-engine completion while YouTube is connected and Spotify verification is pending. Prevent the restored-session startup race that discarded the engine and left playback stuck in Connecting.

0.4.36: Artist discography without related-artist shelves, navigable cached Music pages, one contextual fullscreen control, queue alongside lyrics/visualizers, compact appearance presets and visualizer controls. Preserve upstream credits and compatibility identifiers.

0.4.37: Separate lyrics and visualizer buttons, permanent Track Peek neighbours with artwork and reserved shuffle next track, removal of proxy controls and manual/system proxy routing. Existing saved proxy options are ignored; native HTTP, yt-dlp and YouTube sign-in use direct connections.

0.4.38: Hidden playlists at the bottom of Home with account-scoped local visibility and restore actions. Removes the Home top-song shelf and its background request. Playlist context menus expose supported service actions. Visualizers open inside the current window; fullscreen remains a separate control.

0.4.39: Removes the Recommended for you Home shelf, its preference, data state, request/response handling and unused Spotify API endpoint wrapper.

0.4.40: Track Peek cards are capped at 180 logical pixels and sit beside the transport controls instead of stretching across the player.

0.4.40 also fixes immediate playlist removal and stale library responses, and updates the Vibefy installer introduction while retaining upstream license notices.

0.4.41: Optional verified mod packages loaded in-process after restart; aligned mod catalogue and separate settings; removed JuiceVault browser sign-in button; collection reload no longer truncates at 30; installer version validation. Windows signing remains unavailable.

0.4.42: Fix audio-worker runtime shutdown, clear stale network-wait state on reconnect, preserve optimistic queue selection, restore encoded local-track labels safely, distinguish same-name Cast devices, and redact untrusted OAuth redirect paths. Repair obsolete unit-test fixtures for the current native queue, search, account setup and interface.

0.4.43: Continue the queue past unavailable recordings with a three-failure limit; distinguish source connectivity and rate-limit errors; invalidate pending playback work during shutdown.

0.4.44: Optional SoundVisualizer package with 12 adapted 2D effects, bounded visual history and clipping; Cloudflare Jam shared by desktop and mobile web with link/code invitations; audio-edition checks for censored recordings, faster confirmed official-audio selection and hydrated queued tracks; durable linked deletion and owned YouTube playlist deletion.
