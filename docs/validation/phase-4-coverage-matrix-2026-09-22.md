# Phase 4 (Equipment) coverage — opened 22 September 2026

Developer-verified only; nothing here is Codex-accepted or game-verified. Same vocabulary as the
Phase 1 / 2 / 3 matrices. This file records the Phase 4 rows touched by the 21 September P5
package (receipt 12 of that campaign) and the R3 continuation; the Hockey Shop editing surfaces
that pre-date them are described in `docs/STATE-OF-THE-REPO.md`.

| Row | X360 07 / 08 / 09 / 10, PS3 08–10 | X360 / PS3 11–12 | X360 13–15 / Legacy, PS3 13–15 / Legacy | PS4 15–21 | Recomp | Receipt |
|---|---|---|---|---|---|---|
| Catalogue / listing: the assigned id's files on the install, catalogue size and range | impl (grammar unmeasured: "no folder" / counts as found) | impl+tested, dev (NHL 11: `texlib_<id>` families, sticks under `stick/`; PS3 12 / 13 archive-only → "no folder") | impl+tested, dev (Legacy: 3 296 / 3 296 assigned ids with files) | separate (vault chain, research 2026-09-20 handoff 09) | by identity | 12 (21 Sep) |
| Existing-id **component set**: "has the family's set, missing: …" | **not measured** (stated on the card) | **single container** per id (`texlib_<id>`) — impl+tested, dev (NHL 11 seam) | **impl+tested, dev** — R52 majority sets on gloves / pads / blocker / trapper / goalie stick / mask paint, per in-game / front-end group; census reproduces R52 on Legacy (both consoles) and the same shape on 13 / 15; sticks / skates / helmets single container | not measured (stated) | by identity | 15 |
| Family census (the reassignment target set = complete ids) | — | — | impl+tested, dev (Legacy 53 / 57 / 50 / 58 / 38 / 280 complete of 53 / 67 / 68 / 66 / 38 / 280) | — | by identity | 15 |
| Existing-id reassignment (validated against the set) | not impl — ordinary cell edit; the card shows the consequence | not impl | not impl (the validation set now exists; no refusal path) | not impl | not impl | 12, 15 |
| New-id registration (place the majority set + `fe*` copy) | not impl | not impl | not impl — the set to place is known (R52 §2); no writer, no runtime card | not impl | not impl | 15 |
| Visor type / mask shell (`helmet/visor n1`, `goaliehelmet n1`) | not a slot | not a slot | not a slot — not checked | — | — | 12 |

## 4.1 hockey sticks — 25 September 2026 (receipt 19, developer-verified)

| Row | X360 07 / PS3 07 (research) | X360 08–10, PS3 08–10 | X360 11–Legacy, PS3 11–Legacy | PS4 15 | PS4 16–21 | Recomp | Receipt |
|---|---|---|---|---|---|---|---|
| 4.1 stick model / curve / flex / tape columns on the equipment companion | n/a — no companion table (tab states it) | present (stick 8 bits range 190 on 08; 5 bits on 09; 7 on 10; curve / flex 0..15 on 08, 0..3 after; 4-bit tape colour) | present (stick 7 bits; curve / flex 0..3; tape colour 4 bits) | present (as 11–Legacy) | present (stick 7 → 10 bits; blade / shaft tape styles 16+, 24-bit RGB tape colours 17+, tape types 19+) | by identity | 19 |
| 4.1 edit → tab → validate → save / reopen → undo; range refusal | n/a | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | by identity | 19 |
| 4.1 stick catalogue labels | — | **provisional** (Legacy-anchored vocabulary, unscoped) | Legacy: anchored (texlib reads); 11–15: provisional | provisional | provisional | by identity | 19 |
| 4.1 on-ice appearance / picker preview agreement | not measured (runtime) | not measured | not measured | not measured | not measured | — | 19 |

## 4.1 correction — 25 September 2026 (receipt 21, developer-verified): title-scoped equipment vocabularies

| Item | X360 07 / PS3 07 | X360 08–15, PS3 08–15 | X360 / PS3 Legacy | PS4 15–21 | Recomp | Receipt |
|---|---|---|---|---|---|---|
| stick / glove / helmet / skate / visor / tape / tuck pickers | n/a (no equipment companion) | stored **number** + "no catalogue for this title" note — impl+tested | Legacy catalogue labels with provenance in the tooltip — impl+tested | stored **number** + note — impl+tested | by identity with Legacy | 21 |
| an id outside every catalogue: written, kept through an unrelated edit, saved, read back | — | impl+tested (range top per title) | impl+tested | impl+tested | by identity | 21 |
| no declared title → no equipment labels | — | impl+tested | impl+tested | impl+tested | — | 21 |
