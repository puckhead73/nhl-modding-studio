# C0.4a — QA evidence identity and merge contract

> **Package:** C0.4a (roadmap [C0](../plans/project-completion-roadmap.md) item 4).
> **Deliverable:** [docs/plans/qa-evidence-contract.md](../plans/qa-evidence-contract.md).
> **Type:** documentation and inspection only. No generator was run, no fixture accessed, no source
> changed, no evidence file rewritten, no support status promoted. **Nothing staged, committed or
> pushed.**
> **Date:** 2026-09-07. **Revision R1** 2026-09-07 — the contract was reviewed as not yet
> implementable; §§8–11 below record what R1 changed. **Author role:** developer.
> Reviewer: Codex (architect).
> **Inputs:** completion roadmap C0 / §5, [C0.3](c0-03-guidance-reconciliation.md),
> [C0.2](c0-02-baseline-execution.md), `crates/tdb-qa/src/{catalog,matrix,lib}.rs` and
> `src/bin/qa_inventory.rs`, `crates/tdb-qa/tests/inventory.rs`,
> `crates/tdb-headmesh/tests/matrix_guard.rs` and `src/matrix.rs`,
> [docs/audits/qa-ledger.md](../audits/qa-ledger.md), and every CSV under `docs/audits/data/`.

---

## 1. C0.3 corrections completed first

Both were applied to the documents and recorded in the C0.3 receipt (new section 9, "R1
corrections"). Neither promoted or demoted a support status; both narrowed a claim.

### 1.1 The DTO binding evidence (CLAUDE.md)

C0.3 wrote that "the committed bindings match only because they were regenerated (C0.2 §4.7 found
all 124 byte-identical)". C0.2 §4.7 compared the bindings **inside the C0.2 working-tree snapshot**,
which by design includes modified and untracked files. It proves those 124 working-tree files were
byte-identical before and after `export_typescript`; it proves nothing about the bindings committed
at HEAD. **Verified: 10 paths under `app/src/bindings/` are modified or untracked relative to HEAD**
(`git status --porcelain app/src/bindings/`). CLAUDE.md now states exactly what was proved, about
which tree, and records the divergence.

### 1.2 Operation-specific coverage (`docs/01-scope-and-compatibility.md`)

C0.3 wrote that automated coverage "means the anchor matrices (`mutation_matrix`,
`operation_matrix`, `save_matrix`, `surface_matrix`) executed for that target" — which reads as all
four covering every 🟡 cell, and lets a main-DB result be mistaken for container coverage. Measured
from the committed CSVs:

| Matrix | Distinct units with rows |
|---|---|
| `mutation-matrix.csv` | 27 |
| `operation-matrix.csv` | 27 |
| `surface-capabilities.csv` | 27 |
| `save-matrix.csv` | **26 — no `ps3/nhl07`** |
| any matrix, for `legacyrecomp` | **0** |

The legend now scopes 🟡 to the **main-database path**, a per-matrix table states each matrix's
operation class and unit coverage, and explicit notes record that main-DB coverage implies neither
container nor packaging nor asset coverage, and that **outer-console packaging (Q04) is required and
separately unproven everywhere**, with Xbox 360 STFS recorded blocked. The research-only (🔬 PS3
NHL 07) and Recompiled (⬜, no anchor) distinctions are preserved unchanged.

---

## 2. Findings

Nine defects are catalogued as E1–E9 in the contract §1.3. The four that drove the design:

**F1 — the generated ledger has no family column.** `matrix::build_csv` iterates `FAMILIES` in its
inner loop but its header
(`game,platform,generation,fixture,status,automated_test,manual_case,last_verified,evidence,blocker`)
never emits it. 392 rows = 28 units × 14 families, so a row's family is its **ordinal position**
within its unit block. `tests/inventory.rs` pins that header string, so the omission is currently
load-bearing. Any family insertion or reorder silently re-labels every later row.

**F2 — a status is attributed to a gate that never covered the unit.** PS3 NHL 07 `save-raw-db`
carries `status = edit-roundtrip-pass`, `automated_test = studio_domain::save_matrix::save_matrix_over_all_fixtures`,
`last_verified = 2026-08-02` — while `save-matrix.csv` contains **no PS3 NHL 07 rows at all**. The
status comes from `matrix.rs` lines 170–182, which key on `p.has_main_db` (a file existing on disk),
not on any observed case. This is the concrete instance of "fixture presence cannot refresh a
verification date", and it is the acceptance case A9.

**F3 — verification dates are compile-time constants.** `PHASE0_DATE`/`PHASE3_DATE`/`PHASE4_DATE`
(`matrix.rs` lines 28–33) are re-emitted on every generator run regardless of whether any test ran.
The contract deletes them and replaces them with run receipts that record what actually executed.

**F4 — manual evidence has no home and a rival home.** The generator can never assign
`game-verified`, rewrites `game-qa-matrix.csv` wholesale, and deletes/rebuilds `docs/audits/data/schema/`.
Manual sign-off therefore lives only in the hand-maintained `docs/audits/data/assets/<title>/<platform>/capability.csv`
tree — plus a **second, conflicting** hand-maintained file, `docs/audits/data/asset-capability-matrix.csv`,
which covers the same subject with different columns and **omits the `heads` row entirely**. Only
`tdb-headmesh`'s `matrix_guard` binds any of it to anything.

Additional: `save-outer-package` is emitted `not-applicable` for PS3 and PS4 (`matrix.rs` line 218)
although original-console packaging is **required** under Q04 — an accepted-requirement conflict
resolved in the contract by R-Q04 (reclassify to `missing-fixture` + `unresolved`, asserting no new
capability). This is a labelling conflict inside the generator, not a conflict between accepted
documents, so no dependent design was stopped.

---

## 3. Contract decisions

> **R1 note.** The decisions below are the R0 set; §8 records where R1 replaced or tightened them.

| Decision | Choice | Reason |
|---|---|---|
| Identity | `target / family / operation [ / variant ]` **+ a mandatory structured `conversion` component wherever coverage depends on the source (R1)**, with `target = <title>-<platform>` | `<title>-<platform>` is **already** `tdb_headmesh::matrix::GameKey`, so headmesh pairs join with no translation layer. Adding `operation` is what makes coverage per-operation rather than per-family. |
| Ordering | never an identity; every file key-sorted | fixes F1/E2 |
| Layers | A automated observations · B durable manual records · C derived summaries, with generators structurally unable to write B | the direct answer to F4 |
| Storage | A = CSV + JSON run receipt · B = **one TOML file per record** · C = CSV | one file per manual record makes an append a new file: no merge conflicts, no whole-file rewrite, and no path by which a generator can clobber sign-off. TOML matches `defs/`, allows comments, and `toml` is already a dependency. C stays CSV so existing consumers migrate by column name. |
| Declared vs. executed | **R1: explicit hashed `keys-declared.txt` / `keys-executed.txt` key-set files; counts are derived checks only** | fixes F2/F3 and makes the C0.2 §4.4 skip-as-pass failure representable |
| Freshness | digest over a **declared implementation scope**, not dates or commits | an unrelated commit invalidates nothing; a change inside the scope invalidates exactly the records that declared it |
| Stale evidence | retained as history, cannot qualify current output | roadmap §5 |
| Conflicts | manual pass + automated fail ⇒ `blocked` + `conflict`, both citations shown | manual success can never mask an automated failure |
| Unknowns | `"unknown"` is a legal recorded value; `unknown-applicability` is a distinct level | missing history is never invented; missing fixtures never become `not-applicable` |

---

## 4. Migration risks

| Risk | Mitigation in the contract |
|---|---|
| Migrating `game-qa-matrix.csv` by transcription would bake in the ordinal-implied families | **Superseded by R1.** Regenerating from the live catalog is now *prohibited*: the family/unit ordering is frozen and hashed with the source (R-FREEZE), and every migrated row is labelled `confidence = "positional"` because a row reordered inside its unit block is undetectable (R-POSITIONAL). |
| Fanning a family-level `edit-roundtrip-pass` out to every operation would invent evidence | **Tightened by R1.** Legacy labels are not observations at all: they migrate to non-qualifying layer-L claims with no receipts and no synthesized dates (R-CLAIM). |
| Demotions (F2's PS3 NHL 07 row, R-Q04 packaging cells) could read as capability loss | They are corrections of unsupported claims; the contract states so explicitly at each mapping |
| Coarse destination-level headmesh evidence could be read as pair-specific proof | **R1:** migrated at `granularity = "destination"` and never back-filled onto pair keys (R-NO-BACKFILL, E11) |
| A source cell could be dropped silently | **R1:** every source cell — including every `not_run` and blank — gets exactly one accounting row, and the count must reconcile exactly (R-ACCOUNT, A12) |
| Retiring `capability.csv` could break `matrix_guard` | Add the new assertion alongside the old, prove both agree on all 15 pairs (A11), retire the old in a **separate** commit |
| The duplicate asset matrix (E7) could be silently resolved by picking a winner | Both retained; divergence recorded as `unresolved = duplicate-source`; disposition is architect question Q-B |
| `install = external` has no established meaning | Preserved as `inconclusive` + unresolved; question Q-A |
| Dates embedded in free-text `evidence` strings | Migrated only where a date is literally present; otherwise `performed_utc = "unknown"` |

---

## 5. Checks run

| Check | Result |
|---|---|
| Relative links in both new documents and in the two edited documents | **All resolve** (filesystem check of every relative Markdown target) |
| Proposed identity traced through `tdb_qa::catalog::CATALOG`/`FAMILIES` | 28 units × 14 families = **392**, matching the committed row count exactly |
| Proposed `target` token traced through `tdb_headmesh::matrix::GameKey` | `GameKey::title()`/`platform()` split on the first `-`, so `nhllegacy-xbox360` and `nhl21-ps4` round-trip; `matrix_guard` builds its CSV path from exactly those two parts |
| Per-matrix unit coverage | computed from the four committed CSVs (table in §1.2) |
| PS3 NHL 07 `save-raw-db` claim | reproduced against `save-matrix.csv`: **no matching rows** |
| Capability-CSV vocabulary | `pass`/`partial`/`external`/`not_run` — disjoint from `tdb_qa::catalog::STATUS_VOCAB`, and unvalidated by any test |
| Duplicate asset matrix | `asset-capability-matrix.csv` 4 rows vs. `capability.csv` 5 rows; no `heads` row in the former |
| Producer inventory | `grep` for `QA_WRITE` and `audits/data/` across `crates/` — five emitters found; the remaining CSVs have no producer and are hand-maintained |

No build, test suite, generator, formatter or linter was run. No fixture or game file was touched.

---

## 6. Next implementation boundaries *(R0 — superseded by §10)*

> Retained for traceability. The current split is the three-stage S1/S2/S3 plan in §10, which
> defers all CI work until the command and its input contract are implemented and validated.

- **C0.4b** — implement `tdb_qa::{observe, manual, merge}` plus `qa-evidence check|build|install`,
  and the acceptance cases A1–A8, A13, A14. No migration of existing records; no removal of any
  current artifact. CI adds `check` only.
- **C0.4c** — migrate `game-qa-matrix.csv` and the asset capability CSVs (contract §4), land the
  dual `matrix_guard` assertion (A11), reconcile record counts (A12), and correct
  `qa-ledger.md` line 10. Retiring any current artifact is a separate, reviewed commit.
- **Not in either:** any QA UI, any change to Studio's Project schema, and any promotion of support.

---

## 7. C0.3 commit-boundary inventory

**Reference:** the preserved C0.2 snapshot at
`H:\c0-baseline\c0-02-20260907-085022\source`, **read only** — it captures the working tree as it
stood *before* C0.3, so `HEAD → snapshot` isolates pre-existing edits and `snapshot → worktree`
isolates C0.3/C0.4a edits. Line counts below ignore CR so line-ending differences do not distort them.

### 7.1 Tracked files

| File | Pre-existing (HEAD→snap) | C0.3/C0.4a (snap→WT) | Staging disposition |
|---|---:|---:|---|
| `CLAUDE.md` | 0 | 210 | safe to stage whole |
| `README.md` | 0 | 44 | safe to stage whole |
| `docs/DESIGN.md` | 0 | 50 | safe to stage whole |
| `docs/01-scope-and-compatibility.md` | 0 | 104 | safe to stage whole |
| `docs/00-vision.md` | 0 | 5 | safe to stage whole |
| `docs/03-tdb-format.md` | 0 | 11 | safe to stage whole |
| `docs/04-source-database.md` | 0 | 2 | safe to stage whole |
| `docs/05-save-data.md` | 0 | 2 | safe to stage whole |
| `docs/glossary.md` | 0 | 3 | safe to stage whole |
| `docs/EDIT-MODEL.md` | 0 | 12 | safe to stage whole |
| **`docs/ROADMAP.md`** | **8** | **8** | ⚠ **MIXED — do not stage whole** |
| **`docs/plans/phase-6-asset-tools.md`** | **13** | **8** | ⚠ **MIXED — do not stage whole** |
| `docs/audits/README.md` | 4 | 0 | **pre-existing only — not C0.3's to commit** |
| `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` | 1 | 0 | **pre-existing only** |
| `docs/plans/full-game-qa-editability.md` | 21 | 0 | **pre-existing only** |
| `docs/plans/phase-6.1-portraits-logos-spikes.md` | 44 | 0 | **pre-existing only** |

Everything else modified in the working tree (60 tracked source files, `Cargo.toml`, `Cargo.lock`,
`app/**`) is pre-existing and untouched by C0.3/C0.4a.

### 7.2 The two mixed files, at hunk level

**`docs/plans/phase-6-asset-tools.md` — separable.** Two disjoint regions:
- pre-existing: the **P-S2 portrait slug/matcher rewrite** at roughly lines 37–52;
- C0.3: the **sequencing-update banner** at lines 2–15.
They share no context lines, so `git add -p` can take the banner hunk alone.

**`docs/ROADMAP.md` — NOT separable by hunk.** The pre-existing edit added the 2026-09-05 completion
banner (lines 3–10). C0.3 then **rewrote the last line of that same block** into seven lines. Both
changes land in one hunk against HEAD, so hunk-level staging cannot split them; separating them
needs line-level editing (`git add -e`) or a decision to commit the banner as a single unit.
**Architect decision required** before either file is committed.

### 7.3 Untracked files (no HEAD side; all-or-nothing)

| Path | Origin |
|---|---|
| `docs/STATE-OF-THE-REPO.md` | pre-existing file, **C0.3 edited it** (26 lines) — the whole file is new to git |
| `docs/plans/project-completion-roadmap.md`, `completion-design-questions.md` | pre-existing |
| `docs/audits/headmesh-uv-compat.md`, `legacy-head-donor-census.md`, `nhl21-ps4-head-asset-binding.md`, `nhl24-legacy-patha-roundtrip.md` | pre-existing |
| `docs/validation/c0-03-guidance-reconciliation.md` | C0.3 |
| `docs/plans/qa-evidence-contract.md`, `docs/validation/c0-04a-evidence-contract.md` | C0.4a |

### 7.4 Line endings — recorded, no action needed

C0.3 rewrote several files with LF where the snapshot had CRLF (`CLAUDE.md`, `docs/DESIGN.md`,
`docs/EDIT-MODEL.md`, `docs/ROADMAP.md`, `docs/plans/phase-6-asset-tools.md`, and others).
`core.autocrlf = true`, so git normalises to LF in the index: `git diff --numstat` shows **only real
content changes** and no line-ending churn. No repair is required, and none was attempted. The
counts in §7.1 are CR-insensitive for the same reason.

---

## 8. R1 — contradictions found and how they were resolved

The R0 contract was not implementable: it specified mutually exclusive behaviours in seven places.
Each is now resolved in the contract by a named rule.

| # | R0 contradiction | R1 resolution |
|---|---|---|
| **X1** | Runs overwrote `<emitter>.csv` and its receipt, yet the contract claimed earlier evidence was preserved. Membership was carried by *counts*, which cannot prove which keys ran. | Immutable per-run directories under `auto/runs/<run_id>/`; explicit `keys-declared.txt` / `keys-executed.txt` are the membership evidence and are hashed in the manifest; counts are demoted to derived checks (V4). Run states `completed / failed / interrupted`, with a **missing `state.json` meaning `interrupted`** (R-STATE). A missing emitter erases nothing and implies no observation (R-ABSENT). |
| **X2** | "the newest non-superseded record qualifies" — an implicit recency rule that silently discards disagreement. | Selection is explicit and recency-free: active = completed and not superseded. Disagreement between active runs or active records is **retained with citations** and yields `withheld` plus a `conflict` token (R-NOWIN, §4.2). Supersession cycles, non-superset supersession and duplicate ids are hard errors (V7, V8, V2). |
| **X3** | Dangling references were "hard errors" **and** orphan observations were "silently ignored". | Split by input class: **active** inputs (`auto/runs/`, `manual/`, published C) fail hard (R-ACTIVE-STRICT); **legacy and candidate** inputs are not active and carry explicit `unresolved` tokens instead (R-IMPORT-SOFT). An orphan reference in an active run is now V8, never ignored. |
| **X4** | Conversion coverage collapsed: all 15 headmesh pairs shared one `<destination>/assets-heads/install` key, so one pair's proof would have qualified the rest. | A structured `conversion` component (`from`, `class`, `path`, `recipe`) is **mandatory** wherever coverage depends on the source (R-CONV), so each pair has a distinct key. `nhl24` is a conversion source that is never a coverage target (R-SOURCE-ONLY). Case ids are separate from coverage keys, and uniqueness is `(run_id, case_id, key)`, so independent cases on one operation coexist (R-MULTI). |
| **X5** | One ordered `level` scale conflated scope, implementation, observation and qualification, and roll-up was "the minimum" over values that are not ordered. | Four independent axes (`applicability`, `implementation`, `automated`, `manual`) with `qualification` **derived** by an explicit 9-row decision table, and roll-up by its own explicit 9-row table (§4.3, §4.4). Game acceptance is its own operation with its own key and required identity fields; a manual pass on preview/export/install never yields `game-verified` (R-GAME, R-IDENTITY). Stale failures are distinguished from current failures (R-STALE-FAIL). |
| **X6** | Migration "re-keyed by regenerating from the same catalog", fabricated `automated-tested` levels from legacy labels, and would have collapsed `XPR2/RX2` and invented dates. | Inputs are frozen with digests **including the family/unit ordering** used to interpret them, and regenerating from a changed catalog is refused (R-FREEZE, A25). Legacy labels become non-qualifying **layer-L claims** with no receipts and no synthesized timestamps (R-CLAIM). Positional family assignment is labelled as such, and content-contradicting rows go to `_unmapped/` (R-POSITIONAL). Raw formats, `external`, `not_run` and date precision are preserved verbatim (§6.4). Every source cell gets an accounting row (R-ACCOUNT). |
| **X7** | Recovery was "`git checkout` the single installed path", and publication was several independent renames. | A journalled multi-phase publication with a retained prior generation and a deterministic `publish --recover` per phase (§8.2). **`git checkout` is explicitly prohibited** as recovery (R-NO-GIT), because this tree carries dozens of unrelated working changes. Input digests are re-verified immediately before anything moves (A30). |

Three further defects were found during the R1 review and added to the contract's §1.3:

- **E10** — every emitter overwrites its single CSV, so a non-run and a wiped run are indistinguishable.
- **E11** — `capability.csv` holds **one `heads` row per destination** while `headmesh-matrix.toml`
  defines **15 pairs across 12 distinct destinations**; the coarse row cannot express pair-specific
  proof, yet `matrix_guard` gates pair promotion on it. Migration therefore records it at
  `granularity = "destination"` and must never back-fill it onto pair keys (R-NO-BACKFILL).
- **E12** — `qa_inventory` deletes and rebuilds `docs/audits/data/schema/` with no journal and no
  retained copy. Recorded, and explicitly **not** fixed by C0.4b (R-TRANSITION).

---

## 9. Q-A – Q-E, closed

| # | Decision |
|---|---|
| Q-A | `install = external`: preserve the raw value and column context; **no game-install qualification**; no meaning invented. |
| Q-B | Duplicate asset matrices: preserve both, freeze both, publish their differences; **no retirement in C0.4**; neither wins by default. |
| Q-C | `QA_WRITE=1` emitters: transition to explicit output/install in **bounded later packages**; C0.4b only adds a parallel run-directory write. |
| Q-D | Freshness scopes: the contract now **proposes** versioned operation/variant dependency definitions for the first emitters, with shared dependencies (`defs/qa/scopes.toml`). This is implementation work, **not** a user decision. |
| Q-E | `qa-ledger.md` line 10: correct during **C0.4c**. |

---

## 10. Bounded implementation split

| Stage | Contents | CI |
|---|---|---|
| **S1 — pure core** | key parse/serialize, schema types, validation V1–V12, selection, the two decision tables, freshness digests. Pure functions; no writing, no CLI. | none |
| **S2 — emitters + publication** | `observe` run writer, `check` / `build` / `publish` / `publish --recover`, provenance sidecars, journal. No existing write path removed. | a gate may be considered **only after** this lands and `check` has run against real inputs |
| **S3 — migration** | freeze, `migrate`, accounting, `migrate-install`, dual headmesh assertion, `qa-ledger.md` correction. | none |

---

## 11. R1 checks

| Check | Result |
|---|---|
| All 12 fenced examples parsed with `json` / `tomllib` / `csv` | **all parse**; CSV row widths match their headers |
| All 5 coverage keys in the document parsed against the contract's own ABNF | **all valid**; every `family` part is in `tdb_qa::catalog::FAMILIES` |
| All 14 hash-shaped literals | **all exactly 64 hex characters** |
| Internal consistency sweep | fixed two self-contradictions the checks exposed: an example observation carried a key the same example declared out of scope (violating the contract's own V6), and the synthetic run counts implied whole-catalog coverage for a save-container emitter |
| Cross-reference consistency | the layer-C example cites the layer-L example's `claim_id`; manifest and state `run_id` values match; the run-id format matches its stated construction |
| Document links | all relative links in both documents resolve |

No build, test suite, generator, migration or fixture access was involved.

---

## 12. Open issues

| # | Issue | Owner |
|---|---|---|
| **U1** | **Unresolved architectural ambiguity:** the required-operation table (contract §4.1) makes `game-acceptance` required for every `assets-*` family on every target. Under Q02 that is consistent with "no release until complete", but it leaves asset-family roll-ups permanently `unqualified` until manual campaigns run. Confirm this is intended reporting rather than a definition to soften. |
| **U2** | The `conversion.class` vocabulary (`head-authored`, …) is proposed, not derived from an existing registry. It should ultimately come from the same source as `headmesh-matrix.toml`'s path and id schemes. |
| **U3** | Whether layer-A run directories are committed to git or retained out of tree. Committing gives durable, reviewable history; it also grows `docs/` on every run. Not decided here. |
| — | Staging disposition of the two mixed files (§7.2), especially `ROADMAP.md`, whose pre-existing and C0.3 edits share one hunk | architect |
| — | C0.2 defects B1–B5, B7 and the Windows-CI gap (D11) remain open and unfixed | C0.2 R1–R6 |
