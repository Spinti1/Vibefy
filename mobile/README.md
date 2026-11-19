# Vibefy mobile 0.2.1 hybrid preview

The user authorized a lightweight hybrid app after the native-only port began. The current entry points use Android WebView and Apple WKWebView with app-style navigation and the existing mobile interface. Native-only prototype screens and player code are retained for later engine integration; they are not the active playback engine in this build.

## Implemented

- No browser address bar or persistent Reload/Open in browser toolbar. Native launch cover, dark surfaces and Android Back/dialog navigation.
- Persistent WebView/WKWebsiteDataStore profiles. Existing 0.1.0 local libraries on the same hosted origin are not wiped. Queue metadata now survives a cold app launch; it does not include music files or account secrets.
- Browser HTTP caching stays enabled. Rendering expensive offscreen playlist tiles can be skipped. Active animation frame pacing is not capped; heavyweight desktop visualizers/Track Peek/Oneko are absent from the mobile web interface.
- Native document export for metadata-only playlist backups, main-frame/origin-restricted bridge, and platform document selection for JSON imports.
- Jam code/link joining uses the existing relay and desktop protocol. `vibefy://jam?code=...` opens the app's Join screen. This works across networks; automatic same-Wi-Fi discovery/Spotify Connect-style playback transfer is not implemented yet.

## Important remaining limitations

Google account connection inside the embedded app is not implemented. Opening a separate browser does not connect this app's storage. JuiceVault's existing account form remains subject to provider acceptance. Account tokens can expire; persistent website data is not a guarantee of permanent login.

YouTube uses the existing official player, including its visible video. Background/lock-screen YouTube playback is not guaranteed. Native Media3/AVPlayer prototype source is not presented as an integrated YouTube engine. Physical Android/iPhone flow, battery and frame-time measurement are still required before describing performance as proven.

The app requires the hosted Vibefy service. It is not an offline player. Device-local playlists can be lost when app data is cleared or the app is uninstalled; export a metadata backup first.

## Builds and signing

Use the mobile workflow on the development branch. Android uses SDK 35/Gradle 8.9/JDK 21. APK release signing uses the existing locally retained publisher key. iOS device builds run on macOS and produce an unsigned IPA for SideStore/iLoader signing. No signing credentials or account data are committed.

See PORT_STATUS.md and the repository's CREDITS.md. AndroidX WebKit and Media3 are Apache-2.0; iOS WebKit and AVFoundation are system frameworks. The hybrid design is disclosed rather than advertised as fully native.
