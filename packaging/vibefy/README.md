# Vibefy Windows Setup

Use Inno Setup 7 (or a compatible 6.x compiler) with the built distribution directory. The package must contain vibefy.exe, vibefy.ico, the official standalone yt-dlp.exe, and the complete auto-sync runtime/model/notices. Do not include profiles, caches, passwords or cookies.

```powershell
ISCC.exe /DVersion=0.4.17 /DAppDir="C:\path\to\distribution" /DOutputDir="C:\path\to\release" packaging\vibefy\vibefy.iss
```

The new Vibefy AppId is distinct from Spotifast. Setup installs per user without administrator rights. Silent updates honor /DIR from the existing updater helper. The marker must contain exactly `vibefy-installer-v1`. No uninstall rule removes per-user data, cache, credentials or libraries. The installer is not code-signed.

Release assets must be named `vibefy-vVERSION-x86_64-pc-windows-msvc-setup.exe`. Generate checksums.txt over the exact final setup bytes; upload both as GitHub release assets. Before publication, confirm GitHub's asset size and digest match local SHA-256. Do not replace assets after a published release; publish a new version instead.
