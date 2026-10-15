# Developer lane — STATUS (overnight 2026-09-18)

Campaign: [overnight-two-agent-campaign-2026-09-18](../../plans/overnight-two-agent-campaign-2026-09-18.md).
Lane: developer (Studio code/tests/defs/bindings/implementation docs). Nothing committed, staged
or pushed; no game/emulator launched; no live install, anchor or disc image modified.

**Morning entry point:** [MORNING-REVIEW.md](MORNING-REVIEW.md).

> **R2 (18 September, daytime): [OVERNIGHT-DEV-R2](../../validation/overnight-2026-09-18-review-and-r2-dispatch.md)
> executed — receipt [08](08-r2-write-recovery-and-image-ownership.md). F1/F2 corrected with
> regressions; F3 reconciled: Phase 1 is **partially implemented**, per the
> [coverage matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md). The disc-image writer is
> not cleared for live use until Codex reviews receipt 08. Stopped for Codex review; Phase 1
> implementation resumes with matrix packages P1-A … P1-H after that review.**

> **R3 (18 September): [OVERNIGHT-DEV-R3](../../validation/overnight-dev-r2-review-and-r3-dispatch.md)
> executed — receipt [09](09-r3-lifecycle-bound-partial-recovery.md): partial-range recovery is
> bound to the durable record lifecycle (`writing` / new `restoring`); completed records refuse
> old/new mixtures (`mixed`); the API suite uses the deterministic process-probe seam with the
> closed-gate case tested. Codex review pending; Write cards still withheld.**

## Completed (all: implementation complete / automated verified / review pending)

| # | Item | Receipt |
|---|---|---|
| 1 | LEAGUE-PARITY-COMPLETION-R1 F1–F5 (Codex review pending) | [01](01-r1-corrections.md) |
| 2 | Xbox 360 disc-image route: `tdb-xiso`, locate/read, range-patch apply/restore, staging, UI | [02](02-xbox360-iso-route.md) |
| 3 | Research handoffs consumed (traced key tables, source inventory); archive files as containers; NHL 12 PS3 encrypted-ISO finding | [03](03-handoffs-and-archive-containers.md) |
| 4 | Phase 1.2: competition groups, membership operation, schedule browse/validate | [04](04-phase-1-2-membership-groups-schedules.md) |
| 5 | Phase 1.4 league logos (index tables, card, 8_8_8_8 replace); 1.5 audio absence recorded | [05](05-phase-1-4-league-logos-and-1-5-audio.md) |
| 6 | LOC-W2 free text in place (tdb-loc `retext`, pending kind, panel mode) — 1.3 / 2.3 prerequisite | [06](06-loc-w2-free-text-in-place.md) |
| 7 | Phase 2.3 opening slice: team text card (per-team dictionary keys by `artabbr`, edits via the dictionary routes) | [07](07-phase-2-3-team-text.md) |
| 8 | **R2**: write-failure recovery (in-place save + image range writer), image review/operation ownership, Phase 1 coverage reconciliation, batch checksum refresh | [08](08-r2-write-recovery-and-image-ownership.md) |
| 9 | **R3**: lifecycle-bound partial recovery (`restoring` intent, `mixed` refusal, durable re-read), deterministic emulator-gate tests | [09](09-r3-lifecycle-bound-partial-recovery.md) |

Phase 1 implementation coverage (**corrected under R2 F3**; the earlier "done per family" wording
is withdrawn): see the [Phase 1 coverage matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md).
In one line — 1.1 identity edit + rename routes implemented and tested on every family (NHL 07
blocked; the disc-image route awaits Codex review); 1.2 groups/membership/schedule *browse* only
(no schedule editor/import/generation, no rules/mode inventory, no PS4 group names); 1.3 two
bounded dictionary operations (re-point, in-place free text), no growth/locales/consumer trace;
1.4 index + preview + one real XPR2 loose replace (RW4/DDS/GNF, archive placement, non-logo
imagery open); 1.5 no operation, "no separately named Xbox bank" only.

## Claims

- `p1.1-xbox360-iso-member-route` — ready (developer); **R2 F1/F2 corrections applied, Codex review pending**.
- Consumed research (ready): `r0-xbox360-source-inventory`, `p1.1-xbox360-league-code-key-tables`,
  `p1.2-competition-groups-and-schedules`, `p1.3-localization-capacity`, `p1.4-xbox360-league-logo-assets`,
  `p2.7-audio-banks-xbox360` (1.5 part), `p2.3-team-localization-keys` (2.3 slice).
- Delivered by research after the overnight lane closed, **not yet consumed** (Phase 1 inputs):
  `p1.2-ps4-groups-and-consumer-leads` (handoff 17 → matrix P1-C), `p1.4-league-logo-packaging-xplat`
  (handoff 16 → P1-E/F), `r1-corrections-and-chunklzx-vectors` / `codec-chunklzx-framing` (handoff 40 →
  P1-F chunkzip repack). Phase 2/3 inputs held: `p2-team-asset-joins-xbox360`, `p3-portrait-equipment-thumbs-xbox360`.

## Checks (latest full run)

Checkpoint 2 (after receipts 01–02): `cargo test -p studio-domain -p studio-api -p tdb-xiso` 384 passed /
0 failed / 11 ignored; bindings in sync; vitest 63. Since then every new/changed test file passed
individually (see receipts 03–05). **Checkpoint 4 (after receipt 05, 07:05): 398 passed / 0 failed /
12 ignored; bindings in sync** (`local/runs/overnight-development-2026-09-18/checkpoint4-rust.txt`).
Frontend: `tsc` clean, vitest 68/68. Checkpoint 5 (LOC-W2 suites): 31/0/0, bindings in sync.
**Checkpoint 6 (final, after receipt 07, 07:50): `cargo test -p tdb-loc -p studio-domain -p studio-api -p tdb-xiso`
= 416 passed / 0 failed / 18 ignored (ignored = private-fixture and real-source opt-in cases);
bindings in sync (97 roots); vitest 9 files / 68 passed** (`local/runs/overnight-development-2026-09-18/checkpoint6-rust.txt`).
Nothing was changed after this run except this status line.

**R2 checkpoint (18 September, daytime, after receipt 08):** see receipt 08 §Checks —
`local/runs/overnight-development-2026-09-18/r2-checkpoint1-rust.txt` (Rust: 426 passed / 0 failed / 18 ignored), bindings `--check`
(99 roots), `tsc --noEmit`, vitest (10 files / 78), `run-quarantine-tests.ps1` 27/27, batch
`SHA256SUMS.txt` verifies.

## Toolchain / env / restart

- Repo `E:\Repositories\nhl-modding-studio`, branch `feat/phase5-asset-tooling`, dirty tree holds all
  uncommitted implementation — work here, not a HEAD-only worktree.
- Rust stable (`rust-version = 1.96`); `cargo test -p studio-domain -p studio-api -p tdb-xiso`
  (~20 min cold). App: `cd app && npx tsc --noEmit -p . && npx vitest run`.
- Bindings: `cargo run -p studio-api --bin export-bindings -- --check` (99 roots after R2).
- Real-source opt-in tests (read-only): `NHL_STUDIO_XISO=<iso or .big>` (+ `_LISTING`, `_GAME`,
  `_KEY`, `_DONOR`) for `tdb-xiso`/`studio-domain xiso_source`/`studio-api iso_route`;
  `NHL_STUDIO_XBOX_TREE=<X:\…\NHL Legacy\NHL Legacy>` for `league_logos::real_tree…`.
- Quarantine fixtures: `powershell -NoProfile -ExecutionPolicy Bypass -File local\runs\league-parity-batch\tests\run-quarantine-tests.ps1`.
- Disk: E: ~18 GB free at start; no large artifacts added (staging/records in temp dirs, removed).

## Owned artifact manifest / cleanup

- `docs/receipts/overnight-development-2026-09-18/` — receipts 01–07, MORNING-REVIEW, this file.
- `local/runs/overnight-development-2026-09-18/` — small text logs only (`xiso-locate-run{1,2}.txt`,
  `xiso-plan-real-*.txt`, `tree-archive-plan-nhl14.txt`, `checkpoint*-rust.txt`).
- `local/coordination/overnight-2026-09-18/claims/p1.1-xbox360-iso-member-route.json`.
- Temp dirs created by this lane's tests are removed by the tests (the two `xiso_source` unit
  tests leaked ~290 MB per run under `%TEMP%` until 07:50 — fixed to clean up; the leaked dirs
  were deleted). Pre-existing `nhlms-s2c-api-*` / `nhlms-p3-op-*` / `nhlms-league-campaign-*`
  dirs under `%TEMP%` belong to older suites/sessions and were left alone. No processes left
  running after the final checkpoint completes; scratch edit helpers live only in the session
  scratchpad outside the repo.

## Next useful tasks (R2 order: unfinished Phase 1 first; Phase 2 does not expand)

0. **Stop point:** Codex review of receipt 08 (R2 F1/F2/F3). Nothing resumes before it.
1. Matrix packages in order — P1-A schedule editing/import (research 12), P1-C PS4 groups
   (handoff 17), P1-D localization scope (per-title LOC-W2 round trips, locales), P1-E/F logo
   replace parity + archive placement (handoffs 16, 40), P1-B/G/H research asks (rules/mode
   inventory, non-logo imagery, audio consumers) sent to research.
2. Phase 2.3 stays as delivered (team text card); the `artabbr` join on PS4/NHL 07 is a P1-D-adjacent
   check, not a Phase 2 expansion. `p2-team-asset-joins-xbox360` / `p3-…` stay held.
