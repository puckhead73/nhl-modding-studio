# Handoff R56 — Gen7 kit selectors (Teams 2.6): every `stockteamjerseys` row resolves to its jersey / pant / sock textures by `(style, teamartid, variant)` on NHL 11–Legacy, both consoles (627–704 / 627–704 per title; PS3 12 / 13 by archive member listing); the unreferenced shipped kits are counted per title; custom-team rows have no per-team files

> **Corrections — 2026-09-22 pm ([research R74](../overnight-2026-09-22-pm/06-developer-observations-reconciled.md), developer observations O-2 / O-3 / O-4 / O-6).** (O-2) NHL 11 / 12 socks come as **two files per kit**, `sock_<style>_<0|1>_<artid>_<variant>_cm`; measured on every pair (756 on NHL 11, 973 on NHL 12): the `_1_` file is the **horizontal mirror** of the `_0_` file (459 / 704 clear, 153 / 124 identical because symmetric, the rest mirror-like or flat) — one kit, both files required, both copied on any re-pointing; no selector between kits. (O-3) The "teams with rows" figures here count distinct kit art ids; the Team-page count is team-table rows owning ≥ 1 kit row (developer: NHL 12 213, NHL 14 216, NHL 15 216). (O-4) "Files without a row" become "complete candidates" vs "incomplete leftovers" (developer counts: 20 / 22 / 1 / 2 / 0 / 58 incomplete of 117 / 277 / 208 / 317 / 54 / 286 on X360 11 / 12 / 13 / 14 / 15 / Legacy). (O-6) PS3 NHL 12 / 13 archive members are `.rpsgl` — the name grammar transfers, the bytes never do.

Research lane, 21 September 2026 pm, roadmap step 2.6 gen7 (prompt step 5 "Teams: kit variants and selectors"). Claim
`research-r56-gen7-kit-selectors`. Read-only over the anchors and the loose trees; nothing game-verified; no writer.

Machine-readable: [`data/r56-gen7-kit-selector-join.json`](data/r56-gen7-kit-selector-join.json) (per tree: kit files per family, table
columns and value censuses, join rates under three candidate keys, rows without files, files without rows, teams without rows,
`teamid` → `artid` consistency). Probe `r56_gen7_kit_selector_join.py` (≈ 30 s).

## Plain-language summary

The gen7 jersey table (`stockteamjerseys`, 627–704 rows on NHL 11–Legacy) selects a team's kits with three numbers — `style` (0 / 1),
the team's art id and `variant` — and those three numbers are exactly the file name of the kit textures: `rendering/jersey/
texlib_<style>_<teamartid>_<variant>`, the same for `pant/`, and `sock/sock_<style>_<teamartid>_<variant>_cm` from NHL 13 on (`sock_<style>_<0|1>_<artid>_<variant>_cm`, two files per kit, on 11 / 12). Every row
on every measured title resolves to all three files (PS3 and Xbox 360 alike); `jerseyindex` is the per-team ordinal (0, 1, 2 …) and
`fevariant` equals `variant` on every row. The trees also hold kits that no row references (26 on NHL 15 up to 322 on NHL 14 per family)
— shipped variants a kit selector could offer. The 30 custom (create-a-team) rows per title point at the shared art id 20054 and have
no per-team texture files, so custom kits are built another way (not measured); NHL 09 / 10 have no jersey table (the era gap the
Phase 2 matrix records) although their trees carry 480–544 kit textures per family.

## 1. Join (X360 / PS3; `stockteamjerseys`)

| Title | rows | jersey files / pant / sock | `(style, teamartid, variant)` → jersey / pant / sock | `teamid → exhibitionteams.artid == teamartid` | teams with rows / total (the 30 without = league 13 custom teams) | files without a row: jersey / pant / sock |
|---|---|---|---|---|---|---|
| NHL 11 (X360 = PS3) | 627 | 751 / 762 / 756 (socks `sock_<style>_<0│1>_<artid>_<variant>_cm`, two per kit) | **627 / 627 / 627** | 627 / 627 | 212 / 242 | 124 / 135 / 129 |
| NHL 12 (X360 = PS3, the PS3 side by **archive member listing** of `cacherender.big` + `nocacherender.big`) | 676 | 967 / 979 / 973 (NHL 11 sock form) | **676 / 676 / 676** | 676 | 218 / 248 | 293 / 305 / 297 |
| NHL 13 (X360; PS3 by archive listing: 865 / 865 / 865 files, same joins, 185 spare) | 684 | 864 / 864 / 864 | **684 / 684 / 684** | 684 | 218 / 248 | 184 / 184 / 184 |
| NHL 14 (X360 = PS3) | 704 | 1 024 / 1 025 / 1 022 | **704 / 704 / 704** | 704 | 222 / 252 | 322 / 323 / 320 |
| NHL 15 (X360 = PS3) | 659 | 683 / 684 / 683 | **659 / 659 / 659** | 659 | 222 / 252 | 26 / 27 / 26 |
| Legacy (X360 = PS3) | 703 | 905 / 959 / 946 | **703 / 703 / 703** | 703 | 222 / 252 | 207 / 261 / 248 |

Alternatives tested: `(style, teamartid, jerseyindex)` resolves 457–534 (jerseyindex is the per-team ordinal, not the file variant);
`fevariant` = `variant` on every row (same 100 %). Value spaces (Legacy): `style` 0 (290) / 1 (413); `variant` 0–10 (0 / 1 dominant);
rows per team 1–13 (2 on 131 teams, 3 on 52; the NHL clubs 7–13); flags `isdefault` (440 / 703), `hastie` (230), `islightcolor` (307),
`helmetlogo` (535), `helmetnumbers` (259), `islockable`; `locstring` = the kit's name key index. `customteamjerseys`: 60 rows (2 per custom
team, `teamartid` 20054, `jerseyback` column) — **0 / 60** resolve to files (no `texlib_*_20054_*`): custom kits are not per-team files.

## 2. Contract input (developer, Phase 2.6 gen7)

- **Listing / selector:** show each team's kits from its `stockteamjerseys` rows with the resolved jersey / pant / sock textures (the
  NHL 11 / 12 socks carry an extra 0 / 1 sub-index — two files per kit); label `isdefault`, `islightcolor`, `hastie`, `helmetlogo` / `helmetnumbers` as the row's flags.
- **Reassignment:** a row's `variant` (and `style`) may be re-pointed only at a `(style, teamartid, variant)` triple that has all three
  files — the "files without a row" set is the shipped spare pool per team art id; `teamartid` must stay the team's `artid` (703 / 703
  consistent — treat a mismatch as a refusal).
- **New kit:** a new variant = three new textures named by the triple (+ the kit's `locstring` name) and a new row — static; runtime
  acceptance unmeasured (card material).
- **Refusals:** NHL 09 / 10 (no table; files exist — selection mechanism unknown), custom teams (art id 20054 has no files).

## 3. Not claimed

What `style` 0 / 1 means beyond "the two jersey mesh styles" (handoff 10's shared meshes); which row the game uses when several are
`isdefault`; NHL 09 / 10 kit selection; the custom-kit path; runtime.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r56_gen7_kit_selector_join.py docs/research/overnight-2026-09-21-pm/data/r56-gen7-kit-selector-join.json   (≈ 30 s)
```
Executed four times (key discovery on Legacy; the PS3 12 / 13 archive-listing mode through the 2026-09-18 `eb_list.parse`, no extraction; the NHL 11 / 12 sock form).
