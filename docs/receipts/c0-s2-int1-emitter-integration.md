# C0-S2-INT1 — Real emitter integration and non-empty evidence validation

> **Package:** C0-S2-INT1. **Date:** 2026-09-08. **Author role:** developer.
> Reviewer: Codex (architect).
> **Status: SUPERSEDED IN PART — see the review correction below.** Original status was
> "Passed, with three items referred to the architect" (§8).

## ⚠ Review correction (C0-S2-INT1-R1, 2026-09-08)

Two claims in this receipt are **withdrawn**; see
[c0-s2-int1-r1-execution-truth.md](c0-s2-int1-r1-execution-truth.md) §1 for the measurements.

1. **The "215 `automated-qualified`" figure must not be cited.** Those observations declared
   freshness scopes at v1, and v1 omitted dependencies their assertions actually exercise
   (`Capabilities::probe`, `RosterModel::build`, `Project::load`, the invariant audit, and the
   emitter test sources carrying the coverage-key mapping). With the scopes corrected and
   versioned, **the same preserved runs** yield **0 `automated-qualified`** and 246 `withheld /
   stale-evidence`. Under the corrected semantics a fresh run yields **140**, not 215:
   `save-inner-container`'s 75 rows are withheld by the conservative variant rule.
2. **"No emitter was blocked and no fixture was unavailable" conflated two claims**, and the second
   half is contradicted by this receipt's own table. All four emitters *completed*; **28
   declared fixtures did not exist** and are recorded as `skipped-missing-fixture`. "The emitter
   finished" says nothing about whether the material it needed was there.

Additionally, §3.1's interruption window is narrowed: an unsealed journal is no longer invisible to
the merge — it is loaded as incomplete execution evidence, cited, never selected, and blocking
wherever it persisted a failure.


> Nothing staged, committed or pushed. **No live evidence publication, no migration installation,
> no CI activation, no consumer retirement, no canonical-model decision.** `QA_WRITE` was **never**
> set during this package's verification. Every publication, recovery and observation ran in an
> isolated directory outside the repository.
>
> This closes the two prerequisites C0-BULK1-R1 §7 **G3** left open: the four `QA_WRITE=1` emitters
> now have a parallel `observe` call, and `qa-evidence check` has been validated against a real,
> **non-empty** evidence tree. Nothing here promotes any support status — the first real merge
> produces **0 `game-verified`** rows.

---

## 1. What was built

| # | Change | Why |
|---|---|---|
| **1** | [`evidence::emit::Emitter`](../../crates/tdb-qa/src/evidence/emit.rs) — the emitter-facing observation lifecycle over `RunBuilder` | `RunBuilder` accumulates a whole run in memory and seals it in one call, so a panic in case 200 discarded the 199 observations already made. `Emitter` adds journalling, a panic-recording case wrapper and a drop guard. |
| **2** | Opt-in switch `QA_EVIDENCE_ROOT` | The only thing that turns observation on. Unset ⇒ a disabled sink that creates no file. `QA_WRITE` is untouched and independent. |
| **3** | In-progress journal at `<root>/auto/in-progress/<journal_id>/` | Run identity and declarations persist **before the first case runs**; observations are appended as they happen. A hard kill leaves the journal, so interruption is visible on disk. |
| **4** | [`crates/tdb-qa/build.rs`](../../crates/tdb-qa/build.rs) + `emit::toolchain_identity` | Real toolchain identity. The CLI previously reported `std::env::consts::ARCH` (`x86_64`) as `target-triple` — a **fabricated identity**. It now reports the real triple, or `None`, and the CLI and the emitters share one resolver so their digests can agree. |
| **5** | `defs/qa/scopes.toml` `shared-core` **v1 → v2** | v1 declared `defs/metadata/**`, which does not exist. Under R-FRESH-UNKNOWN one unmatched glob makes the digest `unknown`, and every other scope shares `shared-core`, so **every** freshness digest was permanently `unknown` and nothing could ever be fresh or `automated-qualified`. See §7 D1. |
| **6** | `qa-evidence check` reports unsealed journals | Informational only; not an active input, changes no exit code. |
| **7** | The four emitters gain a parallel `observe` call | Checkpoint 1. Their CSV write paths, drift assertions and `QA_WRITE` behaviour are **unchanged**. |

### Files changed

| Path | Change |
|---|---|
| `crates/tdb-qa/src/evidence/emit.rs` | **New.** `Emitter`, journal, drop guard, `toolchain_identity`, `scope_digests`, `file_digest`, `utc_now`. 3 unit tests. |
| `crates/tdb-qa/build.rs` | **New.** Captures `rustc -vV` line 1 and cargo's `TARGET`; emits nothing when unresolvable. |
| `crates/tdb-qa/src/evidence/mod.rs` | Wired `emit`. |
| `crates/tdb-qa/src/evidence/store.rs` | `OBSERVATION_HEADER` made `pub(crate)` so the journal writes the sealed row shape. No behaviour change. |
| `crates/tdb-qa/src/bin/qa_evidence.rs` | Uses `emit::toolchain_identity` (removes the ARCH substitution); `report_in_progress`. |
| `crates/tdb-qa/tests/evidence_emit.rs` | **New.** 7 lifecycle cases over synthetic roots. |
| `defs/qa/scopes.toml` | `shared-core` v2; the dead `defs/metadata/**` glob replaced by `defs/enums/**` + `defs/relationships/**`. |
| `crates/studio-domain/Cargo.toml` | `tdb-qa` added as a **dev-dependency** (test-only; no runtime dependency, no cycle). |
| `crates/studio-domain/tests/{surface,mutation,operation,save}_matrix.rs` | Declaration, `case` wrapping and observation. Existing assertions and CSV paths unchanged. |
| `docs/receipts/c0-s2-int1-emitter-integration.md` | **New.** This receipt. |

---

## 2. Checkpoint 1 — exact emitter → coverage-key mapping

Every mapping below is justified by the assertions the case actually executes. Where an emitter
operation has no honest key in the closed vocabulary it is mapped to **nothing** and the key is left
absent (R-ABSENT: absence asserts nothing). Guessing a family from an adjacent assertion is the
E1/E2 positional error this contract exists to remove.

### 2.1 `surface_matrix` → `db-read`

| Key | Recorded when |
|---|---|
| `<target>/db-presentation/read` | the main DB parsed, the metadata index loaded, `RosterModel::build` succeeded and all 18 surfaces probed |

Not claimed: `save-raw-db` (nothing is saved); any `assets-*` key (a probed `portraits`/`heads`
*state* is a capability report, not asset coverage).

### 2.2 `mutation_matrix` → `db-edit`

| Key | Recorded when |
|---|---|
| `<target>/db-operations/scalar-edit` — `pass` | one codec representative completed stage → save → re-parse (CRC/size) → undo to byte equality → redo to byte equality, and its out-of-range value was refused |
| same key — `refused-clean` | the per-anchor negative gates: `update=False` and key/identity fields refused an edit with no byte or undo-history change |

One observation per (anchor, codec) and one per (anchor, negative-gates); R-MULTI keeps them all.

**Deliberately not claimed:** `save-raw-db/save` + `/reopen`. The session's `save()` on a copied
loose `.db` *is* mechanically the raw-DB write path, so this is a plausible second mapping — but the
test asserts a field round-trip, not a save contract. Referred to the architect (§8 Q1).

### 2.3 `operation_matrix` → `db-edit`

| Emitter operation | Coverage key operation |
|---|---|
| `create-skater`, `create-goalie`, `clone-skater`, `clone-goalie` | `db-operations/roster-create` |
| `delete-player` | `db-operations/roster-delete` |
| `move-player` | `db-operations/roster-move` |
| `trade-slots` | `db-operations/roster-trade` |
| `set-captaincy`, `set-jersey`, `set-dressed`, `strategy-edit` | `db-operations/scalar-edit` |
| `release-then-sign`, `add-remove-membership`, `autocorrect-jerseys`, `retire-unretire-jersey`, `repair-roster-blocks`, `trade-draft-pick`, `career-add-delete` | **unmapped — recorded nowhere** (§8 Q2) |

Outcome mapping: `Status::Pass` → `pass`; `Status::RefusedClean` → `refused-clean`;
`Status::NoFixture` (applicable by schema, no entity to exercise it) → `skipped-missing-fixture`.

### 2.4 `save_matrix` → `save-containers`

| Key | Recorded when |
|---|---|
| `<target>/save-inner-container/container-detect` | `pass` — the codec was identified from bytes alone and truncated input was refused cleanly; `refused-clean` — the bytes classify to no supported save shape |
| `<target>/save-inner-container/container-extract` | `pass` — a payload came out and parses as a coherent inner TDB; `refused-clean` — no coherent inner (multi-part / non-DB payload), declined without a panic or a partial result; `not-attempted` — a loose DB or an unsupported shape, where extraction does not apply |
| `<target>/save-inner-container/container-repack` | `pass` — identity repack preserved the inner bytes with valid checksums and was deterministic, a modified payload was carried faithfully, and (where the fixture is a ROSTER save) a real `EditSession` scalar edit survived save → reopen; `not-attempted` — this fixture carries no coherent inner payload |

Not claimed: `save-raw-db` (the `RawDb` branch classifies a loose DB and saves nothing);
`save-outer-package` (no console packaging is exercised); `db-operations/scalar-edit` (the semantic
roster edit runs on the *inner subset* of a save and is recorded as part of what `container-repack`
proves, not as main-database coverage).

**No `variant` component is used.** `vocab::catalog_universe()` enumerates variant-less required
cells, so a `.../container-repack/v4` key would sit *beside* the required cell rather than satisfy
it, leaving the required one `not-attempted`. The container version is recorded in the observation
`detail`. Referred to the architect (§8 Q3).

### 2.5 Declared-but-absent units

All four emitters declare the **whole 28-unit catalog** and record
`skipped-missing-fixture` for a unit they find no fixture for — `legacyrecomp-recomp` in every case,
plus the units with no save fixtures for `save_matrix`. That makes E8 ("nothing distinguishes
declared from observed coverage") explicit instead of expressing it by absence.

---

## 3. Checkpoint 2 — execution truth

| Requirement | How it is met |
|---|---|
| **Declare before executing** | Every key is declared up front from `tdb_qa::catalog::CATALOG`, and persisted to the journal, before the first case runs. `RunBuilder::observe` refuses an undeclared key (V6). |
| **Record `pass` only after the assertions succeed** | `Emitter::case` records **nothing** on success. The caller records the outcome afterwards, so a `pass` row can only follow the assertions that earned it. |
| **Absent fixture vs declared-not-executed vs executed failure vs interruption** | `skipped-missing-fixture` (no fixture), `not-attempted` (the run reached the key and deliberately did not run it), `fail` (a case panicked), and an unsealed journal + `state = failed`/`interrupted` respectively. All four appear in the real runs or the regression suite. |
| **A later panic must not lose earlier work** | `Emitter::case` catches the unwind, records `fail`, and the `Drop` guard seals the run as `failed` with **every** observation made before it. Proven by `a_panicking_case_records_an_executed_failure_keeps_the_earlier_work_and_resumes_the_panic`. |
| **Original failure behaviour retained** | The panic is re-raised with `resume_unwind` **immediately**, with the original payload. The test fails at the same point, with the same message. Nothing is swallowed. |
| **Persist identity early enough that interruption is observable** | The journal directory (identity + declarations) is created before any case runs; observations are flushed as they happen. Demonstrated with a real `taskkill /F` in §5.4. |
| **Real digests and toolchain identity** | `implementation_digest` is computed from `defs/qa/scopes.toml` over real file content; `fixture_sha256` is the real SHA-256 of the anchor or save fixture; `toolchain` is the real `rustc -vV` string. |
| **Unknown stays unknown** | An unresolvable toolchain identity is `None` ⇒ the scope digest is the literal `unknown` ⇒ freshness `unknown` ⇒ `withheld`. No substitute value is invented (this is exactly what §1 item 4 repaired). |

### 3.1 The remaining interruption window, stated honestly

A **hard kill** — `SIGKILL`/`taskkill /F`, `abort`, a stack overflow, or a `panic = "abort"` profile
— runs no destructor, so the run is never sealed. What survives is the journal directory:
identity, every declaration, and every observation up to the last flush. `check`/`build` **do not
read it**; `check` reports its presence and nothing more, and the merge output is byte-identical
with or without it (verified in §5.4). Converting a journal into a sealed `interrupted` run is not
implemented here — it would need a decision about who may write a run directory the emitter did not
seal, which is the architect's (§8 Q4).

There is also **no power-loss durability** (no `fsync` anywhere) and **no concurrency control**,
unchanged from C0-BULK1-R1 G1/G2 and R2 H1/H2.

---

## 4. Checkpoint 3a — synthetic lifecycle regression

`cargo test -p tdb-qa --test evidence_emit` — **7 passed, 0 failed**. Every case runs over a
synthetic root under `%TEMP%\tdb-qa-evidence-emit\`; nothing reads or writes `docs/audits/`,
`anchors/`, a game tree or a private fixture.

| Case | Asserts |
|---|---|
| `without_the_opt_in_variable_an_emitter_creates_no_file_at_all` | with `QA_EVIDENCE_ROOT` unset: sink disabled, `declare`/`observe`/`finish` are no-ops, **0 files** in the sandbox, no `auto/` directory, `load_runs` → 0 |
| `an_evidence_root_inside_the_repository_is_refused` | a root resolving inside the repo is refused (U3 — layer-A storage in git is undecided) |
| `a_completed_run_separates_a_pass_from_an_absent_fixture` | 2 declared / 1 executed; a skip is declared but never executed; real input digest recorded; toolchain is a real `rustc …` string or the literal `unknown`; the journal is removed once sealed; the run validates |
| `a_panicking_case_records_an_executed_failure_keeps_the_earlier_work_and_resumes_the_panic` | the panic still propagates; exactly one run sealed, state `failed`; the earlier `pass` survives; the panicking key is `fail` carrying the assertion message; the merge reports `executed-unselected` + `withheld` for it, and the passing key in the same failed run qualifies nothing either |
| `a_run_that_is_never_sealed_leaves_an_observable_journal_and_no_run` | `mem::forget` (what a hard kill does): journal with identity, declarations and observations present; **0 runs**; nothing sealed |
| `repeated_runs_are_all_retained` | two runs, two directories, both `completed`, nothing overwritten (E10) |
| `a_completed_pass_qualifies_only_while_its_implementation_digest_is_current` | the emitter records a **real** `sha256:` digest; fresh ⇒ `automated-qualified` with the run cited *and selected*; one changed byte ⇒ the same observation is retained and cited, `stale`, `withheld` |

Whole crate: `cargo test -p tdb-qa` — **158 passed, 0 failed, 0 ignored**
(63 lib + 32 acceptance + 7 emit + 12 migration + 36 publication + 8 inventory).
Per the evidence rules that is baseline health, not target qualification.

---

## 5. Checkpoint 3b — real runs and the end-to-end evidence path

### 5.1 Write behaviour inspected first

Before any observed run, all four emitters were executed in **default mode** and produced no
evidence file (§4 case 1 proves the mechanism; §5.5 re-proves it on the real emitters after the
change). `QA_WRITE` was never set: the four `docs/audits/data/*matrix*.csv` / `surface-capabilities.csv`
files are byte-identical before and after everything below.

### 5.2 The four real runs

Evidence root `H:\c0-s2-int1\run1` (outside the repository). Command per emitter:
`QA_EVIDENCE_ROOT=H:/c0-s2-int1/run1 cargo test -p studio-domain --test <emitter>`.

| Emitter | Result | `run_id` | Scope / `implementation_digest` | Declared | Executed | Observations |
|---|---|---|---|---|---|---|
| `surface_matrix` | passed, sealed `completed` (6.8 s) | `surface_matrix-20260908t015354z-9ee70318` | `db-read` v1 · `sha256:f38b768232064d8f…` | 28 | 27 | 28 — 27 `pass`, 1 `skipped-missing-fixture` |
| `mutation_matrix` | passed, sealed `completed` (113 s) | `mutation_matrix-20260908t015404z-e442ccf6` | `db-edit` v1 · `sha256:357f3d6529688c4b…` | 28 | 27 | 175 — 147 `pass`, 27 `refused-clean`, 1 `skipped-missing-fixture` |
| `operation_matrix` | passed, sealed `completed` (330 s) | `operation_matrix-20260908t015600z-45fa6d93` | `db-edit` v1 · `sha256:357f3d6529688c4b…` | 140 | 119 | 302 — 271 `pass`, 10 `refused-clean`, 21 `skipped-missing-fixture` |
| `save_matrix` | passed, sealed `completed` (74 s) | `save_matrix-20260908t020134z-a65232c1` | `save-containers` v1 · `sha256:348bcc141fe26f9c…` | 84 | 76 | 369 — 190 `pass`, 74 `refused-clean`, 99 `not-attempted`, 6 `skipped-missing-fixture` |

**874 real observations, 280 declared keys, 4 completed runs.** All four ran to completion —
**no emitter was blocked and no fixture was unavailable**. `mutation_matrix` and
`operation_matrix` additionally carry 27 real per-anchor `input_digests` each; `surface_matrix` and
`save_matrix` carry their fixture digests per observation instead (`fixture_sha256`).

### 5.3 Loading the non-empty tree through `check` / `build` / `publish`

Published set directory `H:\c0-s2-int1\data` (isolated; `docs/audits/data` was never the target).

| Step | Result |
|---|---|
| `check` before publishing | 8 files differ from a fresh build, exit **2** |
| `build --out out1` | 8 files, exit 0 |
| `build --out out2` | 8 files, exit 0 |
| **`diff -r out1 out2`** | **byte-identical, 8/8 — A1 determinism on real inputs** |
| `publish --from out1` | published 8, id `20044c9b027e7a80`, prior generation retained |
| `check` after publishing | `ok: 2016 row(s), no validation failure, no drift`, exit **0** |
| `publish --from out2` (identical content) | id `b697d6e846344a32`, a **second** retained generation — no collision |
| `publish --recover` | `phase completed — nothing to do`, exit 0 |
| `build --out <evidence root>/auto` | refused: `path-overlap … resolves inside the evidence root`, exit 4 |

**The first non-empty merge** (2016 rows, the declared universe):

| Axis | Distribution |
|---|---|
| `qualification` | **215 `automated-qualified`** · 36 `not-applicable` · 1765 `unqualified` · **0 `game-verified`** |
| `automated` | 217 `pass` · 5 `mixed` · 2 `declared-not-executed` · 28 `skipped-missing-fixture` · 1764 `not-attempted` |
| `manual` | 2016 `none` |
| `fresh` | 252 `fresh` · 1764 `unknown` (nothing selected ⇒ never `fresh`) |
| `applicability` | 978 `required` · 1002 `optional` · 36 `not-applicable` |
| `conflict` / `unresolved` | empty on every row |

* **Citations** — 252 rows carry an `auto_runs` citation with its selection state, e.g.
  `operation_matrix-20260908t015600z-45fa6d93[completed+selected]`. The provenance sidecar records
  all four run digests, the `manual/` digest (the empty-tree digest — layer B untouched, V11) and
  the scope-definitions digest.
* **Declared vs executed membership** — the 28 `skipped-missing-fixture` rows are exactly the
  declared-but-absent units; the 2 `declared-not-executed` rows are `nhl15-ps4`
  `container-extract`/`container-repack`.
* **Qualified rows** span 27 distinct targets across `db-presentation` (27), `db-operations` (113)
  and `save-inner-container` (75).
* **`nhl07-ps3`** — every write/acceptance cell is `not-applicable` citing `research-only-unit`,
  **including two whose observations passed**. Applicability correctly overrides an executed pass on
  a non-shipping unit (A41).
* **A real coverage gap surfaced:** all three `nhl15-ps4` save fixtures (`ROSTER`, `PROFILE`,
  `BEAGM`) classify to **no supported save shape**, so PS4 NHL 15 has `container-detect = mixed`
  and no extraction or repack at all. This matches `save-matrix.csv` row-for-row (`unsupported`,
  `read-only`) — the contract makes it visible as `unqualified` instead of leaving it implicit.

### 5.4 Interruption, on a real run

`H:\c0-s2-int1\run2` (a copy of `run1`), `operation_matrix` started with observation on and
**hard-killed with `taskkill /F` after 45 s** — no destructor ran.

| Observed | Result |
|---|---|
| `auto/in-progress/operation_matrix-20260908t020438z-3a6b8d23/` | present, with `journal.json` (full identity), `keys-declared.txt` (**all 140** declarations) and `observations.csv` (**76** observations) |
| `auto/runs/` | still the 4 copied runs — **nothing was sealed** |
| `check` | reports `1 unsealed emitter journal(s) … they are not observations and qualify nothing`, then `ok: 2016 row(s), no validation failure, no drift`, exit **0** |
| `build` output vs `out1` | **byte-identical** — an unsealed journal contributes nothing to the merge |

### 5.5 Default-mode non-writing proof

| Check | Result |
|---|---|
| `evidence_emit` case 1 | with the variable unset: **0 files** created in the sandbox, no `auto/` directory |
| Real emitters re-run with `QA_EVIDENCE_ROOT` and `QA_WRITE` both unset (`env -u`) | `surface_matrix` **passed**, `save_matrix` **passed**; no evidence tree anywhere |
| `docs/audits/evidence/` | **does not exist** in the repository at any point |
| A root inside the repository | refused by `Emitter::from_env` (regression case) |

---

## 6. Verification

| Check | Command | Result |
|---|---|---|
| Emitter lifecycle | `cargo test -p tdb-qa --test evidence_emit` | **Passed** — 7/7 |
| Whole `tdb-qa` crate | `cargo test -p tdb-qa` | **Passed** — **158**, 0 failed, 0 ignored |
| The four emitters, observation on | `QA_EVIDENCE_ROOT=… cargo test -p studio-domain --test <each>` | **Passed** — 4/4 |
| The four emitters, default mode | `cargo test -p studio-domain --test <each>` | **Passed** — 4/4 (all four before the observed runs; `surface_matrix` + `save_matrix` re-run after) |
| Strict clippy, scoped | `cargo clippy -p tdb-qa -p studio-domain --all-targets -- -D warnings` | **Passed**, exit 0 |
| Formatting, repo-wide | `cargo fmt --all -- --check` | **Passed**, exit 0 |
| DTO bindings | `cargo run -p studio-api --bin export-bindings -- --check` | **Passed** — 70 export roots, 124 files, in sync, exit 0 |
| Full workspace suite | — | **Not run.** No cross-workspace concern: the only non-test change outside `tdb-qa` is a `[dev-dependencies]` line. |

Log and evidence locations (all outside the repository):

* `H:\c0-s2-int1\run1\` — the four real runs · `H:\c0-s2-int1\run2\` — the interrupted-run copy
* `H:\c0-s2-int1\out1\`, `out2\`, `out3\` — build outputs · `H:\c0-s2-int1\data\`, `data2\` — published sets
* `H:\c0-s2-int1\logs\` — before/after hash lists, fmt and DTO check output

### Preservation

| Check | Measured how | Result |
|---|---|---|
| HEAD | `git rev-parse HEAD` | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25`, unchanged |
| Index | `git diff --cached --name-only` | **empty** — 0 entries |
| `anchors/**` | 552 files, `sha256sum -c` against a pre-run list | **0 mismatches** |
| `docs/audits/**` | 69 files, `sha256sum -c` against a pre-run list | **0 mismatches** — the four emitter CSVs included |
| `git status --porcelain` | diffed against the list taken at package start | **exactly 3 added entries**, all intended: `M crates/studio-domain/Cargo.toml`, `?? crates/tdb-qa/build.rs`, `?? crates/tdb-qa/tests/evidence_emit.rs`. **0 removed.** 236 → 239 entries |
| Pre-existing dirty/untracked work | same diff | **preserved** — nothing reset, restored, cleaned, stashed or checked out |
| Files changed inside pre-existing untracked directories | listed in §1 | `crates/tdb-qa/src/evidence/{emit,mod,store}.rs`, `crates/tdb-qa/src/bin/qa_evidence.rs`, `defs/qa/scopes.toml`, and this receipt |
| Layer B (`manual/`) | provenance digest | `sha256:e3b0c442…` — the empty-tree digest; never written (R-SEP, V11) |
| Live publication / migration installation / staging / commit / push | — | **none** |
| Game trees, private fixtures, boot probes | — | not read, not written, not enabled |

---

## 7. Defects found and fixed

| # | Defect | Repair |
|---|---|---|
| **D1** | `defs/qa/scopes.toml` `shared-core` v1 declared `defs/metadata/**`, a path that **does not exist**. R-FRESH-UNKNOWN makes one unmatched glob poison the whole digest, and every other scope shares `shared-core` — so **all six** scope digests were permanently `unknown`, every observation would have been not-fresh, and **no row could ever have reached `automated-qualified`**. It was invisible while the tree was empty. | `shared-core` bumped to **v2** over `defs/enums/**` + `defs/relationships/**`, the two definition trees the engine actually loads. All six scopes now produce real digests. No record declared v1 (none existed), so nothing is invalidated. |
| **D2** | `qa_evidence.rs` resolved `target-triple` to `std::env::consts::ARCH` (`"x86_64"`) — a **fabricated identity** for a value the contract requires to be real or `unknown`. It also read `option_env!("RUSTC_VERSION")`, which nothing sets, so `rustc-version` was always `unknown`. | Both resolve through the shared `emit::toolchain_identity`, backed by a `build.rs` that records the real `rustc -vV` string and cargo's `TARGET`, and emits nothing when either is unavailable. A regression case asserts the triple is never the architecture string. |

---

## 8. Referred to the architect — not decided here

| # | Question |
|---|---|
| **Q1** | Should `mutation_matrix` also claim `save-raw-db/save` + `/reopen`? Its session `save()` on a copied loose `.db` *is* the raw-DB write path and it re-parses with a CRC/size check, but the test's subject is a field round-trip. Left unclaimed rather than inferred. |
| **Q2** | Seven `operation_matrix` operations have **no key in the closed `db-operations` vocabulary**: `release-then-sign`, `add-remove-membership` (free-agency/membership lifecycle), `autocorrect-jerseys`, `retire-unretire-jersey`, `repair-roster-blocks` (bulk repair / jersey administration), `trade-draft-pick` (an asset transfer moving no roster slot), `career-add-delete` (a career statistics row). They execute and assert as before but are recorded nowhere. Extend §4.1's vocabulary, or accept the gap? |
| **Q3** | `vocab::catalog_universe()` enumerates **variant-less** required cells, so a variant key (`…/container-repack/v4`) would create a parallel required cell and leave the variant-less one `not-attempted`. `save_matrix` therefore records the container version in `detail` and uses no `variant`. Should the universe enumerate variants? |
| **Q4** | Should an unsealed journal be convertible into a sealed `interrupted` run (a `qa-evidence seal-interrupted` command, or `check` doing it)? That means writing a run directory the emitter did not seal, which is a contract decision, not an implementation one. |
| **Q5** | **U3 is still open** and this package assumes the conservative reading: `QA_EVIDENCE_ROOT` **must be outside the repository**, enforced by a hard refusal. Nothing about where layer A permanently lives is decided here. |

## 9. Status

| Item | Status |
|---|---|
| **S2 (contract §11)** | **Complete for the two conditions R1 G3 named** — the four emitters are wired, and `check`/`build`/`publish`/`recover` are validated against a real non-empty tree. Whether S2 is *closed* is the architect's call; the CI gate remains **not activated** (R-NO-CI). |
| C0 | **Not complete.** This package closes G3 only. R1 G4–G6 and R2 H1–H4 carry over unchanged: no `fsync` durability, no concurrency control, hardlink aliasing undetected, file-symlink reparse coverage unavailable, the canonical adapter format gaps, checkpoint 7's unmeasured write surface, and every open product question. |
| Q-C (the `QA_WRITE=1` emitters) | The *parallel `observe` call* is now in place, as §8.3 R-TRANSITION requires. Moving the emitters **off** their in-place CSV writes is still a later bounded package; nothing was retired and no consumer was migrated. |

Stopping for Codex review.
