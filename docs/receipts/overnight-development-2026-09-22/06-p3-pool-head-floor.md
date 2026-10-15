# Receipt 06 — Priority 3/4: the pool-head floor follows the title's era (a defect the research named)

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
HEAD `8d60a6d` (unchanged). Committed anchors read only. Nothing staged, committed, pushed, launched
or written live. Claim: `dev-p3-pool-head-floor`. Research consumed (unedited): 2026-09-21 pm handoff
[11](../../research/overnight-2026-09-21-pm/11-gen7-player-portrait-head-containment.md) (`research-r53-gen7-player-assets`) §1–2.

## 1. Plain-language summary

Most players in these games share a small set of generic head models; a few hundred stars have their
own. Studio told the two apart partly by the head's id number, using the rule "anything from 60 000
up is generic" — which is how NHL 14 and later number them. The research lane measured every gen7
title and found the earlier games (NHL 09 through 13) number their generic heads from **10 000**
instead. On those five titles Studio's rule matched nothing, so it reported that the game had no
generic heads at all.

Studio also has a second, independent test — "is this head worn by more than one person?" — which
kept the visible behaviour mostly right, and that is why this never showed up as a broken screen.
But the id rule was still wrong, and it is the one that catches a generic head worn by exactly one
player, which the other test cannot see by definition.

The fix does not hard-code a list of games. Studio now looks at the file it has open: if any head id
reaches 60 000 the game uses the later numbering, otherwise the earlier one. A patched or
hand-modified database therefore classifies itself correctly instead of being described by a table
of what someone else's copy looked like.

## 2. The defect and the fix

Research R53 §2, verbatim: *"classify bespoke / pool by `headid == artid` and the era's pool range
(10 000+ on 09–13, 60 000+ on 14+ — **Studio's 60 000 rule is wrong for 09–13**)"*.

| | Before | After |
|---|---|---|
| Pool floor | `POOL_HEAD_MIN` = 60 000, fixed | measured per file in `scan_player_assets`: 60 000 when any head id reaches it, else `POOL_HEAD_MIN_EARLY_GEN7` = 10 000 |
| NHL 09 / 11 / 13 pool population | **0** | 3 711 / 5 053 / 5 142 players |
| Consequence for `mesh_path` | a pool head on 09–13 was only kept out of "this player's own head" by the uniqueness rule; a generic worn by exactly one player slipped through | the id rule catches those too (NHL 09's `10097`, NHL 11's `10097` / `10128`, NHL 13's three) |
| `head_shared` (the portable signal) | unchanged | unchanged — it remains the primary discriminator, and the two now agree on 99 %+ of players |

Why a measured floor and not a title list: `scan_player_assets` has no title parameter, and adding
one would ripple through every caller for a value the data already carries. Bespoke head ids are
portrait ids (buckets 0–12 000), so the 10 000 floor is only ever consulted on a file that has no
60 000+ ids at all — the two id spaces do not overlap in practice.

## 3. Evidence

`cargo test -p studio-domain --lib the_pool_head_floor -- --nocapture`
([r1-p3-pool-head-floor.log](../../validation/evidence/overnight-development-2026-09-22/logs/r1-p3-pool-head-floor.log)):

| Anchor | Floor chosen | Pool heads / players | Research figure |
|---|---|---|---|
| X360 NHL 09 | 10 000 | 3 711 / 3 968 | 3 712 "players_other" |
| X360 NHL 11 | 10 000 | 5 053 / 5 310 | 5 055 |
| X360 NHL 13 | 10 000 | 5 142 / 5 412 | 5 144 |
| X360 NHL 15 | 60 000 | **5 275** / 5 534 | **5 275** (exact) |
| X360 NHL Legacy | 60 000 | **5 385** / 5 632 | **5 385** (exact, the top of the handoff's 3 663–5 385 range) |

The small residuals on 09 / 11 / 13 (1–2 players) are the head-id-0 rows, which Studio excludes from
the pool by construction.

Asserted per title: the presence of 60 000+ ids matches the expected era; the pool population is
non-empty; every pool head is at or above the era floor; **no pool head is credited with an own
mesh**; ≥ 99 % of players on a pool head are on a *shared* head (the two independent signals agree);
and — the regression the fix closes — on the early titles the **old** fixed floor would have found
zero pool heads.

### A captured failure, and what it taught

The first version asserted that *every* pool head is shared. It failed on NHL 09 with id `10097`, and
then on Legacy with 12 of 201 ids. Those are real: a generic head worn by exactly one roster player.
Rather than delete the check, it now measures the agreement over **players** (≥ 99 %) and prints the
singly-worn ids — because those ids are precisely the cases the uniqueness rule cannot see and the
era floor can. The second threshold attempt (5 % of distinct ids) was also wrong for Legacy and was
replaced for the same reason: the meaningful population is players, not ids.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib` | **180 passed, 0 failed, 2 ignored** |
| `cargo test -p studio-api --lib` | **31 passed** after updating the deliberate export-root count guard from 126 to 128 (the two arena-venue DTOs of receipt 05) |
| `cargo test -p studio-domain --lib assets` | 55 passed |

## 5. Coverage effect (Phase 3.4)

| Row | Before | After |
|---|---|---|
| Player head classification, X360 / PS3 **NHL 09–13** | pool range not recognised (fixed 60 000 floor) — the pool population read as empty | **impl+tested (dev)** — the era floor is measured per file; pool populations match research R53 on five anchors, two exactly |
| Player head classification, NHL 14–Legacy / PS4 | unchanged | unchanged (60 000), now stated as measured rather than assumed |
| What the game draws for any of these | not established | unchanged — no runtime card; this corrects a *classification*, not a write path |

## 6. Open / not done

The rest of handoff R53 is **not** implemented in this package and stays open:

- the per-title portrait / head **file containment** card on every gen7 title (Studio's existing
  asset card is Legacy-shaped);
- `hasportrait` as a display flag that must never be derived from, or written from, a file check
  (93 NHL 13 / 127 Legacy shipped contradictions) — Studio does not currently derive it, but the
  guarantee is untested;
- the "shipped without asset" refusals (NHL 09's 5 bespoke heads and 5 portraits);
- reassignment validation against the unused shipped id sets.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `p3_pool_floor.py`, `p3_pool_test.py` | deleted at checkpoint |
| Evidence `r1-p3-pool-head-floor.log` | retained |
| Temp roots / background processes | none |

## 8. Next action

The rest of R53 §2 as its own package: the gen7 portrait / head containment card, with
`hasportrait` shown as the DB flag it is and the "shipped without asset" refusals listed.
