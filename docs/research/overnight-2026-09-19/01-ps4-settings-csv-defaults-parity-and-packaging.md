# Handoff P1-BS-CSV — NHL 16–21 PS4 stored-default parity (`setting_csv`) and the CSV member packaging contract

Research lane, 19 September 2026, fresh session under
[p1bs-r1-acceptance-and-csv-parity-dispatch](../../validation/p1bs-r1-acceptance-and-csv-parity-dispatch.md)
(queue entry R1 of the [overnight research brief](../../plans/overnight-research-2026-09-19.md)). Claim
`research-p1bs-csv`. Consumer: developer Phase 1.2 (settings defaults; P1-BS-2 reader, P1-BS-4 CSV default edit),
P1-F (archive placement). Extends [handoff 22 rev. 3](../overnight-2026-09-18/22-settings-catalogue-defaults-and-cross-title-equivalents.md)
(NHL 15 PS4 DB contract, accepted for stored-default scope) to the six CSV titles; reuses handoffs
[16](../overnight-2026-09-18/16-league-logo-packaging-cross-platform.md) §3 (S-B chain) and
[20](../overnight-2026-09-18/20-dictionary-growth-invariants.md) §6 (PS4 placement) for packaging.

**Revision 2 (2026-09-19, after the [P1-BS-CSV review addendum](../../validation/p1bs-csv-review-and-overnight-addendum.md)).**
Correction notice for a consumed handoff: §12 lists every change. Measured data (§0–§2, §5.4 numbers, the two JSON
files) is unchanged; the contract text (§3, §4, §5.3, §7) is narrowed to the six binding clarifications: compression is
a reproduction target not a loader rule; NHL 18–21 global is an explicit **paired** edit; the first writer's parser
is a measured subset (the §3.4/§4 conflict is resolved); the stale "excluded cells" baseline sentence is removed;
the NHL 17 append card needs a same-title control; the nine-vector count is five changed / four unchanged.

Machine-readable (under [`data/`](data/)):
- [`p1bs-csv-sources.json`](data/p1bs-csv-sources.json) — **authority for §0–§1**: every `setting_csv/*.csv` member of
  every `databigs/*.big` of the six PS4 installs (archive identity, entry, offset, stored/plain sizes and SHA-256,
  chunkzip chunk plan, text characteristics, census comparison), eboot/`param.sfo` identities.
- [`p1bs-csv-defaults-contract.json`](data/p1bs-csv-defaults-contract.json) — **authority for §2–§6**: per title the
  schema variant, member map with slot slack, the 21 eligible legacy ids (present/absent, global cell(s), per-context
  rows with their domain list), choice lists, integrity counts, encoder-provenance counts and the developer vectors
  with exact line locators, before/after hashes and illustrative re-encoded sizes.

Probes (`local/runs/overnight-research-2026-09-19/probes/`): `p1bs_csv_sources.py`, `p1bs_csv_contract.py` (both
read-only; they import `eb_list.py` from the 2026-09-18 probe set rather than copying it). No application code,
defs, bindings or tests changed; no install, archive, image, anchor or save written; no launch; no game bytes
retained beyond identifiers, small integers, dictionary keys, single catalogue CSV lines and hashes.

**Evidence vocabulary:** **observed** = read from the named install member or the Studio source file; **inferred** =
a reading whose producing mechanism was not traced; **unknown** = not established. Everything about what the game
*does* with a stored default (which context a screen reads, precedence, where a chosen value is persisted) is
**unknown** here exactly as in handoff 22; nothing below narrows that.

## Plain-language summary

The settings catalogue that NHL 15 keeps in its database is, on NHL 16–21 (PS4), a set of plain comma-separated text
files inside one archive (`inxp_00.big`). This handoff confirms that the earlier census read the right files — the
real NHL 20 install (not the NHL 19 anchor) and, for NHL 19, all twelve patch archives, none of which carries a copy
— so each title has exactly one stored copy in the inspected installs. It then maps, for each title, the same
league/competition defaults that the accepted NHL 15 operation edits: where each default sits (one cell on NHL 16, the
`_DEFAULT_` row on NHL 17, a pair of equal cells on NHL 18–21, plus one row per screen), which values are allowed
(taken from that title's own files, which differ from NHL 15 — e.g. the tie-break list is longer and its numbering
shifts, the cup-type list has two entries instead of four, and five of the 21 NHL 15 rows do not exist from NHL 17 on),
and which requests must be refused. Editing one cell means rewriting the text file (only that cell's characters
change) and re-compressing it; on five of the nine illustrative edits the compressed file changed size (by −1 to
+16 bytes) although the text length did not, so it may or may not fit its old slot. Studio already has the archive
planner/stager that handles both cases on a copied target, but its compressor neither frames the file the way EA's
files are framed nor reproduces EA's bytes (standard zlib level 9 in 128 KiB chunks does, on every stock file
measured — a reproduction target, not proof that the game rejects other valid streams), and its API route only
accepts database members. Those two gaps are the smallest implementation dependencies; whether the game then shows or
uses the edited default remains a user test. On NHL 18–21 the global default is stored in two places that are equal in
stock; Studio should offer that as one explicit paired edit of two cells, not as a one-cell edit.

## 0. Sources (observed; full hashes in `p1bs-csv-sources.json`)

| Title | Install (`H:\Emulators\ShadPS4\Games\…`) | `eboot.bin` SHA-256 (size) | `param.sfo` version strings | `databigs/inxp_00.big` | Members |
|---|---|---|---|---|---|
| NHL 16 | `CUSA01909` | `e4845ac1…` (58 936 564) | 01.00 | EB `0x00110400`, 234 entries, 299 883 104 B | 9 |
| NHL 17 | `CUSA04326` | `6aa1ff7b…` (73 395 278) | 01.00 | 1 185 entries, 273 371 840 B | 12 |
| NHL 18 | `CUSA07569` | `59e33089…` (80 134 904) | 01.00 | 1 354 entries, 390 197 792 B | 11 |
| NHL 19 | `CUSA11126` | `426297a6…` (100 369 766) | 01.00 / 01.55 / 01.60 | 2 329 entries, 445 758 880 B | 11 |
| NHL 20 | **`CUSA15439`** (the real NHL 20 install; the repository `anchors/ps4/nhl20` are NHL 19 base members) | `82ea5a63…` (98 710 655) | 01.00 | 14 567 entries, 517 362 304 B | 11 |
| NHL 21 | `CUSA18182` | `fc95ca66…` (104 845 239) | 01.00 / 01.01 | 2 950 entries, 777 373 328 B | 11 |

All six `inxp_00.big` are EB `0x00110400` (20-byte rows) with `archive_size == file length`. Every eboot identity
equals the handoff 17 witness. Members: `nhl/data/common/gamemodes/setting_csv/{setting,context,contextsetting,contextgroup,group,choice}.csv`
on every title; NHL 16 adds `settinggroup`, `stocksettingdependencies`, `renderingintro`; NHL 17–21 replace `settinggroup`
with `groupsetting` and add `choicelist`, `range`, `settingservice`, `renderingintro`; NHL 17 alone also ships
`stocksettingdependencies` (identical bytes to NHL 16's, `e5ac8abf…`).

## 1. Source gaps closed (dispatch item 1)

- **Every census hash reproduces.** All 65 members (9 + 12 + 11 × 4) decode to the plain-text SHA-256 recorded in
  `p1bs-ps4-setting-csv-catalogues.json` (`census: "same plain hash"` on every row; no census path missing).
- **NHL 20 provenance.** The census members come from `CUSA15439/databigs/inxp_00.big` (entries 2274 … 13251 as
  recorded); this is the real NHL 20 install, not the NHL 19-based anchor. Its `param.sfo` carries only 01.00 and the
  install has no `patch_*` archive.
- **NHL 19 patch archives.** `patch_inxp_51.big` … `patch_inxp_56.big` (2 001 / 735 / 1 383 / 685 / 592 / 567 entries)
  and `patch_game_91.big` … `patch_game_96.big` were opened and their full path tables scanned: **no `setting_csv`
  member in any of them**. The NHL 19 CSV catalogue therefore has one stored copy (`inxp_00.big`) in this install
  (01.60). This is an observation on the twelve inspected patch archives, not a statement about other patch levels.
- **Duplicates.** Scanning every `*.big` of every install (78 archives, name tables only): no `setting_csv` path
  occurs outside `inxp_00.big`, no path occurs twice, and no two members share a payload span
  (`path_copies` empty on every title). Presence of a patch copy — had one existed — would not have established
  precedence; none exists to reason about. **Scope of this uniqueness statement:** the 78 inspected `databigs/*.big`
  archives of the six installs. It does not establish the absence of every loose, patch-level or runtime source the
  game could read (none was found in the inspected trees; other patch levels were not available).
- **Text characteristics (observed on all 65 members).** No BOM, no byte ≥ 0x80, no NUL, no double-quote byte, no
  tab, comma delimiter, **CRLF on every line including the last**, no blank line, every data row has exactly the
  header's column count (no ragged rows).
- **Wrapper.** 53 members are `chunkzip` (padded header: `"chunkzip"`, u32 2, `fullsize`, window **0x20000**,
  `n_chunks`, u32 0x10, 12 zero bytes = 40 B; each chunk `[zsize u32][flag u32 = 1]` placed at the next
  `off % 16 == 8` so chunk data is 16-aligned; **no trailing bytes after the last chunk**: stored size == consumed).
  `n_chunks = ceil(fullsize / 0x20000)` — 2 chunks only for `contextsetting.csv` on NHL 18–21 (166–206 KB). The twelve
  small members (`settingservice` 68 B, `range` 382–519 B, `stocksettingdependencies` 292 B) are stored **raw**
  (the size threshold EA applied is not established: raw ≤ 519 B, compressed ≥ 2 319 B plain).
- **Slot slack.** Members are 16-byte aligned in the archive, so the zero padding after a member is 0–15 bytes
  (per member in the JSON: e.g. `contextsetting.csv` 12 / 1 / 3 / 3 / 2 / 7 B on NHL 16–21; `setting.csv` 10 / 8 /
  0 / 8 / 12 / 13 B).

## 2. Variants and the per-title map of the accepted NHL 15 rows (dispatch item 2)

### 2.1 Three schema variants (observed)

| Variant | `setting.csv` columns | `contextsetting.csv` columns | Global default | Choice labels |
|---|---|---|---|---|
| **NHL 16** | `id,key,name,string_id,description_id,is_choice,min_value,max_value,default_value,remap_id` (267 rows) | `context_id,setting_id,min_value,max_value,default_value,choices,is_enabled` (570) | `setting.default_value` (one cell) | `choice.csv` is a placeholder (`CHOICE_n` / `TXT_CHOICE_n`, 613 rows) — labels only from the NHL 16 eboot arrays (= NHL 15 arrays, order **inferred**, handoff 22 §1.7) |
| **NHL 17** | `id,name,legacy_id,is_global,is_choice,choice_to_range,name_loc,description_loc,extra_loc` (351) — **no default column** | `context_id,setting_id,is_enabled,default_value,potential_values` (8 306) | the `contextsetting` row with `context_id = -2` (`_DEFAULT_`) | explicit: `choice.csv` `(choicelist_id, id, name, value, name_loc)` |
| **NHL 18–21** | NHL 17's + `default_value,potential_values,usage` (18: 376; 19: 389; 20: 409 + `audio_id`… 13 cols; 21: 427) | NHL 17's + `is_visible,usage` (8 944 / 10 232 / 10 703 / 10 969) | **two cells**: `setting.default_value` **and** `contextsetting[-2].default_value`; equal on all 16 eligible rows per title in stock (`global.consistent` in the JSON; the broader ad-hoc check covered the 32 handoff-22 candidate ids present on each title — the 16 eligible plus the 16 excluded-but-present — and found 32/32 equal). Stock equality is an **observation**, not an engine requirement | explicit, as NHL 17 |

`setting.id` on every variant = the NHL 15 `legacyid` for the Phase 1 rows (handoff 22 §3). NHL 17's `legacy_id`
column = the NHL 15 setting `id` (e.g. 153 on rows 72/86/116) — it is the *setting identity*, not the row key.

### 2.2 What a stored `default_value` is (observed)

- On NHL 17–21 `potential_values` names a **choice list** (`choicelist.csv` id) for every eligible row (`is_choice 1`,
  `choice_to_range 0`; no eligible row is range-based). The stored default is a **choice `value`**, not a list index:
  every eligible default (128 / 137 / 171 / 177 / 177 rows on 17–21) is a `value` of its row's list, and the witness
  that separates the two readings is `SHOOTOUT_NUMBER_OF_ROUNDS` (id 240, list 1: ids 0/1, values 3/5, default `3`
  — valid only as a value). On the ten lists the eligible rows use, `id == value` on every choice row, so the
  distinction cannot bite there.
- **The per-context row's own `potential_values` governs its domain**, not the setting row's: `88
  TOURNAMENT_ROUND_ROBIN_TIE_BREAK` carries list 10 (4 values) on its `_DEFAULT_` row but list **12** (8 values,
  default `2`) on the tournament contexts (80/81 on NHL 17–18; 81/131 on 19–21). Every other eligible row uses one
  list everywhere.
- On NHL 16 the domain is `setting.min_value..max_value` (all eligible rows `min_value 0`, `is_choice 1`); the
  eligible `contextsetting` rows carry **empty** `min_value/max_value/choices` (no per-context override) and
  `is_enabled 1`. 88 of NHL 16's 570 `contextsetting` rows have an empty `default_value`; none of them is an
  eligible row.
- `is_enabled` / `is_visible` / `usage` semantics are **unknown**; every eligible per-context row in stock has
  `is_enabled 1` and (17–21) `is_visible 1`, `usage` ∈ {2, 3}.

### 2.3 NHL 16 (21 of 21 ids present; keys = handoff 22 §2.1 roles)

| id | key | global (`setting.csv` `default_value`) | bounds (`min_value..max_value`) | per-context rows (`contextsetting.csv` context → default) |
|---|---|---|---|---|
| 67 | `DYNASTY_STARTING_ROSTERS` | 0 | 0..1 | — |
| 68 | `DYNASTY_SEASON_TIE_BREAK` | 2 | 0..5 | 3 DYNASTY_FLOW → 2, 5 MUD_CREATION_FLOW → 2, 30 BE_A_PRO_FLOW → 2, 37 SEASON_MODE_FLOW → 2 |
| 69 | `DYNASTY_ALLSTARGAME` | 1 | 0..2 | — |
| 70 | `DYNASTY_SALARYCAP` | 0 | 0..1 | 3 → 1, 5 → 1 |
| 71 | `DYNASTY_SEASON_OVERTIME_LOSS` | 1 | 0..1 | 3 → 1, 5 → 1, 30 → 1, 37 → 1 |
| 72 | `DYNASTY_PLAYOFFS_LENGTH` | 3 | 0..4 | 3 → **4**, 5 → 3, 30 → **4**, 37 → **4** |
| 73 | `DYNASTY_PLAYOFFS_TIE_BREAK` | 0 | 0..2 | 3 → 0, 5 → 0, 9 TOURNAMENT_EA_FLOW → 0, 30 → 0, 37 → 0 |
| 77 | `DYNASTY_LENGTH` | 0 | 0..4 | 3 → 0 |
| 83 | `TOURNAMENT_NUMBER_ROUND_ROBINS` | 4 | 0..6 | — |
| 85 | `TOURNAMENT_STARTING_ROSTERS` | 0 | **0..1** | 37 → 0 |
| 86 | `TOURNAMENT_ELIMINATION_SERIES_LENGTH` | 2 | 0..4 | 9 → 3 |
| 87 | `TOURNAMENT_ELIMINATION_TIE_BREAK` | 1 | 0..2 | — |
| 88 | `TOURNAMENT_ROUND_ROBIN_TIE_BREAK` | 4 | 0..5 | — |
| 89 | `TOURNAMENT_ROUND_ROBIN_OVERTIME_LOSS` | 0 | 0..1 | — |
| 116 | `GAME_MODE_PLAYOFF_SERIES_LENGTH` | 3 | 0..4 | — |
| 117 | `GAME_MODE_CUP_TYPE` | 0 | 0..3 | — |
| 118 | `GAME_MODE_PRELIMINARY_SERIES_LENGTH` | 3 | 0..4 | — |
| 119 | `GAME_MODE_SEEDING_TYPE` | 0 | 0..4 | — |
| 120 | `GAME_MODE_HOME_AND_AWAY_SCHEDULE` | 0 | 0..2 | — |
| 123 | `GAME_MODES_SEASON_START_PERIOD` | 0 | 0..1 | — |
| 212 | `MUD_STARTING_ROSTERS` | 0 | 0..1 | 5 → 0 |

Differences from NHL 15 that must not be transplanted: `StartingRosters` 67/85 are **0..1** (NHL 15: 0..2 —
`defaultdraft` is gone); NHL 16 has rows for the Season context 37 (NHL 15 had none) and its Franchise/BAP/Season
`PlayoffSeriesLength` default is 4 (`Authentic`), not 3. Labels on NHL 16 are inferred (eboot arrays), shown as
"inferred" pills next to the raw code exactly as the NHL 15 contract requires.

### 2.4 NHL 17–21 (16 of 21 ids present; 83, 89, 118, 119, 123 absent from `setting.csv` on all five)

| id | key (17–21) | list (`potential_values`) | global default 17 / 18 / 19 / 20 / 21 | contexts with a row on NHL 21 (`*` = per-context list differs; 17–20 subsets in the JSON) |
|---|---|---|---|---|
| 67 | `DYNASTY_STARTING_ROSTERS` | 13 `DYNASTY_STARTING_ROSTERS` | 0 / 0 / 0 / 0 / 0 | 37, 48, 66, 83, 84, 126 |
| 68 | `DYNASTY_SEASON_TIE_BREAK` | 12 `DYNASTY_SEASON_TIE_BREAK` | **7** / 7 / 7 / 7 / 7 | 3, 30, 37, 48, 66, 70, 72, 73, 83, 84, 107, 108, 126, 130, 133, 134, 146 |
| 69 | `DYNASTY_ALLSTARGAME` | 65 `ALLSTAR_TYPE` | 2 / 2 / 2 / 2 / 2 | 37, 48, 66, 83, 84, 126 |
| 70 | `DYNASTY_SALARYCAP` | 34 `RULE_TOGGLE` | 1 / 1 / 1 / 1 / 1 | 3, 4, 37, 48, 66, 70, 73, 83, 84, 107, 108, 126, 130, 133, 146 |
| 71 | `DYNASTY_SEASON_OVERTIME_LOSS` | 11 `DYNASTY_SEASON_OVERTIME_LOSS` | 1 / 1 / 1 / 1 / 1 | 3, 30, 37, 48, 66, 70, 72, 73, 83, 84, 107, 108, 126, 130, 133, 134, 146 |
| 72 | `DYNASTY_PLAYOFFS_LENGTH` | 9 `DYNASTY_PLAYOFFS_LENGTH` | 4 / 4 / 4 / 4 / 4 | 3, 30, 37, 48, 66, 70, 72, 73, 83, 84, 107, 108, 126, 130, 133, 134, 146 |
| 73 | `DYNASTY_PLAYOFFS_TIE_BREAK` | 10 `DYNASTY_PLAYOFFS_TIE_BREAK` | 0 / 0 / 0 / 0 / 0 | 3, 30, 34, 37, 48, 66, 70, 71, 72, 73, 83, 84, 107, 108, 126, 130, 132, 133, 134, 146 |
| 77 | `DYNASTY_LENGTH` | 8 `DYNASTY_LENGTH` | 0 / 0 / 0 / 0 / 0 | 3, 37, 48, 66, 70, 83, 84, 107, 108, 126, 130, 146 |
| 85 | `PLAYOFF_STARTING_ROSTERS` (renamed) | 13 | 0 / 0 / 0 / 0 / 0 | 48, 66, 73, 83, 84, 126, 133 |
| 86 | `PLAYOFF_ELIMINATION_SERIES_LENGTH` (renamed) | 53 `PLAYOFF_SERIES_LENGTH` | 3 / 3 / 3 / 3 / 3 | 34, 48, 66, 71, 83, 84, 126, 132 |
| 87 | `PLAYOFF_ELIMINATION_TIE_BREAK` (renamed) | 10 | 0 / 0 / 0 / 0 / 0 | 48, 66, 83, 84, 126 |
| 88 | `TOURNAMENT_ROUND_ROBIN_TIE_BREAK` | 10 (global) / **12** on tournament contexts | 0 / 0 / 0 / 0 / 0 (tournament rows: 2) | 48, 66, 81\*, 83, 84, 126, 131\* (17–18: 80\*, 81\*) |
| 116 | `GAME_MODE_PLAYOFF_SERIES_LENGTH` | 9 | 3 / 3 / 3 / 3 / 3 | 48, 66, 80, 83, 84, 126 |
| 117 | `GAME_MODE_CUP_TYPE` | 57 `GAME_MODE_CUP_TYPE` (**2 entries**) | 0 / 0 / 0 / 0 / 0 | 37, 48, 66, 83, 84, 126 |
| 120 | `GAME_MODE_HOME_AND_AWAY_SCHEDULE` | 34 `RULE_TOGGLE` (**2 entries**) | **1** / 1 / 1 / 1 / 1 | 48, 66, 80, 83, 84, 126 |
| 212 | `MUD_STARTING_ROSTERS` | 13 | 0 / 0 / 0 / 0 / 0 | 37, 48, 66, 83, 84, 126 |

Choice lists (identical text on NHL 17, 18, 19, 20 and 21; `id == value` on every row of all ten):

| list | values → `name_loc` (explicit) |
|---|---|
| 8 `DYNASTY_LENGTH` | 0 `25` · 1 `5` · 2 `10` · 3 `15` · 4 `20` |
| 9 `DYNASTY_PLAYOFFS_LENGTH` / 53 `PLAYOFF_SERIES_LENGTH` | 0 `SingleKnockout` · 1 `BestOf3` · 2 `BestOf5` · 3 `BestOf7` · 4 `Authentic` |
| 10 `DYNASTY_PLAYOFFS_TIE_BREAK` | 0 `ContinuousOT` · 1 `Shootout` · 2 `OT3ThenShootout` · 3 `5MinThenShootout` |
| 11 `DYNASTY_SEASON_OVERTIME_LOSS` | 0 `0Points` · 1 `1Point` |
| 12 `DYNASTY_SEASON_TIE_BREAK` | 0 `ContinuousOT` · 1 `Shootout` · 2 `OT3ThenShootout` · 3 `5MinThenShootout` · 4 `SingleOt4on4` · 5 `SingleOt5on5` · 6 `NoOvertime` · 7 `authentic` |
| 13 `DYNASTY_STARTING_ROSTERS` | 0 `current` · 1 `default` |
| 34 `RULE_TOGGLE` | 0 `OFF` · 1 `ON` |
| 57 `GAME_MODE_CUP_TYPE` | 0 `TXT_BATTLE_FOR_THE_CUP_NOW` · 1 `TXT_HEAD_TO_HEAD_MODE` |
| 65 `ALLSTAR_TYPE` | 0 `none` · 1 `WorldVsNA` · 2 `EastVsWest` |

Differences from NHL 15/16 that must not be transplanted: the tie-break vocabulary (`OT3ThenShootout` inserted at 2,
`authentic` 7 — NHL 15 code 2 `5MinThenShootout` is code 3 here; list 10 has four entries, NHL 15's three);
`StartingRosters` 0..1; `GAME_MODE_CUP_TYPE` has two entries with different meanings from NHL 15's four trophies;
`HomeAwayType` 120 is a two-value toggle (NHL 15: three schedule patterns); 88's tournament rows use list 12 with
default 2, while its global row uses list 10 with default 0. Context ids 3/30/34/37 keep their NHL 15/16 lineage
names (`DISPLAY_FRANCHISE_CREATION_CLASIC`, `DISPLAY_BE_A_PRO_CREATION`, `DISPLAY_PLAYOFF_CREATION`,
`DISPLAY_SEASON_CREATION` on NHL 21); the `DEFAULT_*_VALUES` contexts (66, 69–73, 81, 104, 111–113, 116, 117, 123,
140) are additional stored rows whose relation to the `DISPLAY_*` rows is **unknown** — the review lists all rows of a
setting so the user sees both.

## 3. Per-title stored-default contract (NHL 16–21) — proposal, not authorisation

Mirrors handoff 22 §2 (accepted for NHL 15) with the CSV-specific rules below. Runtime effect unknown throughout.

### 3.1 Eligible target

A target is *(title, legacy id, scope)* where scope is `global` or an existing `contextsetting` row `(context_id,
setting_id)`. Eligible ids: the 21 of handoff 22 §2.1 **minus the ids absent from that title's `setting.csv`**
(NHL 17–21: 83, 89, 118, 119, 123 → refused "not present in this title's catalogue"; the handoff 22 exclusions
— 74, 81, 82, 122, 125, 115/121/179/222, 15, 84, 153, 190, 203, 213, 217, `NULL` rows — stay excluded here for the
same reasons, plus 82/122 `NumberOfTeams` because their list changed across titles and were not re-examined).

The **global** scope resolves to: NHL 16 `setting.csv` row `id == legacy id`, column `default_value`; NHL 17
`contextsetting.csv` row `(-2, legacy id)`, column `default_value`; NHL 18–21 an explicit **paired stored-default
edit**: `setting.csv` `default_value` **and** `contextsetting.csv` `(-2, legacy id)` `default_value` are two cells in
two members, shown as such in the review (both members, both cells, both old values), bound to one project-bound
review ticket, one undo operation and one atomic apply/restore (two `Replacement`s in one `stage::plan`; a failure
of either leaves both unchanged). It is never described as a one-cell operation. Preconditions: the two cells hold
the **same current value** and their domains are compatible (`setting.potential_values ==
contextsetting[-2].potential_values`, both resolving to the same choice list). An unequal pair (never in stock)
**refuses the paired operation** with both values shown; a separate repair/editor route for such a pair is *not*
authorised by this handoff. The reserved `_DEFAULT_` context (−2) is never presented as an ordinary screen context in
the per-context editor, so the paired rule cannot be bypassed by editing `(−2, id)` alone; NHL 17, which has only the
`_DEFAULT_` cell, is the single-cell global case.

### 3.2 Domain and encoding

The written text is the decimal code with no sign, no leading zeros and no surrounding whitespace, exactly as the
stock cells are written. Allowed values:

- NHL 16: integers `min_value ≤ code ≤ max_value` of the setting row (`is_choice 1`); if the target `contextsetting`
  row carries a non-empty `min_value`/`max_value`/`choices` override (none does for eligible rows in stock), the row is
  refused as "per-context override not interpreted" rather than guessed.
- NHL 17–21: `code ∈ { choice.value | choice.choicelist_id == row.potential_values }` for **the target row's own**
  `potential_values` (global: the `_DEFAULT_` row's list; NHL 18–21 paired edit: `setting.potential_values` must equal it, else the
  pair is refused as incompatible). A list whose rows have `id ≠ value` is not used by any eligible row; if one is encountered
  the target is refused ("list numbering ambiguous") rather than resolved by assumption.
- A code equal to the current cell is a no-op review.

Labels: NHL 16 — eboot array order, shown as *inferred*; NHL 17–21 — `choice.name_loc`, shown as *explicit* (the
dictionary key; the resolved English text comes from `eng_us.db` when available, else the key).

### 3.3 Duplicates, missing rows, nulls, disabled rows

- A target `(context_id, setting_id)` with **no** row is refused: "no stored default for this setting on this screen;
  adding one is a row-growth operation that is not offered" — with no statement about which default the game would use.
- More than one row with the same key (never in stock: 0 duplicates in every `contextsetting.csv` and `setting.csv`
  checked on the real members) → refused as ambiguous.
- An **empty** `default_value` cell (NHL 16 has 88 such rows, none eligible) → refused as "null stored default; writing
  a value where EA stores none is not offered".
- A row with `is_enabled 0` or `is_visible 0` (none eligible in stock) → refused with the flag shown; semantics unknown.
- A setting row whose `is_choice` ≠ 1 or `choice_to_range` ≠ 0 (none eligible) → refused.

### 3.4 Eligibility of the opened file; parser subset; second edit; reopen

**Parser contract for the first writer (resolves the earlier §3.4 strict rule vs §4 tolerant rule).** The writer
operates only on a clearly defined **measured subset**, which every stock member satisfies (§1 text characteristics):

- supported encoding: bytes < 0x80 only, **no BOM**;
- record boundaries: every record terminated by CRLF (the last one included), no LF-only or CR-only lines, no blank
  lines;
- header: first record; the expected column names for the title's variant present exactly once each — a **duplicate
  required header name** is refused; extra/unknown columns are allowed and preserved by position;
- target records: unquoted, comma-separated, exactly the header's column count; any `"` byte anywhere in the member,
  any byte ≥ 0x80, any NUL, any ragged record → the member is **browse-only / refused with the reason**, never
  re-serialised lossily.

BOM, alternate line endings, non-ASCII data and quoted or multiline records are outside the subset. They may be
admitted later only together with explicit parser and span tests; no naive line splitting that could land inside a
quoted multiline field is acceptable. §4's "preserve verbatim" rows describe how bytes outside the target cell are
treated *within* the subset (copied, never re-emitted); they do not widen the subset.

**Eligibility** is evaluated on the **opened member text**, not on "stock": (a) the member is within the subset;
(b) the target row exists exactly once (an ambiguous row binding or an ambiguous choice binding is a refusal);
(c) the current cell is a valid code of the row's domain. Unrelated anomalies inside the subset are preserved and
reported once (e.g. NHL 16's 88 empty defaults).

**Baseline and second edit.** A member that already carries reviewed Studio edits stays eligible under (a)–(c).
Validation is **not** narrowed by earlier reviews: the current target row and its domain are re-validated on every
edit exactly as on a stock member, and previously edited cells are ordinary cells. Two edits to the same cell in one
review collapse to the last value.

**Reopen.** A saved copied output reopens like any archive: parse → decode the member → (a)–(c). Re-opening does not
require, retain or restore the previous in-memory review ticket. Comparing the decoded plain-text SHA-256 with the
last review's recorded value is an **output-verification test** (T-STAGE / the transaction's post-apply check), not a
prerequisite for opening a file. A member now stored in a different wrapper form than the source entry
(raw ↔ chunkzip) is reported, not silently accepted.

### 3.5 Review description (human-readable; ids behind disclosure)

"NHL 18 PS4 — stored catalogue default of *Playoff Series Length* for the Franchise creation screen
(`DISPLAY_FRANCHISE_CREATION_CLASIC`, context 3) changed from code 4 (*Authentic*) to code 3 (*Best of 7*). The global
default (4) and the 15 other stored rows of this setting are unchanged. Whether this stored default is what the game
offers or applies has not been established. The text file `contextsetting.csv` is rewritten (one cell) and
re-compressed; its compressed size changes from 25 805 to N bytes, which [fits its slot / is appended at the end of
the archive]." Undo: cell-level inverse; on the archive side the original member bytes are restored by the
transaction's recorded prior identity (§5.6). For the NHL 18–21 paired global edit the description names both members
and both cells ("… the global default stored in `setting.csv` row 72 **and** in `contextsetting.csv` row `_DEFAULT_`/72,
both from code 4 to code 3; two text files are rewritten and re-compressed …") and the undo is one operation.

## 4. One-field CSV preservation contract (dispatch item 3)

Defined on the **plain text** (after chunkzip decode, before re-encode) of a member **inside the §3.4 subset**. The
serializer is a byte splice, not a CSV re-emit: it never re-parses and re-writes rows it does not touch. Rows below
that say "preserved verbatim" for a BOM / byte ≥ 0x80 / non-CRLF terminator describe the splice's behaviour on bytes
outside the target cell; a member exhibiting them is nevertheless outside the first writer's subset and is refused
before any splice (rev. 2).

| Property | Rule | Observed stock (all 65 members) |
|---|---|---|
| Encoding / BOM | bytes are copied; no BOM is added or removed. Rev. 2: a BOM or any byte ≥ 0x80 puts the member outside the first writer's subset (§3.4) → refused | no BOM, pure ASCII |
| Newline style | the target line keeps its own terminator; the trailing terminator of the file is preserved. Rev. 2: the subset requires CRLF on every record incl. the last; anything else → refused | CRLF everywhere incl. last line |
| Delimiter | comma; the target line is split on commas only. Rev. 2: any `"` byte anywhere in the member (not only on the target line) → member refused (quoting semantics not interpreted; no naive splitting inside a quoted field) | no `"` in any member |
| Quoting | the written code is never quoted; a currently quoted target cell is refused (none exist) | none |
| Other fields of the target row | byte-identical; the splice replaces only `[cell_start, cell_end)` of the target cell | — |
| Other rows, header | byte-identical (whole-file compare outside the spliced range) | — |
| Unknown / extra columns | the header is matched by **name**, not position; extra columns (e.g. NHL 20/21 `audio_id`, the two trailing `-1,-1` cells of NHL 20/21 `setting.csv` rows) are carried unchanged; a header lacking `default_value`, `context_id`/`setting_id` (or `id` on `setting.csv`) refuses the member; a header naming a required column **twice** refuses the member | NHL 20/21 `setting.csv` has 13 columns vs 11 on NHL 18/19; no duplicate header names |
| Column count | the target row must have exactly the header's column count before and after the splice (a code never contains a comma) | uniform on every member |
| Result | `new_plain = plain[:cell_start] + code + plain[cell_end:]`; `len(new_plain) − len(plain) = len(code) − (cell_end − cell_start)` (0 for every vector below — a one-digit code replacing a one-digit code) | — |

**Verification of the plain text:** (1) whole-buffer equality outside `[cell_start, cell_end)`; (2) re-parse both
buffers with the same tolerant reader (`csv` on `utf-8-sig`, `newline=""`) and assert row-count equality, header
equality and cell-wise equality except the one `(row, column)`; (3) the recorded before/after SHA-256 (the JSON
carries them for every vector).

## 5. Packaging contract — chain, primitives, gaps (dispatch item 3, read-only inspection of Studio)

### 5.1 The measured chain (S-B of handoff 16 with the CSV specifics)

`databigs/inxp_00.big` (EB `0x00110400`, 20-byte rows `[block][word_4 = 0][size][word_12][word_16 = djb2(path)]`,
`archive_size == length`) → member at `block << 4` → **chunkzip** (§1) → CSV text. Three small members are raw.
No BIGF/RefPack layer, no disc image, no loose copy on PS4 (handoff 20 §6). Two DB-style copies do not exist here
(§1): one stored copy per title.

### 5.2 What Studio already has (observed in source; nothing here was executed)

| Primitive | Location | What it does | Fit for this contract |
|---|---|---|---|
| EB reader (front tables only, streaming) | `crates/tdb-archive-big/src/eb.rs` `EbArchive::parse`, `decode_payload` | lists entries, decodes a member (`chunkzip` → plain; `chunklzx` refused) | read path complete |
| chunkzip decoder | `crates/tdb-archive-codec/src/lib.rs` `decompress_chunkzip` (padded variant, `((off−8+15)&~15)+8` chunk rule, arbitrary `zsize`) | decodes every observed member (census used the same rule) | complete |
| chunkzip encoder | `compress_chunkzip` | **single chunk**, header word `chunk_size = fullsize` (not 0x20000), deflate via `flate2` **miniz_oxide** backend (`Cargo.lock`), `Compression::best()` | **not native-framed** (the stager's own `chunkzip_framing_note` flags it), and **not byte-faithful** (§5.4) |
| Replacement planner / stager / verifier | `crates/tdb-archive-big/src/stage.rs` `BoundSource::bind` → `plan` → `stage_file` → `verify` | binds the source identity and every entry's span/digest; each `Replacement {entry_index, expected_path, expected_stored_sha256, new_stored, expected_decoded}` is placed **no-op** / **fit-in-slot** (new stored ≤ slot end = next 16-byte line or next entry) / **append-and-retarget** (payload appended at the 16-aligned staged EOF, row `block`/`size` and header `archive_size` patched, old bytes left in place); `new_stored` may be `chunkzip` (only `chunklzx` is refused) and is decoded and compared with `expected_decoded` at plan time and again at verify; every untouched byte compared with the source; the 20-byte-row `word_12` (path-derived, formula unknown) is preserved verbatim and carried as an **unresolved qualification item**, as is "native loader acceptance" for every non-no-op replacement → a staged output is never `deployable()` by this module | the archive side of the contract is **already implemented**; qualification stays open by design |
| In-memory EB editor | `eb.rs` `EbFile::{replace_payload_in_place, replace_payload}` (whole archive in memory, full reflow) | alternative used by tests | not needed; `stage` is the route for 270–780 MB archives |
| Transaction / copied target | `crates/studio-domain/src/txn.rs` `stage_archive` / `archive_artifact` (re-runs `verify`), `tdb_txn::{prepare, apply}` with journal and restore; API `Registry::copied_target_review_archive_members` in `crates/studio-api/src/registry/copied_target.rs` | one change set replaces archive members on a **copied** target root with recorded prior identities and rollback | restore and copied-output identity are covered; the API route is the gap (§5.3) |
| Same-length disc-image route (`iso_patch`) | `studio-domain/src/iso_patch.rs` | not applicable on PS4 | — |

### 5.3 What is missing (smallest implementation dependencies; reuse P1-F — do not build a parallel packer)

- **D1 — native-framed chunkzip encoder with decoded-content preservation.** `compress_chunkzip_native(plain)`:
  40-byte header with window word `0x20000`, `n_chunks = ceil(len/0x20000)`, each chunk header at the next
  `off % 16 == 8` with zero padding, `flag 1`, raw deflate per chunk. **Requirements:** native framing as measured,
  and an independent decode of the output equal to the input plain text. **Reproduction target (not a loader
  rule):** zlib 1.3.1 level 9 (default strategy, window −15, via CPython 3.12's `zlib`) reproduces all 53 stock members
  byte-for-byte (§5.4). That is evidence that EA's members are consistent with this implementation; it does **not**
  show that the loader rejects other valid deflate streams, nor which implementation EA used. Prefer a backend that
  reproduces the evidence when practical (record backend and version in the encoder's provenance note and compare
  against the real vectors); Studio's current `flate2`/`miniz_oxide` output is not native-framed and reproduces 1/26
  measured chunks. **Unchanged members keep their original stored bytes** (no re-encode of a member the plan does
  not edit). Edited output keeps the stager's existing qualification disclosures ("native loader acceptance",
  `word_12`) and the "encoder provenance" note; a warning is narrowed only by an actual test (e.g. 53/53 stock
  reproduction narrows the provenance note to "edited content re-encoded with a backend that reproduces every
  measured stock member"), never by the choice of backend alone. Raw members stay raw (never wrapped); a chunkzip
  member is never demoted to raw. This is the same dependency P1-F needs for PS4 GNF members; one encoder serves both.
- **D2 — API route for a text member.** `copied_target_review_archive_members` accepts only the `primary`/`dictionary`
  artifact roles, requires a bare TDB database, and refuses any member whose `stored_form != Raw`. A CSV edit needs a
  `csv`/`text` artifact role that carries `(member path, new plain bytes, expected plain SHA-256, wrapper form of the
  source entry)` and lets the route build `Replacement { new_stored: compress_chunkzip_native(plain) | plain,
  expected_decoded: ExpectedDecoded::of(plain) }` for a `Chunkzip` (or `Raw`) entry. No change to `stage.rs` is needed.
- **D3 — the one-cell CSV editor** (§3–§4): parse-by-header-name locator, byte splice, plain-text verification, review
  text; NHL 18–21 global = two members in one plan (`stage::plan` already accepts several replacements per archive).
- Optional **D4** — a reader (P1-BS-2) that presents §2 from the live members; the contract JSON is its fixture.

### 5.4 Changed compressed length (observed, illustrative)

| Vector | Plain length | Stored (EA) | zlib-9 native re-encode of the **unedited** text | … of the **edited** text | Slot slack | Placement (stage rule) |
|---|---|---|---|---|---|---|
| NHL 16 V1 `(3,72)` 4→3 | 8 047 → 8 047 | 1 556 | 1 556 (= EA) | 1 556 | 12 | fit-in-slot |
| NHL 16 V3 `(3,71)` 1→0 | 8 047 | 1 556 | 1 556 | **1 557** | 12 | fit-in-slot |
| NHL 17 V1 `(3,72)` | 120 469 | 23 439 | 23 439 | **23 447** (+8) | **1** | **append-and-retarget** |
| NHL 17 V2 `(−2,72)` | 120 469 | 23 439 | 23 439 | 23 442 (+3) | 1 | append |
| NHL 18 V1 `(3,72)` | 166 384 (2 chunks) | 25 805 | 25 805 | 25 805 (=) | 3 | fit-in-slot |
| NHL 18 V2b `setting.csv` 72 | 31 518 | 7 392 | 7 392 | 7 391 (−1) | 0 | fit-in-slot |
| NHL 19 V1 `(3,72)` | 191 552 | 29 869 | 29 869 | **29 885** (+16) | 3 | append |
| NHL 20 V1 `(3,72)` | 200 689 | 31 374 | 31 374 | 31 374 | 2 | fit-in-slot |
| NHL 21 V1 `(3,72)` | 205 779 | 32 185 | 32 185 | 32 185 | 7 | fit-in-slot |

Same plaintext length gives a different compressed length on **five** of the nine displayed vectors (+1, +8, +3,
+16 and **−1** — NHL 18 V2b shrinks by one byte) and the same length on **four**; over all 22 edit vectors in the
JSON it is 11 changed / 11 unchanged. Neither outcome can be assumed; the placement is decided by `stage::plan`
after encoding. Sizes above are CPython 3.12 `zlib` 1.3.1 level-9 output, equal to EA's bytes on the unedited text;
a different backend gives different numbers, which is why the placement is computed from the actual encoder output,
never from the plain-text length.

### 5.5 Archive metadata preservation (what may differ in the staged archive)

Fit-in-slot: the member's bytes `[offset, offset + new_size)` and zero fill to the old end; the row's `size` word.
Append: the appended payload at the old 16-aligned EOF plus its zero padding; the row's `block` and `size` words;
header `archive_size` (u64 at 0x18). In both cases: every other row byte-identical (including `word_4`, `word_12`,
`word_16` of the edited row — same path, so both hash words stay correct), header words 0x10 and 0x20..0x30 and the
inter-table gaps copied verbatim, every other entry's span byte-identical, the old payload bytes left in place on
append (dead, unreferenced). `plan.changed_ranges` enumerates exactly these ranges; `verify` checks them and the
complement. The NHL 15 EB `0x00100400` "declared `archive_size` smaller than length" anomaly (handoff 22 / stage
docs) does not occur on any of the six `0x00110400` archives (`archive_size == length` on all).

### 5.6 Copied-output reopen and restore

The route stages into a **new** file (`stage_file` refuses an existing output), re-verifies, and `tdb_txn::apply`
replaces the copied target's archive under a journal keyed by the source identity (whole-file SHA-256 + length,
`BoundSource::bind`). Restore = the journal's prior copy; a second reopen must show the original stored bytes and
plain SHA-256. Reopen after apply: parse → decode → §3.4 reopen rule (plain SHA-256 equals the review's recorded
value; wrapper form equals stock). Every requirement here is already in `txn.rs`/`stage.rs`; the CSV contract adds
the plain-text identity to the artifact record.

## 6. Developer vectors (offline; all from `p1bs-csv-defaults-contract.json` `vectors`)

Each vector names the member path and entry, the header-name match, the exact old and new line, the 0-based byte
offset of the line in the plain text, before/after plain SHA-256, original stored size, slot slack and the
illustrative re-encoded sizes. Summary:

| Vector | Target | NHL 16 | NHL 17 | NHL 18 | NHL 19 | NHL 20 | NHL 21 |
|---|---|---|---|---|---|---|---|
| V1 context | `contextsetting` `(3, 72)` `default_value` → 3 | `3,72,,,4,,1` → `3,72,,,3,,1` | `3,72,1,4,9` → `3,72,1,3,9` | `3,72,1,4,9,1,3` → `…,3,9,1,3` | same as 18 | same | same |
| V2 global | NHL 16 `setting` row 72 → 2; NHL 17 `(−2, 72)` → 3; NHL 18–21 explicit **paired edit** V2a `(−2, 72)` + V2b `setting` row 72 → 3 (one review, one undo, atomic) | `…,1,0,4,3,153` → `…,1,0,4,2,153` | `-2,72,1,4,9` → `-2,72,1,3,9` | two members | two members | two members (13-col `setting` row `…,4,9,3,-1,-1`) | same |
| V3 second edit on the file carrying V1 | `(3, 71)` → 0 | `3,71,,,1,,1` → `3,71,,,0,,1` | `3,71,1,1,11` → `…,0,11` | `3,71,1,1,11,1,3` → `…,0,11,1,3` | same | same | same |
| V4 no-op | `(3, 72)` → current value | no-op | no-op | no-op | no-op | no-op | no-op |
| R1 missing row | `(34, 72)` | refused | refused | refused | refused | refused | refused |
| R2 out of domain | `(3, 72)` → 5 | refused (max 4) | refused (list 9) | refused | refused | refused | refused |
| R3 | NHL 16: `(3, 68)` → 7 (max 5; the NHL 17+ `authentic` code does not exist) · NHL 17–21: id 83 | refused | refused (absent) | refused | refused | refused | refused |
| R4 context-specific domain | `(80 or 81, 88)` → 7 allowed (list 12); `(−2, 88)` → 7 refused (list 10) | n/a | 80 | 80 | 81 | 81 | 81 |

Additional offline vectors the developer should add: a synthetic duplicate `(3,72)` line (refused ambiguous); an
NHL 18 file where `setting.default_value ≠ contextsetting[−2]` (paired edit refused with both values shown; no
separate repair route); an NHL 18 pair whose `potential_values` differ (refused incompatible); a paired-edit failure
injected on the second `Replacement` (both members unchanged, journal restored); a member
with a `"` in the target line (refused); a header with `default_value` renamed (refused) and one with an extra
column appended (accepted, preserved); the D1 acceptance test (53/53 stock members byte-identical); and the
full `stage`/`verify` round trip on a copied `inxp_00.big` for one fit-in-slot and one append vector (NHL 18 V1 /
NHL 17 V1 with EA-faithful sizes), asserting `changed_ranges` = exactly §5.5 and every other member's digest equal.

## 7. User-review cards (not authorised here; user execution only; no live placement by Studio)

**Card P1BS-CSV-1 — one stored default, one title, copied target.** Titles: NHL 16–21 PS4; start with NHL 18 (V1 is
fit-in-slot, so the archive length and every offset stay identical — the treatment differs from the control only in
one member's bytes and one row's `size` word). All treatments on a copy of `databigs/inxp_00.big`; keep the original;
restore by hash afterwards. Record for every run: the copy's SHA-256 before/after, the plan's placement, the creation
screen's displayed option *before touching it*, and the day-1 mode save (`save_db.py`, `leaguestates` types 29–34,
41–43, 46–49 as in handoff 22 card P1BS-1).

- **Controls.** C0: stock archive, fresh profile → Franchise creation, record the displayed *Playoff Series Length*.
  C1: same with the existing profile. A C0/C1 difference supports a profile-associated difference; it is a control
  result to be matched, not a located setting store.
- **Treatment A.** V1: `(3, 72)` 4 → 3 in `contextsetting.csv` (context 3 `DISPLAY_FRANCHISE_CREATION_CLASIC`). Fresh
  profile; Franchise creation without touching the option; day-1 save.
- **Treatment B (separate run, stock A).** V2 global pair 4 → 3.
- **Treatment C (separate run).** V3 `(3, 71)` 1 → 0 (*Season Overtime Loss* 1 Point → 0 Points) — a second, distinct
  setting before any selected-value association is proposed.
- **Treatment D (NHL 17 only, separate run) with its own controls.** V1 there is append-and-retarget: the archive
  grows and the row is retargeted. An NHL 18 fit-in-slot result cannot explain an NHL 17 failure, so D needs
  **same-title, same-build controls** that separate packaging from consumption: **D0** — retarget the unchanged
  `contextsetting.csv` (identical plain text, appended and re-pointed; a byte-faithful re-encode makes this a pure
  placement test) and confirm the title boots and the creation screen looks stock; **D1** — the V1 treatment
  appended; **D2** — if D1 shows no visible change, a separately demonstrated NHL 17 treatment under the same
  placement (e.g. a visible edit in another appended member of the same archive) to show the retargeted row is
  loaded at all. Outcomes: D0 fails → packaging (retargeting) not accepted on NHL 17, D1 uninformative; D0 boots and
  D1 changes the screen → row honoured and default consumed; D0 boots, D1 unchanged, D2 visible → packaging honoured,
  this stored row not consumed (as A-unchanged on a fit-in-slot title); D0 boots, D1 and D2 unchanged → **inconclusive**,
  next control is an unrelated fit-in-slot edit on NHL 17. Never record an unobserved acceptance.

Bounded verdicts (each holds for that title, profile state and placement only): *displayed option changed* under A or
B → supports "the creation screen offers this stored row"; A-changed/B-unchanged or the reverse is a single-setting
observation about which row is read, not a rule. *Unchanged* → does **not** exclude the CSV: the `DEFAULT_*_VALUES`
row (70 `DEFAULT_FRANCHISE_VALUES` on NHL 17+) or the `_DEFAULT_` row may be the one read, a profile choice may mask
it, or the treatment may have been rejected/not loaded — next controls: edit row 70 (or 111/117 on NHL 18+), then the
fresh-profile C0, then verify the treatment bytes were loaded (an unrelated visible edit in the same archive). A
`leaguestates` type moving by exactly the treatment delta → a candidate association only (handoff 22 §4 rules;
third value on the same setting before upgrading it). Under D the distinction between "retargeted row not
honoured" and "row honoured, default not consumed" comes only from the NHL 17 controls D0/D2 above, never from an
NHL 18 fit-in-slot result; without them the D result is recorded as inconclusive together with the next control.

## 8. Tests (evidence-specific)

- **T-SRC.** `p1bs_csv_sources.py` reproduces `p1bs-csv-sources.json`: 65 members, all `same plain hash`, 0 copies,
  NHL 19 patch archives 0 hits, text properties as §1.
- **T-VAL.** On NHL 17–21 every eligible default (global and per-context) ∈ its row's list values; `id == value` on the
  ten lists; NHL 18–21 `setting.default_value == contextsetting[−2]` (32/32 per title); 88's rows 80/81/131 carry
  list 12; NHL 16 eligible rows: `min_value 0`, empty overrides, `is_enabled 1`; five ids absent on 17–21.
- **T-ENC.** Native-framed zlib-9 re-encode of the 53 stock chunkzip members is byte-identical (53/53; backend
  zlib 1.3.1 via CPython 3.12); the single-chunk `compress_chunkzip` output differs in header words for every member
  and in deflate bytes for 25/26 measured chunks. Independent decode of every re-encoded member equals the plain
  text (the property the contract requires; byte identity is the reproduction target).
- **T-SPLICE.** Every V/R vector of §6 on the real member text: plain SHA-256 before/after as recorded; whole-buffer
  equality outside the cell; tolerant re-parse equal except the cell; refusals with the recorded reason.
- **T-STAGE.** `BoundSource::bind` → `plan` → `stage_file` → `verify` on a copied `inxp_00.big` (NHL 18 fit-in-slot,
  NHL 17 append): `changed_ranges` exactly §5.5; other entries' digests unchanged; reopen decodes to the edited text;
  `deployable() == false` with the two open items named (`word_12`, native loading).

## 9. Unknowns (explicit)

1. Which stored row a creation screen reads (`DISPLAY_*` vs `DEFAULT_*_VALUES` vs `_DEFAULT_` vs `setting.csv`), and
   any precedence; whether `is_enabled`/`is_visible`/`usage` gate it. 2. Whether the loader accepts a re-encoded
   member (byte-faithful or not) and a retargeted row. 3. EA's raw-vs-chunkzip size threshold. 4. Selected-value
   storage (unchanged from handoff 22 §4). 5. Other patch levels than those installed (NHL 19 01.60 patches inspected;
   NHL 20/21 installs carry no patch archive). 6. Xbox 360 / PS3 / Legacy Recompiled: no data-file representation
   found in the sources inspected by handoff 22 — unchanged, and still "not found in inspected sources", not "absent".

## 10. Handoff 22 revision 3 (dispatch item 5 — wording only; measured data unchanged)

Applied in [22 rev. 3](../overnight-2026-09-18/22-settings-catalogue-defaults-and-cross-title-equivalents.md) §11:
(1) §2.1 states 21 distinct legacy ids across 20 displayed rows (67 and 85 share a line); (2) §2.5 tightened: table
headers and the file header are unchanged by a same-length cell edit, so their CRCs must stay byte-identical even if
the writer recomputes them; only the covering body CRC (next table's `prior_crc` or the EOF CRC) may change, at an
offset derived and verified independently; unrelated invalid checksums are not repaired; (3) §2.4: prior reviewed
edits are baseline, not an exemption — target identity and bounds are validated on every edit and no old review
ticket is needed to reopen; (4) §1.9: `HwRX.NVBd` remains structurally inferred, unlike the executable-witnessed
`stocksettings` fields — the decoder package may apply it but must keep that distinction, must key by table **and**
field with a `qbPU.CkjP` regression, and claims no "every field externally verified"; (5) §4: a fresh/existing-profile
difference is a control result needing matched conditions, not a located store; (6) §2.7/§5: NHL 16–21 now point
here. The catalogue JSON's `readings.value` no longer says "every `id` match[es] the executable": it states 222/223
(`id`), 222/223 (`min`), 186/223 (all six) with the exceptions.

## 11. Reproduce, checks run, cleanup

```
python local/runs/overnight-research-2026-09-19/probes/p1bs_csv_sources.py docs/research/overnight-2026-09-18/data/p1bs-ps4-setting-csv-catalogues.json <sources.json>
python local/runs/overnight-research-2026-09-19/probes/p1bs_csv_contract.py docs/research/overnight-2026-09-18/data/p1bs-ps4-setting-csv-catalogues.json <sources.json> <contract.json>
```

Checks actually executed this session (all read-only): the two probes above (logs in
`local/runs/overnight-research-2026-09-19/p1bs-csv/*.log`); ad-hoc python over the catalogue JSON for the
value-vs-index witness, `id ≠ value` lists, duplicate keys, empty cells, enabled/visible flags, `setting` vs `_DEFAULT_`
equality; chunk-header/padding byte dump on four members; zlib-9 byte-identity on all 53 compressed members; a
throwaway Rust binary (`flate2 = 1.1.9`, `Compression::best()`, built in the session scratchpad and deleted) on the
26 chunks of NHL 16/18/21 → 1/26 byte-identical. **Not executed:** any Studio binary or `cargo test`; any
`stage`/`verify` run (T-STAGE is specified, not run); any game launch.

Temporary artifacts (rev. 2 cleanup, per the addendum): `local/runs/overnight-research-2026-09-19/p1bs-csv/` —
the two regenerable JSON copies and `tables.md` **removed** (containment checked; regenerable by the commands above);
`sources.log` and `contract.log` (the probe stdout of the runs that produced the published data — small,
non-regenerable as a record of that execution) and `rev2_edit.py`/`rev3_edit.py` (the exact wording-edit scripts)
**retained**; scratchpad `deflate-probe/` build tree — removed earlier; no extracted members retained (all decoding
in memory). No processes left running.

## 12. Revision 2 — change list (2026-09-19)

Applied by `local/runs/overnight-research-2026-09-19/p1bs-csv/rev2_edit.py` after the
[review addendum](../../validation/p1bs-csv-review-and-overnight-addendum.md). No measurement changed; the JSON
data files are unchanged except one provenance string (`encoder_provenance.note` now records the backend version).

1. **Compression (addendum 1).** §5.3 D1 and §8 T-ENC: native framing and decoded-content preservation are the
   requirements; zlib 1.3.1 level-9 reproduction of 53/53 stock members is a reproduction target with recorded
   backend/version, not a universal loader rule; unchanged members keep original stored bytes; qualification
   disclosures stay unless narrowed by an actual test. Summary paragraph reworded accordingly.
2. **Paired defaults (addendum 2).** §3.1, §3.2, §3.5, §6 V2: NHL 18–21 global is an explicit paired edit of two
   cells in two members (one review, one undo, atomic apply/restore); matching current values and compatible
   domains required; unequal or incompatible pairs refuse the bounded operation; no separate repair route is
   authorised; `_DEFAULT_` is not an ordinary screen context. §2.1: stock equality is an observation.
3. **Parser contract (addendum 3).** §3.4 rewritten: the first writer's measured subset (ASCII no BOM, CRLF records,
   unique required headers, unquoted comma records, uniform column count); everything else browse-only/refused with
   a reason; extra columns survive; duplicate required headers and ambiguous row/choice bindings refuse. §4 table
   rows aligned with the subset (BOM / ≥ 0x80 / non-CRLF / any `"` byte / duplicate header → refused).
4. **Baseline/reopen (addendum 4).** §3.4: the sentence "baseline validation excludes exactly the cells recorded in
   earlier reviews" is removed; every edit re-validates the current target and domain; reopen never retains or needs
   the previous ticket; matching the last review hash is an output-verification test.
5. **Runtime controls (addendum 5).** §7 treatment D gains same-title NHL 17 controls D0/D2 and an explicit
   inconclusive outcome; §1 uniqueness limited to the 78 inspected archives.
6. **Evidence cleanup (addendum 6).** §5.4: five changed / four unchanged (−1 named), 11/11 over all 22 vectors;
   §2.1: 16 eligible ids vs the 32-id broader integrity check (16 eligible + 16 excluded-but-present) stated
   separately. Observed source facts and chosen Studio policy are labelled as such in §3.1/§3.4.
