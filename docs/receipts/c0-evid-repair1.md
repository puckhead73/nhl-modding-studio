# C0-EVID-REPAIR1 — self-contained migration tests and portable drift guards

**Package:** C0-EVID-REPAIR1 · **Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` ·
**HEAD at start and at end:** `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` (unchanged) ·
**Index at start and at end:** empty
**Status: SUPERSEDED IN PART** — see the review correction below.

## ⚠ Review correction (C0-EVID-REPAIR1-R1, 2026-09-09)

Two claims below are **withdrawn**; see
[c0-evid-repair1-r1-explicit-integration-coverage.md](c0-evid-repair1-r1-explicit-integration-coverage.md) §1.

1. **"Not a skip" is withdrawn (§1, §3.2, §3.4).** The real-registry case was a **conditional
   pass**: with the registry absent it asserted only the absence and returned green, which is worse
   than a skip because an unmet requirement reported as `ok`. It is now `#[ignore]`d with the
   prerequisite in its reason, and fails when explicitly selected against an absent, unreadable or
   empty registry.
2. **"223 passed · 0 failed" is withdrawn as stated (§1, §6.2).** That total counted the conditional
   pass, implying real-registry integration had **executed**; it had not. The corrected figure is
   **222 passed · 0 failed · 1 ignored**, in both representations.

Everything else in this receipt stands: the synthetic A11 and divergence cases, the six mutations,
the Checkpoint-B comparison repair, the merge distribution, the clippy and formatting results and
the preservation figures are unaffected. An ignored integration test is an **explicit coverage
gap**, not a failed invariant.

> **Result: both checkpoints closed.** The two migration cases are self-contained at HEAD, the eight
> drift guards are newline-independent, and the revised candidate is **223 passed / 0 failed** in
> **both** the CRLF and the all-LF representation — where the same slice was 209/6 in C0-EVID-PROBE1.
>
> Nothing staged, committed or pushed. No live publication, no migration installation, no CI change,
> no schema or product change, no new dependency, no feature import, no game-data access, no
> live-tree build or test, and no prior evidence deleted. **`QA_WRITE` was never set**, and **no
> committed CSV was rewritten** (12 hashed before and after every run). The failed C0-EVID-PROBE1
> candidate is retained unchanged.

**Dependency status.** C0-EVID-PROBE1 is treated as **diagnostic evidence only**. Its §4.1 and §4.2
findings are what this package repairs; none of its figures are carried forward as verification.

---

## 1. Result in one paragraph

Both defects C0-EVID-PROBE1 diagnosed are closed at their root, and neither repair weakens what the
tests assert. **Checkpoint A**: `evidence_migration`'s two failing cases no longer read inputs that
HEAD does not carry — a11 now builds a **labelled synthetic recipe registry** (five pairs, two
deliberately sharing one destination) and drives it through the **real** `migrate::headmesh_pair_keys`
path, and the duplicate-matrix case builds **labelled synthetic source matrices with a known
disagreement** and drives them through the **real** freeze → migrate → divergence path. The live
hand-edited `capability.csv` is neither read for that case nor modified, and real-registry
integration is recorded as **pending coverage that executes if the registry is present** rather than
deleted or silently skipped. **Checkpoint B**: the eight guards compare **records**, not bytes,
through one shared helper (`evidence::csv::compare_records`) whose separator/payload distinction is
the CSV parser's — so a CR or CRLF **inside a quoted field stays payload** and still compares
unequal, which a naive substitution would hide. The one JSON guard is handled by a restricted-format
argument that is **tested, not asserted**. Six deliberate mutations prove every new guard
load-bearing, including one that reintroduces the naive substitution and fails exactly the
quoted-field case and nothing else. The pre-existing `inventory` clippy diagnostic is gone and
strict clippy exits **0**. Post-repair capture reproduces the merge exactly — 4 completed runs,
deterministic double build, `ok: 2175 row(s)`, **140 `automated-qualified`, 0 `game-verified`**, 159
rows still `withheld`, and `headmesh-convert` still **`unknown`** with its scope unshrunk.

---

## 2. Changed paths

**Live edits — 7 files, all tests plus one shared comparison helper. Nothing else was touched.**

| Path | Change | Checkpoint |
|---|---|---|
| `crates/tdb-qa/src/evidence/csv.rs` | **+`Drift` + `compare_records`** — the shared record-level comparison, plus 5 new unit tests. No existing function changed | B (the "narrowly necessary shared comparison helper") |
| `crates/tdb-qa/tests/inventory.rs` | 4 guards rewired through `csv_matches_committed` / `json_matches_committed`; +1 restricted-format test; module doc corrected (the clippy fix) | B |
| `crates/tdb-qa/tests/evidence_migration.rs` | a11 rewritten over a synthetic registry; duplicate-divergence rewritten over synthetic sources; +2 cases (`real_headmesh_registry_integration_is_pending_separate_coverage`, `the_committed_asset_matrices_still_migrate_and_neither_is_retired`); 3 synthetic builders | A |
| `crates/studio-domain/tests/{mutation,operation,save,surface}_matrix.rs` | drift assertion → `compare_records`, one call site each | B |

**Deliberately not changed:** `migrate.rs` and every other production module (§8 item 1), the
committed CSVs and JSON snapshots, `defs/qa/scopes.toml`, the emitter observation lifecycle, and
`docs/audits/**`. `crates/tdb-qa/tests/inventory.rs` enters the evidence slice for the first time
(C0-EVID-PROBE1 excluded it and used HEAD's copy); it carries its pre-existing C0-INT1 rustfmt
reflow with it, disclosed here as C0-SLICE1 disclosed the same for the four emitters.

**Live hashes after the work** (`logs\80-live-changed-after.sha256`):

| File | SHA-256 |
|---|---|
| `crates/tdb-qa/src/evidence/csv.rs` | `1c807dfa335de0fc289a22a9bcb75223593698a5e653e99feefbd063a16d773d` |
| `crates/tdb-qa/tests/inventory.rs` | `00c40ee0fd4cdcb4ebb5e1708768a82a7dbd3ffe65195eb90551c625146b50e5` |
| `crates/tdb-qa/tests/evidence_migration.rs` | `22abf6d7d362b960ee4d2c588ee3a77515af2abb71a27a3383543507ea1fc21d` |
| `crates/studio-domain/tests/mutation_matrix.rs` | `60952ea3df98fc1f2b84ebd654442ef825aecd56a4bbed8e183898cfd4c3de72` |
| `crates/studio-domain/tests/operation_matrix.rs` | `60bb736d92d9eb0a09affe06f3d6110cc703e2b463d3288160a3524ef0405d5c` |
| `crates/studio-domain/tests/save_matrix.rs` | `8a0896ed4edb686ee6e960027cafce3d21a0e3a1d0a823c530763d57d4c3fe89` |
| `crates/studio-domain/tests/surface_matrix.rs` | `4f7fbf1e7a254f7ca6a7ee2c982290a0a967606094fe99e29ae3d45dc2a83b7a` |

---

## 3. Checkpoint A — self-contained migration invariants

### 3.1 a11 — pair identity over a synthetic registry

`synthetic_headmesh_registry` writes a registry into test scratch whose **bytes carry their own
label** (`# SYNTHETIC — built by evidence_migration.rs. Not a copy of any registry and not evidence
about any real conversion pair.`). It declares **5 pairs across 3 destinations**, with **two pairs
sharing one destination** and differing only by conversion — the exact shape R-NO-BACKFILL exists
for. The shared destination is spelled `nhllegacy-xbox360` because `migrate` assigns that target to
`capability.frozen.csv` rows; **the pairs, sources, schemes and counts are invented.**

The **real** path is exercised end to end: `migrate::headmesh_pair_keys` parses the registry, then
`migrate::freeze` → `migrate::migrate` → `approve_and_install` → `store::load_legacy` →
`merge::merge`. What it asserts:

| Assertion | Why it is the invariant, not the data |
|---|---|
| 5 keys over 3 destinations, 2 on the shared one | measured from the file the test just wrote, and labelled as such |
| every key distinct; every key carries a `conversion` | pair identity does not collapse |
| the two shared-destination keys have **distinct conversions** | they are separated by conversion, not merged into the destination |
| the destination-level `heads` claim **is present** in layer L, carries `granularity-destination` **and** `not-pair-specific`, and `!qualifies()` | the claim exists and states its own limits — the back-fill has something to back-fill *from* |
| every pair row: `automated = NotAttempted`, `manual = None`, **not** `GameVerified`, **not** `AutomatedQualified` | the negative qualification assertions are preserved, and the last one is new |

**No real pair count is presented as integration coverage.** C0-EVID-PROBE1 §4.1's version asserted
`14 pairs / 8 destinations / 6 shared` read from the uncommitted registry; those numbers are gone,
not relocated.

### 3.2 Real-registry integration — recorded as pending, and it executes when possible

`real_headmesh_registry_integration_is_pending_separate_coverage` is **not a skip**. If
`defs/appearance/headmesh-matrix.toml` is present it parses it through the real path and asserts
real pair keys are distinct and carry conversions; if it is absent it asserts the absence explicitly
and documents what the case becomes when the registry lands. At HEAD the registry is absent, so the
requirement is **recorded and visibly unmet** rather than deleted, and the synthetic case above does
**not** claim to replace it.

### 3.3 Duplicate-matrix divergence — a known disagreement, synthetic sources

`synthetic_source_root` builds a repository-shaped directory holding the three migration sources:
the **committed** `game-qa-matrix.csv` is copied in unchanged (it is self-contained at HEAD, and the
positional accounting it drives is meaningless against invented rows), and the two asset matrices
are synthetic, labelled inside their own cells (`SYNTHETIC …`), and carry **two deliberate
disagreements**: `heads` exists only on the left, and `equipment/identify` is `partial` on the left
and `pass` on the right. Real `freeze` → `migrate` → `divergence` then runs over them.

| Assertion | Result |
|---|---|
| the left-only `heads` row is published with **both** provenances | `left_value = present`, `right_value = absent`, `unresolved = duplicate-source-row-missing` |
| the value disagreement survives with **both claims intact and attributed** | `partial` vs `pass`, each with its own `left_source`/`right_source`, `unresolved = duplicate-source` |
| agreeing cells are **not** reported as divergence | `portraits/discover` absent from the output |
| neither source is retired | both frozen copies present; `verify_freeze` passes |
| both sides survive as layer-L claims with provenance | one `equipment` claim per source file, each with a non-empty `source_sha256`, a real `source_row`, and `!qualifies()` |

**The live `capability.csv` is not read by this case, not copied, and not modified** — its
pre-existing hand edit is left exactly as found (§7). A second, new case
(`the_committed_asset_matrices_still_migrate_and_neither_is_retired`) keeps the **committed** files
under test for their *shape* — that they migrate and that neither is retired — which is
self-contained because it does not depend on a disagreement a hand edit can add or remove.

### 3.4 Synthetic vs real coverage, stated separately

| Case | Inputs | What it proves |
|---|---|---|
| `a11_pair_keys_stay_pair_specific_and_no_destination_claim_is_back_filled` | **synthetic** registry + **synthetic** asset matrices + **committed** qa-matrix | the R-NO-BACKFILL / pair-identity invariant |
| `real_headmesh_registry_integration_is_pending_separate_coverage` | the real registry **when present** | **pending** — unmet at this commit, recorded not skipped |
| `both_duplicate_asset_matrices_…` | **synthetic** sources with a known disagreement | R-DUP: both claims, provenances and the disagreement survive |
| `the_committed_asset_matrices_still_migrate_and_neither_is_retired` | **committed** sources | the real files still migrate; neither retired |
| `a12`, `a25`, `a27`, `a28`, `a31`, `a32`, `external_and_not_run_and_partial…` | **committed** sources, unchanged | kept as they were — still self-contained |

---

## 4. Checkpoint B — newline-independent drift checks

### 4.1 The helper, and why it cannot hide payload

`evidence::csv::compare_records(generated, stored)` returns `Ok(())` or a `Drift` naming the
offending record. It does **no** normalisation: identical bytes short-circuit, otherwise both sides
are parsed with the module's existing RFC-4180 reader and compared **record by record, field by
field, in order**.

The separator/payload distinction is the **parser's**, not a string substitution's:

| Input | Parsed | Consequence |
|---|---|---|
| `a,b\r\nc,d\r\n` vs `a,b\nc,d\n` | same 2 records | **equivalent** — the intended repair |
| `note,"line1\r\nline2"` vs `note,"line1\nline2"` | field values `line1\r\nline2` vs `line1\nline2` | **different** — quoted-field payload is preserved |
| `a,"b\rc"` | field value `b\rc` | lone CR inside quotes is payload |
| `a,b\rc` (unquoted) | parse error | a bare CR outside quotes stays an error, on whichever side carries it |

Field whitespace, record order, record count and every value are compared verbatim; **nothing is
trimmed, sorted, rounded or normalised**, and no stored file is rewritten. A trailing terminator on
the last record (or its absence) is not a difference, on either side.

### 4.2 The one JSON guard — a restricted-format argument, tested

`schema/*.json` snapshots are compared by `json_matches_committed`, which asserts the generator
emitted **no CR at all** and then compares against the stored text with `\r\n` → `\n`. That
substitution is safe **because JSON is restricted here**: RFC 8259 forbids a raw CR inside a string
(it must be `\r`), so the only CR a snapshot can carry is half a checkout's terminator.
`json_escapes_cr_so_a_raw_cr_can_only_be_a_terminator` pins **both halves** — that `serde_json`
escapes a CR in the data, and that `schema::to_json` of a real snapshot contains no CR. If either
ever stops holding, that test fails before the helper can hide anything.

### 4.3 The eight guards, and only those eight

`inventory::{anchor_manifest,schema_snapshots,schema_diff,qa_matrix}_matches_committed` and the four
`*_matrix` emitters. A repository-wide grep confirms **exactly 8 call sites** across 5 test files —
no other comparison in the tree was touched. `qa_matrix_is_well_formed`, the schema-directory
membership check and every other assertion in those files are unchanged.

**Evidence hashes and provenance bytes are not normalised anywhere**: the change is confined to the
five test files plus the new helper, and no digest, manifest or sidecar path was touched.

### 4.4 The emitter failure path is unchanged

The new guard panics **without** calling `ev.finish()`, exactly as the previous `assert_eq!` did, so
the drop guard still seals a drift-failing run `failed` and its observations still qualify nothing.
This was written, reviewed and corrected during the work — an earlier draft sealed such a run
`completed`, which would have let a failing run's observations qualify; it was removed before any
verification run.

### 4.5 The clippy fix

The single pre-existing diagnostic (`doc_lazy_continuation`, `tests/inventory.rs:7`) is resolved by
correcting that module doc's own structure while rewriting it. **No `#[allow]` was added anywhere,
and no unrelated lint or file was touched.** Strict clippy over the whole package now exits **0**.

---

## 5. Load-bearing regressions — six deliberate mutations

Each was applied in the scratch candidate, executed, then reverted; the tree hashes identical
afterwards (§7).

| # | Mutation | Expected | Observed |
|---|---|---|---|
| **M1** | `compare_records` short-circuits on `replace("\r\n","\n")` — the naive fix | only the quoted-field case fails | **8 passed, 1 FAILED**: exactly `a_crlf_inside_a_quoted_field_is_payload_and_still_compares_unequal`. Every other CSV case still passed, so the guard is precise, not merely strict |
| **M2** | one value in the committed `anchors-manifest.csv` changed (`main-database` → `MAIN-database`) | the inventory guard fails | **FAILED**, reporting `record 3 differs` |
| **M3** | one state in the committed `surface-capabilities.csv` changed (`editable` → `readable`) | the emitter guard fails | **FAILED**, `surface-capabilities.csv drift — regenerate with …`, `record 2 differs` |
| **M4** | two synthetic pairs given identical source+path, collapsing their keys | a11's distinctness assertion fires | **FAILED**: `no two pairs share a key` |
| **M5** | `LegacyClaim::qualifies()` returns `true` — R-LEGACY defeated | a11 catches the back-fill | **FAILED**: `R-LEGACY` |
| **M6** | the synthetic sources changed to **agree** on `equipment/identify` | the divergence case fires | **FAILED**: `the deliberate equipment/identify disagreement` |

M1 is the one that matters most: it shows the shared helper is not merely a renamed substitution.
M2/M3 show genuine content drift still fails in both guard families. M4–M6 show the new synthetic
cases assert their invariants rather than their fixtures.

---

## 6. Checkpoint C — isolated verification

### 6.1 Candidates

| | |
|---|---|
| Scratch root | `H:\c0-evid-repair1\` — confirmed absent before creation |
| Base | `git archive` of `ef0fdd7`, archive SHA-256 `e35186907d6e8aae24a83dd870b2a2ea0434236099de507557f3a5c0babf95cd`, **1160 files** |
| Overlay | **34 enumerated files** (C0-EVID-PROBE1's 33 + `crates/tdb-qa/tests/inventory.rs`), placed individually from a manifest |
| Lockfile | the same **two C0 lines** (`studio-domain` += `tdb-qa`, `tdb-qa` += `toml 0.8.2`); **no new dependency** |
| `candidate-crlf\` | overlay written CRLF — what a commit checks out on Windows. **1186 files** |
| `candidate-lf\` | the same overlay, with the 41 committed generated CSV/JSON artefacts converted to LF |
| Target dirs | **distinct per tree**: `target-crlf`, `target-lf`, `target-clippy`, `target-mut` — the C0-EVID-PROBE1 §4.5 hazard, avoided by construction |
| Retained | `H:\c0-evid-probe1\` — **21 227 files, not written by this package**; the failed probe candidate is unchanged |

### 6.2 Checks

| # | Command | Tree | Exit | Result |
|---|---|---|---|---|
| 1 | `cargo test -p tdb-qa --locked --offline --no-fail-fast` | CRLF | **0** | **223 passed · 0 failed** (probe: 209/6) |
| 2 | `cargo test -p tdb-qa --locked --offline --no-fail-fast` | **LF** | **0** | **223 passed · 0 failed** — identical |
| 3 | `cargo clippy -p tdb-qa --all-targets --locked --offline -- -D warnings` | CRLF | **0** | **zero diagnostics** |
| 4 | `cargo test -p studio-domain --test surface_matrix` | CRLF | **0** | passed (8.4 s) |
| 5 | `cargo test -p studio-domain --test mutation_matrix` | CRLF | **0** | passed (152.8 s) |
| 6 | `cargo test -p studio-domain --test operation_matrix` | CRLF | **0** | passed (490.0 s) |
| 7 | `cargo test -p studio-domain --test save_matrix` | CRLF | **0** | passed (93.3 s) |
| 8 | `surface_matrix` + `save_matrix` | **LF** | **0** | passed (6.0 s / 77.8 s) |
| 9 | `rustfmt --check` on all 7 changed files | live bytes | **0** | clean (three files were reformatted to rustfmt's preferred shape during the work) |

Per-target detail for check 1: lib **72** (67 + the 5 new CSV cases) · `qa_evidence` 0 ·
`qa_inventory` 0 · `evidence_acceptance` 32 · `evidence_discovery` 13 · `evidence_emit` 7 ·
`evidence_file_reads` 9 · `evidence_journal` 31 · **`evidence_migration` 14** (12 + 2 new) ·
`evidence_publication` 36 · **`inventory` 9** (8 + 1 new). Nothing ignored, nothing filtered.

`QA_WRITE`, `QA_EVIDENCE_ROOT` and `NHL_STUDIO_FIXTURES` were `unset` before every invocation;
`--locked --offline` with `CARGO_NET_OFFLINE=true` throughout.

### 6.3 Capture and merge after the repairs

Fresh external root `H:\c0-evid-repair1\evroot`, `QA_WRITE` unset:

| Step | Result |
|---|---|
| the four emitters with `QA_EVIDENCE_ROOT` set | **4/4 passed**, **4 runs sealed `completed`**, no journal left behind |
| `build --out out1` / `--out out2` | 10 files each; **`diff -r` byte-identical, 11/11** |
| `publish --from out1 --data <scratch>` | published 10, id `4f68748435f62590`, prior generation retained |
| `check` | **exit 0** — `ok: 2175 row(s), no validation failure, no drift` |
| `publish --recover` | exit 0, transaction already resolved |
| **committed CSVs (12) before vs after** | **12 OK, 0 changed** — nothing regenerated |

**The merged matrix is unchanged by the repairs** — 2175 rows:

| Axis | Distribution |
|---|---|
| `qualification` | **140 `automated-qualified`** · 36 `not-applicable` · **159 `withheld`** · 1840 `unqualified` · **0 `game-verified`** |
| `automated` | 230 `pass` · 32 `mixed` · 121 `declared-not-executed` · 28 `skipped-missing-fixture` · 1764 `not-attempted` |
| `manual` | 2175 `none` — layer B untouched (`manual/` digest `sha256:e3b0c442…`, the empty-tree digest) |
| `fresh` | 411 `fresh` · **1764 `unknown`** |

The 140 qualified rows span 27 targets across `db-presentation` (27) and `db-operations` (113) —
identical to the pre-repair capture. **Incomplete, unknown and stale evidence remains
non-qualifying**: the 159 `withheld` variant cells and the 1764 `unknown` freshness rows are
unchanged, and nothing was promoted.

### 6.4 `headmesh-convert` stays unknown

Its declared inputs are **absent** in the candidate (`crates/tdb-headmesh/src`,
`defs/appearance/{headmesh-matrix,head-uv-classes,legacy-bone-names}.toml`), and `defs/qa/scopes.toml`
was **not edited** — the scope was not narrowed to whatever happens to exist. Measured directly
(scratch-only probe, removed afterwards):

```
SCOPE asset-codecs      v1 = sha256:1b9d91b8207e7…
SCOPE db-edit           v2 = sha256:29e3ed4025e58…
SCOPE db-read           v2 = sha256:d65738a43fedf…
SCOPE headmesh-convert  v1 = unknown
SCOPE save-containers   v2 = sha256:5df9e649d8c9e…
SCOPE shared-core       v2 = sha256:be01a2ea98715…
```

Under R-FRESH-UNKNOWN that scope can never be fresh and can qualify nothing until its inputs exist.

---

## 7. Preservation

| Check | Result |
|---|---|
| `git rev-parse HEAD` | `ef0fdd74…` → `ef0fdd74…` — **unchanged** |
| Index | **empty** → **empty** |
| `git status --porcelain=v1 -z -uall` SHA-256 | `bde8a4911842e5cd…` → **`bde8a4911842e5cd…` — byte-identical**, 732 entries both times (the 7 edited files were already ` M`/`??`; no entry appeared or vanished) |
| `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` | still ` M` with **exactly its pre-existing +1 row**, `sha256:bffb4ac6c0b74f0c…` — **not read for the divergence case, not modified** |
| `docs/audits/**` otherwise | not written |
| Candidate tree (`candidate-crlf`) | **1186 files, 1186 OK, 0 FAILED** against the pre-run manifest; membership delta **0**. (`du -sb` moved 756 275 435 → 756 275 497 — directory-entry metadata from the scratch probe's create/delete; **every file is byte-identical**) |
| Live files vs candidate | all 34 identical modulo CR — **0 mismatches** |
| `H:\c0-s2-int1\` prior evidence | **259 files, 259 OK, 0 FAILED** |
| `H:\c0-evid-probe1\` (the failed probe) | **retained, 21 227 files; no command in this package wrote to it** |
| Publication / migration install / staging / commit / push / CI | **none** |

---

## 8. Remaining gaps and referrals

1. **A production `migrate` change was considered and not made — reported, not expanded.** a11's
   registry path returns `"unknown"` for a missing `id_scheme` and `"unknown"` for absent fields
   rather than failing, and `migrate` hardcodes `nhllegacy-xbox360` as `capability.frozen.csv`'s
   target. Both shaped how the synthetic fixture had to be written (the shared destination must be
   spelled `nhllegacy-xbox360`). Whether either should change is a **separate review**; this package
   changed no production migration code.
2. **Real-registry integration is still unmet** (§3.2). It executes automatically once
   `defs/appearance/headmesh-matrix.toml` is committed; until then the requirement is recorded and
   visibly unsatisfied. It is **not** claimed to be covered by the synthetic case.
3. **The `heads` divergence question is untouched.** The live hand edit to `capability.csv` remains
   an open C0.4-owned decision (C0-SLICE1 E4). This package removed the test's *dependence* on it;
   it did not resolve its disposition.
4. **Linux/CI unverified.** Everything here ran on Windows. The repair makes the guards
   representation-independent, which should make them pass on both, but no CI run was made and
   Windows CI is still C0.2 R6, open.
5. **No workspace-wide `fmt`/clippy claim.** Check 9 is scoped to the seven changed files; check 3 is
   `-p tdb-qa`. The repo-wide C0.2 defects are untouched.
6. **The slice now includes `tests/inventory.rs`**, which carries its pre-existing C0-INT1 rustfmt
   reflow (§2). Committing the slice commits that reflow for that one file.

---

## 9. The newline defect, recorded here

**Recorded in this receipt; no historical C0.2 evidence was rewritten.**

| | |
|---|---|
| **What** | Generators write LF; a Windows checkout under `core.autocrlf=true` materialises the committed generated artefacts CRLF. Eight drift guards read the stored file raw and compared bytes, so they failed from a clean checkout while passing in a working tree |
| **Scope** | `docs/audits/data/{anchors-manifest,schema-diff,game-qa-matrix,surface-capabilities,mutation-matrix,operation-matrix,save-matrix}.csv` and `docs/audits/data/schema/*.json` |
| **Discovered** | C0-EVID-PROBE1 §4.2 — reproduced on a pure-HEAD control tree, cause isolated by converting one file to LF |
| **Class** | The same class C0-CAN-EOL1 repaired inside `tdb-canonical`, and which its §9 item 4 left open for other crates |
| **Status now** | **Closed for these eight guards** by §4. Not a C0.2 register edit: `docs/validation/c0-02-baseline-execution.md` was **not** modified, and whether the register should gain an entry remains the architect's call |
| **Not claimed** | That no other crate has the same pattern. Only the eight identified guards were audited and repaired |

---

## 10. Files changed

| File | Change |
|---|---|
| `crates/tdb-qa/src/evidence/csv.rs` | `Drift`, `compare_records`, 5 unit tests |
| `crates/tdb-qa/tests/inventory.rs` | 4 guards rewired, +1 test, clippy fix |
| `crates/tdb-qa/tests/evidence_migration.rs` | 2 cases rewritten, +2 cases, 3 synthetic builders |
| `crates/studio-domain/tests/{mutation,operation,save,surface}_matrix.rs` | 1 call site each |
| `docs/receipts/c0-evid-repair1.md` | this record |

**Nothing staged, committed or pushed. No live publication, no migration installation, no CI change,
no game-data access, and no prior evidence deleted. Stopping here for Codex review.**
