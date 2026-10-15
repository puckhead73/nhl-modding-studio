# Receipt 06 — Priority 3 (Teams 2.1): team re-identification under the refuse-or-cascade contract

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Committed anchors on temp copies only. Claim: `dev-p3-team-reidentification`.
Research consumed: 2026-09-21 handoff 10 (`research-r37-team-reid-contract`, `ready` — a **policy
proposal**, consumed under the overnight authorization at the risk of rework if Codex changes it).

## 1. Plain-language summary

A team's two numbers — its index (what every roster, jersey, equipment and award row points at)
and its art id (what its logo files and jersey rows are keyed by) — are no longer editable as bare
cells. The Team page now shows an "Identity" card that counts, table by table, what depends on
each number and states the contract: renumbering a team is refused (the counts say why); the art
id changes only by **swapping** it with another existing team, which rewrites both teams' rows in
the jersey and award tables together in one step with one undo, and is refused when a spare-kit id
or a third team sharing the id is involved. Even a raw edit of the art-id cell is refused by the
write layer with the same reason, so no path can leave the dependents dangling.

## 2. What users can now do

- Team page → **Identity (index / art id)**: see the index and art id, every consumer table with
  its row count, the disposition of each change, and other teams sharing the art id.
- **Swap art ids** with another team: one click, one pending step (shown in the pending changes as
  the multi-table edit), undo restores both teams and their jersey / award rows.
- Read the refusal reason when a swap or a bare art-id edit is not allowed.

## 3. Implementation

| Component | Change |
|---|---|
| `crates/studio-domain/src/team_reid.rs` (`5bc36b8272c5b651`) | the dependency graph of handoff 10 §1 as tables (index consumers ×12, art-id consumers ×3); `is_guarded_identity` (`artid` on `exhibitionteams` / NHL 07 `stockteamtable` / `customteamtable`); `team_identity_dependencies` (read-only counts, sharers, dispositions); `plan_artid_swap` with the refusals of §2 (same team, roster copy, no art column, equal ids, spare-kit ids 255 / 1000–1004, a third team sharing either id) and the cell plan: `exhibitionteams.artid` ×2 + the `teamartid` of both teams' rows in `stockteamjerseys` / `customteamjerseys` / `stockawardstable` (only rows carrying the team's own art id follow — a spare-kit id on a jersey row stays). |
| `crates/studio-domain/src/session.rs` (`4726419bbb42be52`) | `set_cells_int_tables` — several tables in **one** transaction (validated + encoded first; one undo entry); `prepare_field_edit` and `set_cells_int_rows` refuse a guarded identity with the contract reason. |
| `crates/studio-domain/src/workspace.rs` (`98ec544197dc2c68`) | `apply_artid_swap` — plan, one canonical transaction, roster mirror of the shared tables, `abort_canonical` on a refused mirror (receipt 02 D7), `record_mirrored`. |
| `crates/studio-domain/src/field_profile.rs` (`47167547f4cb2a42`) | the "Art ID" cell is read-only. |
| `crates/studio-api` (`dto.rs`, `registry.rs`, `commands.rs`, `lib.rs`, `bindings.rs`) | `TeamIdentityDependenciesDto`, `DependencyCountDto`, `ArtIdSwapDto`; `team_identity_dependencies`, `team_artid_swap`; bindings exported. |
| `app/src/components/TeamIdentityCard.tsx` (`b622870a438ee363`), `pages/TeamDetail.tsx` (`bf69cc9000da56ab`), `lib/client.ts` | the card (mutation bound to team + project + generation; owning-project summary refresh; refusal text). |

## 4. Regression evidence (developer)

| Case | Asserts |
|---|---|
| `workspace.rs::artid_swap_cascades_the_jersey_rows_of_both_teams_as_one_mirrored_step` (NHL 14 PS3 nhlng + ROSTER) | dependency counts (roster rows > 0, jersey rows > 0, `refused` wording); the plan's tables; apply = one undo step with `edits == total_cells`; read-back from bytes: the two `kOtt.artid` exchanged, every changed jersey cell belongs to one of the two teams and carries the other's art id, no other row changed; mirrored into the roster's team table; undo → both files byte-exact, clean; refusals (same team, unknown team) change nothing; `set_field` and `set_cells_int_rows` on the art id refused with "swap art ids"; the profile's Art ID cell is read-only |
| `studio-api/tests/team_reid.rs` (Legacy X360 + **NHL 21 PS4**, temp copies) | registry seam: dependencies (PS4: `stockawardstable.teamartid` present); the shared-art refusal on Legacy's placeholder rows carrying art id 0 (a real stock condition the contract refuses); the swap on two unique clubs → `undo_depth 1`, exchanged ids, no sharers; bare `set_field` refused; undo → clean; `save` byte-exact to the anchor |
| `read_matrix.rs` invariant | `editable == updatable && !key && !guarded` on all 27 anchors (the guard is the one documented exception) |
| `TeamIdentityCard.test.tsx` (3 jsdom cases) | contract wording + counts; swap args `{projectId, teamA, teamB}` and the applied summary; a completion landing after a team switch shows nothing on the new team while the owning project's summary still refreshes; refusal text |

Runs: `cargo test -p studio-domain --test workspace --test read_matrix --test field_profile` (13 / 2 / 8
passed); `cargo test -p studio-api --test team_reid` (1 passed, 50 s); bindings "in sync"
([p3-bindings-check.log](../../validation/evidence/overnight-development-2026-09-21/logs/p3-bindings-check.log));
`npx tsc` clean; `npx vitest run` **22 files, 162 passed**
([p3-frontend-full.log](../../validation/evidence/overnight-development-2026-09-21/logs/p3-frontend-full.log)).

## 5. Coverage (Phase 2 matrix 2.1, row "`artid` / `index` re-identification")

| X360 07 | X360 08–15 / Legacy | PS3 08–15 / Legacy | PS4 15–21 | Recomp |
|---|---|---|---|---|
| impl (guard covers the stock / custom table; swap untested on 07 — its thin table has the art column but no jersey tables) | impl+tested, dev (Legacy registry) | impl+tested, dev (NHL 14 workspace + roster mirror) | impl+tested, dev (NHL 21 registry incl. awards) — **art-id moves refused** (vault packs; handoff 10 §2) | by identity |

Index renumbering: **refused everywhere** (as the contract proposes). Art-id move to an unused id
with pre-placed art (gen7): **not implemented** (the file-completeness check per C9 family is a
follow-up; the operation refuses today). Runtime: which screens draw which art after a swap — card
T-4 (MORNING-REVIEW).

## 6. Open / omissions

- The contract is a research proposal; if Codex amends it (e.g. allows moves with pre-placed art
  first), the plan function is the one place to change.
- The pending-changes review shows the swap as its multi-table edit; no dedicated review card.
- Not tested: the roster-only session refusal through the UI; NHL 07 swap (no jersey tables).

## 7. Cleanup

Scratchpad `reid_api.py` deleted at checkpoint. No temp roots (the tests remove theirs), no processes.
