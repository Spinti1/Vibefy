[CmdletBinding(SupportsShouldProcess)]
param()
$ErrorActionPreference='Stop'
$source=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$target=Join-Path $source 'target'
$cache=Join-Path $source '.cache'
$dist=Join-Path $source 'dist'
$version=([regex]::Match((Get-Content -LiteralPath (Join-Path $source 'Cargo.toml') -Raw),'(?m)^version = "([0-9.]+)"')).Groups[1].Value
if($version -notmatch '^\d+\.\d+\.\d+$'){throw 'Cannot determine the current version'}
$latest=Join-Path $dist "vibefy-v$version-x86_64-pc-windows-msvc-setup.exe"
if(!(Test-Path -LiteralPath $latest)){throw 'Build the current installer before cleaning artifacts'}
if(Get-Process cargo,rustc,link,ISCC -ErrorAction SilentlyContinue){throw 'Wait for active compilation and packaging to finish'}

function Remove-Artifact([string]$path,[string]$boundary) {
    if(!(Test-Path -LiteralPath $path)){return}
    $resolved=(Resolve-Path -LiteralPath $path).Path
    $parent=(Resolve-Path -LiteralPath $boundary).Path
    if(!$resolved.StartsWith($parent+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw "Artifact leaves its allowed directory: $resolved"}
    $item=Get-Item -LiteralPath $resolved -Force
    if($item.Attributes -band [IO.FileAttributes]::ReparsePoint){throw "Refusing linked artifact: $resolved"}
    if($item.PSIsContainer){
        $stack=[Collections.Generic.Stack[string]]::new()
        $stack.Push($resolved)
        while($stack.Count){
            foreach($child in Get-ChildItem -LiteralPath $stack.Pop() -Force){
                if($child.Attributes -band [IO.FileAttributes]::ReparsePoint){throw "Refusing artifact tree containing a link: $resolved"}
                if($child.PSIsContainer){$stack.Push($child.FullName)}
            }
        }
    }
    if($PSCmdlet.ShouldProcess($resolved,'Delete regenerable development artifact')){Remove-Item -LiteralPath $resolved -Recurse -Force}
}

# Keep the production executable and newest Setup. User data lives outside these roots.
foreach($relative in @('debug','release\deps','release\build','release\.fingerprint','release\incremental','release\examples')){
    Remove-Artifact (Join-Path $target $relative) $target
}
$release=Join-Path $target 'release'
if(Test-Path -LiteralPath $release){
    Get-ChildItem -LiteralPath $release -File | Where-Object {$_.Extension -in @('.rlib','.rmeta','.d','.pdb')} | ForEach-Object {Remove-Artifact $_.FullName $target}
}
Remove-Artifact (Join-Path $cache 'distribution') $cache
if(Test-Path -LiteralPath $cache){
    Get-ChildItem -LiteralPath $cache -File | Where-Object {
        $_.Name -match '^(unit-[a-z-]+|art-palette-tests|builtin-selector-tests|visual-cover-tests|checkpoint-tests|topbar-layout-tests)\.(exe|pdb)$' -or
        $_.Name -match '^app-(baseline|before-[a-z-]+|reconstructed(-\d+)?|repaired-checkpoint)\.rs$' -or
        $_.Name -in @('app.rs.seventh-backup','gh.zip','inno-bootstrap.exe')
    } | ForEach-Object {Remove-Artifact $_.FullName $cache}
}
if(Test-Path -LiteralPath $dist){
    Get-ChildItem -LiteralPath $dist -File | Where-Object {$_.Name -match '^vibefy-v\d+\.\d+\.\d+-x86_64-pc-windows-msvc-setup\.exe$' -and $_.FullName -ne $latest} | ForEach-Object {Remove-Artifact $_.FullName $dist}
}
Write-Output 'Build artifacts cleaned. The current executable, latest installer, sources and user data are retained.'
