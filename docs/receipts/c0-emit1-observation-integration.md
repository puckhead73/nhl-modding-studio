# C0-EMIT1 — isolate and verify matrix observation capture

**Package:** C0-EMIT1 · **Date:** 2026-09-15 (UTC; 2026-09-14 local) · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **Baseline and HEAD throughout:** `58d1bc073bbf9e80d5afb4782bc6c132725e5fed` ·
**Index before and after:** empty · **Nothing staged, committed or pushed.**

> **Result.** The four `studio-domain` matrix emitters' observation wiring has a concrete,
> reproducible boundary over HEAD: **5 source/configuration files + 1 lock line**, no manifest
> change beyond the crate's own `[dev-dependencies]`, and **no prerequisite repair** — every API the
> wiring calls (`Emitter`, `observe_faceted`, `observe_unmapped`, `csv::compare_records`) is already
> committed at `819940a`. The four emitter files are byte-identical to the versions
> C0-EVID-REPAIR1 §2 verified on 2026-09-09. Two further items were **added by this package** and
> are listed separately so they can be struck: a freshness-scope widening (`db-edit` and
> `save-containers` → v3, §4.3) and two focused lifecycle cases (§4.5).
>
> On an isolated candidate (HEAD archive + overlay, external build output, `QA_WRITE` and
> `QA_EVIDENCE_ROOT` unset): all four emitters **pass in disabled mode and create no file**. With
> `QA_EVIDENCE_ROOT` pointing at a disposable root: **4 completed runs, 952 observations, 189
> unmapped records, 1 452 facets**; `build` twice → **byte-identical**; `publish` into a disposable
> data directory → `check` **`ok: 2175 row(s), no validation failure, no drift`**. The merge yields
> **140 `automated-qualified` · 159 `withheld` (all `unknown-applicability`, the conservative
> variant rule) · 36 `not-applicable` · 1 840 `unqualified` · 0 `game-verified`**. Emitter
> completion is not fixture coverage: **28 `skipped-missing-fixture`** observations and **121
> `declared-not-executed`** rows are reported separately in §3.4.
>
> `tdb-qa`: **230 passed · 0 failed · 1 ignored** (the uncommitted-registry case, unchanged).
> Clippy on the emitter test targets: zero diagnostics from the overlay; the 8 `studio-domain` lib
> diagnostics and the `tdb-rx2`/`tdb-archive-codec` ones are HEAD code (C0.2 baseline, §6.2).

---

## 1. The boundary

### 1.1 Overlay — exact files and hashes

SHA-256 of each file **as placed in the candidate** (CRLF, the representation `git archive`
renders under `core.autocrlf=true`; the five live emitter/manifest files are already CRLF on disk, the
two package-added files are LF live and were converted). `git hash-object` of every overlay file
equals the corresponding live file's blob, so the overlay is exactly what a commit would stage.

**A. The implemented wiring (reused, byte-identical to C0-EVID-REPAIR1 §2's manifest)**

| Path | HEAD → overlay | SHA-256 (candidate) |
|---|---|---|
| `crates/studio-domain/Cargo.toml` | M, **+3 lines**: `tdb-qa = { path = "../tdb-qa" }` under `[dev-dependencies]` with its comment | `3b28e92b9267ace885917c389613db5e7a5f957cb2683e51495e38f182e210e8` |
| `crates/studio-domain/tests/surface_matrix.rs` | M (+130 / −21 vs HEAD) | `4f7fbf1e7a254f7ca6a7ee2c982290a0a967606094fe99e29ae3d45dc2a83b7a` |
| `crates/studio-domain/tests/mutation_matrix.rs` | M (+429 / −112) | `60952ea3df98fc1f2b84ebd654442ef825aecd56a4bbed8e183898cfd4c3de72` |
| `crates/studio-domain/tests/operation_matrix.rs` | M (+473 / −103) | `60bb736d92d9eb0a09affe06f3d6110cc703e2b463d3288160a3524ef0405d5c` |
| `crates/studio-domain/tests/save_matrix.rs` | M (+595 / −107) | `8a0896ed4edb686ee6e960027cafce3d21a0e3a1d0a823c530763d57d4c3fe89` |
| `Cargo.lock` | M, **exactly one line**: `+ "tdb-qa",` in the `studio-domain` dependency list (constructed from `HEAD:Cargo.lock`, not from the live lock) | `4a4cc094aa6f277601e293f89247a61c4270a6171fee93d62c5dc3da10810747` |

The live root `Cargo.toml` (+2 headmesh workspace members) and the rest of the live lock delta
(`blender-job`, `tdb-headmesh`, `zune-*`, `tdb-rx2 += sha2`, `image += zune-jpeg`) are **feature-side
drift and were not copied**. The candidate uses HEAD's root manifest.

**B. Added by this package (§4.3, §4.5) — separable**

| Path | Change | SHA-256 (candidate CRLF / live LF) |
|---|---|---|
| `defs/qa/scopes.toml` | `db-edit` v2 → **v3** (+`crates/tdb-enums/src/**`, +`crates/studio-domain/src/team_tabs.rs`); `save-containers` v2 → **v3** (+`crates/tdb-enums/src/**`); `db-read` and `shared-core` unchanged | `78283e3786859e64…` / `3672d2e0dd815d43…` |
| `crates/tdb-qa/tests/evidence_emit.rs` | **+2 cases** (§4.5); no existing case changed | `61413ea7bbb4d186…` / `ed0cfc466d46b051…` |

Nothing else. `crates/tdb-qa/**` and `defs/qa/**` were otherwise clean against HEAD before this
package (verified with `git status --porcelain`), so the evidence core is consumed as committed.

### 1.2 Separating wiring from formatting and lint edits

Each emitter's HEAD → working-tree delta was decomposed mechanically: `rustfmt --edition 2021`
was applied to HEAD's copy, and the result diffed against the working-tree file. What survives that
diff is the non-formatting content.

| File | HEAD → `rustfmt(HEAD)` | `rustfmt(HEAD)` → live (the real delta) | Non-wiring content in the real delta |
|---|---|---|---|
| `surface_matrix.rs` | 14 lines | 116 lines | none |
| `mutation_matrix.rs` | 280 lines | 171 lines | `bit_width % 8 == 0` → `is_multiple_of(8)`; `repeat('Q').take(n)` → `repeat_n` ×2 (`manual_is_multiple_of`, `manual_repeat_n`) |
| `operation_matrix.rs` | 252 lines | 225 lines | `(expected && a) \|\| !expected` → `!expected \|\| a` ×12 (`nonminimal_bool`; logically identical) |
| `save_matrix.rs` | 279 lines | 370 lines | none — the split of `container_roundtrip` into extract/repack proofs is part of the wiring (R1 F4: attribution to the operation whose assertions ran) |

All four live files are `rustfmt --check` clean. The three lint edits are not observation wiring,
but they are what lets the four test targets lint clean under `-D warnings` on rustc 1.96
(`manual_is_multiple_of` is warn-by-default there — the same lint fires on HEAD's `assets.rs` and
`view.rs`, §6.2). They are carried in the overlay as part of the REPAIR1-verified files rather than
reverted; separating them would produce files no earlier package verified.

The wiring itself, per emitter: `Emitter::from_env` + whole-catalog declaration before the anchor
loop; `Emitter::case` around each asserting block; `observe`/`observe_hashed`/`observe_faceted`/
`observe_unmapped` **after** the assertions; the drift assertion replaced by `compare_records`
with the panic deliberately outside `finish()`; `ev.finish()` last. Mappings are those of
C0-S2-INT1 §2 as corrected by R1 (unmapped operations recorded, variant keys and facets, conservative
parent cells).

### 1.3 Prerequisite repairs

**None required.** The two candidates were checked and excluded:

* the CRLF drift failure C0-EVID-PROBE1 §4.2 recorded for a clean checkout is closed at HEAD by
  `evidence::csv::compare_records` (committed at `819940a`), which the overlay's emitters call —
  all four pass on the CRLF candidate (§3.1);
* `defs/appearance/headmesh-matrix.toml` remains uncommitted, but the only case needing it is
  `#[ignore]`d with its prerequisite named (R1 of REPAIR1) and is not on this slice's path.

## 2. The candidate

| | |
|---|---|
| Base | `git archive 58d1bc07…` → `…/scratchpad/emit1/candidate`, **1 192 files** (CRLF representation) |
| Overlay | §1.1 A placed by explicit copy; lock patched by one insertion; then §1.1 B. Membership unchanged: **1 192 files** |
| Build output | `CARGO_TARGET_DIR=…/scratchpad/emit1/target-candidate`, dedicated to this tree (PROBE1 §4.5's shared-target hazard avoided by construction) |
| Environment | `QA_WRITE` and `QA_EVIDENCE_ROOT` verified unset in the shell of every disabled-mode and `tdb-qa` command; `QA_EVIDENCE_ROOT` set only for §3.2, to `…/scratchpad/emit1/evroot` (outside the repository and the candidate) |
| `--locked --offline` | accepted the one-line lock on every command — the patched lock is exactly what cargo would generate |
| Preservation manifests | SHA-256 of all 1 192 files taken before execution, after the disabled runs, before capture and after everything (§6.3) |

## 3. Executed checks and outcomes

### 3.1 Disabled mode (`QA_EVIDENCE_ROOT` unset, `QA_WRITE` unset)

`cargo test -p studio-domain --test <emitter> --locked --offline` in the candidate:

| Emitter | Result | Runtime |
|---|---|---|
| `surface_matrix` | **ok, 1 passed** | 7.3 s |
| `mutation_matrix` | **ok, 1 passed** | 159.6 s |
| `operation_matrix` | **ok, 1 passed** | 458.5 s |
| `save_matrix` | **ok, 1 passed** | 75.9 s |

The existing assertions executed (record-level drift comparison against the committed CSVs; the
27-anchor, `>100` fixture and round-trip counts). **No file was created**: the candidate manifest
after the four runs is identical to the one before (0 differing entries of 1 192); no `auto/`,
`in-progress/` or `docs/audits/evidence` directory exists anywhere under the candidate or the
scratchpad; the four committed CSVs in the candidate are byte-unchanged.

### 3.2 Enabled mode (`QA_EVIDENCE_ROOT=…/emit1/evroot`, `QA_WRITE` unset)

| Emitter | Result | Runtime | `run_id` | Scope · `implementation_digest` |
|---|---|---|---|---|
| `surface_matrix` | ok, sealed **`completed`** | 6.7 s | `surface_matrix-20260915t013547z-25dd5e66` | `db-read` **v2** · `sha256:cee54b96e299830b…` |
| `mutation_matrix` | ok, sealed **`completed`** | 121.1 s | `mutation_matrix-20260915t013554z-949e1174` | `db-edit` **v3** · `sha256:f92bd43b91dc9c6d…` |
| `operation_matrix` | ok, sealed **`completed`** | 456.9 s | `operation_matrix-20260915t013756z-427feffd` | `db-edit` **v3** · same digest |
| `save_matrix` | ok, sealed **`completed`** | 102.8 s | `save_matrix-20260915t014533z-8823e3ea` | `save-containers` **v3** · `sha256:b8ab5bbb8c7625f9…` |

Toolchain on every run: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `host_os = windows`. No journal
remained under `auto/in-progress/` after the four seals.

**What the observations identify** (checked on the rows, not inferred):

| Run | Declared | Observed keys | Executed keys | Observations by outcome | Input digests | Unmapped | Facets |
|---|---:|---:|---:|---|---:|---:|---:|
| `surface_matrix` | 28 | 28 | 27 | 27 `pass` · 1 `skipped-missing-fixture` | — (fixture digest per row) | 0 | 0 |
| `mutation_matrix` | 28 | 28 | 27 | 147 `pass` · 27 `refused-clean` · 1 skipped | 27 anchor paths → SHA-256 | 0 | 0 |
| `operation_matrix` | 140 | 140 | 119 | 271 `pass` · 10 `refused-clean` · 21 skipped | 27 | **189** | 0 |
| `save_matrix` | 243 | 243 | 237 | 190 `pass` · 74 `refused-clean` · 177 `not-attempted` · 6 skipped | — | 0 | **1 452** (codec / container-version / save-mode / slot × 363) |

* `declared − observed = 0` and `observed − declared = 0` on every run: no key was observed without
  a prior declaration, and every declared key received a row.
* Case ids are the real test paths with the unit or slot appended, e.g.
  `studio_domain::save_matrix::save_matrix_over_all_fixtures#BLUS314360200 - ROSTER-repack`.
* Fixture digests are the real file hashes: the `nhl10-ps3/db-presentation/read` row carries
  `e87b8c806b1d351a…`, the SHA-256 of `anchors/ps3/nhl10/databases/nhlng.db`; the PS3 NHL 15 ROSTER
  rows carry `fd234dc030586fb1…`, the value C0-REG1b §2 measured independently.
* The 189 unmapped records are 27 anchors × the seven `db-operations`-less operations
  (157 `pass` · 16 `refused-clean` · 16 `skipped-missing-fixture`), each with reason
  `operation-not-in-closed-db-operations-vocabulary` — recorded, never forced onto a neighbouring key.
* Implementation scopes: the three used scopes resolve to real digests over the candidate tree;
  `headmesh-convert` still resolves to `unknown` (its paths do not exist at HEAD) and no emitter
  declares it — the unknown scope stays in the definitions and withholds, it does not disappear.

### 3.3 The isolated merge path

`qa-evidence` built from the candidate (`cargo build -p tdb-qa --bin qa-evidence --locked --offline`);
every command with `--repo <candidate> --root <evroot>` and an explicit disposable `--data`.

| Step | Result | Exit |
|---|---|---|
| `check --data <empty dir>` | `10 published file(s) differ from a fresh build` (nothing published yet) | **2** |
| `build --out out1` | built 10 files (+ `build-inputs.json`) | 0 |
| `build --out out2` | built 10 files | 0 |
| `diff -r out1 out2` | **byte-identical, 11/11** | — |
| `publish --from out1 --data data-scratch` | published 10, id `fba63218d22e4811`, prior generation retained at `.qa-prev-fba63218d22e4811` | 0 |
| `check --data data-scratch` | **`ok: 2175 row(s), no validation failure, no drift`** | **0** |
| `publish --recover --data data-scratch` | `phase completed — the transaction is already resolved; nothing to do` | 0 |
| `build --out <evroot>/auto` | refused: `path-overlap … resolves inside the evidence root` | 4 |

**Selected runs** (`build-inputs.json` = the matrix provenance's `merge_input_digests`): exactly the
four run directories above, each cited `[completed+selected]`, plus `manual/` at the empty-tree
digest `sha256:e3b0c442…` (layer B never written, V11). `scope_definitions_digest =
sha256:78283e37…` — the SHA-256 of the candidate's `defs/qa/scopes.toml` byte-for-byte.

**The published matrix — 2 175 rows** (2 016 variant-less universe + 159 variant cells):

| Axis | Distribution |
|---|---|
| `qualification` | **140 `automated-qualified`** (`db-presentation` 27, `db-operations` 113; 27 distinct targets) · 159 `withheld` · 36 `not-applicable` · 1 840 `unqualified` · **0 `game-verified`** |
| `reason` on withheld | 159 × `unknown-applicability` — every variant cell, and nothing else |
| `automated` | 230 `pass` · 32 `mixed` · 121 `declared-not-executed` · 28 `skipped-missing-fixture` · 1 764 `not-attempted` |
| `fresh` | 411 `fresh` · 1 764 `unknown` (nothing selected ⇒ never fresh) |
| `applicability` | 978 `required` · 1 002 `optional` · 36 `not-applicable` · 159 `unknown` |
| `manual` / `implementation` | 2 175 `none` / 2 175 `unknown` |
| `conflict` / `unresolved` | empty on every row; `qa-evidence-conflicts.csv` and `-unresolved.csv` are header-only |
| `nhl07-ps3` | 36 write/acceptance cells `not-applicable` citing `research-only-unit` (including cells whose observations passed); `db-presentation/read` is `required` by the vocabulary and is `automated-qualified` |

The distribution matches R1 / PROBE1 / REPAIR1's shape exactly; it is reported here **from this
package's own run**, not carried over.

### 3.4 Reported separately — what completion does not mean

| Category | Count | What it is |
|---|---:|---|
| Missing fixtures | **28** observations | `legacyrecomp-recomp` in all four emitters (no anchor tree at all); 21 `operation_matrix` `Status::NoFixture` cells (schema carries the feature, the anchor ships no entity to exercise it); 6 `save_matrix` units with no save fixture. 17 targets carry at least one such row |
| Unsupported variants | 3 `nhl15-ps4` save fixtures | classify to no supported save shape: `container-detect/unsupported = refused-clean`, extract and repack `not-attempted`; the variant-less parents `not-attempted`. The gap is visible as `unqualified`, not hidden |
| Declared-not-executed | **121** rows | the 84 variant-less `save-inner-container` parent cells (deliberately left unsatisfied, §4.4) plus the parent cells of the units above |
| Failures | **0** | no `fail` observation, no `failed`/`interrupted` run in this capture; the failure path is exercised by the regression suite (§4.5) and was demonstrated on real emitters in S2-INT1 §5.4 / R1 §3 |
| Incomplete execution | **0** journals left | all four runs sealed; the mid-run journal snapshot in §4.1 is the only in-progress state observed |
| Withheld qualification | **159** rows | every variant cell: `applicability = unknown` because no accepted document declares which variants a target must cover. **No required-variant catalog was introduced** |
| Unmapped executions | **189** records | seven `operation_matrix` operations outside the closed `db-operations` vocabulary (S2-INT1 §8 Q2, still open) |
| Game verification | **0** cells | unchanged |

## 4. Integration semantics

### 4.1 Declarations precede execution — observed on a live run

While `mutation_matrix` was still running, its journal was read (read-only):
`keys-declared.txt` held **all 28** declarations at a point when `observations.csv` held **49**
records, and `journal.json` already carried the full identity (`db-edit` v3, real digest, real
toolchain). `Emitter::declare` persists each declaration before returning, and `RunBuilder::observe`
refuses an undeclared key (V6), so an observation cannot precede its declaration.

One nuance is stated rather than hidden: `save_matrix` declares its **variant-bearing** keys
lazily, immediately before the case that uses them, because the variant is the codec the bytes
classify to and is unknown before classification. Every variant-less parent is declared up front.
A hard kill mid-run therefore leaves the not-yet-reached fixtures' variant keys undeclared while
their parents are declared. This is inherent to a content-derived variant and is not changed here;
§4.5's second case pins down that a late declaration is persisted and behaves like an early one.

### 4.2 Failure and interruption remain represented by the committed journal mechanism

`crates/tdb-qa/src/evidence/{emit,journal,merge,store}.rs` are consumed exactly as committed at
`819940a` (clean against HEAD before and after this package). The regression suite covering that
mechanism ran green in the candidate: `evidence_journal` **31**, `evidence_emit` **9** (7 + 2 new),
`evidence_file_reads` 9, `evidence_discovery` 13. The emitters' own failure path — a drift panic
after the last observation with `finish()` never reached — now has a direct case (§4.5).

### 4.3 Freshness scopes vs. code actually exercised

Checked by reading what each emitter calls, on HEAD's `studio-domain` sources:

| Scope | Exercised by the emitters | Declared at v2? | Action |
|---|---|---|---|
| `db-read` | `TdbFile::parse`, `FieldMetaIndex`, `RosterModel::build`, `Capabilities::probe`, `SurfaceCapabilities::probe` (`surfaces.rs`, `capabilities.rs`, `view.rs`, `tdb-relations`) | yes | none — unchanged at v2 |
| `db-edit` | `Project::load` (builds `RelationRegistry` **and `EnumRegistry`**); every `EditSession` operation passes `project.enums()` into `tdb-edit`; `sess.validate()` → `validate_invariants(db, relations, enums)`; `strategy-edit` picks its cell via `team_strategies()` in **`team_tabs.rs`** | `crates/tdb-enums/src/**` **no** (only the `defs/enums/**` definitions, via `shared-core`); `team_tabs.rs` **no** | **v3**: both added |
| `save-containers` | `SaveContainer` / PS4 codec (`tdb-savedata`), and the ROSTER semantic edit through `Project::load` + `EditSession` — the same enum-registry seam | `crates/tdb-enums/src/**` **no** | **v3**: added |
| `audit.rs` | *not* on the exercised path (`validate()` returns the raw report; `audit_report()`/`humanize` is UI-facing and not called) | — | correctly absent |

Contract §9 Q-D classifies scope refinement as ordinary implementation work; the bump is versioned,
no committed run declares v2 (none is committed), and the comments in `scopes.toml` record why.
The captured runs in §3.2 therefore declare **v3** for `db-edit` / `save-containers` and **v2** for
`db-read`. Codex may strike item B of §1.1 — the capture would then need re-running so the runs
declare the committed versions, and the merge shape would be unchanged.

**Gaps found and left as they are** (reported, not repaired here):

* **Per-title metadata is not part of any fixture identity.** `fixture_sha256` and the
  `input_digests` cover the `.db` only; the `*-meta.xml` beside each anchor — which drives every
  codec decision the emitters assert — is digested nowhere (the `shared-core` v2 comment records
  it as "not declared here"). A metadata edit would not stale any observation. This needs either a
  second input digest per anchor in the emitters or a scope entry over `anchors/**/*-meta.xml`;
  both are boundary decisions for the architect.
* `headmesh-convert` stays `unknown` at HEAD (PROBE1 §5.4) — benign, no emitter declares it.

### 4.4 Conservative parent/variant handling preserved

Every `save_matrix` fixture observation is recorded against a variant-bearing key with the codec,
container version, save mode and slot as facets; each of the 84 variant-less parents receives an
explicit `not-attempted` (for covered units) or `skipped-missing-fixture` (for units with no save
fixture) with the reason in its detail. The merge withholds all 159 variant rows
(`unknown-applicability`) and qualifies **no** parent from variant evidence. **No required-variant
catalog was introduced and no headmesh pair coverage was inferred**; `headmesh_pair_keys` still has
no production caller.

One cosmetic defect noticed on the rows: the covered-parent detail string in `save_matrix.rs`
(the `NotAttempted` branch of the final loop) contains runs of literal spaces where a string
continuation was intended, so the published detail reads
`…recorded against a                  variant-bearing key…`. It is payload only, affects no key,
outcome, digest or decision, and is left untouched so the emitter stays byte-identical to the
REPAIR1-verified file; it should be corrected whenever that file is next edited.

### 4.5 Focused coverage added — two cases in `crates/tdb-qa/tests/evidence_emit.rs`

Both are behaviours the wiring relies on that the seven existing cases did not pin down; both run
over a synthetic root under the system temp tree and touch nothing in the repository.

| Case | Asserts |
|---|---|
| `a_panic_outside_any_case_is_sealed_failed_by_the_drop_guard_with_every_observation_kept` | two passing observations, then a panic **outside** any `Emitter::case` with `finish()` never reached (the emitters' drift guard): the panic propagates; exactly one run is sealed **`failed`**; both `pass` observations are retained; no fabricated `fail` row is added; the journal is removed; the merge reports both keys `executed-unselected` and neither `automated-qualified` |
| `a_key_declared_after_observation_has_begun_is_persisted_and_stays_below_its_parent` | a variant-less parent is declared and observed `not-attempted`; only then is `…/container-repack/v4` declared and observed `pass`: the late declaration is persisted to `keys-declared.txt` before its observation; the sealed run declares both keys and validates; the variant row is `pass` / `withheld` / `unknown-applicability`; the parent row is `declared-not-executed` and not qualified |

`cargo test -p tdb-qa --test evidence_emit` → **9 passed**. Whole crate → **230 passed · 0 failed ·
1 ignored** (72 unit · 32 acceptance · 13 discovery · 9 emit · 9 file-reads · 31 journal · 19 + 1
ignored migration · 36 publication · 9 inventory).

## 5. Determinism, publication and check — recorded inputs

| Input | Value |
|---|---|
| Evidence root | `…/scratchpad/emit1/evroot` — 4 run directories, no journals, no `manual/`, no `legacy/` |
| Repository for scope digests | the candidate tree (HEAD + overlay) |
| Runs selected at build | the four in §3.2, all `completed+selected`; no run superseded, none unselected |
| Layer B | absent — provenance digest `sha256:e3b0c442…` |
| Scope definitions | `sha256:78283e37…` (candidate `defs/qa/scopes.toml`) |
| Output | 10 files: `qa-evidence-{matrix,unmapped,unresolved,conflicts}.csv`, `asset-matrix-divergence.csv`, each with a `.provenance.json` |
| `build` × 2 | byte-identical |
| `publish` target | `…/scratchpad/emit1/data-scratch` only; **`docs/audits/data` was never named** on any command |
| `check` after publish | exit 0, 2 175 rows |
| CI | nothing touched (R-NO-CI) |

## 6. Validation, baseline vs regression, preservation

### 6.1 Tests

| Command (candidate) | Result |
|---|---|
| four emitters, disabled mode | 4 / 4 passed (§3.1) |
| four emitters, `QA_EVIDENCE_ROOT` set | 4 / 4 passed, 4 sealed `completed` (§3.2) |
| `cargo test -p tdb-qa --locked --offline` | 230 passed · 0 failed · 1 ignored |

### 6.2 Lint and format — baseline failures distinguished from regressions

| Check | Result | Classification |
|---|---|---|
| `cargo clippy -p tdb-qa --all-targets --locked --offline -- -D warnings` | **exit 0** | clean |
| `cargo fmt -p tdb-qa -- --check` | **exit 0** | clean |
| `rustfmt --check` on the five overlay `.rs` files | **all clean** | clean |
| `cargo clippy -p studio-domain --test {surface,mutation,operation,save}_matrix -- -D warnings` | exit 101 — errors in `tdb-rx2/src/playermesh.rs`, `tdb-archive-codec/src/{dds,lzx,refpack}.rs` | **baseline** — HEAD code, C0.2 known repo-wide clippy failure |
| same with `--no-deps` | exit 101 — **8** diagnostics, all in `studio-domain/src/{assets,attributes,formula,head_catalog,view}.rs` | **baseline** — HEAD lib sources (no `src/` file is in the overlay) |
| same, warnings non-fatal, diagnostics attributed by path | the four test targets and `evidence_emit.rs` contribute **0** diagnostics | the overlay is clean; the gate cannot go green until the lib's baseline defects are repaired by their owner |
| `cargo fmt -p studio-domain -- --check` | 617 diff hunks | **baseline** — C0.2 known; none in the overlay files |

No assertion was weakened and no committed CSV regenerated. `QA_WRITE` was never set.

### 6.3 Preservation

| Check | Result |
|---|---|
| Candidate manifest: before → after disabled runs → after capture/merge/lint | 1 192 files, **0 differing entries** at every step; no `auto/`, `in-progress/`, `.qa-prev-*` or `docs/audits/evidence` under the candidate |
| Live `anchors/**` + `docs/audits/**` | 621 files, `sha256` list before vs after: **0 mismatches** (the four emitter CSVs included) |
| Live `docs/audits/evidence/` | does not exist, before or after |
| HEAD / index | `58d1bc07…` unchanged / empty |
| `git status --porcelain` | 255 → 257 entries: exactly the two deliberate edits (`M defs/qa/scopes.toml`, `M crates/tdb-qa/tests/evidence_emit.rs`) added, **0 removed**, nothing else changed; the five overlay-A files and root `Cargo.toml`/`Cargo.lock` byte-unchanged |
| Pre-existing dirty/untracked work | untouched — no checkout, reset, stash, clean or branch change |
| Prior evidence (`H:\c0-s2-int1\…`, PROBE1/REPAIR1 candidates) | not read, not written |
| Logs and manifests | `…/scratchpad/emit1/logs/` (overlay manifest, candidate manifests ×4, live manifests, run logs, merge log, matrix summary, clippy logs) |

## 7. Proposed commit boundary

Two options, both fully verified above on one candidate:

| Option | Paths | Notes |
|---|---|---|
| **B1 — wiring only** | `crates/studio-domain/Cargo.toml`, the four `crates/studio-domain/tests/*_matrix.rs`, `Cargo.lock` (1 line) — **6 paths, 0 A · 6 M** | the REPAIR1-verified files unchanged; runs would declare `db-edit`/`save-containers` **v2**, which is what a capture at that commit would produce |
| **B2 — B1 + this package's additions** | B1 + `defs/qa/scopes.toml` (v3 widening) + `crates/tdb-qa/tests/evidence_emit.rs` (+2 cases) — **8 paths, 0 A · 8 M** | what the §3 capture actually ran against |

Subject proposal: `feat(qa): wire matrix emitters to the evidence observation lifecycle`. Root
`Cargo.toml` is not part of either option. Documentation (this receipt) stays outside the commit.

## 8. Remaining decisions — not made here

| Area | Status |
|---|---|
| **Storage (U3)** | the conservative reading stands: `QA_EVIDENCE_ROOT` must be outside the repository (refused otherwise). Where layer A lives permanently is undecided; this capture lives in a scratchpad and is disposable |
| **Selected-run retention / recapture** | a published generation cites run ids that exist only in that root. Reproducing `check` from a clean checkout needs either immutable retention of the selected runs or a recorded recapture procedure (NEXT1 §8 / COMMIT-EVID1 §5 U3/D2). Not resolved |
| **Consumer transition (R-TRANSITION, Q-C)** | the four `QA_WRITE=1` in-place CSV writes are untouched and still the consumers' source; the observe call is parallel. Retiring the in-place writes is a later bounded package |
| **CI (R-NO-CI)** | nothing activated; a clean-checkout proof would additionally need the lib-level clippy baseline repaired (§6.2) and Windows CI (C0.2 R6) |
| Vocabulary / variants | Q1 (`mutation_matrix` + `save-raw-db`), Q2 (seven unmapped operations — 189 records this run), Q3/R3 (required-variant catalog), Q4 (sealing an interrupted journal) remain open |
| Metadata identity | the `*-meta.xml` gap in §4.3 |
| Cosmetic | the `save_matrix` detail-string spacing in §4.4 |

## 9. Limits honoured

No staging, commit or push; no new adapter; no headmesh work; no external repository; no game
installation; no live evidence migration or publication (`docs/audits/data` and `docs/audits/evidence`
never written); no capability promotion; no cleanup beyond the two listed edits. Stopping for Codex
review.
