# Receipt 10 — Priority 4 (Players 3.5): a player's recorded names (read-only audio bindings)

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Read-only against the real Xbox 360 NHL 15 tree; the PS3 NHL 15 anchor opened
as-is. Claim: `dev-p4-player-name-calls`. Research consumed: 2026-09-21 handoff 07 §1–2 rows 1–2
(`playernames.audioid` → pbp `player_ID`; `exhibitionplayerbiotable.audioid` → announcer
`playerid` 2163) and handoff 14 (PS4: **no** player-name binding — the PS4 card stays refused).

## 1. Plain-language summary

A roster player's name is spoken by two different systems on the Xbox 360 / PS3 titles: the
play-by-play commentators, who record first and last names as a catalogue (`playernames`) matched
to a player by the spelling, and the rink announcer, who is keyed by a number in the player's bio
row. The Player page now shows a **Recorded name** card with both: which catalogue rows spell the
player's first / last name and how many recorded samples each has (a name can be in the catalogue
with no recording — the card says so rather than offering it), and whether the announcer has a
recording for the bio audio id. Against the research's census of NHL 15 the listing is exact:
5 311 distinct announcer ids in the roster, 4 464 with a recording (84 %), and the only
play-by-play names without a table are the three the research lists (plus id 0, "no recording").
Nothing is decoded, played or written; changing a player's audio id stays an ordinary cell edit
with no validation against the key set (handoff 07 §4 is a proposal).

## 2. What users can now do

- Player page (PS3 / Xbox 360 NHL 11–Legacy with an install root): the recorded first / last
  name rows, their sample counts and bank resolution, the announcer's samples for the bio audio
  id; explicit "no recording" states; the coverage reason on other titles (incl. PS4).

## 3. Implementation

| Component | Change |
|---|---|
| `crates/studio-domain/src/speech_catalogue.rs` | `player_name_calls(session, game, platform, root, player)`: coverage gate; the player's names and bio `audioid` through the metadata names; `playernames` rows matched by `name` text (case-insensitive) against `isfirstname`; pbp `player_ID` tables per matched row; announcer `playerid` tag (from the announcer EVT0 when it names one, else the constant 2163) and its tables for the bio id; audio id 0 = no recording on both sides; notes for every absence; `name_call_tables` shared with the team listing. `team_reid::table_by_name` made `pub(crate)`. |
| `crates/studio-api` (`dto.rs`, `registry.rs`, `commands.rs`, `lib.rs`, `bindings.rs`) | `PlayerNameCallsDto` / `PbpNameEntryDto` (sample ids capped at 64 with the full count); `Registry::player_name_calls`; command `player_name_calls`; bindings exported; the root-count guard raised to 124. |
| `app/src/components/PlayerNameCallsCard.tsx` (+ `.test.tsx`), `pages/PlayerDetail.tsx`, `lib/client.ts` | the card (two columns, per-row grouping, no-recording states, coverage reason, install-root gated). |

## 4. Regression evidence (developer)

| Case | Asserts |
|---|---|
| `studio-api/tests/speech_catalogue.rs::player_name_calls_resolve_the_research_counts_on_nhl15` (`--ignored`, real X360 NHL 15 tree + PS3 anchor roster, 5 534 players, ≈ 8 min) | every player's names equal the roster's; pbp `player_ID` = tag 2; announcer tag 2163; distinct non-zero bio ids **5 311**, with a recording **4 464** (research 84 % row: exact); every listed sample resolves in the bank on both sides; the pbp rows without a table are ⊆ {0, 33465, 33469, 33484}; 4 559 / 5 534 players have at least one recorded name row |
| `PlayerNameCallsCard.test.tsx` (4 jsdom cases) | args `{projectId, playerId, installRoot}`; both sides' wording, cap "+39", the catalogue-without-recording line; both-absent states with the note; the coverage reason; nothing without an install root |

Runs: [p4-player-name-calls.log](../../validation/evidence/overnight-development-2026-09-21/logs/p4-player-name-calls.log)
(1 passed, 488 s; the first run failed on my expectation — audio id 0 rows were not in the
research's keyless list; diagnosed as the sentinel, expectation widened, rerun); the aggregate
default run of the four crates after receipts 07–09:
[p3-aggregate-default-run3.log](../../validation/evidence/overnight-development-2026-09-21/logs/p3-aggregate-default-run3.log)
(run 1 stopped on the bindings root-count guard — 123 ≠ 119, four roots added by receipts 05–07
without the guard being run; fixed and diagnosed there; run 2 died on a **full disk** — my own
build output had grown to 132 GB, 65 GB of stale test binaries older than three hours were deleted
from `target/debug/deps`; run 3 = the clean run, see §4a); bindings "in sync"; `npx tsc` clean;
`npx vitest run` (with the card) below in receipt 11's closing run.

### 4a. Aggregate run 3

`cargo test --no-fail-fast -p tdb-archive-codec -p tdb-archive-big -p studio-domain -p studio-api`:
**501 passed, 0 failed, 44 ignored** (fixture / mount-gated, never counted as passing) — the state
after receipts 07, 08 and the group-label half of 09; the glyph gate and this receipt's code were
compiled after it started and are covered by their own runs above.

## 5. Coverage (Phase 3 matrix 3.5, read-only listing)

| X360 07–10 | X360 11–15 / Legacy | PS3 11 / 14 / 15 / Legacy | PS3 12 / 13 | PS4 | Recomp |
|---|---|---|---|---|---|
| refused with reason | impl; **dev-tested on 15** (X360 tree + PS3 anchor) | impl (shared parsers) | refused (archive-only speech) | refused (no binding — handoff 14) | by identity |

## 6. Open / omissions

- Reassignment validation (handoff 07 §4) not implemented; card R35-AUD-1 is the runtime test of
  the announcer binding.
- One call parses the four speech files afresh (≈ 90 ms); no cache — fine for a page, slow for
  the census test.
- Name matching is exact ASCII-case-insensitive text; a diacritic-differing catalogue spelling
  would not match (none seen on NHL 15's 4 559 matched players, not measured elsewhere).

## 7. Cleanup

Scratchpad `player_calls.rs`, `player_test.rs` deleted. No temp roots, no processes.
