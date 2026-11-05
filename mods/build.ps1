param([string]$Output = "$PSScriptRoot/packages")
$ErrorActionPreference='Stop'
New-Item -ItemType Directory -Force $Output | Out-Null
foreach($id in @('nyan','wavelink','juicevault','oneko','catjam','youtube','trackpeek')) {
 & rustc --edition 2024 --crate-type cdylib --target wasm32-unknown-unknown --cfg $id -C opt-level=z -C panic=abort -C strip=symbols "$PSScriptRoot/src/module.rs" -o "$Output/$id.wasm"
 if($LASTEXITCODE -ne 0){throw "Could not compile $id"}
}
