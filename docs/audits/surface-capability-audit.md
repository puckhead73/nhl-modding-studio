# Surface & capability audit (Phase 1)

> **Status:** established 2026-08-01 (Phase 1 of the
> [Full-game QA and editability plan](../plans/full-game-qa-editability.md)).
> **Machine-readable companion:** [`data/surface-capabilities.csv`](data/surface-capabilities.csv)
> (the probed state of every surface on every one of the 27 main-DB anchors, drift-guarded by
> `crates/studio-domain/tests/surface_matrix.rs`).

Phase 1 makes the UI stop lying: every database surface is gated on a **probed capability** derived
from the loaded file (table + key + join + required fields), never from the title id. An absent
feature is *omitted*, not rendered as an empty tab; a present-but-not-authorable surface is *disabled
with a reason*; empty optional data stays an honest empty state.

## The capability contract

Each surface carries a `SurfaceCapability { state, reason }`
([`studio_domain::surfaces`](../../crates/studio-domain/src/surfaces.rs)), surfaced to the UI on
`ProjectSummaryDto.surfaces` and consumed by the pure tab-selection helper
([`app/src/lib/surfaces.ts`](../../app/src/lib/surfaces.ts)).

| State | Meaning | UI behavior |
|---|---|---|
| `not-applicable` | the schema does not carry the feature | tab omitted; no query fired |
| `readable` | present, read-verified; no whole-surface write claimed | tab shown, read views |
| `editable` | present; a write path is implemented + regression-covered today | tab shown, editors (per-field `update` still governs individual controls) |
| `disabled` | present but not authorable yet (assets) | tab shown with a reason; no editing |
| `unverified` | present but semantics not ground-truthed | tab shown read-only with caution |

**Probes validate the complete minimum requirement**, not mere table presence — e.g. NHL 07 ships an
`StEO` arena table but with **no `arenaname` field**, so `arenas` is `not-applicable` (a name-less
list would be a blank-surface defect), and the read matrix
(`crates/studio-domain/tests/read_matrix.rs`) proves the verdict agrees with what the builders
actually construct on all 27 anchors.

## Surface → requirement map

Tables/fields are studio (community) ids. "Absent behavior" is what the domain builder returns when a
requirement is missing — uniformly an empty vec / `None` / dropped section (never a panic).

| Surface | Required table | Key/join/fields | Optional | Edit path | Absent behavior |
|---|---|---|---|---|---|
| **players** | `ubPc` (+ model) | bio identity; companions `dSvy`/`mHuy` | style, contract, draft, rookie fields | field edits + entity ops | empty list |
| **teams** | `kOtt` (+ model) | name via `kOtt` or `HqzU`/`fLEY` (thin-index join `mCXg`+`RREB`) | arena link `buDa` | field edits | empty list |
| **arenas** | `StEO` | `arenaname` field; `kOtt.buDa → StEO.vfEq` reverse join for tenants | city, type | raw fields | `not-applicable` (NHL 07 has no `arenaname`) |
| **career** | any of `tsSy`/`OAqO`/`USfF`/`nCDp` | `game_id` join = `wBIz+1` | per-column stat fields | `add_career_season` (nhlng-only / aligned-mirror) | 4 empty sections; **`not-applicable` on roster-save display** |
| **attributes** | `dSvy`/`mHuy` | rating classification (name prefix + range) | style `QgFs`/`mQoE`/`wJus`, potential, biases | `set_field` | `default` (no kind) |
| **traits** | `dSvy`/`mHuy` trait slots (`wBhb`…) | 32-slot decode | — | `set_player_trait` | `not-applicable` (gen6) |
| **lines** | `eGlu` | named 1-bit slot fields (≠ `rosterstatus`) | per-slot archetype `QgFs` | slot edits, `set_dressed` | `None` |
| **strategies** | `EeYo` | `vfEq` key; career mirror `JUfQ` | 60 tactic/bias fields | `set_field` (mirrors to `JUfQ`) | `None` (schema present ⇒ empty-row is honest empty, not absent) |
| **captaincy** | `eGlu` | slot captaincy/jersey | — | `set_captaincy`, `set_jersey_number` | `not-applicable` |
| **contracts** | `ubPc` | `kKhd` (cap) / `IhDG` (term) | two-way, UFA year, scale | `set_field` | `None` cells |
| **draft_picks** | `qHav` | `Uwdb` current owner | orig owner, year/round/overall | `trade_draft_pick` | empty vec / `not-applicable` (NHL 10–11) |
| **jerseys** | `xjgw` | `mCXg` team FK (0-indexed) | style/variant/colour fields | `set_field` | empty / `not-applicable` (≤ NHL 11) |
| **team_equipment** | `hHbv` | `dXSB` team FK (1-indexed) | 9 BGR colour zones | `set_field` | empty / `not-applicable` (NHL 07) |
| **player_equipment** | `xmja`/`fMVl` | — | — | none built | **`disabled`** (Phase 6) where present; `not-applicable` on gen6 |
| **portraits / logos / heads / audio** | asset resources | — | — | none built | **`disabled`** (Phase 5/6 — never promoted here) |

## Retired numbers, memberships, depth chart, transactions

Not standalone tabs: **retired numbers** (`wcSL`, team FK `dXSB` 0-indexed) ride on `TeamView`;
**memberships** ride on `PlayerView` (resolved to team names, `None` documented null); **Depth Chart**
and **Contracts** are roster-list-driven (no optional table) so they always show; **Transactions** and
**Pending Edits** are project-level and unconditional.

## Frontend gating applied

- **Player detail** ([`PlayerDetail.tsx`](../../app/src/pages/PlayerDetail.tsx)): tabs filtered by
  `PLAYER_TABS`; the **Career** query no longer fires when `career` is `not-applicable`; **Portrait**
  stays as a `disabled` placeholder driven by the backend reason.
- **Team detail** ([`TeamDetail.tsx`](../../app/src/pages/TeamDetail.tsx)): tabs filtered by
  `TEAM_TABS` — Lines/Strategies/Jersey&Captaincy/Draft/Jerseys/Equipment omit when their table is
  absent; Depth Chart / Contracts always show.
- **Nav** ([`AppShell.tsx`](../../app/src/AppShell.tsx)): the **Arenas** subtab drops out when
  `arenas` is `not-applicable`.
- **Arenas page** ([`Arenas.tsx`](../../app/src/pages/Arenas.tsx)): direct-URL safety — shows an
  explicit unavailable state and fires no query when `arenas` is `not-applicable`.

Tab selection is a pure function (`resolveTabs`/`visibleTabKeys`) unit-tested in
[`surfaces.test.ts`](../../app/src/lib/surfaces.test.ts) without a browser.

## Known-risk disposition (initial-audit defects)

| # | Risk | Disposition |
|---|---|---|
| 1 | Team tabs unconditional | **Fixed** — gated on `TEAM_TABS` surfaces. |
| 2 | Portrait placeholder | **Honest** — `portraits` is `disabled` with a reason; not claimed supported. |
| 3 | Empty table looks like absent feature | **Fixed** — capability = schema presence; empty-row surfaces stay `editable`/`readable` and show honest empty states; only truly absent schema is `not-applicable`. |
| 4 | NHL 07 no equipment tables | **Verified** — `team_equipment`/`player_equipment` `not-applicable` on NHL 07 (probe + `surfaces.rs` test). |
| 5 | NHL 10–13 style/archetype semantics | Field-level `player_style` cap already gates archetype cells; unchanged. |
| 6 | NHL 10–12 no arena link | Arena *link* (Venue card) gated server-side by `arena_link`; the Arenas *list* is independent (`StEO`+`arenaname`) and correctly available on NHL 10–12. |
| 7 | Draft/rookie/junior/UFA vary | Field-profile omits absent cells; `draft_picks` surface `not-applicable` on NHL 10–11 (no `qHav`). |
| 8 | Career differs main vs roster display | `career` probed on the **displayed** file → `not-applicable` on a roster save lacking career tables. |
| 9 | PS4 loc missing NHL 19/21 | Localization is onboarding (Phase 0 matrix `missing-fixture`), not a DB presentation surface. |
| 10 | Legacy Recompiled no anchor | Probing runs on whatever DB loads (the Xbox-derived Legacy `nhlng.db`); no title-specific assumption. |
