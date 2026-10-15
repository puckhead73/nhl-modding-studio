# QA evidence contract

> **Status:** proposed 2026-09-07, revised **R1** 2026-09-07 (milestone **C0.4a**), revised
> **R2** 2026-09-07 (package **C0-BULK1**, checkpoint 1), amended **2026-09-07** by packages
> **C0-S2-INT1-R2/R3/R4** (qualification row 4b and §4.5 integrity findings — a clarification of
> existing implemented behaviour, no new product behaviour). Nothing here promotes any support status.
> R1 was a specification only; **R2 is the reconciled specification that S1–S3 implement**
> (`crates/tdb-qa/src/evidence/`).
> **Companions:** [project completion roadmap](project-completion-roadmap.md) (C0 item 4, §5
> continuous QA and evidence invalidation), [full-game QA plan](full-game-qa-editability.md) (the
> verification procedures), [C0.4a validation record](../validation/c0-04a-evidence-contract.md).
>
> **R1 scope:** the first draft was internally contradictory in seven areas (mutable run files,
> implicit newest-wins selection, orphan receipts both fatal and ignored, conversion pairs collapsing
> into one key, a single ordered status scale, a migration that regenerated from a live catalog and
> fabricated receipts, and `git checkout` as recovery). §§2–9 below are rewritten; §1 is unchanged
> except for three defects added by the R1 review.
>
> This document covers **QA evidence only**. It is not Studio's Project schema, and it defines no UI.
>
> **R2 scope:** R1 remained internally contradictory in eight further places, all of which would
> have destroyed or hidden evidence in the implementation. They are resolved by the named rules in
> **§12**, and the affected tables in §§3–5 and §10 are corrected in place and marked `(R2)`.
> R2 changes **no** closed decision (§9 Q-A – Q-E stand), removes no manual evidence, and relaxes
> no validation rule. Where R2 narrows a requirement it does so only by applying an already
> approved scope decision (roadmap §1) that R1 had blanket-overridden.
>
> **Every example in this document is synthetic.** Hashes are shape-valid placeholders, and no
> example carries a real person's sign-off.

---

## 1. Current producers and consumers

### 1.1 Producers

| Artifact | Producer | Trigger | Written how |
|---|---|---|---|
| `docs/audits/data/anchors-manifest.csv` | [`crates/tdb-qa/src/manifest.rs`](../../crates/tdb-qa/src/manifest.rs) via [`bin/qa_inventory.rs`](../../crates/tdb-qa/src/bin/qa_inventory.rs) | manual `cargo run -p tdb-qa --bin qa-inventory` | whole-file rewrite |
| `docs/audits/data/schema/<platform>-<game>.json` (×27) | [`crates/tdb-qa/src/schema.rs`](../../crates/tdb-qa/src/schema.rs) | same | **directory deleted and rebuilt** (`qa_inventory.rs` lines 24–27) |
| `docs/audits/data/schema-diff.csv` | [`crates/tdb-qa/src/diff.rs`](../../crates/tdb-qa/src/diff.rs) | same | whole-file rewrite |
| `docs/audits/data/game-qa-matrix.csv` (392 rows) | [`crates/tdb-qa/src/matrix.rs`](../../crates/tdb-qa/src/matrix.rs) | same | whole-file rewrite |
| `docs/audits/data/mutation-matrix.csv` | [`crates/studio-domain/tests/mutation_matrix.rs`](../../crates/studio-domain/tests/mutation_matrix.rs) | `QA_WRITE=1 cargo test … --test mutation_matrix` | whole-file rewrite, else drift-assert |
| `docs/audits/data/operation-matrix.csv` | [`crates/studio-domain/tests/operation_matrix.rs`](../../crates/studio-domain/tests/operation_matrix.rs) | `QA_WRITE=1` | same |
| `docs/audits/data/save-matrix.csv` | [`crates/studio-domain/tests/save_matrix.rs`](../../crates/studio-domain/tests/save_matrix.rs) | `QA_WRITE=1` | same |
| `docs/audits/data/surface-capabilities.csv` | [`crates/studio-domain/tests/surface_matrix.rs`](../../crates/studio-domain/tests/surface_matrix.rs) | `QA_WRITE=1` | same |
| `docs/audits/data/assets/nhllegacy/xbox360/identities.csv` | [`crates/studio-domain/src/assets.rs`](../../crates/studio-domain/src/assets.rs) (test, line ~4519) | `QA_WRITE=1` | same |
| `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` | **none — hand-maintained** | human edit | manual |
| `docs/audits/data/assets/nhllegacy/xbox360/archives.csv` | **none — hand-maintained** | human edit | manual |
| `docs/audits/data/asset-capability-matrix.csv` | **none — hand-maintained** | human edit | manual |
| `docs/audits/data/drift-matrix.csv`, `savefile-inventory.csv`, `anchor-tables-full.csv` | **none — hand-maintained** (superseded narratives) | human edit | manual |

### 1.2 Consumers

| Consumer | Reads | Asserts |
|---|---|---|
| [`crates/tdb-qa/tests/inventory.rs`](../../crates/tdb-qa/tests/inventory.rs) | all four `qa-inventory` artifacts | byte-equality with regeneration; header string; status vocabulary; 27 anchors |
| [`crates/tdb-headmesh/tests/matrix_guard.rs`](../../crates/tdb-headmesh/tests/matrix_guard.rs) | `defs/appearance/*.toml` + `docs/audits/data/assets/<title>/<platform>/capability.csv` | a promoted `[[pair]]` needs a `heads` row; matrix `game_verified` ⇔ CSV `game_verified == pass`; UV classes measured |
| the four `*_matrix.rs` tests | their own CSV | drift |
| [`docs/audits/qa-ledger.md`](../audits/qa-ledger.md) | narrative over `game-qa-matrix.csv` | — |
| [`docs/01-scope-and-compatibility.md`](../01-scope-and-compatibility.md) | narrative, per-target evidence level | — |
| `docs/assets/games/nhllegacy/xbox360/ASSET_PROVENANCE.md` | narrative over the asset CSVs | — |

### 1.3 Defects this contract must fix

| # | Defect | Evidence |
|---|---|---|
| **E1** | `game-qa-matrix.csv` **iterates `FAMILIES` but emits no `family` column** (`matrix.rs` `build_csv`, header line 325). A row's family is implied by its **ordinal position** in its unit's 14-row block. | 10-column header; 392 rows = 28 units × 14 families |
| **E2** | Row identity is therefore `(game, platform, ordinal)`. Inserting, removing or reordering a family silently re-labels every later row. | `build_csv` nested loop |
| **E3** | Status is derived from **fixture presence**, not an executed case. `save-raw-db` becomes `edit-roundtrip-pass` whenever `has_main_db` is true. | `matrix.rs` 170–182 |
| **E4** | A cited gate is attributed to a unit it never covered. **PS3 NHL 07 `save-raw-db` claims `edit-roundtrip-pass` citing `save_matrix_over_all_fixtures`, but `save-matrix.csv` has no PS3 NHL 07 rows.** | `game-qa-matrix.csv` vs `save-matrix.csv` |
| **E5** | `last_verified` is a **hardcoded constant** (`PHASE0_DATE`/`PHASE3_DATE`/`PHASE4_DATE`, lines 28–33), re-emitted whether or not anything ran. | `matrix.rs` |
| **E6** | Manual sign-off has **nowhere to live**: the generator can never assign `game-verified` and rewrites whole files. | `matrix.rs` doc comment |
| **E7** | `asset-capability-matrix.csv` (4 rows) and `assets/nhllegacy/xbox360/capability.csv` (5 rows) **duplicate the same subject with different columns and content**; the former omits `heads` entirely. | both files |
| **E8** | Nothing distinguishes *declared* from *observed* coverage. No matrix covers `legacyrecomp`; `save_matrix` covers 26 of 27 units. | computed from the four CSVs |
| **E9** | Capability CSV values (`external`, `not_run`, `partial`) are an **undeclared, unvalidated vocabulary** disjoint from `STATUS_VOCAB`. | `capability.csv` |
| **E10** *(R1)* | Every automated emitter **overwrites its single CSV**. There is no run history, so an earlier result is destroyed by a later run and a non-run is indistinguishable from a wiped run. | the four `QA_WRITE=1` emitters |
| **E11** *(R1)* | The headmesh capability CSV records **one `heads` row per destination**, but 15 conversion `[[pair]]`s target 12 distinct destinations. Destination-level evidence cannot express pair-specific proof, yet `matrix_guard` uses it to gate pair promotion. | `capability.csv` vs `defs/appearance/headmesh-matrix.toml` |
| **E12** *(R1)* | `qa_inventory` **deletes and rebuilds `docs/audits/data/schema/`**. An interrupted run leaves the directory partially reconstructed with no journal and no retained prior copy. | `qa_inventory.rs` 24–27 |

---

## 2. Layers, identity and storage

### 2.1 Four layers, strictly separated

| Layer | Contents | Written by | Qualifies output? |
|---|---|---|---|
| **A — observations** | what an automated case actually did, in an immutable run | automated emitters | yes, when selected and fresh |
| **B — manual records** | durable human verification, incl. game acceptance | humans; **and** an explicitly approved migration install (§7.2) | yes, when selected and fresh |
| **L — legacy claims** | pre-contract status labels migrated verbatim | migration only | **never** |
| **C — derived summaries** | the roll-up docs and guards read | the merge tool | — |

**R-SEP.** An ordinary generator may read A, B, L and write only C and its own new A run directory.
It has no write path under the manual root. The single exception is `qa-evidence migrate-install`
(§7.2), which is a separate command requiring an explicit approval file.

**R-LEGACY.** A layer-L record is a *claim*, never an observation and never a manual record. It can
never produce `automated-qualified` or `game-verified`, and it never contributes a verification date.

### 2.2 Coverage key

A **coverage key** identifies *what is covered*. It is not a case id and not a run id.

```abnf
key         = target "/" family "/" operation [ "/" variant ] [ "?" conversion ]
target      = title "-" platform          ; nhllegacy-xbox360, nhl21-ps4, legacyrecomp-recomp
conversion  = "from=" token ";class=" token ";path=" token ";recipe=" token
token       = 1*( %x61-7A / DIGIT / "." / "_" / "-" / "@" )   ; lowercase only
```

| Part | Vocabulary | Notes |
|---|---|---|
| `target` | `<title>-<platform>` | **Already `tdb_headmesh::matrix::GameKey`**, so headmesh joins need no translation. |
| `family` | the 14 ids in `tdb_qa::catalog::FAMILIES` | the missing column of E1 |
| `operation` | closed per family (§4.1) | fixes E3/E8: coverage is per operation |
| `variant` | format/container variant **only where it changes coverage** — `v1`, `v4`, `ps4-lz4`, `stfs` | omitted when the operation is variant-independent |
| `conversion` | required whenever coverage depends on the source | fixes E11 |

**R-CONV.** A `conversion` component is **mandatory** for any operation whose coverage depends on
the source target, source asset class, destination, recipe/profile or conversion path. All four
fields are present when the component is present, in the fixed order above; unknown fields take the
literal `unknown`, which makes the key valid but blocks qualification (§4.3). Two headmesh pairs with
different sources, paths or recipes therefore have **different keys** and can never collapse.

**R-SOURCE-ONLY.** A value may appear as `conversion.from` without being a coverage `target`.
`nhl24` is a conversion source only: it is not in `tdb_qa::catalog::CATALOG`, it is never the
`target` of any key, and its presence as a source asserts nothing about editor support for it.

**R-ORDER.** Row order carries no meaning anywhere. Files are key-sorted for determinism; consumers
look records up by key.

Worked pair keys (synthetic, but structurally what migration would produce):

```
nhllegacy-xbox360/assets-heads/install?from=nhl24;class=head-authored;path=authored-weights;recipe=patha@1
nhl15-xbox360/assets-heads/install?from=nhl24;class=head-authored;path=authored-weights;recipe=patha@1
nhllegacy-xbox360/assets-heads/game-acceptance?from=nhl24;class=head-authored;path=authored-weights;recipe=patha@1
```

### 2.3 Case ids and multiple observations per key

A **case id** identifies the procedure that produced an observation, independently of what it covers:

- automated: `<crate>::<test-target>::<test-fn>[#<sub-case>]`, e.g.
  `studio_domain::save_matrix::save_matrix_over_all_fixtures#v4-repack`;
- manual: a procedure reference into the QA plan plus a local discriminator.

**R-MULTI.** Many observations may share one coverage key. Uniqueness is on the triple
`(run_id, case_id, key)` — never on `key` alone. Two independent cases exercising the same operation
are both retained and both cited; nothing deduplicates them.

### 2.4 Storage formats

| Layer | Format | Location | Reason |
|---|---|---|---|
| A | CSV observations + JSON manifest/state, **one immutable directory per run** | `docs/audits/evidence/auto/runs/<run_id>/` | CSV keeps the emitters' existing shape and reviewability; JSON carries the nested run metadata. One directory per run fixes E10 — nothing is overwritten. |
| B | **TOML, one file per record** | `docs/audits/evidence/manual/<target>/<family>/<record_id>.toml` | Appending a record is a new file: no merge conflicts, and no whole-file rewrite by which sign-off can be clobbered. TOML matches `defs/`, allows comments, and the `toml` crate is already a `tdb-headmesh` dependency. |
| L | TOML, one file per claim + one accounting CSV | `docs/audits/evidence/legacy/` | claims are reviewed like manual records but must be structurally distinguishable from them |
| C | CSV + JSON provenance sidecar | `docs/audits/data/` | existing docs and guards already read CSV; consumers migrate by column name |

### 2.5 Directory layout

```
defs/qa/scopes.toml                                  # versioned freshness scope definitions (§5.3)
docs/audits/evidence/
  auto/runs/<run_id>/{manifest.json,state.json,keys-declared.txt,keys-executed.txt,observations.csv}
  auto/runs-index.csv                                # append-only run_id -> manifest digest
  manual/<target>/<family>/<record_id>.toml
  legacy/frozen/                                     # byte copies + digests of legacy sources
  legacy/claims/<claim_id>.toml
  legacy/migration-accounting.csv
  candidates/<migration_id>/                         # migration output; never auto-installed
docs/audits/data/                                    # layer C (published)
  qa-evidence-matrix.csv            + .provenance.json
  qa-evidence-conflicts.csv         + .provenance.json
  qa-evidence-unresolved.csv        + .provenance.json
  asset-matrix-divergence.csv       + .provenance.json
  .qa-publish-journal.json                           # publication journal (§8)
```

---

## 3. Layer schemas

### 3.1 Layer A — an immutable run

`run_id = <emitter>-<utc-basic>-<8 hex of manifest digest seed>`, e.g. `save_matrix-20260907T120411Z-9f3ca1b2`.
A run directory is **write-once**: created, populated, sealed. It is never edited or deleted by any
generator.

`keys-declared.txt` / `keys-executed.txt` — one coverage key per line, sorted, LF. These files **are**
the membership evidence; the counts in the manifest are derived checks over them.

`manifest.json` (synthetic example):

```json
{
  "schema": "qa-run-manifest/1",
  "run_id": "save_matrix-20260907T120411Z-9f3ca1b2",
  "emitter": "save_matrix",
  "started_utc": "2026-09-07T12:04:11Z",
  "host_os": "windows",
  "toolchain": "rustc 1.96.0 (x86_64-pc-windows-msvc)",
  "keys_declared_sha256": "3b1f0c9d2e4a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f9012a3b4c5d",
  "keys_executed_sha256": "7e2d1a0b9c8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f",
  "keys_declared_count": 104,
  "keys_executed_count": 96,
  "observations_sha256": "a1b2c3d4e5f60718293a4b5c6d7e8f9012a3b4c5d6e7f8091a2b3c4d5e6f7081",
  "scope_id": "save-containers",
  "scope_version": 1,
  "implementation_digest": "sha256:1b9c0d1e2f3a4b5c6d7e8f9012a3b4c5d6e7f8091a2b3c4d5e6f70819a2b3c4d",
  "input_digests": {
    "anchors/ps3/nhl15/savefiles": "sha256:44a0b1c2d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8"
  },
  "supersedes": [],
  "supersede_mode": "none"
}
```

`state.json` — the **terminal state marker**, written last:

```json
{ "schema": "qa-run-state/1", "run_id": "save_matrix-20260907T120411Z-9f3ca1b2", "state": "completed", "ended_utc": "2026-09-07T12:35:02Z", "failure": null }
```

**R-STATE.** `state` ∈ `completed | failed | interrupted`. **A run directory with no readable
`state.json` is `interrupted`** — a state is never inferred from file presence, timestamps or
counts. Only `completed` runs are selectable (§4.2); `failed` and `interrupted` runs are retained,
readable and citable as history, and their observations never qualify anything.

`observations.csv`:

```csv
schema,key,case_id,outcome,fixture_sha256,output_sha256,detail
qa-observation/1,nhl15-ps3/save-inner-container/container-repack/v4,studio_domain::save_matrix::save_matrix_over_all_fixtures#v4-repack,pass,44a0b1c2d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8,c9d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8,"detect -> extract -> edit -> repack -> reopen; checksums valid"
qa-observation/1,nhl11-ps3/save-inner-container/container-repack/v1,studio_domain::save_matrix::save_matrix_over_all_fixtures#v1-repack,skipped-missing-fixture,,,"declared and attempted; no v1 fixture present"
```

Note what is **not** in this file: `nhl07-ps3/save-raw-db/save` appears in neither
`keys-declared.txt` nor `observations.csv`, because this emitter does not cover that unit. Per V6 an
observation may only carry a declared key, so "the emitter never covered it" is expressed by
**absence**, and the merge reports it as `automated = not-attempted` (§4.2). It is never written as
an observation row.

`outcome` ∈ `pass | fail | refused-clean | skipped-missing-fixture | not-attempted`, where
`not-attempted` covers a declared key the run reached but deliberately did not run (a disabled
probe); a key the emitter does not cover is absent entirely.
`skipped-missing-fixture` is mandatory where a case returned early — the C0.2 §4.4 failure mode
(15 cases reporting `ok` without exercising anything) must be representable and must never read as a
pass.

**R-ABSENT.** The absence of a new run for an emitter **erases nothing and asserts nothing**. Earlier
completed runs stay selected. A key that no completed run declares is `not-attempted`; a key declared
but not executed is `declared-not-executed`. Neither is a pass, a fail, or a reason to drop history.

### 3.2 Layer B — manual record (synthetic example)

```toml
schema = "qa-manual-record/1"
record_id = "20260806-nhllegacy-xbox360-heads-install-01"
key = "nhllegacy-xbox360/assets-heads/install?from=nhl24;class=head-authored;path=authored-weights;recipe=patha@1"
result = "pass"                 # pass | fail | partial | inconclusive
example = true                  # synthetic illustration, not real evidence

[procedure]
kind = "operation-check"        # operation-check | game-acceptance
reference = "docs/plans/full-game-qa-editability.md#head-install"
evidence = ["docs/audits/nhl24-legacy-patha-roundtrip.md"]
notes = "Converted head installed into an extracted tree and re-read."

[performed]
by = "synthetic-example"
date_raw = "2026-08-06"
date_precision = "day"          # day | minute | instant | unknown

[target]
title = "nhllegacy"
platform = "xbox360"
runtime = "unknown"
runtime_build = "unknown"
game_build = "unknown"

[source]
studio_commit = "unknown"
studio_dirty = "unknown"
implementation_digest = "unknown"
scope_id = "headmesh-convert"
scope_version = 1

[[artifact]]
role = "output"
path = "(private) texlib_67.rx2"
sha256 = "unknown"

[provenance]
entered_utc = "2026-09-08T09:00:00Z"
entered_by = "synthetic-example"
origin = "authored"             # authored | migrated
```

**R-UNKNOWN.** Any field whose value is not recorded is the literal `"unknown"`. It is never
inferred, back-dated or defaulted. `date_raw` + `date_precision` preserve exactly the precision the
source had; a bare date never becomes a UTC instant.

**R-SUPERSEDE.** Supersession is explicit and additive; nothing is deleted:

```toml
supersedes = ["20260806-nhllegacy-xbox360-heads-install-01"]
supersede_reason = "corrected"  # corrected | rerun | invalidated-by-change
```

### 3.3 Layer L — legacy claim (synthetic example)

```toml
schema = "qa-legacy-claim/1"
claim_id = "gameqa-nhl07-ps3-row03"
example = true
source_file = "docs/audits/evidence/legacy/frozen/game-qa-matrix.frozen.csv"
source_sha256 = "5f4e3d2c1b0a9988776655443322110ffeeddccbbaa99887766554433221100f"
source_row = 3
source_row_raw = "nhl07,ps3,gen6,anchors/ps3/nhl07/databases/NHL07NG.db,edit-roundtrip-pass,studio_domain::save_matrix::save_matrix_over_all_fixtures,,2026-08-02,...,..."

claim_status_raw = "edit-roundtrip-pass"
claim_last_verified_raw = "2026-08-02"
claim_evidence_raw = "loose-DB save is a byte-identical no-op round trip"
maintenance = "generated"       # generated | hand
qualifies = false               # always false; layer L never qualifies

[mapping]
target = "nhl07-ps3"
family = "save-raw-db"
family_source = "positional-assumption"   # see R-POSITIONAL
confidence = "positional"
operation = "unknown"
unresolved = ["cited-gate-has-no-rows-for-this-unit", "operation-not-decomposable"]
```

### 3.4 Layer C — derived summary

Four axes, never one scale (§4.3). Header:

```csv
schema,key,target,family,operation,variant,conversion,applicability,implementation,automated,manual,qualification,fresh,reason,auto_runs,manual_records,legacy_claims,conflict,unresolved
qa-summary/2,nhl07-ps3/save-raw-db/save,nhl07-ps3,save-raw-db,save,,,required,implemented,not-attempted,none,unqualified,unknown,"no completed run declares this key",,,gameqa-nhl07-ps3-row03,none,no-observation
```

---

## 4. Deterministic merge, selection and qualification

### 4.1 Operation vocabulary and required evidence

`required` marks operations that must be qualified for the family to be complete; optional
operations are reported but never gate a roll-up.

**(R2)** The `Required` column below is the *family-level declaration*. It is **not** the final
applicability of a cell: the required set is resolved per `(target, family, operation)` by
**R-APPL-OPS** (§12.7), which classifies every operation `read-only` / `write` / `acceptance` and
applies the roadmap's research-only scope decision. R1's blanket "all" — which required
`game-acceptance` of every family on every unit, including read-only families and the research-only
NHL 07 PS3 unit — is replaced by that resolution.

| Family | Operations | Class of each | Declared required |
|---|---|---|---|
| `db-presentation` | `read` | read-only | `read` |
| `db-operations` | `scalar-edit`, `roster-create`, `roster-move`, `roster-trade`, `roster-delete`, `game-acceptance` | write ×5, acceptance | all |
| `save-raw-db` | `save`, `reopen`, `game-acceptance` | write, read-only, acceptance | all |
| `save-inner-container` | `container-detect`, `container-extract`, `container-repack`, `game-acceptance` | read-only, read-only, write, acceptance | all |
| `save-outer-package` | `outer-package`, `game-acceptance` | write, acceptance | all (see R-Q04) |
| `onboarding-*` | `discover`, `identify` | read-only ×2 | `discover` |
| `assets-*` | `discover`, `identify`, `preview`, `export`, `replace-modified`, `rebuild`, `install`, `game-acceptance` | read-only ×4, write ×3, acceptance | `replace-modified`, `install`, `game-acceptance` |

**R-GAME.** `game-acceptance` is its **own operation with its own key**. A manual `pass` on
`preview`, `export`, `rebuild` or `install` records exactly that operation and **never** yields
`game-verified`. Game acceptance requires a layer-B record with `procedure.kind = "game-acceptance"`
and **all** of these known (not `"unknown"`):

`artifact[role=output].sha256`, `target.runtime`, `target.runtime_build`, `target.game_build`,
`source.studio_commit`, `source.implementation_digest`.

**R-IDENTITY.** If any required identity is `"unknown"`, qualification is `withheld` with
`reason = unknown-identity`. The record is retained in full; it is history, not qualification.

### 4.2 Selection — explicit, never "newest wins"

**Automated.** Selection is **per key**, not per run. **(R2)** For a key `k`, a run is *suppressed
at `k`* only when some **`completed`** run supersedes it at `k` — see **R-SUPERSEDE-STATE** and
**R-PARTIAL-KEYS** (§12.1, §12.2). The **active run set at `k`** is every run that is `completed`,
declares `k`, and is not suppressed at `k`. Selection reads no timestamp, mtime, filename order or
directory order (**R-NO-TIME**, §12.4).

| Situation | Result |
|---|---|
| exactly one active run declaring the key | that run's observations are selected |
| two or more active runs declaring the key, agreeing | all selected, all cited |
| two or more active runs declaring the key, disagreeing | **all retained and cited**; `conflict = automated-disagreement`; qualification `withheld` |
| only `failed`/`interrupted` runs declare the key, and at least one **executed** it | **(R2)** nothing selected; `automated = executed-unselected`; every observation cited; if any is `fail`, `reason = unreviewed-failure` (§12.3) |
| only `failed`/`interrupted` runs declare the key, none executed it | nothing selected; `automated = not-attempted`; the runs are cited under `unresolved` |
| no run declares the key | `automated = not-attempted` |

A run may declare `supersedes` only when its declared key set is a superset of the superseded run's;
otherwise it must set `supersede_mode = "partial"` and list `supersede_keys`. Anything else is a
validation error (V8). **(R2)** Every `supersede_keys` entry must be declared by **both** runs (V13),
and a `failed`/`interrupted` run's `supersedes` has **no suppressing effect at all** (§12.1).

**Manual.** The **active record set** for a key is every record not named in any present record's
`supersedes`.

| Situation | Result |
|---|---|
| one active record | selected |
| several active records agreeing | all selected, all cited |
| several active records disagreeing | **all retained and cited**; `conflict = manual-disagreement`; qualification `withheld` |

**R-NOWIN.** Neither layer resolves a disagreement by recency, file order or path. Disagreement is a
first-class output, not something the merge decides.

### 4.3 Four independent axes

| Axis | Values |
|---|---|
| `applicability` | `required` · `optional` · `not-applicable` · `unknown` |
| `implementation` | `implemented` · `partial` · `absent` · `unknown` |
| `automated` | `pass` · `fail-current` · `fail-historical` · `mixed` · `skipped-missing-fixture` · `declared-not-executed` · **`executed-unselected` (R2)** · `not-attempted` |
| `manual` | `pass` · `fail` · `partial` · `inconclusive` · `none` |

**(R2)** `not-attempted` means *no completed or non-completed run executed the key*.
`declared-not-executed` means a run reached the key and deliberately did not run it.
`executed-unselected` means the key **was executed**, but only inside runs that are not selectable
(`failed` / `interrupted`). The three are never merged: an unexecuted case and an executed failing
observation are distinguishable in every output (§12.3).

`qualification` is **derived** from those four, never stored independently:

| # | Condition (first match wins) | `qualification` |
|---|---|---|
| 1 | `applicability = not-applicable` (with a cited reason) | `not-applicable` |
| 2 | `applicability = unknown` | `withheld` (`reason = unknown-applicability`) |
| 3 | any unresolved conflict on the key | `withheld` (`reason = conflict`) |
| 4 | `automated = fail-current`, or a current recorded blocker | `blocked` |
| **4a (R2)** | an **uncleared executed failure** exists for the key (§12.3), in any retained run | `withheld` (`reason = unreviewed-failure`) |
| **4b (C0-S2-INT1-R2)** | a **damaged** in-progress journal declares the key (§4.6) | `withheld` (`reason = damaged-evidence`) |
| 5 | manual pass exists but a required identity is `"unknown"` | `withheld` (`reason = unknown-identity`) |
| 6 | freshness `unknown` or `stale` on the selected evidence | `withheld` (`reason = stale-evidence`) |
| 7 | operation is `game-acceptance`, manual `pass`, fresh, identities complete | `game-verified` |
| 8 | `automated = pass`, fresh | `automated-qualified` |
| 9 | otherwise | `unqualified` |

**(R2)** Row 4a is placed **above** the identity, freshness and pass rows deliberately: an executed
failure must be able to withhold qualification that would otherwise be granted by a passing run,
and it must do so whether or not the run that observed it terminated cleanly.

**(C0-S2-INT1-R2)** Row 4b sits directly below 4a and above the identity, freshness and pass rows,
for the same reason: evidence that cannot be read back with certainty may *be* a failure on the key,
so an older completed pass must not go on qualifying it. See §4.6.

**R-STALE-FAIL.** `fail-historical` (a failure whose selected evidence is stale) does **not** trigger
row 4. It is surfaced as `unresolved = historical-failure-not-rerun` and prevents rows 7–8 from
being reached by any stale evidence anyway (row 6). A stale pass is likewise retained as history and
cannot qualify.

### 4.4 Roll-ups — explicit, not a minimum

`min` over these axes is meaningless because the values are not totally ordered. A roll-up over a
scope S (a family for a target, or a target) uses only S's **required** operations, in this order:

| # | Condition (first match wins) | Roll-up |
|---|---|---|
| 1 | S has no required operations defined | `undefined-scope` |
| 2 | any required op `qualification = withheld` with `reason = unknown-applicability` | `unknown` |
| 3 | any required op `blocked` | `blocked` |
| 4 | any required op `withheld` (any other reason) | `withheld` |
| 5 | any required op `unqualified` | `unqualified` |
| 6 | every required op `game-verified` | `game-verified` |
| 7 | every required op ∈ {`game-verified`, `automated-qualified`} | `automated-qualified` |
| 8 | every required op `not-applicable` | `not-applicable` |
| 9 | otherwise | `unqualified` |

**R-CHILD.** A roll-up is a presentation value. It is never stored as evidence, never cited as
proof, and a passing child operation never raises it past this table. "Portrait install passed on
Legacy Xbox 360" does not make `assets-portraits` — let alone the target — verified.

### 4.5 Applicability

**R-APPL.** `not-applicable` requires a positive cited reason (schema absence proven by a committed
snapshot, or a recorded scope decision). Absence of a fixture yields `missing-fixture` on the
observation and `unqualified` on the row. Absence of knowledge yields `applicability = unknown`.
Neither may be written `not-applicable`.

**R-Q04.** `save-outer-package` is **required scope under Q04** for Xbox 360, PS3 and PS4. It may be
`blocked` or `unqualified`; it may **not** be `not-applicable` on those platforms. `matrix.rs` line
218 currently emits `not-applicable` for PS3 and PS4; migration records that as a legacy claim and
sets the derived row to `applicability = required`, asserting no new capability.

---

### 4.6 Integrity findings on incomplete execution evidence

An in-progress journal (`<root>/auto/in-progress/<journal_id>/`, deliberately outside `auto/runs/`)
is loaded read-only and is never sealed by a reader. Loading it can produce **integrity findings**: machine-readable tokens recording
material that was persisted but could not be verified. A journal carrying any *damaging* finding is
**damaged**, and row 4b withholds qualification across every key that journal **declared** — the
conservatively affected scope, because a damaged record's own key is precisely what cannot be
trusted and is therefore never guessed at.

| Finding | Damaging | Meaning |
|---|---|---|
| `truncated-trailing-record:<file>` | yes | the append was interrupted; that record is dropped, everything before it is kept |
| `corrupt-record:<file>:<line>` | yes | the record's digest or escaping did not verify; neighbours are unaffected |
| `unknown-record-schema:<file>:<line>`, `unparseable-key:…`, `unknown-outcome:…` | yes | the record parsed but its content did not |
| `sealed-run-mismatch:<run_id>:<why>` | yes | the journal names a sealed run that is **not** the same evidence (identity disagrees, or the run lacks records the journal verified) |
| **`sealed-run-unverifiable:<run_id>:<why>` (C0-S2-INT1-R3)** | yes | the journal is **already damaged** and names a run its surviving records match. Equivalence cannot be established, because the dropped records are exactly the ones the comparison could not examine, so the journal is **retained** rather than deduplicated |
| `sealed-run-missing:<run_id>` | no | the named run is not present; the journal's records are still digest-verified, so it is reported and kept, and it does not withhold on its own |

`sealed-run-unverifiable` introduces **no new conflict class, axis value or qualification outcome**.
It is an integrity finding like the others, handled by row 4b, and its only distinct effect is on
**deduplication**: a clean journal proved equivalent to a present run is still dropped as the same
evidence, and a damaged one never is.

Two conditions are fatal at load rather than reported, because both remove the bound that makes
damage tolerable: damaged **declarations** (the declared scope would be unknown) and damage in a
journal that declares **nothing at all**.

## 5. Validation and freshness

### 5.1 Validation rules (hard failures on active inputs)

| # | Check |
|---|---|
| V1 | Every key parses against the §2.2 grammar and every part is in its declared vocabulary. |
| V2 | `(run_id, case_id, key)` is unique within a run; `record_id` is globally unique in layer B; `claim_id` globally unique in layer L. |
| V3 | `run_id` in `manifest.json` equals its directory name; `state.json`'s `run_id` matches. |
| V4 | `keys_declared_sha256` / `keys_executed_sha256` / `observations_sha256` match the files; the `_count` fields match the line counts. **Counts are derived checks — the key files are the membership evidence.** |
| V5 | `keys-executed.txt` ⊆ `keys-declared.txt`. |
| V6 | Every observation's key ∈ `keys-declared.txt`; every `pass`/`fail` observation's key ∈ `keys-executed.txt`. |
| V7 | Supersession graph over runs and over manual records is acyclic. |
| V8 | A `supersedes` reference resolves to a present run/record, and the key-set superset rule (§4.2) holds unless `supersede_mode = "partial"` with explicit `supersede_keys`. |
| V9 | `game-verified` in layer C requires a layer-B `game-acceptance` record; no other input can produce it. |
| V10 | Vocabularies (`outcome`, `result`, all four axes, `state`, `date_precision`) are closed; unknown tokens fail (fixes E9). |
| V11 | Every file under the manual root has an unchanged SHA-256 across any generator invocation. |
| V12 | A conversion-dependent operation (§2.2 R-CONV) without a `conversion` component fails. |
| **V13 (R2)** | Every `supersede_keys` entry is declared by **both** the superseding and the superseded run. |
| **V14 (R2)** | `supersede_mode = "full"` requires the superset rule; `"partial"` requires a non-empty `supersede_keys`; `"none"` requires an empty `supersedes`. |
| **V15 (R2)** | A `not-applicable` applicability carries a non-empty cited reason (R-APPL); an empty reason fails. |

**R-ACTIVE-STRICT.** V1–V12 are **hard errors for active inputs** — anything in `auto/runs/`,
`manual/`, and the published layer C. There are no warnings.

**R-IMPORT-SOFT.** Layer L and `candidates/` are **not** active inputs. A dangling reference or an
unmappable field there is represented explicitly as an `unresolved` token on the record and is
reported by `check`, not thrown as a validation error. This is how the R0 contradiction is resolved:
*malformed active input fails; an unreconciled historical import is a recorded state.* An orphan
reference in an **active** run is therefore V8, never "silently ignored".

### 5.2 Freshness inputs

Freshness is content-identity based. It is never date-based, never commit-based, and never
mtime-based.

**R-CONTENT.** The digest covers file **content** only. A touched file with identical bytes changes
nothing. A changed byte invalidates every record that declared the containing scope.

**R-FRESH-UNKNOWN.** If any freshness input is missing or unreadable, the digest is `unknown` and
freshness is `unknown` — which is treated as not-fresh everywhere (row 6 of §4.3). Missing inputs
never produce `fresh`.

### 5.3 Versioned scope definitions

`defs/qa/scopes.toml` — hand-authored, versioned, and itself part of every digest that references it
(synthetic example covering the first emitters):

```toml
schema = "qa-scopes/1"

[[scope]]
id = "shared-core"
version = 1
paths = ["crates/tdb-core/src/**", "crates/tdb-defs/src/**"]
config = ["defs/metadata/**", "defs/enums/**"]
toolchain = ["rustc-version", "target-triple"]

[[scope]]
id = "db-read"
version = 1
shared = ["shared-core"]
paths = ["crates/studio-domain/src/view.rs", "crates/studio-domain/src/surfaces.rs"]
schema = ["docs/audits/data/schema/**"]

[[scope]]
id = "db-edit"
version = 1
shared = ["shared-core"]
paths = ["crates/tdb-edit/src/**", "crates/studio-domain/src/session.rs"]

[[scope]]
id = "save-containers"
version = 1
shared = ["shared-core"]
paths = ["crates/tdb-savedata/src/**"]

[[scope]]
id = "headmesh-convert"
version = 1
shared = ["shared-core"]
paths = ["crates/tdb-headmesh/src/**", "crates/tdb-rx2/src/**"]
recipe = ["defs/appearance/headmesh-matrix.toml", "defs/appearance/head-uv-classes.toml", "defs/appearance/legacy-bone-names.toml"]
```

`implementation_digest` = SHA-256 over the canonical serialization of: `scope_id`, `scope_version`,
the sorted `(relative path, content sha256)` pairs from `paths` + `config` + `schema` + `recipe`, the
resolved digests of every `shared` scope, and the resolved `toolchain` identity strings. These
definitions are a **proposal for the first emitters**, not a user decision (§9 Q-D).

---

## 6. Conservative migration

Migration is a distinct command with a distinct output directory. It **never** writes layer B
(§7.2), never fabricates a receipt, and never invents a date.

### 6.1 Freeze the inputs first

**R-FREEZE.** Before any mapping, migration copies each legacy source byte-for-byte into
`docs/audits/evidence/legacy/frozen/` and records its SHA-256, **together with the
catalog/family ordering used to interpret it**:

```json
{
  "schema": "qa-legacy-freeze/1",
  "frozen_utc": "2026-09-08T10:00:00Z",
  "sources": {
    "game-qa-matrix.frozen.csv": "5f4e3d2c1b0a9988776655443322110ffeeddccbbaa99887766554433221100f",
    "capability.frozen.csv": "0011223344556677889900aabbccddeeff00112233445566778899aabbccddee",
    "asset-capability-matrix.frozen.csv": "99aabbccddeeff00112233445566778899aabbccddeeff001122334455667788"
  },
  "family_order": ["db-presentation","db-operations","save-raw-db","save-inner-container","save-outer-package","onboarding-install","onboarding-localization","assets-portraits","assets-logos","assets-jerseys","assets-equipment","assets-heads","assets-arenas","assets-audio"],
  "family_order_sha256": "c0ffee00112233445566778899aabbccddeeff00112233445566778899aabbcc",
  "unit_order": ["nhl07-xbox360","nhl07-ps3","nhl08-xbox360"]
}
```

**Regenerating from the live catalog is not a safe migration** and is prohibited: a later catalog
edit would silently re-interpret frozen rows. Migration reads the frozen CSV and the frozen ordering
only.

**R-POSITIONAL.** Key-syntax validation **cannot** detect a legacy row that was reordered within its
unit block — the row would still parse. Every migrated row therefore carries
`family_source = "positional-assumption"` and `confidence = "positional"`. Where a row's own content
contradicts its positional family (for example an `assets-*` slot whose evidence text describes save
containers), the row is emitted **without a family**, marked `unresolved = positional-conflict`, and
preserved in `_unmapped/`. No family is guessed.

### 6.2 Legacy labels are claims, not observations

**R-CLAIM.** `read-pass` / `edit-roundtrip-pass` / `discovered` / `not-applicable` in the frozen CSV
are **historical claims**. They migrate to layer L verbatim — raw status, raw `last_verified` string,
raw evidence and blocker text — and:

- no run manifest, receipt, `run_id` or `observations.csv` is created for them;
- no `performed_utc` is synthesized; `date_raw` keeps the source string and `date_precision = "day"`;
- they never contribute to `automated`, `manual` or `qualification`.

**R-HAND.** A hand-maintained CSV may be a human's transcription of an automated result.
`maintenance = "hand"` records **who maintained the file**, and asserts nothing about whether a human
executed anything. A hand-maintained cell never becomes a manual game-acceptance record.

### 6.3 Cell-level accounting

**R-ACCOUNT.** Every source cell — including every `not_run`, every blank and every `not-applicable` —
gets exactly one row in `docs/audits/evidence/legacy/migration-accounting.csv`. Nothing is dropped.

```csv
schema,source_file,source_row,source_column,source_value_raw,disposition,claim_id,unresolved
qa-migration-accounting/1,capability.frozen.csv,2,install,external,preserved-raw,cap-nhllegacy-xbox360-portraits-install,external-meaning-undefined
qa-migration-accounting/1,capability.frozen.csv,6,reimport_unchanged,not_run,preserved-no-evidence,cap-nhllegacy-xbox360-heads-reimport,none
qa-migration-accounting/1,capability.frozen.csv,2,format,BIGF->refpack->XPR2/RX2,preserved-raw,cap-nhllegacy-xbox360-portraits-install,variant-not-mapped
```

`disposition` ∈ `mapped | preserved-raw | preserved-no-evidence | unmapped | conflict`.
The accounting row count must equal the source cell count exactly (acceptance case A12).

### 6.4 Field-level rules

| Source | Rule |
|---|---|
| `format = "BIGF->refpack->XPR2/RX2"` | **Never collapsed.** Preserved verbatim in `legacy_format_raw`. A `variant` is assigned only from an explicit declared mapping table; absent one, `variant` is omitted and `unresolved = variant-not-mapped`. |
| `install = "external"` | Preserved raw with its column context. **No meaning is invented and no game-install qualification results** (§9 Q-A). |
| `not_run` | Preserved as `preserved-no-evidence`. It is neither a pass, a fail, nor a deletion. |
| `partial` | Preserved as `result = "partial"` + `unresolved`; never rounded either way. |
| a date inside free-text evidence | `date_raw` + `date_precision = "day"`. **A date in a note does not establish a UTC instant.** |
| headmesh `heads` row | Migrated at **destination granularity only**: `granularity = "destination"`, `unresolved = not-pair-specific`. It is **never** expanded into pair-specific keys (E11). |

### 6.5 Headmesh guard transition

`matrix_guard.rs` currently gates 15 pairs on one destination-level `heads` row. During transition:

1. the existing assertion keeps running unchanged;
2. a second assertion reads the layer-C row for the **pair-specific** key
   `<destination>/assets-heads/game-acceptance?from=…;class=…;path=…;recipe=…`;
3. because migration produces only destination-level claims, the pair-specific keys start at
   `unqualified` — this is expected, and it is **not** a regression: the coarse evidence never was
   pair-specific proof;
4. the old assertion is removed only in a separate, reviewed commit, once pair-specific records
   exist for the pairs that are actually promoted.

**R-NO-BACKFILL.** The destination-level claim must never be copied onto pair-specific keys.

---

## 7. Commands

### 7.1 Ordinary commands (never write layer B)

| Command | Reads | Writes | Exit |
|---|---|---|---|
| `qa-evidence check` | A, B, L, C | **nothing** | 0 ok · 2 drift vs published C · 3 malformed active input |
| `qa-evidence build --out <dir>` | A, B, L | `<dir>` only (outside the repo tree) | 0 ok |
| `qa-evidence publish --from <dir>` | `<dir>` | the layer-C set in `docs/audits/data/` (§8) | refuses unless `check <dir>` passes **and** input digests re-verify |
| `qa-evidence publish --recover` | journal + retained prior set | completes or rolls back one interrupted publication | — |
| `qa-evidence observe …` (library API for emitters) | — | one new run directory under `auto/runs/` | — |

Library surface: `tdb_qa::evidence::{key, observe, manual, legacy, merge, qualify, publish}`.
`manual` exposes **loading only**; no writer exists in it.

### 7.2 Migration commands (the only path into layer B)

| Command | Writes | Guard |
|---|---|---|
| `qa-evidence migrate --out evidence/candidates/<migration_id>` | candidates + accounting only | never touches `manual/` |
| `qa-evidence migrate-install --from <migration_id> --approve <approval.toml>` | adds records under `manual/` and `legacy/` | approval file must name the migration id and carry the candidate-set digest; any mismatch refuses |

```toml
schema = "qa-migration-approval/1"
migration_id = "mig-20260908T100000Z"
candidate_set_sha256 = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
approved_by = "synthetic-example"
approved_utc = "2026-09-08T11:00:00Z"
accepted_records = 47
accepted_claims = 392
```

**R-MIG-SEP.** `migrate-install` is the sole exception to R-SEP, it is never invoked by a test, a
build or CI, and it refuses without a matching approval digest. Original frozen sources are never
modified or deleted by it.

**R-DUP.** Neither `asset-capability-matrix.csv` nor `assets/**/capability.csv` wins by default.
Both are frozen and migrated; their differences are published to `asset-matrix-divergence.csv`.
Nothing is retired in C0.4 (§9 Q-B).

---

## 8. Publication and recovery

### 8.1 The generated output set

`build` emits, and `publish` installs, exactly:

| Path | Sidecar |
|---|---|
| `docs/audits/data/qa-evidence-matrix.csv` | `.provenance.json` |
| `docs/audits/data/qa-evidence-conflicts.csv` | `.provenance.json` |
| `docs/audits/data/qa-evidence-unresolved.csv` | `.provenance.json` |
| `docs/audits/data/asset-matrix-divergence.csv` | `.provenance.json` |

Each sidecar records the tool version, the merge input digests (run manifests, manual record
digests, legacy freeze digest), the scope definitions digest, and the output digest.

### 8.2 Multi-file publication on Windows

Separate renames are not a transaction. Publication is journalled:

1. **Prepare** — write the full new set into `docs/audits/data/.qa-publish-<id>/`; write
   `.qa-publish-journal.json` with `phase = "prepared"`, every target path, each target's current
   digest, and each staged file's digest.
2. **Re-verify** — recompute every merge input digest. Any mismatch aborts; nothing has moved.
3. **Retain** — move the current published set into `docs/audits/data/.qa-prev-<id>/`;
   `phase = "retained"`.
4. **Install** — move staged files into place one by one; `phase = "installing"` with a per-file
   completion list.
5. **Seal** — `phase = "completed"`; the staging directory is removed; `.qa-prev-<id>/` is retained
   (last 3 generations).

`publish --recover` reads the journal:

| Journal phase found | Recovery |
|---|---|
| `prepared` | delete staging; published set untouched |
| `retained` | restore from `.qa-prev-<id>/`; delete staging |
| `installing` | using the per-file list, either finish the remaining moves or restore all from `.qa-prev-<id>/` — the journal names which files are already installed, so the choice is deterministic |
| `completed` | nothing to do |

**R-NO-GIT.** `git checkout` is **not** a recovery mechanism and must never be suggested: this
repository routinely carries dozens of unrelated working-tree changes, and a checkout would destroy
them. Recovery uses the journal and the retained prior set only. Layer B is never involved, because
publication never touches it.

### 8.3 Transition of the existing generator write paths

**R-TRANSITION.** `qa-evidence publish` initially owns **only** the four new layer-C files. It does
not replace anything else. The existing write paths remain, unchanged and explicitly out of scope for
C0.4b:

| Existing path | Status after C0.4b |
|---|---|
| `game-qa-matrix.csv` (`qa-inventory`) | still generated as today; the new matrix is published **alongside** it. Retirement is a later, separate package after consumers migrate. |
| `anchors-manifest.csv`, `schema/`, `schema-diff.csv` (`qa-inventory`) | unchanged. **E12 (delete-and-rebuild of `schema/` with no journal) is recorded, not fixed here.** |
| the four `QA_WRITE=1` emitters | unchanged in C0.4b; they gain a parallel `observe` call that writes a run directory. Moving them off in-place CSV writes is a later bounded package (§9 Q-C). |

One new command does not replace all existing generator write paths, and this contract does not
claim it does.

---

## 9. Closed decisions

| # | Question | Decision |
|---|---|---|
| **Q-A** | Meaning of `install = external` | **Closed.** Preserve the raw value and its column context. It confers **no game-install qualification**. No meaning is invented. |
| **Q-B** | The two duplicate asset matrices | **Closed.** Preserve both, freeze both, publish their differences to `asset-matrix-divergence.csv`. **No retirement in C0.4**; neither wins by default. |
| **Q-C** | The `QA_WRITE=1` emitters | **Closed.** Transition to explicit `build`/`publish` in bounded later packages. C0.4b only adds a parallel `observe` run-directory write. |
| **Q-D** | Freshness scopes | **Closed as a proposal, not a user decision.** §5.3 proposes versioned scope definitions for the first emitters, with shared dependencies. Refinement is ordinary implementation work. |
| **Q-E** | `qa-ledger.md` line 10 ("no numbers are hand-maintained") | **Closed.** Correct it during **C0.4c**, with the migration. |

---

## 10. Acceptance cases

| # | Case | Asserts |
|---|---|---|
| A1 | `build` twice from identical inputs → byte-identical output set | determinism |
| A2 | SHA-256 of every file under `manual/` unchanged after `build`, `publish`, `migrate` and a full `cargo test` | R-SEP / V11 |
| A3 | Shuffle layer-A row order and layer-B file order → identical C | R-ORDER |
| A4 | Delete a fixture, re-run → key becomes `skipped-missing-fixture`; no prior verification date persists | E3/E5 |
| A5 | Run the merge with no emitter run present → no key is `automated-qualified`; **prior completed runs still selected** | R-RUN / R-ABSENT |
| A6 | Manual `pass` + current automated `fail` on one key | `qualification = blocked`, both cited (§4.3 row 4) |
| A7 | Touch a scope file without changing bytes → still fresh; change one byte → exactly the declaring records stale | R-CONTENT |
| A8 | `skipped-missing-fixture` observation | never yields `automated-qualified` |
| A9 | PS3 NHL 07 `save-raw-db` | `automated = not-attempted` and the legacy claim cited (E4). **(R2)** `applicability` is `not-applicable` with the cited reason `research-only-unit`, so `qualification = not-applicable`, **not** `unqualified` — the unit is not a shipping target (roadmap §1). The R1 expectation of `unqualified` assumed the blanket required-set R-APPL-OPS replaces. Either way the `edit-roundtrip-pass` claim is refuted and never reproduced. |
| A10 | `save-outer-package` on PS3/PS4 | never `not-applicable` (R-Q04) |
| A11 | 15 headmesh pairs | old guard still passes; pair-specific keys exist and are `unqualified`; no destination claim back-filled (R-NO-BACKFILL) |
| A12 | Migration accounting | row count == source cell count exactly, including every `not_run` and blank (R-ACCOUNT) |
| A13 | Malformed **active** record | `check` exits 3, names file and line; C not rewritten |
| A14 | Malformed **legacy/candidate** record | `check` exits 0 with an `unresolved` row; not a hard error (R-IMPORT-SOFT) |
| A15 | Operation-level `pass` | never raises family or target roll-up past §4.4 |
| A16 | Run directory with no `state.json` | classified `interrupted`; never selected; observations retained and citable |
| A17 | Two completed non-superseded runs disagreeing on a key | both retained and cited; `withheld`, `conflict = automated-disagreement` |
| A18 | Supersession cycle; `supersedes` to a missing active run; duplicate `record_id` | each a distinct hard validation error (V7, V8, V2) |
| A19 | Partial supersession without `supersede_mode = "partial"` | hard error V8 |
| A20 | Two independent cases on one key in one run | both retained; uniqueness is `(run_id, case_id, key)` (R-MULTI) |
| A21 | Two headmesh pairs sharing a destination | distinct keys; a pass on one leaves the other `unqualified` (R-CONV) |
| A22 | `nhl24` | appears only as `conversion.from`; never as a coverage `target` (R-SOURCE-ONLY) |
| A23 | Manual `pass` on `install` with no `game-acceptance` record | `game-verified` is **not** produced (R-GAME) |
| A24 | `game-acceptance` record with `runtime_build = "unknown"` | `withheld`, `reason = unknown-identity` (R-IDENTITY) |
| A25 | Migration run against a **changed** catalog | refuses; the frozen `family_order_sha256` no longer matches (R-FREEZE) |
| A26 | A legacy row reordered inside its unit block | migration cannot detect it; every migrated row carries `confidence = "positional"`, and content-contradicting rows land in `_unmapped/` (R-POSITIONAL) |
| A27 | Migration output | contains **no** run manifest, receipt or synthesized `performed_utc` (R-CLAIM) |
| A28 | `migrate-install` without a matching approval digest | refuses; `manual/` unchanged (R-MIG-SEP) |
| A29 | `publish` interrupted at each journal phase | `publish --recover` restores or completes deterministically; no `git` command involved; `manual/` untouched (§8.2) |
| A30 | `publish` when a merge input digest changed after `build` | refuses before moving anything (§8.2 step 2) |
| A31 | `format = "BIGF->refpack->XPR2/RX2"` | preserved verbatim; no `variant` invented (§6.4) |
| A32 | A date appearing only in free-text evidence | `date_precision = "day"`; no UTC instant produced (§6.4) |

### 10.1 R2 acceptance cases

| # | Case | Asserts |
|---|---|---|
| A33 | A `failed` run naming a completed run in `supersedes` | the completed run stays active and selected; the failed run's supersession is recorded and reported, never applied (R-SUPERSEDE-STATE) |
| A34 | An `interrupted` run (no `state.json`) naming a completed run in `supersedes` | identical to A33 — no erasure by an interrupted replacement |
| A35 | `supersede_mode = "partial"` with one key listed | the superseded run is suppressed **only** at that key; every other key it declares stays selected (R-PARTIAL-KEYS) |
| A36 | `supersede_keys` naming a key the superseding run does not declare | hard error V13 |
| A37 | Executed `fail` in an `interrupted` run, no other run declares the key | `automated = executed-unselected`; `qualification = withheld`, `reason = unreviewed-failure`; the observation is cited; **not** `not-attempted` (§12.3) |
| A38 | Executed `fail` in a `failed` run **plus** a completed passing run that does not supersede it | `withheld`, `reason = unreviewed-failure`, both cited — a passing run cannot silently bury an executed failure |
| A39 | Same as A38 but the completed run supersedes the failed run at that key | `automated-qualified`; the failure is retained and still cited as history |
| A40 | `db-presentation/read`, `onboarding-install/discover` | `game-acceptance` is not in the required set; the roll-up can reach `automated-qualified` without any manual record (R-APPL-OPS) |
| A41 | Every `write`/`acceptance` operation on the research-only unit `nhl07-ps3` | `applicability = not-applicable` with the cited reason `research-only-unit`; read-only operations are unaffected; no shipping target is altered |
| A42 | Two disagreeing completed runs whose `started_utc` differ | still `withheld`, `conflict = automated-disagreement`; the later timestamp wins nothing (R-NO-TIME) |
| A43 | An emitter contributes no run in this merge | every prior citation, conflict and `unresolved` token for its keys is still published unchanged (R-CITE-ALL) |
| A44 | Two headmesh pairs sharing a destination, one passing | per-pair cells; the family roll-up for that destination is **not** raised, and neither pair's evidence is copied onto the other (R-CONV-PAIR) |
| A45 | A `not-applicable` applicability with an empty reason | hard error V15 |

---

## 11. Implementation split

| Stage | Contents | Gate |
|---|---|---|
| **S1 — pure core** (C0.4b-1) | key parse/serialize, schema types, validation V1–V12, selection, the §4.3/§4.4 decision tables, freshness digest computation. Pure functions over in-memory inputs; no file writing, no CLI, no CI change. | A1, A3, A5–A10, A13–A24 run as unit tests over synthetic fixtures |
| **S2 — emitters and publication** (C0.4b-2) | `observe` run-directory writer, `check`/`build`/`publish`/`publish --recover`, provenance sidecars, journal. Emitters gain a parallel `observe` call; no existing write path is removed. | A2, A29, A30; a real run directory validates |
| **S3 — migration** (C0.4c) | freeze, `migrate`, accounting, `migrate-install`, dual headmesh assertion, `qa-ledger.md` correction. | A11, A12, A25–A28, A31, A32 |

**R-NO-CI.** No CI change is proposed until S2 has landed and `qa-evidence check` has been run and
validated against real inputs. Adding a CI gate for a command whose input contract is not yet
implemented would only produce a failing job.

---

## 12. R2 — contradictions found and how they are resolved

R1 left eight defects that would have destroyed or hidden evidence once implemented. Each is now
resolved by a named rule. No closed decision (§9) is reopened, no manual evidence is affected, and
no validation rule is relaxed — V13, V14 and V15 are added, none is removed.

| # | R1 contradiction | R2 rule |
|---|---|---|
| **X8** | §4.2 excluded a run from the active set when it was "named in **any present run's** `supersedes`". A `failed` or `interrupted` replacement run therefore **erased** the completed run it meant to replace, leaving the key with no evidence at all — the exact opposite of R-ABSENT and of "failed runs are retained as history". | **R-SUPERSEDE-STATE** (§12.1) |
| **X9** | §4.2 defined `supersede_mode = "partial"` and `supersede_keys` but never said what partial supersession *does*. Implemented literally, a partial supersession removed the superseded run entirely, discarding evidence for keys the superseding run never covered. | **R-PARTIAL-KEYS** (§12.2) |
| **X10** | "only `failed`/`interrupted` runs declare the key ⇒ `automated = not-attempted`" collapsed *an executed failure* into *never attempted*, and §4.3's table then let a passing run from another emitter qualify the key with the failure invisible. R1 simultaneously claimed those observations are "retained, readable and citable". | **R-EXECUTED-FAIL** (§12.3) |
| **X11** | `run_id` embeds a UTC basic timestamp and manifests carry `started_utc`/`ended_utc`, while R-NOWIN forbade recency selection — with no statement that the implementation must not read them. | **R-NO-TIME** (§12.4) |
| **X12** | R-ABSENT said a missing emitter "erases nothing", but §8.1's published set is rebuilt wholesale from the current merge, so a key whose only citations came from an emitter absent this run would publish with empty citation columns. | **R-CITE-ALL** (§12.5) |
| **X13** | §4.1 required `game-acceptance` for `db-operations`, all three save families and every `assets-*` family on **every** catalog unit — including the read-only `container-detect`/`discover`/`preview`/`export` operations and the research-only `nhl07-ps3` unit that the roadmap explicitly excludes from shipping scope. This blanket requirement was also the substance of open issue **U1**. | **R-APPL-OPS** (§12.7); U1 closed in §12.8 |
| **X14** | R-CONV made pair keys distinct, but §4.4's roll-up scope is "a family for a target", which re-collapses every conversion pair for that destination into one cell — reintroducing E11 one level up. | **R-CONV-PAIR** (§12.6) |
| **X15** | R-APPL required `not-applicable` to carry "a positive cited reason", but no validation rule enforced it, so an uncited `not-applicable` would pass V1–V12 and silently suppress a required cell. | **V15** |
| **X16** | §3's decisions table resolved *manual pass + automated fail* to `blocked` **plus** a conflict token, while §4.3 row 3 withholds on *any* conflict — so the resolved case could never reach row 4, contradicting acceptance case A9's sibling A6, which requires `blocked`. | **R-CONFLICT-CLASS** (§12.9) |

### 12.1 R-SUPERSEDE-STATE — only a completed run can supersede

A `supersedes` list is honoured **only** when the run declaring it has `state = "completed"`. A
`failed` or `interrupted` run's `supersedes` and `supersede_keys` are parsed, validated (V7, V8,
V13, V14) and **reported**, and have no effect on any other run's selectability.

The same rule applies to layer B: a manual record supersedes another only when it is itself a
well-formed, present, active record, and only for the coverage key both share. A record never
supersedes a record for a different key.

*Rationale.* A replacement run that crashes half-way is the single most likely way to lose evidence
in practice. Under R1 it would have removed the prior run and contributed nothing in its place.

### 12.2 R-PARTIAL-KEYS — supersession is a per-key operation

Supersession suppresses the superseded run **at specific keys only**:

| `supersede_mode` | Suppressed key set |
|---|---|
| `"none"` | none; `supersedes` must be empty (V14) |
| `"full"` | every key the **superseded** run declares — which the superset rule (V8) guarantees the superseding run also declares |
| `"partial"` | exactly the keys listed in `supersede_keys`, each of which must be declared by **both** runs (V13) |

Outside the suppressed set the superseded run stays active and its observations stay selected and
cited. Supersession never deletes a run directory, never removes an observation, and never removes a
citation.

### 12.3 R-EXECUTED-FAIL — an executed failure stays visible

An observation is **executed** when its `outcome` is one of `pass`, `fail` or `refused-clean`.
`skipped-missing-fixture`, `not-attempted` and absence are **not** executed.

1. If a key's only observations come from non-`completed` runs and at least one is executed, the
   axis value is `executed-unselected` — never `not-attempted`. Nothing is selected and nothing
   qualifies from it.
2. An executed `fail` observation is **uncleared** unless some **`completed`** run that supersedes
   the observing run *at that key* (§12.2) itself executed that key with `outcome = pass`.
3. An uncleared executed failure sets `unresolved = uncleared-executed-failure`, cites the observing
   run and case, and triggers qualification row **4a** (`withheld`, `reason = unreviewed-failure`)
   — **regardless of the observing run's terminal state**, and regardless of any passing run that
   did not explicitly supersede it.
4. Clearing a failure never deletes it. The cleared observation remains in its immutable run
   directory and remains cited as history on the published row.

This is what makes an unexecuted case and an executed failing observation different values with
different consequences, and it is why a crashed or abandoned run cannot be used — deliberately or
accidentally — to make a real failure disappear.

### 12.4 R-NO-TIME — no timestamp participates in any decision

Selection, supersession, conflict handling, qualification and roll-up read **no** timestamp, no file
mtime, no directory-iteration order and no filename ordering. `started_utc`, `ended_utc`,
`entered_utc`, `frozen_utc`, `approved_utc` and the timestamp embedded in a `run_id` are recorded
metadata, reported verbatim, and never compared to choose a winner. Determinism comes from key
sorting (R-ORDER) alone.

### 12.5 R-CITE-ALL — citations and conflicts survive an absent emitter

Every published row lists **every** input that mentions its key — selected or not — with the state
that made it selectable or not: active completed runs, `failed`/`interrupted` runs, superseded runs,
active and superseded manual records, and legacy claims. A conflict token, once produced by inputs
that are still present, is republished on every subsequent merge until those inputs change. An
emitter contributing no run in a given merge changes nothing about its earlier runs' citations,
conflicts or `unresolved` tokens.

### 12.6 R-CONV-PAIR — a roll-up never crosses a conversion identity

For conversion-dependent operations the roll-up scope is `(target, family, conversion)`, not
`(target, family)`. Every distinct conversion identity is its own required cell, enumerated from the
declared recipe registry rather than from whatever evidence happens to exist. Source and destination
are both preserved: the key's `target` is the destination and `conversion.from` is the source. A
family roll-up for such a family is the §4.4 table applied over the **per-pair** cells, so a pass on
one pair cannot raise the family, and no pair's evidence is ever read as another's.

### 12.7 R-APPL-OPS — required evidence follows approved applicability

Every operation carries a class: `read-only`, `write` or `acceptance` (§4.1, `Class of each`).
Applicability of a cell `(target, family, operation)` resolves in this order:

1. If the target's catalog unit is `UnitKind::Research` — today only `nhl07-ps3` — every `write` and
   `acceptance` operation is `applicability = not-applicable`, reason `research-only-unit`, citing
   the roadmap §1 decision *"NHL 07 PS3 is research-only, not a shipping target."* Read-only
   operations are unaffected and remain required exactly as declared.
2. `game-acceptance` is required for a family on a target **only if** that family has at least one
   **required `write`** operation there. A family whose required set is entirely read-only —
   `db-presentation`, `onboarding-*` — never requires game acceptance.
3. `save-outer-package` remains required on Xbox 360, PS3 and PS4 under R-Q04. R2 does not weaken it.
4. Otherwise the declared required set of §4.1 applies unchanged.

This narrows nothing that an approved document required. It removes only the requirement R1 invented
by writing "all" across families and units the roadmap had already scoped differently. It does **not**
soften game acceptance for write paths: the roadmap's *"including game acceptance for writes"* and
Q02 are preserved exactly.

### 12.8 U1, closed

**U1** asked whether requiring `game-acceptance` for every `assets-*` family on every target is
intended reporting or a definition to soften. **Closed by R-APPL-OPS**, and only in the part that
approved guidance already settles: game acceptance stays required for every family with a required
write operation on every **shipping** target, so asset families do remain `unqualified` until the
manual campaigns run — that is intended under Q02 and is not softened. What is removed is the part
R1 added without authority: the requirement on read-only families and on the research-only unit.

### 12.9 R-CONFLICT-CLASS — two kinds of conflict, only one of which withholds

A conflict **within** one layer — `automated-disagreement`, `manual-disagreement` — has no defined
resolution, so it withholds (§4.3 row 3). A conflict **between** layers whose resolution the contract
already states — `manual-pass-automated-fail`, resolved to `blocked` by row 4 — is recorded as a
conflict token, published, and resolved by the row that defines it. Row 3 therefore tests only the
first class. Both classes are always published in the `conflict` column with all citations; the
distinction changes which row decides, never what is reported.

**U2** (the `conversion.class` vocabulary) and **U3** (whether layer-A run directories are committed)
are **not** decided here. U3 blocks nothing in S1–S3: every command takes its evidence root as an
argument, and no run directory is written into the repository by this implementation.
