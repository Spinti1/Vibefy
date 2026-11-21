# Current mobile direction

The user subsequently authorized an optimized hybrid app. Current app entry points are WebView/WKWebView plus native app navigation and document export. The native-only work below is retained as a development foundation, not advertised as the current released engine. See README.md for the exact 0.2.1 feature/limitation list. Automatic same-network desktop discovery and complete Google/native audio integration remain pending.

# Native mobile port status

## Requested product

Native Android APK and iOS IPA, with the desktop app's workflows adapted to touch. The website must not be the application's UI. The retired WebView preview does not satisfy this request.

## Implemented development code

- Android native views for Home, submitted search, local playlist creation and track menus, public import requests, queue and transport.
- Media3 player in a MediaSessionService, audio focus, headset disconnection handling and media-session controls.
- Android atomic metadata writes on a separate writer executor, so a slow search cannot delay playlist saves. Playback queue, shuffle/repeat and position checkpoints are separate from the library.
- iOS SwiftUI Home, search, playlists, queue, song rows and transport. AVPlayer with background audio configuration and system media commands. Atomic playlist writes and playback checkpoints.
- No WebView/WKWebView in the native application's UI source. The existing public API is used only for JSON metadata.
- Native development kept on branch native-mobile-development. No native release has been published and no desktop installation or user data changed.

## Required before a usable native release

1. YouTube source resolution on each device. The current hosted deployment disables /api/audio, so calling that endpoint is not a working mobile engine. The desktop external yt-dlp process is not directly portable to iOS. Evaluate embedding a supported extraction runtime or a maintained portable extractor, preserving license notices and respecting unavailable/private/protected recordings. Do not silently enable shared-server audio or present a WebView as the workaround.
2. Finish native account authorization and secure token/session storage. Existing website authorization is not a native account implementation.
3. Complete Spotify import matching and pagination; current API request integration alone is not equivalent to the desktop import flow.
4. Port lyrics matching/rendering, Jam transport/state/controls and the compatible mods. Share tested core logic rather than claim parity from native screens alone.
5. Native backup document import/export and metadata migration. Preserve the retired app's storage; do not delete it to reset the new app.
6. Touch UI polish, accessibility, and actual Android/iPhone playback/background/interruption testing.

## Evidence

Android local assembleDebug, assembleRelease and lintRelease succeeded (0 errors, 25 warnings primarily native text/resource lint). iOS checked via macOS GitHub Actions; final native build is tracked in .cache/native-mobile-verification.md.

The historical 0.1.0 release is marked superseded, and main/mobile/source.json now has no installable entries. Historical files remain available; no user device data was deleted.


## Mobile 0.2.2: Google browser handoff

The current published mobile architecture is hybrid, following the user's later authorization to use an optimized WebView. Earlier native prototype notes above are historical; those screens are not the distribution target.

Google consent now runs on the existing registered web origin in the system browser, with a deliberate Return to Vibefy link. A fresh 256-bit AES-GCM key and random 192-bit state bind the response to the waiting app page. The return URL contains only authenticated ciphertext; the key is sent in the initial fragment and immediately removed from browser history. Neither grant nor key is sent to Vibefy's server or saved in localStorage. Foreign frames/origins cannot launch the native bridge. Responses expire after five minutes and are accepted once. A killed/reloaded app must restart the connection. Tokens expire according to Google and require reconnection; this does not add refresh tokens or bypass Google verification/user limits.

Checks: Android release build and lint; shared queue/backup tests; actual browser-page encryption round trip, wrong state, tampering, timeout, permission denial; hosted account-controller integration including connected label and playlist API load without GIS inside WebView. iOS is compiled by macOS CI. Account-specific consent, 2FA and physical phone/browser transitions require a real device and remain explicitly unverified in this unattended session.


## 0.2.3 regression investigation

Reproduced the 0.2.2 failure from the actual tagged app-shell.js with documentElement=null for both native bridges. The live hosted repair recovers that failed initialized state and sends connectGoogle to the native browser handler. Added document-start/repair/ordinary-browser tests and four Android update-policy tests (version ordering, trusted HTTPS hosts, release asset metadata, exact size+SHA-256). Google itself remains in the system browser; the embedded app never falls back to a Google popup when its bridge is unavailable. Real account consent/2FA on a physical phone remains unverified; synthetic tests do not claim to replace that check.


## 0.2.4 account persistence

Google grants now survive app process exit while unexpired: device-bound iOS Keychain and authenticated Android Keystore encryption. Stored sessions retain the original absolute expiry and granted scopes; no token goes into browser storage or playlist backups. Explicit logout and Google 401 clear protected storage, while network failure does not. Per-page nonces prevent stale bridge replies across reloads; Android account I/O is independent from updater network work.

The current implicit/token-model authorization does not issue refresh tokens. No matching web-client secret is configured, so automatic offline renewal is not implemented. Reconnection after Google expiry remains required; this release fixes loss on every reopen rather than promising indefinite authorization.
