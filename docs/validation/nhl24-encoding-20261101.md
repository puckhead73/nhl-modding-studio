# NHL 24 player encoding, archetypes and X-Factors, 1 November 2026

**Summary.**
- **Encoded fields now editable.** Weight, height, birth date, contract dollars and ratings are
  editable for NHL 24. NHL 24's player rows follow the same contract as PS4 NHL 15–21.
- **Archetypes and nationalities.** Player archetype (style) and nationality now show names on
  NHL 24 instead of raw numbers.
- **Evidence.** Two in-game screenshots of the Edmonton Oilers roster supplied by the user,
  plus public birth dates and nationalities. Every value checked matches.
- **X-Factors on NHL 27.** They are ordinary database fields on NHL 27, and NHL27Kit reads them
  there.
- **X-Factors on NHL 24.** They are *not* in NHL 24's database, and NHL24Kit and the NHL 24
  Roster Editor do not handle them. Where NHL 24 stores per-player X-Factors is still open.
- **AI traits.** NHL 24 now uses its own trait names, from the game's trait table (section 3).

Target: NHL 24 PS4 CUSA37933 01.60 (shadPS4), install `F:\Games\NHL24`. Studio game id `nhl24`.

## 1. Encoding contract and vocabularies

**Changes:**
- `encoding::SchemaFamily::of_game` now returns `Ps4` for `nhl24`. Without this the session
  reported an encoding conflict, and the encoded cells were read-only.
- The NHL 24 schema already classifies as `Ps4` by its layout signature, so the change agrees
  with `schema_family`.
- `defs/enums/player-style.toml` and `defs/enums/nationality.toml` add `nhl24` to `[scope]`.

**Evidence:** the test `nhl24_roster::oilers_match_the_in_game_roster_screen` (studio-api,
ignored; needs `NHL24_INSTALL` and `NHL24_OODLE`) passes. It compares nine Oilers on the
roster screen.

| Player | In-game style → Studio | Cap hit / years (in game = Studio) | Birth date (Studio = public) → in-game age | Nationality (public) |
|---|---|---|---|---|
| Connor McDavid | PLY → Playmaker | $12.5M / 3 | 1997-01-13 → 26 | Canada |
| Leon Draisaitl | SNP → Sniper | $8.5M / 2 | 1995-10-27 → 27 | Germany |
| Zach Hyman | PWF → Power Forward | $5.5M / 5 | 1992-06-09 → 31 | Canada |
| Mattias Ekholm | TWD → Two-Way Defenseman | $6.0M / 3 | 1990-05-24 → 33 | Sweden |
| Darnell Nurse | TWD → Two-Way Defenseman | $9.25M / 7 | 1995-02-04 → 28 | Canada |
| Evan Bouchard | OFD → Offensive Defenseman | $3.9M / 2 | 1999-10-20 → 23 | Canada |
| Ryan Nugent-Hopkins | TWF → Two-Way Forward | $5.125M / 6 | 1993-04-12 → 30 | Canada |
| Evander Kane | PWF → Power Forward | $5.125M / 3 | 1991-08-02 → 32 | Canada |
| Warren Foegele | PWF → Power Forward | $2.75M / 1 | 1996-04-01 → 27 | Canada |

- **McDavid's header:** 6'1", 193 lb, shoots left, all equal in game and in Studio. This
  confirms the weight base of 120 and the height encoding.
- **Ages:** all nine are consistent with a game date between 13 January and 20 October 2023.
  This confirms zero-indexed months and days.
- **Ratings:** Studio's derived overall runs 2–4 above the in-game OVR for all nine players,
  in nearly the same order (99/97, 98/95, 90/88, 91/87, 91/87, 90/87, 90/87, 87/84, 85/83).
  That fits the raw 0–99 rating scale with no offset. The gap is the overall formula's NHL 14/15
  calibration. Individual ratings were not compared, because no rating screen was captured.
- **Editable:** the test asserts that weight, height, year, month, day, contract dollars and at
  least one rating are editable.

**Evidence level:**
- Cap hit, years, styles, McDavid's height and weight, and ages are *game-verified* by the
  user's screenshots. The birth dates themselves are read on screen only as ages.
- The rating scale is *measured* (overall consistency) and not game-verified per rating.
- No encoded-field *write* has been checked in game yet.

## 2. X-Factors

| Source | Where per-player X-Factors live | Basis |
|---|---|---|
| NHL 27 PS5 schema (`nhl_game-meta.xml`, PPSA34063) | `exhibitionskaterai` (`dSvy`), `exhibitiongoalieai` (`mHuy`), `customskaterai` (`NXqo`), `customgoalieai` (`GezY`). Fields: `xfactor1ability` (`dimw`), `xfactor2ability` (`LFln`), `xfactor3ability` (`nTld`), range −1..254; `xfactor1tier` (`oxwo`), `xfactor2tier` (`XBaQ`), `xfactor3tier` (`iJzI`), range 0..5 | read from the install's own metadata |
| NHL27Kit 1.0.0 | Reads those three ability/tier pairs from the player's ratings table. Names come from localization `TXT_XFACTOR_<id>` and `TXT_XFACTOR_SHORT_DESC_<id>` for ids 0..31, with goalie abilities 26..30. Tier names come from `TXT_ABILITY_TIER_1..3` (Specialist, All-Star, Elite). Icons are at `ui/textures/common/icons/playerabilities/xfactors/<size>/xfactor<id>_<tier>` | format facts from a read-only decompile of `NHL27Kit.Core.XFactors` and `NHL27Kit.App.RosterPage` (user-pointed, 1 Nov 2026); no code ported |
| NHL24Kit / NHL 24 Roster Editor | Not handled: no X-Factor, superstar or ability strings in `NHL24Kit.dll` or `NHL24RosterEditor.dll` (ASCII and UTF-16) | string search |
| NHL 24 `nhl_game.db` | **None.** No field named `xfactor*` or `*abilit*`. The 32 skater `trait` slots (0..63) are AI traits and signature moves (below), not X-Factors. Cale Makar has no traits at all | metadata search; McDavid, Makar, Draisaitl, MacKinnon, Ovechkin and others dumped |
| NHL 24 collector files | Ability *definitions* only: `creation_zone_csv/xfactorabilitiesglobal.csv` (skater ids 1..49, goalie 200..224, `TXT_XFACTOR_<id>` keys), `woc_loadout_csv/xfactorabilities.csv` (superstar and zone variants), `playerclassxfactorabilities.csv` (World of Chel class unlocks), `service-defs/**/x_factors` schemas (up to 65 abilities). `customization_csv/xfactortopfifty.csv` is a header and placeholder only | extracted read-only |
| NHL 24 `league/resource_kernel/xfactorskernel.brk` | A binary rules kernel, not an assignment table: player ids are not found consistently | id search |
| NHL 24 assets in `globals`, `globalsfull`, `levels/game` and `ui` (base and 01.60) | Only X-Factor commentary, sound and UI art (272 assets); no data asset with per-player assignments | asset-name search |

**Not yet searched for NHL 24:** the `contentsb`, `contentlaunchsb` and `worldssb` asset names
(about 1.4M assets), the executable, and the `loc` databases (for the ability-name keys only).
EA's online roster updates may also carry the assignments; that is unverified.

**Consequences:**
- **NHL 27:** X-Factor editing can follow the database directly (three slots, ability −1..254,
  tier 0..5). Ability names come from the dictionary.
- **NHL 24:** nothing in Studio can capture X-Factors until the storage is found.

## 3. NHL 24 AI traits (side finding)

`creation_zone_csv/playertraits.csv` is NHL 24's own trait vocabulary:
- 1 Net Presence, 2 Net Agitator, 3 Setup Man Low, 4 Shooter Slot, 5 Rush Joining D
- 7 Shoots On The Rush, 8 Drives The Net, 9 Pinching D, 10 Finishes Hit
- 11 One Time Specialist, 12 Puck Protectors, 13 Offensive Opportunist, 14 Non Physical
- 15 Hip Checker, 16 Dangler, 17 Hard Backchecker, 18 Lazy Backchecker
- 19 Peripheral Player, 20 Risk Taker D Zone, 21 Puck Moving D, 22 Risk Taker O Zone
- 23 Speedster, 24 Risky Dangler, 25 Setup Man, 26 Perimeter Sniper, 27 Big Shooter
- 28 Responsible
- 30–37 signature moves: Between Legs Shot, Between Legs Pass, One Handed Deke, Datsyuk Toe
  Drag, Datsyuk Flip, Lacrosse Goal, Kucherov Deke, Between Legs One Timer.

Studio's `attributes::SKATER_TRAITS` (from NHL 14 community research) disagrees for NHL 24:
- **Different names:** 20 (Studio "Shooter Slot (Defense)"), 22, 24, 25 and 26.
- **Unlabelled:** 28 and 30–37.

**Fixed 1 Nov 2026.** `attributes::TraitVocabulary` chooses the dictionary from the project's
title (`TraitVocabulary::for_game`). `nhl24` uses `NHL24_SKATER_TRAITS`, which holds the game's
own names, title-cased, with the file's "LAZY BACKCHEKER" spelling corrected. Every other title
keeps the classic dictionary. Goalies keep the classic set on all titles, because NHL 24's table
covers skater types only.
- `view::player_view` takes the title from the enum registry's scope. Values missing from NHL
  24's table (6, 29) show as "Trait #n" with the note "Not in NHL 24's own trait table".
- **Unit test:** `attributes::tests::nhl24_uses_its_own_skater_trait_names`.
- **NHL 24 test:** `oilers_match_the_in_game_roster_screen` asserts McDavid's 19 traits all
  resolve to NHL 24 names (for example Setup Man Low, Speedster, Risky Dangler, Perimeter
  Sniper, Datsyuk Toe Drag, Between Legs One Timer), and that the trait menu offers
  Kucherov Deke.
- **Evidence level:** the names are the game's own data, but they have not been compared with
  an in-game trait screen.

## 4. Checks and cleanup

- **NHL 24 tests:** the two `nhl24_roster` ignored tests pass.
- **Other trait tests:** the studio-domain test targets that touch traits pass. `read_matrix`
  and `surface_matrix` fail on NHL 07 PS3 arenas, identically on a clean worktree of
  `01df28b2`, so they are baseline failures.
- **studio-domain failures:** `capabilities::nhl07_is_the_gen6_shape`,
  `capabilities::nhl10_gates_features_but_ratings_resolve_by_name` and
  `discovery::tests::ps3_detect_resolves_modding_minimum` fail on a clean worktree of
  `01df28b2` as well. They are baseline failures, not caused by this change.
- **Full comparison not run.** The full studio-domain before/after failure comparison was
  interrupted by the user and has not been rerun.
- **Scratch:** extracted NHL 24 X-Factor files, the NHL 27 metadata, the unpacked NHL27Kit
  libraries and their decompiled output are in the session scratchpad `xf/`. They are not
  committed and will be deleted with the session. The temporary probe tests were removed, as
  were the clean worktree and its build folder.
