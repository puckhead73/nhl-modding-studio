# Handoff R115 (P9) — League 1.2 tournament slots (`championsschedule`, `spenglerschedule`, NHL 17 `wcschedule`): participants, placeholders and phase candidates separated; one explicit consistent join (World Cup: slot = rank by `index`, agreeing with the group ids), a **CHL participant candidate** in the database (the `ischampionsleague` jersey flag marks exactly 32 clubs on every PS4 18–21 source), a coherent Spengler placeholder reading, and competing interpretations kept where the binding is unresolved; no edit authorization

Research lane, 26 September 2026, published 03:01 MDT (system clock). Review `research-20260926-r107-r112-review.md`, queue item 5.
Claim `research-p9-tournament-slots`. Read-only over the PS4 anchors and fixture copies (R113 source list). Builds on R36 (2026-09-21
handoff 08), whose over-strong sentences are corrected in §5. Interpretations are labelled as such; nothing game-verified.

Machine-readable: [`data/r115-tournament-slots.json`](data/r115-tournament-slots.json) (per source and table: rounds, slot ids, pairs,
dates, round-0 groups, league members with group ids, join tests J1 / J2 / JC, Spengler placeholder structure, chance estimate).

## Plain-language summary

The tournament tables are templates: they list games between numbered positions, not between named clubs. This package asked what
each number means. For NHL 17's World Cup, the numbers line up exactly with the eight World Cup teams in database order, and that order
agrees with the two groups the database records — the one case where a position can be tied to a team with two independent pieces of
evidence. For the Champions Hockey League, the database marks exactly 32 clubs with a Champions League jersey on every PS4 title from
NHL 18 on — the 8 guest clubs plus 24 clubs from the modelled leagues — which is the first database candidate for who plays; which club
sits in which position is still open. On NHL 19, reading the positions as those 32 clubs in database order gives eight groups with no two
clubs from the same league, which is very unlikely by chance; on NHL 18, 20 and 21 the same reading would put clubs of one league
together, so it is probably not how those titles fill the groups. For the Spengler Cup, six numbers are the six clubs and four more stand
for "the winner or loser of the opening game" of each group; the knockout numbers are places in the standings. None of this is enough to
build an editor that assigns clubs to positions.

## 1. Coverage

| Table | Sources with rows (identical template per title where noted) | Rows | Rounds (games · slot ids) |
|---|---|---|---|
| `wcschedule` | PS4 17 only | 17 | 0: 12 · 8; 3: 2 · {8}; 4: 3 · {8} |
| `championsschedule` | PS4 18, NHL 19 base = patch 56, real NHL 20, NHL 21 | 125 | 0: 96 · 32; 3: 16 · 16; 4: 8 · 8; 5: 4 · 4; 6: 1 · 2 |
| `spenglerschedule` | PS4 18, NHL 19 base = patch 56, real NHL 20, NHL 21 | 11 | 0: 6 · 10; 4: 2 · 4; 5: 2 · 4; 6: 1 · 2 |
| `worldchampschedule` / `worldjuniorschedule` | real NHL 20, NHL 21 | **0** (capacity 100 / 60) | — |

Not present: PS4 15 / 16, gen7 (NHL 11 / 12 `fcc_tournaments`, 75 rows, remain unanalysed — boundary). Round-0 CHL groups: contiguous slot
blocks {0–3} … {28–31} on PS4 18, real NHL 20, NHL 21; **interleaved** on NHL 19 base / patch 56 ({0, 8, 16, 27}, {1, 20, 21, 31}, …).

## 2. Slot classes

| Class | Where | Evidence | Status |
|---|---|---|---|
| **Participant slots** | `wcschedule` round 0 (0–7); `championsschedule` round 0 (0–31); `spenglerschedule` round 0 ids 0–5 | each id plays a full group programme (WC 3, CHL 6 games; Spengler: see placeholders) | participant identity: WC — explicit consistent join (§3); CHL — candidate set, mapping open (§4); Spengler — two readings (§3) |
| **Placeholders** | `wcschedule` rounds 3–4: every game `8 v 8`; `spenglerschedule` round 0 ids 6–9 | WC: a single out-of-range id on both sides; Spengler: ids 6 / 7 only ever oppose club 2 and ids 8 / 9 only club 5, the club absent from its group's day-1 game (0 v 1, 3 v 4) | reading **R-WL**: ids n + 2k, n + 2k + 1 = the two clubs of day-1 game k (winner / loser; which is which is unresolved); consistent on all five sources |
| **Phase candidates (seeds)** | `championsschedule` rounds 3–6; `spenglerschedule` rounds 4–6 | ids re-numbered per round: CHL round 3 pairs k v 15−k (16 seeds, two legs), round 4 k v 7−k, round 5 k v 3−k, final 0 v 1; Spengler round 4 `1 v 5`, `4 v 2`, round 5 `3 v 10`, `0 v 11`, final `1 v 0` | interpretation: standings / bracket seeds filled at run time (e.g. Spengler 0 / 3 group winners, 1 / 4 seconds, 2 / 5 thirds, 10 / 11 quarter-final winners); the DB holds no binding for them — not a verified runtime rule |

## 3. Explicit joins tested

| Join | Test | Result |
|---|---|---|
| **J1** slot = rank by `index` among the tournament league's members | World Cup (league `WorldCup`, 8 clubs): round-0 groups {0, 1, 2, 7} / {3, 4, 5, 6} vs `divisiongroup` | **agree**: {CAN, CZE, EUR, USA} all 44, {FIN, NA, RUS, SWE} all 45 |
| J1 | Spengler (`TXT_SPENGLER_CUP`, 6 clubs), groups under R-WL {0, 1, 2} / {3, 4, 5} vs `divisiongroup` 75 / 76 | **agree** on NHL 19 base, patch 56, real NHL 20 (ranks 0–2 all 76, 3–5 all 75); **disagree** on NHL 21 (ranks 0, 3, 4 = 75; 1, 2, 5 = 76 — TRI / HCD / CANS v UFA / TPS / HCAP); **untestable** on PS4 18 (all six `SC1`–`SC6` in group 75) |
| **J2** slot = position inside the `divisiongroup` (groups by id, members by index) | Spengler | consistent with the groups by construction; on NHL 19 / 20 it orders group 75 first, so it **conflicts with J1** about which trio is slots 0–2 even where both keep the groups intact |
| J1 | CHL (`TXT_CHAMPIONS_LEAGUE`, 8 guest clubs) | not testable — 8 members for 32 slots, one group id (64) |

Competing Spengler interpretations therefore remain: (a) J1 with NHL 21's group ids not being the tournament groups; (b) J2 (group ids
decide, rank order ignored); (c) the runtime assigns clubs to ids by another rule. A table edit cannot choose among them statically.

## 4. CHL participant candidate (JC)

`stockteamjerseys.ischampionsleague = 1` on **64 rows = 32 clubs × 2 jerseys** on every PS4 18–21 source (field absent on PS4 15–17):
the 8 league-9 guest clubs plus 24 domestic clubs, mix per title — PS4 18 / NHL 19: SHL 5, Liiga 5, Czech 4, Swiss 4, DEL 3, EBEL 3;
real NHL 20: SHL 5, Swiss 5, Liiga 4, Czech 4, DEL 3, EBEL 3; NHL 21: SHL 5, Swiss 5, Liiga 4, DEL 4, Czech 3, EBEL 3. The club lists
are in the JSON. **Measured:** the flag's population equals the tournament's size and league-9 membership on every source. **Not
established:** that the game selects participants by this flag (it may only choose the jersey).

Slot mapping reading **I1** "slot = rank by `index` within the flagged set": on NHL 19 base / patch 56 the interleaved groups then hold
four different leagues each (0 of 8 groups with two clubs of one league; chance under uniform random assignment ≈ 27 in 200 000, seeded
Monte Carlo). On PS4 18, real NHL 20 and NHL 21 the contiguous groups would hold same-league clubs in all 8 groups. Reading **I2** "round-0
slots are draw positions filled at run time" fits the contiguous templates. I1 is supported on NHL 19 only; neither is established.

## 5. Corrections to R36 (dated note placed on 2026-09-21 handoff 08)

- "the participants of these slots are resolved at runtime (group standings)" — holds as an interpretation for knockout seeds only; round-0
  slots are participant slots whose binding is unresolved (World Cup: consistent join; CHL / Spengler: §3–4).
- "there is no per-slot team binding to edit" — overstated: not established in either direction.
- "the other 24 CHL participants … are chosen by a mechanism outside the database tables (not found here)" — superseded: the
  `ischampionsleague` jersey flag is a database candidate (§4); selection by it is not established.

## 6. Developer-facing boundary

Browse-only stays right for the tournament tables. Safe to show: round labels (group stage / knockout rounds by `round`), placeholder
ids as "to be decided" (WC `8`, Spengler 6–9 as "winner / loser of game k" **labelled as an interpretation**), knockout ids as seeds, and
the World Cup group stage with club names through J1 (evidence: agrees with the group ids on the one source). Not authorised: assigning
clubs to CHL or Spengler slots, editing the jersey flag as a "participant" switch, or generating tournament rows. Discriminating runtime
tests (user, optional, on a disposable copy): change one CHL domestic club's `ischampionsleague` jerseys to 0 and add a flag to a non-participant, start a CHL
season and see who appears; swap two Spengler clubs' `divisiongroup` on NHL 21 and see whether the group-stage pairings follow.

```
python local/runs/overnight-research-2026-09-26/probes/r115_tournament_slots.py <out.json>
```
Executed once; output published through `safe_patch.py put`.

> **Addendum 2026-09-26 03:05:** NHL 11 / 12 `fcc_tournaments` (75 / 75 rows, fields `assetid`, `name`, `tournamentdbid`; absent on NHL 13) is the Ultimate Team tournament **name catalogue** — `TOURNAMENT_OFFLINE_NAME_nnn` / `TOURNAMENT_LIVE_NAME_nnn` keys with an asset id (19 distinct) and a 1-based id — beside the other `fcc_*` card tables. It holds no games, slots or dates and is outside League 1.2 schedules / tournaments.
