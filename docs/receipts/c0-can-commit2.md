# C0-CAN-COMMIT2 — commit the repaired, verified canonical slice

**Package:** C0-CAN-COMMIT2 · **Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling`

| | |
|---|---|
| **Result** | **Committed.** All gates passed on the exact staged tree |
| **Commit** | `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` |
| **Parent** | `0a791892dcfef11d079e873c2444d5e70bf29262` (as authorized) |
| **Commit tree** | `824d19da7303d25d1a5443b44405fb44c4b65d03` — **identical to the verified staged tree** |
| **Subject** | `feat(canonical): add evidence-qualified adapters and asset profiles` |
| **Scope** | **27 files, 21 728 insertions, 0 deletions** |
| **Canonical suite** | **200 passed · 0 failed · 0 ignored · 0 filtered** |
| **Index after** | empty |
| **Pushed?** | **No.** Push is not authorized and was not attempted |

> This receipt is left **uncommitted**, as are the four earlier ones. No push, no second commit, no
> live-tree build or test, no game-data access, no repository newline-policy change, no `.gitattributes`
> or git-configuration edit, no broad reset, and no deletion of prior evidence.

---

## 1. Result in one paragraph

The canonical slice is committed as `ef0fdd7` on the authorized parent. Its tree hash equals the
staged tree that was exported and verified byte-for-byte beforehand, so what was checked is exactly
what landed. Staging reproduced the C0-CAN-COMMIT1 boundary precisely — the two mixed-file blobs came
out with the **same ids** (`61355c88…`, `f68b9140…`) despite being rebuilt independently from HEAD —
and all 25 files in the exported tree were **byte-identical** to C0-CAN-EOL1's verified CRLF
candidate, so the line-ending representation that previously broke the suite is the one that was
tested and committed. `cargo test` returned **200 passed, 0 failed, 0 ignored**; `clippy -D warnings`
and `fmt --check` both exit 0 with zero diagnostics. Every one of the 27 live paths is byte-identical
to its pre-package hash.

---

## 2. Preflight

| Check | Required | Observed | Verdict |
|---|---|---|---|
| HEAD | `0a791892…` | `0a791892…` | **match** |
| Index | empty | 0 staged paths | **empty** |
| Scratch root | new | `H:\c0-can-commit2\` confirmed absent before creation | **new** |
| 25 input hashes | 23 from C0-SLICE2 §2.3 + 2 revised from C0-CAN-EOL1 §8.2 | `sha256sum -c` → **25 OK, 0 failed** | **match** |
| `Cargo.toml` | recorded | `5272dcbddaaeb834e1defbb659f090becc6eca5167f63c512c9481b849459e82` | recorded |
| `Cargo.lock` | recorded | `7d0c4876be54ba5bb486a76d49417322a4c8eb11422b72928841d7bed8e3ba6d` | recorded |
| Worktree | recorded | **242** `git status --porcelain` entries | recorded |

The expected-hash file was assembled explicitly: the C0-SLICE2 list with the two test-module entries
removed, then the C0-CAN-EOL1 §8.2 values appended —
`build_override_tests.rs` → `59f84e98…`, `asset_tests.rs` → `2bd9621d…`. HEAD blobs of the mixed
files before staging: `9d52f2c5…`, `953c3370…`.

---

## 3. Staging — exactly the authorized 27

The 25 files were staged **one at a time by explicit path** in a loop over the manifest; no directory
argument, no `git add -A`, no glob. The two mixed files were **never staged from their dirty
working-tree copies**: each was rebuilt from `git show HEAD:<path>`, patched with a single guarded
insertion (asserted LF-only and unique-anchor), written with `git hash-object -w` and installed with
`git update-index --cacheinfo` — index-only, leaving the working-tree bytes untouched.

| Path | HEAD blob | Staged blob | Change |
|---|---|---|---|
| `Cargo.toml` | `9d52f2c5…` | `61355c8848bb0abe340f40e560936925cd22b9f1` | **+1** line: `"crates/tdb-canonical",` |
| `Cargo.lock` | `953c3370…` | `f68b91406671619982d7b2095412a03c1c795599` | **+10** lines: the `[[package]] tdb-canonical` entry |

Both blob ids match C0-CAN-COMMIT1's exactly — an independent reconstruction landing on the same
bytes.

| Requirement | Observed |
|---|---|
| Exactly the 27 authorized paths | sorted staged list `diff`s clean against the authorized list |
| Composition | **25 `A` + 2 `M`** |
| Deletions | **0** (`--diff-filter=D` → 0) |
| `Cargo.toml` / `Cargo.lock` numstat | `1 0` / `10 0` |
| `git diff --cached --check` | exit **0** |
| No receipts, docs, feature or evidence-system paths | none present — the staged set is exactly the manifest plus the two mixed files |

---

## 4. Verification of the exact staged tree

### 4.1 Scratch

`H:\c0-can-commit2\` — new. `candidate\` holds the exported tree, `logs\` the evidence, and
`target\` the build output, all **outside the candidate's inputs**. Prior scratch roots
(`c0-slice2`, `c0-lock1`, `c0-can-commit1`, `c0-can-eol1`) were read but never written.

### 4.2 Export and identity

`git write-tree` → **`824d19da7303d25d1a5443b44405fb44c4b65d03`**, archived
(SHA-256 `3325fa9825a69fbe4ecae2cea1ec52e6e37e99e8eab03280bb2262e51d885a48`) and extracted:
**1160 files** = HEAD's 1135 + the 25 additions. Hooks inspected first: **no `.cargo/config.toml`,
no `build.rs` under `crates/`, no toolchain pin.**

### 4.3 Newline accounting

| Measurement | Result |
|---|---|
| The 25 files **on disk** (live) | **23 LF, 2 CRLF** |
| The 25 files **in the exported staged tree** | **0 LF, 25 CRLF** — `git archive` applied the working-tree conversion under `core.autocrlf=true` |
| Exported tree vs **C0-CAN-EOL1 `cand-crlf`** (the verified representation) | **25 IDENTICAL-BYTES**, 0 newline-only, 0 content differences |
| `Cargo.toml` / `Cargo.lock` vs the blobs built from HEAD | content-identical, CRLF only |

This is the point the whole repair exists for: the committed form is the CRLF form, and it is
byte-for-byte the tree C0-CAN-EOL1 proved green. No inference from raw working-tree hashes was
relied on.

### 4.4 Checks

Run in `H:\c0-can-commit2\candidate\` only, `CARGO_NET_OFFLINE=true`, external `CARGO_TARGET_DIR`:

| Command | Exit | Result |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **0** | **200 passed · 0 failed · 0 ignored · 0 filtered**; 0 build warnings |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | **0** | 0 warnings |
| `cargo fmt -p tdb-canonical -- --check` | **0** | clean |

All diagnostics are canonical-scoped; a `-p tdb-canonical` build surfaces no dependency warnings.
`Cargo.lock` hashed identically after the runs, so `--locked` mutated nothing.

**The workspace check was not rerun**, per the package. C0-SLICE2 §4.4 run 5 and C0-CAN-COMMIT1 §4.5
remain the supplementary evidence for it, measured on earlier trees; nothing here re-establishes it,
and no such claim is made.

### 4.5 Candidate integrity, before and after

| Check | Result |
|---|---|
| Membership | **1160 → 1160**, listings `diff` clean — **no generated or gitignored file appeared** (the scoped runs do not trigger the Tauri codegen that the earlier workspace check did) |
| The 27 inputs | `sha256sum -c` → **all 27 unchanged** |
| Whole tree | `tar -df` → **zero** `Contents differ` / `Size differs` / `Mod time differs` across all 1160 members |

---

## 5. The commit

Created with `git commit -F` from a message file, so no shell quoting reached the message.

| Check | Observed |
|---|---|
| Commit | `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` |
| Parent | `0a791892dcfef11d079e873c2444d5e70bf29262` — the authorized parent |
| Tree | `824d19da…` — **equals the verified staged tree**, so the commit is the boundary that was checked |
| `--numstat` totals | **27 paths · 21 728 insertions · 0 deletions** |
| Trailer | `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>` present |
| Index after | **empty** |
| Pushed | **No.** `git status -sb` shows `## feat/phase5-asset-tooling` with no upstream |

### 5.1 Preservation and status accounting

**All 27 live paths are byte-identical** to their preflight hashes — the 25 sources plus `Cargo.toml`
and `Cargo.lock`, whose dirty working copies were never written.

Working-tree status moved **242 → 239**, and the delta is exactly the three expected disappearances:

```
- ?? crates/tdb-canonical/
- ?? defs/adapters/
- ?? defs/canonical/
```

Those three untracked-directory entries collapsed because every file beneath them became tracked —
21 + 3 + 1 = the 25 committed files. Nothing else changed: `Cargo.toml` and `Cargo.lock` remain
` M` with their feature-side content intact, and no path was added to the status list. A follow-up
`git status --untracked-files=all` over those three trees returns **empty**, confirming no file was
left behind and none was committed beyond the 27.

`git ls-files` now reports **21** files under `crates/tdb-canonical/` and **4** under
`defs/canonical/` + `defs/adapters/`.

### 5.2 What remains uncommitted in the two mixed files

`Cargo.toml`'s live delta fell from 3 added lines to **2** (`tdb-headmesh`, `blender-job`);
`Cargo.lock`'s from 52 to **42**. Everything still outstanding there is feature-side —
`blender-job`, `tdb-headmesh`, `zune-core`, `zune-jpeg`, plus `studio-domain`'s `tdb-qa` edge and
`tdb-instance-registry`'s `toml` edge. No canonical content remains uncommitted.

---

## 6. Evidence retained

All five scratch roots are present and untouched: `H:\c0-slice2`, `H:\c0-lock1`,
`H:\c0-can-commit1`, `H:\c0-can-eol1`, `H:\c0-can-commit2`. The **failed** C0-CAN-COMMIT1 candidate
still verifies byte-identical to its staged archive, and C0-CAN-EOL1's `cand-lf` and `cand-crlf`
both still hold 1160 files. Nothing was deleted.

The commit history now reads:

```
ef0fdd7 feat(canonical): add evidence-qualified adapters and asset profiles
0a79189 fix(build): record studio-domain TOML dependency in lockfile
a2bbdc4 docs(validation): record accepted DTO drift gate
```

---

## 7. Unresolved questions for the architect

1. **Five receipts are now uncommitted** — `c0-slice2-canonical-probe.md`,
   `c0-lock1-baseline-lockfile-repair.md`, `c0-can-commit1.md`,
   `c0-can-eol1-test-portability.md` and this one. Do they land as one documentation commit, and
   should any be mirrored into `docs/validation/`?
2. **Nothing is pushed.** Three commits now sit ahead of the branch's last published state.
3. C0-CAN-EOL1 §9's open items stand: whether a Linux/CI reproduction is wanted before the branch
   moves further, and whether the deliberate raw (unnormalized) reads in `tests.rs`/`asset_tests.rs`
   are the intended long-term shape.
4. `docs/STATE-OF-THE-REPO.md` and `CLAUDE.md`'s crate table still describe a workspace without
   `tdb-canonical`; updating them is documentation work this package did not touch.

---

## 8. Package boundaries observed

Preflight against the revised hash set, explicit-path staging, index-only mixed-file construction,
isolated verification of the exact staged tree, one authorized commit, and this receipt. **No push**,
no second commit, no live-tree build or test, no game-data access, no newline-policy or
`.gitattributes` change, no broad reset, no receipts or documentation in the commit, and no deletion
of prior evidence. **Stopping here for architect review.**
