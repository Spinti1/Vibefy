# Vibefy Windows Setup

Use Inno Setup 7 (or a compatible 6.x compiler) with the built distribution directory. The package must contain vibefy.exe, vibefy.ico, the official standalone yt-dlp.exe, and the complete auto-sync runtime/model/notices. Do not include profiles, caches, passwords or cookies.

```powershell
ISCC.exe /DVersion=0.4.17 /DAppDir="C:\path\to\distribution" /DOutputDir="C:\path\to\release" packaging\vibefy\vibefy.iss
```

The new Vibefy AppId is distinct from Spotifast. Setup installs per user without administrator rights. Silent updates honor /DIR from the existing updater helper. The marker must contain exactly `vibefy-installer-v1`. No uninstall rule removes per-user data, cache, credentials or libraries. The installer is not code-signed.

Release builds use full LTO with speed optimizations retained. Setup uses solid LZMA2/max compression. This reduces the download where possible without removing runtime files; it does not shrink the installed speech model or audio cache. Compare final bytes with the previous release rather than assuming a fixed saving.

Release assets must be named `vibefy-vVERSION-x86_64-pc-windows-msvc-setup.exe`. Generate checksums.txt over the exact final setup bytes; upload both as GitHub release assets. Before publication, confirm GitHub's asset size and digest match local SHA-256. Do not replace assets after a published release; publish a new version instead.

SmartScreen checks both publisher and file reputation. The current package has
no Authenticode signing identity configured. A self-signed certificate does not
solve this warning. To prepare signed distribution, the publisher must obtain
a trusted code-signing identity (for example Microsoft Artifact Signing or a
CA-issued certificate), complete its identity validation, and configure secure
signing access. Never commit a signing private key or password.

Sign the final Vibefy application executable before compiling Setup, then sign
the final Setup executable with a trusted timestamp. Validate both signatures
with Authenticode before generating checksums or uploading release assets.
Keep the same publisher identity across releases. Third-party runtime files
retain their original provenance; do not change their upstream signatures.
Signing may reduce warnings as reputation grows but cannot guarantee that a
new installer will immediately avoid SmartScreen. Store distribution is a
separate option. Do not disable SmartScreen or Defender to distribute Vibefy.

Microsoft reference: https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation

## Development disk cleanup

Rust compilation and test artifacts are local developer files; they are not installed for users. After building and installing the current version, run `packaging/vibefy/clean-build-artifacts.ps1` to retain the production executable and newest Setup while removing old installers, debug artifacts, release intermediates and obsolete scratch binaries. Preview with `-WhatIf`. The script refuses active builds and linked artifact trees, and never visits application data or credentials. A later build regenerates its dependencies and will take longer.
