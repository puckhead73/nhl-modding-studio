# Receipt 02 (T1) — Teams 2.2: relationship edits disclosed, saved, reopened and undone on every anchor

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Nothing staged, committed, pushed,
launched or written live; every save landed in a temp directory, the committed anchors were read
only. Claim: `dev-t1-team-relationships`. Clock: 05:24 → 05:48 (MDT).

## 1. Plain-language summary

The team page could already move a team to another home arena, but it did so with a bare dropdown:
nothing said who already played at the new arena or that the old one would be left without a team,
and no test had ever proved that such a move, a retired number, a traded draft pick or a strategy
change survives a save and reopen on any title. Now the arena picker shows a disclosure first —
the exact database cell it writes, the teams already at the chosen arena, the teams that stay at
the current one (or that it becomes vacant) — and only **Assign** writes; a refusal (same arena,
an arena that is not in the title's arena table, a title whose team table has no home-arena link)
is shown instead of a write. A single test now runs those four relationship edits on **all 27
committed anchors**: the arena move where the link exists (15 anchors), retired numbers (26), draft
pick trades (18) and one strategy cell (27), each edit → validation delta → save → reopen on a copy
→ undo, with the per-title table recorded as evidence. Team / league creation, cloning and
deletion remain **not implemented** (no reference / allocation contract); they are inventoried in
the ledger, not hidden.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-api/src/dto.rs` | **new** `ArenaAssignmentPreviewDto` (cell, from / to arena refs, other tenants, vacancy, allowed / refusal); `ArenaRefDto` derives `Clone, Debug` |
| `crates/studio-api/src/registry.rs` | **new** `arena_assignment_preview(id, team_id, to_arena)` — read-only, composed from `team_view` / `arena_view` / `list_teams` (names resolved, filtered by team id) |
| `crates/studio-api/src/commands.rs`, `src/lib.rs`, `src/bindings.rs` | command registered; export root added |
| `app/src/bindings/ArenaAssignmentPreviewDto.ts` (sha256 `07b5e60f…`) | generated; `--check` in sync |
| `app/src/lib/client.ts` | `arena_assignment_preview` shape |
| `app/src/components/EditableField.tsx` (sha256 `f7bd2246…`) | `ArenaSelect` is a disclosed two-step assignment: pick → preview (bound to project / team / record / field / candidate, `staleTime 0`) → **Assign** writes the previewed cell with the previewed id through `set_field`; refusal / pending / failed previews never write; a target change drops the candidate; an in-flight ref guard ignores a second click |
| `app/src/components/FieldProfile.tsx` (sha256 `1c1f5ae0…`), `app/src/pages/TeamDetail.tsx` | `teamId` threaded to the arena cell (read-only fallback when absent) |
| `app/src/components/ArenaSelect.test.tsx` (**new**, sha256 `c6de503d…`) | 6 cases (§4) |
| `crates/studio-api/tests/team_relationships.rs` (**new**, sha256 `9daec699…`) | the 27-anchor sweep (§3) |

## 3. Coverage (from the sweep's own table, [`t1-team-relationships-sweep.log`](../../validation/evidence/overnight-development-2026-09-24/logs/t1-team-relationships-sweep.log))

| Edit | Anchors where it is **impl+tested (dev)** | Anchors where it is **n/a (measured)** |
|---|---|---|
| Home arena: preview disclosure (tenants at target, who stays, vacancy), refusals (same arena; uncatalogued id), `set_field`, tenant counts move, no new integrity errors, save → reopen, one undo | **15** — X360 / PS3 NHL 13, 14, 15, Legacy; PS4 15–21 | **12** — X360 / PS3 NHL 07–12: no `kOtt` home-arena link; the preview refuses with that reason |
| Retired numbers: retire, save → reopen, un-retire on the copy, undo | **26** | NHL 07 (both consoles): no `wcSL` table — refused, nothing changed |
| Draft pick trade to another team, save → reopen, undo | **18** — X360 / PS3 NHL 11–Legacy, PS4 15–21 | NHL 07–10: no picks listed (table absent on the era) |
| One strategy cell in range, save → reopen, undo | **27** | — |

The sweep pins these sets exactly (a change in any anchor's shape fails it). Observed, not
asserted: on PS4 16–21 the second team in list order carries an empty name (a placeholder row),
so the draft-pick row reads "traded to " there; the disclosure filters tenants by team id, never
by name (names collide between placeholder / All-Star rows).

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-api --test team_relationships -- --nocapture` | **1 passed** (27 anchors, 207 s), table in the log |
| `npx vitest run src/components/ArenaSelect.test.tsx src/components/FieldProfile.test.tsx` | **7 passed** (6 new: writes nothing on pick + exact `set_field` args once; who-stays text; refusal disables Assign; pending / failed preview never dispatch and a new candidate re-previews with stale answers dropped; target change drops the candidate; failed write keeps the disclosure) |
| `npx vitest run` (full) | 32 files, 219 passed (at this checkpoint) |
| `npx tsc --noEmit` | clean |
| `export-bindings --check` | in sync (149 roots) |

## 5. Decisions and limitations

- **Disclosure, not a server-held review.** The arena write is one validated FK cell; the
  disclosure is informational (who shares, who stays), so it is bound in the UI to (project, team,
  record, field, candidate) and the write carries exactly the previewed cell and id. A concurrent
  change of tenants between preview and write would only stale the names shown, never the cell
  written; the engine validates the write itself. This differs deliberately from the texture route
  (receipt 01), where the *operation performed* could change.
- "Do not add arena reassignment" in the brief's 2.5 is read as the asset-side venue re-pointing
  (`arenaartid`, R68 withdrawn) — untouched. The DB team → arena relationship is 2.2 scope and was
  already editable; this package adds its disclosure and its parity proof.
- The draft-pick and strategy cases exercise one representative row / cell per anchor; lines,
  captaincy and contracts stay covered by their own earlier suites, not by this sweep.
- Creation / cloning / deletion of teams and leagues: **not implemented**; contract missing (ledger).
- No desktop run: cards V-47 / V-48 in `MORNING-REVIEW.md`.

## 6. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `patch_t1_backend.py`, `patch_t1_frontend.py` | kept until the closeout receipt, then deleted |
| Temp roots `nhlms-teamrel-sweep-*` (81 saved copies per run) | removed on drop; verified absent at closeout |
| Background processes | none from this package at closeout |

## 7. Next action

T2 — Teams 2.6 PS3 NHL 12 / 13 kit member replacement through the archive-member route ([receipt 03](03-t2-ps3-kit-member-replacement.md)).
