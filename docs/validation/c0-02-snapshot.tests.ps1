<#
.SYNOPSIS
    Focused verification for docs/validation/c0-02-snapshot.ps1.

.DESCRIPTION
    Builds small synthetic git repositories in a temporary directory and asserts that the helper
    enforces the guarantees it claims. It never touches the NHL Modding Studio repository, never
    creates a full application snapshot, and never reruns any baseline check.

    Each case is deterministic. Where the helper must observe a change that happens *during* a
    snapshot, the test drives it through the helper's -TestHookAfterCopy parameter rather than
    racing the copy.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File docs\validation\c0-02-snapshot.tests.ps1
#>
[CmdletBinding()]
param([string]$Helper, [string]$WorkRoot)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Continue'

if (-not $Helper) { $Helper = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) 'c0-02-snapshot.ps1' }
if (-not (Test-Path -LiteralPath $Helper)) { throw "helper not found: $Helper" }
if (-not $WorkRoot) { $WorkRoot = Join-Path $env:TEMP ("c0-02-helper-tests-" + [Guid]::NewGuid().ToString('N').Substring(0, 8)) }
[void][IO.Directory]::CreateDirectory($WorkRoot)

$script:Pass = 0; $script:Fail = 0; $script:Skip = 0; $script:Lines = @()

function Note { param([string]$s) Write-Output "      $s"; $script:Lines += "      $s" }
function Ok { param([string]$n, [string]$d) $script:Pass++; Write-Output "PASS  $n — $d"; $script:Lines += "PASS  $n — $d" }
function No { param([string]$n, [string]$d) $script:Fail++; Write-Output "FAIL  $n — $d"; $script:Lines += "FAIL  $n — $d" }
function Sk { param([string]$n, [string]$d) $script:Skip++; Write-Output "SKIP  $n — $d"; $script:Lines += "SKIP  $n — $d" }

function Invoke-Helper {
    param([string]$Repo, [string]$Dest, [string]$Hook)
    $a = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $Helper, '-RepoRoot', $Repo, '-Destination', $Dest)
    if ($Hook) { $a += @('-TestHookAfterCopy', $Hook) }
    $out = & powershell.exe @a 2>&1 | Out-String
    return [pscustomobject]@{ Code = $LASTEXITCODE; Out = $out }
}

function New-SynthRepo {
    param([string]$Name)
    $r = Join-Path $WorkRoot $Name
    [void][IO.Directory]::CreateDirectory($r)
    & git -C $r init -q 2>&1 | Out-Null
    & git -C $r config user.email 'c0-tests@example.invalid' | Out-Null
    & git -C $r config user.name 'C0 Tests' | Out-Null
    return $r
}

function Add-File {
    param([string]$Repo, [string]$Rel, [byte[]]$Bytes)
    $p = Join-Path $Repo ($Rel -replace '/', '\')
    [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($p))
    [IO.File]::WriteAllBytes($p, $Bytes)
    return $p
}
function B { param([string]$s) [Text.Encoding]::UTF8.GetBytes($s) }

function Get-Sha { param([string]$p)
    $sha = [Security.Cryptography.SHA256]::Create()
    $fs = [IO.File]::OpenRead($p)
    try { ([BitConverter]::ToString($sha.ComputeHash($fs))).Replace('-', '').ToLowerInvariant() } finally { $fs.Dispose(); $sha.Dispose() }
}

Write-Output "C0.2 helper verification"
Write-Output "  helper    : $Helper"
Write-Output "  work root : $WorkRoot"
Write-Output ''

# =============================================================================================
# T1 — successful copy: spaces, non-ASCII, mixed line endings, allowlist and exclusions.
# =============================================================================================
$t = 'T1 happy-path'
try {
    $repo = New-SynthRepo 'repo-happy'
    # Tracked, awkward names, and deliberately mixed line endings.
    Add-File $repo 'src/crlf file.txt'        ([byte[]](B "alpha`r`nbeta`r`n"))            | Out-Null
    Add-File $repo 'src/lf file.txt'          ([byte[]](B "alpha`nbeta`n"))                | Out-Null
    Add-File $repo 'src/pägès/Ünï cöde.tsx'   ([byte[]](B "export const x = 'ü';`r`n"))    | Out-Null
    Add-File $repo 'src/bin.dat'              ([byte[]](0..255))                            | Out-Null
    & git -C $repo add -A 2>&1 | Out-Null
    & git -C $repo commit -qm init 2>&1 | Out-Null
    # Untracked: two on the allowlist, several that must be excluded.
    Add-File $repo 'app/src/Legends Page.tsx' ([byte[]](B "// allowlisted`r`n"))            | Out-Null
    Add-File $repo 'defs/appearance/head-ids.toml' ([byte[]](B "[a]`nb = 1`n"))             | Out-Null
    Add-File $repo 'scratch/notes.md'         ([byte[]](B 'off-allowlist'))                 | Out-Null
    Add-File $repo 'corpus/historical.db'     ([byte[]](B 'private'))                       | Out-Null
    Add-File $repo 'app/node_modules/pkg/index.js' ([byte[]](B 'dep'))                      | Out-Null
    Add-File $repo 'app/src/nested/node_modules/deep/x.js' ([byte[]](B 'nested dep'))       | Out-Null
    Add-File $repo 'crates/tdb-headmesh/target/debug/build.log' ([byte[]](B 'build out'))   | Out-Null

    $dest = Join-Path $WorkRoot 'dest-happy'
    $r = Invoke-Helper -Repo $repo -Dest $dest
    $src = Join-Path $dest 'source'

    if ($r.Code -ne 0 -or $r.Out -notmatch 'SNAPSHOT-OK') { No $t "helper failed: $($r.Out.Trim())" }
    else {
        $bad = @()
        foreach ($rel in 'src/crlf file.txt', 'src/lf file.txt', 'src/pägès/Ünï cöde.tsx', 'src/bin.dat',
            'app/src/Legends Page.tsx', 'defs/appearance/head-ids.toml') {
            $a = Join-Path $repo ($rel -replace '/', '\'); $b = Join-Path $src ($rel -replace '/', '\')
            if (-not (Test-Path -LiteralPath $b)) { $bad += "missing in snapshot: $rel" }
            elseif ((Get-Sha $a) -ne (Get-Sha $b)) { $bad += "byte mismatch: $rel" }
        }
        foreach ($rel in 'scratch/notes.md', 'corpus/historical.db', 'app/node_modules/pkg/index.js',
            'app/src/nested/node_modules/deep/x.js', 'crates/tdb-headmesh/target/debug/build.log') {
            if (Test-Path -LiteralPath (Join-Path $src ($rel -replace '/', '\'))) { $bad += "should have been excluded: $rel" }
        }
        # Line endings must survive verbatim in both directions.
        $crlf = [IO.File]::ReadAllBytes((Join-Path $src 'src\crlf file.txt'))
        $lf = [IO.File]::ReadAllBytes((Join-Path $src 'src\lf file.txt'))
        if (($crlf -join ',') -notmatch '13,10') { $bad += 'CRLF was converted' }
        if (($lf -join ',') -match '13,10') { $bad += 'LF was converted to CRLF' }
        $man = Join-Path $dest 'evidence\manifests\snapshot-manifest.txt'
        if (-not (Test-Path -LiteralPath $man)) { $bad += 'no manifest' }
        elseif (-not (Test-Path -LiteralPath (Join-Path $dest 'evidence\manifests\snapshot-manifest.sha256'))) { $bad += 'no manifest hash sidecar' }
        if (Test-Path -LiteralPath (Join-Path $src 'evidence')) { $bad += 'evidence written inside snapshot source' }

        if ($bad.Count -eq 0) {
            Ok $t 'spaces, non-ASCII, binary and mixed line endings copied byte-exact; allowlist honoured; nested node_modules/target/corpus excluded; manifest + hash written outside source'
        } else { No $t ($bad -join '; ') }
    }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

# =============================================================================================
# T2 — existing destination is rejected without altering it.
# =============================================================================================
$t = 'T2 existing-destination'
try {
    $repo = Join-Path $WorkRoot 'repo-happy'
    $dest = Join-Path $WorkRoot 'dest-existing'
    [void][IO.Directory]::CreateDirectory($dest)
    $sentinel = Join-Path $dest 'sentinel.txt'
    [IO.File]::WriteAllText($sentinel, "do not touch`r`n")
    $before = Get-Sha $sentinel
    $countBefore = @(Get-ChildItem -LiteralPath $dest -Recurse -Force).Count

    $r = Invoke-Helper -Repo $repo -Dest $dest
    $after = Get-Sha $sentinel
    $countAfter = @(Get-ChildItem -LiteralPath $dest -Recurse -Force).Count

    if ($r.Code -eq 0) { No $t 'helper succeeded against an existing destination' }
    elseif ($r.Out -match 'SNAPSHOT-OK') { No $t 'printed SNAPSHOT-OK on a rejected destination' }
    elseif ($before -ne $after) { No $t 'sentinel content was modified' }
    elseif ($countBefore -ne $countAfter) { No $t "destination gained $($countAfter - $countBefore) entries" }
    else { Ok $t 'rejected, exit 1, sentinel byte-identical, no entries added' }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

# =============================================================================================
# T3 — destination inside the repository is rejected before any output is created.
# =============================================================================================
$t = 'T3 destination-inside-source'
try {
    $repo = Join-Path $WorkRoot 'repo-happy'
    $dest = Join-Path $repo 'snapshot-here'
    $before = (Get-ChildItem -LiteralPath $repo -Recurse -Force | Measure-Object).Count
    $r = Invoke-Helper -Repo $repo -Dest $dest
    $after = (Get-ChildItem -LiteralPath $repo -Recurse -Force | Measure-Object).Count

    if ($r.Code -eq 0) { No $t 'helper accepted a destination inside the repository' }
    elseif (Test-Path -LiteralPath $dest) { No $t 'output directory was created before rejection' }
    elseif ($before -ne $after) { No $t "repository gained $($after - $before) entries" }
    elseif ($r.Out -notmatch 'resolves inside the repository') { No $t "wrong rejection reason: $($r.Out.Trim())" }
    else { Ok $t 'rejected before creating output; repository entry count unchanged' }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

# =============================================================================================
# T3b — a destination that reaches the repository through a junction is rejected.
# =============================================================================================
$t = 'T3b destination-alias-via-junction'
try {
    $repo = Join-Path $WorkRoot 'repo-happy'
    $alias = Join-Path $WorkRoot 'alias-to-repo'
    $mk = & cmd.exe /c "mklink /J `"$alias`" `"$repo`"" 2>&1 | Out-String
    if (-not (Test-Path -LiteralPath $alias)) {
        Sk $t "cannot create a junction in this environment: $($mk.Trim())"
    } else {
        $dest = Join-Path $alias 'snap-via-alias'
        $r = Invoke-Helper -Repo $repo -Dest $dest
        if ($r.Code -eq 0) { No $t 'helper accepted a destination aliased into the repository' }
        elseif (Test-Path -LiteralPath $dest) { No $t 'output created before rejection' }
        elseif ($r.Out -notmatch 'resolves inside the repository') { No $t "wrong reason: $($r.Out.Trim())" }
        else { Ok $t 'junction alias resolved to the repository and rejected before output' }
    }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

# =============================================================================================
# T4 — a reparse point among the selected inputs is fatal before copying.
# =============================================================================================
$t = 'T4 reparse-in-selection'
try {
    $repo = New-SynthRepo 'repo-reparse'
    Add-File $repo 'src/a.txt' ([byte[]](B 'a')) | Out-Null
    & git -C $repo add -A 2>&1 | Out-Null
    & git -C $repo commit -qm init 2>&1 | Out-Null
    $outside = Join-Path $WorkRoot 'outside-tree'
    [void][IO.Directory]::CreateDirectory($outside)
    [IO.File]::WriteAllText((Join-Path $outside 'linked.tsx'), 'linked')
    $link = Join-Path $repo 'app\src\linkdir'
    [void][IO.Directory]::CreateDirectory((Join-Path $repo 'app\src'))
    $mk = & cmd.exe /c "mklink /J `"$link`" `"$outside`"" 2>&1 | Out-String

    if (-not (Test-Path -LiteralPath $link)) {
        Sk $t "cannot create a junction in this environment: $($mk.Trim())"
    } else {
        $dest = Join-Path $WorkRoot 'dest-reparse'
        $r = Invoke-Helper -Repo $repo -Dest $dest
        if ($r.Code -eq 0) { No $t 'helper snapshotted a selection containing a reparse point' }
        elseif ($r.Out -match 'SNAPSHOT-OK') { No $t 'printed SNAPSHOT-OK despite a reparse point' }
        elseif (Test-Path -LiteralPath $dest) { No $t 'output created before the reparse scan rejected the run' }
        elseif ($r.Out -notmatch 'reparse') { No $t "wrong rejection reason: $($r.Out.Trim())" }
        else { Ok $t 'junction inside an allowlisted directory detected and rejected before copying' }
    }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

# =============================================================================================
# T5 — a missing/unreadable member fails, with no success marker.
# =============================================================================================
$t = 'T5 missing-input'
try {
    $repo = New-SynthRepo 'repo-missing'
    Add-File $repo 'src/kept.txt' ([byte[]](B 'kept')) | Out-Null
    Add-File $repo 'src/gone.txt' ([byte[]](B 'gone')) | Out-Null
    & git -C $repo add -A 2>&1 | Out-Null
    & git -C $repo commit -qm init 2>&1 | Out-Null
    [IO.File]::Delete((Join-Path $repo 'src\gone.txt'))   # tracked, now absent from the working tree

    $dest = Join-Path $WorkRoot 'dest-missing'
    $r = Invoke-Helper -Repo $repo -Dest $dest
    if ($r.Code -eq 0) { No $t 'helper succeeded with a missing tracked member' }
    elseif ($r.Out -match 'SNAPSHOT-OK') { No $t 'printed SNAPSHOT-OK with a missing member' }
    elseif (Test-Path -LiteralPath $dest) { No $t 'output created despite an unreadable member' }
    elseif ($r.Out -notmatch 'missing or unreadable') { No $t "wrong rejection reason: $($r.Out.Trim())" }
    else { Ok $t 'missing tracked member rejected before output; no success marker' }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

# =============================================================================================
# T5b — an unreadable (exclusively locked) member fails deterministically.
# =============================================================================================
$t = 'T5b locked-input'
try {
    $repo = New-SynthRepo 'repo-locked'
    Add-File $repo 'src/locked.bin' ([byte[]](B 'locked')) | Out-Null
    & git -C $repo add -A 2>&1 | Out-Null
    & git -C $repo commit -qm init 2>&1 | Out-Null
    $lockPath = Join-Path $repo 'src\locked.bin'
    $fs = New-Object IO.FileStream($lockPath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::None)
    try {
        $dest = Join-Path $WorkRoot 'dest-locked'
        $r = Invoke-Helper -Repo $repo -Dest $dest
        if ($r.Code -eq 0) { No $t 'helper succeeded with an exclusively locked member' }
        elseif (Test-Path -LiteralPath $dest) { No $t 'output created despite an unreadable member' }
        elseif ($r.Out -notmatch 'missing or unreadable') { No $t "wrong rejection reason: $($r.Out.Trim())" }
        else { Ok $t 'exclusively locked member rejected before output' }
    } finally { $fs.Dispose() }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

# =============================================================================================
# T6 — membership added during the snapshot is detected after copying.
# =============================================================================================
$t = 'T6 membership-drift'
try {
    $repo = New-SynthRepo 'repo-drift-membership'
    Add-File $repo 'src/a.txt' ([byte[]](B 'a')) | Out-Null
    & git -C $repo add -A 2>&1 | Out-Null
    & git -C $repo commit -qm init 2>&1 | Out-Null
    Add-File $repo 'app/src/one.tsx' ([byte[]](B 'one')) | Out-Null

    $dest = Join-Path $WorkRoot 'dest-drift-membership'
    $newFile = (Join-Path $repo 'app\src\two.tsx') -replace "'", "''"
    $hook = "[IO.File]::WriteAllText('$newFile','two')"
    $r = Invoke-Helper -Repo $repo -Dest $dest -Hook $hook

    if ($r.Code -eq 0) { No $t 'helper reported success after allowlisted untracked membership changed' }
    elseif ($r.Out -match 'SNAPSHOT-OK') { No $t 'printed SNAPSHOT-OK after membership drift' }
    elseif ($r.Out -notmatch 'allowlisted untracked membership changed') { No $t "wrong reason: $($r.Out.Trim())" }
    elseif (-not (Test-Path -LiteralPath (Join-Path $dest 'SNAPSHOT-INCOMPLETE.txt'))) { No $t 'partial output not marked incomplete' }
    else { Ok $t 'new allowlisted untracked file detected by the post-copy re-enumeration; partial output marked SNAPSHOT-INCOMPLETE' }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

# =============================================================================================
# T7 — input bytes changed during the snapshot are detected (torn copy).
# =============================================================================================
$t = 'T7 content-drift'
try {
    $repo = New-SynthRepo 'repo-drift-content'
    Add-File $repo 'src/a.txt' ([byte[]](B "original`r`n")) | Out-Null
    & git -C $repo add -A 2>&1 | Out-Null
    & git -C $repo commit -qm init 2>&1 | Out-Null

    $dest = Join-Path $WorkRoot 'dest-drift-content'
    $target = (Join-Path $repo 'src\a.txt') -replace "'", "''"
    $hook = "[IO.File]::WriteAllText('$target','MUTATED')"
    $r = Invoke-Helper -Repo $repo -Dest $dest -Hook $hook

    if ($r.Code -eq 0) { No $t 'helper reported success after a source file changed under the copy' }
    elseif ($r.Out -match 'SNAPSHOT-OK') { No $t 'printed SNAPSHOT-OK after a torn copy' }
    elseif ($r.Out -notmatch 'torn snapshot') { No $t "wrong reason: $($r.Out.Trim())" }
    elseif (-not (Test-Path -LiteralPath (Join-Path $dest 'SNAPSHOT-INCOMPLETE.txt'))) { No $t 'partial output not marked incomplete' }
    else { Ok $t 'changed source bytes detected by the post-copy re-hash; partial output marked SNAPSHOT-INCOMPLETE' }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

# =============================================================================================
# T8 — a non-repository source is rejected.
# =============================================================================================
$t = 'T8 not-a-repository'
try {
    $plain = Join-Path $WorkRoot 'plain-dir'
    [void][IO.Directory]::CreateDirectory($plain)
    [IO.File]::WriteAllText((Join-Path $plain 'x.txt'), 'x')
    $dest = Join-Path $WorkRoot 'dest-plain'
    $r = Invoke-Helper -Repo $plain -Dest $dest
    if ($r.Code -eq 0) { No $t 'helper accepted a directory that is not a git working tree' }
    elseif (Test-Path -LiteralPath $dest) { No $t 'output created before rejection' }
    else { Ok $t 'non-repository source rejected before output' }
} catch { No $t "unexpected error: $($_.Exception.Message)" }

Write-Output ''
Write-Output "TOTAL passed=$script:Pass failed=$script:Fail skipped=$script:Skip"
Write-Output "work root preserved for inspection: $WorkRoot"
if ($script:Fail -gt 0) { exit 1 } else { exit 0 }
