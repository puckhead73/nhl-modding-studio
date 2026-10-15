# Handoff R42-0 — Correction checkpoint: coordinator review R1–R3 of the 2026-09-21 research campaign applied

Research lane, 21 September 2026, 16:55–17:05 MDT. Brief: [overnight-research-2026-09-21-review-r1.md](../../validation/overnight-research-2026-09-21-review-r1.md)
§"Corrections required before further consumption". Applied in research-owned records only, with dated notes that preserve the superseded
text; nothing under application code, `defs/`, bindings, developer matrices or developer receipts was touched. Claim
`research-r42-corrections-r1-r3`.

## Plain-language summary

Three things the previous campaign said too strongly are now said exactly. (1) The Xbox 360 "slot reference" rule for descriptor word
`DW8` was a majority observation (about 90 % of descriptors) presented as a generator; it is now bounded to the tested cases, the
exceptions are counted per title, the only supported contract is *preserve the word* (which the developer's resize already does), and the
deleted scratch census is replaced by a runnable probe that also tests a candidate explanation for the exceptions. (2) The PS4 player-name
negative results excluded five *direct* joins; they do not show the id is unused, and the user card that would have "proved" consumption
by listening is rewritten as an observation with controls. (3) The coverage bookkeeping is reconciled: the NHL 13 residual that handoff 04
had already resolved is struck, the commentary contract row names revision 2b with its two coverage ranges, and the PS4 GNF counts are
split into readable / predicted-format / passing / cube / unknown-format with NHL 20 stated as not censused (its own census is running).

## 1. Where each correction landed

| Review item | File (2026-09-21 campaign) | Change |
|---|---|---|
| R1 `DW8` bound | [11-gen7-texture-descriptor-rules.md](../overnight-2026-09-21/11-gen7-texture-descriptor-rules.md) | dated note at the top: measured applicability (26 118 / 28 745 Legacy, 22 650 / 25 275 NHL 15, 20 057 / 22 129 NHL 11, 15 985 / 17 680 NHL 09), exceptions counted, "descriptor order ≠ slot order" marked as an inference; descriptor order / slot identity / byte offset declared distinct; preserving-edit contract + refusal boundary; §2 table row annotated; NHL 07 `DW12` figure marked unreliable (counter subtracted `DW8`) |
| R1 r32b probe | `local/runs/overnight-research-2026-09-21-pm/probes/r42_rw4xb2_slot_join.py` | restores the r32b logic (rule A) and adds the join test (rule B / C), tile-aligned pitch, shipped maxima and slot-size census; running over all 10 X360 trees → [handoff 02](02-rw4xb2-slot-join-and-resize-bounds.md) |
| R1 GNF pitch sample | `local/runs/overnight-research-2026-09-21-pm/probes/r43_gnf_header_census_per_title.py` | full-population pitch test per title (no sample), one checkpoint JSON per title; running NHL 20 → 15 → 17 → 18 → 19 → 16 → 21 → [handoff 03](03-gnf-per-title-census-and-resize-bounds.md) |
| R2 handoff 14 | [14-ps4-player-name-audio-binding-negative-results.md](../overnight-2026-09-21/14-ps4-player-name-audio-binding-negative-results.md) | dated note: each hypothesis restated as (key, id space, population, evidence); excludes those direct joins only; partial / namespaced / conditional / transformed / code-resident uses untested; "not consumed" not claimed; developer refusal retained |
| R2 cards | [USER-CARDS.md](../overnight-2026-09-21/USER-CARDS.md) → [USER-CARDS.md](USER-CARDS.md) (this campaign) | R40-AUD-1, R31-GNF-1, R32-TEX-1 superseded by observation-only restatements with positive controls and named next discriminators; no testing requested |
| R3 NHL 13 residual | [STATUS.md](../overnight-2026-09-21/STATUS.md) closing receipt | struck; the residual is the within-table take selection on every title (Q-R25-1), nothing NHL-13-specific |
| R3 rev. 2b | [DEVELOPER-QUEUE.md](../overnight-2026-09-21/DEVELOPER-QUEUE.md) row 1 | rev. 2b named; sentence / graffiti coverage NHL 09–Legacy vs. `CTX0` context coverage NHL 11–Legacy (09 / 10 have no `audio/context`) |
| R3 GNF counts | [15-ps4-gnf-header-rules.md](../overnight-2026-09-21/15-ps4-gnf-header-rules.md), queue row 13, atlas row 15 | readable 68 083 / 148 066; predicted-format 68 083 / 145 225; passing 68 060 / 145 136; cube 23 / 89; `0x28` 0 / 2 841; NHL 16 + 21 only; NHL 15 / 17 / 18 / 19 / 20 not censused there; mipped-cube ratio 8 × noted |
| R3 atlas | [00-ATLAS.md](../overnight-2026-09-21/00-ATLAS.md) rows 11 / 15, Q-R32-1, new §5 | annotated in place, continuation linked |

## 2. Preserved

Every superseded sentence stays in its file under the dated note; the r32b JSON stays as the record of what the deleted script measured;
the review's disposition of the accepted NHL 21 E1 test is untouched. No claim of the 2026-09-21 campaign was re-marked; the correction is
a new claim in the shared claims directory.

## 3. Not done here

The `DW8` exception generator itself (handoff 02, after the census completes), the NHL 15 / 17 / 18 / 19 / 20 GNF coverage (handoff 03),
and the resize boundary vectors (handoffs 02 / 03) — the research content the review asked for next, published as their probes finish.
