# C0-BULK1 · Checkpoint 7 — C0 closure candidate

> **Package:** C0-BULK1, checkpoint 7 of 7. **Date:** 2026-09-07. **Author role:** developer.
> Reviewer: Codex (architect).
> **C0 status: closure candidate — architect review pending. NOT complete.**
> **⚠ Corrected by C0-BULK1-R1 (2026-09-07)** — see §2.5 (an incorrect isolation claim) and §4
> (checkpoint 3 and 6 statuses, restated).
> **Nothing staged, committed or pushed.** No capability promoted; **no cell anywhere is
> game-verified**.

## 1. C0's seven requirements — artifact, verification, blocker, exit status

| # | Requirement | Implemented artifact | Executed verification | Remaining blocker | Exit status |
|---|---|---|---|---|---|
| **1** | Inventory tracked and untracked work | [c0-01](../validation/c0-01-baseline-inventory.md) | prior package | — | **completed and verified** (prior) |
| **2** | Run fmt/clippy/tests/frontend/DTO/QA drift; prove an isolated build | [c0-02](../validation/c0-02-baseline-execution.md), [c0-int1](../validation/c0-integration-01-baseline-repairs.md), [c0-dt1](../validation/c0-dt1-dto-drift-gate.md), [c0-fx1](../validation/c0-fx1-explicit-fixture-tests.md) | this package re-ran repo-wide `fmt --check`, the consolidated workspace suite and the DTO `--check` (§2) | **B1/B2 repairs live only in the working tree, not at HEAD** (C0-INT1 §7.2); B4–B7 open; Windows CI open | **implemented, awaiting a commit decision** |
| **3** | Reconcile documentation with evidence | [c0-03](../validation/c0-03-guidance-reconciliation.md) | prior package | staging disposition of two mixed files | **completed** (prior) |
| **4** | Separate generated from manual evidence; deterministic merge | contract **R2** + `crates/tdb-qa/src/evidence/**` + `qa-evidence` CLI; migration candidate | **108 cases** originally; **150 after C0-BULK1-R1** | **S2 is `partial`** — emitters unwired and `check` never validated against non-empty observations; migration candidate **not approved, not installed**; U3 undecided | **partial — S1 and S3 implemented, S2 partial** *(corrected by R1)* |
| **5** | Fixture/runtime register | [c0-05](../validation/c0-05-fixture-runtime-register.md) | measured counts (anchors, saves, paths, ignores, pairs); links resolve | **FR1–FR12**; FR2/FR3/FR12 are architect decisions | **implemented; 12 registered blockers** |
| **6** | Map every completion item to milestone / evidence cell / owner / decision | [c0-06 §6](../validation/c0-06-canonical-project-and-adapters.md) + the mechanised 2,016-row declared universe | the universe regenerates and was published in an isolated tree | **no item has an owner** | **partial — blocked on an architect decision** |
| **7** | Canonical Project schema + versioned adapter contract + registry + asset profiles | `defs/canonical/`, `defs/adapters/`, `crates/tdb-canonical` | 16 cases (5 positive over the committed files, 11 negative) | registry population **2/28 units, 1/14 families**; the adapter format **cannot yet represent** enum stored codes, null sentinels, pool behaviour, companion rules or capacity (U-A3/U-A4/U-A6) | **partial — validation machinery implemented; expressive coverage and population both incomplete** *(corrected by R1: previously "contract complete")* |

## 2. Checks run in this checkpoint

| Check | Command | Result |
|---|---|---|
| Working-tree preservation | `sha256sum -c` over the 217 baseline files | **Passed** — only the **4** files this package intended to change differ (§6) |
| Status preservation | baseline vs final `git status --porcelain` | **Passed** — **0 entries removed**, 14 added, all intended |
| Index | `git diff --cached --name-only` | **Passed** — empty throughout |
| Repo-wide formatting | `cargo fmt --all -- --check` | **Passed**, exit 0, empty output |
| Consolidated Rust suite | `cargo test --workspace --locked` | **Passed** — exit 0, **799 passed, 0 failed, 33 ignored** across 106 test binaries |
| DTO drift gate | `cargo run -p studio-api --bin export-bindings -- --check` | **Passed** — exit 0; 70 export roots, 124 binding files, *"Bindings are in sync."* |
| Repo-wide strict clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | **Passed** — exit 0, zero diagnostics |
| Frontend build/tests | — | **Not run, with reason** (§2.1) |
| Relative links in all new/edited documents | scripted check of 10 files | **Passed** — 0 broken |

### 2.1 Why the frontend checks were not run

This package changed **no file under `app/`**. Every `app/` modification in the working tree is
pre-existing and was verified byte-identical to the baseline snapshot by the preservation check
above. `npm ci` / `npm run build` / `npm test` would therefore re-measure the pre-existing state that
[C0.2 checks 1–3](../validation/c0-02-baseline-execution.md) already recorded as passing, at the cost
of a fresh `node_modules` install. Recorded as **not run**, not as passing.

### 2.2 Broad checks — side effects inspected first

Per the package's limits, each broad command's current write behaviour and private-fixture
dependencies were inspected **before** running it:

| Command | Write behaviour | Private-fixture dependency | Isolation used |
|---|---|---|---|
| `cargo fmt --all -- --check` | none — `--check` never writes | none | none needed |
| `cargo test --workspace --locked` | **does not write the DTO bindings**: `bindings::tests::live_bindings_match_the_dto_contract` asserts `check()` leaves the tree byte-identical *and* membership unchanged. `QA_WRITE` unset, so the four matrix emitters assert instead of rewriting. `QA_ASSET_OUT` unset, so both boot write probes stay disabled. **It does NOT follow that the suite has no repository write path — see §2.5.** | **reads** live private game data on `X:` and `H:\Emulators` (27 cases, C0.2 §4.5). Read-only: C0.2 §3.3 established no reachable write site derives its destination from a game root | ran on the live tree, guarded by before/after digests of `app/src/bindings/**` and `docs/audits/data/**` (§2.3) — **narrower than the claim originally made here; see §2.5** |
| `export-bindings --check` | renders to a temp dir and compares; repairs nothing | none | none needed |
| `clippy --workspace` | compile only | none | none needed |

**The prior baseline and repair snapshots were not touched.** `H:\c0-baseline\`,
`H:\c0-repair\` and `H:\c0-integration\` were neither read into nor written by this package; every
sandbox it created is under `H:\c0-bulk1\` or the system temp directory.

### 2.3 Guard digests around the broad run

| Artefact | Before | After |
|---|---|---|
| `app/src/bindings/**` (digest of the sorted per-file hash list) | `6514ed3ae83e56f60d6628ea0720dece0a51122b8f0427fd2c74f04ff22201ef` | **identical** |
| `docs/audits/data/**` (41 files) | recorded per file | **identical** — `diff` of the before/after hash lists is empty |

### 2.4 Results of the broad run

| Measure | Value |
|---|---|
| Exit code | **0** |
| Test binaries | 106 |
| Passed | **799** |
| **Failed** | **0** |
| Ignored | 33 |
| `failures:` blocks in the log | **0** |

The 33 ignored, per binary: **26** in `tdb-rx2` (the C0-FX1 private-fixture cases) plus 7
pre-existing corpus/fixture-gated cases — one each in `studio-domain` (`head_census`),
`ea_match_key`, `formula_real`, `import_corpus_e2e`, `import_history_cost`, `tdb-archive-big`
(`chunklzx_decodes_against_extracted_oracle`) and a `studio-api` doc-test. These are the cases the
[register](../validation/c0-05-fixture-runtime-register.md) enumerates.
**They are recorded as not run, never as coverage.**

The evidence and canonical work contributes **124** of the 799: 47 unit + 32 acceptance + 12
migration + 9 publication in `tdb-qa`, 16 in `tdb-canonical`, and the 8 pre-existing `tdb-qa`
inventory cases still passing unchanged.

Both guard digests came back identical, which confirms in this run what the source inspection
predicted: **the suite wrote neither the DTO bindings nor any `docs/audits/data/**` file.**

### 2.5 ⚠ Review correction (C0-BULK1-R1) — the "no repository write path" claim was wrong

This receipt originally justified running the consolidated suite on the live tree by asserting that
**"the suite has no repository write path, which was verified rather than assumed."** That assertion
is **withdrawn**. It is contradicted by evidence already recorded in this repository:

> [C0.2 §3.1](../validation/c0-02-baseline-execution.md): *"The only workspace build script is
> `src-tauri/build.rs` calling `tauri_build::build()`. It writes into `src-tauri/gen/schemas/`,
> **inside the source tree**… Four files appeared there during the run."*

So a repository write path exists, is known, and was known before I wrote the claim. Worse, my
preservation check could not have detected it: `src-tauri/gen/` is **gitignored**
(`.gitignore:17`), so it never appears in `git status --porcelain`, and the guard digests I took
covered only `app/src/bindings/**` and `docs/audits/data/**`.

What was actually established, stated exactly:

| Statement | Status |
|---|---|
| The DTO bindings were byte-identical before and after | **verified** — digest `6514ed3a…` both times |
| `docs/audits/data/**` (41 files) was byte-identical before and after | **verified** — per-file `diff` empty |
| No **tracked** file changed unexpectedly | **verified** — `git status` delta and the baseline hash check |
| `src-tauri/gen/schemas/` was not rewritten | **not hashed before the run.** Inspected afterwards: the four files carry mtimes from 2026-07-04, which is consistent with no rewrite but is **not** a byte-level proof, and mtime is not evidence |
| Any other write outside those three sets | **not measured at all** |

**Endpoint hashes do not prove the absence of intermediate writes.** A file written and restored
between the two measurements would be invisible to this check — the same limitation C0.2 §3.3
recorded about its own endpoint verification.

**The live-run deviation.** The package instructed using a fresh verified isolated snapshot for broad
checks with side effects. I ran the consolidated suite **on the live working tree** instead, on the
strength of the withdrawn claim. That was a deviation, and the correct record is: the run's isolation
rests on the three verified endpoints above, not on an absence of write paths. **Nothing observed
suggests damage** — the tracked tree, the bindings and the audit data are all provably unchanged —
but the guarantee is narrower than stated. The full suite is **not** re-run for this correction; the
package explicitly does not require it, and re-running it would not retroactively widen the
measurement.

## 3. CI — nothing added, with the unmet prerequisite named

The package permits preparing proposed Windows/QA CI changes **if prerequisites are met**. Neither
set is met, so **no workflow file was written and no gate was added**.

| Proposed gate | Prerequisite | State |
|---|---|---|
| `qa-evidence check` in CI | **R-NO-CI**: S2 must have landed *and* `check` must have run and been validated **against real inputs** | S2 has landed and `check` ran against real freshness inputs — but against an **empty evidence tree**, because `docs/audits/evidence/` does not exist in the repository and **U3 (whether layer-A runs are committed) is undecided**. A gate today would validate an absent root. **Not met.** |
| Windows CI job (C0.2 R6) | the repo-wide `fmt`/`clippy` gates must pass **at HEAD** | Both pass in the **working tree** only; C0-INT1 §7.2 records that the repairs cannot be committed standalone. A Windows job at HEAD would fail immediately. **Not met.** |

Adding either gate now would produce a failing job against unapproved or unavailable inputs, which
the package explicitly forbids.

## 4. Closure classification

### 4.1 Completed and verified

- Contract reconciliation (**R2**: nine contradictions X8–X16, thirteen new rules, V13–V15).
- The evidence core, observation, publication, recovery and migration — **108 executing cases**,
  scoped clippy and fmt clean.
- ~~Journalled publication with tested recovery from every journal phase.~~ **Superseded by
  C0-BULK1-R1**, which found the publication defective and replaced it; see
  [c0-bulk1-r1-publication-safety.md](c0-bulk1-r1-publication-safety.md).
- The fixture/runtime register, with measured counts and 12 named blockers.
- The canonical/adapter/profile **validator** — 16 cases, 11 of them negative. (The *contract's
  expressive coverage* is incomplete; see §1 item 7 as corrected.)
- Working-tree preservation: 0 baseline entries lost, 3 intended modifications, empty index.

### 4.2 Implemented but awaiting architect review / install approval

| Item | What is waiting |
|---|---|
| Contract **R2** | acceptance of the nine resolutions, including U1's closure and R-CONFLICT-CLASS |
| Migration candidate `mig-c0-bulk1` | digest `sha256:39e56bb1ff0c6c82d340bf4d0af6ff1000a867d16760b7f9139dd047b9d046d0`; **not approved, not installed**. Install command in [receipt 04](c0-bulk1-04-migration.md) |
| The whole package's source | **no commit is authorized**; nothing staged |
| Q-C emitter wiring | the four `QA_WRITE=1` emitters have **not** been given their parallel `observe` call, deliberately |

### 4.3 Blocked by a user / architect decision

| # | Decision | Effect while open |
|---|---|---|
| **U3** | Where the evidence tree lives, and whether layer-A runs are committed | blocks the QA CI gate |
| **U2** | `conversion.class` / recipe-id vocabulary | pair keys are stable but not semantically final |
| **M1 / P3** | Is `assets-jerseys` the family the legacy `uniforms` rows belong to? | 2 migration claims stay unmapped |
| **FR2** | Does a mounted game install belong behind the fixture root? | 27 cases stay machine-dependent and silently so |
| **FR3** | Outer-package fixture acquisition and storage policy | Q04's required cells have no route at all |
| **FR12** | Owner assignment | **C0 item 6 cannot close** |
| **P1, P2, P4** | Canonical model questions (stored vs computed rating; target-native extensions; asset overrides) | adapter format cannot be finalised |

### 4.4 Blocked by fixture / runtime evidence

- **26** `tdb-rx2` cases — six private fixtures absent (FR1).
- **7 + 20** cases silently dependent on mounted game trees (FR2).
- `legacyrecomp-recomp` has **no anchor** (FR4); `ps3/nhl07` has **no save fixture** (FR5).
- **No runtime build identity or deployment path** for any of the four runtimes (FR6) — which is
  precisely why R-IDENTITY withholds every possible game acceptance.
- Audio: no fixtures, no implementation (FR10).

### 4.5 Not implemented

- Registry population beyond 2 of 28 units and 1 of 14 asset families.
- Adapter-format support for enum stored codes, null sentinels, string-pool behaviour,
  companion/cardinality rules and capacity (U-A3, U-A4, U-A6).
- Any Project UI, converter or Build engine — out of scope by instruction.
- Any CI change (§3).
- Retirement or migration of any existing consumer (`game-qa-matrix.csv`, `matrix_guard`, the four
  emitters) — R-TRANSITION respected throughout.

## 5. Roadmap tracking

[docs/plans/project-completion-roadmap.md](../plans/project-completion-roadmap.md) gained a **C0
status block** immediately after the C0 exit paragraph: per-item state, links to every record, and
an explicit statement of the exit criteria that remain open (ownership; the registry's inability to
express every proven field and asset constraint; the pending migration approval). It is marked
**"closure candidate — architect review pending. Not complete."** No historical receipt was
rewritten, and no earlier record was edited.

## 6. Preservation summary

| Check | Result |
|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` — unchanged from start to finish |
| Index | **empty** at every boundary |
| Reset / restore / clean / stash / checkout / branch change / blanket staging | **none, at any point** |
| Baseline working-tree entries removed | **0** |
| Baseline files whose bytes changed | **4** — `Cargo.lock`, `Cargo.toml`, `docs/plans/qa-evidence-contract.md` (R2) and `docs/plans/project-completion-roadmap.md` (the C0 status block, §5). All intended; the other 213 are byte-identical |
| Prior baseline/repair snapshots (`H:\c0-baseline`, `H:\c0-repair`, `H:\c0-integration`) | **untouched** |
| `docs/audits/data/**` | **byte-identical** before and after the consolidated suite (§2.3) |
| `anchors/**`, `defs/` pre-existing files, game trees | not written |
| Commits / pushes | **none** |
