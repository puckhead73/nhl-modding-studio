# Receipt 07 — Priority 3 (Teams 2.7): the team's name calls (read-only audio binding)

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Read-only against the real Xbox 360 NHL 15 / Legacy trees on `X:`; committed
anchors opened as-is (no write). Claim: `dev-p3-team-name-calls`. Research consumed: 2026-09-21
handoff 07 (`research-r35-gen7-audio-id-bindings`) §1–3 and its `r35-gen7-audio-id-bindings.json`.

## 1. Plain-language summary

On the gen7 titles the play-by-play commentary "knows" a club by a small number in its team row
(the audio id). The research lane established which speech table that number keys: the
`team_ID` parameter of the play-by-play graffiti tables — 207 of the 211 NHL 15 club ids and 206
of 212 Legacy ids have recorded calls there. The Team page now shows a **Name calls** card: how
many recorded samples the play-by-play tags with this club, grouped by how many other parameters
those samples carry, whether every listed sample exists in the sample bank, and how many lines the
keyword index counts as mentioning the club. The four / six clubs without a recording say so
plainly; a club whose audio id is 0 is reported as having no name id (a `(team_ID, 0)` table exists
in the data but is not a club's name). Nothing is decoded or played, and nothing is written — the
reassignment contract of handoff 07 §4 is a proposal and stays unimplemented.

## 2. What users can now do

- Team page → **Name calls (commentary, read-only)** on PS3 / Xbox 360 NHL 11 / 14 / 15 / Legacy
  (Xbox 360 12 / 13 too) with an install root open: the recorded name-call samples of the club,
  their grouping and bank resolution, the keyword mention count (15 / Legacy).
- Outside the measured coverage (NHL 07–10, PS3 12 / 13's archive-only speech, PS4) the card
  states the reason from the catalogue's coverage gate instead of guessing.

## 3. Implementation

| Component | Change |
|---|---|
| `crates/studio-domain/src/speech_catalogue.rs` | `team_name_calls(game, platform, root, audio_id)` — coverage gate shared with the catalogue; parses EVT0 (for the `team_ID` parameter id — refuses with a reason when a title has none), GRF0 and SBKR; lists the `(team_ID, audio_id, n)` tables sorted by tag count with per-table bank resolution, distinct counts, KWD0 mentions when the index is loose; audio id 0 = "no recorded name id" (research counts are over non-zero ids); ids outside u32 refused. `team_audio_id(session, team)` reads `exhibitionteams.audioid` through the metadata name. |
| `crates/studio-domain/src/team_reid.rs` | `team_row` made `pub(crate)` for reuse. |
| `crates/studio-api` (`dto.rs`, `registry.rs`, `commands.rs`, `lib.rs`, `bindings.rs`) | `TeamNameCallsDto` / `NameCallTableDto` (sample ids capped at 64 with the full `sample_count`); `Registry::team_name_calls`; command `team_name_calls`; bindings exported (`TeamNameCallsDto.ts`, `NameCallTableDto.ts`, "in sync"). A title whose team table has no audio column returns an empty listing with the note, not an error. |
| `app/src/components/TeamNameCallsCard.tsx` (+ `.test.tsx`), `pages/TeamDetail.tsx`, `lib/client.ts` | the card (install-root gated like the catalogue, `retry: false`, coverage refusal shown as text), mounted under the Team logos card. |

Wording follows the research contract: samples are what the play-by-play "tags with this team";
"which one plays is runtime"; a club without a table has "no recorded call". Arena audio ids are
**not** listed (their tag is open, Q-R35-1); player-name bindings (`playernames.audioid` /
announcer `playerid`) are Phase 3.5 work for the Players package.

## 4. Regression evidence (developer)

| Case | Asserts |
|---|---|
| `studio-api/tests/speech_catalogue.rs::team_name_calls_resolve_the_research_counts` (`--ignored`, real X360 NHL 15 + Legacy trees; PS3 **and** X360 anchors of each) | for every team: the audio id reads; `team_ID` found; a club with a call has ≥ 1 group, every listed sample resolves in the bank, the 64-cap holds; a club without one carries the "no play-by-play table" note; audio id 0 carries the sentinel note. Totals per anchor: **NHL 15 211 non-zero ids, 207 with a call, 4 without** (`{114, 333, 336, 345}`) on both platforms' anchors; **Legacy 212 / 206 / 6** (`{114, 118, 333, 336, 345, 511}`) — exactly the research's 207 / 211 and 206 / 212. |
| `TeamNameCallsCard.test.tsx` (4 jsdom cases) | args `{projectId, teamId, installRoot}`; group wording, cap "+69", bank and keyword counts; the no-call note; the coverage reason; no install root → nothing rendered, nothing called |

Runs: [p3-team-name-calls.log](../../validation/evidence/overnight-development-2026-09-21/logs/p3-team-name-calls.log)
(1 passed, 151 s — the final of four runs; the first three failed on **my** expectations, not the
join: 208 ≠ 207 because I counted the audio-id-0 club, diagnosed against `r35` `distinct_nonzero`,
then a stale row in the expectation table; each diagnosis is in this receipt, the earlier outputs
were overwritten by the reruns of the same case); bindings "in sync"; `npx tsc` clean;
`npx vitest run` **23 files, 166 passed**
([p3-frontend-full-2.log](../../validation/evidence/overnight-development-2026-09-21/logs/p3-frontend-full-2.log)).
Not run: a launch; a desktop pass.

## 5. Coverage (Phase 2 matrix 2.7 "team audio / name calls", read-only listing)

| X360 07–10 | X360 11–15 / Legacy | PS3 11 / 14 / 15 / Legacy | PS3 12 / 13 | PS4 | Recomp |
|---|---|---|---|---|---|
| refused with reason (older speech system / XML) | impl; **dev-tested on 15 + Legacy** (11–14 implemented via the shared parsers, not exercised tonight) | impl (shared loose files); PS3 anchors dev-tested on 15 + Legacy | refused (archive-only speech) | refused (separate chain) | by identity |

## 6. Open / omissions

- Reassignment of a club's audio id (handoff 07 §4) — not implemented; the cell stays an ordinary
  editable integer with no validation against the `team_ID` key set. Proposal awaits Codex.
- Arena audio ids (chant / city / arena) — not listed (tag open).
- NHL 11–14 not exercised through this listing tonight (parsers are the receipt-05 ones that
  reproduced the research model on all 10 loose trees).

## 7. Cleanup

Scratchpad `new_test.rs` deleted. No temp roots, no processes.
