# Receipt 08 — OVERNIGHT-DEV-R2: write-failure recovery, image review/operation ownership, Phase 1 coverage reconciliation

Developer, 18 September 2026 (daytime). Dispatch:
[overnight-2026-09-18-review-and-r2-dispatch](../../validation/overnight-2026-09-18-review-and-r2-dispatch.md).
Status: **F1/F2 implementation complete / automated verified / Codex review pending; F3
reconciled.** No game or emulator launched; no live install, anchor or real disc image written
(every real-file check stayed read-only or on temp copies; the R2 regressions run on synthetic
images and anchor copies only). Nothing staged, committed or pushed. NHL21-E1 stays accepted and
closed. **Stopped here for Codex review**; Phase 1 implementation does not resume before it.

## Plain-language summary

Codex found that both of Studio's "write it, then check it" engines could tell you *nothing
changed* when something had. The in-place save renamed a file into place and only then read it
back; if that read-back failed or found the wrong bytes, the file it had just replaced was left
out of the rollback and out of the report. The disc-image range writer had the same shape: once
the bytes were going into the image, a failure part-way (a cut-off write, a flush error, a failed
final record) reported "nothing had been written" or "the image is as it was", and its Restore
refused the half-written range as unknown. Both now treat a destination as *attempted* from the
moment its mutation starts: every attempted file or range is put back from its verified backup
and classified, "nothing changed" is only said after all of them verified back to their original
bytes, and a partly written range is recognised as *in transit* (every byte is either the old or
the new byte at its position) and recovered from the record — foreign bytes are still never
overwritten. The image writer records its intent (`writing`) on disk before the first byte
changes, keeps a copy of both the old and the new bytes, and returns the records of a failed
write instead of hiding them behind an error. No restart detection is claimed for the in-place
save: its prior bytes live only in memory, and the receipts now say so.

Codex also found that the image panel showed one plan and wrote another: "Yes, write" sent only a
folder path, the server re-planned from whatever was on disk, and nothing tied the write to the
project, its revision, or other operations on the same image. Now every image command names the
project; a plan is a server-held *review* (the exact patches with their bytes, the staged files'
identities, the manifest snapshot and the workspace revision) and the apply names the review it
applies. Before any byte changes the apply refuses a review that is stale, superseded, another
project's, taken while the project was dirty, or whose project was edited since; refuses when the
staged bytes or manifest changed after the review; refuses when the platform's emulator is
running; and then holds the image and the staged files under the same reservation the copied-
target engine uses, so a second staging root for the same image (however spelled), a restore, a
save, a new review, a close or opening the held image again are refused with the holder named
until it finishes. A reused staging folder is validated as a whole (title, platform, metadata
copy, staged files, image length), and a restore only accepts a records directory that this
staging wrote for this image and its recorded ranges. The panel binds its confirm to the review
and withdraws it when the project moves; both image panels now have component tests.

Codex's third finding was that the overnight status called Phase 1 "implementation complete"
while its own receipts showed partial coverage. That wording is withdrawn. The canonical
statement is now an operation-by-target matrix —
[phase-1-coverage-matrix-2026-09-18.md](../../validation/phase-1-coverage-matrix-2026-09-18.md) —
with one of six levels per cell and the remaining packages (P1-A … P1-I) that would close the
phase, each naming the research handoff it reuses. STATUS, the morning review, the roadmap's
Phase 1 section and receipts 01/02/04/05/06 point at it (the receipts keep their text and carry a
correction note). The morning cards are regrouped — **A** existing candidate identity tests, **B**
new surfaces to look at in Studio without placement, **C** withheld — and every card that told
the user to press the Disc-image panel's Write is withheld until this receipt is reviewed. The
Phase 2.3 team-text slice is untouched. The batch package's stale checksum inventory is refreshed
after re-running the 27 quarantine fixture cases.

## Findings dispositions

### F1 — both writers omitted the failing, potentially mutated destination from rollback — **fixed**

`crates/studio-domain/src/in_place_save.rs`
- `place` is split into `stage_temp` (temp file only — the destination is untouched, so a failure
  here keeps it out of the recovery set), `rename_into_place` and `verify`. `publish` pushes the
  destination onto `attempted` **before** the rename; every later failure — the rename itself,
  the new `Event::AfterRename` / `Event::BeforeReadBack` injection points, the read-back I/O, the
  identity mismatch, `AfterPublished` — rolls back and classifies every attempted destination, the
  failing one included. `Outcome::RolledBack` (and `nothing_changed()`) is produced only when all
  of them verified back; otherwise `Split` names the failing destination among `holding_new` /
  `restored` / `unknown`.
- Module docs: the restart-detection sentence is replaced by an explicit non-claim (prior bytes
  are in memory only; the `.tdb-save-tmp` sibling is the only durable trace).

`crates/studio-domain/src/iso_patch.rs` (record schema `iso-range-patch/2`; v1 records still read)
- `prepare` (before the first image byte changes): boundary re-check, record dir, `prior.bin`
  verified, **`replacement.bin`** verified, `record.json` with the new **`writing`** status —
  durable mutation intent.
- `mutate`: range write → flush → read-back → final record, with `Hooks` injection points
  `BeforeRangeWrite`, `BeforeFlush`, `BeforeReadBack`, `BeforeFinalRecord` and
  `truncate_range_write(entry) -> Option<usize>` (an interrupted write). Any failure after the
  write started runs the bounded restore immediately and returns
  `ApplyResult::Failed { record, cause, state, restore_error }`; `apply_with` never loses the
  record. `apply` (no hooks) keeps the `Result` shape for domain tests.
- `classify` / `state` / `status`: `prior` / `replacement` / **`partial`** (every byte is the prior
  or the replacement byte at its position, both recorded strings verified against the record's
  identities) / `unknown`. `restore` recovers `partial` (`RestoreOutcome::RestoredPartial`) and
  still refuses `unknown` with both identities named.

`crates/studio-domain/src/iso_staging.rs`
- `apply_planned(planned, records_dir, hooks) -> Result<ApplyOutcome>`: `Err` only before any
  byte changed (nothing to write); otherwise `Committed` / `RolledBack` / `Split` with
  `attempted: Vec<AttemptedRange { role, entry_path, record, state, restore_error }>` — the failing
  range first (when it got as far as a record), then the earlier ranges newest first, each restored
  and classified. `describe()` gives the plain-language account; `nothing_changed()` is
  `RolledBack` only. `apply(root)` (plan-now convenience) and `new_records_dir` kept.
- `validate_records_dir(root, dir)`: an apply directory under `<root>/records/`, every record naming
  this staging's image (canonical) and one of its located ranges (member, entry, length).
- `records()` reports `partial` for interrupted ranges, failed applies included.

### F2 — the reviewed image plan was not the plan applied; operations bypassed workspace coordination — **fixed**

`crates/studio-api/src/registry/iso.rs`, `registry.rs` (`Registry.iso: Mutex<IsoState>`),
`copied_target.rs` (`reserve` / `ReservationGuard` made `pub(super)`), `error.rs` (`ApiError::DiscImage`).
- **Project binding.** `iso_open` records `IsoBinding { root, manifest }` for the new project and
  refuses to open a staging of an image that is currently held; `close_project` forgets the
  binding and its reviews. `iso_plan / iso_status / iso_apply / iso_restore / iso_records` take
  `project_id`; a project not opened through the image route is refused.
- **Server-held review.** `iso_plan(project_id)` → `IsoReview { revision, dirty, manifest snapshot,
  planned (patches with replacement bytes) }` under a `review_id` (a newer review supersedes the
  older one for that project); the DTO carries `review_id`, `project_id`, `workspace_revision`,
  `dirty`.
- **Apply = the review.** `iso_apply(project_id, review_id)` consumes the review and refuses, before
  any byte changes: unknown/consumed/superseded review; another project's review; a review taken
  while dirty; workspace dirty now; workspace revision moved (checked before and again under the
  reservation); manifest changed since open or since review; any staged file whose identity differs
  from the reviewed one; the platform's emulator running (`tasklist` on Windows / `ps` elsewhere,
  the `deploy.ps1` name mapping: xenia/xenia_canary, rpcs3, shadPS4, nhllegacy; an unreadable
  process list refuses rather than passing). It then applies exactly `review.planned` through
  `apply_planned`; the DTO reports `outcome` (`committed` / `rolled-back` / `split`), `message`,
  `records`, `attempted` (state + restore error per range), `failed`, `cause`.
- **Serialization by destination identity.** Apply and restore reserve the image and every staged
  file under `reservation_key` (canonical directory + leaf, case-folded where the file system
  folds) plus every open workspace on them, with holder `disc-image write of review N (project P)`
  / `disc-image restore of NNNN (project P)`. While held: a second staging root/project of the
  same image (spelled differently), a restore, a save/reload, a new review, `close_project`
  (`busy`) and `iso_open` of the image are refused naming the holder. `iso_status` reports the
  binding, `in_flight`, pending reviews and `cancellable: false` with the note that a started
  range write completes or is put back and cannot be cancelled.
- **Reused staging validation** (`iso_staging::stage`): same image *and* same title/platform,
  staged files present, metadata copy present and identical to the metadata being opened with,
  image length unchanged; otherwise refused with the reason. **Restore path validation**:
  `validate_records_dir` (above).
- **Panel** (`app/src/components/DiscImagePanel.tsx`): project-bound calls; the review is shown
  (`review N at revision …`); the write button is enabled only when the review is current (this
  project, this revision, not dirty on either side) and nothing holds the image; the confirm is
  bound to `{ reviewId, revision }` and withdrawn when the project moves; `Yes, write` sends that
  review id; failed applies render every attempted range with its state and restore error;
  `partial` records offer Restore; the status note is shown. `IsoStagedDto.project_id` added.

### F3 — Phase 1 closure exceeded the delivered scope — **reconciled**

- New canonical [Phase 1 coverage matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md):
  per operation × target family, levels *impl+tested / impl, test missing / not impl / research
  unknown / n/a (evidence) / blocked*, user review pending throughout; remaining packages P1-A
  (schedule editing/import/generation), P1-B (rules/mode inventory — research ask), P1-C (PS4
  groups, handoff 17), P1-D (localization scope), P1-E (logo replace parity), P1-F (logo archive
  placement, handoffs 16/40), P1-G (non-logo imagery — research ask), P1-H (audio consumers),
  P1-I (runtime review).
- Roadmap Phase 1 section: coverage-statement paragraph pointing at the matrix; active priority =
  unfinished Phase 1; the overnight "complete" wording superseded.
- STATUS: closure paragraph replaced; claims updated (research handoffs 16/17/40 not yet consumed);
  R2 checkpoint; next tasks reordered (stop point first, then P1-A…).
- MORNING-REVIEW: correction banner; the "new tonight" bullets marked partial/open; cards regrouped
  A/B/C; every Disc-image Write withheld (Legacy image + archive panel, Recompiled archive panel,
  NHL 15/14/13/12/11/10 images); restoration summary adjusted.
- Receipts 01/02/04/05/06: correction notes at the top, historical text unchanged.
- Phase 2.3 (receipt 07, `team_text.rs`, `TeamTextCard`) untouched.

### Batch helper/test checksum inventory — **refreshed**

`local/runs/league-parity-batch/tests/run-quarantine-tests.ps1` re-run: **27 passed, 0 failed**
(`tests/quarantine-results.txt` rewritten). `SHA256SUMS.txt` regenerated in place (three entries
had drifted: `quarantine.ps1` `adad74d1…→7eaee3d2…`, `tests/quarantine-results.txt`
`407df6c2…→7c8cf826…`, `tests/run-quarantine-tests.ps1` `601cc77b…→0586729b…`); `sha256sum -c`
verifies every line.

## Regressions added (all synthetic images / anchor temp copies; no real image touched)

| Case | Where | Asserts |
|---|---|---|
| `r2_f1_a_verification_mismatch_after_the_rename_puts_the_destination_back` | `in_place_save.rs` | Codex's probe: wrong expected identity → `RolledBack`, `nothing_changed()`, destination bytes = prior |
| `r2_f1_faults_after_rename_and_at_read_back_of_a_second_file_roll_back_both` | `in_place_save.rs` | `AfterRename` / `BeforeReadBack` on the second file → both files back to prior, no temp files |
| `r2_f1_the_failing_destination_is_classified_when_its_own_rollback_fails` | `in_place_save.rs` | failing file named in `holding_new`, earlier file `restored`, `!nothing_changed()` |
| `r2_f1_an_interrupted_range_write_is_recorded_classified_and_restored` | `iso_patch.rs` | truncated write → `Failed{state: Prior}` after the immediate restore, image byte-identical; crash simulation: `writing` record, `partial` state, `RestoredPartial`; one foreign byte → `unknown`, refused |
| `r2_f1_faults_at_flush_read_back_and_final_record_return_the_record_and_restore` | `iso_patch.rs` | each fault returns the record with status `restored`, range `prior`, image byte-identical |
| `r2_f1_a_second_range_failure_rolls_back_both_ranges_and_keeps_both_records` | `iso_staging.rs` | second-range faults (range write, flush, final record, partial) → `RolledBack` with both ranges attempted/`prior`, image byte-identical, both records listed |
| `a_second_patch_failure_restores_the_first` (updated) | `iso_staging.rs` | refused second range (record dir exists) → `RolledBack` with the first range put back |
| `r2_f2_reusing_a_staging_validates_title_platform_metadata_files_and_image_length` | `iso_staging.rs` | title/platform/metadata/missing file/image-length refusals |
| `r2_f2_a_stale_or_foreign_review_and_a_running_emulator_are_refused_before_mutation` | `studio-api/tests/iso_route.rs` | dirty after review; review taken dirty then saved; staged bytes changed after review (no record created); another project's review; `xenia_canary` running (apply) and `xenia` running (restore) refused; other platforms' emulators do not block; image unchanged at every refusal |
| `r2_f2_conflicting_operations_are_serialized_by_image_identity_while_an_apply_runs` | `iso_route.rs` | apply parked inside the reservation: second project/root of the same image (alias spelling) refused naming the holder; save, new review, `close_project` (`busy`), `iso_open` refused; after release the second plan re-verifies the changed range; a restore is refused while the other project's apply runs; both restore to byte-identical |
| `r2_f1_a_failed_apply_returns_its_records_and_the_image_is_as_it_was` | `iso_route.rs` | through the registry (`IsoFault` seam): truncated second range → `outcome: rolled-back`, two attempted ranges `prior`, records listed by `iso_records`; fault at the first range's final record → same contract; then a clean commit + restore |
| main flow (updated) | `iso_route.rs` | project-bound plan/status; dirty review refused server-side; superseded/consumed review refused; foreign records dir refused; closed project forgets its binding |
| `DiscImagePanel.test.tsx` (9) | app | + confirm withdrawn on dirty/revision move and rebound to the newer review; dirty/foreign-revision reviews disabled; busy status disables write/restore; rolled-back and split outcomes render every attempted range, `partial` offers Restore |
| `IsoOpenPanel.test.tsx` (6, new) | app | inspection bound to the image field; no-database image cannot open; open arguments; late open/inspection dropped after the field changed; server refusal shown |

## Checks

| Check | Result |
|---|---|
| `cargo test -p tdb-loc -p studio-domain -p studio-api -p tdb-xiso` | **426 passed / 0 failed / 18 ignored** (67 suites; ignored = private-fixture and real-source opt-in cases) — `local/runs/overnight-development-2026-09-18/r2-checkpoint1-rust.txt` |
| `cargo test -p studio-domain --lib in_place_save` / `iso_` | 7 / 13 passed |
| `cargo test -p studio-api --test iso_route` | 4 passed, 1 ignored (real-image opt-in) |
| `cargo run -p studio-api --bin export-bindings -- --check` | in sync, 99 roots (+`IsoStatusDto`, `IsoAttemptedDto`) |
| `cd app && npx tsc --noEmit` | clean |
| `cd app && npx vitest run` | 10 files / 78 passed |
| `run-quarantine-tests.ps1` | 27 passed, 0 failed |
| `sha256sum -c local/runs/league-parity-batch/SHA256SUMS.txt` | all OK |

Not run (known-failing baseline, C0.2): `cargo fmt --check`, `cargo clippy -D warnings`.

## Changed-source identities (SHA-256, first 16 hex)

| File | SHA-256 |
|---|---|
| `crates/studio-domain/src/in_place_save.rs` | `229fc74cae5e49cf…` |
| `crates/studio-domain/src/iso_patch.rs` | `a5e58a2723bbe151…` |
| `crates/studio-domain/src/iso_staging.rs` | `1cc301104c9f213d…` |
| `crates/studio-api/src/registry/iso.rs` | `b3592a27a2a45549…` |
| `crates/studio-api/src/registry.rs` | `d7122fd0fcb062d5…` |
| `crates/studio-api/src/registry/copied_target.rs` | `f66b2740328a453e…` |
| `crates/studio-api/src/error.rs` | `16efc09b214403f5…` |
| `crates/studio-api/src/dto.rs` | `5642a7652bf467f1…` |
| `crates/studio-api/src/commands.rs` | `91160f84554834d9…` |
| `crates/studio-api/src/lib.rs` | `229f30de41210f73…` |
| `crates/studio-api/src/bindings.rs` | `2bd35aec3ac4dd03…` |
| `crates/studio-api/tests/iso_route.rs` | `776f9ae7033ce8a2…` |
| `app/src/components/DiscImagePanel.tsx` | `7e1e8a36cb721883…` |
| `app/src/components/DiscImagePanel.test.tsx` | `764767e5572bd371…` |
| `app/src/components/IsoOpenPanel.test.tsx` (new) | `6880c5f6bf69ca9d…` |
| `app/src/lib/client.ts` | `12676e7b83c79b96…` |
| `app/src/bindings/IsoPlanDto.ts` / `IsoApplyDto.ts` / `IsoStatusDto.ts` (new) / `IsoAttemptedDto.ts` (new) / `IsoStagedDto.ts` / `IsoRecordDto.ts` / `IsoRestoreItemDto.ts` | `eec57131f1c6187f…` / `42482053351aa310…` / `780f6475e9e2804d…` / `7a2e1704379a1b99…` / `9da2ccde8fb922d4…` / `8df533d623896f96…` / `c90ebc74f2129a36…` |
| `local/runs/league-parity-batch/SHA256SUMS.txt` | `a3d274b391cca245…` |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` (new) | `56c2fe5d0c31214f…` |

Docs changed (no hash needed): `docs/plans/entity-editing-parity-roadmap.md` (Phase 1 coverage
statement), `docs/receipts/overnight-development-2026-09-18/{STATUS.md, MORNING-REVIEW.md, 01, 02,
04, 05, 06}` (correction notes), this receipt.

## Corrected per-target readiness (short form; the matrix is authoritative)

| Route / operation | Ready for user review | Withheld / open |
|---|---|---|
| 1.1 identity rename on the existing candidate rows (PS4 16–20 archives, NHL 15 PS4 base/patch, NHL 19 PS4 quarantine, Legacy Xbox loose/packed via `deploy.ps1`, Legacy PS3 hdd0/disc, PS3 08–15 loose, Recompiled `deploy.ps1` row) | yes — group **A** cards, identities unchanged | — |
| Disc-image / Studio archive-panel writes (Legacy image + `cacheboot.big`, Recompiled `cacheboot.big`, NHL 10–15 images) | **no** — withheld (group **C**) until this receipt is reviewed | F1/F2 Codex review |
| 1.2 groups/membership/schedule browse (all families but NHL 07) | Studio-only look (group **B**), no placement | editor/import/generation, PS4 names, rules/mode |
| 1.3 re-point + in-place free text | Studio-only, and the same runtime tile check as a rename on A rows | locales, growth, consumer trace |
| 1.4 logo index/preview; loose XPR2 replace on Legacy Xbox | Studio-only; optional loose replace with `.bak` | RW4/DDS/GNF replace, archive placement, non-logo imagery |
| 1.5 | nothing to review | audio consumers |
| 2.3 team text card | Studio-only look (preserved, not expanded) | — |

## Cleanup

- Temp dirs: the R2 tests remove their own (`nhlms-inplace-*`, `nhlms-isopatch-*`, `nhlms-isostage-*`,
  `nhlms-isoroute-*`). Runs that were killed while the serialization test was still deadlocked (two
  background runs and the debugging runs before the test-side fix) left `nhlms-isoroute-serial-*`
  dirs under `%TEMP%`; removed after the final checkpoint (see below). Nothing else created outside
  the repo except the session scratchpad's edit specs.
- Debug `eprintln!` lines added while locating the test deadlock were removed from `iso.rs` and
  `iso_route.rs` (grep `DBG` = 0).
- No processes left running after the final checkpoint.
- Repo additions are small text files only (no binaries, no fixtures); disk unchanged in practice.

## Unresolved questions (for Codex)

1. The running-emulator exclusion lists processes by executable stem; it cannot tell *which* image
   Xenia has open. Refusing any running Xenia for any Xbox image is deliberate and conservative —
   acceptable, or should it be advisory when the image path is not among the emulator's open
   handles (would need a handle probe)?
2. `IsoReview` keeps the replacement bytes in memory (a few MB per staged file). Fine for the
   two-file staging; say if a disk-backed review is preferred.
3. The in-place save's split state remains process-local by design (no journal). If a durable
   in-place journal is wanted, that is a new package, not a correction.
