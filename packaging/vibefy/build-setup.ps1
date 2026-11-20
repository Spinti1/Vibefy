param([string]$Version, [string]$Iscc = "$PSScriptRoot/../../.cache/inno/ISCC.exe", [string]$RuntimeDir)
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
if($RuntimeDir){
    $runtime=(Resolve-Path -LiteralPath $RuntimeDir).Path
    foreach($name in @('vibefy.ico','yt-dlp.exe')){
        Copy-Item -LiteralPath (Join-Path $runtime $name) -Destination (Join-Path $distribution $name) -Force
    }
    $sync=Join-Path $runtime 'auto-sync'
    $manifest=Get-Content -LiteralPath (Join-Path $sync 'manifest.json') -Raw | ConvertFrom-Json
    $syncOutput=Join-Path $distribution 'auto-sync'
    New-Item -ItemType Directory -Force (Join-Path $syncOutput 'Release') | Out-Null
    $model=Join-Path $sync 'ggml-base.bin'
    if((Get-FileHash -LiteralPath $model -Algorithm SHA256).Hash -ne $manifest.model_sha256){throw 'Auto Sync model digest mismatch'}
    foreach($name in @('ggml-base.bin','manifest.json','LICENSE-ggml.txt','LICENSE-model.txt','LICENSE-whisper.cpp.txt','README.txt')){
        Copy-Item -LiteralPath (Join-Path $sync $name) -Destination (Join-Path $syncOutput $name) -Force
    }
    foreach($entry in $manifest.files.PSObject.Properties){
        $name=$entry.Name
        if($name -notmatch '^[a-zA-Z0-9_-]+\.(dll|exe)$'){throw 'Invalid Auto Sync runtime filename'}
        $file=Join-Path $sync ('Release/'+$name)
        if((Get-Item -LiteralPath $file).Length -ne $entry.Value.bytes -or (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ne $entry.Value.sha256){throw "Auto Sync runtime digest mismatch: $name"}
        Copy-Item -LiteralPath $file -Destination (Join-Path $syncOutput ('Release/'+$name)) -Force
    }
}
foreach($relative in @('vibefy.ico','yt-dlp.exe','auto-sync/manifest.json','auto-sync/ggml-base.bin','auto-sync/Release/whisper-cli.exe')){
    if(!(Test-Path -LiteralPath (Join-Path $distribution $relative) -PathType Leaf)){
        throw "Missing package runtime: $relative. Supply -RuntimeDir with a verified Vibefy runtime bundle."
    }
}
$output=Join-Path $source 'dist'
New-Item -ItemType Directory -Force $output | Out-Null
& $Iscc ('/DVersion='+$Version) ('/DAppDir='+$distribution) ('/DOutputDir='+$output) "$PSScriptRoot/vibefy.iss"
if($LASTEXITCODE -ne 0){throw 'Installer build failed'}
$setup=Join-Path $output "vibefy-v$Version-x86_64-pc-windows-msvc-setup.exe"
$hash=(Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText((Join-Path $output 'checksums.txt'),($hash+'  '+[IO.Path]::GetFileName($setup)+[Environment]::NewLine),[Text.UTF8Encoding]::new($false))
@{version=$Version;path=$setup;bytes=(Get-Item -LiteralPath $setup).Length;sha256=$hash;signature=(Get-AuthenticodeSignature -LiteralPath $setup).Status.ToString()} | ConvertTo-Json
