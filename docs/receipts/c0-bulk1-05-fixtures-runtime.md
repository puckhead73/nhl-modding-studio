# C0-BULK1 · Checkpoint 5 — Fixture and runtime register

> **Package:** C0-BULK1, checkpoint 5 of 7. **Date:** 2026-09-07. **Author role:** developer.
> **Status: Passed** — the register is complete for everything authorized evidence supports.
> Two of its blockers are architect decisions and are referred, not taken.
> Nothing staged, committed or pushed. **No fixture acquired, no game tree read or written, no boot
> probe run, no runtime launched, no policy changed.**

## Status

**Passed.** [docs/validation/c0-05-fixture-runtime-register.md](../validation/c0-05-fixture-runtime-register.md)
is the C0 item-5 register. It enumerates every fixture and runtime need the repository's own evidence
supports, with hashes where available, storage policy, blocker and next action per row.

## Files changed

| Path | Change |
|---|---|
| `docs/validation/c0-05-fixture-runtime-register.md` | **New.** The register (registers A–F, 12 consolidated blockers). |
| `docs/receipts/c0-bulk1-05-fixtures-runtime.md` | **New.** This receipt. |

No source file, no `defs/`, no `anchors/`, no `docs/audits/data/**` file was touched.

## Implemented content

| Register | Covers | Headline |
|---|---|---|
| **A** | main databases + schema metadata | 27 committed units; `legacyrecomp` has **no anchor of any kind**; `nhl07-ps3` is research-only |
| **B** | save fixtures | **26 of 27** units carry saves; `ps3/nhl07` carries **0** — the same gap that leaves `save-matrix.csv` at 26 units and makes E4's claim uncitable |
| **C** | outer console packaging (Q04) | **zero fixtures anywhere**; required on all three console platforms; a storage-policy decision |
| **D** | asset fixtures | 6 private-explicit keys (**26 cases not run**), 48 hardcoded-path lines across 11 files, the 7 remaining mounted gates, extracted trees, audio, conversion pairs |
| **E** | crate-local + generated | `default_ea_formulas.json` committed; corpus present but snapshot-excluded; tester dir present; Blender presence **unknown**; 9 further `#[ignore]`d cases enumerated |
| **F** | runtimes | 4 runtimes covering all 27 shipping targets, user-confirmed; **every build string, deployment path and acceptance record is `unknown`** |

**Measured for this register, not quoted:** anchor role counts (552 files across 8 roles), per-unit
save-file counts, 11 files / 48 lines / 30 distinct hardcoded path literals, `#[ignore]` counts per
file, and the headmesh pair/destination counts.

## Requirements of the checkpoint, and how each is met

| Requirement | How |
|---|---|
| Title/platform/build identity | registers A, B and F; **build identity is `unknown` everywhere** and is recorded as blocker FR6 rather than filled in |
| Fixture/variant | registers A–E, including PS4 container directory shapes and PS4 `.big` variants |
| Available runtime | register F, from the user-confirmed roadmap inventory only |
| Owner or owner-unassigned | **every row is `owner-unassigned`** — §8 states this plainly as the largest outstanding item against C0's exit criterion |
| Hashes where available | anchors are cited to `anchors-manifest.csv` (per-file SHA-256) rather than duplicated; the tester executable hash is cited to the roadmap; private fixtures have no hash because they are absent |
| Storage policy | per row: committed / private-never-committed / mounted-implicit / untracked |
| Blocker and next action | per row, consolidated into FR1–FR12 |
| Database / save / outer-package needs | registers A, B, C |
| Archive / texture / mesh / audio needs | register D (§5.1–5.6) |
| Crate-local fixtures | register E |
| Extracted-tree dependencies | §5.4, recorded as a distinct fixture class with an unverifiable extraction step |
| FX1's 26 converted cases | §5.1 — all six keys, all 26 cases, all recorded **not run** |
| The seven remaining mounted-data gates | §5.3, listed individually, with the wider 27-case C0.2 §4.5 set |
| Returned-early tests are not coverage | stated as a rule in §5.1 and again in §10; no row describes a skip as coverage |
| "not found at inspected locations" ≠ "does not exist" | defined in §1.1, applied in every row, restated in §10 |
| The two former texlib sources | §5.1a — shared **logical role** recorded; **physical provenance recorded as unresolved**; byte identity explicitly **not** claimed (blocker FR8) |
| No fixture acquisition / policy change | §10 and the header; FR2 and FR3 referred to the architect unchanged |

## Findings this checkpoint produced

| # | Finding |
|---|---|
| 1 | **`CUSA11126` vs `CUSA11116`** — a hardcoded shadPS4 path names content id `CUSA11126` while `discovery::ps4_title_ids` maps `nhl19` to `CUSA11116`. One digit apart. Which is right, or whether both are real variants, is **unknown**. Recorded as FR9; **no code was changed and neither value was "corrected"**. |
| 2 | **30 distinct hardcoded literals across 11 files**, versus C0.2's *33 paths across 11 files*. Two of C0.2's four roots are gone (removed by C0-FX1); the residual difference is a counting-method difference and is **not** claimed as a further reduction. |
| 3 | **Three bare `X:\rendering\…` literals name no title.** Which title's tree they belong to is unknown and is not guessed. |
| 4 | The **runtime-identity gap and the evidence contract line up exactly**: with every `runtime_build` unknown, R2's R-IDENTITY withholds every possible game acceptance. No cell can be game-verified today for a *recorded* reason rather than by accident. |

## Exact checks and results

| Check | Result |
|---|---|
| Anchor file/role counts | **Passed** — 552 files; role histogram from `anchors-manifest.csv` |
| Per-unit save-file counts | **Passed** — enumerated by directory walk of `anchors/*/*/savefiles` |
| Hardcoded path scan (`crates/**/*.rs`) | **Passed** — 11 files, 48 lines, 30 distinct literals; list saved |
| `#[ignore]` census | **Passed** — per file, whole `crates/` tree |
| Headmesh pair/destination count | **Passed** — 14 pairs, 8 destinations (agrees with checkpoint 4) |
| Relative links in the register | **Passed** — every target exists |
| Any fixture acquisition, game-tree read/write, boot probe, runtime launch | **Not run** — none attempted |

## Evidence / log locations

- `H:\c0-bulk1\logs\cp5-abs-paths.txt` — the 30 distinct hardcoded literals.
- `H:\c0-bulk1\logs\cp5-files-with-abs-paths.txt` — the 11 files.
- The register itself carries every count inline.

## Preservation checks

| Check | Result |
|---|---|
| HEAD / index | `a2bbdc4…` / **empty** |
| `anchors/**` | **read only** — file listing and the committed manifest; no anchor opened for content, none written |
| Game trees on `X:` and `H:\Emulators` | **not touched.** No path outside the repository was opened by this checkpoint |
| `corpus/` | listing only |
| Source files | **none modified** |
| Files written | 2 documents |

## Unresolved decisions / blockers

Full list in the register §9 (FR1–FR12). The ones that **block other work**:

| # | Blocker | Owner |
|---|---|---|
| **FR2** | Whether a mounted game install belongs behind the same fixture root as private files. C0-FX1 §5.4 referred this question to C0's register; the register **records it and does not answer it**, because a new mounted-install policy needs approval. Until it is answered, 7 `tdb-rx2` cases (and 20 more elsewhere) stay machine-dependent and silently so. | **architect** |
| **FR3** | Outer-package fixture acquisition and storage policy, against EA data policy rule 2. | **architect** |
| **FR12** | Every row is owner-unassigned. This is the outstanding half of C0's exit criterion; test routes and blockers are now enumerated, owners are not. | **architect** |

FR1, FR4–FR11 are recorded blockers that need fixtures or evidence, not decisions.

## Prerequisites for the next checkpoint

None. Checkpoint 6 (canonical Project schema and adapter contracts) depends on the accepted Q07a
decisions and the committed schema snapshots, not on any register row — though the register's
unknowns are exactly what checkpoint 6 must be able to express as blockers rather than defaults.
