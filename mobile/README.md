# Vibefy mobile 0.2.6 hybrid preview

The user authorized a lightweight hybrid app after the native-only port began. The current entry points use Android WebView and Apple WKWebView with app-style navigation and the existing mobile interface. Native-only prototype screens and player code are retained for later engine integration; they are not the active playback engine in this build.

## Implemented

- No browser address bar or persistent Reload/Open in browser toolbar. Native launch cover, dark surfaces and Android Back/dialog navigation.
- Persistent WebView/WKWebsiteDataStore profiles. Existing 0.1.0 local libraries on the same hosted origin are not wiped. Queue metadata now survives a cold app launch; it does not include music files or account secrets.
- Browser HTTP caching stays enabled. Rendering expensive offscreen playlist tiles can be skipped. Active animation frame pacing is not capped; heavyweight desktop visualizers/Track Peek/Oneko are absent from the mobile web interface.
- Native document export for metadata-only playlist backups, main-frame/origin-restricted bridge, and platform document selection for JSON imports.
- Jam code/link joining uses the existing relay and desktop protocol. `vibefy://jam?code=...` opens the app's Join screen. This works across networks; automatic same-Wi-Fi discovery/Spotify Connect-style playback transfer is not implemented yet.

## Important remaining limitations

Google connection opens the registered Vibefy website in the system browser, then returns an encrypted, single-use grant to the waiting app. Tap Return to Vibefy after consent. Reloading or killing the app during authorization requires restarting the connection. Mobile 0.2.5 stores renewable grants in device-protected storage, renews access before expiry and retries renewal after returning from the background. Connect once after updating to enable this; existing access-only grants cannot be converted automatically. Google verification, consent and user limits still apply. JuiceVault's existing account form remains subject to provider acceptance. Account tokens can expire; persistent website data is not a guarantee of permanent login.

YouTube uses the existing official player, including its visible video. Background/lock-screen YouTube playback is not guaranteed. Native Media3/AVPlayer prototype source is not presented as an integrated YouTube engine. Physical Android/iPhone flow, battery and frame-time measurement are still required before describing performance as proven.

The app requires the hosted Vibefy service. It is not an offline player. Device-local playlists can be lost when app data is cleared or the app is uninstalled; export a metadata backup first.

## Builds and signing

Use the mobile workflow on the development branch. Android uses SDK 35/Gradle 8.9/JDK 21. APK release signing uses the existing locally retained publisher key. iOS device builds run on macOS and produce an unsigned IPA for SideStore/iLoader signing. No signing credentials or account data are committed.

See PORT_STATUS.md and the repository's CREDITS.md. AndroidX WebKit and Media3 are Apache-2.0; iOS WebKit and AVFoundation are system frameworks. The hybrid design is disclosed rather than advertised as fully native.


## 0.2.3 mobile account fix and Android updates

The 0.2.2 document-start script read document.documentElement before WebKit/WebView created it, set its initialized flag, then aborted. The web page therefore fell back to the embedded Google popup. Initialization now installs the native authorization bridge without accessing DOM nodes, and delays styling until DOMContentLoaded. The initialized flag is set only after bridge setup. The hosted repair also recovers partially initialized 0.2.2 profiles without clearing their data. Ordinary browsers and foreign frames are excluded.

Android 0.2.3 adds automatic GitHub release checks (at most once per six hours), a manual check in Settings, bounded HTTPS downloads, GitHub release size/SHA-256 checks, and an Android installer handoff through a narrowly scoped FileProvider. Android itself requires the user's installation permission and confirmation. Older APKs did not contain this updater; users must install this update once, over the existing app. A hosted Android notice links to the current APK for those versions.


## Protected account persistence and renewal (0.2.5)

iOS uses a device-bound Keychain item; Android uses app-specific Android Keystore AES-256-GCM encryption. Credentials are excluded from browser storage and playlist backups. The trusted main-frame bridge reads/writes only that account record.

New mobile connections use Google's authorization-code model through the system browser. The Vercel endpoint exchanges the code using the existing Vibefy Web client; only its secret is held in protected server configuration. The server does not persist user tokens. The encrypted one-time app callback carries the grant to the waiting app, which stores the refresh credential locally. Renewal submits that credential over HTTPS to the fixed same-origin endpoint and on to Google. API calls use the access token directly with Google.

Expired access is renewed before library requests and after background suspension. Concurrent renewal is shared. Network/server/configuration errors keep the protected credential; only explicit disconnect or confirmed Google invalid_grant clears it. A YouTube API 401 triggers one renewal/retry instead of deleting the account. Reconnect once (Keep me signed in) after updating from an access-only version. Google can still revoke access or require consent; permanent authorization is not guaranteed. Physical-device authentication and background suspension remain to be validated on an iPhone and Android phone.

## 0.2.6 QR imports and mobile presentation

A desktop QR opens a metadata preview at /web#share=<id>. Save a local copy without signing in; Keep updated retrieves changes on opening it or returning to the foreground, with conditional requests and a one-minute minimum interval. vibefy://playlist?id=<id> opens the preview in installed apps. Turning off following allows independent local edits. Metadata backups retain the read-only subscription, never its desktop owner key.

Native phone pinch/page zoom is disabled. YouTube now-playing displays cover artwork; this presentation change does not remove the existing YouTube video stream or guarantee audio-only network traffic. Background playback limitations remain.


### Audio-only playback in 0.2.7

Public YouTube recordings resolve on the device. Only M4A audio streams are selected; the hybrid UI retains artwork and existing queue/Jam controls without loading a YouTube iframe. No audio files are retained. Native resolution uses NewPipeExtractor on Android (GPL-3.0-or-later; Android target distributed under the same license), and Alexander Eichhorn's YouTubeKit on iOS (MIT). See `licenses/` and the exact dependency pins. Corresponding Android source is the `mobile/android/` tree at the release commit. Provider restrictions and unavailable audio are reported without switching back to video.

Mobile 0.2.8 adds in-app playlist QR scanning with camera-only permission. Frames are decoded locally with jsQR 1.4.0 (Apache-2.0); no recording/upload. Capture stops on close, successful scan and background. Phone Home places Scan above Create; phone backup buttons and the redundant header library shortcut are hidden. Existing playlist metadata and sessions are preserved.
