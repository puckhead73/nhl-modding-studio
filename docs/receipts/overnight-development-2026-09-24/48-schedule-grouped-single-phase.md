# Receipt 48 — single-phase grouped schedule generation (R123): user groups, shipped groups by exact identity, host rules checked, independent post-check

**Developer-verified; not Codex-accepted; nothing launched; no runtime claim.** HEAD `8d60a6d` (unchanged; nothing
staged). Claim `dev-schedule-grouped-single-phase`. Direction: [dispatch after receipt 45](../../plans/development-20260928-after-receipt45.md) §3.
Research consumed: R123 (handoff 16) and R113 / R113b (handoff 07). The observation for research is
[observations/48](observations/48-R123-validator-gaps-and-phase-decision.md).

## 1. Plain-language summary

- **Generate a league schedule from groups.** In League → Schedule → Generate, choose "groups", put each club in a
  group, and set the games per pair inside a group and across groups. Studio turns that into the games of every
  pair and builds the table with the existing generator. Preview, one-step apply, undo, and save to a copy all work
  as before.
- **"Use the shipped groups" only when it's exactly the shipped table.** It fills the groups and counts only when
  the file you opened, the table and the league's membership all match what research recorded. Otherwise it says
  why, and you enter the groups yourself. Studio never guesses groups from team order or from another title.
  - It matches all 41 anchor copies research recorded: NHL 07 → Legacy on Xbox 360 / PS3, PS4 15–21, and the
    genuine NHL 20.
  - `anchors/ps4/nhl20` is correctly recognised as the NHL 19 base by its hash.
- **Odd game counts need a real host rule.** Choices are lower slot, higher slot, balanced (now precisely defined,
  with every chosen host listed) or "I choose for each pair". An unknown rule, such as the coordinator's
  `invalid-rule`, is refused instead of being ignored. So is a per-pair list that is incomplete, repeated, or names
  the wrong pairs.
- **The built table is checked independently before any write:**
  - the row count;
  - kept rows unchanged in value and position, and every stored index unchanged;
  - each pair's games equal to the request;
  - home / away splits (even counts equal; odd counts give the chosen host exactly one more);
  - no club twice on a date, including against its kept games.

  A failure refuses the plan.
- **Unchanged:**
  - the Austrian two-phase table stays refused;
  - the NHL table stays round-robin, import or your own matrix;
  - no table grows;
  - no NHL opponent pattern is invented.

## 2. Files

| Path | Change |
|---|---|
| `tools/schedule/extract_r113_stock_partitions.py` (**new**), `defs/schedules/stock-partitions.json` (**new**, 30 rows, `from_sha256` of R113b) | complete single-phase stock partitions only (`conforms`, controls detected, cliques complete, not `austrianschedule`); `--check` reproduces |
| `crates/studio-domain/src/schedule_stock.rs` (**new**), `lib.rs`, `project.rs` (loads it) | `r113_fingerprint`, `stock_groups` (hash + table + fingerprint + member count, else the reason) |
| `crates/studio-domain/src/schedules.rs` | `SlotEntry.art_abbrev` (the `artabbr` R113's fingerprint reads; NHL 07 slot 9 shows `MIK` but stores `MOR`) |
| `crates/studio-domain/src/schedule_edit.rs` | `GroupQuotas`, `groups`, `odd_hosts`, `OddHome::Explicit`; `groups_to_matrix` (Q5); explicit-host validation; `check_generated` (post-check); balance rule documented; all hosts reported; two "every stock table is date-sorted" comments corrected |
| `crates/studio-domain/src/workspace.rs` | `schedule_stock_groups`; `schedule_session` public |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | `ScheduleGroupsDto`, `ScheduleOddHostDto`, `ScheduleStockGroupsDto`; config `groups` / `odd_hosts`; `per_pair`; `check_odd_home` refuses unknown rules; `schedule_stock_groups` command (root count 168) |
| `app/src/components/LeagueScheduleCard.tsx` (+ test), `app/src/lib/client.ts`, bindings (4 new / changed) | groups mode, shipped-groups button with reason, per-pair host editor, odd-host picker in both matrix modes |
| tests (**new**) `studio-domain/tests/schedule_stock_r113.rs`, `schedule_grouped.rs`; `studio-api/tests/schedule_edit.rs` (+1 case); literals in `schedule_edit.rs` / `schedule_preservation.rs` gain the two new fields | below |

## 3. Evidence

- **`schedule_stock_r113` 2 / 2.**
  - **Identity:** every anchor whose SHA-256 is a recorded source matches, 41 instances. For each, Studio's
    fingerprint equals R113's, the groups cover every member, and within × pairs + across × pairs equals R113's
    games and the table's active rows.
  - **Fail-closed cases:** another title's hash, an unknown hash, no file, swapped membership, and tables without
    groups (`nhlschedule`, `austrianschedule`, `ahlschedule`).
  - **First run failed, twice (recorded):**
    - the fingerprint used the displayed abbreviation instead of `artabbr`; the fix reads `artabbr`;
    - the test opened sessions without a title id, so the league vocabulary was unselected and NHL 12 → Legacy
      looked "not modelled". A project open sets it, so this was a test mistake, not a product gap. The test
      now sets it, and all 41 match.
- **`schedule_grouped` 4 / 4, `--include-ignored` with `NHL_STUDIO_FIXTURES=E:\nhl-studio-fixtures`:**
  - **X360 NHL 11 Swiss:**
    - shipped groups 4 / 4 / 4, within 6, across 4; then user groups within 14, across 1, whose 48 odd pairs
      have per-pair hosts;
    - the rules are refused in turn: none; empty; incomplete; with an even pair; duplicate; with a
      non-participant;
    - per-pair hosts are honoured; balance is deterministic with all 48 hosts listed;
    - each write goes preview → apply → undo → redo → save a copy under a path with spaces → reopen equal → stale
      preview refused, nothing written;
    - pair counts, splits, indexes and dates are re-checked from the read-back rows by test code independent of
      the generator.
  - **Refusals:** a club in two groups; in none; an empty group; a non-participant; groups and matrix together; the
    wrong total; hosts given with another rule.
  - **PS4 NHL 17 Finnish:** 5 groups of 3 (450 games), no `status` written.
  - **Genuine NHL 20 (CUSA15439) Swiss:** on a byte copy of the private fixture; groups by exact identity
    (`ps4-nhl20-real`), full round trip; the Austrian table offers no groups.
- **Regressions:**
  - `schedule_edit` 15 / 15, including the genuine NHL 20 install case and the all-anchor parity sweep;
  - `schedule_preservation` 4 / 4: the post-check now also runs on its kept-row quota cases and passes.
- **Other suites:**
  - studio-api `schedule_edit` 2 / 2 (new: shipped groups via the Registry, a grouped preview, four invalid rule
    strings refused);
  - vitest `LeagueScheduleCard` 12 / 12, with the new groups case; the first run **failed** on my wrong game
    arithmetic in the fixture and a missing odd-host picker in groups mode, both fixed;
  - `tsc` clean; `export-bindings --write` 4 files.

## 4. Limits

- Dates are Studio's conservative packing (receipt 41). Feasible requests can still be refused, and that is not
  claimed optimal.
- The round-robin path keeps its existing checks; the new independent post-check runs on the matrix and group paths.
- No runtime claim that the game plays a generated grouped table as intended (card P1-A).
- The Austrian phased writer is withheld (observation §2 lists the smallest decision).
