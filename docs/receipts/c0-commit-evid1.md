# C0-COMMIT-EVID1 — commit the verified evidence core

**Package:** C0-COMMIT-EVID1 · **Date:** 2026-09-13 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling`

| | |
|---|---|
| **Result** | **Committed.** All four gates passed on the exact staged tree, exported and run in isolation |
| **Commit** | `819940a9fcf255221ce64bb6b0b2229942ec304a` |
| **Parent** | `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` (HEAD at start, as authorized) |
| **Commit tree** | `72c6da9e5e65e87cb901100be1c6084e4db271fe` — **equal to the verified staged tree** (`git write-tree` before export, re-checked immediately before commit) |
| **Subject** | `feat(qa): integrate evidence core and migration safeguards` |
| **Scope** | **36 paths: 26 added, 10 modified, 0 deleted** — 16 155 insertions, 96 deletions |
| **QA suite on the staged tree** | **228 passed · 0 failed · 1 ignored** |
| **Canonical suite on the staged tree** | **200 passed · 0 failed** |
| **Index after** | empty |
| **Pushed?** | **No.** Not authorized, not attempted |

> Acceptance scope, as stated by Codex: C0-NEXT1's isolated integration boundary and
> C0-MIG-ID1/R1's conversion-identity correction, **within their demonstrated scope**. This commit
> does **not** constitute live activation, real-registry readiness, C0 closure, or blanket acceptance
> of any earlier package. The documentation corrections in §6 are **uncommitted**.

---

## 1. Preflight

| Check | Result |
|---|---|
| HEAD | `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` |
| Index | empty (`git diff --cached --stat` → 0 bytes) |
| `git status --porcelain` | 269 entries (identical to C0-NEXT1's end state) |
| 35 slice files vs C0-NEXT1 `…/n1/overlay-manifest-before.txt` | **0 drift entries** — every path present at its recorded SHA-256; expanded manifest retained at `…/scratchpad/ce1/slice-manifest.txt` |
| Concurrent work | none touching the slice; the live `Cargo.lock` (`91b22103…`) still carries the unrelated `tdb-rx2`/headmesh drift and was **not** used as the staged lock |

## 2. Staging — exact membership

Each of the 35 files was `git add`ed individually from the manifest. The lock was constructed
**outside the working tree**: `git show HEAD:Cargo.lock` → one line inserted →
`git hash-object -w` → `git update-index --cacheinfo 100644,fe6cae30…,Cargo.lock`. The live
`Cargo.lock` was never read for content and is byte-unchanged (`91b22103…` before and after).

| Group | Paths | Op |
|---|---|---|
| Lock | `Cargo.lock` | M — **one line**: `tdb-qa` dependencies gain `"toml 0.8.2"` (`git diff --cached -- Cargo.lock`: `+ "toml 0.8.2",` at line 3915, nothing else) |
| Crate manifest | `crates/tdb-qa/Cargo.toml` | M |
| Build script | `crates/tdb-qa/build.rs` | A |
| Binaries | `crates/tdb-qa/src/bin/qa_evidence.rs` (A), `crates/tdb-qa/src/bin/qa_inventory.rs` (M) | |
| Library | `crates/tdb-qa/src/{lib,catalog,diff,manifest,matrix,schema}.rs` | M ×6 |
| Evidence module | `crates/tdb-qa/src/evidence/{csv,emit,journal,key,merge,migrate,mod,model,observe,paths,publish,scopes,store,testkit,validate,vocab}.rs` | A ×16 |
| Tests | `crates/tdb-qa/tests/evidence_{acceptance,discovery,emit,file_reads,journal,migration,publication}.rs` (A ×7), `crates/tdb-qa/tests/inventory.rs` (M) | |
| Scope definitions | `defs/qa/scopes.toml` | A |

`git diff --cached --name-status`: **26 A · 10 M · 0 D = 36**. Root `Cargo.toml` **not staged**.
`git diff --cached --check`: clean. Git reported the expected `LF will be replaced by CRLF` notices
for every text file (`core.autocrlf=true`); blobs are stored LF-normalised, matching HEAD's
convention.

## 3. Verification on the exact staged tree

`git archive --format=tar 72c6da9e…` extracted into `…/scratchpad/ce1/candidate` (1186 files; a
CRLF checkout representation, as `git archive` applies the autocrlf filter). Build output in
`…/ce1/target`, outside the candidate. `QA_WRITE` and `QA_EVIDENCE_ROOT` unset.
`defs/appearance/headmesh-matrix.toml` absent from the candidate (confirmed), so the real-registry
case could not have read the live registry.

| Command | Result | Baseline (C0-NEXT1) |
|---|---|---|
| `cargo test -p tdb-qa --locked --offline` | **228 passed · 0 failed · 1 ignored** (72 unit; 32 acceptance; 13 discovery; 7 emit; 9 file-reads; 31 journal; 19 migration + **1 ignored**; 36 publication; 9 inventory; 0 doc) | identical |
| `cargo clippy -p tdb-qa --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings/errors | identical |
| `cargo fmt -p tdb-qa -- --check` | exit 0, empty output | identical |
| `cargo test -p tdb-canonical --locked --offline` | **200 passed · 0 failed** | identical |
| Candidate membership (SHA-256 manifest before/after) | 1186 files, **0 differing entries** | — |

No discrepancy arose, so nothing was investigated and no assertion was touched. The determinism
demonstration of C0-NEXT1 §3.1 was **not** rerun: the staged tree's `tdb-qa` sources are
byte-identical to the candidate it ran on, and no relevant difference exists.

**Ignored / unexecuted coverage, stated as gaps:**
- `real_headmesh_registry_integration_over_the_committed_registry` — ignored, prerequisite named
  in its reason; the registry is not committed.
- `headmesh_pair_keys` has **no production caller**; nothing at HEAD enumerates pair rows.
- The `studio-domain` emitter integration (`observe` calls in the four matrix test binaries) is
  **not in this commit**; nothing at HEAD captures a layer-A run.
- Private-fixture and mounted-game-tree cases in other crates were not run.

## 4. Commit and preservation

Committed with `git commit -F` from a message file (`…/ce1/commit-msg.txt`). Verified afterwards:

| Check | Result |
|---|---|
| `git write-tree` immediately before commit | `72c6da9e…` — unchanged since export |
| Commit tree | `72c6da9e…` — **equal** |
| Index after | empty |
| `git status --porcelain` | 249 entries: exactly the **20** entries the commit resolved are gone (9 modified `tdb-qa` paths, the `crates/tdb-qa/src/evidence/` directory entry, 7 test files, `build.rs`, `qa_evidence.rs`, `defs/qa/`); **0 new entries** |
| Slice files on disk | all 35 byte-identical to the preflight manifest |
| Live `Cargo.lock` / root `Cargo.toml` | byte-unchanged (`91b22103…`, `5272dcbd…`); both still show `M` because the working tree carries the unrelated headmesh/`tdb-rx2`/`studio-domain` changes, which are **not** part of this commit |
| Unrelated modified/untracked content | untouched — no checkout, reset, stash, clean or branch change was run |
| Retained prior candidates and logs | read only; nothing deleted |

## 5. Remaining activation blockers (unchanged by this commit)

| Blocker | Owner |
|---|---|
| **Observation capture** — the `studio-domain` emitter wiring is uncommitted; until it lands with its own verified boundary, HEAD produces no layer-A runs | next evidence slice (developer), acceptance (architect) |
| **U3 / D2** — evidence-tree location; layer-A retention or recapture mechanism; selected-run identity per published generation; clean-checkout CI proof | architect |
| **D1** ownership (FR12) · **D7** `uniforms` claim ownership · **D8** U2 class/recipe vocabulary · **D11** activation sequencing | architect |
| **Registry** — `defs/appearance/headmesh-matrix.toml` uncommitted; real-registry case ignored; pair rows cannot be enumerated | headmesh integration (deferred, external) |
| **FR6** — runtime identity: game build CUSA11126 / APP_VER 01.60 is recorded for NHL 19 PS4, the **shadPS4 executable version is not** | register update before any layer-B record |
| Repo-wide `fmt`/`clippy` at HEAD (C0.2, B-series) and Windows CI (R6) | separate track |

## 6. Documentation corrections — uncommitted

Applied outside the commit, as instructed; all four files remain uncommitted:

| File | Correction |
|---|---|
| `docs/receipts/c0-next1-evidence-activation.md` | new **§8** (labelled) correcting §4: (1) pair rows — no production caller, absent registry cannot enumerate; family-level vs pair-specific coverage distinguished; withheld pair rows are **planned**, not demonstrated; (2) layer-A-outside-git is a **proposal** — activation still needs immutable retention/retrieval or reproducible recapture, selected-run identity and clean-checkout CI proof, and committed B/L/C alone do not make `check` reproducible; (3) `CUSA11126 / APP_VER 01.60` is the **game build**, not the emulator version; (4) `819940a` commits the evidence core, **not** the `studio-domain` emitter integration — observation capture is not wired at HEAD |
| `docs/plans/project-completion-roadmap.md` | C0 status: item 4 committed cell → "evidence core: yes — `819940a`; emitter wiring not committed"; the "what is committed" paragraph updated accordingly. C5: game-build vs emulator wording; FR6 stated open |
| `docs/STATE-OF-THE-REPO.md` | §7 rows for the evidence system and the NHL 19 imports corrected the same way |
| `docs/receipts/c0-commit-evid1.md` | this receipt |

## 7. Limits honoured

No live migration, publication, generator run, game installation, capability promotion, headmesh
work, external-repository access, broad cleanup or unrelated repair. No push. Nothing beyond the 36
authorized paths was staged or committed. Stopping for Codex review; **C0-REG1a** (the
`nhllegacy-xbox360` adapter from its own evidence only) is the next package and was **not** started.
