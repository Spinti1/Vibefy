# Native Vibefy mobile port (in development)

The 0.1.0 WebView preview did not meet the requested native-mobile design. It is superseded; no further WebView app releases are planned.

## Native implementation

- Android: Java native Android views, Media3 ExoPlayer/MediaSessionService, Android audio focus, headset handling and media notification integration. No WebView or JavaScript UI runtime.
- iOS: SwiftUI native views, AVPlayer, playback audio session, background audio entitlement and system media commands. No WKWebView or HTML UI.
- Native Home, search-on-submit, playlist creation and metadata storage, song list, queue and transport. Local metadata is stored as an atomic JSON file, separate from the retired WebView's untouched storage.
- Public metadata requests reuse the existing Vibefy HTTPS API; using an API does not render or embed the website.

## Outstanding work, not release-ready

The desktop playback engine uses an external yt-dlp process. iOS cannot run that desktop process, and the current hosted browser deployment disables direct YouTube audio. A mobile-compatible source resolver is still required. The current native players accept the existing direct JuiceVault audio endpoint; YouTube audio cannot be described as working yet.

Account connection, full playlist imports and matching/pagination, backups, Jam, mods, lyrics, visual polish and physical-device validation are still being ported. Existing wrapper storage is not auto-deleted or overwritten. Do not publish this build as desktop feature parity.

Build checks are in `.github/workflows/mobile-native.yml`. These are development artifacts, not a new SideStore release. Version 0.2.0 is reserved for this native development line. APK signing continues to use the locally retained publisher key when a release is ready; iOS unsigned device IPA packaging is for SideStore/iLoader signing.

Credits and licenses remain in the repository root. Native playback uses AndroidX Media3 (Apache-2.0) and Apple system frameworks. Desktop sources, credentials, playlists and listening data are unaffected.
