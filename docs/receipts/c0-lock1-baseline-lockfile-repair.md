# C0-LOCK1 — commit the verified baseline lockfile repair

**Package:** C0-LOCK1 · **Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling`

| | |
|---|---|
| **Commit created** | `0a791892dcfef11d079e873c2444d5e70bf29262` |
| **Parent** | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` |
| **Subject** | `fix(build): record studio-domain TOML dependency in lockfile` |
| **Files changed** | `Cargo.lock` — **1 file, 1 insertion, 0 deletions** |
| **Pushed?** | **No.** Push is not authorized and was not attempted; the branch has no upstream configured |
| **Index after** | empty |

> **This receipt is deliberately left uncommitted.** The authorized commit contains `Cargo.lock`
> and nothing else. No canonical crate, no workspace member, no feature work, no generator, no
> live-tree build or test, no game-data access, no blanket staging, no push.

---

## 1. Result in one paragraph

The one-line baseline repair isolated by [C0-SLICE2](c0-slice2-canonical-probe.md) §4.2 is
committed as `0a79189`. It adds `"toml 0.8.2"` to `studio-domain`'s dependency list in `Cargo.lock`,
recording an edge that `crates/studio-domain/Cargo.toml:25` already declares. **It was never staged
from the dirty working-tree `Cargo.lock`**, which carries 52 further unrelated lines; the staged
blob was constructed from HEAD's own blob with a single insertion and installed index-only, so the
live file was never written, normalized or overwritten. The **exact staged tree** was exported to a
new scratch directory and `cargo check -p tdb-qa --all-targets --locked --offline` passed there with
**exit 0 and zero warnings** — the same command that fails on unmodified `a2bbdc4`. After the commit
the index is empty, the live `Cargo.lock` hash is byte-for-byte what it was before the package
started, and all 242 working-tree entries are unchanged.

---

## 2. Preflight

| Check | Expected | Observed | Verdict |
|---|---|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` | **match** |
| Index | empty | 0 staged paths | **empty** |
| Live `Cargo.lock` SHA-256 | recorded | `7d0c4876be54ba5bb486a76d49417322a4c8eb11422b72928841d7bed8e3ba6d` | recorded |
| Worktree state | recorded | **242** `git status --porcelain` entries | recorded |
| HEAD `Cargo.lock` blob | — | `17662849e252880c4d7494b55cbecf2cfcaf3023` (131 228 bytes, 5465 lines, LF) | recorded |

Neither preflight condition differed, so staging proceeded.

---

## 3. Staging — index-only, constructed from HEAD

**The live working-tree `Cargo.lock` was never staged, read into the index, or modified.** The
repair was built from the committed blob:

1. `git show HEAD:Cargo.lock` → a copy in the scratch log directory (LF, as stored).
2. One insertion applied to that copy, guarded by a uniqueness assertion on the anchor and by an
   assertion that the blob is LF-only, so no newline conversion could occur. Result: 131 243 bytes,
   5466 lines — **+15 bytes, +1 line**.
3. `git hash-object -w` → blob `953c3370892acdb02db41ea4c79e011b4adbdcfe`.
4. `git update-index --cacheinfo 100644,953c337,Cargo.lock` — index-only; **the worktree file is not
   an input to this operation and was not touched**.

### 3.1 The complete staged diff

```diff
diff --git a/Cargo.lock b/Cargo.lock
index 1766284..953c337 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -3373,6 +3373,7 @@ dependencies = [
  "tdb-relations",
  "tdb-rx2",
  "tdb-savedata",
+ "toml 0.8.2",
 ]
 
 [[package]]
```

### 3.2 Required staging assertions

| Requirement | Observed |
|---|---|
| Exactly one changed file | `git diff --cached --name-only` → `Cargo.lock` |
| Exactly one added line, zero deletions | `git diff --cached --numstat` → `1	0	Cargo.lock` |
| `git diff --cached --check` | exit **0**, clean |
| Added line lands in `studio-domain` | Confirmed by reading `git cat-file -p :Cargo.lock` around the hunk: the entry is `[[package]] name = "studio-domain"`, and `"toml 0.8.2"` now follows `"tdb-savedata"` |
| No canonical package entry | `grep -c tdb-canonical` on the staged blob → **0** |
| No `tdb-qa` dependency added | Not present in the diff; the `studio-domain` `tdb-qa` edge remains uncommitted |
| No feature dependencies | `blender-job`, `tdb-headmesh`, `zune-core`, `zune-jpeg` absent from the diff |
| No version changes | The hunk adds a reference to `toml 0.8.2`, already pinned as a package at HEAD. No `[[package]]` block, `version`, `source` or `checksum` line was added, removed or altered |
| No newline-only changes | Diff is one `+` line; `--numstat` shows `1 0`. Line endings preserved (LF blob, as at HEAD) |
| Live file untouched | `sha256sum -c` → `Cargo.lock: OK` (still `7d0c4876…`) |

---

## 4. Verification of the exact staged boundary

### 4.1 Scratch location

| | |
|---|---|
| Root | `H:\c0-lock1\` |
| Pre-existing? | **No** — confirmed absent before creation |
| Prior evidence reused? | **No.** `H:\c0-slice2\` and the older `H:\c0-*` directories were read only for the C0-SLICE2 log cited in §6.2; none was written, moved or deleted |
| Live worktree switched/reset? | **No.** No `checkout`, `reset`, `stash`, `restore` or `read-tree` was run |
| Build outputs | `H:\c0-lock1\target\`, external — logs and inputs kept separate, logs outside `candidate\` |

### 4.2 Export of the staged tree

`git write-tree` → **`99841a7aa0e94ec40ba85d7a13b0996478fb8d6b`**, archived and extracted to
`H:\c0-lock1\candidate\` (**1135 files**). Archive SHA-256
`578a6d062f4aa139bb17a26aebe0516f6d7af60ea62b8198c774aaa984a1d320`.

This is the staged tree itself, not a reconstruction — so what was checked is exactly what was
committed.

| Boundary assertion | Observed |
|---|---|
| `Cargo.toml` vs HEAD | **content identical** (CRLF only, from `git archive`) — no workspace member added |
| `Cargo.lock` vs the staged blob | **content identical** |
| `Cargo.lock` vs HEAD | exactly `3375a3376 > " toml 0.8.2",` — **one added line, nothing else** |
| `crates/tdb-canonical/` present? | **absent** |
| `tdb-canonical` in the lockfile? | **0 occurrences** |

### 4.3 Hooks inspected before execution

| Checked | Finding |
|---|---|
| `.cargo/config.toml` anywhere in candidate | **none** — no registry override, runner or rustflags |
| `build.rs` anywhere in candidate `crates/` | **none** |
| `rust-toolchain` / `rust-toolchain.toml` | **none** — no toolchain pin, no implicit download |
| Network | `CARGO_NET_OFFLINE=true` plus `--offline` |

### 4.4 The check

Run in `H:\c0-lock1\candidate\` only. **Nothing was built or tested against the live source tree.**

```
cargo check -p tdb-qa --all-targets --locked --offline
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.66s
EXIT=0
```

| Assertion | Observed |
|---|---|
| Exit code | **0** |
| Warnings | **0** — `grep -c '^warning'` on the log returns 0 |
| Lockfile unchanged by the run | `sha256sum -c` after the run → **OK**. `--locked` accepted the file and cargo wrote nothing |
| Candidate inputs unchanged | `tar -df staged.tar` after the run: **zero** `Contents differ`, `Size differs` and `Mod time differs` across all 1135 members. Only Windows extraction metadata (`Mode`/`Uid`/`Gid`) differs |

This is the same command that **fails** on unmodified `a2bbdc4` (C0-SLICE2 §4.2, control tree), so
the check demonstrates the repair does the one thing it claims to.

Log: `H:\c0-lock1\logs\10-check.log`; comparison `11-tar-compare.txt`.

Verification did not fail, so no report-and-stop branch was taken and the repair was **not**
broadened.

---

## 5. The commit, and preservation afterwards

Committed with `git commit -F` from a message file, so no shell quoting touched the message.

| Check after commit | Observed |
|---|---|
| Commit | `0a791892dcfef11d079e873c2444d5e70bf29262` |
| Parent | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` |
| `git show --stat` | `Cargo.lock | 1 +` · **1 file changed, 1 insertion(+)** |
| Commit diff content | Byte-identical to §3.1 — the one-line repair and nothing else |
| Trailer | `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>` present |
| **Index** | **empty** (0 staged paths) |
| **Live `Cargo.lock` SHA-256** | `7d0c4876be54ba5bb486a76d49417322a4c8eb11422b72928841d7bed8e3ba6d` — **unchanged** from preflight |
| **Other working-tree changes** | `git status --porcelain` **identical** to the preflight capture, all **242** entries |
| Pushed | **No.** No `git push` was run; the branch has no upstream ref |

### 5.1 What the live `Cargo.lock` looks like now

Its diff against the new HEAD dropped from 53 to **52 added lines** — precisely the one line this
commit absorbed. What remains uncommitted there is the canonical package entry (10 lines), the
`blender-job` and `tdb-headmesh` entries, `zune-core`/`zune-jpeg`, `studio-domain`'s `tdb-qa` edge
and `tdb-instance-registry`'s `toml 0.8.2` edge — all untouched, all still available for whatever
boundaries are accepted later.

---

## 6. Relationship to earlier records

### 6.1 The newly isolated cause

`crates/studio-domain/Cargo.toml:25` declares `toml = "0.8"`; the committed `Cargo.lock` omitted it
from `studio-domain`'s dependency list. The lockfile was stale against its own manifest, so **every**
`--locked` cargo command failed on a clean checkout with `error: cannot update the lock file …
because --locked was passed`.

This cause was isolated in C0-SLICE2 §4.2 using a pure-HEAD control tree with no overlay, which
proved the failure predates and is independent of the canonical slice. It sharpens the earlier,
unattributed observation in **C0-INT1 §7.1** that `--locked` fails at HEAD, and it relates to the
open `cargo fmt` / `cargo clippy` baseline defects recorded in
[docs/validation/c0-02-baseline-execution.md](../validation/c0-02-baseline-execution.md).

**No historical rewrite was performed.** `c0-02-baseline-execution.md` and the C0-INT1 record are
unedited; this receipt records the newly isolated cause and links to them. Whether the C0.2 record
gains a cross-reference is the architect's call.

Note the scope of what is now fixed: this repair makes `--locked` **run**. The `fmt` and `clippy`
repo-wide failures recorded in C0.2 are untouched and remain open.

### 6.2 Correction appended to the C0-SLICE2 receipt

As instructed, a labelled **§11 Corrections** section was appended to
[c0-slice2-canonical-probe.md](c0-slice2-canonical-probe.md), correcting two things while preserving
every other finding in that record:

1. **Lockfile change size.** That receipt repeatedly said "two added lines", conflating change
   *regions* with lines. The candidate's lockfile delta is **two regions totalling 11 added lines**:
   this one baseline dependency line, plus the **ten-line** `tdb-canonical` package entry.
2. **Warning-free claims.** Its scoped `-p tdb-canonical` checks were genuinely warning-free; the
   supplementary full-workspace `check` (run 5) exited 0 but **did** emit warnings — 5 warning
   summaries from `tdb-rx2` and `studio-domain` at HEAD, **none from `tdb-canonical`**. The original
   table said only "exit 0" and did not distinguish the two.

Neither correction changes C0-SLICE2's conclusions: the canonical candidate's own diagnostics are
still zero, and the slice is still verified at its own boundary.

---

## 7. Unresolved questions for the architect

1. Should `docs/validation/c0-02-baseline-execution.md` gain a cross-reference to this repair, now
   that one of its `--locked` symptoms has a named cause and a commit? This package made no such
   edit.
2. Is the canonical slice now cleared for its own commit on top of `0a79189`? C0-SLICE2's single
   stated blocker is resolved by this commit; its remaining lockfile delta is the ten-line package
   entry alone. **No canonical commit was made or attempted here.**
3. Should this receipt and C0-SLICE2 be committed together with whatever lands next, or separately?
   Both are currently uncommitted.
4. `H:\c0-lock1\` is retained as evidence — keep or discard?

---

## 8. Package boundaries observed

Preflight, index-only staging, isolated verification, one authorized commit, and these records.
**No** canonical commit, feature commit, generator run, live-tree build or test, game-data access,
private-fixture root, blanket staging, worktree switch/reset, or push. Prior scratch evidence was
preserved. **Stopping here for architect review.**
