# Codex / Claude working agreement

Accepted 17 September 2026 from the user's direction. Applies to Studio work packages and their independent reviews.

## Ownership and sequence

**Overnight authorization — 19 September 2026:** use the [developer brief](overnight-development-2026-09-19.md) and [research brief](overnight-research-2026-09-19.md) in two fresh Claude sessions. Their bounded queues supersede intermediate review stops for this campaign. P1-C-R2 is accepted offline; the developer starts at D1 (P1-BS-1 scoped settings decoding), then continues at developer-verified checkpoints; research advances independently. All new work remains pending independent review, and visual/manual acceptance is deferred to the user. Sole-code-writer ownership, cleanup and no live deployment/publication remain in force.

Codex is project lead/coordinator: scope, architecture, work-package prompts, independent review, acceptance, roadmap status and session handoffs. Claude is the primary Studio implementation developer. Codex does not become a concurrent application-code writer. Existing external head-converter ownership remains separate.

Follow [entity-editing parity](entity-editing-parity-roadmap.md). Projects/Builds are final optional stretch work. Close a bounded active correctness correction before changing feature tracks; do not use that correction to continue a superseded feature queue. Keep one active implementation package and preserve unrelated working-tree changes.

## Temporary files and builds

Every package identifies its owned scratch roots, temporary builds, generated test inputs, copied targets, probe artifacts and spawned processes. Record these in the receipt with absolute paths, purpose, active owner and disposition. Use package-specific locations; a shared existing cache cannot be claimed as package-owned by assumption.

Keep temporary reproduction material while its package/correction review needs it. Before advancing beyond that related package, preserve concise durable receipts, relevant hashes/test results and any minimal committed regression source, then remove the obsolete temporary builds/files. A session reset alone is not package completion. If a successor genuinely needs an artifact, record the new owner and specific dependency; retain it only while that dependency is active.

Codex reviews the manifest and verifies cleanup at closeout. Cleanup applies to reviewer-owned scratch as well as Claude's. Report removed paths and anything retained with its reason. Failed cleanup is explicit remaining work, not a silent exception.

Never delete by broad name/age/glob inference. Before recursive deletion on Windows, resolve each exact path, verify it stays within its declared scratch root, check ownership and reparse-point behavior, and stop any owned processes holding files. Use native PowerShell literal-path operations. Preserve source code, uncommitted work, anchors, durable receipts, user originals, installed games and live recovery backups. Do not run git clean/reset or delete a whole shared `local`/`target` tree. An active recovery backup is not disposable scratch; resolve recovery before considering its lifecycle.

## Claude receipt format

Every work receipt and handoff begins with **Plain-language summary**: 2–4 short sentences describing what users can now do, what was fixed and what remains unavailable. Put this before the technical report; avoid internal IDs and acronyms unless needed for clarity.

Then provide:

1. Package ID, claimed disposition and precise scope.
2. Changed paths and hashes; baseline revision plus relevant dirty-tree state.
3. Behavior, architecture decisions and each requested correction's disposition.
4. Checks actually executed: commands, results, skips, fixture identity and untested claims. Separate developer results from prior independent review.
5. Remaining blockers and runtime/target limitations.
6. Temporary-artifact/process manifest and cleanup status; retained review material with ownership.
7. Exact next review action. Stop for Codex review rather than starting another package.

Generate DTOs through the documented exporter. Do not stage, commit or push unless explicitly authorized for named reviewed paths under existing repository rules.

## Codex review/report format

The first line of the initial report after a receipt review is one of:

- **Claude session: continue the same session.** Follow with a brief reason.
- **Claude session: start a new session.** Follow with a brief reason.

Choose based on package continuity and available context, not invented token telemetry. Normally retain a session for a bounded correction to code it just wrote; start a fresh session at an accepted package/phase boundary or when stale assumptions and context size threaten reliability. If actual session usage is unavailable, say so only when it matters to the recommendation.

After that line, provide the **next executable prompt for Claude**, self-contained or with precise local document references. Include objective, scope, accepted inputs, deliverables, meaningful checks, temporary-artifact obligations, exclusions and stop point. If acceptance is blocked, the next prompt is a correction package; do not dispatch the next feature anyway.

**After the prompt**, give a truncated roadmap review: active step/package and disposition, next step, principal blocker and deferred scope. Keep technical findings in the linked review unless they materially affect the user's decision. Never present an old review's test results as freshly executed.

## Current transition

**Latest research acceptance:** [P1-B-R2 accepted; settings research dispatched](../validation/p1b-r2-acceptance-and-settings-research-dispatch.md). Start a fresh research session for defaults/context encoding and cross-title equivalents. This supersedes P1-B research corrections below; developer P1-C-R1 remains active.

**Current research correction:** [P1-B-R2](../validation/p1b-r1-review-and-r2-dispatch.md). Same research session: reconcile summary/coverage, complete truncated residual evidence and narrow runtime verdicts. NHL 15 PS4 settings/default catalogue is an accepted research lead. Developer P1-C-R1 remains separate.

**Latest developer review:** [P1-C-R1](../validation/p1c-review-and-r1-dispatch.md). Continue the same developer session for parent-group validation, source-provenance selection and membership summary ownership. The initial P1-C dispatch below is superseded; P1-B stays with research.

**Current developer dispatch:** [P1-A accepted; P1-C PS4 groups](../validation/p1a-r2-acceptance-and-p1c-dispatch.md). Start a fresh developer session. R2 ticket correction passes review; the bounded offline schedule workflow is accepted. This supersedes the P1-A correction dispatches below; P1-B-R1 remains with research.

**Latest research dispatch:** [P1-B-R1 evidence/coverage correction](../validation/p1b-review-and-r1-research-dispatch.md). Continue the same research session; retain measured inventory, correct causal and absence claims, reconcile 43 populated schedule comparisons versus 46 extracted saves, and refine test cards. Developer P1-A-R2 is unchanged.

**Latest developer dispatch:** [P1-A-R2 ticket ownership correction](../validation/p1a-r1-review-and-r2-dispatch.md). Continue the same developer session; R1's stored plans pass focused tests, but cross-workspace ticket collisions were independently reproduced. This supersedes the R1 correction dispatch below. P1-B stays with research.

**Research lane:** [LOC-W3 revision 3 accepted; P1-B research dispatched](../validation/loc-w3-r3-acceptance-and-p1b-research-dispatch.md). Start a fresh research session for competition rules and mode initialization. This does not replace the developer correction below.

**Current dispatch:** [P1-A-R1](../validation/p1a-review-and-r1-dispatch.md). Continue the same developer session to bind schedule applies to their previews, correct UI async ownership, and complete applicable offline parity coverage. P1-A remains open; this supersedes the initial P1-A dispatch below.

**Latest review:** [R3 acceptance and P1-A dispatch](../validation/overnight-dev-r3-acceptance-and-p1a-dispatch.md). R3 passed independent focused review. Start a fresh developer session for cross-title source schedule editing/import/generation within supported limits. This supersedes the correction dispatches below; live image Write cards remain withheld. Earlier decisions on emulator exclusion, in-memory reviews and in-place journaling remain in force.

**Post-overnight review, 18 September:** continue the developer session for [OVERNIGHT-DEV-R2](../validation/overnight-2026-09-18-review-and-r2-dispatch.md). This is the current dispatch and supersedes continued overnight feature execution: fix write/recovery and reviewed-image-plan ownership, reconcile Phase 1 coverage, then stop for Codex review. No live image writes or game launches are cleared by the overnight receipts.

**Current overnight authorization, 18 September 2026:** the user authorizes two fresh Claude sessions under the [two-agent campaign dispatch](overnight-two-agent-campaign-2026-09-18.md): one research lane and one sole Studio developer. That dispatch supersedes intermediate review stops for this run, including R1's stop instruction. Finish the corrections, then proceed through the roadmap with per-step implementation parity; defer visual/manual acceptance until the user's return. Use its explicit ownership and nonblocking coordination protocol. Xbox installations exist under `X:\Games\Emulation\Roms\XBOX360`; earlier missing-install conclusions are superseded. Legacy Recompiled reuses Xbox Legacy research, with `_compiled` discovery addressed separately. No live deployment/game launches or git publication overnight.

NHL21-E1 is accepted by the user and closed for its tested scope. Continue the same Claude session for [LEAGUE-PARITY-COMPLETION-R1](../validation/league-parity-completion-review-and-r1-dispatch.md): save/reload integrity, async UI ownership, interrupted quarantine recovery and target-specific binding evidence. Complete corrections as one reviewed package; no live deployment or game launches. User acceptance remains authoritative.

S2-C-R4 is accepted for the reviewed copied-target API scope and its review scratch cleanup is complete. L1.1-A is accepted as a planning input with corrections in the [overnight execution brief](league-overnight-execution-2026-09-17.md).

**User-authorized overnight exception, 17 September 2026:** Claude may proceed through that brief's implementation queue without user updates or intermediate Codex/user review stops. Use developer verification checkpoints and durable receipts, continue independent work around external blockers, and leave completed implementation pending subsequent independent review. This exception overrides the per-package stop instruction for this run only. Cleanup, evidence honesty, sole-writer ownership and no staging/commit/push remain in force. Supporting localization reader/archive work is authorized without declaring Phase 1.1 parity complete. The old automatic Texture Lab continuation remains superseded.
