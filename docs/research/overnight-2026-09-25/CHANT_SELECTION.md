# CHANT_SELECTION — gen7 club-chant selection evidence (R95; follow-up to R90 / card T-CHANT-1)

Research lane, 25 September 2026. Read-only over X360 NHL 15, PS3 NHL 15 and X360 NHL 10 (`audio/na_en_loadfile.xml`, `audio/sfx/*.abk`,
`audio/sfx/aemscrwd.csi`) and the NHL 15 anchors (R90's join, not re-run). Machine-readable: `data/r95-crowd-script.json`,
`data/r95b-controlcenter.json`. Nothing here demonstrates runtime selection; each level below is stated with what it is.

> **Note 2026-09-25 14:20 (R97, [10-correction-checkpoint-direction-audit.md](10-correction-checkpoint-direction-audit.md) §1):** "the only database value in 55–66 anywhere is the Finnish arena's `chantaudioid` 61",
> "the only database path", "no database field and no bank data selects them" and "the key, if any, is code-side" are **withdrawn as
> stated**. R90 inspected only `exhibitionarena.chantaudioid` and the integer fields of the twelve matched clubs' team / arena rows. The
> R97a census of every integer field of every table on the 18 gen7 anchors shows: the Finnish arena's **goal-horn, period-horn and chant
> ids are all 61** (and its arena / city audio ids on 09 / 10); SHL clubs without a club bank carry team `audioid` 64–66 (AIK, Leksand,
> Örebro); 3–10 players carry name-call `audioid` 55–66; 178–403 non-audio fields hold 55–66 values. Established: club association by
> name (bank / module / control-bank tables); loading by bank number; no club's own arena or team row carries its bank number; **no
> selecting consumer traced**. R90's handoff table row "arenas referencing a club bank 55–66 = 0" was a transcription error — the R90
> JSON records `[61]` on all 11 trees. Page wording: handoff 10 §1.4. Card: T-CHANT-1 rev. 3.

## 1. The five levels, separated

| Level | Finding | Where |
|---|---|---|
| **String / name presence** | The 12 club picker names (`brynasSamplePicker_msg/_snd` … `timraSamplePicker_msg/_snd`) occur in exactly three places per title: the club's own bank `0NN_chant.abk`, the crowd event-definition module `aemscrwd.csi`, and the crowd **control** bank `controlCenter.abk` (`AemsCrowdControlBank`, loader `loadtype="temp"`). Same three on X360 15, PS3 15 and X360 10. | r95 L3 |
| **Reference to a bank / chant** | `controlCenter.abk` (0 sample records, 189 names) defines each club picker as a **`_msg` node of class 2 with 28 bytes (X360) / 56 bytes (PS3) that are zero in their leading 20+ bytes**, with no pointer from any other named node into it (0 parents, 0 adjacent constants) — a message *endpoint* by name, carrying no data that could select. The generic pickers of the same bank (`playOffChantSamplePicker`, `homeScoredOnGen…`, `ownGoal…`) are defined the same way. | r95b |
| **Reachable selection logic** | `aemscrwd.csi` is a MOIR module: a 196-entry name table (`u32 string ptr | u16 id | 6 × 0` at 0x2c, strings from 0x389c to EOF) **plus a 12 096-byte body between table and strings (3 477 nonzero bytes) that R20's "name table only" reading does not cover** — a compiled body exists. As u16 big-endian words, that body contains 8 of the 196 name ids, **none of them a club picker id** (club ids 3 286, 4 073, 17 499, 18 908, …). Whether the body selects anything, and by what encoding, is not decoded (opcode format unknown; the u16 reading is one probe, not a decoding). | r95b (X360; the PS3 module has another table layout — not parsed) |
| **The input that chooses** | **Loading, not playing, is the first gate:** every team-chant bank is a loader asset `AemsCrowdTeamChantBank` with `descr="<bank number>"` (73 / 73 on each title), exactly like `AemsSfxGoalHornBank` (`descr` = horn id, the id space of `exhibitionarena.goalhorn`). A club bank can only be resident if something requests its number. In the database the only value in 55–66 anywhere is the Finnish arena's `chantaudioid` 61 (R90's counterexample) → under arena-keyed loading the Linköping bank is the only club bank ever loaded, and at a Finnish arena. Whether the engine passes the arena's `chantaudioid`, a team field or a code table to the loader is **not observable offline**. | r95 L1 + R90 |
| **Proven binding** | **None.** No data structure found offline maps a team to a club bank. Two candidate interpretations remain: (a) the club banks are unreachable content on the shipped titles except through arena id 61 (an authoring inconsistency); (b) code-side team-keyed selection whose key is in the executable. The R90 sentence "selected by the compiled crowd module by name" is **narrowed**: the module *names* the pickers; nothing shows it selects them, and the club endpoints in the control bank are empty. | — |

## 2. Counterexample kept and tested

The Finnish arena (`exhibitionarena.chantaudioid` = 61, tenants Finnish clubs on NHL 13–Legacy; R90) is the one database path to a club
bank. Any proposed selector must explain it: under (a) it is the *only* place a club chant can play, and it plays the Linköping
chant; under (b) it is irrelevant to the club selection and the arena would receive the generic treatment of id 61 whatever that is.
Across titles the value is stable (R90: 61 on every anchor 13–Legacy), so it is authored, not noise.

## 3. Smallest additional observation (replaces T-CHANT-1's first observation; card revised as T-CHANT-1 rev. 2)

1. **X360 NHL 15, exhibition game at the Finnish arena (chantaudioid 61)** as any two teams: a Linköping club chant (the `061_chant`
   bank's pickers — 3 records) audible → chant banks load by the arena's `chantaudioid` and the club banks are reachable that way;
   nothing club-specific → either the bank does not load by that field or the module never triggers club pickers.
2. **Control: Frölunda at home (arena → bank 45)**: a Frölunda chant here would show a team-keyed path that is not in the database
   (interpretation b); none is consistent with (a).
3. Second control as before: Växjö (no club bank).

Neither observation identifies the key; together they separate "loader keyed by the arena field" from "code-side team selection".

## 4. Consequence for the pages (developer; not Codex-accepted)

Unchanged from R90 for the listing; the wording "selected by the compiled crowd module" should read **"club banks are named by the
crowd module and the crowd control bank; no database field and no bank data selects them; loading is keyed by bank number (loader
`descr`) and only arena chant id 61 reaches a club bank"**. No edit route follows.
