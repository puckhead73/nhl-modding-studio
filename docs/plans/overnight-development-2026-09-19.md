# Overnight developer brief — 19 September 2026

You are Claude, the sole Studio developer, starting a fresh session in `E:\Repositories\nhl-modding-studio`. Codex coordinates and reviews; the user decides runtime acceptance. Execute this brief for as long as usage and context allow. Do not stop at an intermediate receipt, ask for routine review, or wait for the research lane. Save all visual/manual reviews for the user tomorrow. This campaign supersedes earlier per-package stop instructions for its listed work only. Your checkpoints are developer-verified, **not Codex-accepted**.

## Read first and establish the checkpoint

Read these sources, following links only as needed for the current package:

- `docs/plans/coordinator-developer-working-agreement.md`
- `docs/plans/entity-editing-parity-roadmap.md`
- `docs/validation/phase-1-coverage-matrix-2026-09-18.md`
- `docs/validation/p1c-r2-acceptance-2026-09-19.md`
- `docs/validation/p1bs-r1-acceptance-and-csv-parity-dispatch.md`
- `docs/validation/p1bs-csv-review-and-overnight-addendum.md` and `docs/research/overnight-2026-09-19/01-ps4-settings-csv-defaults-parity-and-packaging.md` (CSV research delivered; consume with the coordinator clarifications)
- Research atlas `docs/research/overnight-2026-09-18/00-ATLAS.md`; handoff 22 rev. 2 and its four-site metadata table; handoff 20 **revision 3**, not its superseded contracts.

Inspect the working tree and current code. P1-C-R2 is now independently accepted for its bounded offline scope: both supplied single-group slots are validated, and the focused regressions pass. Start implementation at D1; do not repeat the completed correction or its unchanged verification suites. Preserve all unrelated dirty files, enum deletions, diagnostics, root zips and existing evidence. No staging, commits, pushes, resets or clean operations.

Create your lane-owned records under `docs/receipts/overnight-development-2026-09-19/`: `STATUS.md`, numbered receipts and `MORNING-REVIEW.md`. Record the starting revision and file identities for your changes. Keep implementation parity separate from independent review and runtime acceptance.

## Parallel coordination — neither lane waits

Research has a separate brief and owns its new research directory. You alone edit application code, crates, definitions, bindings, tests, implementation matrix and morning cards. Never modify the research lane's active files. Existing research handoffs are inputs, not a licence to overwrite them.

Use `local/coordination/overnight-2026-09-19/claims/`. Each lane writes only its own prefixed claim files (`dev-<topic>.json` / `research-<topic>.json`): owner, topic, status (`active`, `ready`, `blocked`, `complete`), owned paths, dependencies, outputs and timestamp. Write through a temporary sibling then replace. A claim is coordination, not proof the work is correct. Read the other lane's claims/status at package boundaries and when a dependency arises; no polling loop. If the directory is absent, create it idempotently. Consult previous campaign claims for existing results, not active ownership by assumption.

- Research ready: consume the named checkpoint, record its hash/version, verify its assumptions with tests; do not follow a file being rewritten underneath an implementation.
- Needed topic unclaimed: claim a developer-owned bounded investigation, publish its findings in your lane's receipt directory, then implement when evidence permits. Research must avoid duplicating it.
- Needed topic actively researched: park that dependent slice with a precise question and move to the next independent Phase 1 package below. If no Phase 1 implementation can advance, investigate the next unclaimed roadmap topic in your own lane. Never implement a guessed writer to stay busy.
- External/manual blocker: record it, prepare a morning card, continue elsewhere. Runtime-only blockers do not prevent subsequent offline Phase 1 work. They also do not count as parity completed.

## Ordered execution queue

### D0 — Completed checkpoint: P1-C-R2 accepted offline

Read the acceptance record and proceed directly to D1. Preserve the single-group validation, witnessed pairs, exceptional-row handling and project-bound UI behavior. No additional P1-C work is queued here. Runtime group consumption, unknown names/mechanisms and NHL 15 patch coverage keep their documented limits; they do not reopen the accepted correction or block settings decoding.

### D1 — P1-BS-1 scoped metadata decoding

Implement the accepted table-plus-field and declaration-shape-scoped interpretation for `fRvu.SbEu`, `fRvu.CkjP`, `HwRX.QHRR`, `HwRX.NVBd`. Preserve the structural-only evidence for `NVBd`; do not describe it as externally confirmed. No general negative-range rule change. Prove `qbPU.CkjP` and unrelated signed/sentinel fields remain unchanged, as do the existing career plus/minus sites. Test decode/encode inverses, bounds and declaration mismatches. Use actual NHL 15 PS4 base and available patch metadata/data, with hashes and explicit private-fixture skips. No defaults editor in this checkpoint.

### D2 — Read-only settings catalogue integrated into Studio

Expose the NHL 15 five-table catalogue through the existing domain/API/UI architecture: setting row identity, context, stored global/context defaults, sibling references, raw codes, inferred labels and anomaly disclosures. Keep unsupported/unknown meanings explicit. Shared ids and names are not unique row identities. Handle missing and duplicate targets without inventing rows. Bind async UI results to current project/context. Add useful automated component/API coverage; save visual review for morning. No claims about what the game offers or applies.

The CSV source map and contract are now delivered. Consume handoff 01 with its coordinator addendum; do not wait for research to repeat this task. Reuse this model for each NHL 16–21 schema variant rather than creating a second editor. Keep title-specific bounds, contexts and choice vocabularies. Missing packaging blocks writing, not read-only integration.

### D3 — One-cell stored-default editing and parity extension

Implement handoff 22 rev. 2 **with the coordinator acceptance note taking precedence**. The eligibility table has 21 distinct setting IDs across 20 displayed rows. Validate actual row identity, declaration, min/max/size and context; preserve unrelated stock anomalies. Earlier edits never exempt a cell from validation. Reopened edited files remain eligible without historical tickets.

Use the existing edit/undo/pending/copied-target architecture. If an operation has a separate preview and Apply, hold the exact plan server-side under an opaque project-instance-bound ticket and workspace revision; Apply never recomputes from mutable inputs. Stale/foreign/consumed tickets, changed imports and ambiguous targets refuse without mutation. No new save/recovery mechanism merely for settings.

Prove only the selected cell bits and its exact covering body CRC change. Unchanged table/file headers require unchanged header CRCs. Do not repair unrelated bad checksums as a side effect. Prove undo/redo, pending values, two successive edits, save/reopen and copied-target restore. Use valid negative tests, not only tests mirroring implementation. No executable/profile/save writes.

Extend to NHL 16–21 only from the checkpointed CSV contract and verified packaging support. If packaging is missing, advance D4 as the shared prerequisite, then return. Gen7 executable-backed settings remain explicitly blocked unless independent evidence supports a separately bounded safe path; this brief does not authorize executable patching.

### D4 — Shared packaging required by Phase 1 (P1-F)

Inspect existing codecs/stagers first. Consume research handoffs 16, 18, 20 and the new CSV contract. Implement missing RefPack/chunkzip and archive member re-plan support in shared primitives, prioritizing what unblocks settings, localization and logos. Do not fork one writer per feature. Preserve unexplained archive words and disclosure; offline structural validity is not loader qualification. Keep disc-image writes same-length only; refuse growth there. No live image writes overnight.

CSV research has identified that the existing archive planner already supports fit-in-slot and append-and-retarget. Reuse it; prioritize native-framed encoding and the safe text-member API/artifact route rather than rebuilding planning. Stock bitstream reproduction is evidence, not a proven universal loader constraint. NHL 18–21 paired stored-default edits must name both cells and be atomic; follow the addendum's syntax, reserved-context and runtime-control limits.

For each supported measured chain prove unchanged-input round trip, changed-member decode/reopen, non-target member identity, offset/size/alignment correctness, bounds/overflow/truncation refusal, reviewed-plan ownership, fault recovery, and byte-exact copied-target restore. Changed encoded length must be accounted for at every wrapper. Do not route around transaction cancellation, alias exclusion or recovery guarantees. Unknown semantics that prevent safe construction are a recorded blocker, not guessed constants.

### D5 — Localization parity (P1-D / LOC-W3)

Close per-title/platform/locale LOC-W2 coverage using each source's own dictionary and provenance. Reuse the existing locale census; PS3 dictionaries are not Xbox proxies. Then implement LOC-W3 revision 3 in bounded checkpoints: preserving existing-key growth; supported new-key insertion; canonical rebuild/new glyph work only under its separate proof requirements.

Class A is **nondecreasing** hashes, equal-hash runs allowed. Preserve collision multisets, null strings, baseline order, sharing and user-selected sharing scope. Class B insertion is refused, while supported existing-key growth remains available. Do not overwrite shared strings accidentally. Rebuild equivalence is semantic under the contract's allowed differences, not a false byte-identity promise. Test LOC-W1/W2 outputs as inputs. Separate parser, packaging, lookup and glyph-rendering evidence; prepare user cards for the last two. Integrate through Studio, not a standalone proof-only command.

### D6 — League asset parity (P1-E, then G/H when evidence is ready)

Use measured container/codec families: PS3 RW4ps3 08–11, Xbox RW4, PS3 DDS, Xbox XPR2, PS4 NHL 15 PNG, PS4 16–21 GNF. Finish missing readers/writers and real-family copied-file round trips before broadening scope. Same-dimension replace first; size-changing replace only with the measured Apt geometry and all container dependencies handled. Integrate previews/import/export/replace through existing asset flows and transactions. Do not infer every texture layout from a league-logo sample.

Consume the research non-logo imagery and league-audio inventories as they become ready. Implement only demonstrated bounded routes; no fabricated league-audio absence from a filename scan. Prepare visual/audio review cards instead of launching games or asking for overnight review.

### D7 — Continue only after Phase 1 offline parity is accounted for

For each current operation, bring every applicable target to implemented-and-tested, or name the exact external/evidence blocker and the next action. Code shared across a generation does not replace per-target fixture evidence. Prefer closing Phase 1 test gaps over starting another feature. Unresolved implementation gaps stay active, not renamed external blockers.

If all executable Phase 1 work is exhausted, research ahead into Phase 2; begin Phase 2 implementation only when Phase 1 remaining items are specifically external/manual blockers and recorded as such. Follow Teams: identity, additional DB entries/relationships, localization, assets, arena/ice, kits, audio. Complete each operation's cross-title parity before the next. Do not expand the preserved team-localization slice out of sequence merely because it already exists. No Projects/Builds work.

## Verification, cleanup and morning handoff

No emulator/game launches, desktop visual review, live deployment, quarantine, user profile/settings changes or writes to installed games/disc images/anchors. Work on disposable copies; private fixtures are read-only inputs. NHL21-E1 is user-accepted and closed; never reopen it for screenshots. The user remains pass authority.

Reuse the existing build cache. Check free space before large builds/extracts; avoid giant image copies where bounded extraction suffices. If projected work would exhaust space, clean only your obsolete package-owned artifacts after evidence preservation, or switch to a smaller independent task. Do not delete shared `target/debug/incremental` or another lane's data. If resource constraints prevent safe work, checkpoint that condition and continue only work that fits.

After each bounded package: run meaningful focused checks, update the matrix with per-target evidence (including missing/skipped fixtures), record changed paths/hashes and a plain-language-first receipt, clean obsolete owned scratch, then continue. At stable milestones run broader affected suites; exclude the pre-existing untracked `zz_diag` explicitly rather than launching it accidentally or claiming a full suite passed. No redundant full-suite loops without a new reason.

Maintain `STATUS.md` continuously: completed checkpoint, active package, next queue entry, blockers, claims and retained artifacts. Before context/usage exhaustion finish or safely checkpoint the current operation; do not attempt to bypass usage limits. Leave runnable commands and a continuation instruction, no orphan processes.

`MORNING-REVIEW.md` groups cards **by title**, so the user can review multiple operations in one pass: exact source/output identities, what to observe, restore steps, evidence limits and dependencies. Keep unsafe/unready write cards withheld. Start the final receipt with what users can now do, then tests, unresolved issues and cleanup. No progress pings or intermediate review stops are needed overnight.
