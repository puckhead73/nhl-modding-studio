# Handoff R4 — league audio consumers (P1-H): what is league-keyed inside the generic banks and definitions

Research lane, 19 September 2026 (queue entry R4 of the [overnight research brief](../../plans/overnight-research-2026-09-19.md)).
Claim `research-r4-league-audio`. Consumer: developer **P1-H** (coverage-matrix row "League audio consumers") and,
as a shared-format dependency only, Phase 2.7 (horns/PBP). Extends handoffs
[15](../overnight-2026-09-18/15-audio-banks-xbox360.md) (Xbox path inventory) and
[19](../overnight-2026-09-18/19-goalhorn-ids-and-hornbank-static.md) (PS4 horn bank); their inventories are not
repeated. No audio was decoded or played; no launch; no writes.

Machine-readable: [`data/r4-league-audio-consumers.json`](data/r4-league-audio-consumers.json) — per source
(Xbox 360 Legacy/15/12/11/10/08, PS3 Legacy/12/10/08, PS4 15–21): audio member inventory per archive
(counts by extension/folder) and every definition/context member whose identifier strings match the league leads,
with the member's magic, decoded length, SHA-256 and the matching names. Probe: `league_audio_survey.py`.

**Evidence vocabulary:** **observed** = identifier strings read from the named members (event definitions, context
tables, Lua/XML/config); **inferred** = a reading of what a name means; **unknown** = not established. Nothing here
traces which sample plays; that is the bank index (undecoded, handoff 19 §2) plus runtime.

## Plain-language summary

There is no separate "league sound bank" on any platform — handoff 15 already said so for Xbox file names — but
the *definitions* that drive commentary and crowd sound do know about leagues, and they say so in plain identifiers
that survive in every title from NHL 10 to NHL 21: the play-by-play engine has conditions such as `League_is_AHL`,
`League_is_NHL`, `cup_type_Calder_Cup`, `cup_type_Stanley_Cup` and (from NHL 19/20) `INTRO_MATCHUP_LEAGUE_AHL`,
`…_INTERNATIONAL`, `…_CHAMPIONS_GEN/FINAL`, `…_NATIONAL_SWISS`, `LEAGUE_nhl_or_ahl`; the crowd engine has
`INTRO_worldCupMode` / `gameMode_World_Cup` / `matchup_between_International_teams`; NHL 21's UI-sound table has a
`CHL_Wipe`; and NHL 19–21's music system swaps in a Nordic music bank when the *locale* is Nordic (not the league).
So league audio is **indirectly selected**: the game evaluates league/cup/mode conditions in compiled context
scripts and emits a named event, and the event resolves to samples in the generic PBP/crowd/UI banks. What Studio
could offer today is limited to what those formats allow: the definitions are readable (names, ids, structure) and
the banks are self-contained members that can be extracted, but no bank index or sample codec is decoded here, so
"replace the AHL cup-win line" is blocked on the SND10 bank formats, while "re-point a team's horn id" (Phase 2)
stays the one supported audio operation. NHL 10 ships its context tables as plain XML — the best static witness of
the condition vocabulary — and the same condition names recur as strings in the binary tables of every later title.

## 0. Sources and formats (observed)

| Family | Definition members (per title) | Container / magic | Readable? |
|---|---|---|---|
| Xbox 360 / PS3 NHL 10 | `audio.big:audio/context/{pbp_eng,crowds}_contexts.xml` (3.4 MB / 1.2 MB plain XML), `audioboot.big:…speecheventsubscr.xml`, `…sentencedata.xml`, `pbp_eng_speecheventdef.csi` (`MOIR`), `*_contextgasscript.gas` (`CSIS` bytecode) | XML plaintext + MOIR + compiled GAS | XML fully; MOIR names; GAS no strings |
| Xbox 360 / PS3 NHL 11–Legacy | `nocache.big` (`data0.big` on 11): `audio/context/{pbp_eng,crowds}_contexts.bin` (`CTX0`), `…contexteventsubscr.bin` (`EVT0`); `audioboot.big`: `pbp_eng_speecheventdef.csi` (`MOIR`), `…speecheventsubscr.bin` (`EVT0`), `…sentencedata.bin` (`SPC0`) — identical member sets on PS3 (279/280 names; PS3 keeps `pbp_eng_samples.sbs/.sbr` **loose** under `USRDIR/audio/speech/`, plus loose `ann_eng_*`), sizes differ (platform codecs) | binary tables with embedded identifier strings | names + ids only |
| PS4 15–21 | `inxp_40.big` (`inxp40.big` on 15): `audio/speech/eng/pbp_contexts.bin` (`0XTC`), `pbp_contexteventsubscr.bin` (`0TVE`), `pbp_speecheventdef.csi` (`MOIR`), `pbp_sentencedata.bin` (`0CPS`), `crowds/crowds_contexttooloutput.bin` (`0XTC`), `augasm/*.bin` (`0GUA` AUGASM scripts: `music.bin`, `rinkannouncer.bin` …), `*_harmony_events.{csi,bin,xml}`, `ui_sfx/ui_ufo.ufo` (XML), `config/*.cfg`, `nhl/data/common/audio/lua/nhl_audio.lua` (plain Lua); banks `*.sbr/*.sbs` (`SBle`/`SBKS`) | little-endian counterparts of the gen7 tables + plaintext XML/Lua/cfg | names + ids; XML/Lua fully |
| NHL 08 (both) | `audioboot.big` 13, `data0.big` 233 audio members | no league-lead strings in any text-ish member | — |

Studio decode status: none of `MOIR`/`CTX0`/`0XTC`/`EVT0`/`SPC0`/`0GUA`/`SBle` is parsed by Studio; the members are
extractable as whole payloads (chunkzip/chunklzx/raw wrappers per handoffs 40/02). The PS4 `music.bin` AUGASM script
is where `music_inxp_nordic` is bound (`@IsNordicLocale`), and `harmony_events.xml` lists UI event ids as
`InterfaceId {"CHL_Wipe", 8660, 5592}` — plain text.

## 1. League-keyed records (observed names; full lists in the JSON)

| Engine | Title range | Identifiers (exact) | Where | Reading (inferred) |
|---|---|---|---|---|
| PBP context conditions | NHL 10 (XML) | `League_is_AHL`, `League_is_NHL`, `League_is_Valid`, `cup_type_Calder_Cup`, `cup_type_Stanley_Cup`, `cup_type_EA_Cup`, `cup_type_generic_Cup`, `cup_type_generic_NOT_AHL_or_NHL`, `playoff_cup_type_{Calder,Stanley,EA,Generic}_Cup`, `gameMode_International`, `gameMode_International_Mode_mute`, `DYN_EXTRO_AllStar_*` | `pbp_eng_contexts.xml` `<Condition name=…>` | commentary contexts gated by league/cup/mode; the condition bodies are `scriptID`s into the compiled `.gas` (no source) |
| PBP output events | NHL 10 → Legacy (Xbox+PS3), PS4 15–17 | `CUP_END_CUP_AHL`, `CUP_END_WON_AHL` (10); + `CUP_END_CUP_CHL`, `CUP_END_WON_CHL` (11+), `RAY_FIRST_STOPPAGE_INTERNATIONAL_TEAMS` (15+/PS4) | `pbp_eng_speecheventdef.csi`, `…contexts.bin`, `…sentencedata` | cup presentation lines for the Calder Cup / CHL cups; international-teams line |
| PBP output events | PS4 18 | `BETWEEN_THE_GLASS_FIRST__INTERNATIONAL_TEAMS__RAY` | same | |
| PBP output events | PS4 19 | + `INTRO_MATCHUP_LEAGUE_CHAMPIONS_GEN/FINAL`, `INTRO_MATCHUP_LEAGUE_NATIONAL_SWISS` (also in `patch_inxp_52/53/54.big` copies) | same | Champions Hockey League and Swiss league intros |
| PBP output events + conditions | PS4 20–21 | `INTRO_MATCHUP_LEAGUE_AHL`, `…_ALLSTAR_GAME`, `…_INTERNATIONAL`, `…_NHL_ALUMNI_TEAMS`, `…_TOP_VS_BOTTOM` (21), `…_CHAMPIONS_GEN/FINAL`, `GOAL_AWAY_ALUMNI_TEAM`, `GOAL_AWAY_INTERNATIONAL_TEAM`, condition `LEAGUE_nhl_or_ahl`, `GAME_OVER_CUP_WON_CHAMPIONS_LEAGUE`, `GOAL_USER_SCORED_CHAMPIONS_LEAGUE_BAP`, `FACEOFF_RESULT_PERIOD_1_OPENING_CHL_*` | `pbp_contexts.bin`, `pbp_speecheventdef.csi` | per-league matchup intros; the league enters as a pre-evaluated condition, no raw league-id variable name appears |
| Crowd contexts | NHL 10 → Legacy; PS4 16–21 | `INTRO_worldCupMode` with condition `gameTypeWorldFinal` (gen7); `gameMode_World_Cup`, `gameMode_NOT_World_Cup`, `matchup_between_International_teams`, `CHANT_AWAY_WORLDCUP_*`, `GOAL_WORLDCUP_{GENERIC,CANADA_OR_USA,VS_CANADA_or_USA}_{home,away}`, `Own_GOAL_*_WORLDCUP_*`, `NIS_CUP_WORLDCUP`, `PEN_CALL_WORLDCUP_Generic` (PS4 21) | `crowds_contexts.{xml,bin}`, `crowds_contexttooloutput.bin` | competition-keyed crowd behaviour (World Cup / international), not club-league-keyed |
| UI SFX | PS4 21 | `CHL_Wipe` (`InterfaceId` 8660/5592), `EASHL_*_Wipe`, `BAP_LeagueLeaders_Show` | `ui_sfx_harmony_events.{csi,bin,xml}`, `ui_ufo.ufo` | a CHL-branded UI transition sound |
| Music | PS4 19–21 | `MusicInxpNordic` ← `@IsNordicLocale`; banks `music_inxp_nordic.sbr/.sbs` (4.7 MB), `musicbank(.sbr/.sbs 837 MB)`, `musicbank_embargoed` | `augasm/music.bin`, `music/` | **locale**-keyed alternative music bank (Nordic SKUs), not league-keyed |
| Front-end BAP | PS4 16–21 | `NEWS_AROUND_LEAGUE_*`, `BAP_CHAMPIONS_LEAGUE_*` | `fe_bap_commentary/*` | mode narration, "league" = the user's league generically |
| Stats lines (all) | 10–21 | `STATS_LEAGUE_LEADERS_*`, `stats_league_leaders_type`, `LG_STANDINGS_LEAD_LEAGUE` | PBP | league-leader stats, league-agnostic |

**Not league-keyed by name (scoped negative):** goal/period horns and chants (arena-row ids, handoffs 15/19); the
rink announcer (`RinkAnnouncerSamplesEnglishLcu1–10`, per-arena "LCU" sample sets on PS4); menu music selection
(locale/embargo only). NHL 08 carries none of the lead strings in its text-ish audio members.

## 2. Id joins and parsing confidence

- **Trigger/condition/event ids.** NHL 10 XML exposes them (`Context id`, `Condition scriptID`, `Csis SystemCrc /
  InterfaceCrc`, `Sentence id`); the same identifier strings occur in the binary `CTX0`/`0XTC`/`EVT0`/`0TVE`/`SPC0`/
  `0CPS` tables of every later title, so their record layouts can be pinned against the NHL 10 XML (a bounded
  decoder task; **not done here** — confidence: names only).
- **Sample binding.** `SPC0`/`0CPS` sentence data → phrase → sample slot in `pbp_*_samples.sbr` (SND10 `SBle`/`MOIR`
  index) → `.sbs` stream. The SBR/DSET index is the same open item as handoff 19 §2 (goal-horn bank); decoding it
  serves horns, PBP and league lines alike — the one **shared-format dependency** this handoff points at.
- **League input.** Conditions such as `League_is_AHL` are evaluated by compiled GAS (`CSIS` bytecode) over runtime
  state; the variable that carries the league is not named in any static member (the XML declares only trigger
  params such as `cup_endings_sequence_number`). Which league id maps to which condition is therefore **unknown**
  statically (the names `AHL`/`NHL`/`CHL`/`Calder_Cup`/`Stanley_Cup`/`generic_NOT_AHL_or_NHL` bound the vocabulary:
  gen7 commentary distinguishes NHL, AHL, CHL/EA cup and "generic other league").

## 3. Extraction / decoding available vs blocked

| Operation | Status |
|---|---|
| Extract any definition or bank member (Xbox chunklzx/raw, PS3 chunkzip/raw, PS4 chunkzip/raw) | implemented read paths (handoffs 40 ✓ for chunklzx via research decoder; Studio's `decompress_chunklzx` still needs the handoff 40 fixes) |
| Read the league vocabulary (names, ids) from definitions | possible now (string scan); structured decode = bounded new work against the NHL 10 XML witness |
| Replace a whole bank member of a different size | packaging per handoff 02 (EB/BIG4 rules); PS3 loose `audio/speech/*` = file write |
| Replace / add a league-specific line or crowd cue | **blocked**: SND10 bank index and sample codecs (XMA / EALayer3 / XAS — not identified per platform here) undecoded; the context tables are compiled |
| Re-point a horn id (Phase 2) | supported route (DB field); runtime card in handoff 19 |

## 4. P1-H matrix wording (proposal)

"League audio: no league-named bank on any platform (15 ✓, PS3 identical member set, PS4 inspected); league-keyed
*records* exist inside the generic PBP/crowd/UI definitions on NHL 10–Legacy (Xbox+PS3) and PS4 15–21 (research 04,
names listed — identifiers only; the event → sample → codec binding is unresolved, rev. 2) and are selected indirectly by
compiled conditions; Nordic music is locale-keyed; no Studio operation is
scoped beyond extraction until the SND10 bank index is decoded (shared with 2.7 horns) — cell = **research unknown →
blocked (format)** with the blocker named, not n/a."

## 5. Cards (later; bounded)

- **R4-AUD-1 (NHL 21 PS4, listen-only, needs no edit) — rev. 2 wording (review F3):** play an AHL-vs-AHL exhibition
  and an NHL game from the same profile; record whether the pre-game commentary differs. A difference is **consistent
  with** a league-keyed intro (`INTRO_MATCHUP_LEAGUE_AHL`) but does not show that this named event fired: the two games
  also differ in teams, arena, presentation package and random line selection. Isolation needs controls on the same
  title — repeated runs of each setup (line-selection variance), and the closest same-teams pairing the mode allows
  (e.g. an AHL club in an AHL context vs the same club in a custom/other-league context) — before the difference is
  attributed to the league condition. The event → sample → codec binding stays unresolved either way (§3).
- **R4-AUD-2 (shared with 2.7):** handoff 19 §3 test 1 (horn id → sound) is the discriminating runtime test for the
  bank-index question; no league-specific card is proposed until the index is decoded.

## 6. Reproduce, checks, cleanup

`python local/runs/overnight-research-2026-09-19/probes/league_audio_survey.py <out.json>` (string scan of every
text-shaped audio member ≤ 3 MB on 17 sources; wrappers decoded in memory). Ad-hoc python: NHL 10 XML condition
listing, the GAS header check, the Xbox-vs-PS3 Legacy member comparison (279 shared names, 7 equal sizes, `ABKC`/`SBKS`/
`S10S`/`MOIR` magics on both; PS3 `pbp_eng_samples` loose). Scratch `local/runs/…/r4-audio/` deleted after the JSON
was copied to `data/`.
