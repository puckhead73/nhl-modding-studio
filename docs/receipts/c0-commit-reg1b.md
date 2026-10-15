# C0-COMMIT-REG1b — commit the reviewed NHL 15 PS3 adapter

**Package:** C0-COMMIT-REG1b · **Date:** 2026-09-14 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling`

| | |
|---|---|
| **Result** | **Committed.** All three gates passed on the exact staged tree, exported and run in isolation |
| **Commit** | `58d1bc073bbf9e80d5afb4782bc6c132725e5fed` |
| **Parent** | `1ede5baa8ba1ab261a253c9bfdcc5e0557116e99` (HEAD at start, as authorized) |
| **Commit tree** | `6af4f70446124acd08520ad6d2cb9f099671a502` — **equal to the tested staged tree** (`git write-tree` before export, re-checked after the checks and immediately before commit) |
| **Subject** | `feat(canonical): register measured NHL15 PS3 adapter` |
| **Scope** | **8 paths: 3 added, 5 modified, 0 deleted** — 2 649 insertions, 950 deletions |
| **Canonical suite on the staged tree** | **225 passed · 0 failed · 0 ignored** |
| **Index after** | empty |
| **Pushed?** | **No.** Not authorized, not attempted |

> Acceptance scope, as stated by Codex: C0-REG1b **within its recorded scope**. The adapter is
> partial (10 / 2 969 fields), `player.shoots` and `player.team` remain blocked, the capacities are
> directory capacities of one fixture, and no cell is game-verified. The documentation in §6 is
> **uncommitted**.

---

## 1. Preflight

| Check | Result |
|---|---|
| HEAD | `1ede5baa8ba1ab261a253c9bfdcc5e0557116e99` |
| Index | empty (0 cached paths) |
| Eight files vs [C0-REG1b §1](c0-reg1b-nhl15-ps3-adapter.md) | **8 / 8 OK** — SHA-256 of each live file equals the REG1b manifest; no drift, nothing to investigate |
| Live `Cargo.lock` / root `Cargo.toml` / `crates/tdb-canonical/Cargo.toml` | root manifest and lock carry only the pre-existing unrelated working-tree drift; the crate manifest is unchanged from HEAD. **None was staged**: the staged tree uses HEAD's manifests and lock, as REG1b recorded (its dev-dependencies were added by C0-COMMIT-REG1) |

## 2. Staging — exact membership

Each of the eight files was `git add`ed individually, in the order listed by the package.

| Status | Path |
|---|---|
| A | `defs/adapters/nhl15-ps3.toml` |
| M | `defs/adapters/nhllegacy-xbox360.toml` |
| M | `crates/tdb-canonical/src/lib.rs` |
| A | `crates/tdb-canonical/src/target_evidence.rs` |
| M | `crates/tdb-canonical/src/legacy_xbox360_tests.rs` |
| A | `crates/tdb-canonical/src/nhl15_ps3_tests.rs` |
| M | `crates/tdb-canonical/src/snapshot_tests.rs` |
| M | `crates/tdb-canonical/src/tests.rs` |

`git diff --cached --name-status`: **3 A · 5 M · 0 D = 8**. `git diff --cached --check`: clean.
`git write-tree` → `6af4f70446124acd08520ad6d2cb9f099671a502`.

## 3. Verification on the exact staged tree

`git archive 6af4f704…` extracted into `…/scratchpad/reg1b-candidate` (**1 192 files**, equal to
`git ls-tree -r` of the staged tree and to the REG1b candidate's membership). Build output in
`…/scratchpad/reg1b-target` via `CARGO_TARGET_DIR`, outside the candidate. `QA_WRITE` and
`QA_EVIDENCE_ROOT` verified unset in the shell that ran every command.

| Command | Result | Baseline (REG1b) |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **225 passed · 0 failed · 0 ignored** (doc-tests 0) | identical |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0, no warnings | identical |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty output | identical |
| Candidate membership after the checks | 1 192 files; 0 files newer than the export marker; no `target/` inside the candidate | — |
| Candidate content after the checks | every one of the 1 192 blobs re-hashed with `git hash-object` equals the staged-tree entry — **0 differing** | — |
| Staged tree after the checks | `git write-tree` → `6af4f704…`, unchanged | — |

No discrepancy arose; no assertion was touched.

## 4. Commit and preservation

Committed with `git commit -F …/scratchpad/reg1b-commit-msg.txt`. The message describes the
partial registration (8 properties / 10 of 2 969 fields, 44 / 30 / 6 block disposition), the
directly measured PS3 ROSTER directory (fixture path, version word 2 → ZlibV4 framing, directory
capacities not runtime limits), the byte-identity with the Xbox 360 NHL 15 files as an observation
that carries nothing, the target-specific semantic evidence (birth-date origin and offsets,
companion partition, main-team offset-binary encoding and absence code, `team.name` round trip),
the shared `target_evidence.rs` test mechanics with Legacy's cases and numbers unchanged, and keeps
handedness / main-team parent blocked and runtime qualification untouched.

| Check | Result |
|---|---|
| Commit tree | `6af4f704…` — **equal** to the tested staged tree |
| Parent | `1ede5baa…` |
| Index after | empty |
| `git status --porcelain` | 255 entries (164 modified, 91 untracked): exactly the **8** entries the commit resolved are gone; **0 new entries**; `crates/tdb-canonical/` and `defs/adapters/` are clean |
| Live `Cargo.lock` / root `Cargo.toml` / `CLAUDE.md` / `README.md` | still `M` with their pre-existing unrelated drift, byte-unchanged by this package |
| Unrelated modified/untracked content | untouched — no checkout, reset, stash, clean or branch change was run |
| Push | not attempted |

## 5. Remaining blockers (unchanged by this commit)

| Blocker | Status |
|---|---|
| `player.team` parent table (`kOtt.vfEq`) on NHL 15 PS3 and Legacy | candidate; containment in 246 keys + name is not a foreign key; mapping blocked |
| `player.shoots` stored codes on NHL 15 PS3 and Legacy | candidate; the enum file is anchored on another title; counts recorded as counts only |
| 2 959 of 2 969 NHL 15 PS3 fields (2 955 of 2 965 Legacy) | unmapped, recorded as unknown |
| Runtime capacity ceilings | fixture directory capacities only; no runtime limit demonstrated on any target |
| NHL 15 Xbox 360 / NHL 10 roster-save capacities | untested until their own fixtures are measured (no proxy; the PS3 ↔ Xbox 360 main-DB identity carries nothing) |
| Registry population | **4 / 28 database units committed**, 1 / 14 asset families — deliverable not met |
| Game verification | no cell anywhere |

## 6. Documentation — uncommitted

Outside the commit, as instructed. **Uncommitted after this package:**

- `docs/receipts/c0-commit-reg1b.md` (this), `c0-reg1b-nhl15-ps3-adapter.md`, and every earlier
  receipt under `docs/receipts/`, which remains an untracked directory;
- `docs/plans/project-completion-roadmap.md` — C0 item 7 and the registry note now report **4/28
  units committed** (`58d1bc0`), partial coverage and the two blocked mappings retained;
- `docs/STATE-OF-THE-REPO.md` — §7 row likewise;
- everything else in the working tree that was already modified or untracked before this package.

## 7. Limits honoured

No new adapter, feature work, headmesh work, external-repository access, game installation,
evidence activation/publication, capability promotion or cleanup. No push. Nothing beyond the eight
authorized paths was staged or committed; no manifest or lock was touched. Stopping for Codex review.
