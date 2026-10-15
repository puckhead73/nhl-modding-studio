# C0-COMMIT-REG1 — commit the reviewed Legacy adapter

**Package:** C0-COMMIT-REG1 · **Date:** 2026-09-14 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling`

| | |
|---|---|
| **Result** | **Committed.** All three gates passed on the exact staged tree, exported and run in isolation |
| **Commit** | `1ede5baa8ba1ab261a253c9bfdcc5e0557116e99` |
| **Parent** | `819940a9fcf255221ce64bb6b0b2229942ec304a` (HEAD at start, as authorized) |
| **Commit tree** | `b082fb892ce9d5ff567619bb1d2e1856fc9d6cb6` — **equal to the verified staged tree** (`git write-tree` before export, re-checked immediately before commit) |
| **Subject** | `feat(canonical): register measured Legacy Xbox 360 adapter` |
| **Scope** | **9 paths: 3 added, 6 modified, 0 deleted** — 3 254 insertions, 19 deletions |
| **Canonical suite on the staged tree** | **214 passed · 0 failed · 0 ignored** |
| **Index after** | empty |
| **Pushed?** | **No.** Not authorized, not attempted |

> Acceptance scope, as stated by Codex: the accumulated C0-REG1a / R1 / R2 slice **within its recorded
> scope**. The adapter is partial (10 / 2 965 fields), `player.shoots` and `player.team` remain
> blocked, and no cell is game-verified. The documentation in §6 is **uncommitted**.

---

## 1. Preflight

| Check | Result |
|---|---|
| HEAD | `819940a9fcf255221ce64bb6b0b2229942ec304a` |
| Index | empty (`git diff --cached --stat` → 0 bytes) |
| `git status --porcelain` | 263 entries (recorded) |
| Eight source/configuration files vs C0-REG1a-R2 §7 | **8 / 8 OK** — `sha256sum -c` over the R2 overlay manifest; no drift, nothing to investigate |
| Live `Cargo.lock` / root `Cargo.toml` | `f1ed4397…` / `5272dcbd…` — the live lock carries the pre-existing unrelated drift (42 lines vs HEAD) plus the two lines this package authorizes; it was **not** used as the staged lock |

## 2. Staging — exact membership

Each of the eight files was `git add`ed individually. The lock was constructed outside the working
tree: `git show HEAD:Cargo.lock` → the two lines inserted → `git hash-object -w` (blob
`bb021221…`) → `git update-index --cacheinfo 100644,<blob>,Cargo.lock`. Root `Cargo.toml` not staged.

| Status | Path |
|---|---|
| A | `defs/adapters/nhllegacy-xbox360.toml` |
| A | `crates/tdb-canonical/src/snapshot_tests.rs` |
| A | `crates/tdb-canonical/src/legacy_xbox360_tests.rs` |
| M | `crates/tdb-canonical/Cargo.toml` |
| M | `crates/tdb-canonical/src/lib.rs` |
| M | `crates/tdb-canonical/src/model.rs` |
| M | `crates/tdb-canonical/src/tests.rs` |
| M | `crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml` |
| M | `Cargo.lock` |

**Lock delta** (`git diff --cached -- Cargo.lock`, the whole diff):

```
@@ -3814,7 +3814,9 @@ version = "0.0.0"
 dependencies = [
  "serde",
  "sha2",
+ "tdb-core",
  "tdb-qa",
+ "tdb-savedata",
  "toml 0.8.2",
 ]
```

`git diff --cached --name-status`: **3 A · 6 M · 0 D = 9**. `git diff --cached --check`: clean.
Every staged blob, LF-normalised, hashes identically to the live file and to the R2 candidate's copy
(eight-way check recorded in the scratchpad log); the exported-tree hashes of three files differ from
R2's manifest only by the CRLF checkout representation `git archive` applies.

## 3. Verification on the exact staged tree

`git archive b082fb89…` extracted into `…/scratchpad/cr1/candidate` (1 189 files — the same
membership as the R2 candidate). Build output in `…/cr1/target`, outside the candidate. `QA_WRITE`
and `QA_EVIDENCE_ROOT` verified unset in the shell that ran every command.

| Command | Result | Baseline (R2) |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **214 passed · 0 failed · 0 ignored** | identical |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0, no warnings | identical |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty output | identical |
| Candidate membership (SHA-256 manifest before/after) | 1 189 files, **0 differing entries** | — |

No discrepancy arose; no assertion was touched. The `tdb-core` / `tdb-savedata` suites were not
rerun: those crates' sources in the staged tree are HEAD's, byte-identical to the R2 candidate's,
where they passed 47.

## 4. Commit and preservation

Committed with `git commit -F …/cr1/commit-msg.txt`. The message describes the partial adapter,
the target-bound verification, the fixture-specific capacities, the sentinel representation fixes,
and states that the codec-order coverage is a test helper and the crate ships no production adapter
executor.

| Check | Result |
|---|---|
| `git write-tree` immediately before commit | `b082fb89…` — unchanged since export |
| Commit tree | `b082fb89…` — **equal** |
| Index after | empty |
| `git status --porcelain` | 255 entries: exactly the **8** entries the commit resolved are gone (5 modified `tdb-canonical` paths, 2 untracked test modules, the untracked adapter); **0 new entries**; `Cargo.lock` still shows `M` for the unrelated 42-line drift only |
| Eight live files | byte-identical to the R2 manifest after the commit (8 / 8 OK) |
| Live `Cargo.lock` / root `Cargo.toml` | byte-unchanged (`f1ed4397…`, `5272dcbd…`) |
| Unrelated modified/untracked content | untouched — no checkout, reset, stash, clean or branch change was run |

## 5. Remaining blockers (unchanged by this commit)

| Blocker | Status |
|---|---|
| `player.team` parent table (`kOtt.vfEq`) | candidate; containment + name is not a foreign key; mapping blocked |
| `player.shoots` stored codes | candidate; matching handedness counts prove counts only; no independently supported label in the repository |
| 2 955 of 2 965 Legacy fields | unmapped, recorded as unknown |
| Runtime capacity ceilings | fixture directory capacities only; no runtime limit demonstrated |
| NHL 15 / NHL 10 roster-save capacities | untested until their own fixtures are measured (no proxy) |
| Registry population | 3 / 28 database units, 1 / 14 asset families — deliverable not met |
| Game verification | no cell anywhere |

## 6. Documentation — uncommitted

Outside the commit, as instructed. **Uncommitted after this package:**

- `docs/receipts/c0-reg1a-legacy-xbox360-adapter.md`, `c0-reg1a-r1-legacy-evidence-correction.md`,
  `c0-reg1a-r2-representation.md`, `c0-commit-reg1.md` (this) — and every earlier receipt under
  `docs/receipts/`, which remains an untracked directory;
- `docs/plans/project-completion-roadmap.md` — C0 item 7 now reports 3/28 units **committed**
  (`1ede5ba`), partial coverage and the two blocked mappings retained;
- `docs/STATE-OF-THE-REPO.md` — §7 row likewise;
- everything else in the working tree that was already modified or untracked before this package
  (`CLAUDE.md`, root `Cargo.toml`, the drifted `Cargo.lock`, `app/`, `studio-*`, headmesh crates, …).

## 7. Limits honoured

No feature work, headmesh work, external-repository access, evidence generation/publication, game
installation, capability promotion, broad cleanup or unrelated repair. No push. Nothing beyond the
nine authorized paths was staged or committed. **C0-REG1b (NHL 15 PS3 from its own attributed
evidence and directly measured save fixture) was not started.** Stopping for Codex review.
