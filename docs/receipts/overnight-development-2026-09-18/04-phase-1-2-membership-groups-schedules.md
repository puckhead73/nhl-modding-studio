# Receipt 04 — Phase 1.2: competition groups, league membership, source schedules

> **R2 correction (18 September, F3):** this receipt covers groups, membership and schedule *browse/validate*. It does not deliver the roadmap's schedule editing/import/generation workflow, rules/season/mode-initialization coverage, PS4 group names or league row creation; the per-target status is the [Phase 1 coverage matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md) §1.2 (packages P1-A/B/C).

Overnight developer lane, 18 September 2026. Consumes research handoff P1.2-X
(`docs/research/overnight-2026-09-18/12-phase1-competition-structure-and-schedules.md`,
`data/xbox360-competition-groups.json`, `data/schedule-tables-survey.json`; claim
`p1.2-competition-groups-and-schedules`, ready 06:25). Status: **implementation complete /
automated verified / user review pending.** Nothing launched, nothing live written.

## Plain-language summary

Phase 1.2 asks what else a league is made of besides its name: which teams belong to it, how it
is split into conferences and divisions, and where its season schedule comes from. Studio now
answers all three on every applicable title. **Groups:** NHL 12 through Legacy keep conference and
division as numbers pointing into a table inside each game's executable; those five tables (52 /
52 / 50 / 50 / 49 entries) are now baked into Studio, so team pages and league pages show
"Eastern Conference / Atlantic Division" in the title's own words instead of raw numbers, and the
numbers are never mixed between titles. **Membership:** a team can be moved to another league from
its page in one step, with the conference and division chosen from what that league actually has
on that title; impossible combinations (an AHL division under the NHL, a foreign group id, an
unknown league) are refused with a plain reason; older titles (NHL 08–11) and PS4 do the same with
their own simpler data and say what they cannot name. **Schedules:** every league's stock season
table is browseable — each game's date and teams — and checked (games per team, All-Star squads,
dates, ordering); the real anchors check out (Legacy opens Oct 7 Montréal at Toronto, 82 games
per club, the two All-Star squads at one game; NHL 10 has no All-Star row; NHL 21 runs 31 × 82
plus a 32-team table). Editing a game is an ordinary cell edit; generating or importing a season is
not claimed.

## What the research established and how it was consumed

| Fact (P1.2-X) | Consumed as |
|---|---|
| Team rows on NHL 12 → Legacy carry global `leaguegroup`/`conferencegroup`/`divisiongroup` ids into a title-specific executable table; ids differ per title | `defs/leagues/groups/{nhl12,nhl13,nhl14,nhl15,nhllegacy}.toml` generated from the JSON (kind derived from anchor usage, league from block structure — Winter Classic copies point into the NHL block); `competition_groups::GroupTable` / `ResolvedGroups`; catalog `competition`; `LeagueCatalog::structure(code)`; witness = the title's Xbox executable (`traced`), PS3/Recompiled `by-reference` |
| NHL 08–11 store local `conference` (1 bit) / `division` (3 bits), names untraced | `membership::GroupModel::LocalBits`: written raw when given, labelled as untraced numbers |
| PS4: `stockleagues` FK; group ids present, table in the encrypted executable | `GroupModel::Ps4`: FK validated (dangling refused), group ids raw when given, disclosed |
| Schedule rows: 0-indexed month/day, running index, `home`/`away` = position within the league; All-Star row on 07–Legacy; NHL 10 has 1 230 rows; PS4 15+ 6-bit slots, 31 teams from NHL 18, `nhl32schedule` | `schedules::{schedule_tables, schedule_view}` by metadata names; slot = rank of the team among its league's members by the team table's global `index` (the handoff's "position within its league" — verified: Legacy `nhlschedule[0]` = TOR (rank 27) vs MTL (rank 15); SHL slots 0–13 = the clubs at global index 62–75) |

Correction to the handoff's wording, established tonight: the team table's `index` is **global**
(Legacy: NHL 0–31, SHL 62–75), so a schedule slot is the team's *rank within its league* by that
index, not the raw `index` value. The handoff's NHL example (index 0–31) is the case where both
coincide.

## Implementation

- `crates/studio-domain/src/competition_groups.rs` (new): table loading (`defs/leagues/groups`),
  per-session resolution (dictionary text → observed text → key), `validate_membership`.
- `crates/studio-domain/src/leagues.rs`: `LeagueVocabularies` loads group tables;
  `LeagueCatalog.competition`; `group_label` prefers the table; `LeagueStructure` on `LeagueView`.
- `crates/studio-domain/src/membership.rs` (new): `options` / `plan`; `Workspace::set_cells_int`
  (mirrored multi-cell op), `team_membership_options`, `set_team_membership` (one transaction,
  one undo entry, refusals leave no history).
- `crates/studio-domain/src/schedules.rs` (new): table classification (`source` / `future` /
  `debug` / `favorite` / `mode-local` / `variant`), league matching by stem with disclosure,
  game rows with dates and resolved teams, findings (All-Star/exhibition squads, uneven games,
  members without games, unresolved slots, bad dates, self-games, index order).
- `studio-api`: `team_membership_options`, `set_team_membership`, `schedule_tables`,
  `schedule_view` (registry + commands); DTOs `MembershipOptionsDto`, `MembershipResultDto`,
  `LeagueStructureDto` (on `LeagueViewDto`), `ScheduleTableDto`, `ScheduleViewDto`; bindings
  exported (94 roots).
- App: `LeagueMembershipCard` on the Team Data tab; `StructureCard` and `LeagueScheduleCard` on
  the League page.

## Per-target disposition (Phase 1.2)

| Family | Groups | Membership | Schedules |
|---|---|---|---|
| NHL 12–15, Legacy — Xbox 360 | table **traced** (own executable) | validated global ids | browse + validate |
| NHL 12–15, Legacy — PS3; Legacy Recompiled | same table **by-reference** | same | same |
| NHL 08–11 (both platforms) | local numbers, names untraced (disclosed) | league code + raw numbers | browse + validate |
| NHL 15–21 PS4 | ids raw (table in encrypted executable) | FK-validated league + raw ids | browse + validate (no `status` field; 31/32-team layouts) |
| NHL 07 | — | **refused** (thin stock/custom index) | browse (schedule tables exist; league match by stem) |

Not implemented / not claimed: schedule editing beyond generic cell edits, season generation or
import (C6), custom season lengths, how Season/Franchise copies the source table
(`leaguescheduletable`), and any runtime effect of a moved team — all queued as runtime cards.

## Checks

| Check | Result |
|---|---|
| `cargo test -p studio-domain --test leagues` | 10 passed (incl. `competition_groups_resolve_per_title_and_validate_membership`) |
| `cargo test -p studio-domain --test membership` | 4 passed (Legacy global groups incl. refusals/undo; NHL 11 local bits + range refusal; NHL 21 FK refusal + raw ids; NHL 07 refused) |
| `cargo test -p studio-domain --test schedules` | 2 passed (Legacy NHL + SHL; NHL 10; NHL 21 incl. `nhl32schedule`) |
| `cargo test -p studio-api --test membership` | 1 passed (registry seam: options, structure, refusal, move, team view, pending review names `conferencegroup`/`divisiongroup`, undo) |
| `tsc --noEmit`; `vitest run` | clean; 68 passed |
| Checkpoint 3 (`cargo test -p studio-domain -p studio-api -p tdb-xiso`; bindings `--check`) | see `local/runs/overnight-development-2026-09-18/checkpoint3-rust.txt` |

## Changed paths

New: `defs/leagues/groups/*.toml` (5), `crates/studio-domain/src/{competition_groups.rs,membership.rs,schedules.rs}`,
`crates/studio-domain/tests/{membership.rs,schedules.rs}`, `crates/studio-api/tests/membership.rs`,
`app/src/components/{LeagueMembershipCard.tsx,LeagueScheduleCard.tsx}`, bindings `Membership*Dto.ts`,
`LeagueGroupsDto.ts`, `CompetitionGroupDto.ts`, `LeagueStructureDto.ts`, `Schedule*Dto.ts`.
Modified: `crates/studio-domain/src/{lib.rs,leagues.rs,session.rs,workspace.rs}`,
`crates/studio-domain/tests/leagues.rs`, `crates/studio-api/src/{dto.rs,convert.rs,registry.rs,commands.rs,lib.rs,bindings.rs}`,
`app/src/lib/client.ts`, `app/src/pages/{TeamDetail.tsx,LeagueDetail.tsx}`.
