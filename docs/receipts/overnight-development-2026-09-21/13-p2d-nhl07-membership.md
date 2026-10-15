# Receipt 13 — Priority 2-D (Leagues 1.1 / 1.2, NHL 07): competition membership on the stock team table

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Committed NHL 07 anchor opened as-is (in memory). Claim: `dev-p2d-nhl07-membership`.
Research consumed: 2026-09-21 handoff 13 (`research-r39-nhl07-competition`) §1 and §3.

## 1. Plain-language summary

NHL 07 keeps a team's league, conference and division on the stock team row itself; the table
Studio treated as "the team table" is only a selector pointing at those rows. Until tonight the
Team page refused league membership on NHL 07 outright ("the league code lives on a thin index").
Following the research, membership on NHL 07 now edits the stock row the selector points at, the
NHL's conference / division numbers carry the 2006-07 names (Eastern / Western; Northeast,
Atlantic, Southeast, Central, Northwest, Pacific — inferred from the membership, marked candidate),
a league move states that the schedules rank teams inside their league and are not rewritten,
and the two All-Star rows refuse to leave league 0 because the NHL schedule tables address them
directly. Other leagues' numbers stay raw. Nothing about schedule generation changed.

## 2. What users can now do

- NHL 07 Xbox 360 (and the research-only PS3 copy): Team page → League membership card shows the
  current league / conference / division with names for NHL clubs, moves a team between leagues,
  edits the local numbers, undoes; the team header shows the conference / division labels.

## 3. Implementation

| Component | Change |
|---|---|
| `crates/studio-domain/src/membership.rs` | `target()`: on a title whose league table is `stockteamtable` (`HqzU`), resolve the selector row's `(table, teamid)` to the stock row (`index` key) and edit there; custom-table rows refused. `state()` (local bits): labels from the vocabulary's `[[groups]]` under the league code, else the raw number. `plan()` (NHL 07): All-Star rows 30 / 31 refuse a league change; a league move carries the rank note. `options()` notes for NHL 07. |
| `defs/leagues/gen6-nhl07.toml` | `[[groups]]` for `rasz` (conference 0 Western / 1 Eastern) and `xgeX` (divisions 0–5) under league 0, candidate confidence, sourced from handoff 13 §1's membership. |
| `crates/studio-domain/src/view.rs` | the team view's conference / division labels fall back to the local-bit fields, read from the stock row on the thin-selector title. |

## 4. Regression evidence (developer)

| Case | Asserts |
|---|---|
| `studio-domain/tests/membership.rs::nhl07_membership_edits_the_stock_row_with_labelled_local_bits` (replaces the "refused explicitly" case) | Boston: no blocker, `LocalBits`, league 0, conference 1 "Eastern", division 0 "Northeast"; Chicago "Western" / "Central"; the team view carries the labels; Canada (league 1) shows the raw "untraced" number; a local-bit write (3 cells on `HqzU`) reads back as "Atlantic"; a league move to the DEL carries the rank note and reads back; `EAS` (stock row 30) refuses a league change with the All-Star reason; two undos → labels back, clean |
| `membership.rs` (the other 4 cases), `leagues.rs` (11), `read_matrix.rs` (2), `field_profile.rs` (8) | unchanged behaviour on every other anchor |

Runs: `cargo test -p studio-domain --test membership` (5 passed; the first run failed on the
generic local-bits note — the NHL 07 note added, diagnosed); `--test leagues --test read_matrix
--test field_profile` (11 / 2 / 8 passed).

## 5. Coverage (Phase 1 matrix 1.1 / 1.2, NHL 07 row)

| X360 07 | PS3 07 (research-only) |
|---|---|
| **impl + dev-tested**: browse with labels, league move, local-bit edit, All-Star guard; schedule tables not rewritten (noted) | same code path (identical anchors per the research); not exercised |

## 6. Open / omissions

- Schedule generation for NHL 07 (the NHL 08 era row + 58 / 29-game quotas, handoff 13 §2–3):
  Studio's generator is a plain round-robin (`GenerationConfig.rounds`) — no per-era divisional
  quota table exists in Studio to add a row to; a quota-based generator is a separate package.
- The division names are inferred from the 2006-07 membership, not traced from the game's
  dictionary; they carry `candidate` confidence.
- The Leagues page's league-level structure view (conferences / divisions per league) is unchanged
  for NHL 07 — the labels surface on the team's membership card and header.

## 7. Cleanup

No scratch files. No temp roots, no processes.
