# S2-C-R1 review and S2-C-R2 correction dispatch

Claude session: continue the same session. Two bounded corrections remain in the current transaction/API lifecycle; do not start league work yet.

Reviewed 17 September 2026 by Codex. Plain-language result: the original missing-header and same-file-session reproductions now refuse unsafe saves, and focused tests pass. A new Windows filename-case alias still lets another workspace overwrite a reserved file during apply. Package acceptance is withheld pending this correction and the staging lifecycle/cleanup item below.

## Independent checks

The four principal source/test hashes match the submitted receipt:

| File | SHA-256 |
|---|---|
| crates/studio-api/src/registry/copied_target.rs | b4d6001842683ddac1486098185a068ae05507ad194d18e7ab8d811b195e78c6 |
| crates/studio-domain/src/copied_target.rs | 85db98ca2490ae07ac8ed3f66263a6beceb1f47dabc17026d1141dd5b34f15be |
| crates/studio-api/src/registry.rs | e33fda14ecbceb877ee41da36a720b2eeb9a6e777ff44729c01952d9a1715a83 |
| crates/studio-api/tests/copied_target.rs | 6f3d93f3e6ecb5c89995d92a9932e3caecbcdf77b5b7fc83c99bba104454bcda |

Executed independently against the current working tree:

- `cargo test --locked --offline -p studio-api --test copied_target --target-dir local/reviews/s1-codec-target`: 10 passed.
- `cargo test --locked --offline -p studio-domain --test copied_target --test txn_coordination --target-dir local/reviews/s1-codec-target`: 6 + 3 passed.
- `cargo run --locked --offline -p studio-api --bin export-bindings --target-dir local/reviews/s1-codec-target -- --check`: 76 roots / 139 files, in sync.
- Extended the existing review-only `local/reviews/s2c-probe` and ran `cargo run --offline --manifest-path local/reviews/s2c-probe/Cargo.toml --target-dir local/reviews/s1-codec-target`. This separate harness uses its own lockfile; output observations are evidence, not a test-suite pass.

No application source edits, live installation, commits or staging by this review. The index is empty. Full workspace/clippy/frontend/runtime campaigns were not repeated. Earlier reviewer scratch remains while this correction is active; do not claim package cleanup is closed.

## R1 — Save As bypasses reservations through Windows filename case (P1)

`crates/studio-api/src/registry.rs:2218–2221` canonicalizes only the parent directory, then appends the caller's filename unchanged. `Reservation::covers` compares PathBuf values exactly. The file stored in the reservation is `nhlng.db`; Save As to `NHLNG.DB` refers to that same file on the tested Windows filesystem but does not match the reservation key.

Independent reproduction uses only copied anchor files under `local/reviews`:

1. Open the copied primary and roster, edit and prepare a transaction.
2. Open an unrelated DB as another workspace and give it a distinguishable field value.
3. At the transaction's `AfterReplaced(primary)` hook, call `Registry::save(unrelated_id, <copy>/NHLNG.DB)`.
4. The competing write returns `Ok(2820656)` instead of refusing; the enclosing operation subsequently errors because the committed file no longer matches the initiating session's working bytes.

Observed output:

```text
EXISTING HEADER BOUND INPUTS: 1
APPLY AFTER HEADER DELETION: Err(... content-header ... is gone ...)
ALIAS SAVE DURING APPLY: Err(... project 1 is locked ...)
UNRELATED SAVE AS UPPERCASE RESERVED FILE: Ok(2820656)
APPLY WITH ALIAS WRITER: Err(... transaction ... committed and is restorable, but the session could not adopt ... different inner database ...)
```

Scratch reproduction: `E:/Repositories/nhl-modding-studio/local/reviews/s2c-probe-data-70568`. Harness source SHA-256: `2a6d19e5340dd0eacadd5354f1b7f8965944da389517f83b3c286e79276b91fc`.

Required correction: use a consistent filesystem-aware destination identity for reservation creation and every conflicting write check. For existing destinations, resolve the final file rather than only its parent; cover equivalent Windows case spellings and supported path aliases. For planned absent files, use a consistent parent-plus-leaf identity that prevents equivalent spellings bypassing the guard without conflating distinct files on case-sensitive filesystems. Refuse unresolved/unsafe aliases where a sound identity cannot be established. Consider final-component redirection, not only parent-directory redirection. Keep this within shared reservation checks; do not migrate all writers or expand external-process guarantees.

Add the exact unrelated-workspace Save As regression and assert refusal **before any bytes change**, successful initiating transaction, intact session state and release of reservations. Cover apply and whole-root recovery/restore guards using the same identity rules. Preserve unrelated-file access. Fixing the downstream rebind error alone would hide the symptom without closing the competing write.

## R2 — prepared staging ownership and normal teardown (P2)

The developer disclosed that prepared handles left at process exit leave staging directories behind. Source inspection confirms that `Prepared`/`CopiedState` have no staging ownership teardown, and `remove_staging` ignores removal errors. After this review's focused tests a new directory `H:/TEMP/nhl-modding-studio/copied-target-staging/45068-0-0` remained; no process with ID 45068 was running when inspected. This is a current lifecycle gap, not merely the 17 historical directories the developer reports deleting.

Provide deterministic cleanup for normal prepared-handle abandonment/registry or session teardown, respecting transaction abort/recovery ordering. Stop retaining an unused prepared transaction and its staging indefinitely after explicit close/disposal. Report cleanup failures or retain a tracked cleanup obligation rather than silently claiming success. Add a focused normal-teardown regression and test that another active registry/handle's staging is preserved.

Do not promise cleanup after forced termination through Drop alone. Record any crash-retained files with their recovery purpose and safe lifecycle. No broad orphan sweeper, PID-only deletion rule, deletion of live recovery journals, or filesystem cleanup outside verified owned roots. This is a bounded lifecycle closeout, not a new maintenance subsystem.

## Next prompt for Claude

Continue as sole Studio developer on the existing dirty branch. Read this review and `docs/plans/coordinator-developer-working-agreement.md`.

Execute **S2-C-R2**:

1. Close R1's reproduced filename-case reservation bypass, covering existing and planned destination identity consistently. Add the exact before-write regression and related alias checks; retain unrelated workspace availability.
2. Close R2's normal prepared-handle teardown/staging lifecycle gap. Preserve recoverable state where cleanup/abort fails; do not introduce unsafe global temp deletion.
3. Run the affected domain/API tests, binding drift check and the reviewed reproduction. Generate bindings through the exporter only if the change needs DTO updates. No unrelated corpus campaign or feature work.
4. Begin the receipt with a short plain-language summary; include per-finding result, changed hashes, commands/results, limitations and owned scratch inventory. If inspecting Codex's harness, do not rewrite it; put production regressions in the normal suite. Preserve review-owned scratch until Codex closes the package; clean your obsolete owned artifacts safely and report exact dispositions.

No frontend, live installation, Project/Build work, staging, commits or pushes. Stop for Codex review. On acceptance and cleanup, Codex will issue L1.1-A in a fresh Claude session.

## Cleanup register and roadmap

- Codex-owned, retained for active correction review: `local/reviews/s2c-probe`, `s2c-probe-data-48452`, `-56128`, `-58772`, `-70568`; the last is this review's new reproduction. Clean them at package closeout after durable regression/evidence retention.
- `local/reviews/s1-codec-target` remains the active shared review build directory for S2-C-R2. Its retention is an explicit current dependency, not permission to keep it indefinitely after the series closes.
- `H:/TEMP/.../45068-0-0` is a newly observed test staging leftover. Its full root and name are recorded above; fix ownership/lifecycle before cleanup and do not infer deletion authority solely from its PID-shaped name.
- The reported deletion of the previous 17 directories is developer evidence; this review did not witness those deletions. Do not delete unrelated `local/runs/codex` or shared target caches based on this receipt alone.
- Active: S2-C-R2. Phase 1.1 league identity remains next. Other phases queued; Projects/Builds remain final stretch.
