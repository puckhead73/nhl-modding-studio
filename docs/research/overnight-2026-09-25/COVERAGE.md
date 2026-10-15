# COVERAGE — `.ast` streams by title, platform and record class (R91 framing; R92 decode where a member was decoded)

Class = platform | codec | sample rate | channel_config | loop / one-shot. `framed` = every stream of the class tiles under the R91 grammar with
the sample-word rules of AST_FRAMING.md §1; `decoded (diagnostic)` = at least one member decoded by R92 with the count checks D1 / D3 passing;
nothing is game-verified. Counts are referenced streams (loop = stub records; each has one walked body).

> **Scope note 2026-09-25 (R97 §3, [10-correction-checkpoint-direction-audit.md](10-correction-checkpoint-direction-audit.md)):** rows here are the **11 loose trees** (X360 NHL 10–Legacy, PS3 NHL 11 / 14 / 15 / Legacy);
> `decoded (diagnostic)` means one member of the class was decoded (R92: 14 vectors; R96: 106), never every stream. The archive-backed
> PS3 NHL 09 / 10 / 12 / 13 and X360 NHL 09 rows are in handoff 11 (R98), not here.

| Tree | class | streams | status |
|---|---|---|---|
| x360-nhl10 | x360 · EA-XMA · 22050 Hz · cc0 · oneshot | 51 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl10 | x360 · EA-XMA · 22050 Hz · cc3 · oneshot | 3 | framed |
| x360-nhl10 | x360 · EA-XMA · 24000 Hz · cc0 · oneshot | 574 | framed |
| x360-nhl10 | x360 · EA-XMA · 32000 Hz · cc1 · loop | 6 | framed |
| x360-nhl10 | x360 · EA-XMA · 32000 Hz · cc1 · oneshot | 18 | framed |
| x360-nhl10 | x360 · EA-XMA · 32000 Hz · cc3 · loop | 3 | framed |
| x360-nhl10 | x360 · EA-XMA · 32000 Hz · cc3 · oneshot | 1 | framed |
| x360-nhl10 | x360 · EA-XMA · 44100 Hz · cc0 · oneshot | 11 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl10 | x360 · EA-XMA · 44100 Hz · cc1 · oneshot | 21 | framed |
| x360-nhl10 | x360 · EA-XMA · 48000 Hz · cc0 · oneshot | 3 | framed |
| x360-nhl11 | x360 · EA-XMA · 22050 Hz · cc0 · oneshot | 51 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl11 | x360 · EA-XMA · 22050 Hz · cc3 · oneshot | 3 | framed |
| x360-nhl11 | x360 · EA-XMA · 24000 Hz · cc0 · oneshot | 574 | framed |
| x360-nhl11 | x360 · EA-XMA · 32000 Hz · cc1 · loop | 1 | framed |
| x360-nhl11 | x360 · EA-XMA · 36000 Hz · cc0 · oneshot | 37 | framed |
| x360-nhl11 | x360 · EA-XMA · 36000 Hz · cc1 · loop | 7 | framed |
| x360-nhl11 | x360 · EA-XMA · 36000 Hz · cc1 · oneshot | 18 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl11 | x360 · EA-XMA · 36000 Hz · cc3 · loop | 3 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl11 | x360 · EA-XMA · 36000 Hz · cc3 · oneshot | 1 | framed |
| x360-nhl11 | x360 · EA-XMA · 44100 Hz · cc0 · oneshot | 11 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl11 | x360 · EA-XMA · 44100 Hz · cc1 · oneshot | 21 | framed |
| x360-nhl11 | x360 · EA-XMA · 48000 Hz · cc0 · oneshot | 3 | framed |
| x360-nhl12 | x360 · EA-XMA · 22050 Hz · cc0 · oneshot | 633 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl12 | x360 · EA-XMA · 24000 Hz · cc0 · oneshot | 574 | framed |
| x360-nhl12 | x360 · EA-XMA · 32000 Hz · cc1 · loop | 1 | framed |
| x360-nhl12 | x360 · EA-XMA · 36000 Hz · cc0 · oneshot | 74 | framed |
| x360-nhl12 | x360 · EA-XMA · 36000 Hz · cc1 · loop | 12 | framed |
| x360-nhl12 | x360 · EA-XMA · 36000 Hz · cc1 · oneshot | 36 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl12 | x360 · EA-XMA · 36000 Hz · cc3 · loop | 5 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl12 | x360 · EA-XMA · 36000 Hz · cc3 · oneshot | 3 | framed |
| x360-nhl12 | x360 · EA-XMA · 44100 Hz · cc0 · oneshot | 1722 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl12 | x360 · EA-XMA · 44100 Hz · cc1 · oneshot | 34 | framed |
| x360-nhl12 | x360 · EA-XMA · 44100 Hz · cc3 · oneshot | 3 | framed |
| x360-nhl12 | x360 · EA-XMA · 48000 Hz · cc0 · oneshot | 195 | framed |
| x360-nhl12 | x360 · EA-XMA · 48000 Hz · cc1 · loop | 1 | framed |
| x360-nhl12 | x360 · EA-XMA · 48000 Hz · cc1 · oneshot | 10 | framed |
| x360-nhl13 | x360 · EA-XMA · 22050 Hz · cc0 · oneshot | 626 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl13 | x360 · EA-XMA · 24000 Hz · cc0 · oneshot | 574 | framed |
| x360-nhl13 | x360 · EA-XMA · 31000 Hz · cc1 · loop | 8 | framed |
| x360-nhl13 | x360 · EA-XMA · 32000 Hz · cc1 · loop | 1 | framed |
| x360-nhl13 | x360 · EA-XMA · 36000 Hz · cc0 · oneshot | 74 | framed |
| x360-nhl13 | x360 · EA-XMA · 36000 Hz · cc1 · loop | 4 | framed |
| x360-nhl13 | x360 · EA-XMA · 36000 Hz · cc1 · oneshot | 28 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl13 | x360 · EA-XMA · 36000 Hz · cc3 · loop | 5 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl13 | x360 · EA-XMA · 36000 Hz · cc3 · oneshot | 4 | framed |
| x360-nhl13 | x360 · EA-XMA · 44100 Hz · cc0 · oneshot | 1716 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl13 | x360 · EA-XMA · 44100 Hz · cc1 · oneshot | 35 | framed |
| x360-nhl13 | x360 · EA-XMA · 44100 Hz · cc3 · oneshot | 3 | framed |
| x360-nhl13 | x360 · EA-XMA · 44100 Hz · cc5 · oneshot | 1 | framed |
| x360-nhl13 | x360 · EA-XMA · 48000 Hz · cc0 · oneshot | 195 | framed |
| x360-nhl13 | x360 · EA-XMA · 48000 Hz · cc1 · loop | 1 | framed |
| x360-nhl13 | x360 · EA-XMA · 48000 Hz · cc1 · oneshot | 10 | framed |
| x360-nhl14 | x360 · EA-XMA · 22050 Hz · cc0 · oneshot | 626 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl14 | x360 · EA-XMA · 24000 Hz · cc0 · oneshot | 574 | framed |
| x360-nhl14 | x360 · EA-XMA · 31000 Hz · cc1 · loop | 8 | framed |
| x360-nhl14 | x360 · EA-XMA · 32000 Hz · cc1 · loop | 1 | framed |
| x360-nhl14 | x360 · EA-XMA · 36000 Hz · cc0 · oneshot | 74 | framed |
| x360-nhl14 | x360 · EA-XMA · 36000 Hz · cc1 · loop | 4 | framed |
| x360-nhl14 | x360 · EA-XMA · 36000 Hz · cc1 · oneshot | 28 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl14 | x360 · EA-XMA · 36000 Hz · cc3 · loop | 5 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl14 | x360 · EA-XMA · 36000 Hz · cc3 · oneshot | 4 | framed |
| x360-nhl14 | x360 · EA-XMA · 44100 Hz · cc0 · oneshot | 1708 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl14 | x360 · EA-XMA · 44100 Hz · cc1 · oneshot | 31 | framed |
| x360-nhl14 | x360 · EA-XMA · 44100 Hz · cc5 · oneshot | 1 | framed |
| x360-nhl14 | x360 · EA-XMA · 48000 Hz · cc0 · oneshot | 195 | framed |
| x360-nhl14 | x360 · EA-XMA · 48000 Hz · cc1 · loop | 1 | framed |
| x360-nhl14 | x360 · EA-XMA · 48000 Hz · cc1 · oneshot | 10 | framed |
| x360-nhl15 | x360 · EA-XMA · 22050 Hz · cc0 · oneshot | 626 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl15 | x360 · EA-XMA · 24000 Hz · cc0 · oneshot | 574 | framed |
| x360-nhl15 | x360 · EA-XMA · 31000 Hz · cc1 · loop | 8 | framed |
| x360-nhl15 | x360 · EA-XMA · 32000 Hz · cc1 · loop | 1 | framed |
| x360-nhl15 | x360 · EA-XMA · 36000 Hz · cc0 · oneshot | 74 | framed |
| x360-nhl15 | x360 · EA-XMA · 36000 Hz · cc1 · loop | 4 | framed |
| x360-nhl15 | x360 · EA-XMA · 36000 Hz · cc1 · oneshot | 28 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl15 | x360 · EA-XMA · 36000 Hz · cc3 · loop | 5 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl15 | x360 · EA-XMA · 36000 Hz · cc3 · oneshot | 4 | framed |
| x360-nhl15 | x360 · EA-XMA · 44100 Hz · cc0 · oneshot | 1697 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-nhl15 | x360 · EA-XMA · 44100 Hz · cc1 · oneshot | 31 | framed |
| x360-nhl15 | x360 · EA-XMA · 44100 Hz · cc5 · oneshot | 1 | framed |
| x360-nhl15 | x360 · EA-XMA · 48000 Hz · cc0 · oneshot | 195 | framed |
| x360-nhl15 | x360 · EA-XMA · 48000 Hz · cc1 · loop | 1 | framed |
| x360-nhl15 | x360 · EA-XMA · 48000 Hz · cc1 · oneshot | 10 | framed |
| x360-legacy | x360 · EA-XMA · 22050 Hz · cc0 · oneshot | 626 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-legacy | x360 · EA-XMA · 24000 Hz · cc0 · oneshot | 574 | framed |
| x360-legacy | x360 · EA-XMA · 31000 Hz · cc1 · loop | 8 | framed |
| x360-legacy | x360 · EA-XMA · 32000 Hz · cc1 · loop | 1 | framed |
| x360-legacy | x360 · EA-XMA · 36000 Hz · cc0 · oneshot | 74 | framed |
| x360-legacy | x360 · EA-XMA · 36000 Hz · cc1 · loop | 4 | framed |
| x360-legacy | x360 · EA-XMA · 36000 Hz · cc1 · oneshot | 28 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-legacy | x360 · EA-XMA · 36000 Hz · cc3 · loop | 5 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-legacy | x360 · EA-XMA · 36000 Hz · cc3 · oneshot | 4 | framed |
| x360-legacy | x360 · EA-XMA · 44100 Hz · cc0 · oneshot | 1697 | framed + decoded (diagnostic: a member of this class on some tree) |
| x360-legacy | x360 · EA-XMA · 44100 Hz · cc1 · oneshot | 31 | framed |
| x360-legacy | x360 · EA-XMA · 44100 Hz · cc5 · oneshot | 1 | framed |
| x360-legacy | x360 · EA-XMA · 48000 Hz · cc0 · oneshot | 195 | framed |
| x360-legacy | x360 · EA-XMA · 48000 Hz · cc1 · loop | 1 | framed |
| x360-legacy | x360 · EA-XMA · 48000 Hz · cc1 · oneshot | 10 | framed |
| ps3-nhl11 | ps3 · EALayer3 v1 · 22050 Hz · cc0 · oneshot | 73 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl11 | ps3 · EALayer3 v1 · 22050 Hz · cc3 · oneshot | 3 | framed |
| ps3-nhl11 | ps3 · EALayer3 v1 · 24000 Hz · cc0 · oneshot | 146 | framed |
| ps3-nhl11 | ps3 · EALayer3 v1 · 24000 Hz · cc1 · oneshot | 2 | framed |
| ps3-nhl11 | ps3 · EALayer3 v1 · 44100 Hz · cc0 · oneshot | 435 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl11 | ps3 · EALayer3 v1 · 44100 Hz · cc1 · loop | 1 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl11 | ps3 · EALayer3 v1 · 44100 Hz · cc1 · oneshot | 39 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl11 | ps3 · EALayer3 v1 · 48000 Hz · cc0 · oneshot | 359 | framed |
| ps3-nhl11 | ps3 · EALayer3 v1 · 48000 Hz · cc3 · loop | 3 | framed |
| ps3-nhl11 | ps3 · EALayer3 v1 · 48000 Hz · cc3 · oneshot | 47 | framed |
| ps3-nhl14 | ps3 · EALayer3 v1 · 22050 Hz · cc0 · oneshot | 621 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl14 | ps3 · EALayer3 v1 · 22050 Hz · cc1 · oneshot | 5 | framed |
| ps3-nhl14 | ps3 · EALayer3 v1 · 24000 Hz · cc0 · oneshot | 146 | framed |
| ps3-nhl14 | ps3 · EALayer3 v1 · 24000 Hz · cc1 · oneshot | 2 | framed |
| ps3-nhl14 | ps3 · EALayer3 v1 · 44100 Hz · cc0 · oneshot | 2188 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl14 | ps3 · EALayer3 v1 · 44100 Hz · cc1 · loop | 2 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl14 | ps3 · EALayer3 v1 · 44100 Hz · cc1 · oneshot | 59 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl14 | ps3 · EALayer3 v1 · 44100 Hz · cc3 · loop | 5 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl14 | ps3 · EALayer3 v1 · 44100 Hz · cc5 · oneshot | 1 | framed |
| ps3-nhl14 | ps3 · EALayer3 v1 · 48000 Hz · cc0 · oneshot | 551 | framed |
| ps3-nhl14 | ps3 · EALayer3 v1 · 48000 Hz · cc1 · oneshot | 15 | framed |
| ps3-nhl14 | ps3 · EALayer3 v1 · 48000 Hz · cc3 · loop | 2 | framed |
| ps3-nhl14 | ps3 · EALayer3 v1 · 48000 Hz · cc3 · oneshot | 96 | framed |
| ps3-nhl15 | ps3 · EALayer3 v1 · 22050 Hz · cc0 · oneshot | 621 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl15 | ps3 · EALayer3 v1 · 22050 Hz · cc1 · oneshot | 5 | framed |
| ps3-nhl15 | ps3 · EALayer3 v1 · 24000 Hz · cc0 · oneshot | 146 | framed |
| ps3-nhl15 | ps3 · EALayer3 v1 · 24000 Hz · cc1 · oneshot | 2 | framed |
| ps3-nhl15 | ps3 · EALayer3 v1 · 44100 Hz · cc0 · oneshot | 2177 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl15 | ps3 · EALayer3 v1 · 44100 Hz · cc1 · loop | 2 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl15 | ps3 · EALayer3 v1 · 44100 Hz · cc1 · oneshot | 59 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl15 | ps3 · EALayer3 v1 · 44100 Hz · cc3 · loop | 5 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-nhl15 | ps3 · EALayer3 v1 · 44100 Hz · cc5 · oneshot | 1 | framed |
| ps3-nhl15 | ps3 · EALayer3 v1 · 48000 Hz · cc0 · oneshot | 551 | framed |
| ps3-nhl15 | ps3 · EALayer3 v1 · 48000 Hz · cc1 · oneshot | 15 | framed |
| ps3-nhl15 | ps3 · EALayer3 v1 · 48000 Hz · cc3 · loop | 2 | framed |
| ps3-nhl15 | ps3 · EALayer3 v1 · 48000 Hz · cc3 · oneshot | 96 | framed |
| ps3-legacy | ps3 · EALayer3 v1 · 22050 Hz · cc0 · oneshot | 621 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-legacy | ps3 · EALayer3 v1 · 22050 Hz · cc1 · oneshot | 5 | framed |
| ps3-legacy | ps3 · EALayer3 v1 · 24000 Hz · cc0 · oneshot | 146 | framed |
| ps3-legacy | ps3 · EALayer3 v1 · 24000 Hz · cc1 · oneshot | 2 | framed |
| ps3-legacy | ps3 · EALayer3 v1 · 44100 Hz · cc0 · oneshot | 2177 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-legacy | ps3 · EALayer3 v1 · 44100 Hz · cc1 · loop | 2 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-legacy | ps3 · EALayer3 v1 · 44100 Hz · cc1 · oneshot | 59 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-legacy | ps3 · EALayer3 v1 · 44100 Hz · cc3 · loop | 5 | framed + decoded (diagnostic: a member of this class on some tree) |
| ps3-legacy | ps3 · EALayer3 v1 · 44100 Hz · cc5 · oneshot | 1 | framed |
| ps3-legacy | ps3 · EALayer3 v1 · 48000 Hz · cc0 · oneshot | 551 | framed |
| ps3-legacy | ps3 · EALayer3 v1 · 48000 Hz · cc1 · oneshot | 15 | framed |
| ps3-legacy | ps3 · EALayer3 v1 · 48000 Hz · cc3 · loop | 2 | framed |
| ps3-legacy | ps3 · EALayer3 v1 · 48000 Hz · cc3 · oneshot | 96 | framed |

Totals: 29962 referenced streams framed, 0 refused, on 11 trees / 137 `.ast` files; loop bodies 146 / same verified.

Not covered: archived trees (PS3 09 / 10 / 12 / 13, X360 09 — their `.ast` members were not extracted; grammar expected to hold, unmeasured); PS4 (no `.ast`; SBle `.sbs` streams are a different container, R9 / R83 / R84); cc 5 decode (framed only).
