# Handoff A-PS4 — goal-horn ids, the PS4 `hornbank.sbr`, and where static evidence ends

Research lane, 18 September 2026 (follow-up dispatch item 3). Consumer: developer 2.7 (team/arena cues on
PS4), 1.5 (league audio). Machine-readable: [`data/ps4-goalhorn-survey.json`](data/ps4-goalhorn-survey.json)
(NHL 15 / 18 / 21 install DBs: goal-horn field distributions and the per-league id sets),
[`data/ps4-nhl21-audio-banks.csv`](data/ps4-nhl21-audio-banks.csv). Probes: `ps4_horn_survey.py`.
Evidence: **observed** (DB fields, bank headers, UI contract) / **hypothesis** where marked. No audio was
decoded or played; "filename absence" is not used as evidence of anything below.

## Plain-language summary

On PS4 the goal horn a team plays is chosen by a small number on its arena row (`goalhorn`), exactly as on
Xbox 360 — NHL 15 PS4 even uses the same numbers as NHL 15 Xbox for every stock arena. From NHL 18 the
number space was renumbered (NHL clubs 1–31, a shared default 41), and NHL 21 adds the "custom goal horn"
fields (up to three elements, a flair and blast/pause lengths) that the Creation Zone uses; in the stock DB
every arena still has the default single-element horn. All PS4 horn audio sits in one bank
(`nhl/data/kettle/audio/sfx/hornbank.sbr`, 44.6 MB, samples embedded), so the number is a **sound id inside
that bank**, not a file name. How the number maps to a sample inside the bank needs the EA SBR/DSET index
format, which was only partly read here; that, and what the UI's `Filter` field means, are the open items,
each with a minimal in-game test listed at the end.

## 1. Static relationships (observed)

| Title (witness) | `exhibitionarena` horn fields | Id space actually used | Notes |
|---|---|---|---|
| NHL 15 PS4 (`ae9d1d56…`) | `goalhorn` 9-bit, `periodhorn` 9-bit, `arenaaudioid`, `cityaudioid` | 42 distinct ids 0–136: NHL arenas 0–29 (one per club), DEL 30–41, SHL 45, Liiga 61, AHL/international 136 | **identical to NHL 15 Xbox 360 per arena** (39/39 stock arenas same `goalhorn`/`periodhorn` pair) → the Xbox `{id:03}_goal.abk` bank names *are* these ids; on PS4 the same ids address the bank |
| NHL 18 PS4 (`65eaa6e8…`) | same four fields | 33 distinct: NHL 1–31 (renumbered, one per club incl. VGK 30/31), `41` on 253 of 303 arenas (shared default), AHL also 2/26, WHL 5/12, EASHL 0/7/25/29 | `periodhorn` still uses the NHL 15 numbering (0–136) — the two fields no longer share a space |
| NHL 21 PS4 (`90ee1ec4…`) | + `goalhorniscustom` (1 bit), `goalhornelement1/2/3` (10-bit, −1..1022), `goalhornflair`, `goalhornblastsnum` (2), `goalhornblast1–3length`, `goalhornpause1–2length` | 33 distinct: NHL 1–31, default 41 (286/450), Alumni league reuses 1–30, EASHL 690, Pro Am subsets; **stock rows all `iscustom 0`, `element1 501`, others 0, `blastsnum 1`** | custom-horn builder record (Creation Zone); `periodhorn` collapsed to 0–2 |

UI contract (handoff 17): `nhl-data:audio:ids:get` returns `GoalHornList[{Id, Filter, Name}]` — the id is
the `goalhorn` value; `Name` a display string; `Filter` an integer. **Hypothesis:** `Filter` partitions
the list by league/category (the DB shows league-clustered id sets: NHL 1–31, European/AHL 41, EASHL 690);
it could equally be a "custom vs stock" flag — not decidable statically.

Xbox 360 recap (handoff 15): per-id members `nocache.big:audio/sfx/{id:03}_goal.abk` / `_period.abk`
for ids 000–029, 100, 194 (Legacy) — one bank per id — plus the generic `horns.abk`. So the **id space is
shared across platforms up to NHL 15** (same DB numbers), while the **packaging differs**: one ABK file per
id on Xbox, one SBR bank on PS4.

## 2. The PS4 bank (`hornbank.sbr`, observed header only)

```
0x00 "SBle"  0x04 u32 file size (0x02A916E0)  0x08 u16 version 4  0x0A u16 0x0002  0x0C u32 bank hash
0x10 "MRAH"  0x18 u32 0x50 (pointer block)  0x20 u32 0x60 (data start)
0x50 u32 0x02A90540 (DSET offset), 0x58 u32 0x02A90FC0, 0x5C u32 0x02A90550
0x60 "data" … 44.6 MB of sample data (codec not identified; EALayer3/XAS expected from the O6 music work)
0x02A90540 "TESD" (DSET, 0xC8 header): count 0x29D (669), field descriptors "MAR." (.RAM) and "DIS." (.SID)
           .SID block 0x02A90608–0x02A91080 = 670 × 4 bytes; followed by a sequential u16 0..N index
```
The 670-entry .SID table is where a sound id → sample offset binding must live, but its record layout was
**not decoded** (reading the block as u16 shows plausible ids — 1, 2, 690, 501 … — mixed with hashes and
sentinels, so a naive "list of ids" reading is wrong). Two empty banks (`applausebank.sbr`,
`applausebankgoal.sbr`, 80 B each, count 0) show the minimal header. This is the boundary of the static
evidence: **the bank's index format (EA SND10 SBR/DSET) is the next research item, not a Studio task.**

## 3. Where static evidence ends — minimal user tests (deferred; no launches performed)

1. **Id → sound binding (NHL 21 PS4):** in an otherwise stock DB set one NHL arena's `goalhorn` from its club
   id to `2` (Winnipeg) and another to `41`; play a game at each — Winnipeg's horn / the generic horn confirm
   that the DB number selects the bank sound and that 41 is the shared default. Static evidence for the
   *number* is the DB; the *sound* only a run can show.
2. **`Filter` semantics:** open the arena/Creation Zone horn picker for an NHL club and for a European club and
   note whether the list differs (league filter) or shows stock vs custom sections (kind filter).
3. **Custom horn fields:** set `goalhorniscustom 1`, `goalhornelement1..3` to three different 500-range ids,
   `goalhornblastsnum 3` on one arena; if the game plays a three-blast composite, the element ids are
   hornbank sound ids too (hypothesis).
4. **Xbox fallback (still open from handoff 15):** an arena id with no `{id}_goal.abk` — which horn plays.

## 4. League audio consumer leads (static; not tracing code)

- PS4: no league-keyed horn/period entries exist in the DB (arena rows only); league presentation audio, if
  any, would be event-driven inside `sfx`/`fe` banks — the UI contracts under `service-defs/data/audio`
  expose only city/team/goal-horn lists and integer narration, i.e. the front end has no league-audio
  service. That is a statement about the UI data contract, not about the runtime sound engine.
- Xbox 360: same conclusion as handoff 15 (narrowed): no league-named bank; generic banks undecoded.
