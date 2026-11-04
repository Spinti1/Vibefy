param([string]$Version, [string]$Iscc = "$PSScriptRoot/../../.cache/inno/ISCC.exe")
$ErrorActionPreference='Stop'
$source=(Resolve-Path "$PSScriptRoot/../..").Path
if(!$Version){$Version=([regex]::Match((Get-Content "$source/Cargo.toml" -Raw),'(?m)^version = "([0-9.]+)"')).Groups[1].Value}
if($Version -notmatch '^\d+\.\d+\.\d+$'){throw 'Invalid version'}
$distribution=Join-Path $source '.cache/distribution'
$binary=Join-Path $source 'target/release/spotifast.exe'
$versionInfo=[Diagnostics.ProcessStartInfo]::new($binary, '--version')
$versionInfo.UseShellExecute=$false
$versionInfo.CreateNoWindow=$true
$versionInfo.RedirectStandardOutput=$true
$versionProcess=[Diagnostics.Process]::Start($versionInfo)
if(!$versionProcess.WaitForExit(10000)){
    $versionProcess.Kill()
    throw 'Timed out checking the application version; no installer was built'
}
$binaryVersion=$versionProcess.StandardOutput.ReadToEnd().Trim()
if($versionProcess.ExitCode -ne 0 -or $binaryVersion -notmatch ('(?:^|\s)'+[regex]::Escape($Version)+'$')){
    throw "Application version '$binaryVersion' does not match installer version '$Version'. Build the application first."
}
New-Item -ItemType Directory -Force $distribution | Out-Null
Copy-Item -LiteralPath $binary -Destination "$distribution/vibefy.exe" -Force
$output=Join-Path $source 'dist'
New-Item -ItemType Directory -Force $output | Out-Null
& $Iscc ('/DVersion='+$Version) ('/DAppDir='+$distribution) ('/DOutputDir='+$output) "$PSScriptRoot/vibefy.iss"
if($LASTEXITCODE -ne 0){throw 'Installer build failed'}
$setup=Join-Path $output "vibefy-v$Version-x86_64-pc-windows-msvc-setup.exe"
$hash=(Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText((Join-Path $output 'checksums.txt'),($hash+'  '+[IO.Path]::GetFileName($setup)+[Environment]::NewLine),[Text.UTF8Encoding]::new($false))
@{version=$Version;path=$setup;bytes=(Get-Item -LiteralPath $setup).Length;sha256=$hash;signature=(Get-AuthenticodeSignature -LiteralPath $setup).Status.ToString()} | ConvertTo-Json
