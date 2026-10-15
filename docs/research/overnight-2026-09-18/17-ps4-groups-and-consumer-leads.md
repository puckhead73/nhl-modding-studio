# Handoff P1.2-PS4 — PS4 competition groups (executable table) and UI consumer leads (league, audio, arena, calendar)

Research lane, 18 September 2026 (R1 follow-on). Consumer: developer Phase 1.2 (`GroupModel::Ps4` currently
shows raw ids), 1.4/1.5 (league presentation and audio on PS4), C6 schedules. Machine-readable:
[`data/ps4-competition-groups.json`](data/ps4-competition-groups.json) (same shape as
`xbox360-competition-groups.json`: per title `groups[{group_id,key,text,evidence[,art_id]}]` +
`team_table_groups_by_league`), the raw per-title eboot walks in [`data/ps4-groups/`](data/ps4-groups/), and
[`data/ps4-nhl21-service-defs-digest.json`](data/ps4-nhl21-service-defs-digest.json) (URN / description /
schema keys of the NHL 21 `service-defs/data/{league,competition,arena,audio,asset,calendar,jersey,
competitive_seasons}` contracts — a derived digest, no file bytes). Probes: `ps4_group_table.py`,
`ps4_groups.py`, `ps4_groups_build.py`, `ps4_service_defs.py`, `ps4_member_grep.py`.

Evidence: **executable table discovery** (static; observed in the seven `eboot.bin` files under
`H:\Emulators\ShadPS4\Games\CUSA*` — fake-signed SELF/ELF with plaintext segments, relocation-walked) is kept
separate from **consumer tracing**, which was not done: nothing below says which code reads the table. The
id ↔ key binding is established by cross-checking the executable order against every group id the team
table uses; where the two disagree the row is marked `inferred`.

**Source witnesses (corrected 2026-09-18 follow-up; `data/ps4-install-witnesses.json`, and the `witness`
block of every title in `ps4-competition-groups.json`).** Each mapping is built from *one install's own
files*: the `eboot.bin` (SHA-256 recorded) and the base `nhl_game.db` / `eng_us.db` read out of that
install's `inxp_00.big` (SHA-256 recorded) — **not** from the repo anchors. This matters for NHL 20: the repo
`anchors/ps4/nhl20` databases are NHL 19 base copies (`265e4a68…`), whereas the real CUSA15439 DB is
`1e200478…` (10 226 516 B); the NHL 20 rows now use the real DB (same group-id usage as NHL 21 except Pro Am
80 vs 97 teams). Anchor = install for NHL 15/16/17/18/19/21 (hashes equal). Build notes: NHL 15 `param.sfo`
lists 01.00/01.06 and ships a `patch_inxp00.big` DB (`abe36f58…`); NHL 19 lists 01.00/01.55/01.60 and ships
six patch DBs (`patch_inxp_51…56`) — its group-id usage in `patch_inxp_56` equals the base except league 23
(Pro Am) growing 80 → 84 teams, so the mapping holds for the patched build too; the NHL 19 eboot on disk is
the one the install carries (`426297a6…`), its exact patch level is not separately recorded. NHL 21 lists
01.00/01.01 (one DB).

## Plain-language summary

PS4 titles keep the conference/division names in the executable like NHL 12–Legacy do, but as a plain pointer
array of dictionary keys (NHL 15/16: `{key, art id}` records like gen7). The team table's `leaguegroup` /
`conferencegroup` / `divisiongroup` values map onto that array with a fixed offset: **id 1 = NHL, ids 6…N =
the array from `WesternConference` onward**. That rule resolves every group id the seven anchors use except:
NHL 17, where the executable stores the ECHL block earlier than the ids imply (the ids follow the NHL 18
layout — resolved from data usage, marked inferred); NHL 18's Spengler Cup ids 74–76 (not in its array; keys
taken from NHL 19–21); and NHL 19–21's ids 77 (Alumni/All-Time) and 78 (Pro Am), whose keys are unknown.
Studio can now label PS4 conferences and divisions in the title's own words with the same per-title caveat
as gen7 (ids do not transfer between titles), and the executable's league-name debug list, the
`stockleagues` row and the UI service contracts show how the PS4 front end consumes league presentation
(art id + two colours), league text (`LocStringId` + `OrderId`) and audio ids (team/city/goal-horn lists).

## 1. Group table per title (observed in the executables; `data/ps4-competition-groups.json`)

| Title (CUSA) | Format | Table VA | Record | Entries from `WesternConference` | id rule | Exceptions |
|---|---|---|---|---|---|---|
| NHL 15 (00561) | SELF, non-PIE (base `0x400000`) | `0x3d57a30` | 16 B `{char* key, u64 art id}` — `NHL` first | 49 (…`CUSTOM_TEAMS`) | id 1 = NHL; id k ≥ 6 = entry k−6 | none; 6 AHL divisions (15–20), European ids 21–25 |
| NHL 16 (01909) | ELF | `0x330fd50` | 16 B, `NHL` first | 48 | same | none |
| NHL 17 (04326) | SELF | `0x3f91cc0` | 8 B pointer, `NHL` first | 55 (ids 6–60) | same for ids 6–42 | ids 43–60: the array stores `ECHL, W, E, Central, Mountain, North, South` **before** `TournamentFile…CUSTOM_TEAMS`; the DB uses tournament 43–52, `CUSTOM_TEAMS` 53, ECHL block 54–60 (8-team World Cup league uses 43/44/45 = tournament/PoolA/PoolB; 27-team ECHL uses 60 with 54/55 and 56–59) → rows marked `inferred` |
| NHL 18 (07569) | SELF | `0x464f340` | 8 B, **five** `NHL` pointers precede | 68 in the array (…`TXT_QUALIFYING_POOL` = 73) + 3 inferred | same (the five NHL pointers sit exactly at ids 1–5) | ids 74–76 (`TXT_SPENGLER_CUP`, `TXT_GROUP_CATTINI`, `TXT_GROUP_TORRIANI`) absent from the array; DB league 20 uses 74/75 → inferred from the NHL 19–21 order |
| NHL 19 (11126) | ELF | `0x58f6720` | 8 B, no NHL prefix | 71 (…`TXT_GROUP_TORRIANI` = 76) | same | ids 77 (leagues 21 `TXT_LEAGUE_NHL_ALUMNI` and 22 `TXT_LEAGUE_NHL_ALL_TIME`, both `stockleagues.artid` 25) and 78 (league 23 `TXT_LEAGUE_PROAM`) **unknown** — no static string-pointer run in any of the three executables names them (`ps4_string_runs.py`: every RELATIVE-relocation run of stride 8–32 pointing at strings was checked for `ALUMNI`/`PROAM`; only mode/trigger/settings tables reference those words). Hypothesis (not evidence): these two are appended at runtime per league, labelled by the league's own key. Keep raw + debug label |
| NHL 20 (15439) | SELF | `0x57f17c0` | 8 B | 71 | same | as NHL 19 |
| NHL 21 (18182) | SELF | `0x5e041f8` | 8 B | 71 | same | as NHL 19 |

Resolved layout on NHL 19–21 (ids): 1 NHL · 6/7 W/E conference · 8–11 `NHLDivisionA–D` · 12 AHL · 13/14 ·
15–18 Atlantic/North/Central/Pacific · 19 O2 Extraliga · 20 Swiss · 21 SM-liiga · 22 DEL · 23 elitserien ·
24 Norway · 25 OHL · 26/27 · 28–31 Central/East/Midwest/West · 32 QMJHL · 33–35 East/Maritimes/West · 36 WHL
· 37/38 · 39–42 BC/Central/East/US · 43 `TXT_TOURNAMENT` · 44/45 PoolA/B · 46 Playoffs · 47/48 ConferenceA/B ·
49–52 DivisionA–D · 53 `CUSTOM_TEAMS` · 54/55 · 56–59 Central/Mountain/North/South · 60 ECHL · 61 Champions
League · 62 EBEL · 63 Allsvenskan · 64–71 `TXT_GROUP_A–H` · 72/73 placement/qualifying pool · 74 Spengler ·
75/76 Cattini/Torriani. Dictionary texts are in the JSON (`eng_us.db` anchors, by key then by hash).

NHL 17 anomaly, restated with its own witness: the CUSA04326 install's `nhl_game.db` (`de48b02b…`, = anchor)
uses ids that follow the NHL 18-style block order (tournament 43–52, `CUSTOM_TEAMS` 53, ECHL 54–60), while the
same install's `eboot.bin` (`6aa1ff7b…`) stores the ECHL block *before* the tournament block in its pointer
array. Both come from the same build, so **position in the pointer array is not, by itself, the runtime id
on NHL 17** — the mechanism that produces the ids (a descriptor table, per-league block lookup, or something
else) was not found statically. The NHL 17 rows for ids 43–60 are therefore `inferred` from data usage
only; the same caveat is the reason ids 1–5 and 77/78 cannot be read off the array either.

Notes: the ids are **title-specific** exactly as on gen7 (NHL 15: European 21–25 and OHL 26; NHL 16+: 19–24
and 25); Winter Classic / All-Star rows reuse NHL ids; NHL 15/16 `art_id` column = the family-wide league art
id (NHL 0, AHL 1, …, custom 5 — matches `stockleagues.artid`). Ids 0 and 2–5 are never used by a team row.

## 2. Consumer leads (static, from data files — not traced into code)

- **League presentation (UI contract)**: `service-defs/data/league/presentation.json` —
  `LeaguePresentation{LeagueId, Presentation{ArtId, PrimaryColor, SecondaryColor}}`; `league/bio.json` —
  `{LeagueId, Bio{LocStringId}, OrderId}`. These are exactly the `stockleagues` columns (`artid`,
  `primarycolor`/`secondarycolor` 24-bit, `name` = dictionary key, `orderid`), so on PS4 a league's logo,
  colours, display name and menu order are DB edits (Phase 1.1/1.4 candidates), while the group names stay in
  the executable. The contract's comment lists the `LeagueTeamLeague` enum order (`NHL, AHL, ELITESERIE,
  SMLIIGA, DEL, EXTRALIGA, SWISS, NORWAY, NATIONAL, OHL, QMJHL, WHL, PROSPECTTEAMS, ECHL, WorldCup,
  CUSTOMTEAMS, NHL, EASHL, CHL, US, RUSSIA, ROWLEAGUE, HUT, LEGENDTEAMS, THREEONTHREE`) — a stale shared
  header, not NHL 21's league ids (which run NHL 0 … Pro Am 23, `CUSTOM_TEAMS` 24/25, CHL 26, USA 27, Russia
  28, ROW 29 per `stockleagues`; the executable's debug list `NHL, AHL, Swedish, Finnish, German, Czech, Swiss,
  Austrian, Swedish 2nd, Champions, International, OHL, QMJHL, WHL, Prospects, ECHL, Custom, EASHL, Mascot,
  Three on Three, Spengler, Alumni, Alumni Six, Pro Am` at NHL 21 file `0x5b9b120` matches the DB order 0–23).
- **Audio ids (UI contract)**: `audio/ids.json` — the UI asks for `CityData.CityList[{Id,Name,DictLetter}]`,
  `TeamData.TeamList[{Id,Name,DictLetter}]` and **`GoalHornData.GoalHornList[{Id,Filter,Name}]`** ("possible
  cities, teams and goal horns that a user can select for their team"); `audio/ids_for_team.json` returns
  `{TeamAudioId, CityAudioId}` for a team id (`NONE` if unassigned). So PS4 has a selectable goal-horn id
  space with a `Filter` (league?) and per-team audio/city ids — the same join shape as the Xbox `audioid` /
  `cityaudioid` fields (handoff 15). **PS4 audio banks (NHL 21, observed by member name;
  `data/ps4-nhl21-audio-banks.csv`, 290 members):** everything sits under `nhl/data/kettle/audio/` as EA
  SBR/SBS pairs (`SBle` little-endian bank headers): `inxp_40.big` — `sfx/hornbank.sbr` (44.6 MB, samples
  embedded in the SBR, **one bank for all goal horns** — no per-team horn members, so the PS4 horn join is an
  id inside this bank), `sfx/sfxbank(_outdoor).sbr`, `crowds/*` (crowd beds, reactions, applause),
  `speech/eng/pbp_samples_inxp.sbr/.sbs` (1.0 GB play-by-play), `rink_announcer/eng/rink_ann_samples_inxp`
  (219 MB), `music/music_inxp(.sbs 557 MB)`, `ui_sfx/*`, `fe_bap_commentary/*`; `game_80.big` —
  `music/musicbank` (837 MB), threes/gauntlet commentary, `speech/eng/pbp_samples_bank_montreal`; crowd
  *meshes* (`rendering/crowd/<city>_crowd.crowd`) in `game_90.big`. No league-named bank member exists in
  NHL 21's archives either; league-related cues, if any, are inside the generic banks (not decoded — same
  scope limit as handoff 15). The SBR index format is the O6 music-SBR work's domain; whether the Xbox
  `{id}_goal.abk` ids equal the PS4 hornbank slot ids is **unknown**.
- **Arena (UI contract)**: `arena/bio.json` `{ArenaId, Bio{CityNameLocId, ArenaNameLocId, Rating,
  MarketSizeId, Age, TeamId, SeatingCapacity}}` and `arena/properties.json` `{MaxAttendance, CityName,
  ArenaName}` — the arena text on PS4 is dictionary-keyed (`*LocId`), consistent with the gen7 `TEAMLINE`/
  `NHLCityName_` families (handoff 23); the PS4 key grammar itself was not measured.
- **Calendar (UI contract)**: `calendar/schedule.json` renders the season from `MonthList[{MonthId,
  NumberOfDays, Year, FirstDayOfWeek}]` and `league_info` `{CurrentDate YYYYMMDD, CurrentPhase,
  CurrentLeagueId (LeagueTeamLeague enum), CurrentLeagueIndex}` — i.e. the mode calendar is derived, which
  fits the `leaguescheduletable`/`leaguescheduleperiods` mode copies (handoff 12 §1.4) rather than the source
  tables.
- 148 service areas exist (`asset`, `jersey`, `competitive_seasons`, `draft`, `franchise`, …); the digest
  covers the eight listed above. Other titles' contracts were not digested (NHL 15's `boot.big` is EB
  `0x00100400`; same probe applies).

## 3. What development can do now

1. Load `ps4-competition-groups.json` as `defs/leagues/groups/{nhl15..nhl21}.toml` analogues with
   `evidence` carried per row (`traced-static` / `inferred` / `unknown`), replacing the raw-id display; keep
   ids 77/78 raw with the debug label as a hint, never as the name.
2. Treat `stockleagues.{artid,primarycolor,secondarycolor,name,orderid}` as the PS4 league presentation
   record (logo, colours, text key, menu order) — the UI contract confirms those five fields are what the
   front end asks for.
3. Register the PS4 audio bank locations (`inxp_40.big` / `game_80.big`, `nhl/data/kettle/audio/**`) as
   source entries; queue the `hornbank.sbr` slot index and the goal-horn `Filter` semantics as open items
   (the UI contract proves per-team horn selection exists on PS4; the bank index was not decoded).

## 4. Reproduce

```
python probes/ps4_group_table.py "H:\Emulators\ShadPS4\Games\CUSA18182\eboot.bin" out.json
python probes/ps4_groups.py anchors/ps4/nhl21/databases/nhl_game.db anchors/ps4/nhl21/databases/nhl_game-meta.xml
python probes/ps4_groups_build.py docs/research/overnight-2026-09-18/data/ps4-competition-groups.json
python probes/ps4_service_defs.py "H:\Emulators\ShadPS4\Games\CUSA18182" out.json league competition arena audio
```
