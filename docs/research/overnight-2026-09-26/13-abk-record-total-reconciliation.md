# Handoff R119 (P14) — developer observation reconciled: R98's per-member `.abk` "records" are table **slot counts** (present records + absent slots), R82's archived totals are **present records** only; the difference is exactly R82's absent slots on every tree (25 / 24 / 37 / 43), almost all in `fe.abk`; no record is missing and no count is wrong

Research lane, 26 September 2026, published 03:45 MDT (system clock). Consumes observation 2 of claim `dev-p48-archive-sound-banks`
(receipt 24): "R98 per-member 'records' totals exceed R82 section 2 present-record totals (2619/2772/4734/5160 vs 2594/2748/4697/5117)".
Claim `research-p14-abk-record-totals`. Derived from the two published JSONs and the two probe sources; no member re-read.

## Plain-language summary

Two research reports counted the entries in the same sound-bank files and got slightly different totals. Both are right: one counted
every numbered slot in each bank's table, including empty ones, and the other counted only the slots that hold a sound. The gap is
exactly the number of empty slots, and nearly all of them are in one front-end bank per title.

## 1. Reconciliation

| Tree | R82 present records (`entries`) | R82 absent slots (`absent`) | sum | R98 `records` (= table `count`) | members differing | where the absent slots are |
|---|---|---|---|---|---|---|
| PS3 NHL 09 | 2 594 | 25 | 2 619 | 2 619 | 2 | `audioboot.big:fe.abk` 31 present / 55 slots; `crowds.abk` 242 / 243 |
| PS3 NHL 10 | 2 748 | 24 | 2 772 | 2 772 | 1 | `fe.abk` 31 / 55 |
| PS3 NHL 12 | 4 697 | 37 | 4 734 | 4 734 | 1 | `fe.abk` 32 / 69 |
| PS3 NHL 13 | 5 117 | 43 | 5 160 | 5 160 | 2 | `fe.abk` 23 / 65; `audioboot2.big:skates_v2.abk` 484 / 485 |
| X360 NHL 09 | 2 426 | 25 | 2 451 | 4 902 = 2 × 2 451 | 2 (per copy) | as PS3 NHL 09; R98 counts the archive member **and** the byte-identical loose copy (372 members) |

Sources: `r98_ast_framing_archived.py` line 117 / 139 stores `tab['count']` (the header slot count; absent slot = offset `0xFFFFFFFF`,
R82 §1); `r82_abk_entry_records.py` reports `entries` (present) and `absent` per table; R82's archived rows in handoff 01 §2 show "—" in
the absent-slot column although the JSON carries the values.

## 2. Developer-facing wording

A listing can show "N sounds (M empty slots)" with N = present records and N + M = the table count; the stream-identity and framing
rules are unaffected (absent slots have no record). The developer's reading of `stored` on all 817 PS3 09 / 10 / 12 / 13 bank members
agrees with R110b (checkpoint 00c corrected the R98 / R100 labels). A dated note is placed on R82 handoff 01 §2 (absent-slot column).
