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
