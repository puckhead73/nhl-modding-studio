# Overnight research brief — 19 September 2026

You are Claude, the research lane, starting a fresh session in `E:\Repositories\nhl-modding-studio`. Work ahead of the sole developer for as long as usage/context allow. Publish bounded usable handoffs as you go; do not wait for morning review or the developer. Save all visual/manual experiments for the user tomorrow. This campaign supersedes earlier research stop instructions for the queue below. New findings remain researcher-verified and pending Codex review; do not mark them independently accepted.

## Starting context and ownership

Latest developer checkpoint: P1-C-R2 is independently accepted offline. The overnight developer begins with P1-BS-1 scoped settings decoding, not competition-group corrections. This does not change your R1-first research queue or ownership.

Read `docs/plans/coordinator-developer-working-agreement.md`, `docs/plans/entity-editing-parity-roadmap.md`, the Phase 1 coverage matrix, research atlas `docs/research/overnight-2026-09-18/00-ATLAS.md`, and `docs/validation/p1bs-r1-acceptance-and-csv-parity-dispatch.md`. Use the latest revisions of handoffs 16–22, especially LOC-W3 revision 3 and settings handoff 22 revision 2. Older matrix text may lag accepted handoffs; reconcile evidence, do not re-research already closed mappings.

Own new work under `docs/research/overnight-2026-09-19/` (`00-ATLAS.md`, `STATUS.md`, numbered handoffs, data and developer vectors), probes/scratch under `local/runs/overnight-research-2026-09-19/`, and your own coordination claims. The developer alone edits application code, definitions, bindings, tests, implementation matrix and consolidated morning cards. You may update the previous research atlas with links and incorporate specifically requested handoff-22 wording corrections. Do not change active developer files or erase existing evidence.

Inventory facts: Xbox games are installed under `X:\Games\Emulation\Roms\XBOX360`; reuse handoff 01. The repository NHL 20 PS4 anchor is NHL 19 base; use the real CUSA15439 source. PS3 dictionaries differ from Xbox. Legacy Recompiled reuses Xbox Legacy payload research; inspect candidate roots rather than assuming `_compiled` exists everywhere. Shared schemas justify shared parsers, not unmeasured title-specific meaning. NHL21-E1 is accepted and closed.

## Nonblocking two-lane protocol

Use `local/coordination/overnight-2026-09-19/claims/`. Write only `research-<topic>.json`; the developer writes `dev-<topic>.json`. Fields: owner, topic, status (`active`, `ready`, `blocked`, `complete`), owned paths, dependencies, outputs and timestamp. Publish through temporary sibling then replacement. Each ready handoff names its source identities, version/hash, exact evidence and unresolved assumptions. Never silently rewrite a ready handoff already consumed; issue a revision and explicit correction notice.

At package boundaries read the developer's claims/status. If it owns research on a topic, choose the next unclaimed topic. If it needs your active topic, publish the smallest coherent verified checkpoint first, then continue. Never wait for acknowledgement. Do not assume an unattended agent stopped because time elapsed; avoid taking its active files. Claims are coordination, not proof. Duplicate discoveries can be documented separately and reconciled later.

## Ordered research queue

### R1 — P1-BS-CSV: settings defaults parity and packaging

**Completed before overnight dispatch.** Handoff `01-ps4-settings-csv-defaults-parity-and-packaging.md` and its data are delivered. Read `docs/validation/p1bs-csv-review-and-overnight-addendum.md`; reconcile its bounded contract/reporting clarifications and clean the reviewed package's owned scratch as your first checkpoint, then start R2. Do not rerun this census or stop for another correction-only review. The original R1 scope below is retained for context, not a fresh task.

Execute the detailed prompt in `docs/validation/p1bs-r1-acceptance-and-csv-parity-dispatch.md`, then continue this queue rather than stopping. Close the real NHL 20 CSV provenance and NHL 19 patch-member gaps. Map NHL 16 vs NHL 17–21 schema variants, exact setting/context row identities, global/context storage, constraints and title-specific choices. Supply a stored-default editing contract that preserves original CSV encoding, quoting, separators, line endings, unknown columns and all non-target cells. Identify actual wrapper chains, changed compressed-length handling and shared P1-F dependencies by inspecting current Studio code read-only.

Incorporate the accepted handoff-22 clarifications: 21 IDs across 20 displayed rows; valid same-length cell edits do not alter unchanged header CRCs; previous edits do not waive validation; subgroup-order evidence is structural; profile differences are associations. Fix stale universal-match prose in JSON without changing measurements. Do not claim loader precedence or selected-value storage from static copies.

### R2 — Resolve packaging evidence gaps that block Phase 1

Reuse handoffs 16/18/20/40 and existing vectors. Catalogue requirements for safe RefPack/chunkzip wrappers and BIG4/BIGF/EB member size changes across measured families, including NHL 15 PS4's EB variant and preserved declared-size anomalies. Distinguish byte layout proved by measurement from unexplained words and loader questions. Prioritize precise questions the developer records, not a broad codec rewrite.

Deliver minimal source-bound vectors, boundary cases, exact preservation masks and refusal cases sufficient for offline implementation. Changed member sizes require accounting for enclosing sizes/offsets/alignment; raw length equality is not compressed length equality. Disc images retain the same-length restriction. Do not resolve unknown archive words by guess or call structurally valid output game-qualified. If a problem requires runtime, produce the discriminating card and move on.

### R3 — Non-logo league imagery (P1-G)

Map selection tiles, banners, backgrounds, league-specific/shared fallback artwork and layout references across installed targets. Start from known league ids/art indices and measured asset grammars. Publish actual consumers/references where discoverable; filename resemblance is only a lead. Identify which assets alias one another, dimensions/mips/formats, packaging chains and the smallest preview/replace operation. Reuse known codecs. Separate scoped absence, inaccessible containers and unsearched areas. Produce contact sheets or previews for tomorrow without asking the user to review them now.

### R4 — League audio consumers (P1-H)

Extend existing audio handoffs 15/19 rather than repeat their path inventory. Inspect generic frontend/SFX/music bank structures and PS3/PS4 coverage to determine whether any records are league-keyed, shared or indirectly selected. Do not infer absence from no league-named file. Document id joins and parsing confidence, available extraction/decoding, and feasible reassignment/replacement versus blocked formats. No playback/game launch overnight. Goal-horn mapping is primarily Phase 2: pursue it here only as a bounded shared-format dependency; otherwise leave the lead for Teams.

### R5 — Remaining Phase 1 blockers, targeted only

Read the developer's updated matrix/claims and choose the highest-value unclaimed static gap: NHL 07 label source, locale coverage not already measured, NHL 15 patch group witness, unknown group identities, container/codec specimen gaps or settings equivalents on gen7. Do not spend the entire run repeating blind searches for ids 77/78 or loader behavior. Bound each investigation by a concrete source set and hypothesis; publish negative results scoped to that set and move on when it becomes runtime-dependent.

No generic claim that executable-backed rules are outside roadmap scope. They remain an evidence/implementation question, but this run does not patch executables. Do not turn a conjectured field into a writable contract just to unblock development.

### R6 — Research ahead through the remaining roadmap

When immediate Phase 1 research is complete, actively owned or externally blocked, move ahead in order while the developer remains on Phase 1. Prepare implementation-ready per-operation cross-title maps, not a premature Phase 1 closure claim:

1. **Teams:** identity and related DB entries/joins; localization; asset selection; arenas and ice; jerseys/pants/kits; horns/PBP/music. Reuse handoffs 19/20/23 and team asset joins. Prioritize unresolved semantics and coverage gaps over re-counting known files.
2. **Players:** identity, relationships/rosters and additional DB entries; localization; portraits/faces/other imagery; name/PBP audio. Preserve per-title field encodings, reference namespaces and capacity limits.
3. **Equipment:** sticks, gloves, helmets, skates and goalie gear; enumerate ids, bindings, model/texture/container families and preview/replace dependencies.
4. **Miscellaneous:** UI, boards, scorebug and menu music, each with exact source chains and shared-resource consequences.

Finish one bounded operation's cross-title map before jumping to another. If the next topic is actively owned by developer research, take the following unclaimed operation and record the skip. Unknown/runtime-dependent cells remain explicit. Do not research Projects/Builds.

## Evidence and handoff requirements

Each handoff starts with a short plain-language summary, followed by: per-title/platform/build sources and hashes; observed vs inferred vs unknown; exact field/asset/container mappings; supported/refused operations; preservation and validation rules; minimal developer vectors; user cards with treatment/control/restoration and bounded outcomes. Keep schema evidence, offline round trips, desktop behavior and game consumption as separate levels. No sibling-title dictionary or schema proxy presented as that title's measurement.

Cards must distinguish no visible effect from wrong placement, rejected treatment, cached/profile choices or another consumer. An observation establishes only its tested scope. The user decides acceptance; do not re-open their accepted NHL21 test or impose screenshots as a pass requirement.

Use existing read-only parsers/probes and dependencies where possible. No writes to originals, installs, archives/images, anchors or user saves; no launches, deployments, quarantine, staging, commits or pushes. Do not execute unfamiliar external code just because a handoff mentions it. Inspect tools before use and prefer repository implementations.

Check disk before extractions. Extract the smallest necessary members/ranges; delete owned regenerable extracts after hashes/results are durable. No retained decrypted executables, bulk archive/image copies or duplicate build trees without a named active dependency. Validate containment/reparse points before cleanup. Never remove developer/shared caches. If blocked by space or access, record it and continue independent work that fits.

Update your atlas/status and ready claims at each checkpoint. Save all review requests for morning. Before usage/context exhaustion leave completed handoffs, the active investigation state, reproducible commands, next queue entry and exact cleanup/retention manifest; stop owned processes. Finish with one concise closing receipt, not a claim that research alone completed product parity.
