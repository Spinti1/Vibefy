# Vibefy mobile 0.2.4 hybrid preview

The user authorized a lightweight hybrid app after the native-only port began. The current entry points use Android WebView and Apple WKWebView with app-style navigation and the existing mobile interface. Native-only prototype screens and player code are retained for later engine integration; they are not the active playback engine in this build.

## Implemented

- No browser address bar or persistent Reload/Open in browser toolbar. Native launch cover, dark surfaces and Android Back/dialog navigation.
- Persistent WebView/WKWebsiteDataStore profiles. Existing 0.1.0 local libraries on the same hosted origin are not wiped. Queue metadata now survives a cold app launch; it does not include music files or account secrets.
- Browser HTTP caching stays enabled. Rendering expensive offscreen playlist tiles can be skipped. Active animation frame pacing is not capped; heavyweight desktop visualizers/Track Peek/Oneko are absent from the mobile web interface.
- Native document export for metadata-only playlist backups, main-frame/origin-restricted bridge, and platform document selection for JSON imports.
- Jam code/link joining uses the existing relay and desktop protocol. `vibefy://jam?code=...` opens the app's Join screen. This works across networks; automatic same-Wi-Fi discovery/Spotify Connect-style playback transfer is not implemented yet.

## Important remaining limitations

Google connection opens the registered Vibefy website in the system browser, then returns an encrypted, single-use grant to the waiting app. Tap Return to Vibefy after consent. Reloading or killing the app during authorization requires restarting the connection. Mobile 0.2.4 stores unexpired grants in device-protected storage and restores them after relaunch. Grants still expire according to Google; reconnect when requested. Google verification, consent and user limits still apply. JuiceVault's existing account form remains subject to provider acceptance. Account tokens can expire; persistent website data is not a guarantee of permanent login.

YouTube uses the existing official player, including its visible video. Background/lock-screen YouTube playback is not guaranteed. Native Media3/AVPlayer prototype source is not presented as an integrated YouTube engine. Physical Android/iPhone flow, battery and frame-time measurement are still required before describing performance as proven.

The app requires the hosted Vibefy service. It is not an offline player. Device-local playlists can be lost when app data is cleared or the app is uninstalled; export a metadata backup first.

## Builds and signing

Use the mobile workflow on the development branch. Android uses SDK 35/Gradle 8.9/JDK 21. APK release signing uses the existing locally retained publisher key. iOS device builds run on macOS and produce an unsigned IPA for SideStore/iLoader signing. No signing credentials or account data are committed.

See PORT_STATUS.md and the repository's CREDITS.md. AndroidX WebKit and Media3 are Apache-2.0; iOS WebKit and AVFoundation are system frameworks. The hybrid design is disclosed rather than advertised as fully native.


## 0.2.3 mobile account fix and Android updates

The 0.2.2 document-start script read document.documentElement before WebKit/WebView created it, set its initialized flag, then aborted. The web page therefore fell back to the embedded Google popup. Initialization now installs the native authorization bridge without accessing DOM nodes, and delays styling until DOMContentLoaded. The initialized flag is set only after bridge setup. The hosted repair also recovers partially initialized 0.2.2 profiles without clearing their data. Ordinary browsers and foreign frames are excluded.

Android 0.2.3 adds automatic GitHub release checks (at most once per six hours), a manual check in Settings, bounded HTTPS downloads, GitHub release size/SHA-256 checks, and an Android installer handoff through a narrowly scoped FileProvider. Android itself requires the user's installation permission and confirmation. Older APKs did not contain this updater; users must install this update once, over the existing app. A hosted Android notice links to the current APK for those versions.


## 0.2.4 protected account persistence

iOS stores the Google grant in a device-bound Keychain item; Android encrypts it with an app-specific Android Keystore AES-256-GCM key. Neither grant is saved in WebKit localStorage, plaintext preferences or playlist backups. Only the trusted main-frame bridge can read/write/clear that one account record. Restoring does not extend the Google expiry. Explicit disconnect and Google 401 remove the stored record; ordinary network errors do not. A failed save is reported rather than silently promising persistence.

The current web-token flow has no refresh token, and no matching web-client secret is configured for an offline authorization-code flow. This fixes loss on each app restart while access remains valid, not permanent authorization after Google's token expiry (typically around one hour). First connect once after installing 0.2.4.
