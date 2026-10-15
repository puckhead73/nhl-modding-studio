<#
.SYNOPSIS
    C0.2 — build a verified, byte-exact snapshot of the current working tree.

.DESCRIPTION
    Copies every tracked file plus the C0.1 section 5.1 allowlist of untracked files into a new
    directory outside the repository, hashing before and after, and re-verifying the live tree
    afterwards. It is read-only against the repository: no stash, checkout, clean, restore or reset,
    and no write of any kind inside the repo.

    It runs NO build or test check. The commands the baseline runs, the environment they must run
    under, and the results of the historical 2026-09-07 run are in
    docs/validation/c0-02-baseline-execution.md.

    Guarantees enforced (each is fatal, and none is left to a later manual step):
      * The repository and destination are canonicalised through reparse points before any output
        directory is created. The destination must not already exist, must not be the repository,
        must not be inside it, and must not contain it.
      * Every selected input, every directory between it and the repository root, and the
        repository's own parent chain are scanned for reparse points. Any hit fails before copying.
      * Enumeration is NUL-delimited, so paths containing spaces or non-ASCII characters survive
        intact. Paths Windows cannot represent are rejected explicitly before any copy.
      * Copying is a per-file byte copy. No archive format, no text/binary heuristic, no newline
        conversion in either direction.
      * Exclusions are applied to the whole member list, including nested build and dependency
        directories at any depth.
      * After copying, BOTH tracked and allowlisted-untracked membership are re-enumerated, live
        source bytes are re-hashed, the git-operation guards are repeated and HEAD is re-checked.
      * Any failure is fatal. Partial output is left in place and marked SNAPSHOT-INCOMPLETE.txt.
        SNAPSHOT-OK is printed only when every step above succeeded.

    Snapshot identity is the SHA-256 of the manifest together with the recorded HEAD. HEAD alone
    does not identify a snapshot, because the snapshot deliberately contains uncommitted bytes.

.PARAMETER RepoRoot
    The repository working tree to snapshot.

.PARAMETER Destination
    A NEW directory, outside the repository, to create. Receives source\ and evidence\.

.PARAMETER TestHookAfterCopy
    Verification-only. A PowerShell command string run after copying and before re-verification, so
    the focused helper tests can perturb the live tree deterministically instead of racing it.
    Never pass this in real use; it is recorded in the manifest whenever it is used.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File docs\validation\c0-02-snapshot.ps1 `
        -RepoRoot E:\Repositories\nhl-modding-studio -Destination H:\c0-baseline\run2
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$RepoRoot,
    [Parameter(Mandatory = $true)][string]$Destination,
    [string]$TestHookAfterCopy
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ---------------------------------------------------------------------------------------------
# Selection rules. These mirror C0.1 section 5.1 exactly and must not be broadened here.
# ---------------------------------------------------------------------------------------------

# Untracked paths that are part of the baseline. Anything untracked and not matching is excluded.
$script:UntrackedAllowlist = @(
    '^crates/tdb-headmesh/',
    '^crates/blender-job/',
    '^crates/studio-domain/src/legends\.rs$',
    '^crates/studio-domain/src/player_manifest\.rs$',
    '^crates/studio-domain/tests/overall_formula\.rs$',
    '^crates/studio-domain/tests/zz_diag\.rs$',
    '^crates/tdb-fhm-store/src/legends\.rs$',
    '^app/src/',
    '^defs/appearance/',
    '^docs/STATE-OF-THE-REPO\.md$',
    '^docs/audits/headmesh-uv-compat\.md$',
    '^docs/audits/legacy-head-donor-census\.md$',
    '^docs/audits/nhl21-ps4-head-asset-binding\.md$',
    '^docs/audits/nhl24-legacy-patha-roundtrip\.md$',
    '^docs/plans/completion-design-questions\.md$',
    '^docs/plans/project-completion-roadmap\.md$'
)

# Build, dependency and out-of-scope trees. Applied to the whole member list at any nesting depth.
# These only ever narrow the selection; they can never add a path.
$script:ExclusionRules = @(
    '(^|/)\.git(/|$)',
    '(^|/)target/',
    '(^|/)node_modules/',
    '(^|/)dist/',
    '(^|/)src-tauri/gen/',
    '(^|/)Staging/',
    '^corpus/',
    '^Nhl19BigStudio-Tester(/|\.zip$)'
)

# ---------------------------------------------------------------------------------------------
# Failure handling. Nothing below prints SNAPSHOT-OK; only the very end does.
# ---------------------------------------------------------------------------------------------

$script:DestCreated = $false
$script:DestFull = $null

function Fail {
    param([string]$Stage, [string]$Message)
    $text = "SNAPSHOT-FAILED [$Stage] $Message"
    if ($script:DestCreated -and $script:DestFull) {
        $marker = Join-Path $script:DestFull 'SNAPSHOT-INCOMPLETE.txt'
        $body = @(
            'This directory holds an INCOMPLETE snapshot and must not be used as a baseline.',
            "utc      : $([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))",
            "stage    : $Stage",
            "reason   : $Message",
            'Its contents are unverified. Delete it and re-run; do not patch it.'
        ) -join [Environment]::NewLine
        try { [IO.File]::WriteAllText($marker, $body, (New-Object Text.UTF8Encoding $false)) } catch { }
        $text += "  (partial output preserved and marked: $marker)"
    }
    Write-Output $text
    exit 1
}

# ---------------------------------------------------------------------------------------------
# Native canonical-path resolution. Resolves junctions, symlinks and mount points, so containment
# checks cannot be defeated by an alias pointing back into a protected location.
# ---------------------------------------------------------------------------------------------

if (-not ('C0Native.Path' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
namespace C0Native {
  public static class Path {
    const uint FILE_FLAG_BACKUP_SEMANTICS = 0x02000000;
    const uint OPEN_EXISTING = 3;
    const uint FILE_SHARE_ALL = 0x7;
    [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
    static extern IntPtr CreateFileW(string p, uint access, uint share, IntPtr sec,
                                     uint disp, uint flags, IntPtr tmpl);
    [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
    static extern uint GetFinalPathNameByHandleW(IntPtr h, StringBuilder buf, uint len, uint flags);
    [DllImport("kernel32.dll", SetLastError=true)]
    static extern bool CloseHandle(IntPtr h);
    /// Fully-resolved path, or null when it cannot be opened.
    public static string Final(string path) {
      IntPtr h = CreateFileW(path, 0, FILE_SHARE_ALL, IntPtr.Zero, OPEN_EXISTING,
                             FILE_FLAG_BACKUP_SEMANTICS, IntPtr.Zero);
      if (h == new IntPtr(-1)) return null;
      try {
        var sb = new StringBuilder(32768);
        uint n = GetFinalPathNameByHandleW(h, sb, (uint)sb.Capacity, 0);
        if (n == 0) return null;
        string s = sb.ToString();
        if (s.StartsWith(@"\\?\UNC\")) return @"\\" + s.Substring(8);
        if (s.StartsWith(@"\\?\"))     return s.Substring(4);
        return s;
      } finally { CloseHandle(h); }
    }
  }
}
'@
}

function Get-LongPath { param([string]$p) if ($p.Length -ge 240 -and $p -notlike '\\?\*') { "\\?\$p" } else { $p } }

function Get-Canonical {
    param([string]$Path)
    $full = [IO.Path]::GetFullPath($Path)
    $resolved = [C0Native.Path]::Final($full)
    if ($resolved) { return $resolved.TrimEnd('\') }
    return $full.TrimEnd('\')
}

function Test-Contains {
    # True when $Outer is $Inner or an ancestor of it. Segment-aware, so C:\a does not "contain" C:\ab.
    param([string]$Outer, [string]$Inner)
    $o = $Outer.TrimEnd('\'); $i = $Inner.TrimEnd('\')
    if ($o.Equals($i, 'OrdinalIgnoreCase')) { return $true }
    return $i.StartsWith($o + '\', 'OrdinalIgnoreCase')
}

function Get-Sha256 {
    param([string]$Path)
    $lp = Get-LongPath $Path
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $fs = New-Object IO.FileStream($lp, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read, 65536)
        try { return ([BitConverter]::ToString($sha.ComputeHash($fs))).Replace('-', '').ToLowerInvariant() }
        finally { $fs.Dispose() }
    } finally { $sha.Dispose() }
}

function Invoke-GitRaw {
    # Runs git and returns stdout decoded as UTF-8, so -z output keeps non-ASCII paths intact.
    param([string]$WorkDir, [string[]]$GitArgs)
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = 'git'
    $psi.WorkingDirectory = $WorkDir
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = New-Object Text.UTF8Encoding $false
    $psi.StandardErrorEncoding = New-Object Text.UTF8Encoding $false
    foreach ($a in $GitArgs) { $psi.Arguments += '"' + ($a -replace '"', '\"') + '" ' }
    $p = [Diagnostics.Process]::Start($psi)
    $out = $p.StandardOutput.ReadToEnd()
    $err = $p.StandardError.ReadToEnd()
    $p.WaitForExit()
    return [pscustomobject]@{ Out = $out; Err = $err; Code = $p.ExitCode }
}

function Get-GitList {
    param([string]$WorkDir, [string[]]$GitArgs, [string]$Stage)
    $r = Invoke-GitRaw -WorkDir $WorkDir -GitArgs $GitArgs
    if ($r.Code -ne 0) { Fail $Stage "git $($GitArgs -join ' ') exited $($r.Code): $($r.Err.Trim())" }
    return @($r.Out -split "`0" | Where-Object { $_ -ne '' })
}

function Get-Members {
    # Returns the ordered member list plus the audit sets, or fails.
    param([string]$Repo, [string]$Stage)
    $tracked = @(Get-GitList -WorkDir $Repo -GitArgs @('ls-files', '-z') -Stage $Stage)
    $untrackedAll = @(Get-GitList -WorkDir $Repo -GitArgs @('ls-files', '--others', '--exclude-standard', '-z') -Stage $Stage)
    if ($tracked.Count -eq 0) { Fail $Stage 'git reported no tracked files; refusing to snapshot an empty selection' }

    $allowRx = ($script:UntrackedAllowlist -join '|')
    $exclRx = ($script:ExclusionRules -join '|')

    $untrackedIn = @($untrackedAll | Where-Object { $_ -match $allowRx })
    $untrackedOut = @($untrackedAll | Where-Object { $_ -notmatch $allowRx })

    # Exclusions narrow the selection only. Removing a TRACKED path means the two rule sets
    # disagree; that is a human decision, not something to apply silently.
    $trackedDropped = @($tracked | Where-Object { $_ -match $exclRx })
    if ($trackedDropped.Count -gt 0) {
        Fail $Stage ("exclusion rules would drop $($trackedDropped.Count) TRACKED path(s), e.g. '" +
            $trackedDropped[0] + "'. Resolve the rule conflict; the helper will not decide it.")
    }
    $untrackedIn = @($untrackedIn | Where-Object { $_ -notmatch $exclRx })

    $members = @(($tracked + $untrackedIn) | Sort-Object -Unique -CaseSensitive)
    return [pscustomobject]@{
        Members       = $members
        Tracked       = @($tracked | Sort-Object -CaseSensitive)
        UntrackedAll  = @($untrackedAll | Sort-Object -CaseSensitive)
        UntrackedIn   = @($untrackedIn | Sort-Object -CaseSensitive)
        UntrackedOut  = @($untrackedOut | Sort-Object -CaseSensitive)
    }
}

function Assert-NoGitOperation {
    param([string]$Repo, [string]$Stage)
    foreach ($g in '.git\index.lock', '.git\rebase-merge', '.git\rebase-apply', '.git\MERGE_HEAD', '.git\CHERRY_PICK_HEAD', '.git\BISECT_LOG') {
        if (Test-Path -LiteralPath (Join-Path $Repo $g)) { Fail $Stage "git operation in progress: $g" }
    }
}

function Get-Head {
    param([string]$Repo, [string]$Stage)
    $h = Invoke-GitRaw -WorkDir $Repo -GitArgs @('rev-parse', 'HEAD')
    if ($h.Code -ne 0) { Fail $Stage "git rev-parse HEAD failed: $($h.Err.Trim())" }
    $b = Invoke-GitRaw -WorkDir $Repo -GitArgs @('rev-parse', '--abbrev-ref', 'HEAD')
    return [pscustomobject]@{ Sha = $h.Out.Trim(); Branch = $b.Out.Trim() }
}

# =============================================================================================
# 1. Validate the repository and the destination BEFORE creating any output.
# =============================================================================================

Write-Output "C0.2 snapshot helper — starting $([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))"

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) { Fail 'validate-repo' "repository root not found: $RepoRoot" }
$repoFull = Get-Canonical $RepoRoot

$top = Invoke-GitRaw -WorkDir $repoFull -GitArgs @('rev-parse', '--show-toplevel')
if ($top.Code -ne 0) { Fail 'validate-repo' "not a git working tree: $repoFull ($($top.Err.Trim()))" }
$topFull = Get-Canonical ($top.Out.Trim() -replace '/', '\')
if (-not $topFull.Equals($repoFull, 'OrdinalIgnoreCase')) {
    Fail 'validate-repo' "RepoRoot is not the working-tree root; git reports '$topFull'"
}

# The destination must be new. An existing target is rejected without being touched, so a stray
# path can never be partially overwritten or have its contents mistaken for a snapshot.
if (Test-Path -LiteralPath $Destination) { Fail 'validate-dest' "destination already exists; refusing to reuse or overwrite it: $Destination" }

$destFullRaw = [IO.Path]::GetFullPath($Destination).TrimEnd('\')
if ([string]::IsNullOrWhiteSpace($destFullRaw) -or $destFullRaw -match '^[A-Za-z]:$') {
    Fail 'validate-dest' "destination must be a directory path, not a volume root: $Destination"
}

# Canonicalise through the nearest EXISTING ancestor, so a junction anywhere above the destination
# is resolved before the containment checks run.
$probe = $destFullRaw
$tail = @()
while ($probe -and -not (Test-Path -LiteralPath $probe)) {
    $tail = , ([IO.Path]::GetFileName($probe)) + $tail
    $parent = [IO.Path]::GetDirectoryName($probe)
    if (-not $parent -or $parent -eq $probe) { break }
    $probe = $parent
}
if (-not $probe -or -not (Test-Path -LiteralPath $probe -PathType Container)) {
    Fail 'validate-dest' "no existing parent directory for the destination: $Destination"
}
$destFull = (@((Get-Canonical $probe)) + $tail) -join '\'
$destFull = $destFull -replace '\\+', '\'

if (Test-Contains -Outer $repoFull -Inner $destFull) { Fail 'validate-dest' "destination resolves inside the repository ($destFull); it must be outside" }
if (Test-Contains -Outer $destFull -Inner $repoFull) { Fail 'validate-dest' "destination resolves to an ancestor of the repository ($destFull)" }
if (Test-Path -LiteralPath $destFull) { Fail 'validate-dest' "destination resolves through a link to an existing path: $destFull" }

$srcRoot = Join-Path $destFull 'source'
$evRoot = Join-Path $destFull 'evidence'
$manRoot = Join-Path $evRoot 'manifests'

Write-Output "  repo        : $repoFull"
Write-Output "  destination : $destFull  (resolved; new)"

# =============================================================================================
# 2. Guards, HEAD, membership.
# =============================================================================================

Assert-NoGitOperation -Repo $repoFull -Stage 'guard-pre'
$headBefore = Get-Head -Repo $repoFull -Stage 'guard-pre'
Write-Output "  HEAD        : $($headBefore.Sha) ($($headBefore.Branch))"

$sel = Get-Members -Repo $repoFull -Stage 'enumerate'
$members = $sel.Members
Write-Output "  members     : $($members.Count)  (tracked $($sel.Tracked.Count) + untracked-allowlist $($sel.UntrackedIn.Count); $($sel.UntrackedOut.Count) untracked paths excluded)"

# ---- filename safety: reject anything Windows cannot represent, before any copy -----------------
$badNames = @()
foreach ($m in $members) {
    $reason = $null
    if ($m -match '[\x00-\x1f]') { $reason = 'control character' }
    elseif ($m -match '[:*?"<>|]') { $reason = 'character reserved on Windows' }
    elseif ($m -match '(^|/)\.\.?(/|$)') { $reason = 'relative path segment' }
    elseif ($m -match '(^|/)[^/]*[ .](/|$)') { $reason = 'segment ending in a space or dot' }
    elseif ($m -match '(^|/)(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(\.|/|$)') { $reason = 'reserved device name' }
    if ($reason) { $badNames += "$m  [$reason]" }
}
if ($badNames.Count -gt 0) {
    Fail 'filename-safety' ("$($badNames.Count) member path(s) cannot be represented faithfully on this filesystem; " +
        "refusing to copy rather than mangle them. First: $($badNames[0])")
}

# ---- existence and readability -----------------------------------------------------------------
$unreadable = @()
foreach ($m in $members) {
    $abs = Get-LongPath (Join-Path $repoFull ($m -replace '/', '\'))
    try {
        $fs = New-Object IO.FileStream($abs, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
        $fs.Dispose()
    } catch { $unreadable += "$m  [$($_.Exception.GetType().Name)]" }
}
if ($unreadable.Count -gt 0) {
    Fail 'readability' "$($unreadable.Count) member(s) missing or unreadable. First: $($unreadable[0])"
}

# ---- reparse scan: members, their ancestor directories, and the repo's own parent chain ---------
$dirsToCheck = New-Object 'System.Collections.Generic.HashSet[string]'
foreach ($m in $members) {
    $relDir = $m -replace '/', '\'
    $d = [IO.Path]::GetDirectoryName($relDir)
    while ($d) { [void]$dirsToCheck.Add($d); $d = [IO.Path]::GetDirectoryName($d) }
}
$reparse = @()
function Test-Reparse {
    param([string]$Abs, [string]$Label)
    try {
        $a = [IO.File]::GetAttributes((Get-LongPath $Abs))
        if (($a -band [IO.FileAttributes]::ReparsePoint) -ne 0) { return $Label }
    } catch { return "$Label  [unreadable attributes: $($_.Exception.Message)]" }
    return $null
}
foreach ($d in $dirsToCheck) { $r = Test-Reparse (Join-Path $repoFull $d) $d; if ($r) { $reparse += "DIR  $r" } }
foreach ($m in $members) { $r = Test-Reparse (Join-Path $repoFull ($m -replace '/', '\')) $m; if ($r) { $reparse += "FILE $r" } }
$anc = [IO.DirectoryInfo]$repoFull
while ($anc) { $r = Test-Reparse $anc.FullName $anc.FullName; if ($r) { $reparse += "ANCESTOR $r" }; $anc = $anc.Parent }
if ($reparse.Count -gt 0) {
    Fail 'reparse-scan' ("$($reparse.Count) reparse point / unreadable path in the selection or its parent chain; " +
        "resolve each explicitly before snapshotting. First: $($reparse[0])")
}
Write-Output "  reparse     : none in $($members.Count) files, $($dirsToCheck.Count) directories and the repository parent chain"

# ---- pre-hash -----------------------------------------------------------------------------------
$pre = [ordered]@{}
$sizes = [ordered]@{}
foreach ($m in $members) {
    $abs = Join-Path $repoFull ($m -replace '/', '\')
    try {
        $pre[$m] = Get-Sha256 $abs
        $sizes[$m] = (New-Object IO.FileInfo((Get-LongPath $abs))).Length
    } catch { Fail 'pre-hash' "cannot hash '$m': $($_.Exception.Message)" }
}
Write-Output "  pre-hash    : $($pre.Count) files"

# =============================================================================================
# 3. Create output and copy raw bytes.
# =============================================================================================

try {
    [void][IO.Directory]::CreateDirectory($destFull)
    $script:DestFull = $destFull
    $script:DestCreated = $true
    [void][IO.Directory]::CreateDirectory($srcRoot)
    [void][IO.Directory]::CreateDirectory($manRoot)
    [void][IO.Directory]::CreateDirectory((Join-Path $evRoot 'logs'))
} catch { Fail 'create-dest' $_.Exception.Message }

foreach ($m in $members) {
    $src = Get-LongPath (Join-Path $repoFull ($m -replace '/', '\'))
    $dst = Join-Path $srcRoot ($m -replace '/', '\')
    try {
        [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($dst))
        [IO.File]::Copy($src, (Get-LongPath $dst), $false)
    } catch { Fail 'copy' "copying '$m': $($_.Exception.Message)" }
}
Write-Output "  copy        : $($members.Count) files (raw bytes; no newline conversion, no links)"

if ($PSBoundParameters.ContainsKey('TestHookAfterCopy') -and $TestHookAfterCopy) {
    Write-Output "  TEST HOOK   : $TestHookAfterCopy"
    try { Invoke-Expression $TestHookAfterCopy } catch { Fail 'test-hook' $_.Exception.Message }
}

# =============================================================================================
# 4. Verify the copies, then re-verify the live tree.
# =============================================================================================

$copy = [ordered]@{}
$mismatch = @()
foreach ($m in $members) {
    $dst = Join-Path $srcRoot ($m -replace '/', '\')
    try { $copy[$m] = Get-Sha256 $dst } catch { Fail 'copy-verify' "cannot hash the copy of '$m': $($_.Exception.Message)" }
    if ($copy[$m] -ne $pre[$m]) { $mismatch += $m }
}
if ($mismatch.Count -gt 0) { Fail 'copy-verify' "$($mismatch.Count) copied file(s) differ from source. First: $($mismatch[0])" }
Write-Output "  copy-verify : $($copy.Count)/$($members.Count) identical"

Assert-NoGitOperation -Repo $repoFull -Stage 'guard-post'
$headAfter = Get-Head -Repo $repoFull -Stage 'guard-post'
if ($headAfter.Sha -ne $headBefore.Sha) { Fail 'recheck-head' "HEAD moved during the snapshot: $($headBefore.Sha) -> $($headAfter.Sha)" }

# Re-enumerate BOTH sets, so an added or removed allowlisted untracked file is caught too.
$sel2 = Get-Members -Repo $repoFull -Stage 'recheck-membership'
$dTracked = @(Compare-Object $sel.Tracked $sel2.Tracked -CaseSensitive)
if ($dTracked.Count -gt 0) { Fail 'recheck-membership' "tracked membership changed during the snapshot ($($dTracked.Count) difference(s)); first: $($dTracked[0].InputObject)" }
$dUntracked = @(Compare-Object $sel.UntrackedIn $sel2.UntrackedIn -CaseSensitive)
if ($dUntracked.Count -gt 0) { Fail 'recheck-membership' "allowlisted untracked membership changed during the snapshot ($($dUntracked.Count) difference(s)); first: $($dUntracked[0].InputObject)" }

$drift = @()
foreach ($m in $members) {
    $abs = Join-Path $repoFull ($m -replace '/', '\')
    try { $now = Get-Sha256 $abs } catch { Fail 'recheck-content' "cannot re-hash '$m': $($_.Exception.Message)" }
    if ($now -ne $pre[$m]) { $drift += $m }
}
if ($drift.Count -gt 0) {
    Fail 'recheck-content' ("$($drift.Count) live source file(s) changed under the copy — torn snapshot. " +
        "Discard and retry; do not patch. First: $($drift[0])")
}
Write-Output "  live-recheck: membership and bytes stable; HEAD unchanged"

# =============================================================================================
# 5. Manifest. Written outside the snapshot source so it cannot alter what a later run hashes.
# =============================================================================================

$scriptPath = $MyInvocation.MyCommand.Path
$gitVer = (Invoke-GitRaw -WorkDir $repoFull -GitArgs @('--version')).Out.Trim()
$autocrlf = (Invoke-GitRaw -WorkDir $repoFull -GitArgs @('config', '--get', 'core.autocrlf')).Out.Trim()

$sb = New-Object Text.StringBuilder
function W { param([string]$s = '') [void]$sb.AppendLine($s) }

W '# C0.2 verified snapshot manifest'
W '# Produced by docs/validation/c0-02-snapshot.ps1. Snapshot identity = SHA-256 of this file'
W '# together with git_head below; HEAD alone does not identify it, because the snapshot'
W '# deliberately contains uncommitted bytes.'
W ''
W '## PROVENANCE'
W "generated_utc      = $([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))"
W "helper             = $scriptPath"
W "helper_sha256      = $(if ($scriptPath -and (Test-Path -LiteralPath $scriptPath)) { Get-Sha256 $scriptPath } else { '(unknown)' })"
W "invocation         = -RepoRoot '$RepoRoot' -Destination '$Destination'$(if ($TestHookAfterCopy) { " -TestHookAfterCopy '$TestHookAfterCopy'" })"
W "test_hook_used     = $(if ($TestHookAfterCopy) { 'YES — this is a verification run, not a baseline snapshot' } else { 'no' })"
W "host               = $env:COMPUTERNAME / $env:USERNAME"
W "powershell         = $($PSVersionTable.PSVersion)"
W "git                = $gitVer"
W "os                 = $([Environment]::OSVersion.VersionString)"
W ''
W '## IDENTITY'
W "repo_root          = $repoFull"
W "snapshot_source    = $srcRoot"
W "evidence_dir       = $evRoot"
W "git_head           = $($headBefore.Sha)"
W "git_branch         = $($headBefore.Branch)"
W "core_autocrlf      = $autocrlf"
W "copy_method        = per-file [IO.File]::Copy of raw bytes; no archive, no newline conversion, no links"
W ''
W '## COUNTS'
W "member_count       = $($members.Count)"
W "tracked_count      = $($sel.Tracked.Count)"
W "untracked_included = $($sel.UntrackedIn.Count)"
W "untracked_excluded = $($sel.UntrackedOut.Count)"
W "total_bytes        = $(($sizes.Values | Measure-Object -Sum).Sum)"
W ''
W '## VERIFICATION RESULTS'
W 'reparse_scan       = clean (all members, all ancestor directories inside the repo, and the repository parent chain)'
W 'filename_safety    = clean (no control/reserved characters, relative segments, trailing space-or-dot, or device names)'
W 'readability        = all members opened for read before hashing'
W "copy_verify        = $($copy.Count)/$($members.Count) snapshot sha256 == source sha256"
W 'membership_recheck = tracked AND allowlisted-untracked sets both re-enumerated after copying; unchanged'
W 'content_recheck    = all member source hashes re-computed after copying; unchanged'
W "head_recheck       = $($headAfter.Sha) (unchanged)"
W 'git_guards         = no index.lock / rebase / merge / cherry-pick / bisect, before and after'
W ''
W '## SELECTION RULES — untracked allowlist (regex, applied to forward-slash repo-relative paths)'
foreach ($r in $script:UntrackedAllowlist) { W "  $r" }
W ''
W '## SELECTION RULES — exclusions (narrow only; applied to the whole member list at any depth)'
foreach ($r in $script:ExclusionRules) { W "  $r" }
W '  Any untracked path not matching the allowlist above.'
W '  Dropping a TRACKED path via an exclusion is a fatal rule conflict, never applied silently.'
W ''
W "## EXCLUDED UNTRACKED PATHS ($($sel.UntrackedOut.Count))"
foreach ($p in $sel.UntrackedOut) { W $p }
W ''
W '## PER-FILE  size<TAB>source_sha256<TAB>snapshot_sha256<TAB>origin<TAB>path'
$trackedSet = New-Object 'System.Collections.Generic.HashSet[string]' (, [string[]]$sel.Tracked)
foreach ($m in $members) {
    $origin = if ($trackedSet.Contains($m)) { 'tracked' } else { 'untracked-allowlist' }
    W ("{0}`t{1}`t{2}`t{3}`t{4}" -f $sizes[$m], $pre[$m], $copy[$m], $origin, $m)
}

$manPath = Join-Path $manRoot 'snapshot-manifest.txt'
try { [IO.File]::WriteAllText($manPath, $sb.ToString(), (New-Object Text.UTF8Encoding $false)) }
catch { Fail 'manifest' "writing the manifest: $($_.Exception.Message)" }
try { $manHash = Get-Sha256 $manPath } catch { Fail 'manifest' "hashing the manifest: $($_.Exception.Message)" }
try { [IO.File]::WriteAllText((Join-Path $manRoot 'snapshot-manifest.sha256'), "$manHash  snapshot-manifest.txt`n", (New-Object Text.UTF8Encoding $false)) }
catch { Fail 'manifest' "writing the manifest hash sidecar: $($_.Exception.Message)" }

Write-Output "  manifest    : $manPath"
Write-Output "MANIFEST_SHA256=$manHash"
Write-Output "GIT_HEAD=$($headBefore.Sha)"
Write-Output "SNAPSHOT-OK  $srcRoot"
exit 0
