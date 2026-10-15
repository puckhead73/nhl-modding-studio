# C0-BULK1 · Checkpoint 1 — Reconcile the evidence contract

> **Package:** C0-BULK1 (bulk C0 closure candidate), checkpoint 1 of 7.
> **Date:** 2026-09-07. **Author role:** developer. Reviewer: Codex (architect).
> **Status: Passed.** Nothing staged, committed or pushed. No generator run, no fixture touched,
> no capability promoted.

## Status

**Passed.** The contract's internal contradictions were resolved before implementation began.
[docs/plans/qa-evidence-contract.md](../plans/qa-evidence-contract.md) is now at **R2** and is the
specification checkpoints 2–4 implement.

## Files changed

| Path | Change |
|---|---|
| `docs/plans/qa-evidence-contract.md` | R2 revision: status banner + R2 scope note; §4.1 required-evidence table replaced with an operation-class table; §4.2 selection rewritten per-key; §4.3 axes + qualification table extended; §5.1 gains V13–V15; §10 A9 corrected; new §10.1 (A33–A45); new §12 (X8–X15, R2 rules, U1 closed) |
| `docs/receipts/c0-bulk1-01-contract.md` | **New.** This receipt. |

`docs/receipts/` was created by this package.

## Implemented behavior — the eight contradictions and their resolutions

| # | R1 contradiction | R2 rule |
|---|---|---|
| X8 | A **`failed`/`interrupted`** run's `supersedes` removed the completed run it replaced, erasing the key's only evidence | **R-SUPERSEDE-STATE** — only a `completed` run's supersession has effect |
| X9 | `supersede_mode = "partial"` had no defined effect; literal implementation dropped the whole superseded run | **R-PARTIAL-KEYS** — supersession suppresses only explicitly covered keys; V13 forces every `supersede_keys` entry to be declared by both runs |
| X10 | An executed `fail` inside a non-completed run collapsed to `not-attempted`, letting another run's pass qualify the key | **R-EXECUTED-FAIL** — new axis value `executed-unselected`; new qualification row **4a** (`withheld`, `reason = unreviewed-failure`) for any *uncleared* executed failure, above the pass/freshness/identity rows. A failure clears only when a **completed** run that supersedes the observer *at that key* executed it and passed |
| X11 | Timestamps were present everywhere while R-NOWIN forbade recency selection | **R-NO-TIME** — no timestamp, mtime or directory order participates in any decision |
| X12 | R-ABSENT promised "a missing emitter erases nothing" while §8.1 rebuilds the published set wholesale | **R-CITE-ALL** — every row republishes every citation and conflict from every present input, selected or not |
| X13 | `game-acceptance` blanket-required for read-only families and for the research-only `nhl07-ps3` unit | **R-APPL-OPS** — operations are classed `read-only`/`write`/`acceptance`; acceptance is required only where a required **write** exists; `UnitKind::Research` write/acceptance cells are `not-applicable` citing roadmap §1 |
| X14 | §4.4's roll-up scope `(target, family)` re-collapsed the distinct conversion pairs R-CONV had separated | **R-CONV-PAIR** — roll-up scope is `(target, family, conversion)`; source and destination both preserved; pairs enumerated from the recipe registry, never from available evidence |
| X15 | R-APPL required a cited reason for `not-applicable`; no rule enforced it | **V15** — an uncited `not-applicable` is a hard error |

Also added: **V14** (`supersede_mode` well-formedness).

## Regression cases added

**A33–A45** (contract §10.1), one per rule above plus A45 for V15. Verified as executed unit tests in
checkpoint 2 — see [c0-bulk1-02-evidence-core.md](c0-bulk1-02-evidence-core.md).

## Corrections to existing examples/tables

- **§4.1** — `Required` re-labelled *declared required*, with a per-operation class column; a note
  states that the final required set is resolved by R-APPL-OPS.
- **§4.2** — the row *"only failed/interrupted runs declare the key ⇒ `not-attempted`"* split into
  two rows: executed (⇒ `executed-unselected`) and not executed (⇒ `not-attempted`).
- **§4.3** — `automated` vocabulary gains `executed-unselected`; qualification table gains row 4a.
- **§10 A9** — PS3 NHL 07 `save-raw-db` now expects `applicability = not-applicable`
  (`research-only-unit`) rather than R1's `unqualified`. The E4 refutation of the
  `edit-roundtrip-pass` claim is unchanged and still asserted.

## Decisions taken, and their authority

Every R2 narrowing is derived from already approved guidance, not from a new product decision:

| Change | Authority |
|---|---|
| Research-only write/acceptance cells are `not-applicable` | roadmap §1: *"NHL 07 PS3 is research-only, not a shipping target."* |
| Game acceptance still required for every write family on shipping targets | roadmap §1 *"including game acceptance for writes"* + Q02 |
| Read-only families never require game acceptance | the absence of any approved requirement; R1 invented it |
| Q-A – Q-E, R-SEP, V11, R-Q04, R-NO-BACKFILL, R-CLAIM, R-FREEZE | preserved verbatim; R2 reopens none |

**U1 closed** (§12.8) — in the part approved guidance settles, and only that part.
**U2 and U3 remain open** and block nothing: no run directory is written into the repository by this
implementation, and every command takes its evidence root as an argument.

## Exact checks and results

| Check | Result |
|---|---|
| Contract reparsed end to end after editing (headings, table widths) | **Passed** — see checkpoint 2 log for the fenced-example reparse |
| Every rule in §12 traced to the R1 text it contradicts | **Passed** — X8–X15 each quote the contradicting R1 clause |
| No closed decision reopened (§9 Q-A – Q-E, R-SEP, V11, R-Q04) | **Passed** — all present and unmodified |
| No validation rule removed or weakened | **Passed** — V1–V12 unchanged; V13–V15 added |
| Manual-evidence rules unchanged | **Passed** — layer B untouched by every R2 rule |

## Evidence / log locations

- Contract: [docs/plans/qa-evidence-contract.md](../plans/qa-evidence-contract.md) §§4.1–4.3, 5.1,
  10.1, 12.
- Baseline snapshot for the whole package: `H:\c0-bulk1\` (logs), scratchpad
  `baseline-status.txt` / `baseline-hashes.txt` (222 paths, 217 files hashed; 5 are directories).

## Preservation checks

| Check | Result |
|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` — unchanged |
| Index | **empty** before and after |
| Reset / restore / clean / stash / checkout / branch change | **none** |
| Files written by this checkpoint | 2 (`qa-evidence-contract.md`, this receipt); no other working-tree path touched |
| Pre-existing tracked modifications and untracked files | 222 paths recorded before editing; both files changed here were already untracked/modified before the package and remain so |

## Unresolved decisions / blockers

| # | Item | Owner |
|---|---|---|
| U2 | `conversion.class` vocabulary should derive from the same registry as `headmesh-matrix.toml` | architect |
| U3 | Whether layer-A run directories are committed to git or retained out of tree | architect / user |
| — | R2 itself requires architect acceptance before the contract is treated as accepted guidance | Codex |

None blocks checkpoints 2–6.

## Prerequisites for the next checkpoint

R2 §§2–5 and §12 are the S1 specification. Checkpoint 2 needs no fixture, no runtime and no
decision beyond R2.
