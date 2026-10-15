# S2-C-R4 review and L1.1-A dispatch

Claude session: start a new session. The bounded copied-target correction passes independent review; the next package changes focus to league identity.

Reviewed by Codex, 17 September 2026. S2-C-R4 implementation accepted for the reviewed copied-target API scope. This does not qualify live installations, every filesystem alias, UI runtime or cross-game editing parity.

## Independent verification

Candidate hashes match the receipt:

- crates/studio-api/src/registry/copied_target.rs: 7d2bda17a04cf5b34e0b02f0f5f4426041ab5dc29d279dadbaf68cda6b7ea212
- crates/studio-api/tests/copied_target.rs: 17a8e9772d879e9434ee5e7f1511811263374a9de521ffe6ddbb1dfe887f5a84

Reviewed recovery reservation ordering, the apply Finish guard, settlement and receipt section 12.3's lifecycle table. Reservation refusal now leaves prepared ownership intact. Apply exits settle staged material against persisted transaction state rather than deleting it unconditionally.

Fresh independent checks:

- `cargo test --locked --offline -p studio-api --test copied_target --target-dir local/reviews/s1-codec-target`: 15 passed, including both R4 regressions and panic/recovery paths.
- `cargo run --locked --offline -p studio-api --bin export-bindings --target-dir local/reviews/s1-codec-target -- --check`: 76 export roots, 140 files, in sync.
- Unchanged reviewer failure harness (SHA-256 8f794922d6f27e9920dcaa1b1b42246a2f24524fc5f690bbf7ef540537f66ed6), run with `cargo run --offline --manifest-path local/reviews/s2c-probe/Cargo.toml --target-dir local/reviews/s1-codec-target`: all previously failing observations corrected in copied fixture s2c-probe-data-44804.

```text
UNRELATED SAVE AS UPPERCASE RESERVED FILE: Err(... held ...)
APPLY WITH ALIAS WRITER: Ok("committed")
ABORT FAILURE STAGING AFTER: true
OUTER HANDLE AFTER REFUSED RECOVERY: true
TEARDOWN AFTER REFUSED RECOVERY: []
OUTER STAGING AFTER TEARDOWN: false
APPLY REFUSAL STAGING RETAINED: true
```

After journal pins were released, normal Drop resolved the retained transactions and removed staging. All journals in the final fixture were terminal. No application code changed by Codex; no staging/commit/push. Broader developer checks were not rerun. Case-sensitive directories, symlink/junction branches, Unicode path equivalence and in-game/runtime qualification remain limited to the evidence previously recorded; no new support claims are inferred.

## Closeout evidence

The reviewer harness was temporarily given an explicit-root recovery mode. Registry::copied_target_recover succeeded and reported apply_allowed for the old -66720, -66316 and -68400 copied roots. A journal inventory then verified every transaction in all 13 listed S2-C reproduction directories was terminal (aborted, committed or rolled-back). These are disposable copied anchor fixtures, not user recovery backups.

Owned closeout inventory under E:/Repositories/nhl-modding-studio/local/reviews:

- s2c-probe (temporary reviewer source; equivalent regressions remain in crates/studio-api/tests/copied_target.rs).
- s2c-probe-data-15100, -44804, -48452, -56128, -58772, -66316, -66720, -68400, -70568, -75772, -76384, -77572, -79836.
- s1-codec-target, the shared S1/S2 reviewer build cache whose current correction dependency is complete.

Owned obsolete staging: H:/TEMP/nhl-modding-studio/copied-target-staging/45068-0-0 (previously recorded disposable test leak), /66316-3-3 and /68400-3-3 (recorded reviewer leaks; associated transactions now resolved). Cleanup outcome is recorded below after execution. Other local/reviews artifacts, shared local/runs builds and unrelated developer work are not included by name similarity.

### Cleanup outcome

Completed: all 15 explicitly listed local/reviews directories (harness, build cache and 13 reproduction directories) and the three explicitly listed H:/TEMP staging directories were removed. Each resolved absolute target was checked for containment and reparse points before literal-path recursive removal; no review executable was running. The staging root then contained zero entries. Regression source and durable review/receipt evidence remain. No S2-C review scratch dependency transfers into L1.1-A. Other historical review directories and unrelated shared caches were not claimed or deleted by this package.

## Executable prompt for Claude — L1.1-A

You are the primary Studio developer in E:/Repositories/nhl-modding-studio; Codex leads and reviews. Start a fresh session and preserve the existing branch and dirty tree. Read CLAUDE.md, docs/plans/coordinator-developer-working-agreement.md, docs/plans/entity-editing-parity-roadmap.md, this acceptance review, docs/EDIT-MODEL.md and the relevant target catalog/QA contract. Reuse existing implementation and verified findings.

Assignment: **L1.1-A — League identity mapping and existing-workflow assessment.** This is a bounded Phase 1.1 assessment, not feature implementation or a new discovery platform.

1. Start with NHL 21's actual available target/fixtures. Trace league/competition identity from native DB tables and fields through metadata, domain/API and the current UI. Record source hashes, record identifiers, encodings, references and read/write consumers. Distinguish primary keys, classification values, display strings and localization references; do not guess semantic meaning from field names alone.
2. Build an evidence-backed equivalent-field/workflow matrix for all 27 shipping target combinations from the actual catalog. Keep NHL 07 PS3 research-only; qualify Legacy Recompiled independently. Record absent fixtures as blocked/untested, not unsupported or not-applicable. Separate verified facts from candidate mappings and runtime unknowns.
3. Select one concrete NHL 21 league and a minimal before/after identity edit within its existing slot. Preserve primary keys and topology. Identify all affected fields/consumers and whether the desired label depends on Phase 1.3 localization. Map the equivalent operation per target without copying raw IDs across games. If no safe directly editable identity value is established, explain that constraint and propose the smallest evidence-gathering step.
4. Assess the existing direct-editing workflow: browse, edit, validate, pending review, copied-target save, reopen and restore. Identify exact gaps and the smallest implementation slice needed. Scripts establish evidence; they do not count as completed Studio UI functionality. Specify runtime inputs and expected visible behavior without claiming unexecuted game tests.
5. Deliver docs/plans/league-identity-parity-matrix.md and a receipt with source/code references, current workflow gaps, proposed bounded implementation package, per-target blockers and acceptance checks. Do not mark Phase 1.1 complete from mapping or one representative target.
6. Begin the receipt with a short plain-language summary. Include checks actually performed and a manifest of any scratch, generated data, builds or processes. Use an explicit package-owned scratch root, clean obsolete artifacts before handoff, and list any retained review dependency. Do not rebuild the entire workspace merely for a documentation assessment.

Exclusions: no application feature code yet, no league expansion/membership/rules changes, no localization or asset implementation, no Projects/Builds, no automatic Texture Lab continuation, no external head-converter work, no live installation, no staging/commits/pushes. Do not alter Codex-owned roadmap/review records. Stop with the assessment for Codex review; Codex will issue the implementation prompt afterward.

## Roadmap status

S2-C correctness correction accepted within the evidence above; closeout cleanup recorded below. L1.1-A league identity assessment is next, followed by implementation and verification across applicable games before Phase 1.2. Teams, players, equipment and miscellaneous remain queued. Projects/Builds remain final optional stretch work.
