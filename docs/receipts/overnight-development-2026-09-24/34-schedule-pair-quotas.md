# Receipt 34 — schedule generation from user-supplied pair quotas; round-tagged rows are no longer rewritten

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged; nothing staged or committed). Claim `dev-schedule-pair-quotas`. Direction:
[overnight prompt after receipt 31](../../plans/development-20260927-overnight-after-receipt31.md) §3 (the explicit
authorisation for a bounded user-supplied pair-quota workflow). Research consumed: R114 (2026-09-26 handoff 08,
as corrected 03:43), R113. **Not in either review bundle** (built before this package).

## 1. Plain-language summary

- **New:** League → Source schedule → Generate… now has a second mode, **pair quotas**. You pick the
  participants, type how many games each pair plays (a grid with one cell per pair, mirrored; "set every
  pair to N" then adjust), and a running total says how many games you have against the rows the table
  must keep. Preview shows the plan; Apply writes it as one undo step; Save writes it like any edit.
- The generator never changes the table's row count, keeps rows that involve a non-participant (All-Star
  squads), gives each pair equal home and away games (an odd count gives one club one extra home game —
  every such pair is named in the preview), never books a club twice on one generated date, and produces
  the same table for the same input.
- It fills nothing in for you: no stock group format, no NHL "which pairs play five times" rule, no other
  title's pattern. Whether the game plays a generated season correctly is a runtime question (card below).
- **Defect fixed:** the PS4 NHL 18–21 Austrian table marks its final within-group block with `round = 1`
  (R114; meaning unknown). Generation writes 0, so a generation that would replace those rows is now
  **refused** with the reason (both the new mode and the balanced round-robin). Editing single games and
  importing a file already kept each row's round (checked).

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/schedule_edit.rs` | `GenerationConfig::pair_quotas`; `plan_quota_generation` (matrix checks, venue split, greedy matchday packing, season-span checks, kept-row clash warning, notes); `round_tag_refusal` shared by both generators; `merge_by_date` factored out of the round-robin path; options note corrected |
| `crates/studio-api/src/dto.rs`, `registry.rs`; `app/src/bindings/ScheduleGenerationConfigDto.ts` (generated) | optional `pair_quotas` |
| `app/src/lib/client.ts`, `app/src/components/LeagueScheduleCard.tsx` | mode switch, `QuotaGrid`, running total, Preview disabled for a non-numeric cell, plan label |
| `crates/studio-domain/tests/schedule_edit.rs` | two new cases (below); five existing configurations gain `pair_quotas: None` |
| `app/src/components/LeagueScheduleCard.test.tsx` | one new case |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/r34-schedule-quotas.log`)

- `schedule_edit` 14 / 14 (+1 ignored, then run): **`pair_quota_generation_builds_the_user_matrix_and_refuses_impossible_ones`**
  (Legacy X360 `swissschedule`, 12 clubs, 300 rows; a user partition 3 × 4 with 6 within / 4 across = 300):
  valid; deterministic; applied → read back from the bytes: every pair's count = its cell, equal venues, 25
  home games each, no club twice on a date, index = position, dates non-decreasing, first date Sep 11;
  51 matchdays for 50 games per club; **saved to a temporary copy and reopened with identical fixtures**; undo
  restores the stock rows; redo re-applies. Odd cells (7 and 5): venues differ by exactly one, "extra home game
  (2 odd pair(s))" in the notes. Refused with nothing written: asymmetric, non-zero diagonal, 299 of 300 games,
  11 × 12 matrix, duplicate participant, spacing past one season.
  **`generation_refuses_to_rewrite_the_austrian_round_tagged_block`**: PS4 NHL 18 (324 rows, 60 tagged) and
  NHL 21 (270 rows, 50 tagged): a right-total quota matrix and a round-robin request both refuse with the
  count; the table's own CSV re-imports with 0 changed cells.
- Genuine NHL 20 (`NHL_STUDIO_FIXTURES`, `--ignored`): the existing real-install case still passes.
- Existing generation / import / edit cases unchanged and passing; studio-api lib 31; bindings `--write`
  (1 file) then `--check` in sync; vitest 256 (the new case: mode switch hides the round-robin blocker, fill,
  running total "3 of 4 games · 1 short", non-numeric cell disables Preview, the exact matrix sent,
  editing a cell drops the preview, Apply uses the new ticket); `tsc` exit 0.

## 4. Limits

- The matchday packing is greedy: it is always valid (no double booking) but not optimal in length; a very
  uneven matrix may need more matchdays than a hand schedule, and the season-span check then refuses with
  the numbers (shorten the spacing).
- The running total in the panel uses the default participants' kept-row count; after changing
  participants the server's refusal gives the exact figure.
- No stock-shaped option (R113 contracts per exact source) and no Austrian two-phase writer: both remain
  proposals. Tournaments (CHL / Spengler / WC) are not generated.

## 5. Card

- **V-76** (desktop, a build that includes receipt 34, on a copy): X360 / PS3 NHL Legacy → Leagues → Swiss
  league → Source schedule → Generate… → pair quotas → Fill 4 → set four pairs to 6 so the total reads
  "… of 300 games" in green → Preview → Apply → Save → reopen: the table shows your counts; Undo restores it.
  PS4 NHL 19 or 21 → Austrian league → pair quotas with the right total → Preview refuses naming the
  round-tagged rows. Runtime (optional, user's evidence only): start a season in that league.

> **Superseded 27 Sep 2026 (receipt 41 §5):** the recipe above is arithmetically wrong. Four games per pair for
> twelve clubs make 264, and raising four pairs to six makes 272, not 300. Use receipt 41 §5 (three groups of four,
> six within and four across). Receipt 41 also replaces this receipt's by-date merge of kept rows (R123 R2).

## 6. Cleanup

The test's temporary copy (`nhl-studio-quota-*`) is removed by the test; scratch scripts deleted.

## 7. Addendum (06:20) — aligned with research handoff 16 (R123), published during this package

Research published a generation contract (R123, 04:59; **not coordinator-accepted**) while this package ran.
Studio now also enforces its **Q11** (an odd pair count needs an explicit host rule: `lower_slot`, `higher_slot`
or `balance`; unset → refused "choose which club hosts the extra game"; the UI shows the choice only when a
count is odd) and **Q12** (a participant with an all-zero row is refused "would have no games"). The test asserts
that with `lower_slot` the lower slot hosts exactly one more game; the 299-game case became 298 (even). Log:
`r34-schedule-quotas-2.log`; vitest case updated (the host rule is sent with the matrix); bindings regenerated
(`ScheduleGenerationConfigDto.ts`), `--check` in sync. Already met: Q1–Q4, Q6, Q7 (via validation), Q8, R1, R3–R9.
Not implemented (proposal / refused): Q5 partitions (a UI convenience over the matrix), Q9 stock-shaped, Q10 /
R10 phases (round-tagged tables are refused). **Divergence for coordinator review:** R123 **R2** keeps rows that
involve a non-participant at their original *position*; Studio's generators (the existing round-robin and this
one) keep their *values* but merge them into date order and re-index. No shipped league table has such rows
(R123 §4); the NHL All-Star row is the case. Not changed without a decision.

## 8. Next

Inspect research once more; otherwise X360 NHL 09 sound banks (bounded) or the next roadmap gap.
