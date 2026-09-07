# C0-INT1 — Integration of the reviewed overnight baseline repairs

> **Type:** integration of an already-reviewed repair set into the **live working tree**.
> **Input:** [c0-overnight-baseline-repairs.md](c0-overnight-baseline-repairs.md) and the six
> patches in `H:\c0-repair\c0-ovn-20260907-105522\evidence\patches\`, approved by Codex.
> **Execution date:** 2026-09-07 (UTC). **Author role:** developer. Reviewer: Codex (architect).
> **Git:** one authorized commit (the overnight report). **No source file was staged or committed.
> Nothing was pushed.** No reset, checkout, stash, cleanup, dependency change or game-file write.

Conventions follow C0.2: **Passed / Failed / Blocked / Not run** describe a check that actually
executed. A case reporting `ok` after returning early on an absent fixture is **skipped coverage**,
never a pass.

---

## 1. Result

The live working tree now contains **exactly** the reviewed repair set — 147 `.rs` files,
byte-for-byte equal to the overnight run's tested final source. No other file was written.

| C0.2 defect | Status **in the working tree** |
|---|---|
| **B1** — `cargo fmt --all -- --check` fails | **Repaired in the working tree.** `--check` exits 0. |
| **B2** — `cargo clippy … -D warnings` fails | **Repaired in the working tree** (evidence reused, §6). |
| **B3** — vacuous `assert!(… \|\| true)` | **Repaired in the working tree.** |
| B4, B5, B6, B7, and the DTO `--check` gate (C0.2 R5) | **Untouched — still open.** |

**These repairs are present in the working tree, not in committed HEAD.** The only commit this
package made is the overnight report itself. Committing the source is a separate, unauthorized step,
and §7 records why it cannot be a single clean commit.

---

## 2. Identity and inputs

| Item | Value |
|---|---|
| Repository | `E:\Repositories\nhl-modding-studio`, branch `feat/phase5-asset-tooling` |
| HEAD before integration | `2a70b4d0ea8c3a213678f5f76f685da940821434` |
| HEAD after the authorized doc commit | `0839daea53f3e7340d2a1d548fcf907ec684e774` |
| Repair workspace (read-only input) | `H:\c0-repair\c0-ovn-20260907-105522` |
| Integration evidence directory (**new**) | `H:\c0-integration\c0-int1-20260907-183530` |
| Index state before and after | **empty** — verified at both boundaries |

The original `base\`, `source\`, `replay\` and `evidence\` trees were **read only**. Nothing in the
overnight workspace was modified, added to or deleted.

### 2.1 The integrated patches — hashes and order

Applied in this order and no other. SHA-256 of each patch file as integrated:

| # | Patch | SHA-256 | Files |
|---|---|---|---|
| 1 | `01-arena-assertion.patch` | `0d20a12d204eedb94579e77d93c9e46f8649fabdf8bd01ec3adf676be1fb9648` | 1 |
| 2 | `02-rustfmt.patch` | `7ab54e594040f4adbaf1a1c798cee16038ea0d2537302ade65a6354485ee0f9a` | 122 |
| 3 | `03-lint-codec-and-foundational.patch` | `b0c5ce141110a5f49349773d81c3755d7c3ba672c1aa4501b71b907dfe8d22cc` | 11 |
| 4 | `04-lint-numeric-machine-applicable.patch` | `ee92f5310df1051ef81510a60296c774055e3ff1eeee04a15809ab7037bc7f26` | 18 |
| 5 | `05-lint-mechanical-machine-applicable.patch` | `1866524400d38f4b317254d8803431b78c30763cc15ce9d628e4063bd45a59bb` | 21 |
| 6 | `06-lint-hand-remaining.patch` | `231475e3e1a2ab24cbaa7d61f2f3ada13abdca03617bb238f0b4f4c73acd2c10` | 25 |

Union of the six file lists: **147 distinct paths**, all `.rs`.

---

## 3. Base verification — the live tree had not drifted

Before anything was written, every affected live file was hashed and compared to the overnight
patch base (`manifests/source-start.sha256`).

| Check | Result |
|---|---|
| Affected paths present live | **147 / 147** — none missing |
| Live bytes equal to the overnight base | **147 / 147 matched. 0 mismatches, 0 absent from base.** |
| Files the repair changed outside the affected set | **0** — the repair's start and final manifests differ on exactly those 147 members |
| HEAD at the write boundary | unchanged (`2a70b4d`) |
| Index at the write boundary | empty |

No live file had changed since the overnight base, so **no conflict arose** and no three-way merge
was needed or attempted. The hashes were re-taken immediately before the write boundary and again
matched (`evidence/live-recheck.sha256` is identical to `evidence/live-before.sha256`).

### 3.1 Backup

A **byte-exact** backup of all 147 affected live files, in their pre-integration state, was written
to:

```
H:\c0-integration\c0-int1-20260907-183530\backup\   (147 files, original tree layout)
```

Verified by re-hashing the backup and diffing against `live-before.sha256`: **identical**. This is
the rollback source; restoring it returns the working tree to its pre-integration bytes exactly.

---

## 4. Preflight against an independent copy

The complete ordered set was applied first to an **independent copy** of the current live affected
source (`preflight\`, seeded from the verified backup), not to the live tree.

| Patch | `patch -p1 --binary` exit | Rejected hunks | Fuzz |
|---|---|---|---|
| 01 … 06 | **0** for all six | **0** | **0** |

Result compared to the reviewed final hashes (`manifests/source-final.sha256`):

- **147 / 147 files exact match. 0 mismatches.**
- **0 extra files** created anywhere in the preflight tree.
- **0** `.rej` or `.orig` files produced.

The preflight therefore reproduced the reviewed final source exactly, which is what authorized the
live write.

---

## 5. Integration and post-write verification

The 147 verified preflight files were copied over their live counterparts. **147 files written; no
other path touched.**

| # | Check | Result |
|---|---|---|
| 1 | Live affected files vs. **reviewed final** hashes | **Passed — 147 / 147 exact.** |
| 2 | Full snapshot membership (1,225 files) vs. `source-final.sha256` | **Passed — 1,225 / 1,225 exact, 0 missing.** |
| 3 | Live `.rs` files that are *not* tested-snapshot members | **0** — every Rust file in the tree is covered by the tested state |
| 4 | `git status` entries lost or status-flipped | **0** — the before/after diff is additions only |
| 5 | Newly-modified paths outside the affected set | **0**; all 96 newly-modified paths are `.rs` files in the affected set |
| 6 | `cargo fmt --all -- --check` | **Passed — exit 0, empty output** |
| 7 | DTO bindings, anchors, `Cargo.lock`, `docs/audits/data/**` changed by integration | **None.** The 12 dirty paths in those trees are pre-existing user work, present in the before snapshot and unmodified here. |
| 8 | Stray `.rej` / `.orig` anywhere in the repository | **0** |
| 9 | Index after integration | **empty — no source staged** |

Check 2 is the strong form of the acceptance criterion: not merely that the 147 changed files are
right, but that **every** member of the tested snapshot is byte-identical in the live tree.

### 5.1 Preservation of earlier work

`git status` went from 121 to 217 entries; the difference is **96 additions and zero removals**,
every one a ` M` on a file in the affected set. All pre-existing modifications and untracked files —
including the frontend work, `Cargo.lock`, the modified DTO binding and the untracked bindings —
survive untouched.

---

## 6. Reused evidence, and why the reuse is valid

Check 2 above establishes that the live Rust source is **byte-identical** to the source the
overnight run tested — not merely equivalent, and not merely for the changed files. Check 3
establishes there is no additional live Rust file outside that tested set. Integration therefore
changed **no** reviewed code, and the overnight evidence is reused **explicitly** rather than re-run:

| Reused evidence | Identity | Result being reused |
|---|---|---|
| Strict clippy | `evidence/logs/51-clippy-strict.log` | `cargo clippy --workspace --all-targets --locked -- -D warnings` → **exit 0, 0 diagnostics** |
| Full suite | `evidence/logs/61-full-workspace-tests.log` | `cargo test --workspace --locked` → **668 passed, 0 failed, 7 ignored** |
| Focused codec/mesh tests | `evidence/logs/60-focused-codec-tests.log` | 308 passed, 0 failed |
| Serial mount-gated run | `evidence/logs/62-mountgated-serial.log` | 347 passed / 0 failed / 5 ignored; 15 missing-fixture skips, 2 disabled boot probes |
| Frontend | C0.2, via the 124-binding before/after hash equality in the overnight §6.2 | no DTO changed |

All of these belong to the final repaired state whose manifest digest is
`6c52dfbdd15be2c831df9572e88773f86ecea4ad7d82cfdfbb50f2efdd9569df` — the state the live tree now
holds. The 31-minute suite was **not** re-run, per the package. `cargo fmt --all -- --check` *was*
re-run live (check 6) and passes independently.

### 6.1 What that evidence still does not cover

Unchanged by this package, and restated so the pass totals are not read as coverage:

- **LZX coverage remains unqualified.** `crates/tdb-archive-codec/src/lzx.rs` has no unit tests and
  its only oracle (`eb::chunklzx_decodes_against_extracted_oracle`) is `#[ignore]`d. The two
  `mem::swap` rewrites were **accepted by the architect on inspection**; nothing has executed them.
- **B4** — 15 cases report `ok` while returning early on an absent fixture, including the whole
  `tdb-rx2` texlib/XPR2/`playermesh` surface that patches 04–06 edited. Those edits carry no
  fixture-level coverage.
- **B5** — the hardcoded absolute paths, including the clean-room fixtures pointing into
  `E:\Repositories\nhl-database-studio`.
- **B6, B7**, the DTO `--check` gate (C0.2 R5) and Windows CI (R6).
- **No cell anywhere is game-verified.** This package qualifies no target.

---

## 7. Commit-boundary plan — prepared, **not executed**

Nothing below has been staged or committed. It records what a source commit would require.

The 147 affected files fall into three classes. Note that `git status` collapses wholly-untracked
directories, so the classification uses `git ls-files`, not the status listing:

| Class | Count | Description |
|---|---|---|
| **A** | **96** | tracked and **clean** before integration — the repair is their only delta from HEAD |
| **B** | **27** | tracked but carrying **pre-existing uncommitted modifications** — repair and in-flight feature work are interleaved in the same file |
| **C** | **24** | **untracked source** — never committed at all |

Lists: `evidence/class-A.txt`, `class-B.txt`, `class-C.txt`.

### 7.1 Class A — a standalone repair commit is possible, and it builds

Verified empirically, not asserted. A probe tree was built from `git archive HEAD` overlaid with
**only** the 96 class-A repaired files (`evidence/boundary-probe-check.log`):

```
cargo check --workspace --all-targets --offline        exit 0
```

So the class-A repairs do **not** depend on the uncommitted work to compile. Two caveats:

- `--locked` fails on that tree (`cannot update the lock file`). This is a **pre-existing HEAD
  condition** — the overlay changes only `.rs` files, which cannot affect dependency resolution —
  and it means a class-A-only commit would need the `Cargo.lock` question resolved separately.
- Two warnings remain (`unused variable: 'year'` in `tdb-rx2`); they are not errors.

### 7.2 Class A alone does **not** close B1 or B2

Also measured on the same probe:

| Gate on HEAD + class A only | Result |
|---|---|
| `cargo fmt --all -- --check` | **Failed** — 413 diffs across **26 files**: the 25 class-B files still unformatted at HEAD, plus `crates/tdb-rx2/src/bc45.rs`, whose *uncommitted* working-tree version is already rustfmt-clean while its HEAD version is not |
| `cargo clippy --workspace --all-targets -- -D warnings` | **Failed** — **47 errors**, every one located in a class-B file: `tdb-fhm-store/src/lib.rs`, `tdb-rx2/src/{gnf,mcd,pack,playermesh,texlib}.rs` |

**Consequence:** B1 and B2 cannot be closed on `main`-visible history by a class-A-only commit. They
are closed in the working tree only, and stay so until the in-flight work lands.

### 7.3 Dependencies that block a standalone repair commit

1. **Two whole crates are untracked.** `crates/tdb-headmesh` (18 affected files) and
   `crates/blender-job` (1) have **zero** tracked files, and are added to the workspace only by the
   **uncommitted** `Cargo.toml` change. Their repairs cannot be committed until the crates and that
   `Cargo.toml` edit are committed — a scope far beyond a lint fix.
2. **Untracked modules are gated by modified module roots.** `crates/studio-domain/src/lib.rs` and
   `crates/tdb-fhm-store/src/lib.rs` are class B; the untracked `legends.rs`, `player_manifest.rs`
   (studio-domain) and `legends.rs` (tdb-fhm-store) are reachable only through them. Committing the
   untracked modules requires committing the modified roots.
3. **27 class-B files are genuinely mixed.** Their repaired bytes contain both the lint/format repair
   and in-flight feature work. Staging them whole would commit unreviewed feature work under a repair
   subject. Per the package, that shortcut was **not** taken and no split was fabricated: separating
   them needs hunk-level review against each file's HEAD version, and three of the five lint
   allowances (`team_kit_preview` ×2 in `studio-api`, `TAG_FX_RENDERABLE` in
   `tdb-rx2/playermesh.rs`) live inside class-B files.
4. **`Cargo.lock`** is already modified in the working tree and is required for any `--locked` gate.

### 7.4 Recommended sequence (for the architect to accept or replace)

1. Land the in-flight Phase-5 asset work, including the untracked crates and the `Cargo.toml` /
   `Cargo.lock` changes, on its own merits.
2. Re-run `cargo fmt --all` and the strict clippy gate on the result — by then the class-B/C files
   are tracked and clean, so the repair becomes a single mechanical commit.
3. Only then claim B1/B2/B3 closed at HEAD.

Committing class A alone first is possible and builds, but it closes no defect and leaves a second
formatting commit to follow; it is offered as an option, not a recommendation.

---

## 8. What was not done

No source file staged or committed. No push. No reset, checkout, stash, restore, cleanup or branch
change. No dependency, lockfile or toolchain change. No `#[ignore]`d test enabled, no test removed or
weakened, no gated test switched on. No generator run and no `docs/audits/data/**` file touched. No
QA status promoted and no capability claimed. No DTO regenerated. No anchor or game directory
written. The overnight workspace's `base\`, `source\`, `replay\` and `evidence\` trees were read
only. The evidence-contract implementation was **not** started.
