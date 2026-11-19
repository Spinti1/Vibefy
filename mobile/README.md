# Vibefy mobile preview 0.1.0

Small native Android and iOS hosts for the existing HTTPS Vibefy web player. This preview requires internet and the deployed Vibefy backend. It is not a port of the Windows Rust playback engine. It shares the web player's search, public playlist imports, local playlist metadata, browser mods and Jam. No audio library or desktop speech model is bundled.

## Current limits

Google does not support its authorization flow in embedded WebViews. The preview offers **Open in browser** for the existing web connection. This does not log the native app in, and browser storage is separate from the app's storage. Export/import playlist metadata to transfer guest collections. Native account authorization is still pending; do not advertise desktop account parity.

YouTube still uses its official embedded player. Background/lock-screen playback is not guaranteed. JuiceVault and SoundCloud availability remains subject to providers. Native download, external file import/export behavior, autoplay and Jam timing need physical-device testing before a stable release. Local playlists belong to the app installation and can be lost when uninstalling. Existing desktop data is untouched.

## Android

Requires JDK 17 or 21, Gradle 8.9, Android SDK 35 and build tools. Run `gradle -p mobile/android assembleDebug assembleRelease lintRelease` from the repository root.

`app-debug.apk` is installable for testing. `app-release-unsigned.apk` is deliberately unsigned: a stable release needs a persistent publisher signing key stored outside source control. Do not ship repeatedly changing debug keys as production updates.

## iOS, SideStore and iLoader

Use **Build Vibefy mobile preview** in GitHub Actions. Its macOS runner creates a real device `.app`, packages `Payload/Vibefy.app` in an unsigned IPA, and generates a source from the IPA's actual bundle ID, version and byte count. No Apple password, certificate or provisioning profile is needed for this unsigned build. SideStore/iLoader handles device signing; Apple's sideloading restrictions and refresh requirements still apply.

The IPA can be imported directly into iLoader or SideStore. For a source link, first attach the exact IPA to the `mobile-v0.1.0` GitHub prerelease, then publish the generated `source.json` at a stable HTTPS URL. Do not list an IPA that has not been uploaded. The generated download URL is version-specific, not `latest`. Bundle ID is `online.usevibefy.mobile` on both platforms.

## Security and credits

Main-frame navigation stays on the fixed Vibefy HTTPS host. External HTTPS links open in the system browser. No native JavaScript bridge, filesystem access, arbitrary URL loader, cleartext exception, or certificate bypass is included. The operating system owns WebView storage. Embedded provider frames remain part of the existing player.

Vibefy and the web interface retain the repository's MIT license and existing [credits](../CREDITS.md). Android uses the system WebView; iOS uses Apple's WebKit. SideStore and iLoader are third-party installation tools, not authors of Vibefy. No new artwork or third-party app source was copied.
