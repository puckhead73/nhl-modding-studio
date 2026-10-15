# NHL21-E1 live install / rollback of databigs\inxp_00.big - rerunnable state machine (R2).
# NOT RUN live by E1-PREP-R1. Requires explicit Codex approval and a stopped emulator.
#
#   .\deploy.ps1 -Mode status     # classify live + sidecars, report what each mode would do (no mutation)
#   .\deploy.ps1 -Mode install    # live := X candidate; original kept beside it (renamed)
#   .\deploy.ps1 -Mode rollback   # live := verified original (primary source, else secondary copy)
#
# Files (same directory as Live): Original = Live + ".e1-original", Incoming = Live + ".e1-incoming",
# Used = Live + ".e1-candidate-used-<timestamp>". Every file is classified by exact length+SHA-256
# as original / candidate / unknown / absent; unknown bytes are never deleted, renamed or overwritten.
# Two renames are NOT one atomic operation: every invocation re-derives the state from disk, so a
# process killed between renames is recovered by simply running the wanted mode again.
#
# State table (L=Live, O=Original, I=Incoming, U=any Used):
#   S0 stock            L=orig O=-    : install -> normal; rollback -> no-op
#   S1 installed        L=X    O=orig : install -> no-op; rollback -> normal (L->U, O->L, drop U)
#   S2 install-mid      L=-    O=orig I=X : install -> finish (I->L) ONLY if the environment gates pass, else refuse and direct rollback; rollback -> O->L, drop I
#   S3 rollback-mid     L=-    O=orig I=- : rollback -> O->L; install -> refused (rollback first)
#   S4 no-primary       L=X    O=-    : rollback -> secondary copy (S->I, verify, L->U, I->L); install -> no-op
#   S5 fallback-mid     L=-    O=-    I=orig : rollback -> I->L
#   S6 fallback-needed  L=-    O=-    I=- : rollback -> secondary copy then I->L (needs space)
#   S7 dup-original     L=orig O=orig : install refused (ambiguous); rollback no-op (original already live; sidecar preserved)
#   any unknown L/O/I   : the affected transitions are refused with the file preserved and named
#
# Exit codes: 0 done/no-op, 2 refused or blocked (nothing mutated in that step), 3 injected test failure.
[CmdletBinding()]
param(
  [ValidateSet('status','install','rollback')][string]$Mode = 'status',
  [ValidateSet('live','fixture')][string]$Profile = 'live',
  [string]$Root = '',                 # fixture profile only: every path is derived under it
  [string]$FailAfter = '',            # test hook: exit 3 after step 'copy' | 'rename1' | 'rename2'
  # fixture-only test hooks via environment: E1_TEST_EMU_RUNNING=1, E1_TEST_FREE_BYTES=<bytes>,
  # E1_TEST_ENUM_FAIL=<user-data folder name> (injected enumeration failure; labelled as injected)
  [string]$ShaOrig = '', [string]$ShaX = '', [long]$Len = 0,   # fixture overrides
  [string]$ExpectedExeSha = ''        # optional install gate (live)
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2

# ---------- profile / paths ----------
if ($Profile -eq 'live') {
  if ($Root -or $FailAfter -or $ShaOrig -or $ShaX -or $Len -or $env:E1_TEST_EMU_RUNNING -or $env:E1_TEST_FREE_BYTES -or $env:E1_TEST_ENUM_FAIL) { throw 'live profile does not accept fixture overrides or test hooks' }
  $Live      = 'H:\Emulators\ShadPS4\Games\CUSA18182\databigs\inxp_00.big'
  $Candidate = 'E:\Repositories\nhl-modding-studio\local\runs\league-nhl21-e1\artifacts\inxp_00.big.X-candidate'
  $Secondary = 'E:\Repositories\nhl-modding-studio\local\runs\league-nhl21-e1\copied\CUSA18182\databigs\inxp_00.big'
  $UserDir   = "$env:APPDATA\shadPS4"
  $LooseGame = 'H:\Emulators\ShadPS4\Games\CUSA18182\nhl\data\common\gamemodes'
  $Log       = 'E:\Repositories\nhl-modding-studio\local\runs\league-nhl21-e1\deploy.log'
  $ConfigPre = 'E:\Repositories\nhl-modding-studio\docs\validation\evidence\league-nhl21-runtime\e0\config.pre.toml'
  $ExePath   = 'H:\Emulators\ShadPS4\shadPS4.exe'
  $SHA_ORIG  = '1c41f578a595dbd5aae7ad921445626636f3a842d8fb869bc054d35e3e849e87'
  $SHA_X     = '4876cd2b3f4b51878fcbf08f8a56249df2eecbc6c086340b869fc9eada9da582'
  $LEN       = 777373328
} else {
  if (-not $Root) { throw 'fixture profile needs -Root' }
  $Root = [IO.Path]::GetFullPath($Root)
  if ($Root -match '^[Hh]:\\' -or $Root -match 'ShadPS4' -or $Root -match '\\anchors\\') { throw "fixture root refused: $Root" }
  if (-not $ShaOrig -or -not $ShaX -or $Len -le 0) { throw 'fixture profile needs -ShaOrig -ShaX -Len' }
  $Live      = Join-Path $Root 'databigs\inxp_00.big'
  $Candidate = Join-Path $Root 'artifacts\candidate'
  $Secondary = Join-Path $Root 'secondary\inxp_00.big'
  $UserDir   = Join-Path $Root 'user'
  $LooseGame = Join-Path $Root 'nhl\data\common\gamemodes'
  $Log       = Join-Path $Root 'deploy.log'
  $ConfigPre = Join-Path $Root 'config.pre.toml'
  $ExePath   = Join-Path $Root 'emu\shadPS4.exe'
  $SHA_ORIG = $ShaOrig.ToLower(); $SHA_X = $ShaX.ToLower(); $LEN = $Len
}
$Dir      = Split-Path $Live -Parent
$Original = "$Live.e1-original"
$Incoming = "$Live.e1-incoming"
$UsedGlob = (Split-Path $Live -Leaf) + '.e1-candidate-used-*'
foreach ($p in @($Live, $Candidate, $Secondary)) { if ([IO.Path]::GetFullPath($p) -ne $p) { throw "path not absolute/normalized: $p" } }
if ($Profile -eq 'fixture') { foreach ($p in @($Live, $Candidate, $Secondary, $UserDir, $LooseGame, $Log)) { if (-not $p.StartsWith($Root)) { throw "fixture path escapes root: $p" } } }

# ---------- helpers ----------
function L($m) { $l = "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') [$Mode] $m"; try { Add-Content -LiteralPath $Log $l } catch {}; Write-Host $l }
function Fail($code, $m) { L "REFUSED: $m"; exit $code }
function Sha($p) { (Get-FileHash -Algorithm SHA256 -LiteralPath $p).Hash.ToLower() }
function IsReparse($p) { try { ((Get-Item -LiteralPath $p -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 } catch { $false } }
function Classify($p) {
  # -> @{ Path; Exists; Kind = absent|original|candidate|unknown|not-a-file|reparse; Sha; Len }
  $r = @{ Path = $p; Exists = $false; Kind = 'absent'; Sha = ''; Len = 0 }
  if (-not (Test-Path -LiteralPath $p)) { return $r }
  $r.Exists = $true
  if (IsReparse $p) { $r.Kind = 'reparse'; return $r }
  if (-not (Test-Path -LiteralPath $p -PathType Leaf)) { $r.Kind = 'not-a-file'; return $r }
  $r.Len = (Get-Item -LiteralPath $p -Force).Length
  if ($r.Len -ne $LEN) { $r.Kind = 'unknown'; return $r }
  $r.Sha = Sha $p
  $r.Kind = if ($r.Sha -eq $SHA_ORIG) { 'original' } elseif ($r.Sha -eq $SHA_X) { 'candidate' } else { 'unknown' }
  return $r
}
function Describe($c) { if ($c.Kind -eq 'absent') { 'absent' } else { "$($c.Kind) (len $($c.Len)$(if ($c.Sha) { ", sha $($c.Sha.Substring(0,16))..." }))" } }
function EmuRunning { if ($Profile -eq 'fixture' -and $env:E1_TEST_EMU_RUNNING) { $env:E1_TEST_EMU_RUNNING -eq '1' } else { [bool](Get-Process shadPS4 -ErrorAction SilentlyContinue) } }
function Free($d) { if ($Profile -eq 'fixture' -and $env:E1_TEST_FREE_BYTES) { [long]$env:E1_TEST_FREE_BYTES } else { (Get-PSDrive ($d.Substring(0,1))).Free } }
function AssertStopped($when) { if (EmuRunning) { Fail 2 "emulator is running ($when); nothing mutated" } }
function Rename($from, $to, $step) {
  AssertStopped "before $step"
  Move-Item -LiteralPath $from -Destination $to
  L "$step`: renamed $(Split-Path $from -Leaf) -> $(Split-Path $to -Leaf)"
  if ($FailAfter -eq $step) { L "INJECTED FAILURE after $step"; exit 3 }
}
function CopyVerify($from, $to, $wantSha, $step) {
  AssertStopped "before $step"
  $need = $LEN + 64MB; $free = Free $Dir
  if ($free -lt $need) { Fail 2 "$step needs $([int]($need/1MB)) MB free in $Dir, have $([int]($free/1MB)) MB; nothing mutated" }
  Copy-Item -LiteralPath $from -Destination $to
  L "$step`: copied $(Split-Path $from -Leaf) -> $(Split-Path $to -Leaf) ($LEN B)"
  $c = Classify $to
  if ($c.Sha -ne $wantSha) { Fail 2 "$step`: copy verification failed ($($c.Kind)); $to left in place for inspection" }
  L "$step`: verified $(Split-Path $to -Leaf) = $wantSha"
  if ($FailAfter -eq $step) { L "INJECTED FAILURE after $step"; exit 3 }
}
function DropKnownCandidate($c, $why) {
  # Only bytes proven to be the candidate (which also exists under artifacts/) are removed.
  if ($c.Kind -eq 'candidate') { Remove-Item -LiteralPath $c.Path -Force; L "removed $(Split-Path $c.Path -Leaf) ($why; verified candidate bytes)" }
  elseif ($c.Exists) { L "PRESERVED $(Split-Path $c.Path -Leaf): $(Describe $c) - not the candidate; inspect by hand" }
}

# ---------- read disk state ----------
if (-not (Test-Path -LiteralPath $Dir -PathType Container)) { Fail 2 "live directory missing: $Dir" }
if (IsReparse $Dir) { Fail 2 "live directory is a reparse point: $Dir" }
$cL = Classify $Live; $cO = Classify $Original; $cI = Classify $Incoming
$cU = @(Get-ChildItem -LiteralPath $Dir -Filter $UsedGlob -Force -ErrorAction SilentlyContinue | ForEach-Object { Classify $_.FullName })
$cC = Classify $Candidate; $cS = Classify $Secondary
L "state: Live=$(Describe $cL) | Original=$(Describe $cO) | Incoming=$(Describe $cI) | Used=[$(($cU | ForEach-Object { "$(Split-Path $_.Path -Leaf)=$($_.Kind)" }) -join '; ')] | Candidate=$(Describe $cC) | Secondary=$(Describe $cS)"
foreach ($c in @($cL, $cO, $cI) + $cU) { if ($c.Kind -in 'reparse','not-a-file') { Fail 2 "$($c.Path) is $($c.Kind); refusing to interpret this directory" } }
$state = if ($cL.Kind -eq 'unknown') { 'LIVE-UNKNOWN' }
  elseif ($cL.Kind -eq 'original' -and $cO.Kind -eq 'absent') { 'S0-stock' }
  elseif ($cL.Kind -eq 'original' -and $cO.Kind -eq 'original') { 'S7-dup-original' }
  elseif ($cL.Kind -eq 'original') { 'S0-stock+odd-original' }
  elseif ($cL.Kind -eq 'candidate' -and $cO.Kind -eq 'original') { 'S1-installed' }
  elseif ($cL.Kind -eq 'candidate') { 'S4-no-primary' }
  elseif ($cO.Kind -eq 'original' -and $cI.Kind -eq 'candidate') { 'S2-install-mid' }
  elseif ($cO.Kind -eq 'original') { 'S3-rollback-mid' }
  elseif ($cI.Kind -eq 'original') { 'S5-fallback-mid' }
  else { 'S6-fallback-needed' }
L "derived state: $state"

# ---------- environment qualification (automated) ----------
# Gates that apply to ANY install completion (fresh or resumed): emulator, loose dir, overlays,
# config drift, exe identity. `-NeedCopy` adds the candidate-artifact check; free space is checked
# by CopyVerify only when a copy actually happens.
$script:ok = $true
$script:checkLines = @()
function Chk($name, $pass, $detail) { $script:ok = $script:ok -and $pass; $script:checkLines += ("check {0,-28} {1}  {2}" -f $name, $(if ($pass) { 'PASS' } else { 'FAIL' }), $detail) }
function EnvChecks([switch]$NeedCopy) {
  $script:ok = $true; $script:checkLines = @()
  Chk 'emulator stopped' (-not (EmuRunning)) ''
  Chk 'no loose game-tree DB dir' (-not (Test-Path -LiteralPath $LooseGame)) $LooseGame
  foreach ($d in 'mods_folder','patches','cheats','custom_configs') {
    $p = Join-Path $UserDir $d
    try {
      if ($Profile -eq 'fixture' -and $env:E1_TEST_ENUM_FAIL -eq $d) { throw "INJECTED enumeration failure (fixture seam E1_TEST_ENUM_FAIL=$d)" }
      if (-not (Test-Path -LiteralPath $p -PathType Container)) { Chk "$d exists" $false "$p missing or not a directory"; continue }
      $n = @(Get-ChildItem -LiteralPath $p -Force -ErrorAction Stop).Count
      Chk "$d empty" ($n -eq 0) "$n entries"
    } catch { Chk "$d readable" $false "enumeration failed: $($_.Exception.Message)" }
  }
  if ($NeedCopy) { Chk 'candidate verified' ($cC.Kind -eq 'candidate') (Describe $cC) }
  if ($ExpectedExeSha -and (Test-Path -LiteralPath $ExePath)) { Chk 'emulator exe identity' ((Sha $ExePath) -eq $ExpectedExeSha.ToLower()) $ExePath }
  elseif ($ExpectedExeSha) { Chk 'emulator exe identity' $false "$ExePath missing" }
  else { $script:checkLines += 'check emulator exe identity          MANUAL  (pass -ExpectedExeSha to automate; record from the run log)' }
  $cfg = Join-Path $UserDir 'config.toml'
  if ((Test-Path -LiteralPath $ConfigPre) -and (Test-Path -LiteralPath $cfg)) {
    $a = @(Get-Content -LiteralPath $ConfigPre | Where-Object { $_ -notmatch '^windowPos[XY] = ' })
    $b = @(Get-Content -LiteralPath $cfg | Where-Object { $_ -notmatch '^windowPos[XY] = ' })
    $dd = @(Compare-Object $a $b).Count
    Chk 'config unchanged vs E0' ($dd -eq 0) "$dd differing lines (window position ignored)"
  } else { $script:checkLines += 'check config unchanged vs E0        MANUAL  (no recorded pre-run config to compare)' }
  return $script:ok
}

# ---------- shared transition logic ----------
# Plan($mode) -> @{ Action = 'mutate'|'noop'|'refuse'; Reason; Steps (human list); Gate = 'env'|'env+copy'|'none' }
# `status` prints the plan; `install`/`rollback` execute exactly the plan they print.
function Plan($which) {
  $p = @{ Action = 'refuse'; Reason = ''; Steps = @(); Gate = 'none' }
  if ($which -eq 'install') {
    switch -Wildcard ($state) {
      'S1-installed'   { $p.Action = 'noop'; $p.Reason = 'already installed (live = candidate, original beside it)' }
      'S4-no-primary'  { $p.Action = 'noop'; $p.Reason = 'candidate already live; no primary rollback source (rollback would use the secondary source)' }
      'S2-install-mid' {
        $p.Gate = 'env'; $p.Action = 'mutate'; $p.Reason = 'resume interrupted install: incoming is the verified candidate, original is verified'
        $p.Steps = @('rename incoming -> live', 'verify live = candidate')
      }
      'S3-rollback-mid' { $p.Reason = 'live is absent with a verified original beside it; run -Mode rollback first, then install from stock' }
      'S7-dup-original' { $p.Reason = 'live and original are both the original; ambiguous prior state - inspect and remove one by hand' }
      'S0-stock+odd-original' { $p.Reason = "original sidecar exists but is $(Describe $cO); preserved - inspect by hand" }
      'S0-stock' {
        if ($cI.Kind -eq 'unknown') { $p.Reason = "incoming sidecar holds unknown bytes ($(Describe $cI)); preserved - move it away by hand" }
        else {
          $p.Action = 'mutate'
          if ($cI.Kind -eq 'candidate') { $p.Gate = 'env'; $p.Reason = 'install from stock, reusing the existing verified incoming candidate (no copy)'; $p.Steps = @('rename live -> original', 'rename incoming -> live', 'verify') }
          else { $p.Gate = 'env+copy'; $p.Reason = 'install from stock'; $p.Steps = @('copy candidate -> incoming (needs free space)', 'verify incoming = candidate', 'rename live -> original', 'rename incoming -> live', 'verify') }
        }
      }
      default { $p.Reason = "state ${state}: live=$(Describe $cL); install refuses" }
    }
    if ($p.Action -eq 'mutate') {
      $envOk = if ($p.Gate -eq 'env+copy') { EnvChecks -NeedCopy } else { EnvChecks }
      if (-not $envOk) {
        $p.Action = 'refuse'
        $p.Reason = "environment qualification FAILED (see checks); files preserved" + $(if ($state -eq 'S2-install-mid') { '; recovery action: -Mode rollback (restores the verified original without these gates)' } else { '' })
        $p.Steps = @()
      }
    }
  } else {  # rollback
    switch -Wildcard ($state) {
      'LIVE-UNKNOWN'    { $p.Reason = "live holds unknown bytes ($(Describe $cL)); refusing to touch it - investigate" }
      'S7-dup-original' { $p.Action = 'noop'; $p.Reason = 'live is already the original; the duplicate original sidecar is preserved (remove by hand if unwanted)' }
      'S0-stock*'       { $p.Action = 'noop'; $p.Reason = 'live is already the original' + $(if ($cI.Kind -eq 'candidate' -or ($cU | Where-Object { $_.Kind -eq 'candidate' })) { '; stale verified-candidate sidecars will be removed' } else { '' }) + $(if ($cI.Kind -eq 'unknown' -or ($cU | Where-Object { $_.Kind -eq 'unknown' })) { '; unknown sidecars stay preserved' } else { '' }) }
      'S1-installed'    { $p.Action = 'mutate'; $p.Reason = 'restore from the primary source'; $p.Steps = @('rename live -> used', 'rename original -> live', 'verify live = original', 'remove used (only if it is the verified candidate)') }
      'S2-install-mid'  { $p.Action = 'mutate'; $p.Reason = 'undo interrupted install'; $p.Steps = @('rename original -> live', 'verify', 'remove incoming (verified candidate)') }
      'S3-rollback-mid' { $p.Action = 'mutate'; $p.Reason = 'finish interrupted rollback / restore missing live'; $p.Steps = @('rename original -> live', 'verify', 'remove used/incoming only if verified candidate') }
      'S5-fallback-mid' { $p.Action = 'mutate'; $p.Reason = 'finish interrupted fallback'; $p.Steps = @('rename incoming -> live', 'verify') }
      { $_ -in 'S4-no-primary','S6-fallback-needed' } {
        if ($cS.Kind -ne 'original') { $p.Reason = "no primary rollback source and the secondary source is $(Describe $cS); live=$(Describe $cL). Nothing will be mutated; supply a verified original ($SHA_ORIG)" }
        elseif ($cI.Kind -eq 'unknown') { $p.Reason = "incoming sidecar holds unknown bytes ($(Describe $cI)); preserved - move it away by hand before the fallback copy" }
        else {
          $p.Action = 'mutate'; $p.Reason = 'restore from the secondary source (fallback copy; this branch needs free space)'
          $p.Steps = @($(if ($cI.Kind -eq 'candidate') { 'remove stale incoming (verified candidate)' }), 'copy secondary -> incoming', 'verify incoming = original', $(if ($cL.Kind -eq 'candidate') { 'rename live -> used' }), 'rename incoming -> live', 'verify') | Where-Object { $_ }
          if ($cO.Exists) { $p.Reason += "; original sidecar is $(Describe $cO) and stays preserved" }
        }
      }
      default { $p.Reason = "state $state not handled for rollback" }
    }
    # Rollback prerequisites: only a stopped emulator (re-checked before each mutation).
    if ($p.Action -eq 'mutate' -and (EmuRunning)) { $p.Action = 'refuse'; $p.Reason = 'emulator is running; nothing mutated'; $p.Steps = @() }
  }
  return $p
}
function PrintPlan($which, $p) {
  L "$which plan: $($p.Action.ToUpper()) - $($p.Reason)"
  foreach ($s in $p.Steps) { L "  step: $s" }
}

# ---------- modes ----------
switch ($Mode) {
  'status' {
    $pi = Plan 'install'
    foreach ($l in $script:checkLines) { L $l }
    if ($script:checkLines.Count -gt 0) { L "environment qualification: $(if ($script:ok) { 'PASS' } else { 'FAIL' }) (applies to install completion only)" }
    PrintPlan 'install' $pi
    $pr = Plan 'rollback'
    L "rollback prerequisites: emulator stopped=$(-not (EmuRunning)); primary source=$($cO.Kind); secondary source=$($cS.Kind)"
    PrintPlan 'rollback' $pr
    exit 0
  }
  'install' {
    $p = Plan 'install'
    foreach ($l in $script:checkLines) { L $l }
    PrintPlan 'install' $p
    if ($p.Action -eq 'noop') { exit 0 }
    if ($p.Action -eq 'refuse') { Fail 2 $p.Reason }
    if ($state -eq 'S2-install-mid') {
      Rename $Incoming $Live 'rename2'
      $cL = Classify $Live; if ($cL.Kind -ne 'candidate') { Fail 2 "post-rename live is $($cL.Kind)" }
      L 'INSTALLED (resumed): live = candidate; original retained'; exit 0
    }
    if ($cI.Kind -eq 'candidate') { L 'reusing existing verified incoming candidate (no copy needed)' }
    else { CopyVerify $Candidate $Incoming $SHA_X 'copy' }
    Rename $Live $Original 'rename1'
    Rename $Incoming $Live 'rename2'
    $cL = Classify $Live; $cO = Classify $Original
    if ($cL.Kind -ne 'candidate' -or $cO.Kind -ne 'original') { Fail 2 "post-install verification: live=$(Describe $cL) original=$(Describe $cO)" }
    L 'INSTALLED: live = candidate (verified); original retained beside it (verified)'; exit 0
  }
  'rollback' {
    $p = Plan 'rollback'
    PrintPlan 'rollback' $p
    if ($p.Action -eq 'refuse') { Fail 2 $p.Reason }
    if ($p.Action -eq 'noop') {
      if ($state -like 'S0-stock*') { DropKnownCandidate $cI 'stale incoming'; foreach ($u in $cU) { DropKnownCandidate $u 'stale used' } }
      exit 0
    }
    switch -Wildcard ($state) {
      'S1-installed' {
        $used = "$Live.e1-candidate-used-$(Get-Date -Format 'yyyyMMdd-HHmmss-fff')"
        Rename $Live $used 'rename1'
        Rename $Original $Live 'rename2'
        $cL = Classify $Live; if ($cL.Kind -ne 'original') { Fail 2 "post-rollback live is $(Describe $cL); used candidate kept at $used" }
        DropKnownCandidate (Classify $used) 'used candidate'
        DropKnownCandidate $cI 'stale incoming'
        foreach ($u in $cU) { DropKnownCandidate $u 'pre-existing used sidecar' }
        L 'ROLLED BACK: live = original (verified)'; exit 0
      }
      'S2-install-mid' {
        Rename $Original $Live 'rename2'
        $cL = Classify $Live; if ($cL.Kind -ne 'original') { Fail 2 "post-rollback live is $(Describe $cL)" }
        DropKnownCandidate $cI 'abandoned incoming'
        L 'ROLLED BACK (from interrupted install): live = original (verified)'; exit 0
      }
      'S3-rollback-mid' {
        Rename $Original $Live 'rename2'
        $cL = Classify $Live; if ($cL.Kind -ne 'original') { Fail 2 "post-rollback live is $(Describe $cL)" }
        foreach ($u in $cU) { DropKnownCandidate $u 'used candidate' }
        DropKnownCandidate $cI 'stale incoming'
        L 'ROLLED BACK (resumed): live = original (verified)'; exit 0
      }
      'S5-fallback-mid' {
        Rename $Incoming $Live 'rename2'
        $cL = Classify $Live; if ($cL.Kind -ne 'original') { Fail 2 "post-rollback live is $(Describe $cL)" }
        foreach ($u in $cU) { DropKnownCandidate $u 'used candidate' }
        L 'ROLLED BACK (fallback resumed): live = original (verified)'; exit 0
      }
      { $_ -in 'S4-no-primary','S6-fallback-needed' } {
        if ($cO.Exists) { L "PRESERVED original sidecar: $(Describe $cO) - not usable as a rollback source" }
        if ($cI.Kind -eq 'candidate') { DropKnownCandidate $cI 'stale incoming (needed for the fallback copy)' }
        CopyVerify $Secondary $Incoming $SHA_ORIG 'copy'
        $used = ''
        if ($cL.Kind -eq 'candidate') { $used = "$Live.e1-candidate-used-$(Get-Date -Format 'yyyyMMdd-HHmmss-fff')"; Rename $Live $used 'rename1' }
        Rename $Incoming $Live 'rename2'
        $cL = Classify $Live; if ($cL.Kind -ne 'original') { Fail 2 "post-rollback live is $(Describe $cL)" }
        if ($used) { DropKnownCandidate (Classify $used) 'used candidate' }
        foreach ($u in $cU) { DropKnownCandidate $u 'used candidate' }
        L 'ROLLED BACK (from secondary source): live = original (verified)'; exit 0
      }
      default { Fail 2 "state $state not handled for rollback; nothing mutated" }
    }
  }
}
