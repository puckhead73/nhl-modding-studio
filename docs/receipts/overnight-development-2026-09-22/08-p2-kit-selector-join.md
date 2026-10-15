# Receipt 08 — Priority 2.6 (gen7): a team's kits joined to the textures that dress them

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
HEAD `8d60a6d` (unchanged). Real Xbox 360 trees and committed anchors read **only**. Nothing staged,
committed, pushed, launched or written live. Claim: `dev-p2-kit-selector-join`.
Research consumed (unedited): 2026-09-21 pm handoff
[14](../../research/overnight-2026-09-21-pm/14-gen7-kit-selector-join.md) (`research-r56-gen7-kit-selectors`).

> **Superseded in part (22 Sep 2026, coordinator review F1 → receipt 10):** the resolver this
> receipt describes decided which components a kit *needs* from the directory listing, so a tree with
> no `sock/` directory made jersey + pant pairs "complete" kits and "complete spares". That rule is
> withdrawn: the contract now comes from the title / platform, every required component (both sock
> files on NHL 11 / 12) is listed with present / missing / unknown evidence, and one rule judges rows
> and spares alike. The real-tree counts below were produced with the old rule; receipt 10 re-measures
> them under the corrected one (they agree on the complete counts because those trees carry socks).

## 1. Plain-language summary

A team's jerseys were already readable as database rows — style, variant, "is this the default",
"does it have a tie" — but nothing connected a row to the three textures that actually dress the
player: the jersey, the pants and the socks. Studio now makes that connection, and on the three real
games checked it finds **every single one**: all 627 kit rows on NHL 11, all 659 on NHL 15, all 703
on NHL Legacy.

It also lists the spare kits — sets of three textures sitting in the game that no row points at
(97, 54 and 228 on those titles). Those are the legitimate targets if a kit is ever re-pointed, so
knowing them is what makes a safe reassignment possible later.

Two things are refused rather than fudged: custom-team kits, which the research measured as having
no per-team files at all, and any row whose team art id disagrees with its team's — a mismatch that
never occurs in shipped data and would mean something is wrong rather than something to guess at.

This is a read-side join. It does not change a kit, and it does not claim which kit the game picks
for a given game.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/kit_files.rs` | **new** — `KitPart`, the measured filename templates, `kit_file` / `sock_files`, `shipped_triples` (the spare pool), `team_art_ids`, and `team_kits` producing the per-team view with flags and refusals. |
| `crates/studio-domain/src/lib.rs` | module registration; `team_tabs` made `pub` (its `TeamJersey` rows are this join's input). |
| `crates/studio-domain/tests/kit_selector_join.rs` | **new** — the real-tree join over every team of three anchors. |

No API, DTO, binding or UI change yet: this package lands the join and its evidence; the Uniforms
surface is the next slice.

## 3. Evidence

`cargo test -p studio-domain --test kit_selector_join -- --ignored`:

| Title | Teams with rows | Rows | **Complete** | Incomplete | Refused | Sock files | Spare kits |
|---|---|---|---|---|---|---|---|
| NHL 11 X360 | 212 | **627** | **627** | 0 | 0 | 627 | 97 |
| NHL 15 X360 | 216 | **659** | **659** | 0 | 0 | 659 | 54 |
| NHL Legacy X360 | 222 | **703** | **703** | 0 | 0 | 703 | 228 |

The row counts equal R56 §1's census exactly (627 / 659 / 703), and the join rate is the handoff's
100 %. The teams-with-rows figures (212 / 216 / 222) also match its "teams with rows" column.

Unit cases (3): the triple resolves all three families and the sock carries its `_cm` suffix; the
spare pool contains only *complete* unused triples (a jersey-only triple is not offered); custom-team
and art-id-mismatch rows are refused with their reasons and attempt no files; without a rendering
tree the kits list and claim no files.

### The sock template: a wrong reading, caught and corrected

R56's note gives one sock template (`sock_<style>_<artid>_<variant>_cm`). Implemented that way, the
real-tree test failed loudly: **0 of 627** NHL 11 rows resolved. Investigating rather than relaxing
the assertion showed jersey and pant resolving fine and only socks missing, and a directory listing
showed NHL 11 socks carry an extra sub-index — `sock_<style>_<0|1>_<artid>_<variant>_cm`, which is
R56 §1's "two per kit" remark, confirmed by the count: **1 512 = 2 × the 756** the census recorded.
(An intermediate reading of a differently-named sibling family in the same directory briefly
suggested "six numeric fields, no `_cm`"; the file count is what settled it.)

Both forms are now implemented and the resolver picks by **looking at the tree**, not by a title
list. The test pins the outcome per title, so neither form can silently stop working.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --test kit_selector_join -- --ignored` | **1 passed** (three trees) |
| `cargo test -p studio-domain --lib kit_files` | 3 passed |

## 5. Coverage effect (Phase 2 matrix 2.6, gen7)

| Row | Before | After |
|---|---|---|
| Kit entries: ordering / selection / components | the `stockteamjerseys` rows were decoded; no join to the textures ("surfacing those arrives with the Assets phase") | **the static join: impl+tested (dev)** — `(style, team art id, variant)` → jersey / pant / sock at 100 % of rows on NHL 11 / 15 / Legacy, both sock forms measured, the shipped spare pool listed per team |
| Reassignment validation set | not available | **available** — the spare pool is exactly R56 §2's "triples with all three files"; no writer yet |
| NHL 09 / 10 | no table | unchanged — refused, selection mechanism unknown |
| Custom teams (art id 20054) | — | refused with the measured reason (0 / 60 resolve) |
| Which kit the game dresses a team in | unknown | unchanged — runtime, not claimed |

## 6. Open / not done

- **No UI yet**: the Uniforms tab does not show the resolved textures. That is the next slice and
  needs no new evidence.
- **PS3** is untested here (the trees are `.rpsgl`; the resolver tries that extension first when
  told the platform is PS3, but no PS3 tree was walked in this package).
- **NHL 12 / 13 / 14** are not in the test list (11, 15 and Legacy span both sock forms and both
  ends of the era; adding the middle three is cheap and unclaimed until done).
- No reassignment, no new-kit creation: both need a writer, validation against the spare pool and a
  runtime card.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `kit_test_edit.py`, `kit_sock_fix.py` | deleted at checkpoint |
| Temp roots | the unit tests' `nhlms-kit-*` directories remove themselves on drop |
| Background processes | none |

## 8. Next action

Surface the join on the Uniforms tab (the resolved jersey / pant / sock per kit, the flags, the
spare pool), then extend the real-tree test to NHL 12 / 13 / 14 and a PS3 tree.
