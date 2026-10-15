# Frozen r67 guided beta verification

Updated 2026-10-01. Development frozen. Preparation complete for initial source identification; operation-specific card/admission reconciliation continues before each test. No Studio/game launch, deployment, original-file write, rebuild, commit or message to another session occurred.

Baseline check: **automated-only pass**. Read-only SHA-256 verification of all 619 extracted manifest entries: zero missing/mismatched files; 620 files including the manifest. Executable and ZIP match the fixed baseline. This session hashed the ZIP as a whole; it did not separately stream-check ZIP members or rerun bundle selftests.

- Executable: `E:\nhlms-review-build-r67-2026-09-28\NHL Modding Studio rc-20260928-r67\nhl-studio.exe`
- Executable SHA-256: `60eee0adbd16df95692c4249f9926d7aff998855cf04958ba897fa12d37bbe65`
- ZIP: `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r67.zip`
- ZIP SHA-256: `6297cda973d2b9368d5809c086d71eb9f72c0ec14aa463493083f73fdfaa0b4a`
- Manifest SHA-256: `c06fe0128311e68df24274fca29dff407579d7406b875c136b5793040eb865d3`
- E: free at preparation: approximately 132 GiB. No install copied; recalculate required space after source selection.

[Matrix](MATRIX.md): 27 distinct targets, each with 90 operation families. No target or phase is complete. Family rows must be split into separate operation outcomes, including each lifecycle step, each equipment component and each save category. Unknown admission remains blocked pending evidence review; it must not become not applicable by default. Patch variants get distinct results.

Prior NHL21-E1 user acceptance is preserved for the tested league identity change, durability, consistency and restoration; see [prior evidence](../league-parity-batch-2026-09-18.md). It is not a blanket r67 pass and is not scheduled for repetition without a changed dependency.

## Pending user step

**N21-S01 — source identification (new test, not an existing card).** Recommend NHL 21 PS4. Ask for the install/source folder and patch if known, or a different preferred first target. Expected: exact source identified without alteration. Report: path and known patch. Restore: none, read-only setup. Await the answer before the next dependent action.

Then inspect that named source read-only, establish the baseline identity, determine the minimum working set, and choose an explicitly owned working-copy/evidence destination outside the tracked repository. Confirm existing backup ownership; do not overwrite backups or copy an entire install by default. Select named records and modest before/after values only after inspecting the user's data. Runtime launch/deployment remains the user's action; shared saves/cache require isolation or a specific backup/restore procedure first.

## Authority and card handling

The guided plan and October 1 freeze/review override older continuation banners. Scope: entity roadmap plus September 27/28 decisions. Coverage inputs: Phase 1–4 matrices, developer LEDGER/LEDGER-PHASE3, receipt 67 and latest ledger amendments. Earlier matrix cells are historical where later receipts explicitly correct them; developer evidence never substitutes for user observation.

Card entry points: `docs/receipts/overnight-development-2026-09-24/MORNING-REVIEW.md` and research `USER-CARDS.md` files. Retrieve the complete relevant card and its latest amendments before use. Initial relevant references include V-60, V-69, V-82, V-87–91, V-94–97; not executed. Preserve distinct failure/recovery and title/platform obligations when combining overlapping setup. A receipt naming a card without instructions requires a clearly labeled new test based on the documented workflow. Do not alter the frozen bundle to perform old missing-helper cards; prepare a separately identified owned test fixture if needed.

Use [ISSUES.md](ISSUES.md) for inherited limitations and new defects. Results must distinguish user-observed pass, automated-only, fail, blocked, unimplemented, untested and evidence-backed not applicable. No completion percentage or release-readiness claim.

## Session update — user authorized Studio launch

2026-10-01: User selected NHL21 and explicitly requested launching Studio. Rechecked executable SHA-256 against r67, then started the packaged executable with its bundle directory as working directory; process ID 178520. Launch command succeeded (automated-only); visible startup and detection remain untested pending user observation. No game launch, deployment or game-data edit performed.

Current pending step (supersedes previous next-step text): N21-S02 — inspect the startup screen. Expected: Studio opens without an error and NHL21 is listed if discovery finds the install. Ask whether NHL21 appears and for any startup error verbatim. Do not invent navigation labels. Source/install path, patch, working-copy destination and restore method remain to be established before edits.

Pending N21-S02: initial launch failed visibly; unrestricted retry PID 245332 started. Await visible-window confirmation. See ISSUES.md for evidence and stop-process error. Source/copy setup still pending.
Follow-up: taskkill /PID 178520 /F succeeded in approved desktop context. Only retry PID 245332 remains; it now reports MainWindowTitle=NHL Modding Studio and Responding=True. This supports a launch-context issue but does not establish root cause or user-visible success. Await user confirmation of the window and NHL21 listing.

2026-10-01 user update: Studio loaded after relaunch — startup now USER-OBSERVED PASS, initial restricted-launch failure retained. User requests full NHL21 checklist for asynchronous batch findings, overriding one-step-at-a-time presentation. Delivered NHL21-CHECKLIST.md (A01–H09; independently numbered within sections), derived tests with source-card attribution and explicit copy/runtime prerequisites. Pending: user's per-item findings, source/patch/copy details, and other observations. No further test or game launch performed; source/copy still unknown. Refer to the checklist for new batch IDs rather than confusing them with internal matrix operation IDs.

2026-10-01 — N21-001 (P1), A01 FAIL / A02 dependent setup BLOCKED: user reports PS4 onboarding incorrectly requires shadPS4-relative game and save locations. Read-only source inspection corroborates; provided roaming savedata root exists with 1/CUSA18182. Exact game folder/patch not yet supplied. Bounded correction: BUG-N21-001-onboarding-paths.md. No code or r67 modifications, no save reads/writes and no developer dispatch. Startup itself remains passed. Await remaining user findings or explicit bounded correction dispatch; do not count unopened downstream flows as failed or passed.

2026-10-01 receipt69/r68 review: independent bundle checks passed for r67/r68, but r68 NOT adopted. Coordinator reproduced P1 stale remembered roster substitution in manual onboarding with exactly one alternative (9 tests passed /1 regression failed). Review and bounded follow-up: r68-review/REVIEW.md. Probe preserved outside app test discovery; temporary sibling removed. No real save opened and no app launched. N21-001 remains open; affected onboarding/selection paused. Original r67 failure retained; NHL21-E1 stays accepted. Await corrected candidate, not user rerun on r68.

## Current checkpoint — r69 accepted for internal user verification

2026-10-01: receipt70 independently reviewed; see R69-REVIEW.md. Focused results:15/15 UI,2/2 unreadable-directory domain,4/4 API (1 real-install test explicitly ignored). Three bundles hash/manifest verified. N21-001 offline correction accepted; user confirmation pending. r69 is the candidate for new results, explicitly superseding r67 for the next pass; r67/r68 evidence remains historical. No r69 startup/user acceptance yet. Development frozen; no further developer work requested. Next: r69 NHL21 A01/A02 and focused source/save/reopen checks, with owned copies before Open because sidecar writes also occur on startup auto-open. Continue full checklist after onboarding. Preserve NHL21-E1 acceptance.
2026-10-01: User explicitly requested Studio launch. Reverified r69 executable SHA-256 and launched its packaged executable with bundle working directory in approved normal desktop context, PID268720. Visible startup and onboarding results await user observation. No other Studio process stopped; no game launch or deliberate game/save edit performed.

## ONB-002 user-directed workflow revision — 2026-10-01

User is reviewing onboarding across all games before the NHL21 detail checklist. New accepted flow: choose fresh bundled vanilla versus own inputs and a project folder; unchanged game/platform selection; confirm project/game-install/save destinations for every target; optional full extraction into the installation with Skip and dependency gates; Settings primary tab Tools extraction. All editing outputs persist in project storage until explicit final deployment. User explicitly confirms anchors as the bundled verified vanilla source. This is a scoped override of the earlier project-folder deferral/freeze, not a regression finding against r69.

Coordinator specification/Claude dispatch: docs/plans/onboarding-20261001-project-workspace.md (ONB-002). Covers all27 shipping targets, source/destination separation, project persistence and legacy sidecars, safe optional extraction, exact anchor/codec inventory and package validation. Source inspection notes anchor identity/coverage gaps and existing extractor overwrite behavior for developer reconciliation. No implementation, extraction, game/save write, app launch or cross-session message performed by coordinator. r69 remains preserved; new workflow acceptance pending implementation and review. Next developer candidate r70 if unused. Existing findings and NHL21-E1 preserved; changed persistence/deployment dependencies must receive focused requalification.

2026-10-01 follow-up: user requests a separate Open existing project flow AFTER ONB-002, restoring the saved game instance and previous edited session. Queued ONB-003 in docs/plans/onboarding-20261001-resume-projects.md. ONB-002 scope text now separates its required durable persistence/service-level reopen from this successor's user-facing project picker/resume flow. Existing-project open must not be confused with importing own database/save files. No implementation or cross-session dispatch performed; next action remains ONB-002 delivery/review, then focused ONB-003 dispatch using the actual project format.

## Expanded ONB-003 — latest user scope, 2026-10-01

User explicitly expands ONB-003 from resume-only to flexible asset-only/roster-only/main-DB-only/combined projects, platform-agnostic durable editable sources/operations, single-file distributable mod package builder and built-in installer. Same-game platform adapters (Legacy Xbox360/PS3/Recompiled example) are included; all cross-game/year conversion and automatic omission of unsupported content remain deferred. The explicit request supersedes the supplied vision's future-release wording for package saving/build/install. Same-day release intent is not publication authorization or evidence of readiness.

Revised full developer specification: docs/plans/onboarding-20261001-resume-projects.md. ONB-002 now links to its newer input/destination requirements; preserve ONB-002's delivery/review boundary. New final destinations may be unbound until needed, supporting edits without an installed game. All edits remain project-owned; distribution packages exclude author-specific paths/baseline dumps and install only through verified adapters plus reviewed recoverable transactions. No changes to feature source, builds, originals or running applications; no developer-session message sent. Next: deliver this revised prompt to the developer for work after ONB-002 review.

## r71 coordinator review — changes required, 2026-10-01

Candidate r71 (ONB-003, receipt 72) identity verified: ZIP/EXE hashes match; all 771 extracted manifest entries pass. Focused package review reproduces five findings with six owned-copy probes: wrong entity from row-index replay; incorrect main/roster scope and missing required roster accepted; partial restore cannot recover after sidecar deletion failure; asset-only Project Save rejects the project; PS4 texture installer uses the DB directory as asset root. The four existing package integration cases pass, but the final six added regressions all fail. No original game/save, candidate bundle or implementation source changed; no app/game launch. Temporary test sibling removed after preserving evidence. No user or runtime acceptance promoted.

Review: [r71-review/REVIEW.md](r71-review/REVIEW.md). Prepared development prompt: [r71-review/CORRECTION-HANDOFF.md](r71-review/CORRECTION-HANDOFF.md). Keep r71 frozen and hold the user package/install pass pending corrections and coordinator re-review. Same-game graphical multi-platform qualification remains incomplete; cross-game conversion remains deferred.

## r72 coordinator review — original fixes verified, roster-only installer gap remains

2026-10-02: ZIP/EXE hashes and all 771 bundle manifest entries verified. Preserved probe file is byte-identical; 10 original/probe cases plus 10 correction cases pass independently. The five r71 defects are closed at their tested automated scope. One new owned-copy regression fails: a valid roster-only package cannot install without a game root; using the save folder as root fails on missing metadata. This is the remaining explicit R2 dependency requirement, tracked as R72-R1/P2, with zero recipient writes. No user/game checks promoted. Full review and bounded development prompt: [r72-review/REVIEW.md](r72-review/REVIEW.md), [r72-review/CORRECTION-HANDOFF.md](r72-review/CORRECTION-HANDOFF.md). No application launched, implementation changed, or candidate modified. Temporary review test preserved and removed. Cross-platform graphical portability gaps remain separate and disclosed.
