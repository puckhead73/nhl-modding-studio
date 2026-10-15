# NHL21-E1-PREP-R1 state-machine tests for deploy.ps1 on disposable fixtures (4 KB payloads).
# Every case gets its own root under tests\fx\; the helper is invoked as a child process so its
# `exit` codes are observed exactly as an operator would see them. Nothing here can reach H: or the
# real install: the helper refuses fixture roots on H:\ or containing "ShadPS4".
$ErrorActionPreference = 'Stop'
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$Script = Join-Path (Split-Path $Here -Parent) 'deploy.ps1'
$FX = Join-Path $Here 'probe-fx'
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
function Run($root, $mode, $failAfter = '', $emu = '', $free = '200000000') {
  $env:E1_TEST_EMU_RUNNING = $emu; $env:E1_TEST_FREE_BYTES = $free
  $psargs = @('-NoProfile','-ExecutionPolicy','Bypass','-File',$Script,'-Profile','fixture','-Root',$root,'-Mode',$mode,'-ShaOrig',$SHA_ORIG,'-ShaX',$SHA_X,'-Len','4096')
  if ($failAfter) { $psargs += @('-FailAfter',$failAfter) }
  $ErrorActionPreference = 'Continue'
  $out = & powershell.exe @psargs 2>&1 | Out-String
  $code = $LASTEXITCODE
  $ErrorActionPreference = 'Stop'
  $env:E1_TEST_EMU_RUNNING = ''; $env:E1_TEST_FREE_BYTES = ''
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

$r = NewRoot 'resume-gates'
$a = Run $r install 'rename1'
Expect ($a.Code -eq 3) 'fixture failed to interrupt'
Set-Content -LiteralPath (Join-Path $r 'user\mods_folder\active.txt') 'active overlay'
Set-Content -LiteralPath (Join-Path $r 'user\config.toml') 'changed = true'
$s = Run $r status
Write-Output ('STATUS FAILED GATES: ' + [bool]($s.Out -match 'install qualification: FAIL'))
$b = Run $r install
Write-Output ('RESUME EXIT: ' + $b.Code + '; ' + (State $r))
Write-Output $b.Out
$r = NewRoot 'status-unknown-incoming'
Put $r 'databigs\inxp_00.big.e1-incoming' $JUNK
$s = Run $r status
Write-Output ('UNKNOWN INCOMING STATUS: ' + $s.Out)
$a = Run $r install
Write-Output ('UNKNOWN INCOMING INSTALL EXIT: ' + $a.Code)