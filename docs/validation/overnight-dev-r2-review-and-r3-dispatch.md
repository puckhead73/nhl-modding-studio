# OVERNIGHT-DEV-R2 review — one remaining recovery correction

18 September 2026. Continue the same developer session. R2's original post-write accounting regressions and frontend regressions pass independently; one newly introduced partial-recovery defect prevents clearance of the image writer. Phase 1 coverage reconciliation is accepted as planning status, not implementation/runtime closure. NHL21-E1 remains accepted and closed.

## Checks and reproduced finding

Codex independently ran `cargo test -p studio-domain --lib r2_f1`: 6 passed. Ran the DiscImagePanel and IsoOpenPanel vitest files: 15 passed. Inspected the attempted-write recovery, server-held image review, reservation and restore paths. The broader receipt results remain developer evidence. A separate temporary integration probe reproduced the following on a four-byte disposable file; no game inputs were touched:

Also reran `cargo test -p studio-api --test iso_route`: 3 passed, 1 failed, 1 ignored. The main synthetic flow failed at line 147 because live process enumeration returned exit 1 in this environment (`the process list could not be read`). This is the intended fail-closed gate, not an observed mutation failure. Make the synthetic main-flow test use the existing deterministic process-probe seam, with explicit unreadable-enumeration refusal coverage; do not weaken the production gate. The suite was not independently green here.

1. Apply `AAAA` → `BBBB` through `iso_patch::apply` and restore successfully.
2. Reload `record.json`, confirming `RecordStatus::Restored`.
3. Make a later independent edit to the file: `BAAA`.
4. `iso_patch::state` returns `Partial` and `restore` returns `Ok(RestoredPartial)`, replacing that later edit with `AAAA`.

The assertion requiring refusal failed as expected. Probe output: `completed record: Restored; state: Partial; restore: Ok(RestoredPartial); bytes: "AAAA"`. The fixture was removed before the assertion; temporary probe source/build outputs were removed at review closeout.

## F1 — P1: a completed record still authorizes partial-write recovery

`crates/studio-domain/src/iso_patch.rs::classify` uses only byte membership in the old/new arrays to authorize `Partial`; it never checks the record's lifecycle state. `restore` therefore treats later external modifications as this transaction's interrupted write even after the transaction has completed and restored. A mixture of old/new values alone is not evidence of an active interrupted mutation.

Bind automatic partial recovery to durable unfinished mutation intent for the correct transaction/range. Completed `Applied`/`Restored` records must not authorize overwriting a subsequently introduced mixture merely because each byte equals an old/new value. Persist a restore-in-progress intent before restore writes so interruptions during restoration remain recoverable; distinguish apply recovery from restore recovery. Reload/validate current durable state at mutation time rather than trusting a stale in-memory record. Keep exact prior/replacement idempotence and backup identity checks, but refuse ambiguous externally changed mixtures unless a distinct explicit recovery decision is made. Do not weaken foreign-byte refusal.

Regressions: the reproduction above; an external mixture after successful apply; a stale `Writing` record object retained after the durable record became completed; interruption during restore; genuine interrupted apply and restore still recover; arbitrary foreign bytes remain untouched. Verify returned classifications and UI recovery affordances match the lifecycle state. Use synthetic files/images only.

## Coordinator decisions on receipt questions

1. Keep platform-wide running-emulator exclusion. No open-handle probing is required for this package; fail closed if enumeration fails.
2. Keep in-memory reviews for the present database/dictionary scope. Supersession, consumption and project close must release them; losing reviews on restart simply requires a new review. Disk-backed snapshots are unnecessary here.
3. Durable in-place-save journaling is separate required reliability work before claiming crash-safe coordinated saves. It need not expand R3. Keep the current process-local limitation explicit and do not claim restart detection/recovery without a journal. This does not waive image-record recovery, which is already in scope.

## Next executable package

Execute OVERNIGHT-DEV-R3: fix only the lifecycle-bound partial recovery above, add the regressions, rerun focused domain/API/UI tests affected by the change and bindings if necessary. Preserve unrelated files, prior candidate bytes, source anchors and originals. No live deployment/image writes, launches, quarantine, staging, commits or pushes.

Deliver one plain-language-first receipt with actual tests, changed hashes and cleanup; stop for Codex review. Keep the image Write cards withheld. After acceptance, resume Phase 1 with the refreshed chunklzx vectors and corrected schedule/group/logo research; do not start a new feature while R3 is open.
