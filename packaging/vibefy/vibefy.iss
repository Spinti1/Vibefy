#ifndef Version
  #error Version is required
#endif
#ifndef AppDir
  #error AppDir is required
#endif
#ifndef OutputDir
  #error OutputDir is required
#endif
[Setup]
AppId={{287821C8-7E11-4902-93CB-C051C1F891EC}
AppName=Vibefy
AppVersion={#Version}
AppPublisher=Spinti
AppPublisherURL=https://usevibefy.online
AppSupportURL=https://github.com/Spinti1/Vibefy/issues
AppUpdatesURL=https://github.com/Spinti1/Vibefy/releases
DefaultDirName={localappdata}\Programs\Vibefy
DefaultGroupName=Vibefy
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
LicenseFile=vibefy-license.txt
InfoBeforeFile=welcome.txt
DisableWelcomePage=no
DisableDirPage=no
AppComments=Independent open-source music player by Spinti, based on Spotifast.
VersionInfoProductName=Vibefy
VersionInfoDescription=Vibefy Setup
VersionInfoCompany=Spinti
OutputDir={#OutputDir}
OutputBaseFilename=vibefy-v{#Version}-x86_64-pc-windows-msvc-setup
SetupIconFile={#AppDir}\vibefy.ico
UninstallDisplayIcon={app}\vibefy.exe
VersionInfoVersion={#Version}.0
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
SetupMutex=VibefySetup
[Messages]
WelcomeLabel1=Welcome to Vibefy Setup
WelcomeLabel2=Install Vibefy, an independent music player by Spinti.%n%nConnect Spotify and YouTube after installation to browse your library and use the available playback features.%n%nThis installer keeps your existing Vibefy settings and accounts during an update.
InfoBeforeLabel=About Vibefy and its components
FinishedHeadingLabel=Vibefy is ready
FinishedLabel=Vibefy has been installed on your computer. Open the app to connect your accounts or continue listening.
[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked
[Files]
Source: "{#AppDir}\vibefy.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#AppDir}\yt-dlp.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#AppDir}\vibefy.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#AppDir}\auto-sync\*"; DestDir: "{app}\auto-sync"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE_SPOTIFAST.txt"; Flags: ignoreversion
Source: "..\..\CREDITS.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\THIRD_PARTY_LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "vibefy-installer.txt"; DestDir: "{app}"; Flags: ignoreversion
[Icons]
Name: "{autoprograms}\Vibefy"; Filename: "{app}\vibefy.exe"
Name: "{autodesktop}\Vibefy"; Filename: "{app}\vibefy.exe"; Tasks: desktopicon
[Run]
Filename: "{app}\vibefy.exe"; Description: "Open Vibefy"; Flags: nowait postinstall skipifsilent
