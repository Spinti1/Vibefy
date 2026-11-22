# Background audio: Safari Home Screen and Brave

Investigation: 22 November 2026. Physical iPhone lock-screen testing was not available in this environment. Browser unit tests cannot establish background playback on a real device.

## What Brave actually provides

Brave documents background streaming in its built-in Playlist feature. Its iOS `PlayerModel.swift` uses AVKit, configures and activates an `AVAudioSession` with the playback category, and owns native remote controls and queue state. `PlaylistMediaStreamer.swift` resolves playable assets through a native web loader and checks `AVURLAsset` playability. This is browser application functionality, not an API granted to any website installed from that browser. This finding concerns Playlist specifically; it does not imply every Brave YouTube tab uses this same route.

- [Brave Playlist documentation](https://support.brave.app/hc/en-us/articles/360060768611-How-do-I-use-Brave-Playlist)
- [Brave native player](https://github.com/brave/brave-core/blob/master/ios/brave-ios/Sources/PlaylistUI/PlayerModel.swift)
- [Brave media resolver](https://github.com/brave/brave-core/blob/master/ios/brave-ios/Sources/Playlist/PlaylistMediaStreamer.swift)
- [Apple native background playback configuration](https://developer.apple.com/documentation/avfoundation/configuring-your-app-for-media-playback)

No Brave source was copied into Vibefy.

## Safari is not categorically unable to play background audio

WebKit's historical Home Screen audio issue was marked as a duplicate of a fixed platform bug. A more recent Safari 26 bug report describes frozen media and silent audio after interruptions, and reports that setting `navigator.audioSession.type = 'playback'` at page load prevented that reproduction. The latter is a reporter's finding, not proof that it fixes Vibefy's embedded YouTube player.

- [Historical Home Screen issue](https://bugs.webkit.org/show_bug.cgi?id=198277)
- [Fixed WebKit platform change](https://bugs.webkit.org/show_bug.cgi?id=232909)
- [Recent interruption report](https://bugs.webkit.org/show_bug.cgi?id=323022)
- [Audio Session specification](https://w3c.github.io/audio-session/)

Vibefy now requests the playback category during initialization, before the first media start, and reapplies it on direct audio play. Unsupported/rejected APIs are ignored safely. Native Android retains its own session. This introduces no background polling, forced resume, silent tracks, or microphone permission.

## Current Vibefy routes

| Route | Owns audio playback | Background status |
| --- | --- | --- |
| Safari/PWA YouTube | Cross-origin YouTube iframe | Unresolved; Vibefy cannot control the provider's internal media element |
| Safari/PWA direct audio | Existing HTML audio element | Correct playback category and Media Session controls; requires physical device tests |
| Android 0.2.11 | Native Media3 service | Implemented native queue and notification controls; not a Safari solution |

On 22 November, the public Vercel audio endpoint returned 502 without delivering audio; the runtime log identified YouTube's bot sign-in challenge. The experimental flag was removed afterwards. Moving the same extractor to another datacenter is not evidence that the source will work there. No user account cookies were uploaded for extraction.

## Recommended route for the requested cross-browser PWA

Use the existing HTML audio element with a real audio-only HTTPS stream, byte ranges for seek/resume, accurate media type and Media Session metadata/actions. For Safari, use a compatible audio codec/container such as AAC in M4A or supported audio-only HLS. Keep the same element mounted while navigating. Resolve the next queue item ahead of time in a bounded metadata cache where possible; test actual transitions while locked because background JavaScript execution is not guaranteed. Do not depend on UI timers, service workers, wake locks or a hidden video as the audio engine.

The missing component is a dependable, authorized source of playable audio URLs, with delivery that works from the user's phone. Vercel's current YouTube extraction route does not meet that requirement. A separate streaming service must demonstrate source access and successful byte-range delivery before it is enabled. It should stream in bounded chunks without persistent music storage, validate source hosts and IDs, rate-limit access, and renew expired source URLs. A native iOS player is another technical route, but would require native app distribution; it does not repair the user's Safari installation.

Acceptance requires real-device checks in Safari tab, Home Screen standalone, and other target browsers: start from a user gesture, lock for several minutes, advance to the next song while locked, pause/resume and seek from lock-screen controls, handle a call/audio interruption and reconnect after a network change. Test direct audio separately from YouTube to distinguish browser failures from source delivery failures. Do not claim full background support until the source path and these cases pass.

## Separate Cloudflare service test

A temporary, separate Worker was deployed on 22 November with a fixed public recording and three requests per minute per client. It accepted no arbitrary URLs, user cookies or credentials and did not change the production Jam Worker or browser player.

- The ordinary public watch response reported `OK`, with four audio format descriptions, but zero direct audio URLs. Metadata success was not playback success.
- A bounded audio-only SABR test using `googlevideo` 5.0.1 reached the media host, which returned HTTP 403. No usable audio sample was obtained. No proof-of-origin token minting or challenge workaround was added.
- A separate check of the protocol used by the existing NewPipe-based native resolver returned `LOGIN_REQUIRED` and zero audio formats from the hosting environment. No account credentials were supplied to continue past that response.

These tests reject simply moving the existing audio extraction path from Vercel to Cloudflare as a demonstrated solution. No PWA switch was enabled. The temporary Worker was removed after the investigation; its failed routes are not advertised as a playback service. A working source-delivery path remains the prerequisite for Safari background playback.

## Direct audio handoff correction

The user confirmed that one JuiceVault track remains audible with the iPhone locked, but the next track appeared to progress silently. The direct audio transition previously paused the element, removed its source, called `load()` with no source, then set the next source and called `load()` again. JuiceVault now assigns its next URL and calls `play()` synchronously before UI work, retaining the same audio element without the empty-source reset. A normal zero-position start no longer seeks back to zero after metadata arrives. Pausing while metadata loads also updates the pending intent, so metadata cannot undo the user's pause.

Late play failures from a superseded source are ignored. Browser Media Session reports direct audio as playing only after the media `playing` event, and reports paused while buffering. Position updates now call `navigator.mediaSession.setPositionState` on the correct object. No silent keep-alive audio, microphone access, extra download, automatic foreground restart, or background polling was introduced. The behavioral tests verify transition ordering, pending pause, old promise rejection isolation, and lock-screen state/position publication. A repeated physical iPhone queue test is still required; these changes do not solve the separate YouTube source-access limitation.
