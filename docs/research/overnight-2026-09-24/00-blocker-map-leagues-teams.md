# R0 — League and Team research blocker map (24 September 2026, 05:15 MDT)

Research lane, fresh overnight session under
[overnight-research-2026-09-24-leagues-teams.md](../../plans/overnight-research-2026-09-24-leagues-teams.md). Claim
`research-r0-blocker-map-20260924`. Baseline recorded, not assumed: HEAD `8d60a6d` on `feat/phase5-asset-tooling`, the working tree
dirty with 791 modified / untracked paths from the developer campaigns (preserved), `docs/research/` and `local/` untracked; no developer
campaign directory or claim for 24 September existed at 04:57 MDT (`local/coordination/overnight-2026-09-23/claims/` holds the six
`dev-r4-*` claims, all delivered). Clock at start: 24 September 04:57 MDT. Sources: the gen7 trees on `X:` (PS3 and Xbox 360), the
committed anchors, the research JSONs of the earlier campaigns. Nothing under `X:`, `anchors/`, application code, `defs/`, bindings or
developer records is written; no launches, no listening; nothing staged, committed or pushed.

This map reconciles the League (1.x) and Team (2.x) research queue against what the developer has implemented (receipts through
2026-09-23 receipt 06 and the two coverage matrices' R4 addenda) and classes every open item by the kind of evidence that closes it.
It is a triage record, not a re-review of the historical receipts.

## Plain-language summary

Most of what the research lane delivered for Leagues and Teams is already in Studio: schedules, competition groups, dictionaries,
logo replacement with archive placement, the PS4 venue listing, the PS3 12 / 13 kit join and preview, the song and horn catalogues.
What remains falls into five kinds. Two are static format questions the lane can still attack tonight (the archive-only gen7 audio
cells, the unidentified speech codecs, the Xbox packed-tail layout under a new method only). Two need writer contracts that do not exist
yet (team identity cascades, kit / arena / audio replacement). One kind can only be settled by running the game (embedded-PCM start,
horn fallback, venue selection, participant selection, which dictionary copy a console loads) and is carded, not researched further.

## 1. Boundaries carried forward (binding on every handoff of this campaign)

| Boundary | Statement in force |
|---|---|
| R60 | INVALID / SUPERSEDED; its slot measurements are cited nowhere. H1 is a **candidate** per-level placement rule with partial static support (R70: 359 located levels agree; 320 unresolved; DXT1 / 3 / 5 on four titles only) — not a full-chain Xbox mip writer contract. |
| EA-MP3 candidate A | diagnostic: the junction test supports A on 20 / 24 vectors (R71 / R75); B is "not supported by this junction test"; nothing heard. |
| Gen7 codec `0x05` | the R77 grammar is established per measured tuple (R79 narrowing, R80 widening); start handling (two warm-up granules + 47 PCM samples) is open (Q-R77-1). `0x09` and EA-XMA are different classes with no decoder route. |
| R73 venue discovery | a measured join (vault level `ID == arenaartid`) with named ambiguities; **re-pointing withdrawn** (Q-R69-1) pending a complete cascade contract. |
| PS3 12 / 13 kit members | `chunkzip`-wrapped (R76); this removes the specific `chunklzx`-encoder blocker only — writer placement, transaction and loader acceptance remain the developer's to assess. |
| NHL 11 / 12 sock pairs | classes stay separate: clear mirror 459 / 704, identical 153 / 124, mirror-like or other 120 / 119, flat 24 / 26 (R74 / R75); both files required. |
| Superseded wording | "impossible" (R71's candidate B) → "not supported by this junction test"; city / arena ids "not bank ids" → "no established binding from these tests; numerical coincidences reported; searched scope = loose numbered banks" (R79). Any earlier queue row that still says otherwise is read through R75 / R79. |

## 2. Blocker map — Leagues (Phase 1)

| Item | Implemented (developer) | What is still open | Class | Next handle |
|---|---|---|---|---|
| 1.1 / 1.3 NHL 07 league labels | team text on 07 blocked (labels not in the dictionary) | where the NHL 07 competition / league labels live (no dictionary key; exhaustive string searches recorded in 2026-09-19 handoff 04 and the `nhl07_label_grep*` probes) | **static, exhausted by string search** — reopen only with a new discriminating method (e.g. a pointer walk from the FE Ion tables, or the NHL 08 executable as an era control) | deferred behind R1–R3; no repeat of the string search |
| 1.2 rules / settings | NHL 10+ 32-byte descriptor tables catalogued (p1bs), PS4 xtitle / eboot settings catalogued; stored defaults editable where fields exist | NHL 08 / 09 X360: settings are FE Ion adapter labels `<Setting>.%d` with code-resident ranges (R55); PS3 08 / 09 unmeasured; which of main-DB default, CSV, exe catalogue, mode-save value wins at runtime | **static bounded (R55)** for 09; **runtime** for precedence | card L-SET-09 stands; NHL 08 executable is the one unmeasured era control |
| 1.2 creation / growth, tournament participants, group mappings, schedule capacity | schedules edit / import / generate inside the table's row count; PS4 groups validated per witness | new rows (league / team creation), CHL participants (code-resident, R55), weighted season generation | **writer contract missing** (row allocation + every referencing table) for creation; **runtime** for CHL (card L-CHL-1) | inventory of references for a new team / league row = Team 2.1 item below (shared) |
| 1.4 league logos / imagery | preview + replace + archive placement on every family; different-dimension writes on XPR2 / PS3 DDS; RW4xb2 mipped resize **refused**; same-size partial replacement disclosed | Xbox packed tail (Q-R65-1 / Q-R66-1) — H1 candidate only; RW4ps3 / GNF descriptor regeneration for changed dimensions | **static, new-method-only** (brief R5) | no package unless an independent method appears; keep the refusal |
| 1.5 audio | song / horn catalogues, dictionary-resolved titles; no reader for the gen7 `.abk` / `SBKR` banks in Studio yet; replacement blocked everywhere | archive-only gen7 cells (PS3 12 / 13 sfx + music, PS3 09 / 10 music) unsurveyed; the chant entry-1 residual; `0x09` / EA-XMA unidentified; league-keyed cues inside generic banks selected by compiled conditions (no static join to a sample); no encoder anywhere | **static** (R1 / R2 of this brief) for coverage and codecs; **writer contract missing** (bank / event mutation) for replacement | **R1 in progress** (handoff 01); R2 next |

## 3. Blocker map — Teams (Phase 2)

| Item | Implemented (developer) | What is still open | Class | Next handle |
|---|---|---|---|---|
| 2.1 identity re-identification (`artid`, team index), copy / create / delete | names / abbreviations / colours through the field profile; colour draft binding | the complete reference and asset dependency graph of an `artid` / team index (2026-09-20 handoff 07 §1 gave a reference census, not a cascade contract); id namespaces, sentinels, collisions; asset families keyed by `artid` (logos, kits, banners, PS4 vault links) | **writer contract missing** — a narrow, enumerable operation is the target (R4 Team scope) | R4 candidate: enumerate every reference per title from the anchors + asset listings already measured; propose refuse-or-cascade with the unknowns named |
| 2.2 additional entries | listing surfaces; league membership editing | edit / save / reload / restore parity for roster links, numbers, affiliations, rivals; team / league creation and deletion (allocation contracts) | **developer integration** for existing tables; **writer contract missing** for creation | creation shares the 2.1 reference graph |
| 2.3 localization | team text by `artabbr` through the project's own dictionary (gen7 + PS4) | shared strings / aliases across consumers; which copy a console loads (runtime) | **runtime** + small static (alias listing) | alias census is cheap if a package needs it |
| 2.4 logos / imagery | preview / replace / archive placement; contradictory scopes refused | changed-dimension RW4xb2 / RW4ps3 / GNF; enumerated-but-unpreviewable families | as 1.4 | as 1.4 |
| 2.5 arenas / ice | arena identity reads; PS4 venue listing by level name (R73) with texpack keys and rink id; venue field inventory (R51) | gen7 `rendering/*` ice / centre-logo members (`chunklzx`, no encoder; Studio's decoder not corrected); PS4 ice texpack selection (`CenterLogo` / `CenterLine` / `IceLine` keys resolve to sets, not a chosen member); home-team vs venue-keyed art; runtime selection (Q-R69-1) | **static bounded** for listing; **writer contract missing** for any texpack replacement; **runtime** for selection | inspection-only contract possible for the PS4 texpack sets; reassignment stays withdrawn |
| 2.6 kits | gen7 kit resolver incl. PS3 12 / 13 archive join and member preview; NHL 11 / 12 sock pairs | PS4 duplicate-pack `(Team, UniformVariation, JerseyFit)` selection (`IsTopScorer` / override groups unresolved); gen7 spare-pool semantics; any component replacement / copy / repoint (loose route exists for textures; archive-member write for PS3 12 / 13 not assessed) | **runtime** for PS4 selection; **writer contract** for repoint / copy | repoint contract = the 2.1 graph restricted to kit rows |
| 2.7 audio | song / horn catalogues; no bank reader | as 1.5, plus the residual horn ids (R81: absent in inspected sources; `aemshorn` segment-built horn = candidate), chant candidates, PS3 12 / 13 not inspected | **static** (R1: PS3 12 / 13 horn survey through the archives) then **runtime** (card H-HORN-1) | R1 in progress |

## 4. Queue reconciliation notes

- Queue rows 0–13 of the 2026-09-22 pm campaign stand as corrected; no summary there contradicts its handoff body after R75 / R79. Row 6
  is superseded by row 9 / 12 (already marked). Row 10's original `cityaudioid` / `arenaaudioid` wording is read through its rev. 1a note.
- The developer's 23 September STATUS still lists "no `chunklzx` encoder" as the PS3 12 / 13 member-write blocker; R76's correction
  (`chunkzip`) is published and consumed by the developer brief's P2 text — no research action.
- The developer's 23 September STATUS names "which dictionary file / screens a PS4 title reads at runtime for song and horn strings" as a
  blocker: **runtime**, no static handle beyond the copy inventory already delivered (2026-09-19 handoff 13 / 20).
- Not repeated this session without a new method: the NHL 07 label search, the R70 classifier, the R71 junction test, the R78 / R81
  loose-bank listings.

## 5. Package order for this session

R1 (archive-backed gen7 audio coverage + the chant residual; PS3 12 / 13 horn survey) → R2 (`0x09` identification; EA-XMA preview
contract assessment) → R3 (consumers: horn ids, name / city / chant joins, PCM start evidence, read vs replacement contracts) → R4
(Team 2.1 reference graph → narrow re-identification contract; League rules / creation references) → R5 (packed tail only under a new
method; packaging gaps). Fallback per the brief: fixture contracts, refusal cases, morning cards, then later phases.
