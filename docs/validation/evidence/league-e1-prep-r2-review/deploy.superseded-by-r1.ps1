# NHL21-E1 live install / rollback of databigs\inxp_00.big - hash-gated, rename-based.
# NOT RUN by E1-PREP. Requires explicit Codex approval and a stopped emulator.
#
#   .\deploy.ps1 -Mode preflight   # checks only
#   .\deploy.ps1 -Mode install     # live := X candidate (original kept beside it, renamed)
#   .\deploy.ps1 -Mode rollback    # live := original (candidate removed after hash check)
#
# The live file is only ever swapped by same-directory renames, so `inxp_00.big` is never a
# partially written file: the incoming bytes are fully copied and hash-verified under another
# name first. Every step refuses to proceed on an unexpected hash.
param([ValidateSet('preflight','install','rollback')][string]$Mode = 'preflight')
$ErrorActionPreference = 'Stop'
$Live      = 'H:\Emulators\ShadPS4\Games\CUSA18182\databigs\inxp_00.big'
$Original  = 'H:\Emulators\ShadPS4\Games\CUSA18182\databigs\inxp_00.big.e1-original'
$Incoming  = 'H:\Emulators\ShadPS4\Games\CUSA18182\databigs\inxp_00.big.e1-incoming'
$Candidate = 'E:\Repositories\nhl-modding-studio\local\runs\league-nhl21-e1\artifacts\inxp_00.big.X-candidate'
$Rollback2 = 'E:\Repositories\nhl-modding-studio\local\runs\league-nhl21-e1\copied\CUSA18182\databigs\inxp_00.big'
$SHA_ORIG  = '1c41f578a595dbd5aae7ad921445626636f3a842d8fb869bc054d35e3e849e87'
$SHA_X     = '4876cd2b3f4b51878fcbf08f8a56249df2eecbc6c086340b869fc9eada9da582'
$LEN       = 777373328
$LooseGame = 'H:\Emulators\ShadPS4\Games\CUSA18182\nhl\data\common\gamemodes'
$Log       = 'E:\Repositories\nhl-modding-studio\local\runs\league-nhl21-e1\deploy.log'

function L($m) { $l = "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') [$Mode] $m"; Add-Content -LiteralPath $Log $l; Write-Host $l }
function Sha($p) { (Get-FileHash -Algorithm SHA256 -LiteralPath $p).Hash.ToLower() }
function Expect($p, $sha) {
  if (-not (Test-Path -LiteralPath $p -PathType Leaf)) { throw "missing: $p" }
  $len = (Get-Item -LiteralPath $p).Length; if ($len -ne $LEN) { throw "length $len != $LEN : $p" }
  $h = Sha $p; if ($h -ne $sha) { throw "REFUSED: $p hashes $h, expected $sha" }
  L "ok $p = $sha"
}
function Preflight {
  if (Get-Process shadPS4 -ErrorAction SilentlyContinue) { throw 'emulator is running; stop it first' }
  if (Test-Path -LiteralPath $LooseGame) { throw "loose game-tree DB path exists: $LooseGame (must be absent for E1)" }
  $mods = "$env:APPDATA\shadPS4\mods_folder"; if ((Get-ChildItem $mods -Force -ErrorAction SilentlyContinue | Measure-Object).Count -gt 0) { throw 'mods_folder is not empty' }
  foreach ($d in 'patches','cheats','custom_configs') { if ((Get-ChildItem "$env:APPDATA\shadPS4\$d" -Force -ErrorAction SilentlyContinue | Measure-Object).Count -gt 0) { throw "$d is not empty" } }
  $free = (Get-PSDrive H).Free; L "H: free $([int]($free/1MB)) MB (need > $([int]($LEN/1MB)) MB for the incoming copy)"; if ($free -lt ($LEN + 256MB)) { throw 'insufficient free space on H:' }
  L "profile: $(Test-Path "$env:APPDATA\shadPS4\savedata\1\CUSA18182\Profile0\DATA")  cache: $(Test-Path "$env:APPDATA\shadPS4\cache\CUSA18182.zip")"
}
Preflight
switch ($Mode) {
  'preflight' {
    Expect $Live $SHA_ORIG; Expect $Candidate $SHA_X; Expect $Rollback2 $SHA_ORIG
    if (Test-Path -LiteralPath $Original) { L "note: $Original already exists (a previous install not rolled back?)" }
    L 'preflight passed; nothing changed'
  }
  'install' {
    Expect $Live $SHA_ORIG
    if (Test-Path -LiteralPath $Original) { throw "refusing: $Original already exists" }
    if (Test-Path -LiteralPath $Incoming) { Remove-Item -LiteralPath $Incoming -Force }
    Expect $Candidate $SHA_X
    L 'copying candidate to incoming (same directory as live)'
    Copy-Item -LiteralPath $Candidate -Destination $Incoming
    Expect $Incoming $SHA_X
    L 'rename live -> original (atomic, same directory)'
    Rename-Item -LiteralPath $Live -NewName (Split-Path $Original -Leaf)
    L 'rename incoming -> live (atomic)'
    Rename-Item -LiteralPath $Incoming -NewName (Split-Path $Live -Leaf)
    Expect $Live $SHA_X; Expect $Original $SHA_ORIG
    L 'INSTALLED: live = X candidate; original retained beside it'
  }
  'rollback' {
    if (-not (Test-Path -LiteralPath $Original)) { throw "no $Original to roll back from (second source: $Rollback2)" }
    Expect $Original $SHA_ORIG
    $h = Sha $Live; L "live currently $h"
    if ($h -ne $SHA_X -and $h -ne $SHA_ORIG) { throw "REFUSED: live archive is neither the candidate nor the original ($h); investigate before touching it" }
    if ($h -eq $SHA_ORIG) { L 'live is already the original; removing nothing'; break }
    $used = "$Live.e1-candidate-used"
    Rename-Item -LiteralPath $Live -NewName (Split-Path $used -Leaf)
    Rename-Item -LiteralPath $Original -NewName (Split-Path $Live -Leaf)
    Expect $Live $SHA_ORIG
    Expect $used $SHA_X; Remove-Item -LiteralPath $used -Force
    L 'ROLLED BACK: live = original (verified); used candidate removed'
  }
}

