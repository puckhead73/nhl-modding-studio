param(
    [string]$TrialRoot = (Join-Path $PSScriptRoot '../../local/nhl27-eboot/compatibility/deke-trial/trial-package'),
    [string]$Runtime = 'F:\Games\NHL 24\nhl24.exe'
)
$ErrorActionPreference = 'Stop'
$TrialRoot = (Resolve-Path -LiteralPath $TrialRoot).Path
$expectedRuntime = '0da12e71bf4f0acea34e758f46b849ecd7f0b90a3e3570ddb1dc71dd5bfc01ce'
if ((Get-FileHash -LiteralPath $Runtime -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedRuntime) {
    throw 'Runtime differs from the audited build.'
}
$receipt = Get-Content -LiteralPath (Join-Path $TrialRoot 'package-receipt.json') -Raw | ConvertFrom-Json
$toc = Join-Path $TrialRoot 'game-mods/Patch/Gen4b/levels/game/game.toc'
if ((Get-FileHash -LiteralPath $toc -Algorithm SHA256).Hash.ToLowerInvariant() -ne $receipt.toc_sha256) {
    throw 'Trial TOC differs from the validated package.'
}
$cas = Join-Path $TrialRoot 'game-mods/Patch/Gen4b/superbundlelayout/nhl_installpackage_01/cas_01.cas'
if ((Get-FileHash -LiteralPath $cas -Algorithm SHA256).Hash.ToLowerInvariant() -ne $receipt.cas_output_sha256) {
    throw 'Trial CAS differs from the validated package.'
}
foreach ($required in @('game/eboot.bin','game-UPDATE/eboot.bin','user/config.json','user/patches/nhl24.xml')) {
    if (!(Test-Path -LiteralPath (Join-Path $TrialRoot $required))) { throw "Missing trial prerequisite: $required" }
}
# The private working directory selects private saves/configuration; game junctions
# supply original read-only base/update files, and game-mods supplies the trial files.
Write-Host 'Starting the 15-clip motion-only trial. New deke logic and full NHL27 parity are not included.'
Push-Location -LiteralPath $TrialRoot
try {
    & $Runtime -g (Join-Path $TrialRoot 'game/eboot.bin') --override-root (Join-Path $TrialRoot 'game') -p nhl24.xml --fullscreen false
} finally { Pop-Location }
