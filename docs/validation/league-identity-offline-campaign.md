# League identity — offline cross-target campaign (O5 / L1.1-F)

> **Run:** overnight execution 2026-09-17 ([brief](../plans/league-overnight-execution-2026-09-17.md)),
> developer-verified only; no independent review, no game run, no live installation.
> **Baseline:** `feat/phase5-asset-tooling` at `8d60a6d` + the overnight working tree.

## What was run

```sh
cargo test -p studio-api --test league_offline_campaign --target-dir local/runs/league-overnight-2026-09-17/target -- --nocapture
```

The test (`crates/studio-api/tests/league_offline_campaign.rs`) walks the 27 shipping targets plus
the NHL 07 PS3 research copy and, per target:

* **PS4 (a `stockleagues` table):** copies the anchor into a fresh marked copied-target root,
  opens it through the Registry (the seam the app calls), resolves the *target-valid equivalent*
  of the NHL 21 change **semantically** — the first present of the name keys `Allsvenskan`,
  `TXT_LEAGUE_NORWAY`, `TXT_LEGEND_TEAMS` — then addresses that row by its stable `leagueid`,
  re-points its `name` key at the existing key `TXT_ROW_LEAGUE`, checks the pending review shows
  exactly that one cell, runs copied-target review → apply, audits the installed file (only the
  edited record differs at record level; header CRC and size valid; length unchanged), reopens the
  file in a fresh registry (key re-pointed, member-team identities and counts unchanged), and
  restores byte-exactly through the transaction (SHA-256 equals the anchor).
* **Xbox 360 / PS3 (no league table):** opens the anchor read-only, asserts the league catalog
  resolves through the vocabulary measured for that game id, that every league labels its member
  teams, and that the name-edit blocker is reported (no edit is attempted — there is nothing to
  edit).
* **Legacy Recompiled:** no fixture → blocked row.

Every anchor's hash was re-checked after the run (unchanged).

## Per-target outcomes

| Target | Fixture | Source | Operation | Studio result | Structural audit | Reopen | Restore | Note |
|---|---|---|---|---|---|---|---|---|
| nhl07 xbox360 | NHL07NG.db 67a00beaa05690d3… | derived-codes (gen6-nhl07) | browse/classify only — no league name in the main DB | 6 leagues, 5 labelled, raw codes 5; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhl08 xbox360 | nhlng.db 587204c3b18e9957… | derived-codes (gen7-nhl08) | browse/classify only — no league name in the main DB | 6 leagues, 6 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) | bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence |
| nhl08 ps3 | nhlng.db 587204c3b18e9957… | derived-codes (gen7-nhl08) | browse/classify only — no league name in the main DB | 6 leagues, 6 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhl09 xbox360 | nhlng.db 0f7c826b2b905c5c… | derived-codes (gen7-nhl09) | browse/classify only — no league name in the main DB | 10 leagues, 9 labelled, raw codes 8; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) | bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence |
| nhl09 ps3 | nhlng.db 0f7c826b2b905c5c… | derived-codes (gen7-nhl09) | browse/classify only — no league name in the main DB | 10 leagues, 9 labelled, raw codes 8; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhl10 xbox360 | nhlng.db e87b8c806b1d351a… | derived-codes (gen7-nhl10) | browse/classify only — no league name in the main DB | 11 leagues, 9 labelled, raw codes 8/9; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) | bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence |
| nhl10 ps3 | nhlng.db e87b8c806b1d351a… | derived-codes (gen7-nhl10) | browse/classify only — no league name in the main DB | 11 leagues, 9 labelled, raw codes 8/9; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhl11 xbox360 | nhlng.db 461f8d641c50baea… | derived-codes (gen7-nhl11) | browse/classify only — no league name in the main DB | 13 leagues, 13 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) | bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence |
| nhl11 ps3 | nhlng.db 461f8d641c50baea… | derived-codes (gen7-nhl11) | browse/classify only — no league name in the main DB | 13 leagues, 13 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhl12 xbox360 | nhlng.db 94b31ff282f976b1… | derived-codes (gen7-nhl12-15) | browse/classify only — no league name in the main DB | 15 leagues, 15 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) | bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence |
| nhl12 ps3 | nhlng.db 94b31ff282f976b1… | derived-codes (gen7-nhl12-15) | browse/classify only — no league name in the main DB | 15 leagues, 15 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhl13 xbox360 | nhlng.db 64460a156dfd4592… | derived-codes (gen7-nhl12-15) | browse/classify only — no league name in the main DB | 15 leagues, 15 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) | bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence |
| nhl13 ps3 | nhlng.db 64460a156dfd4592… | derived-codes (gen7-nhl12-15) | browse/classify only — no league name in the main DB | 15 leagues, 15 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhl14 xbox360 | nhlng.db 84e1bdff74281102… | derived-codes (gen7-nhl12-15) | browse/classify only — no league name in the main DB | 15 leagues, 15 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) | bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence |
| nhl14 ps3 | nhlng.db 84e1bdff74281102… | derived-codes (gen7-nhl12-15) | browse/classify only — no league name in the main DB | 15 leagues, 15 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhl15 xbox360 | nhlng.db 028ae06ad2b25bf5… | derived-codes (gen7-nhl12-15) | browse/classify only — no league name in the main DB | 15 leagues, 15 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) | bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence |
| nhl15 ps3 | nhlng.db 028ae06ad2b25bf5… | derived-codes (gen7-nhl12-15) | browse/classify only — no league name in the main DB | 15 leagues, 15 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhllegacy xbox360 | nhlng.db 105d2186da3d1f0f… | derived-codes (gen7-nhllegacy) | browse/classify only — no league name in the main DB | 16 leagues, 16 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) | bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence |
| nhllegacy ps3 | nhlng.db 105d2186da3d1f0f… | derived-codes (gen7-nhllegacy) | browse/classify only — no league name in the main DB | 16 leagues, 16 labelled; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |
| nhl15 PS4 | nhl_game.db ae9d1d56bf896b83… | stockleagues | league 12 (TXT_LEGEND_TEAMS) name → TXT_ROW_LEAGUE | pending 1 cell · review handle 0 · apply committed · output 48ef259fd68ac3de… | records changed: fsLK#12 · header crc true · size true · len 16981156→16981156 | league 12 name TXT_LEGEND_TEAMS→TXT_ROW_LEAGUE · 1 members unchanged · 246 teams | restored · sha ae9d1d56bf896b83… == anchor | no Allsvenskan/Norway row: Legend Teams used as the equivalent |
| nhl16 PS4 | nhl_game.db 7d95f1dbe1862fdb… | stockleagues | league 7 (TXT_LEAGUE_NORWAY) name → TXT_ROW_LEAGUE | pending 1 cell · review handle 0 · apply committed · output f36a3b7a3965c68e… | records changed: fsLK#7 · header crc true · size true · len 18456008→18456008 | league 7 name TXT_LEAGUE_NORWAY→TXT_ROW_LEAGUE · 2 members unchanged · 252 teams | restored · sha 7d95f1dbe1862fdb… == anchor |  |
| nhl17 PS4 | nhl_game.db de48b02b054e989a… | stockleagues | league 7 (TXT_LEAGUE_NORWAY) name → TXT_ROW_LEAGUE | pending 1 cell · review handle 0 · apply committed · output 03a01ed35158e5db… | records changed: fsLK#7 · header crc true · size true · len 17798544→17798544 | league 7 name TXT_LEAGUE_NORWAY→TXT_ROW_LEAGUE · 2 members unchanged · 256 teams | restored · sha de48b02b054e989a… == anchor |  |
| nhl18 PS4 | nhl_game.db 65eaa6e82c6dbad7… | stockleagues | league 8 (Allsvenskan) name → TXT_ROW_LEAGUE | pending 1 cell · review handle 0 · apply committed · output 7cd6b886f32418f8… | records changed: fsLK#8 · header crc true · size true · len 18171964→18171964 | league 8 name Allsvenskan→TXT_ROW_LEAGUE · 14 members unchanged · 296 teams | restored · sha 65eaa6e82c6dbad7… == anchor |  |
| nhl19 PS4 | nhl_game.db 265e4a68d23ce44c… | stockleagues | league 8 (Allsvenskan) name → TXT_ROW_LEAGUE | pending 1 cell · review handle 0 · apply committed · output eda86b60409fbd04… | records changed: fsLK#8 · header crc true · size true · len 10347860→10347860 | league 8 name Allsvenskan→TXT_ROW_LEAGUE · 14 members unchanged · 419 teams | restored · sha 265e4a68d23ce44c… == anchor | anchor bytes identical to the other title; provenance unresolved (candidate) |
| nhl20 PS4 | nhl_game.db 265e4a68d23ce44c… | stockleagues | league 8 (Allsvenskan) name → TXT_ROW_LEAGUE | pending 1 cell · review handle 0 · apply committed · output eda86b60409fbd04… | records changed: fsLK#8 · header crc true · size true · len 10347860→10347860 | league 8 name Allsvenskan→TXT_ROW_LEAGUE · 14 members unchanged · 419 teams | restored · sha 265e4a68d23ce44c… == anchor | anchor bytes identical to the other title; provenance unresolved (candidate) |
| nhl21 PS4 | nhl_game.db 90ee1ec49d36dbb0… | stockleagues | league 8 (Allsvenskan) name → TXT_ROW_LEAGUE | pending 1 cell · review handle 0 · apply committed · output 343404153ae51177… | records changed: fsLK#8 · header crc true · size true · len 11209672→11209672 | league 8 name Allsvenskan→TXT_ROW_LEAGUE · 14 members unchanged · 433 teams | restored · sha 90ee1ec49d36dbb0… == anchor |  |
| legacyrecomp recomp | none | blocked | blocked — no fixture (shares Legacy Xbox 360 schema by reference; untested) | - | - | - | - | missing-fixture |
| nhl07 ps3 | NHL07NG.db 67a00beaa05690d3… | derived-codes (gen6-nhl07) | browse/classify only — no league name in the main DB | 6 leagues, 5 labelled, raw codes 5; NHL row 32 members; blocked: This title stores no league name in its database | no edit | n/a | n/a (anchor untouched) |  |

## Reading the table

* "Studio result", "Structural audit", "Reopen" and "Restore" are **developer-verified offline**
  outcomes on copied fixtures. **Packaging** (the PS4 `inxp_00.big` writer) and **runtime**
  (in-game display, save/reload) are **not** covered by this campaign and remain blocked for every
  target (see the [parity matrix](../plans/league-identity-parity-matrix.md) §1.5).
* `TXT_ROW_LEAGUE` is a verified dictionary key on the supplied NHL 15/16/17/18/20 PS4
  dictionaries (`tdb-loc` `tests/dictionaries.rs`, O6). NHL 19 and NHL 21 ship no dictionary fixture, so on
  those two targets the key's existence is **not** target-verified.
* PS3 and Xbox 360 anchors are byte-identical per title, so a pair of rows is one measurement of
  lookup selection, not two runtime qualifications.
* NHL 19 / NHL 20 PS4 anchors are byte-identical with unresolved provenance: their rows are
  candidates.
* A membership edit was never used as evidence for a rename; the gen7 rows are browse/classify
  evidence only.
