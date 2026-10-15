# C0-EMIT1-R1 — complete freshness coverage before integration

**Package:** C0-EMIT1-R1, a bounded correction of [C0-EMIT1](c0-emit1-observation-integration.md) §4.3.
**Date:** 2026-09-15 (UTC) · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **Baseline and HEAD throughout:** `58d1bc073bbf9e80d5afb4782bc6c132725e5fed` ·
**Index before and after:** empty · **Nothing staged, committed or pushed.**

> **Result.** The freshness scopes now cover the per-title metadata every emitter reads and the
> file-level closure of what the assertions execute. `shared-core` **v3** declares all 53
> `anchors/**/*-meta.xml` files literally; `db-read` **v3**, `db-edit` **v4** and
> `save-containers` **v4** declare the `attributes.rs` / `view.rs` / `team_tabs.rs` / `error.rs` /
> `tdb-enums` closure. A new test binary, `evidence_freshness` (**7 cases**), copies the committed
> definitions into a synthetic repository, captures through the real `Emitter`, and drives the real
> digest → merge path through every mutation the package names; a guard case fails if any
> `*-meta.xml` under `anchors/` is ever missing from the list. Three deliberate mutations of
> `scopes.toml` proved the cases load-bearing.
>
> On real material: the four emitters were **recaptured** on a fresh candidate under the corrected
> definitions (4 runs `completed`, 952 observations, 189 unmapped, 1 452 facets); `build` twice →
> byte-identical; `publish` into a disposable directory → `check` `ok: 2175 row(s)`. Then, over
> disposable copies of that candidate and the *same unchanged runs*: appending one comment to
> `nhl15-ps3`'s `nhlng-meta.xml` with the DB bytes unchanged turned all **411 cited rows `stale`**
> and **140 → 0 `automated-qualified`**; removing the file did the same; a one-line change to
> `crates/tdb-enums/src/lib.rs` staled exactly `db-edit` + `save-containers` (27 `db-read` rows
> stayed fresh); a change to `surfaces.rs` staled exactly the 28 `db-read` rows; restoring the
> bytes rebuilt the published set **byte-identical**. The C0-EMIT1 runs merged against the
> corrected definitions are `fresh = unknown` and withheld, still cited, their root untouched.
>
> `tdb-qa`: **237 passed · 0 failed · 1 ignored**. The overlay contributes zero clippy diagnostics;
> the `studio-domain` lib gate is **not** clean (8 HEAD diagnostics, C0.2 baseline).

---

## 1. Final scope definitions and coverage

### 1.1 Versions

| Scope | Before this package | Now | What changed |
|---|---|---|---|
| `shared-core` | v2 | **v3** | `schema = [ …53 literal `anchors/<platform>/<game>/databases/*-meta.xml` paths… ]` |
| `db-read` | v2 | **v3** | + `crates/studio-domain/src/attributes.rs` |
| `db-edit` | v3 (C0-EMIT1, uncommitted) | **v4** | + `view.rs`, `attributes.rs`, `error.rs` (keeps EMIT1's `team_tabs.rs`, `tdb-enums`) |
| `save-containers` | v3 (C0-EMIT1, uncommitted) | **v4** | + `team_tabs.rs`, `view.rs`, `attributes.rs`, `error.rs`, `crates/tdb-relations/src/**`, `crates/tdb-domain-validation/src/**` |
| `asset-codecs`, `headmesh-convert` | v1 | v1 | untouched; `headmesh-convert` still resolves `unknown` at HEAD |

Every bump is versioned; no run declaring an older version is rewritten (§3.4 shows the EMIT1 v3
runs reported against their own version). The rationale for each addition is recorded as a `# vN
(C0-EMIT1-R1)` comment beside the scope in `defs/qa/scopes.toml`.

### 1.2 Metadata — verified paths and discovery rules

* **What is consumed.** `surface_matrix`, `mutation_matrix` and `operation_matrix` pair each
  anchor with `NHL07NG-meta.xml` / `nhlng-meta.xml` / `nhl_game-meta.xml` in the same `databases/`
  directory. `save_matrix::meta_for` tries the same names and, for a PS3 game with no metadata of
  its own, **falls back to the Xbox 360 sibling's copy** — so the consumed file is not always under
  the target's own directory, and per-target precision would have to model that fallback.
* **What the merge compares.** `fixture_sha256` and `input_digests` are recorded on runs but the
  merge compares no fixture digest against current content (only scope digests are compared,
  `merge::digest_freshness`). Adding metadata `input_digests` alone would therefore have changed
  nothing; the coverage had to be a scope.
* **Discovery rules.** `Scope::globs()` accepts a literal path, `prefix/**` or `prefix/*` — no
  `*.xml` form. A directory prefix (`anchors/**`, or the 27 `databases/*` prefixes) was tried
  first and **rejected on cost**: `collect_scope_contents` hashes every matched file on every
  `Emitter::from_env`, every `qa-evidence` command and every test that resolves real digests, and
  the debug-profile `sha2` over the 662 MB anchor tree pushed the `tdb-qa` suite past 20 minutes
  (run aborted). The 53 `*-meta.xml` files are listed **literally** instead (a few MB). Their names
  are enumerated, not hand-typed, and the list cannot fall behind silently: the guard case in §2
  asserts every `*-meta.xml` present under `anchors/` is declared and every declared one exists.
* **Semantics under R-FRESH-UNKNOWN.** A changed file → different digest → `stale`. A missing file
  → its literal glob matches nothing → `shared-core` is `unknown` → every scope sharing it is
  `unknown` → not fresh. An unreadable or link-aliased entry at a declared path is a hard error
  from `collect_scope_contents` (R4/R5), so `scope_digests` yields no current digest → `unknown`.
  Measured: §3.
* **Cost after the change:** `qa-evidence build` 1.5 s; `evidence_journal` 40 → 63 s,
  `evidence_emit` 11 → 17 s (each case resolves the real digests over the candidate).

### 1.3 Implementation — the exercised-dependency audit

Read from HEAD's `studio-domain` sources, following what the emitters call:

| Path / crate | Exercised by | Reached through | Declared in |
|---|---|---|---|
| `tdb-core`, `tdb-defs` | all | parse, `FieldMetaIndex`, `FieldMeta::{validate,encode,decode_raw}` | `shared-core` (unchanged) |
| `defs/enums/**`, `defs/relationships/**` | edit, save | `Project::load` → `EnumRegistry::from_dir`, `RelationRegistry::from_dir` | `shared-core` (unchanged) |
| `tdb-relations` | all | `RosterModel::build`; `EditTransaction`; `validate_invariants` | db-read, db-edit, **+save-containers** |
| `tdb-domain-validation` | edit, save | `EditTransaction::commit` (`towx_coherence`, `companion_presence`, …); `sess.validate()` | db-edit, **+save-containers** |
| `tdb-enums` | edit, save | `Project::load`; every `EditSession` op passes `project.enums()`; `validate_invariants(db, relations, enums)`; `view::read_field` | db-edit (EMIT1), save-containers (EMIT1) |
| `tdb-edit` | edit, save | `EditSession::run` → `EditTransaction` | db-edit, save-containers (unchanged) |
| `tdb-savedata` | save; **and `Project::load`** | `detect_source` tries `SaveContainer::parse` / `is_ps4_save` **only after** a raw parse fails — the two edit emitters feed loose DBs, so it is not reached there | save-containers only (documented) |
| `capabilities.rs`, `surfaces.rs` | read | `Capabilities::probe`, `SurfaceCapabilities::probe` | db-read (unchanged) |
| `view.rs` | read, edit, save | `probe_main_team_encoding` (from `Capabilities::probe`); `read_u64_by_studio` (from `set_field`'s strategy mirror); `read_field`, `team_name_by_id` (from `team_strategies`) | db-read, **+db-edit, +save-containers** |
| `attributes.rs` | read, edit, save | `Capabilities::probe` → `rating_label/offset/suffix`, `is_gen6_style_schema`; `view::read_field` → `is_gen6_style_schema`, `synthetic_enum` | **+db-read, +db-edit, +save-containers** |
| `team_tabs.rs` | edit, save | `set_field` compares `STRATEGIES_TABLE`; `strategy-edit` picks its cell via `team_strategies` | db-edit (EMIT1), **+save-containers** |
| `error.rs` | edit, save | every `Result` the assertions match on (`From<EditError>` etc.) | **+db-edit, +save-containers** |
| `project.rs`, `session.rs` | edit, save | entry points | db-edit, save-containers (unchanged) |
| `assets.rs` | — | `view.rs` imports `probe_birthdate_encoding` for the player/bio views; no exercised function reaches it; `Capabilities::probe` does not call it | **left out**, with the reason in the file |
| `audit.rs` | — | `audit_report()`/`humanize` is UI-facing; the emitters call `validate()` (raw report) | left out |
| `lib.rs` | — | re-exports only | left out |
| headmesh crates / `defs/appearance/**` | — | not reached | not added (per package limit) |

`db-edit` and `save-containers` now declare the **same** `studio-domain` / engine closure; the
only differences are the codec crate (`tdb-savedata`, save only) and the `mapping` file. The
duplication is deliberate — folding `db-edit` into `save-containers` via `shared` would also fold
in the two edit emitters' mapping files, staling save evidence whenever an unrelated edit assertion
changes.

## 2. Tests — `crates/tdb-qa/tests/evidence_freshness.rs` (new, 475 lines, 7 cases)

Every case copies the **committed** `defs/qa/scopes.toml` from the repository into a synthetic
repository, creates stub content under every declared glob (so every scope resolves), captures a
completed passing run per emitter scope through the real `Emitter::from_env` (`QA_EVIDENCE_ROOT` →
a sandbox), and merges `store::load_runs` against `emit::scope_digests` of the synthetic repository
*as it currently is*. The keys used (`nhl15-ps3/db-presentation/read`, `…/db-operations/scalar-edit`,
`…/save-inner-container/container-repack`) are `required`, so a fresh pass reaches
`automated-qualified` and freshness alone separates the two outcomes.

| Case | Proves |
|---|---|
| `every_scope_the_emitters_declare_resolves_to_a_real_digest_over_the_synthetic_repo` | every committed scope yields a `sha256:` digest once its globs match; `shared-core` v3, `db-read` v3, `db-edit` v4, `save-containers` v4 are present (a silent rollback fails) |
| `every_metadata_file_under_anchors_is_declared_by_shared_core` | **the guard**: lists (names only) every `*-meta.xml` under the real `anchors/`; each is declared by `shared-core`, and each declared `anchors/…` path exists |
| `a_metadata_edit_with_unchanged_db_bytes_makes_every_dependent_pass_stale_and_withheld` | baseline `fresh`/`automated-qualified` for all three; one appended comment to the metadata, DB bytes asserted unchanged → all three `stale`, `withheld`, `stale-evidence` |
| `a_removed_metadata_file_cannot_leave_dependent_evidence_fresh` | file removed → not fresh, withheld; restored byte-equal → fresh again; whole `anchors/` removed → every emitter scope's current digest is the literal `unknown` and every row withheld |
| `a_run_captured_while_metadata_is_missing_records_unknown_and_never_qualifies` | capture with `anchors/` absent → `implementation_digest = "unknown"` on every run, `fresh = unknown`, withheld; restoring the tree afterwards does not retroactively qualify |
| `a_change_to_a_newly_covered_dependency_invalidates_exactly_the_scopes_that_declare_it` | `tdb-enums` → edit + save stale, read fresh; `attributes.rs` → all three stale; `surfaces.rs` → read stale, edit + save fresh |
| `restoring_the_original_bytes_restores_the_identical_digest_and_freshness` | R-CONTENT: after mutate + restore the digest map is `==` the original and every row fresh again; two merges over identical inputs agree row for row |

**Load-bearing, by deliberate mutation of the live `scopes.toml`** (each restored and hash-verified):

| Mutation | Result |
|---|---|
| M1 — the metadata list removed from `shared-core` | **4 failed** (`meta-edit`, `meta-removed`, `missing-at-capture`, `restore`), 2 passed — exactly the metadata-dependent cases |
| M2 — `crates/tdb-enums/src/**` removed from `db-edit` only | **1 failed**: `…invalidates_exactly_the_scopes_that_declare_it` at *"db-edit v4 declares crates/tdb-enums/src/\*\*"* |
| M3 — one `*-meta.xml` path dropped from the list | **1 failed**: the guard, naming `anchors/ps4/nhl19/databases/nhl_game-meta.xml` |

`crates/tdb-qa/tests/evidence_emit.rs` keeps C0-EMIT1's two cases unchanged.

## 3. Invalidation proven through discovery and merge on real material

### 3.1 The candidate

`git archive 58d1bc07…` (1 192 files) + the overlay of §5 → **1 193 files**, `…/scratchpad/emit1r1/candidate`;
dedicated `CARGO_TARGET_DIR`; `QA_WRITE` never set; `QA_EVIDENCE_ROOT` set only for §3.2, to
`…/emit1r1/evroot`. `--locked --offline` accepted the one-line lock on every command.

**Disabled mode was not repeated.** Its result (C0-EMIT1 §3.1: 4/4 pass, 0 files created) depends
on the four emitter files and `tdb-qa`'s `emit.rs`, all byte-identical to that candidate
(`Emitter::from_env` returns the disabled sink before any scope is read), and on the committed
CSVs, unchanged. Nothing in this package's overlay is on that path.

### 3.2 Recapture (`QA_EVIDENCE_ROOT` set, `QA_WRITE` unset)

| Emitter | Result | Runtime | `run_id` | Scope · `implementation_digest` |
|---|---|---|---|---|
| `surface_matrix` | sealed `completed` | 9.1 s | `surface_matrix-20260915t025106z-3a379455` | `db-read` **v3** · `sha256:c45ed4431c96cc07…` |
| `mutation_matrix` | sealed `completed` | 152.3 s | `mutation_matrix-20260915t025121z-b6bd0e7b` | `db-edit` **v4** · `sha256:b401d91baf8144bd…` |
| `operation_matrix` | sealed `completed` | 503.9 s | `operation_matrix-20260915t025358z-6319c4f8` | `db-edit` **v4** · same |
| `save_matrix` | sealed `completed` | 108.4 s | `save_matrix-20260915t030227z-2212fe54` | `save-containers` **v4** · `sha256:21c5432847a98714…` |

Declared 28 / 28 / 140 / 243; observations 28 / 175 / 302 / 447 (**952**: 635 `pass`, 111
`refused-clean`, 177 `not-attempted`, **28 `skipped-missing-fixture`**); 189 unmapped; 1 452
facets; 27 anchor `input_digests` on each edit run; no journal left behind. Same shape as C0-EMIT1
§3.2, from this package's own capture.

### 3.3 Deterministic build, disposable publication, check

| Step | Result | Exit |
|---|---|---|
| `check --data <empty>` | 10 files differ from a fresh build | 2 |
| `build --out out1` / `--out out2` | 10 files each, **1.5 s** | 0 / 0 |
| `diff -r out1 out2` | **byte-identical** (11/11 incl. `build-inputs.json`) | — |
| `publish --from out1 --data data-scratch` | id `83417efc3c479952`, prior generation retained | 0 |
| `check --data data-scratch` | **`ok: 2175 row(s), no validation failure, no drift`** | 0 |
| `publish --recover` | nothing to do | 0 |
| `build --out <evroot>/auto` | refused, `path-overlap` | 4 |

Merge inputs: the four runs above (`[completed+selected]`), `manual/` at the empty-tree digest;
`scope_definitions_digest = sha256:71c94473…` = the candidate's `scopes.toml`. **Matrix:** 2 175
rows · **140 `automated-qualified`** (`db-presentation` 27, `db-operations` 113, 27 targets) · 159
`withheld` (all `unknown-applicability`) · 36 `not-applicable` · 1 840 `unqualified` · **0
`game-verified`** · `fresh`: 411 / `unknown` 1 764 · 121 `declared-not-executed` · 28
`skipped-missing-fixture` · conflicts/unresolved empty.

### 3.4 Mutations on a disposable copy, same runs, `build` only

`candidate-mut` = a copy of the candidate; the evidence root is read, never written (its file
manifest is identical before and after, verified). Each row is one `qa-evidence build --repo
candidate-mut --root evroot`:

| # | Mutation | cited rows `fresh` | `automated-qualified` | cited rows by family |
|---|---|---|---|---|
| 0 | none | 411 fresh | **140** | read 27 q · ops 113 q · save 159 withheld (variants) |
| 1 | `anchors/ps3/nhl15/databases/nhlng-meta.xml` + one comment; `nhlng.db` SHA-256 asserted unchanged | **411 stale** | **0** | read 28 withheld · ops 135 withheld · save 242 withheld — all `stale-evidence` (variants stay `unknown-applicability`) |
| 2 | that file **removed** | **411 stale** | **0** | same as 1 — `shared-core` resolves `unknown`, which differs from every declared digest |
| 3 | file restored byte-equal | 411 fresh | 140 | **matrix byte-identical to `out1`** |
| 4 | `crates/tdb-enums/src/lib.rs` + one comment | 383 stale · **28 fresh** | **27** | read 27 q (fresh) · ops 135 withheld · save 242 withheld |
| 5 | `crates/studio-domain/src/surfaces.rs` + one comment | 28 stale · 383 fresh | **113** | read 28 withheld · ops 113 q · save unchanged |
| 6 | everything restored | 411 fresh | 140 | **matrix byte-identical to `out1`** |

Rows 1–2 are the package's objective made visible: a metadata edit with untouched database bytes
cannot leave a single dependent row fresh. Rows 4–5 show the new declarations are attributed, not
blanket: each change stales exactly the scopes that declare the file.

**Superseded evidence preserved.** The C0-EMIT1 runs (`…/emit1/evroot`, declaring `db-read` v2,
`db-edit` v3, `save-containers` v3) merged against the corrected candidate: 411 cited rows, all
`fresh = unknown` (their declared versions have no current digest), 246 `withheld /
stale-evidence` + 159 `unknown-applicability`, still cited by their original run ids; the root
still holds its 4 runs, byte-unchanged. They are not reused as current evidence.

## 4. Validation, baseline vs regression, preservation

| Check (candidate) | Result | Classification |
|---|---|---|
| `cargo test -p tdb-qa --locked --offline` | **237 passed · 0 failed · 1 ignored** (72 unit · 32 acceptance · 13 discovery · 9 emit · 9 file-reads · **7 freshness** · 31 journal · 19 + 1 ignored migration · 36 publication · 9 inventory) | clean; the ignored case is the uncommitted-registry case, unchanged |
| `cargo clippy -p tdb-qa --all-targets --locked --offline -- -D warnings` | exit 0 | clean |
| `cargo fmt -p tdb-qa -- --check` | exit 0 | clean |
| `rustfmt --check` on the six overlay `.rs` files | all clean | clean |
| `cargo clippy -p studio-domain --no-deps --test {4 emitters} -- -D warnings` | exit 101: **8** diagnostics in `studio-domain/src/{assets ×3, attributes, formula, head_catalog ×2, view}.rs` | **baseline** — HEAD lib sources, C0.2; the gate is **not clean** and this package does not claim it |
| same with warnings non-fatal, diagnostics by path | the four emitter targets and both `tdb-qa` test files contribute **0**; `tdb-rx2/src/playermesh.rs` 1 (HEAD) | overlay clean |
| Candidate manifest before → after everything | 1 193 files, **0 differing** | — |
| Live `anchors/**` + `docs/audits/**` | 621 files, **0 mismatches**; `docs/audits/evidence` absent | — |
| Live `git status --porcelain` | 258 → 258; the package's own edits are `M defs/qa/scopes.toml`, `M crates/tdb-qa/tests/evidence_emit.rs` (EMIT1), `?? crates/tdb-qa/tests/evidence_freshness.rs`; the three `scopes.toml` mutations were restored to the identical hash each time | — |
| HEAD / index / push | `58d1bc07…` / empty / none | — |
| Prior evidence (`…/emit1/evroot`, `H:\c0-s2-int1\…`) | read only; unchanged | — |

## 5. Commit candidate — exact paths and the minimal lock delta

B2 as accepted, extended by this package: **9 paths, 1 A · 8 M**. SHA-256 as placed in the
candidate (CRLF); `git hash-object` of each equals the live file's blob.

| Path | Op | SHA-256 | Blob |
|---|---|---|---|
| `crates/studio-domain/Cargo.toml` | M | `3b28e92b9267ace8…` | `af8e2b5d` |
| `crates/studio-domain/tests/surface_matrix.rs` | M | `4f7fbf1e7a254f7c…` | `f17f6b89` |
| `crates/studio-domain/tests/mutation_matrix.rs` | M | `60952ea3df98fc1f…` | `6e0117af` |
| `crates/studio-domain/tests/operation_matrix.rs` | M | `60bb736d92d9eb0a…` | `ad98dd71` |
| `crates/studio-domain/tests/save_matrix.rs` | M | `8a0896ed4edb686e…` | `df8559d3` |
| `defs/qa/scopes.toml` | M | `71c944736355f61b…` | `1a2bf8a1` |
| `crates/tdb-qa/tests/evidence_emit.rs` | M | `61413ea7bbb4d186…` | `157f44a4` |
| `crates/tdb-qa/tests/evidence_freshness.rs` | **A** | `4d7542e3f52220f4…` | `0abeb9f0` |
| `Cargo.lock` | M | `4a4cc094aa6f2776…` | `8decfca0` |

**Lock delta — exactly one line**, built from `HEAD:Cargo.lock`:

```
@@ -3372 +3373 @@ name = "studio-domain"
+ "tdb-qa",
```

Root `Cargo.toml` is HEAD's. No `tdb-qa` source file changes. Documentation (this receipt) stays
outside the commit. Subject proposal unchanged: `feat(qa): wire matrix emitters to the evidence
observation lifecycle`.

## 6. Baseline lint blockers and remaining activation decisions

| Item | Status |
|---|---|
| `studio-domain` lib clippy (8 diagnostics) and repo-wide `fmt`/`clippy` at HEAD | C0.2 baseline; blocks a clean-checkout `-D warnings` gate over the emitter targets; owner is the baseline repair track, not this slice |
| Fixture content identity (`.db`, save fixtures) | still not compared by the merge; only `*-meta.xml` is scope-covered. A DB or save fixture swap that keeps its path would not stale evidence. Covering `anchors/**` by scope was measured too costly (§1.2); the remaining options are a cheaper digest source or a merge-side fixture comparison — architect decision |
| `save_matrix` PS3 → Xbox 360 metadata fallback | covered (all metas listed) but worth an explicit note in any future per-target precision |
| Storage (U3), selected-run retention/recapture (D2), consumer transition (R-TRANSITION), CI (R-NO-CI) | unchanged from C0-EMIT1 §8; nothing activated |
| Q1–Q4 / R3 (vocabulary, unmapped operations, required-variant catalog, sealing interrupted journals) | open |
| Cosmetic `save_matrix` detail-string spacing (EMIT1 §4.4) | unchanged; the emitter files are still byte-identical to the REPAIR1-verified versions |

## 7. Limits honoured

No staging, commit or push; no live migration or publication (`docs/audits/data` and
`docs/audits/evidence` never named as a target); no CI change; no game installation; no headmesh
work or headmesh paths added to any scope; no external repository; no registry expansion; no
baseline repair. Every mutation happened in a disposable copy or in the live `scopes.toml` under a
recorded hash that was restored and re-verified. Stopping for Codex review.
