# NHL21-E1-PREP-R1 state-machine tests for deploy.ps1 on disposable fixtures (4 KB payloads).
# Every case gets its own root under tests\fx\; the helper is invoked as a child process so its
# `exit` codes are observed exactly as an operator would see them. Nothing here can reach H: or the
# real install: the helper refuses fixture roots on H:\ or containing "ShadPS4".
$ErrorActionPreference = 'Stop'
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$Script = Join-Path (Split-Path $Here -Parent) 'deploy.ps1'
$FX = Join-Path $Here 'fx'
if (Test-Path $FX) { Remove-Item -LiteralPath $FX -Recurse -Force }
New-Item -ItemType Directory -Force $FX | Out-Null
$rng = New-Object System.Random(20260918)
$ORIG = New-Object byte[] 4096; $rng.NextBytes($ORIG)
$CAND = [byte[]]$ORIG.Clone(); for ($i = 100; $i -lt 118; $i++) { $CAND[$i] = 0xEE }   # same length, 18 bytes differ
$JUNK = New-Object byte[] 4096; $rng.NextBytes($JUNK)
function ShaB($b) { ($(([Security.Cryptography.SHA256]::Create()).ComputeHash($b)) | ForEach-Object { $_.ToString('x2') }) -join '' }
$SHA_ORIG = ShaB $ORIG; $SHA_X = ShaB $CAND; $SHA_JUNK = ShaB $JUNK
$results = @()
function NewRoot($name, [switch]$NoSecondary, [switch]$NoCandidate) {
  $r = Join-Path $FX $name
  foreach ($d in 'databigs','artifacts','secondary','user\mods_folder','user\patches','user\cheats','user\custom_configs','emu') { New-Item -ItemType Directory -Force (Join-Path $r $d) | Out-Null }
  [IO.File]::WriteAllBytes((Join-Path $r 'databigs\inxp_00.big'), $ORIG)
  if (-not $NoCandidate) { [IO.File]::WriteAllBytes((Join-Path $r 'artifacts\candidate'), $CAND) }
  if (-not $NoSecondary) { [IO.File]::WriteAllBytes((Join-Path $r 'secondary\inxp_00.big'), $ORIG) }
  Set-Content -LiteralPath (Join-Path $r 'user\config.toml') "a = 1`nwindowPosX = 5`n"
  Set-Content -LiteralPath (Join-Path $r 'config.pre.toml') "a = 1`nwindowPosX = 9`n"
  return $r
}
function Put($root, $rel, $bytes) { [IO.File]::WriteAllBytes((Join-Path $root $rel), $bytes) }
function DelFile($root, $rel) { Remove-Item -LiteralPath (Join-Path $root $rel) -Force }
function Run($root, $mode, $failAfter = '', $emu = '', $free = '200000000', $enumFail = '') {
  $env:E1_TEST_EMU_RUNNING = $emu; $env:E1_TEST_FREE_BYTES = $free; $env:E1_TEST_ENUM_FAIL = $enumFail
  $psargs = @('-NoProfile','-ExecutionPolicy','Bypass','-File',$Script,'-Profile','fixture','-Root',$root,'-Mode',$mode,'-ShaOrig',$SHA_ORIG,'-ShaX',$SHA_X,'-Len','4096')
  if ($failAfter) { $psargs += @('-FailAfter',$failAfter) }
  $ErrorActionPreference = 'Continue'
  $out = & powershell.exe @psargs 2>&1 | Out-String
  $code = $LASTEXITCODE
  $ErrorActionPreference = 'Stop'
  $env:E1_TEST_EMU_RUNNING = ''; $env:E1_TEST_FREE_BYTES = ''; $env:E1_TEST_ENUM_FAIL = ''
  return @{ Code = $code; Out = $out }
}
function Kind($root, $rel) {
  $p = Join-Path $root $rel
  if (-not (Test-Path -LiteralPath $p)) { return 'absent' }
  $s = (Get-FileHash -Algorithm SHA256 -LiteralPath $p).Hash.ToLower()
  if ($s -eq $SHA_ORIG) { 'original' } elseif ($s -eq $SHA_X) { 'candidate' } elseif ($s -eq $SHA_JUNK) { 'junk' } else { 'other' }
}
function State($root) {
  $used = @(Get-ChildItem (Join-Path $root 'databigs') -Filter 'inxp_00.big.e1-candidate-used-*' -Force | ForEach-Object { Kind $root "databigs\$($_.Name)" })
  "L=$(Kind $root 'databigs\inxp_00.big') O=$(Kind $root 'databigs\inxp_00.big.e1-original') I=$(Kind $root 'databigs\inxp_00.big.e1-incoming') U=[$($used -join ',')]"
}
function Case($name, [scriptblock]$body) {
  try { $r = & $body; Write-Host "PASS $name  $r" }
  catch { $script:results += ,@($name, 'FAIL', $_.Exception.Message); Write-Host "FAIL $name  $($_.Exception.Message)" -ForegroundColor Red; return }
  $script:results += ,@($name, 'PASS', $r)
}
function Expect($cond, $msg) { if (-not $cond) { throw $msg } }
# status must predict the action the mutation mode then takes: MUTATE -> exit 0 with a state change,
# NOOP -> exit 0 with no state change, REFUSE -> exit 2 with no state change.
function Agree($root, $mode, $expectAction, $reasonPattern = '') {
  $st = Run $root status
  $line = ($st.Out -split "`n" | Where-Object { $_ -match " $mode plan: " } | Select-Object -First 1)
  Expect ($line -match "$mode plan: $expectAction\b") "status predicted: $line (wanted $expectAction)"
  if ($reasonPattern) { Expect ($line -match $reasonPattern) "status reason mismatch: $line" }
  $before = State $root
  $act = Run $root $mode
  switch ($expectAction) {
    'MUTATE' { Expect ($act.Code -eq 0 -and (State $root) -ne $before) "$mode should mutate: code $($act.Code) $($act.Out)" }
    'NOOP'   { Expect ($act.Code -eq 0 -and (State $root) -eq $before) "$mode should be a no-op with exit 0: $($act.Out)" }
    'REFUSE' { Expect ($act.Code -eq 2 -and (State $root) -eq $before) "$mode should refuse (2) without change: code $($act.Code) $($act.Out)" }
  }
  "$mode=$expectAction ok"
}

Case '01 clean install then rollback' {
  $r = NewRoot '01'
  $a = Run $r install; Expect ($a.Code -eq 0) "install code $($a.Code): $($a.Out)"
  Expect ((State $r) -eq 'L=candidate O=original I=absent U=[]') (State $r)
  $b = Run $r rollback; Expect ($b.Code -eq 0) "rollback code $($b.Code)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  "install -> $(State $r)"
}
Case '02 idempotent: rollback on stock, install twice, rollback twice' {
  $r = NewRoot '02'
  Expect ((Run $r rollback).Code -eq 0) 'rollback on stock'; Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  Expect ((Run $r install).Code -eq 0) 'install 1'; Expect ((Run $r install).Code -eq 0) 'install 2 (no-op)'
  Expect ((State $r) -eq 'L=candidate O=original I=absent U=[]') (State $r)
  Expect ((Run $r rollback).Code -eq 0) 'rollback 1'; Expect ((Run $r rollback).Code -eq 0) 'rollback 2 (no-op)'
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '03 install interrupted after copy; re-entry reuses verified incoming' {
  $r = NewRoot '03'
  $a = Run $r install 'copy'; Expect ($a.Code -eq 3) "code $($a.Code)"
  Expect ((State $r) -eq 'L=original O=absent I=candidate U=[]') (State $r)
  $b = Run $r install; Expect ($b.Code -eq 0 -and $b.Out -match 'reusing existing verified incoming') "code $($b.Code)"
  Expect ((State $r) -eq 'L=candidate O=original I=absent U=[]') (State $r)
  State $r
}
Case '04a install interrupted after rename1 (live absent); re-entry install completes' {
  $r = NewRoot '04a'
  $a = Run $r install 'rename1'; Expect ($a.Code -eq 3) "code $($a.Code)"
  Expect ((State $r) -eq 'L=absent O=original I=candidate U=[]') (State $r)
  $s = Run $r status; Expect ($s.Out -match 'S2-install-mid') 'status state'
  $b = Run $r install; Expect ($b.Code -eq 0 -and $b.Out -match 'resumed') "code $($b.Code)"
  Expect ((State $r) -eq 'L=candidate O=original I=absent U=[]') (State $r)
  State $r
}
Case '04b install interrupted after rename1; re-entry rollback restores' {
  $r = NewRoot '04b'
  Expect ((Run $r install 'rename1').Code -eq 3) 'inject'
  $b = Run $r rollback; Expect ($b.Code -eq 0) "code $($b.Code): $($b.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '05 second rename fails (live absent, original + incoming present) then incoming lost' {
  $r = NewRoot '05'
  Expect ((Run $r install 'rename1').Code -eq 3) 'inject'
  DelFile $r 'databigs\inxp_00.big.e1-incoming'            # simulate incoming lost as well
  Expect ((State $r) -eq 'L=absent O=original I=absent U=[]') (State $r)
  $i = Run $r install; Expect ($i.Code -eq 2 -and $i.Out -match 'run -Mode rollback first') "install should refuse: $($i.Code)"
  Expect ((State $r) -eq 'L=absent O=original I=absent U=[]') 'install mutated something'
  $b = Run $r rollback; Expect ($b.Code -eq 0) "rollback code $($b.Code)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '06 rollback interrupted after rename1 (live absent, used present); re-entry completes' {
  $r = NewRoot '06'
  Expect ((Run $r install).Code -eq 0) 'install'
  $a = Run $r rollback 'rename1'; Expect ($a.Code -eq 3) "code $($a.Code)"
  Expect ((State $r) -eq 'L=absent O=original I=absent U=[candidate]') (State $r)
  $i = Run $r install; Expect ($i.Code -eq 2) 'install must refuse mid-rollback'
  $b = Run $r rollback; Expect ($b.Code -eq 0 -and $b.Out -match 'resumed') "code $($b.Code)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '07 missing live with verified original only' {
  $r = NewRoot '07'
  Expect ((Run $r install).Code -eq 0) 'install'; DelFile $r 'databigs\inxp_00.big'
  Expect ((State $r) -eq 'L=absent O=original I=absent U=[]') (State $r)
  $b = Run $r rollback; Expect ($b.Code -eq 0) "code $($b.Code)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '08a primary rollback source missing -> secondary fallback (space ok)' {
  $r = NewRoot '08a'
  Expect ((Run $r install).Code -eq 0) 'install'; DelFile $r 'databigs\inxp_00.big.e1-original'
  Expect ((State $r) -eq 'L=candidate O=absent I=absent U=[]') (State $r)
  $b = Run $r rollback; Expect ($b.Code -eq 0 -and $b.Out -match 'from secondary source') "code $($b.Code): $($b.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '08b primary missing, insufficient space for the fallback copy -> refused, nothing mutated' {
  $r = NewRoot '08b'
  Expect ((Run $r install).Code -eq 0) 'install'; DelFile $r 'databigs\inxp_00.big.e1-original'
  $b = Run $r rollback '' '' '1000'; Expect ($b.Code -eq 2 -and $b.Out -match 'needs .* MB free') "code $($b.Code): $($b.Out)"
  Expect ((State $r) -eq 'L=candidate O=absent I=absent U=[]') (State $r)
  State $r
}
Case '08c primary and secondary missing -> blocked with exact explanation' {
  $r = NewRoot '08c' -NoSecondary
  Expect ((Run $r install).Code -eq 0) 'install'; DelFile $r 'databigs\inxp_00.big.e1-original'
  $b = Run $r rollback; Expect ($b.Code -eq 2 -and $b.Out -match 'secondary source is absent') "code $($b.Code): $($b.Out)"
  Expect ((State $r) -eq 'L=candidate O=absent I=absent U=[]') (State $r)
  State $r
}
Case '08d fallback interrupted after copy (live candidate, incoming original) then resumed' {
  $r = NewRoot '08d'
  Expect ((Run $r install).Code -eq 0) 'install'; DelFile $r 'databigs\inxp_00.big.e1-original'
  $a = Run $r rollback 'copy'; Expect ($a.Code -eq 3) "code $($a.Code)"
  Expect ((State $r) -eq 'L=candidate O=absent I=original U=[]') (State $r)
  # helper classifies this as S4 with a stale non-candidate incoming: it must not delete the original bytes there
  $b = Run $r rollback; Expect ($b.Code -eq 0) "code $($b.Code): $($b.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '08e fallback interrupted after rename1 (live absent, incoming original) then resumed' {
  $r = NewRoot '08e'
  Expect ((Run $r install).Code -eq 0) 'install'; DelFile $r 'databigs\inxp_00.big.e1-original'
  $a = Run $r rollback 'rename1'; Expect ($a.Code -eq 3) "code $($a.Code)"
  Expect ((State $r) -eq 'L=absent O=absent I=original U=[candidate]') (State $r)
  $b = Run $r rollback; Expect ($b.Code -eq 0 -and $b.Out -match 'fallback resumed') "code $($b.Code): $($b.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '09 insufficient space blocks install but rename rollback still works' {
  $r = NewRoot '09'
  $a = Run $r install '' '' '1000'; Expect ($a.Code -eq 2 -and $a.Out -match 'needs .* MB free') "install should refuse: $($a.Code)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') 'install mutated'
  Expect ((Run $r install).Code -eq 0) 'install with space'
  $b = Run $r rollback '' '' '0'; Expect ($b.Code -eq 0) "rollback with zero free: $($b.Code): $($b.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '10 nonempty overlay dir and loose dir block install, not rollback' {
  $r = NewRoot '10'
  Expect ((Run $r install).Code -eq 0) 'install'
  Set-Content -LiteralPath (Join-Path $r 'user\mods_folder\x.txt') 'x'; New-Item -ItemType Directory -Force (Join-Path $r 'nhl\data\common\gamemodes') | Out-Null
  $b = Run $r rollback; Expect ($b.Code -eq 0) "rollback code $($b.Code): $($b.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  $a = Run $r install; Expect ($a.Code -eq 2 -and $a.Out -match 'mods_folder empty\s+FAIL' -and $a.Out -match 'loose game-tree DB dir\s+FAIL') "install should refuse: $($a.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') 'install mutated'
  State $r
}
Case '11a unknown live bytes: both modes refuse, file untouched' {
  $r = NewRoot '11a'; Put $r 'databigs\inxp_00.big' $JUNK
  $a = Run $r install; Expect ($a.Code -eq 2) 'install'; $b = Run $r rollback; Expect ($b.Code -eq 2 -and $b.Out -match 'unknown bytes') 'rollback'
  Expect ((State $r) -eq 'L=junk O=absent I=absent U=[]') (State $r)
  State $r
}
Case '11b unknown incoming: install refuses and preserves; unknown used: rollback proceeds and preserves' {
  $r = NewRoot '11b'; Put $r 'databigs\inxp_00.big.e1-incoming' $JUNK
  $a = Run $r install; Expect ($a.Code -eq 2 -and $a.Out -match 'incoming sidecar holds unknown bytes') "install: $($a.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=junk U=[]') (State $r)
  DelFile $r 'databigs\inxp_00.big.e1-incoming'; Expect ((Run $r install).Code -eq 0) 'install'
  Put $r 'databigs\inxp_00.big.e1-candidate-used-20000101-000000-000' $JUNK
  $b = Run $r rollback; Expect ($b.Code -eq 0 -and $b.Out -match 'PRESERVED') "rollback: $($b.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[junk]') (State $r)
  State $r
}
Case '11c unknown original with candidate live -> rollback via secondary, original preserved' {
  $r = NewRoot '11c'
  Expect ((Run $r install).Code -eq 0) 'install'; Put $r 'databigs\inxp_00.big.e1-original' $JUNK
  $b = Run $r rollback; Expect ($b.Code -eq 0 -and $b.Out -match 'PRESERVED original sidecar' -and $b.Out -match 'from secondary source') "rollback: $($b.Out)"
  Expect ((State $r) -eq 'L=original O=junk I=absent U=[]') (State $r)
  State $r
}
Case '11d duplicate original: both modes refuse/no-op, nothing deleted' {
  $r = NewRoot '11d'; Put $r 'databigs\inxp_00.big.e1-original' $ORIG
  $a = Run $r install; Expect ($a.Code -eq 2 -and $a.Out -match 'ambiguous') 'install'
  $b = Run $r rollback; Expect ($b.Code -eq 0 -and $b.Out -match 'preserved') 'rollback'
  Expect ((State $r) -eq 'L=original O=original I=absent U=[]') (State $r)
  State $r
}
Case '11e stale known-candidate incoming on stock: rollback removes it, install reuses it' {
  $r = NewRoot '11e'; Put $r 'databigs\inxp_00.big.e1-incoming' $CAND
  $b = Run $r rollback; Expect ($b.Code -eq 0 -and $b.Out -match 'removed .*stale incoming') 'rollback'
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '12a unreadable precondition directory (real ACL denial, verified) fails install closed; rollback unaffected' {
  $r = NewRoot '12a'
  Expect ((Run $r install).Code -eq 0) 'install'
  $mods = Join-Path $r 'user\mods_folder'
  $ace = "$($env:USERNAME):(R)"
  $ic = & icacls $mods /deny $ace 2>&1 | Out-String
  Expect ($LASTEXITCODE -eq 0) "icacls setup failed ($LASTEXITCODE): $ic"
  $denied = $false
  try { $null = @(Get-ChildItem -LiteralPath $mods -Force -ErrorAction Stop) } catch { $denied = $true }
  try {
    if (-not $denied) { throw 'COVERAGE UNAVAILABLE: the deny ACE did not make enumeration fail in this session; real-ACL negative path not established (see 12b for the injected seam)' }
    $b = Run $r rollback; Expect ($b.Code -eq 0) "rollback: $($b.Out)"
    $a = Run $r install; Expect ($a.Code -eq 2 -and $a.Out -match 'mods_folder readable\s+FAIL') "install should fail closed: $($a.Out)"
    Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  } finally { & icacls $mods /remove:d $env:USERNAME | Out-Null }
  $again = $true; try { $null = @(Get-ChildItem -LiteralPath $mods -Force -ErrorAction Stop) } catch { $again = $false }
  Expect $again 'ACE cleanup failed: folder still unreadable'
  "real ACL denial verified before the helper ran; $(State $r)"
}
Case '12b unreadable precondition directory (INJECTED seam, fixture only) fails install closed' {
  $r = NewRoot '12b'
  $a = Run $r install '' '' '200000000' 'patches'
  Expect ($a.Code -eq 2 -and $a.Out -match 'patches readable\s+FAIL' -and $a.Out -match 'INJECTED') "install should fail closed: $($a.Out)"
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  Expect ((Run $r install).Code -eq 0) 'install without the seam'
  $b = Run $r rollback '' '' '0' 'patches'; Expect ($b.Code -eq 0) "rollback ignores the seam: $($b.Out)"
  "injected evidence only; $(State $r)"
}
Case '13 emulator running blocks both modes before any mutation' {
  $r = NewRoot '13'
  $a = Run $r install '' '1'; Expect ($a.Code -eq 2 -and $a.Out -match 'emulator') 'install'
  Expect ((Run $r install).Code -eq 0) 'install'
  $b = Run $r rollback '' '1'; Expect ($b.Code -eq 2 -and $b.Out -match 'emulator is running') 'rollback'
  Expect ((State $r) -eq 'L=candidate O=original I=absent U=[]') (State $r)
  State $r
}
Case '14 missing candidate artifact blocks install' {
  $r = NewRoot '14' -NoCandidate
  $a = Run $r install; Expect ($a.Code -eq 2 -and $a.Out -match 'candidate verified\s+FAIL') 'install'
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '15 guards: fixture root on H:/ShadPS4 refused; live profile refuses hooks' {
  $ErrorActionPreference = 'Continue'
  $x = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Script -Profile fixture -Root 'H:\Emulators\ShadPS4\Games\CUSA18182' -Mode status -ShaOrig a -ShaX b -Len 1 2>&1 | Out-String
  Expect ($LASTEXITCODE -ne 0 -and $x -match 'fixture root refused') "H root: $x"
  $y = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Script -Profile live -Mode status -FailAfter copy 2>&1 | Out-String
  Expect ($LASTEXITCODE -ne 0 -and $y -match 'does not accept') "live hooks: $y"
  'both refused'
}

Case '16a S2 (install interrupted after rename1) + changed config: install refuses, names rollback, files preserved; rollback restores' {
  $r = NewRoot '16a'
  Expect ((Run $r install 'rename1').Code -eq 3) 'inject'
  Set-Content -LiteralPath (Join-Path $r 'user\config.toml') "a = 2`nwindowPosX = 5`n"
  Agree $r install 'REFUSE' 'recovery action: -Mode rollback' | Out-Null
  $a = Run $r install; Expect ($a.Out -match 'config unchanged vs E0\s+FAIL') "gate not reported: $($a.Out)"
  Expect ((State $r) -eq 'L=absent O=original I=candidate U=[]') (State $r)
  Agree $r rollback 'MUTATE' | Out-Null
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  State $r
}
Case '16b S2 + nonempty overlay dir: install refuses (no copy/space/candidate needed to decide); after cleanup resumes without candidate artifact' {
  $r = NewRoot '16b'
  Expect ((Run $r install 'rename1').Code -eq 3) 'inject'
  Set-Content -LiteralPath (Join-Path $r 'user\mods_folder\m.txt') 'x'
  DelFile $r 'artifacts\candidate'
  $a = Run $r install '' '' '0'; Expect ($a.Code -eq 2 -and $a.Out -match 'mods_folder empty\s+FAIL' -and $a.Out -notmatch 'candidate verified') "should refuse on the overlay only: $($a.Out)"
  Expect ((State $r) -eq 'L=absent O=original I=candidate U=[]') 'files not preserved'
  Remove-Item -LiteralPath (Join-Path $r 'user\mods_folder\m.txt') -Force
  $b = Run $r install '' '' '0'; Expect ($b.Code -eq 0 -and $b.Out -match 'resumed') "resume with zero space and no artifact: $($b.Out)"
  Expect ((State $r) -eq 'L=candidate O=original I=absent U=[]') (State $r)
  State $r
}
Case '17a status/action agreement: stock + unknown incoming' {
  $r = NewRoot '17a'; Put $r 'databigs\inxp_00.big.e1-incoming' $JUNK
  Agree $r install 'REFUSE' 'unknown bytes' | Out-Null
  Agree $r rollback 'NOOP' 'unknown sidecars stay preserved' | Out-Null
  Expect ((State $r) -eq 'L=original O=absent I=junk U=[]') (State $r); State $r
}
Case '17b status/action agreement: duplicate original' {
  $r = NewRoot '17b'; Put $r 'databigs\inxp_00.big.e1-original' $ORIG
  Agree $r install 'REFUSE' 'ambiguous' | Out-Null
  Agree $r rollback 'NOOP' 'preserved' | Out-Null
  Expect ((State $r) -eq 'L=original O=original I=absent U=[]') (State $r); State $r
}
Case '17c status/action agreement: candidate live, primary and secondary missing' {
  $r = NewRoot '17c' -NoSecondary
  Expect ((Run $r install).Code -eq 0) 'install'; DelFile $r 'databigs\inxp_00.big.e1-original'
  Agree $r install 'NOOP' 'candidate already live' | Out-Null
  Agree $r rollback 'REFUSE' 'secondary source is absent' | Out-Null
  Expect ((State $r) -eq 'L=candidate O=absent I=absent U=[]') (State $r); State $r
}
Case '17d status/action agreement: interrupted install (S2), clean environment' {
  $r = NewRoot '17d'
  Expect ((Run $r install 'rename1').Code -eq 3) 'inject'
  $st = Run $r status; Expect ($st.Out -match 'install plan: MUTATE - resume interrupted install' -and $st.Out -match 'rollback plan: MUTATE - undo interrupted install') "status: $($st.Out)"
  Agree $r install 'MUTATE' 'resume' | Out-Null
  Expect ((State $r) -eq 'L=candidate O=original I=absent U=[]') (State $r); State $r
}
Case '17e status/action agreement: fallback needed (live absent, no primary) and fallback mid' {
  $r = NewRoot '17e'
  Expect ((Run $r install).Code -eq 0) 'install'; DelFile $r 'databigs\inxp_00.big.e1-original'; DelFile $r 'databigs\inxp_00.big'
  Agree $r rollback 'MUTATE' 'secondary source' | Out-Null
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r)
  Expect ((Run $r install).Code -eq 0) 'install'; DelFile $r 'databigs\inxp_00.big.e1-original'
  Expect ((Run $r rollback 'rename1').Code -eq 3) 'inject fallback mid'
  Agree $r rollback 'MUTATE' 'finish interrupted fallback' | Out-Null
  Expect ((State $r) -eq 'L=original O=absent I=absent U=[]') (State $r); State $r
}
Case '17f status/action agreement: already installed and stock no-ops do not mutate' {
  $r = NewRoot '17f'
  Agree $r rollback 'NOOP' 'already the original' | Out-Null
  Expect ((Run $r install).Code -eq 0) 'install'
  Agree $r install 'NOOP' 'already installed' | Out-Null
  Expect ((State $r) -eq 'L=candidate O=original I=absent U=[]') (State $r); State $r
}

Write-Host ''
$pass = @($results | Where-Object { $_[1] -eq 'PASS' }).Count; $fail = @($results | Where-Object { $_[1] -eq 'FAIL' }).Count
Write-Host "SUMMARY: $pass passed, $fail failed"
$results | ForEach-Object { "{0,-4} {1}  {2}" -f $_[1], $_[0], $_[2] } | Set-Content -LiteralPath (Join-Path $Here 'results.txt')
"SUMMARY: $pass passed, $fail failed" | Add-Content -LiteralPath (Join-Path $Here 'results.txt')
exit $(if ($fail -eq 0) { 0 } else { 1 })
