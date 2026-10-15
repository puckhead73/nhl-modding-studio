# Handoff R37 — Team `index` / `artid` re-identification: a refuse-or-cascade contract proposal built on the measured reference census

Research lane, 21 September 2026, Priority 2 ("artid / index changes: exact dependency graph and refuse-or-cascade contract"). This is a
**policy proposal** over evidence already published (2026-09-20 handoff 07 §1 and its `r14-team-reference-census.json`, 8 databases; handoff
01 for the jersey triples; the 2026-09-18 handoffs 20 / 23 and 2026-09-19 handoff 06 for the asset families keyed by art id). No new
measurement was needed; nothing here is accepted or implemented. Claim `research-r37-team-reid-contract`.

## Plain-language summary

A team has two identities: its **index** (the row / team id every roster, jersey, equipment, award and schedule row points at) and its **art
id** (the number the art files, jersey packs and a few tables are keyed by). Changing either is not an edit of one cell: the index is used by
thousands of rows and by the schedule tables' rank order, the art id by hundreds of files whose *names* carry it. The safe contract is:
never renumber an index in the first implementation (refuse, with the dependency count shown); allow an art-id change only as a **swap
between two existing teams** (both art sets exist, so only two database columns follow) or as a move to an art id whose complete art set the
user has already placed (checked file by file per title); refuse everything else with the exact list of what would break. This keeps
the identity editor the developer shipped (names, colours) dependency-free and gives the "rebrand as another club's art" case a path.

## 1. Dependency graph (from the census; "→ index" = carries the team index, "→ artid" = carries the art id)

**Database, → index (every title where the table exists):** `exhibitionrostertable.team`, `exhibitionplayerbiotable.team` / `.juniorteam`,
`stockteamjerseys.teamid`, `stockgoalieequipment.team`, `stockteamequipmenttable.team`, `stockretirednumbertable.team`,
`stockawardstable.team`, `customteamjerseys.teamid`, `customteamequipmenttable.team`, `exhibitionteams.ahlaffiliate` / `.rivalteam`
(index-valued by name; census-ambiguous), the `league*` mode-local copies of the same tables; **schedule tables** reference teams by rank
within the league (handoff 12 of 2026-09-18), so the *order* of indices inside a league is a dependency too.

**Database, → artid:** `stockteamjerseys.teamartid` (95 % of rows on PS4 21; the rest are spare-kit art ids), `stockawardstable.teamartid`
(PS4), `customteamjerseys.teamartid`, `exhibitionteams.artid` itself.

**Files, keyed by artid (names carry the number):** gen7 `teamlogos*` families (`t<artid>` members, all 27 sources censused by the
developer's C9), PS4 registry directories (`teamlogos/…/t<artid>`, `size64 / 256 / 512`), PS4 vault packs keyed by `Team` (centre logos,
spotlight, BAP locker — 2026-09-20 handoff 11) and the jersey packs keyed by `(Team, UniformVariation, JerseyFit)` (handoff 01), gen7 jersey
/ rink texlibs by art id (2026-09-18 handoffs), NHL 21 `ArenaID`-keyed packs (arena art id, not team).

**Third id spaces (never cascade):** the NHL club rank 0–31 (`draftteam` / `proteam` / `origteamid`, `stockcareer*stats.team`), custom-team
ids 400–501, league art ids, player art ids — listed by the census as ambiguous / uncovered; they are *not* team index or art id.

## 2. Contract (proposal)

| Operation | Disposition | Rule |
|---|---|---|
| Identity edit keeping `index` and `artid` (names, colours, abbreviations, audio ids, loc keys) | allowed (already shipped) | dependency-free at the DB level; loc keys and audio ids have their own validations (handoffs 04 / 07 / 09) |
| Change `index` (renumber a team) | **refuse** in v1 | show the count of → index rows per table and the league rank position; a later cascade would need a transaction over every table above plus a schedule re-rank — not proposed |
| Swap `artid` between two existing teams A ↔ B | allowed (cascade) | write `exhibitionteams.artid` for both, rewrite `stockteamjerseys.teamartid` / `customteamjerseys.teamartid` / `stockawardstable.teamartid` rows of both teams; no file changes (both art sets exist); refuse if either art id is a spare-kit id (255, 1000–1004) or if a third team shares it |
| Move `artid` to a value not used by any team | allowed only when complete | for every art family the title ships for teams (C9 census), the destination art files must already exist (loose or in the copied archive) — else refuse listing the missing members; PS4 additionally requires the vault packs (`Team`-keyed and jersey triples) — **refuse on PS4** until a vault writer exists |
| Change `artid` to another team's value without swapping | **refuse** | two teams sharing art is representable but the jersey rows would collide; not a supported state |
| Re-identification inside a mode-local (`league*`) copy | **refuse** | the mode tables mirror the stock ids; no cascade path is established |

Every allowed cascade is one review item ("2 DB tables × n rows; 0 files") in the shared pending-changes review, applied as one transaction
with the existing recovery; every refusal names the tables / files that block it.

## 3. What would change this

A per-field semantics pass on the census-ambiguous columns (`*strategies.team_bias*`, `teamtype`, `arenatype` …) could enlarge the cascade
list; a vault writer (Q-R7-1) would unblock PS4 art-id moves; a member-rename capability in the archive planners would allow gen7 art-id
moves without pre-placed files. None is proposed here.
