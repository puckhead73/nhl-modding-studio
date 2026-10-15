# Receipt 09 — OVERNIGHT-DEV-R3: lifecycle-bound partial recovery; deterministic emulator gate in tests

Developer, 18 September 2026. Dispatch:
[overnight-dev-r2-review-and-r3-dispatch](../../validation/overnight-dev-r2-review-and-r3-dispatch.md).
Status: **implementation complete / automated verified / Codex review pending.** Synthetic images
and anchor temp copies only; no game or emulator launched; no live install, anchor or real image
written; nothing staged, committed or pushed. The image Write cards stay withheld. No feature
expansion. Stopped for review.

## Plain-language summary

Codex showed that R2's recovery was too eager: once a range had been written and restored, any
later edit that happened to mix old and new byte values at their positions was still treated as
"our interrupted write" and silently put back to the old bytes. That is fixed by binding partial
recovery to the record's **durable lifecycle**. A byte mixture is only recoverable while the
record on disk says a mutation of that range is unfinished — `writing` (an apply in flight) or the
new `restoring` (a restore in flight, persisted before the first byte goes back). On a completed
record (`applied` / `restored`) the same mixture is classified `mixed`, refused with the reason,
and left untouched; arbitrary foreign bytes stay refused as before. Every classification and every
restore decision now re-reads the record from disk first, so a stale in-memory copy (for example
one still saying `writing` after the durable record completed) cannot authorize anything. An
interrupted restore is itself recoverable through the `restoring` record. The panel offers Restore
only for `replacement` and `partial`; `mixed` is shown as "changed after this write — not
restorable here".

The synthetic API flow no longer depends on the machine's process listing: the main-flow test
uses the deterministic probe seam, and a new test proves the gate fails closed when enumeration
is unreadable (apply and restore refused, nothing written or restored) and passes when readable.
The production gate is unchanged: a live listing failure still refuses.

## F1 — completed record authorized partial recovery — **fixed**

`crates/studio-domain/src/iso_patch.rs`
- `RecordStatus::Restoring` added (`RecordStatus::is_unfinished()` = `Writing | Restoring`).
- `classify`: prior / replacement by identity as before; a prior-or-replacement byte mixture is
  `Partial` **only** when the durable status is unfinished, otherwise the new `RangeState::Mixed`;
  anything else `Unknown`.
- `durable(record)` re-reads `record.json`; `state`, `status` and `restore` decide on it, never on
  the caller's object.
- `restore` (`restore_with` for fault injection): `Prior` → verified no-op; `Replacement` /
  `Partial` → write `restoring` to the record, write the verified backup back, verify, write
  `restored`; `Mixed` → refused naming the completed status ("its write completed, so the mixture
  is a later change"); `Unknown` → refused naming both identities. Backup identity checks and
  idempotence unchanged.
- `iso_staging::ApplyOutcome::describe` lists `mixed` with the unknown ranges (cannot occur right
  after an apply, listed for completeness). DTO docs updated (`restoring`, `mixed`); bindings
  re-rendered (comments only, 99 roots).
- Panel: Restore offered for `replacement` / `partial` only; `mixed` rows carry the explanation.

## Test determinism — **fixed without weakening the gate**

`crates/studio-api/src/registry/iso.rs`: the test probe now returns `Result<Vec<String>, String>`
and flows through the same `map_err` as the live listing. `tests/iso_route.rs` main flow sets a
readable empty probe; `r3_an_unreadable_process_enumeration_refuses_writes_and_restores` covers
the closed gate.

## Regressions (all new cases synthetic)

| Case | Asserts |
|---|---|
| `iso_patch::r3_f1_a_completed_record_never_authorizes_partial_recovery` | Codex's reproduction (apply → restore → later `BAAA`-style edit): `state == Mixed`, `restore` refused, bytes untouched, record stays `restored`; the same after a successful apply alone (`applied`) |
| `iso_patch::r3_f1_a_stale_in_memory_writing_record_does_not_authorize_recovery` | a `Writing` object from `prepare` retained across a completed apply + restore: durable decision wins (`Mixed`, refused) |
| `iso_patch::r3_f1_an_interrupted_restore_is_recorded_and_recoverable` | restore cut short after 900 bytes → durable `restoring`, `Partial`, next restore `RestoredPartial`, image byte-identical, then `AlreadyPrior` |
| `iso_patch::r2_f1_an_interrupted_range_write_is_recorded_classified_and_restored` (extended) | genuine interrupted apply still recovers; the same mixture on the now-completed record is `Mixed` and refused |
| `iso_patch::restore_refuses_unknown_bytes_and_keeps_the_backup` (unchanged) | foreign bytes untouched |
| `iso_route::r3_an_unreadable_process_enumeration_refuses_writes_and_restores` | unreadable listing refuses apply (no record, reservation released) and restore; readable listing proceeds |
| `DiscImagePanel.test.tsx` "offers Restore for replacement and partial ranges only…" | two Restore buttons for four records (`replacement`, `partial`, `mixed`, `unknown`); `mixed` explanation shown |

## Checks (`local/runs/overnight-development-2026-09-18/r3-checkpoint-rust.txt`)

| Check | Result |
|---|---|
| `cargo test -p studio-domain --lib -- iso_ in_place_save` | 23 passed, 0 failed, 1 ignored (real-image opt-in) |
| `cargo test -p studio-domain --test in_place_save` | 7 passed |
| `cargo test -p studio-api --test iso_route` | 5 passed, 0 failed, 1 ignored — deterministic, no live process listing |
| `cargo test -p studio-api --lib bindings` | 25 passed |
| `export-bindings --check` | in sync (99 roots) |
| `npx tsc --noEmit` | clean |
| `npx vitest run` | 10 files / 79 passed |

Not re-run: the full four-crate checkpoint (R2's 426/0/18 stands; only `iso_patch.rs`,
`iso_staging.rs` (one describe filter), `registry/iso.rs`, `dto.rs` comments and tests changed).

## Changed-source identities (SHA-256, first 16 hex)

| File | SHA-256 |
|---|---|
| `crates/studio-domain/src/iso_patch.rs` | `9a9e37b9fec1f0b3…` |
| `crates/studio-domain/src/iso_staging.rs` | `6ff6f2b6752bf639…` |
| `crates/studio-api/src/registry/iso.rs` | `2f6f258574b33662…` |
| `crates/studio-api/src/dto.rs` | `84ac79493ac0a0d2…` |
| `crates/studio-api/tests/iso_route.rs` | `f2305e3a37e7b55f…` |
| `app/src/components/DiscImagePanel.tsx` | `525c74876e9388ad…` |
| `app/src/components/DiscImagePanel.test.tsx` | `e0a285267670ae12…` |
| `app/src/bindings/IsoRecordDto.ts` / `IsoAttemptedDto.ts` | `7596b53c6dcc77fe…` / `b0527dc226782a28…` |

## Coordinator decisions applied

1. Platform-wide emulator exclusion kept; enumeration failure fails closed (now tested).
2. Reviews stay in memory; supersession, consumption and project close release them (unchanged).
3. No in-place-save journal claimed; the process-local limitation stays explicit in
   `in_place_save.rs` docs and receipt 08.

## Cleanup

Tests remove their temp dirs (`%TEMP%` holds no `nhlms-iso*` / `nhlms-inplace*` after the run);
no processes left running; no scratch added to the repo beyond this receipt and the checkpoint
log; edit specs live in the session scratchpad outside the repo.
