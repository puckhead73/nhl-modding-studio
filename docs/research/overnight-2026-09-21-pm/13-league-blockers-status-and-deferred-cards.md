# Handoff R55 — League parity blockers (prompt step 3): what this campaign closed, what it bounded, and the deferred cards for the items only a runtime can settle

Research lane, 21 September 2026 pm. Claim `research-r55-league-blockers-status`. A status handoff over this campaign's measurements
plus two bounded probes (the CHL participant search across the NHL 21 data files; the NHL 09 executable's setting labels). Read-only;
nothing game-verified; no launch.

## Plain-language summary

Of the four league blockers the prompt named — NHL 07 / 08 texture exceptions, archive / container dependencies, tournament participant
selection and settings / rules gaps — two are now measured rather than open: NHL 07 and NHL 08 textures obey the same descriptor rules
as the later titles (NHL 08 inside a RefPack wrapper; 110 NHL 08 jersey libraries are undecodable and stay refused), and the archive
dependency of a league asset is exactly the existing placement chain (loose file = archive member on every measured tree; the developer's
C3 / C4 routes). The other two are outside the data files: the 24 domestic Champions Hockey League participants are chosen by code (no
table in the database, the kernel-entity CSVs or the service definitions — searched), and NHL 08 / 09 settings are defined through the
front end's Ion adapters with per-setting label keys (`SkillLevel.%d`, `PeriodLength.%d` … 60 names in the NHL 09 executable) instead of
the NHL 10+ descriptor table, so their value ranges and defaults are not a static catalogue either. Both get a deferred observation card;
neither blocks the independent work.

## 1. Disposition

| Blocker | Status after this campaign | Where |
|---|---|---|
| NHL 07 texture descriptors | **measured**: `DW8` = pixel-entry index (10 666 / 10 675), `DW12` rule 9 554 / 9 555, tile-aligned pitch 10 666 / 10 666; table-at-EOF layout with untagged payload entries; `DW0 = 3` | handoff 02 §1–2 |
| NHL 08 texture descriptors | **measured**: RefPack (`10 FB`) around the NHL 08+ `RW4xb2` layout, 14 066 / 14 066 on every rule; **110 jersey home / away libraries (`texlib_1_<team>_{0,1}`) undecodable — refuse** (byte-identical in `data0.big`) | handoff 02 §3 |
| Archive / container dependencies (league imagery) | unchanged from the 2026-09-21 handoff 02 §2 and the developer's C3 / C4 / P2-A routes: loose = archive member on every measured gen7 tree (the Xbox NHL 14 stream and PS3 NHL 13 speech being the audio exceptions); the NHL 08 jersey libraries above are the only new dependency caveat (the archive holds the same undecodable bytes) | 2026-09-21 handoff 02, developer receipts 03 / 04 / 08 |
| Tournament participant selection (Q-R36-1) | **not in data**: the NHL 21 `nhl_game.db` (2026-09-21 handoff 08), `kernel_entity_csv/league.csv` / `team.csv` (LEAGUE_ID / NHL_TEAM_ID / AHL_TEAM_ID display tables), the tournament service definitions (`service-defs/**/tournament*`, `league_creation/competition.json`: `SubMode` ∈ {`CHAMPIONS_LEAGUE`, `SPENGLER_CUP`}; `tournament/league_state.json`: `NumberOfTeams` ∈ {2, 4, 8, 16}, `SeriesLength`, tie-break enums) carry no participant list — the selector is code-resident or standings-driven. **Deferred card L-CHL-1** (§2) | this handoff |
| Settings / rules gaps (NHL 08 / 09 X360) | **bounded**: the NHL 09 executable carries no 32-byte setting descriptor table (the `p1bs_gen7_setting_table` anchor walk finds none); settings are `FE::CSettingsIonAdapter` label formats `<Setting>.%d` — 60 setting names (AIDumpPuckFreq … TwoLinePass, incl. `SkillLevel`, `PeriodLength`, `Penalties`, `Fighting`, `Icing`, `TieBreak`, `GameSpeed`, volumes) whose value labels are dictionary keys `<Setting>.<n>` and whose ranges / defaults live in code — no static catalogue; keep the 1.2 cell "research unknown → no data-file copy (NHL 09 form identified)". NHL 08 not opened tonight (13 MB image; same era expected, unmeasured) | this handoff |

## 2. Deferred user cards (observation only; none requested now)

- **L-CHL-1 — CHL domestic participants (PS4 NHL 18–21).** Baseline: an untouched install. Treatment: none (observation). Observable:
  start the Champions Hockey League tournament twice from the same roster state and record the 32 listed participants each time; then
  once more after changing one domestic club's league standing input if the mode offers a season context. Selection: *identical lists*
  → a fixed table in code (next: executable search for 32 team-index runs); *different lists* → standings / random selection at runtime
  (no static handle; the matrix cell becomes "runtime-selected, not editable"). No restore needed.
- **L-SET-09 — NHL 09 Xbox 360 settings source.** Baseline: the shipped `nhl_eng_us.db`. Observable: whether the game-settings screen's
  option labels match the dictionary strings under keys `SkillLevel.0…`, `PeriodLength.0…` (the developer can list those keys statically
  first; if the keys are absent from the dictionary the labels are compiled and the card is void). Selection: labels from the dictionary
  → editable label text only (values remain code); nothing else follows.

## 3. Not claimed

Runtime honouring of any edited template row; NHL 08's executable; anything about the NHL 08 jersey stream beyond "undecodable by the
standard RefPack command set".
