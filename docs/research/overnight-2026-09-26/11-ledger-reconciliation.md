# Handoff R117 (P11) — developer League / Team ledger reconciled with ready research (as of 26 September): which rows can move, which cite withdrawn research, which are unchanged; archive-backed audio (C10) and the chant wording (C5) stay first; schedule, tournament and date-encoding inputs added; no ledger edit

Research lane, 26 September 2026, published 03:05 MDT (system clock). Review `research-20260926-r107-r112-review.md`, queue item 6.
Claim `research-p11-ledger-reconciliation`. Read: `docs/receipts/overnight-development-2026-09-24/LEDGER.md` (24 Sep ledger, 24 Sep
outcome table, 25 Sep refresh), receipts 20–23, the 26 Sep packages 20–22 development review, current queue C1–C14. The ledger is
developer-owned and is **not** edited; this table is the research side of the next refresh.

## Plain-language summary

The developer keeps a ledger of what League and Team editing can do and what is waiting on research. Several of its "waiting on
research" entries now have answers, a few still quote research wording that has since been withdrawn, and some remain genuinely open.
The table below lines them up. The most useful next step for the developer is unchanged: read the sound banks that live inside the
archives of the older titles, using the map published earlier today. Schedule generation now has group data and a checker to work
against, but only as a proposal; the tournament tables have safe browsing labels but still nothing that could be edited.

## 1. Reconciliation

| Ledger row (state as written) | Research ready now | Suggested ledger state | Queue |
|---|---|---|---|
| 2.7 / 1.5 archive-backed PS3 09 / 10 / 12 / 13, X360 09 banks — "ready; next package" | per-bank source map; **all bank / stream members stored unwrapped** (R82 labels corrected); X360 09 loose alias byte-identical | ready (unchanged); use the map, no decompression step for `.abk` / `.ast` members | C10 |
| chant wording on the arena page — "small wording package" | R97 wording (club banks 055–066; only the Finnish arena's ids equal a club number; no selector traced) | ready (unchanged) | C5 |
| cc 3 / 5 de-interleave, loop stub + body preview — "ready, bounded" | unchanged (R92 / R96 vectors; loop rule not a replacement contract) | ready, bounded | C1 / C2 |
| EA-Speex / EA-XMA previews — "decision" | unchanged; developer / coordinator decision | decision | — |
| 1.2 Schedule generation — "NHL / Swiss / AHL blocked — the stock division-weighted format is not established" | NHL: R13 era quotas (NHL 18–21 row confirmed on the real NHL 20); European leagues: stock partitions + quotas + validator on 19 sources (R113), generator facts (R114: formula, 4-bit slots, **Austrian two-phase `round`**, date-order exceptions); AHL / CHL / ECHL: no rule — import / user quotas | **"format not established" is out of date**; the proposal is not an implementation contract (review 2026-09-26) — a generator package would start from handoff 08's refusal list | C11 / C12 / C13 |
| 1.2 Tournament ↔ league binding — "research unknown" | slot classes (participants / placeholders / seeds); WC consistent join; CHL participant **candidate** (`ischampionsleague`, 32 clubs, PS4 18–21); Spengler placeholder reading; competing interpretations kept | browse labels only (round names, "to be decided", seeds, WC group stage by name); no slot assignment | C14 |
| 1.2 Season / Franchise copy of the schedule — "research unknown (save side)" | **already researched 18 Sep** (P1-B handoff 21 §2): 44 of 46 day-1 mode saves hold a copy corresponding to the stock NHL / AHL source under the `leagueteams` mapping, exceptions A–C named; Franchise preseason rows (`round` 6 / 8) in no source; edited-input behaviour unresolved (card P1B-3). European-league copies: **no specimen** (all committed mode saves are NHL modes) | "observed correspondence (stock); edited-input behaviour runtime" — not "research unknown" | new note (this row) |
| 1.2 Conference / division names, NHL 08–11 — "research unknown" | unchanged since 18 Sep (08–11 store a 1-bit local conference + 3-bit division; no name source found) | research unknown (unchanged) | — |
| 1.2 group names 77 / 78, NHL 17 id mechanism | 77 / 78 derived names published 21 Sep (R36 §2: "NHL Alumni / All-Time (single group)", "Pro-Am template teams (single group)"; 75 / 76 Spengler groups); NHL 17 mechanism unchanged | 77 / 78 ready as derived labels | — |
| 2.1 art-id re-key — cites "archive entry = name + djb2 hash word" | **withdrawn** (2026-09-25 handoff 14 §F1): row = `block<<4 | 0 | size | djb2(path)`, the name lives in the fixed-width name-table slot; readiness: six layers, same-length same-directory outer rename designable offline only (handoff 03, corrected today: pool = absence from the enumerated sources, centre-ice logos excluded, CRC updates in the byte mask) | re-key not impl (default refuse) — replace the cited wording | C7 / C9 |
| 1.2 / 2.2 creation — cites "PS4 18–21 non-NHL only, with a named row set" | **withdrawn** (2026-09-24 handoff 06 §F2): the stock rows are what a season ships, not requirements; remaining: measured spare capacity + a proposed experiment | not impl — replace the cited wording | C11 (ledger note) |
| 2.6 kit name keys — "ready, low impact" | nameplate = per-row naming association (R101 / handoff 14); NHL 13 PS3 shipped internal-name mismatch incl. DB-referenced `name_1_138_2_cm` (card K-NAME-1) | ready, low impact; show the internal names behind disclosure if useful | C8 / C9 |
| player encoding (receipt 23 observation) | R116: declared ranges confirmed on 19 sources; stored 1-based NHL 07–10, 0-based NHL 11+; agrees with the contract | no change; `assets.rs` doc comment "NHL 10 only" lags the code | — |
| 1.4 / 2.4 Xbox packed tail, mipped resize | unchanged (H1 candidate only) | unchanged (refused) | — |

## 2. Order suggested to the developer (research view; the coordinator decides)

1. Archive-backed PS3 09 / 10 / 12 / 13 bank listing and diagnostic preview (C10, with C1 / C2 unchanged) — the review's named next step.
2. Chant wording (C5) and arena-to-bank navigation where the source is established.
3. Ledger wording refresh (the two withdrawn citations; schedule, tournament and save-copy rows above).
4. Only then, if chosen: a partition generator under handoff 08's refusals; tournament browse labels (C14).

## 3. Not reconciled here

Runtime-only rows (unchanged, user cards); Phase 3–5 rows (outside this League / Team pass); developer receipts after 23 (none found at
03:05).
