# Studio briefing review and initial dispatch

Reviewed by Codex 17 September 2026. Planning review of CLAUDE.md, the active entity-editing roadmap, S2-C developer receipt and existing S2-C correction review. No corrected implementation was independently tested in this review.

## Findings

- Primary ownership is unchanged: Claude implements Studio; Codex leads and reviews.
- Existing entry-point text still called C0–C10 the execution sequence and required external-mod ingestion into canonical Projects. The current roadmap supersedes both: entity editing is active; Projects/Builds are stretch.
- The latest reviewed implementation package is S2-C, with S2-C-R1 corrections outstanding in the inspected record. The receipt's broad claims about bound headers, reservations and refresh behavior are not accepted until those corrections pass review.
- Shared copied-target safety remains relevant to direct editing. Finish the bounded correction, but do not follow the old dispatch's instruction to proceed into Texture Lab UI next.
- The new cleanup, plain-language receipt and session/prompt/status rules are in the [working agreement](coordinator-developer-working-agreement.md) and linked from CLAUDE.md.

## Session recommendation

Continue the same Claude session for S2-C-R1 because the changes are local to its current transaction work. This recommendation is based on task continuity, not measured token usage. After acceptance and cleanup, start a fresh session for Phase 1.1. If a newer correction receipt exists, present it for review first rather than redoing completed work.

## Next prompt for Claude

You are the primary developer for NHL Modding Studio at `E:\Repositories\nhl-modding-studio`. Codex is project lead and reviewer. Preserve the existing branch, dirty working tree and unrelated work. Do not start a new implementation track.

Read `CLAUDE.md`, `docs/plans/coordinator-developer-working-agreement.md`, and `docs/plans/entity-editing-parity-roadmap.md`. The active product sequence is leagues → teams → players → equipment → miscellaneous presentation, with parity per step. Canonical Projects/Builds are final optional stretch work. The old automatic next step of Texture Lab UI is superseded.

Current bounded assignment: **S2-C-R1**, as specified in `docs/validation/s2c-review-and-correction-dispatch.md`. If this correction is already completed, return the latest receipt and changed-file hashes for Codex review instead of duplicating it. Otherwise:

1. Bind existing content headers as contained, identity-checked inputs. Changed, deleted, unreadable or redirected headers must invalidate review before writes; preserve unchanged headers exactly.
2. Reserve affected files/roots and aliasing sessions, not only the initiating session ID. Make preflight and reservation one synchronized transition; cover apply/recover/restore, newly opened aliases and legacy save paths while leaving unrelated work usable. Preserve dirty edits and release guards on every outcome.
3. Report refresh failures explicitly and retain a coherent, safely blocked session where needed. Never convert missing/corrupt files or failed refresh into an unchanged success. Test primary and roster failures and retry/reload behavior.
4. Add/run the focused regressions required by that dispatch. Run relevant domain/API checks and the generated-binding drift check; regenerate DTOs only through the exporter where required. Report skips and unexecuted runtime cases honestly. Do not rerun unrelated corpus campaigns.
5. Start the receipt with a short plain-language summary. Follow with per-finding results, changed paths/hashes, reservation behavior, error-state examples, commands/results and remaining limitations.
6. Inventory package-owned scratch/probe/copied-target/build paths and processes, including known related prior artifacts. Remove obsolete artifacts whose review dependencies are closed after retaining durable results. Preserve material still needed for this correction review and list exactly why; identify unowned/shared leftovers without deleting them. Codex will verify closeout cleanup before moving to league work.

Scope remains domain/API transaction planning, reservations, session refresh, focused tests and generated DTOs if necessary. No frontend feature work, Project/Build implementation, format expansion, live installation, unrelated fixes, staging, commits or pushes. Do not change Codex-owned roadmap/review documents. Stop after the receipt for Codex review.

## Queued next feature package (not dispatched yet)

**L1.1-A — League identity mapping and existing-workflow assessment.** In a fresh session after S2-C-R1 closure, inventory NHL 21 league identity fields and their read/write/UI consumers, then produce the corresponding target matrix for all shipping combinations. Distinguish literal names from localization keys and IDs from editable display values. Identify the minimal implementation slice and exact runtime inputs needed. Preserve primary keys in the first edit; no topology change, canonical builder or Phase 1.3 localization implementation. Codex will issue the concrete implementation prompt after reviewing this bounded assessment; it is part of Phase 1.1, not a broad registry rewrite.

## Truncated roadmap status

- Active prerequisite: S2-C-R1 direct-editing transaction corrections, awaiting corrected receipt/review.
- Next: Phase 1.1 league identity; begin with L1.1-A mapping and existing-workflow assessment.
- Phases 1.2–5.4: queued; no new acceptance implied by this briefing.
- Projects/Builds: final optional stretch.
