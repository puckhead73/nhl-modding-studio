param([string]$TrialRoot = (Join-Path $PSScriptRoot '../../local/nhl27-eboot/compatibility/deke-trial/expanded-trial'), [switch]$WaitForExit)
$ErrorActionPreference = 'Stop'
$TrialRoot = (Resolve-Path -LiteralPath $TrialRoot).Path
$runtimePath = 'F:\Games\NHL 24\nhl24.exe'
if ((Get-FileHash -LiteralPath $runtimePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne '0da12e71bf4f0acea34e758f46b849ecd7f0b90a3e3570ddb1dc71dd5bfc01ce') { throw 'Runtime build changed.' }
$receipt = Get-Content -LiteralPath (Join-Path $TrialRoot 'package-receipt.json') -Raw | ConvertFrom-Json
foreach ($bank in $receipt.banks) {
    if ((Get-FileHash -LiteralPath (Join-Path $TrialRoot ('game-mods/' + $bank.toc)) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $bank.toc_sha256) { throw 'Trial TOC changed.' }
}
foreach ($cas in $receipt.cas) {
    if ((Get-FileHash -LiteralPath (Join-Path $TrialRoot ('game-mods/' + $cas.path)) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $cas.sha256) { throw 'Trial CAS changed.' }
}
foreach ($relative in @('game/eboot.bin','game-UPDATE/eboot.bin','user/config.json','user/patches/nhl24.xml')) {
    if (!(Test-Path -LiteralPath (Join-Path $TrialRoot $relative))) { throw "Missing trial prerequisite: $relative" }
}
# Launch in the private directory so installed saves/configuration remain isolated.
$arguments = @('-g',('"' + (Join-Path $TrialRoot 'game/eboot.bin') + '"'),'--override-root',('"' + (Join-Path $TrialRoot 'game') + '"'),'-p','nhl24.xml','--fullscreen','false')
$trialProcess = Start-Process -FilePath $runtimePath -WorkingDirectory $TrialRoot -ArgumentList $arguments -WindowStyle Hidden -PassThru
@{pid=$trialProcess.Id;started_utc=[DateTime]::UtcNow.ToString('o');package=$TrialRoot;status='running_unvalidated'} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $TrialRoot 'runtime-session.json')
Write-Output "Expanded animation/metadata trial PID: $($trialProcess.Id)"
if ($WaitForExit) { $trialProcess.WaitForExit(); Write-Output "Runtime exit code: $($trialProcess.ExitCode)" }
