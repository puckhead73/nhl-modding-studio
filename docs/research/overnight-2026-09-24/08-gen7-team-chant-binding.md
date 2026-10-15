# Handoff R90 — gen7 crowd team-chant banks: the bank names decode the binding — twelve of the 73 `AemsCrowdTeamChantBank` banks (055–066) are **club banks whose internal picker is named after a Swedish club** (brynas, djurgarden, farjestad, frolunda, hv71, mora, link[öping], lulea, malmo, modo, sodertalje, timra; content byte-identical per platform from NHL 10 to Legacy), the crowd module script `aemscrwd.csi` carries the same twelve pickers on every title, **no arena's `chantaudioid` ever equals 55–66 and no team / arena audio field carries those ids** — every SHL club's arena points at the generic bank 45 — so the club chant is selected by the compiled crowd script (by name), not by a database field; the arena-keyed join is re-measured per title (24–25 arena chant ids without a bank, tenants = DEL / national / Czech / create-a-team rows), the goalie banks carry no names, and one cross-reference (arena chant id 61, a Finnish arena, addresses the Linköping bank) is kept as a counterexample

Research lane, 24 September 2026, 13:47 → 13:54 MDT (review: "choose the highest-impact ready League / Team blocker" — Leagues 1.5 / Teams 2.7
"league-keyed cues inside generic banks selected by compiled conditions (no static join)"). Claim `research-r90-team-chant-binding`.
Read-only over the 11 loose trees and the committed anchors. Nothing game-verified; nothing listened to; no writer proposed.

Machine-readable: [`data/r90-team-chant-binding.json`](data/r90-team-chant-binding.json). Probe `r90_team_chant_binding.py` (≈ 2 min).

## Plain-language summary

Every gen7 title ships 73 numbered crowd-chant banks. Reading their newly decoded name tables shows that twelve of them are club
chants for Swedish teams, named as such inside the bank, and that the game's crowd script knows exactly those twelve names. But
nothing in the database points at those twelve banks: the Swedish clubs' arena entry names a generic chant bank, and no team or arena
field holds 55–66. So the club chant is chosen by the compiled crowd script, keyed by name, not by any value Studio could edit. That
settles the "how are league-specific chants selected" question statically: the join exists (bank → club by name), the selection key
does not live in the database. The arena-keyed chant join is restated per title with the ids that have no bank (German league,
national and create-a-team rows), the goalie chant banks carry no names, and one odd cross-reference is recorded rather than explained.

## 1. Measurements (11 trees; every chant bank; the title's own anchor)

| | X360 10 | X360 11 | X360 12 | X360 13 | X360 14 | X360 15 | X360 Legacy | PS3 11 | PS3 14 | PS3 15 | PS3 Legacy |
|---|---|---|---|---|---|---|---|---|---|---|---|
| team chant banks present = loader-listed | 73 = 73 | 73 | 73 | 73 | 73 | 73 | 73 | 73 | 73 | 73 | 73 |
| goalie chant banks (4-digit) | 45 | 45 | 45 | 45 | 45 | 45 | 45 | 45 | 45 | 45 | 45 |
| club-named banks (055–066) | 12 | 12 | 12 | 12 | 12 | 12 | 12 | 12 | 12 | 12 | 12 |
| script `aemscrwd.csi` sample pickers / club pickers | 63 / 12 | 63 / 12 | 63 / 12 | 63 / 12 | 63 / 12 | 63 / 12 | 63 / 12 | 63 / 12 | 63 / 12 | 63 / 12 | 63 / 12 |
| arena `chantaudioid` distinct values | 64 | 64 | 62 | 62 | 61 | 61 | 62 | 64 | 61 | 61 | 62 |
| … values with **no bank** (= unlisted) | 25 | 25 | 24 | 24 | 24 | 24 | 25 | 25 | 24 | 24 | 25 |
| … arenas referencing a club bank 55–66 | **0** | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| **corrected 2026-09-25 (R97):** arena rows whose `chantaudioid` ∈ 55–66 per the R90 JSON (`club_banks_referenced_by_any_arena_chantaudioid` = [61]) and the R97a census — the Finnish arena, value 61, whose `goalhorn` / `periodhorn` are 61 too | 15 rows | 15 | 15 | 1 | 1 | 1 | 1 | 15 | 1 | 1 | 1 |
| SHL (league 2) teams → arena chant id | (no `arenaid` field 10–12) | — | — | 12 → 45 | 12 → 45 | 12 → 45 | 14 → 45 | — | 12 → 45 | 12 → 45 | 14 → 45 |
| club banks with a same-named team on the title | 10 | 10 | 9 | 8 | 7 | 8 | 9 | 10 | 7 | 8 | 9 |
| audio / chant / arena fields equal to the club bank id on that club's rows | 0 | 0 | 0 | 0 | 0 | 0 † | 0 † | 0 | 0 | 0 † | 0 † |

† `exhibitionteams.arenaid` == 61 for Linköping (NHL 15) and == 59 for HV71 (Legacy) — the arena *index* happening to equal the bank
number; an arena index is not an audio key (listed in the JSON as coincidental, with the goalie-mask / colour coincidences).

- **Club banks:** `055` brynas (5 records), `056` djurgarden (5), `057` farjestad (4), `058` frolunda (5), `059` hv71 (5), `060` mora (3),
  `061` link (3), `062` lulea (4), `063` malmo (5), `064` modo (4), `065` sodertalje (3), `066` timra (4) — each holds exactly the pair
  `<club>SamplePicker_msg` / `<club>SamplePicker_snd` plus the three generic nodes every chant bank has (`sm_crowdAmbience_v`,
  `crowdReactionsMasterVol_snd`, `streamInterruptRecieve_msg`). Bytes: **one SHA-256 per platform across all titles** (2 distinct per
  file over 11 trees = the X360 and PS3 encodes) — the set is frozen at its NHL 10 state (Mora, Södertälje, Timrå and Malmö are not
  SHL teams on 13–15; Malmö returns on Legacy; AIK / Leksand / Örebro / Växjö / Karlskrona / Rögle / Skellefteå never get a bank).
- **Script:** `aemscrwd.csi` is a `MOIR` module (30 820–46 728 B; one build per platform-era: X360 14 / 15 / Legacy identical
  `615ebc46…`, PS3 14 / 15 / Legacy `7089a8c1…`); its symbol table (12-byte entries `u16 string offset | u16 id | 8 × 0` from 0x2c) lists
  the same 63 `*SamplePicker_msg` names on every title, the 12 club pickers among them; the compiled body is not decoded.
- **Arena-keyed join (per title):** `exhibitionarena.chantaudioid` values with no bank = 30, 31, 32, 33, (35 on 10 / 11), 89, 90,
  100–103, 105–117, 136 (+ 511 on Legacy); their tenants on 13–Legacy: DEL clubs (league 4: 4 rows), national teams (20), Czech (11–14),
  create-a-team (30), Legends / Top Prospects, Hartford. Every SHL club's arena carries **45** (the "EA SPORTS Arena" they share);
  Finnish clubs' Hartwall Areena carries **61** — the number of the Linköping club bank (counterexample: an arena chant id that
  addresses a club bank of another league; not interpreted — either a stale value or the id spaces are unrelated in practice).
- **Goalie chant banks** (45 four-digit ids = goalie announcer ids, R85): no bank-specific names; nothing new.

## 2. Consequence (developer; not Codex-accepted)

- **Arena page (gen7):** `chantaudioid` → "chant bank `<id:03>_chant.abk` present / no bank (unlisted id)" stays the arena-keyed listing;
  add: "bank 055–066 are club chants (Brynäs … Timrå) selected by the crowd script by name; no arena or team field selects them".
- **Team page (gen7, SHL clubs):** "club chant bank present (`0xx_chant.abk`, named `<club>SamplePicker`)" for the 7–10 clubs that have one
  on the title — listing only; **no editable key**: re-pointing a team to another club chant is not a database edit on gen7 (the binding
  is inside the compiled `MOIR` crowd module).
- **Boundary:** the selection *condition* (which team identity the script compares — index, audio id or name hash) is not decoded; the
  static result is "the key is not in the database", not "the key is X". Card T-CHANT-1 tests the observable half.

## 3. Card (deferred; no listening)

- **T-CHANT-1** — on X360 NHL 15, play an exhibition game as Frölunda (bank 058, team present) at home; control: Växjö (no club bank);
  second control: Djurgården (bank 056, team present on 15 but absent on 13 / 14). Outcome: a club chant for Frölunda and none for Växjö
  → the script binds by team identity to the named banks; a chant for both → the club banks are not what plays (or generic chants
  mask it); none → the club banks are unused on this title. Does not identify the key.

```
python local/runs/overnight-research-2026-09-24/probes/r90_team_chant_binding.py docs/research/overnight-2026-09-24/data/r90-team-chant-binding.json
```
Executed twice (the first run failed at the JSON write on a non-cp1252 character; the DB-key test was then split into audio / arena
fields versus coincidental fields). Inline: the crowd-script symbol-table look (`aemscrwd.csi`, X360 NHL 15).

> **Note 2026-09-25 14:20 (R97, [overnight-2026-09-25/10-correction-checkpoint-direction-audit.md](../overnight-2026-09-25/10-correction-checkpoint-direction-audit.md)):** the title's "no arena's `chantaudioid` ever equals 55–66" and the table row above
> ("0" on every tree) were **transcription errors**: the probe's own output records the Finnish arena (chant id 61) on all 11 trees, as this
> handoff's own counterexample paragraph says. The census of every integer field (18 anchors) adds that the same arena rows carry 61 in
> `goalhorn` and `periodhorn` (and `arenaaudioid` / `cityaudioid` on 09 / 10), that SHL clubs without a club bank carry team `audioid` 64–66,
> and that player name-call ids fall in 55–66 — so "selected by the compiled crowd script, not by a database field" is withdrawn as a
> conclusion: club association by name is measured, loading is by number, and **no selecting consumer has been traced**. The R95 note
> below stands only for its measurements (loader, module table, control bank); its "only database path" / "key is code-side" wording is
> superseded by handoff 10 §1.

> **Note 2026-09-25 (R95, [overnight-2026-09-25/CHANT_SELECTION.md](../overnight-2026-09-25/CHANT_SELECTION.md)):** "the club chant is
> selected by the compiled crowd script (by name)" is **narrowed**: the module names the 12 pickers (a 196-name MOIR table plus a 12 KB
> undecoded body whose u16 words contain no club picker id), the crowd control bank `controlCenter.abk` defines them as empty class-2
> message endpoints, and chant banks are loaded by bank number (loader `descr`, like goal horns) — so the only database path to a club
> bank is this handoff's counterexample (arena chant id 61 → the Linköping bank). No selecting data was found offline; the key, if any,
> is code-side. Card T-CHANT-1 is revised (rev. 2: observe the Finnish arena first).
