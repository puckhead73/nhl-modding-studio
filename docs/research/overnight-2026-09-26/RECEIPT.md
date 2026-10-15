# Consolidated receipt — research lane, 26 September 2026 (01:06 → 01:26 MDT, real clock)

Continuation of the 2026-09-25 session under [research-20260926-r102-r106-review.md](../../validation/research-20260926-r102-r106-review.md).
Everything below is researcher-verified and pending review; nothing is game-verified; no writer exists; runtime and visual acceptance
remain the user's.

## Plain-language results

The review's two wording fixes were published first. Then six packages. The logo family everyone called "rink logos" turns out to be a
set of three small pictures per file whose purpose the files do not reveal; a short visual check is carded. The PlayStation 3 NHL 12 / 13
kit files, read straight from their archives, have the same empty reference tables as the other titles, and NHL 13 contains a nameplate,
used by a real Hershey Bears kit, whose internal picture name does not match its file name — a shipped case worth one in-game look. A
rename readiness assessment separates six layers and names the one narrow case that could be designed offline. The five archive-only
titles' sound banks now have a per-bank source map for the developer's next step, and the banks inside those archives turn out not to be
compressed. Finally, the Swiss league's schedule format is measured for the first time, and the NHL quotas are confirmed on the genuine
NHL 20 file; the other eleven league tables are classified as round robins, group rules or ~~real-season replicas~~ shipped irregular schedules (see the correction below).

> **Correction 2026-09-26 02:36 (review `research-20260926-r107-r112-review.md`):** rows 3, 5 and 6 below are corrected in their
> handoffs by dated notes — Swiss groups are not encoded by the six inspected `exhibitionteams` group fields (not "not in the DB");
> irregular tables are shipped irregular schedules with no general rule established (not "real-season replicas"); 4 / 6-game partitions
> are stock-shaped inputs, not universal rules; the R109 pool is absence from enumerated columns and families (centre-ice logos excluded)
> and the DB byte mask includes the required CRC updates. Continuation receipt: [RECEIPT-2.md](RECEIPT-2.md).

## Packages

| # | Handoff | Result | Evidence class |
|---|---|---|---|
| 0 | [00-checkpoint-r102-r106-review.md](00-checkpoint-r102-r106-review.md) | inspection-graph edge wording (decoded empty tables + three constant values; scoped aliases); logo family "overlapping, not superset / grid"; queue C9 reworded; graph v1 preserved | presentation |
| 1 | [01-logo-family-characterisation.md](01-logo-family-characterisation.md) | 3 rasters per member (256×128, 128×64, 128×128 DXT5) on four trees; 127–132 ids without kit files; scoped-negative consumer search; hypotheses H1–H3; card L-LOGO-1 | measured / hypotheses |
| 2 | [02-ps3-12-13-payload-references.md](02-ps3-12-13-payload-references.md) | 12 131 archive members parsed, declared = decoded, empty reference tables; NHL 12 two-file socks; free-id count unchanged by the sock correction; NHL 13 internal-name mismatches incl. DB-referenced `name_1_138_2_cm`; card K-NAME-1 | measured |
| 3 | [03-rename-rekey-readiness.md](03-rename-rekey-readiness.md) | six layers; same-length same-directory outer rename designable offline under stated conditions; destination pool 746–761 ids per tree (DB + archive + loose) | assessment + measured pool |
| 4 | [04-archive-backed-bank-source-map.md](04-archive-backed-bank-source-map.md) | per-bank source map for PS3 09 / 10 / 12 / 13 and X360 09; bank / stream members stored unwrapped (R82 label corrected); no disagreeing copies | derived + measured |
| 5 | [05-schedule-provenance-and-swiss-format.md](05-schedule-provenance-and-swiss-format.md) | R13 NHL 18–21 quotas confirmed on the real NHL 20; Swiss = 3 groups of 4, 6 within / 4 across, groups not in the DB; ledger-lag note | observed data |
| 6 | [06-league-schedule-formats.md](06-league-schedule-formats.md) | 11 league tables classified: round robin ×4 (SHL, Allsvenskan, DEL, Czech), group rule (Swiss, Finnish, Austrian), replicas (ECHL, OHL, QMJHL, WHL) | observed data |

## Corrections placed (dated notes; nothing deleted)

Handoff 15 (2026-09-25) presentation; handoff 14 "overstated free ids" withdrawn; R82 handoff 01 bank wrapper labels; R13 handoff 06
(2026-09-20) NHL 20 provenance and Swiss. Queue history preserved in `overnight-2026-09-25/DEVELOPER-QUEUE-HISTORY.md`.

## Current developer queue

[`overnight-2026-09-25/DEVELOPER-QUEUE.md`](../overnight-2026-09-25/DEVELOPER-QUEUE.md): rows C1–C12 (C10 bank source map and C11
schedule inputs + ledger refresh, C12 remaining league formats added today; C9 reworded).

## Cards (user; none blocks research)

L-LOGO-1 (what the three logo rasters depict — asset view only), K-NAME-1 (NHL 13 PS3 Hershey third kit nameplate renders?); carried:
T-CHANT-1 rev. 3, A-SPX-1 rev. 3, A-AST-1, A-LOOP-1, T-ARTID-1, H-HORN-1 rev. 2.

## Claims

`local/coordination/overnight-2026-09-26/claims/`: p1 – p6, all `ready`. 2026-09-25 claims r105 / r106 carry revision notes. No active
developer claim overlaps; developer receipts 20–23 read.

## Cleanup

Session scratchpad empty (patch scripts deleted after use). No background process of this lane. Kept: probes r107–r112 and run logs
r107 / r108 under `local/runs/overnight-research-2026-09-26/`; JSON outputs under `data/`. Nothing written to `X:`, `H:` outside the
scratchpad, `anchors/`, `defs/`, bindings, application or developer paths. Nothing staged, committed or pushed. Two recurring errors
recorded and corrected in place: estimated time stamps in handoff headers (replaced by file times) and a shell edit that injected a NUL byte
into a probe (removed).

## One exact next task

Test whether the Finnish and Austrian schedule groups are recorded in any database field (as done for Swiss in R111b), then, if not, publish
the stock group partitions per title as a small import table for the generator (read-only; derived from `finnishschedule` /
`austrianschedule`). After that, return to the next League / Team ledger item (1.2 tournament slot semantics for `championsschedule` /
`spenglerschedule`).
