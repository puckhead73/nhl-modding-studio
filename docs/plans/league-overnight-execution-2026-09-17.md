# Autonomous overnight league development

User authorization: 17 September 2026. Claude may execute the queue below without user updates or intermediate review stops, for as long as available usage and the execution environment allow. This explicitly overrides older instructions to stop for Codex after every package, including L1.1-A's assessment-only stop. Codex remains coordinator; Claude remains sole Studio application developer. No scheduled job or external Claude session is launched by this document.

## Assessment disposition and corrections

L1.1-A is accepted as a planning input, with the corrections below. Codex verified the delivered hashes (matrix 91fbf782a32271e51904d1d94794bbb424cf1ca00524d2fc65f15dd97e75367b; receipt 29298af375e4a52ca8e5d9359323a44e42b88885167e1c84d79ec624d14314d3), inspected the global league enum and Team classification field binding, and checked EditSession's baseline/undo semantics. Codex did not independently rerun the 27-anchor measurement campaign or qualify any runtime claims.

Correct these assumptions before building on them:

1. Pending changes must compare the acknowledged loaded/saved baseline with current working bytes. **Do not use undo[0] as the baseline.** EditSession::is_dirty compares against project.db_bytes(); rebind_saved_baseline updates the project while historical undo snapshots can remain. Save → undo is a new pending difference against the saved baseline. Test this explicitly.
2. An edited TDB's other logical records/fields must remain unchanged, but checksum bytes and necessary container bookkeeping can legitimately change. Replace the matrix's claim that every other byte is unchanged with an exact allowed-difference audit covering the edited field and derived integrity bytes. Restore must reproduce the original whole-file hash.
3. NHL 21's dictionary is missing in the assessed fixtures. TXT_ROW_LEAGUE is a verified key in other supplied dictionaries, not yet verified against NHL 21's installed dictionary. The proposed NHL 21 edit is an offline field round-trip test until target-specific dictionary and runtime evidence exist.
4. Similar PS4 layouts do not turn inferred gen7 league labels into confirmed target-specific labels. Preserve candidate provenance, show raw codes where uncertain and never silently reuse a neighboring game's dictionary. Recompiled remains untested without its own fixture. Expand grouped matrix entries into 27 explicit shipping-target rows for execution tracking.
5. Absence of a league-name field in a main DB does not prove every possible identity consumer is localization-only. Record precisely what was inspected, distinguish code identities from display names, and keep unresolved UI/executable consumers open. A reader alone also does not implement editable localization.

## Execution contract

Read CLAUDE.md, the coordinator/developer working agreement, entity-editing-parity-roadmap.md, league-identity-parity-matrix.md, the L1.1-A receipt, EDIT-MODEL.md and relevant format/QA records. Work on the current dirty branch in place. Do not reset, switch branches, create a clean HEAD worktree that omits uncommitted work, stage, commit or push. Preserve other streams and user originals. No live game installation or edits to anchors.

Continue through the queue automatically. Each package has an implementation/verification checkpoint, **not a human approval gate**. Fix failed checks before building dependent functionality. If an external fixture, unknown format or unavailable runtime blocks a package, record the exact blocker and continue the next independent package. Do not label blocked work complete or bypass validation to manufacture progress. This authorizes advancing supporting work while Phase 1.1's runtime parity remains blocked; it does not declare Phase 1.1 complete or authorize skipping its later qualification.

Make routine architectural choices using current patterns and document significant choices in the receipt. Ask no overnight preference questions. If an action truly requires new authorization, leave that action pending and continue authorized work. Do not send progress messages, notifications or repeated status updates. Keep durable checkpoint files instead; produce one consolidated final handoff when the queue is finished, all remaining work is externally blocked, or usage/context limits require stopping. Do not repeatedly poll limits, evade quotas, consume reset credits, purchase usage or schedule automatic future runs.

Keep one active implementation package. No parallel application writers. On context compaction, resume from the checkpoint; do not repeat finished work. Before a context/usage stop, record exact next command/task, last passing checks, changed files, active processes, blockers and scratch ownership. Completion here means developer-verified and pending later independent Codex review, not independently accepted or released.

## Queue — work in this order where dependencies permit

### O1 / L1.1-B — Correct target-specific league identity

Implement the shared domain model and label resolution before the UI. PS4 league identities come from the current session's stockleagues rows, joined by leagueid, not row position or display string. Older layouts expose derived league-code groups; do not fabricate writable stockleagues records. Select vocabularies using explicit target/schema evidence, never a global enum or bit-width alone. Preserve raw identifiers and confidence/source information.

Correct the Team Data League cell to refer to the real league field and expose League group separately where present. Do not reinterpret conference/division group IDs as legacy enum values. Trace headers, lists, filters and Transactions so they share the same resolver. Expose existing membership only; no new league/team topology operations. Prevent duplicate-label collisions by using stable native identity, not label text, as selection keys.

Add a league-to-team relationship check only for targets with an evidenced native table relationship; do not impose PS4 foreign-key rules on gen7. Preserve existing validation behavior for known pre-existing violations and report rather than silently repair them.

Checkpoint: representative differing code layouts and all available anchors exercise correct lookup selection; unknown codes remain explicit; identical bytes across target fixtures do not count as separate runtime qualification. Key fields remain protected. Update matrix corrections and explicit per-target rows.

### O2 / L1.1-C — League API and Studio editing surface

Add typed league summaries/details and commands through existing domain/API/thin-shell patterns; generate bindings using the exporter. Add Leagues navigation, list/detail, member-team links and existing-slot identity editing. Display localization keys as keys when unresolved. Older games show useful code/member information with a clear reason when a name cannot yet be edited. Include search, empty/error/loading states and target switching without stale labels.

Use existing field editing, undo/redo and validation. Enforce byte-length/encoding constraints (not just character counts), immutable primary keys and field availability. Do not expose unknown render/art semantics as a confident identity-edit feature. A renamed/repointed key must not invalidate a detail route or cached entity identity. Extend the QA entity selector using a stable league identifier; name-key lookup may be an optional convenience but must reject ambiguous matches.

Checkpoint: domain/API tests cover read, valid edit, oversize/multibyte rejection, key refusal, undo/redo and target changes. Test frontend behavior using available repo tooling; screenshots of a real running app are UI evidence, DTO dumps only API evidence. If desktop/browser execution is unavailable, record that limit and continue backend checks.

### O3 / L1.1-D — Accurate pending-change review

Implement a baseline-to-working change summary using the actual session baseline. Cover the primary DB and attached roster without conflating file roles or double-counting mirrored changes. Show entity/table/field, native identity and before/after values with truthful labels. Net-zero edits must disappear. Handle row additions/removals or unrepresentable changes explicitly instead of silently omitting them. Do not require per-entry undo for this package.

Reuse current state and avoid maintaining a second divergent edit log. Keep large DB diffs responsive using measured caching/pagination or equivalent current patterns. Invalidate on edit, undo/redo, save/rebind, reload, restore and target changes. Preserve a visible integrity summary for derived checksum/container changes rather than presenting them as user edits.

Checkpoint: edit→undo clean, edit→redo, repeated edits to one field, save→clean, save→undo dirty against the new baseline, restore/reload, attached-roster changes and rejected edits all show accurate pending results. No fake zero-changes state on diff errors.

### O4 / L1.1-E — Copied-target review/apply/recovery UI

Wire the accepted S2-C API into the direct-editing pending review. Provide copied-root selection/validation, review of destinations and dependencies, apply, outcome/status, reload and explicit restore/recovery actions. Keep the engine's marker/containment/identity gates, server-issued handles, revision checks and reservations. Reuse existing onboarding/root-creation APIs where available; do not silently designate an installed game directory as disposable.

Handle stale review, cancellation, busy aliases, dirty reload, partial/failed refresh and staging obligations accurately. UI must not report a refused or unqualified save as a successful in-game installation. Discard obsolete review handles when a panel closes, review changes, navigation changes or a session closes, using the backend lifecycle and respecting errors. Do not create a new writer or substitute legacy save for a failed coordinated apply. Existing Save As can remain clearly scoped to its existing behavior.

Checkpoint: verify a complete league edit→pending list→review→apply→reopen→restore on a copied fixture, cancellation and changed-after-review refusal, and error presentation. Re-run focused S2-C regressions if touched. Use real UI automation/smoke evidence where possible; report unavailable UI execution without claiming it passed.

### O5 / L1.1-F — Cross-target offline campaign

Run the reversible equivalent identity operation on every available PS4 main-DB fixture, resolving records semantically then retaining stable IDs. NHL 21 is the primary case; use target-valid equivalent rows on earlier games, with fixture hashes and why each is equivalent. Verify intended logical delta, valid integrity fields, unaffected identities/membership, save/reopen and byte-exact restore. NHL 19/20 duplicate provenance remains a limitation.

For older targets, verify league browsing/classification and truthful blocked-name-edit behavior. Do not convert a membership edit into evidence for renaming a league. For each of the 27 shipping combinations record source identity, operation, domain/API/UI evidence, structural round-trip, packaging, runtime and restore separately. Research-only NHL 07 PS3 is supplemental. Missing fixtures remain blocked. Do not run arbitrary installed games overnight.

Checkpoint: produce docs/validation/league-identity-offline-campaign.md with reproducible commands and per-target outcomes. Address discovered regressions before continuing dependent work.

### O6 / L1-support-LOC — Localization reader needed for correct league labels

This is explicitly authorized supporting work from Phase 1.3, while Phase 1.1 runtime gates remain open. Reuse the user-supplied reference at C:/Users/puckh/Documents/ChatGPT/NHL 21 Legends Project/audit/localization-developer-handoff and existing repo format evidence. Read those resources before coding; preserve the originals. Do not use unrelated tool repositories.

Implement production Rust read support for the evidenced little-endian varchar/Huffman dictionary layout, with bounded offsets/counts, encoding, pool and CRC validation. Integrate target-and-locale-specific lookup into league labels. Keep keys and localized strings distinct, retain unresolved-key fallback in Studio without claiming the game uses the same fallback, and prevent cross-title cache leakage. Supplied neighboring dictionaries are comparison fixtures, not substitutes for missing NHL 21 data.

Then investigate the supplied big-endian dictionaries with a bounded evidence-driven pass. Implement BE decoding only where measurements support it; otherwise preserve a minimal reproducible unknown and continue. No arbitrary dictionary editing, new-key insertion or hashing/pool rewriting is required by this queue.

Checkpoint: known reference strings and full supplied-fixture row counts agree with the authorized reader; corrupt/truncated/out-of-range inputs fail safely; available BE fixtures are independently checked if implemented; no native integer/endianness assumptions. Re-run league label/UI checks after integration. Save durable evidence and remove regenerable exports.

### O7 / L1-support-EB — PS4 copied-archive replacement prerequisite

If time remains, address the identified 20-byte-row EB archive writer gap using docs/formats/ps4-eb-big.md, existing codecs/planner and authorized available fixtures. First verify current capabilities; do not duplicate an already implemented writer. Scope is replacement of an existing DB member in a copied archive, preserving archive metadata/names/hashes and unaffected members. No archive entry addition/deletion, live deployment, codec substitution or guessed unknown fields.

Implement only supported/evidenced layouts; refuse other variants. Preserve untouched compressed payloads byte-for-byte, maintain alignment and checked wide offsets/lengths, validate reconstructed archive structure and decoded replacement, and use existing staged transaction/restore infrastructure. Do not load multi-gigabyte archives wholesale without a justified bound. Do not automatically process the 17 GB stadium archive merely because it is available.

Checkpoint: no-op/existing-member replacement, growth/shrink where supported, unchanged-member hashes, truncated/invalid layout refusal, failure/cancellation and exact restore on copied fixtures. Synthetic coverage alone does not qualify real PS4 archives; real fixture absence is a blocker. Successful offline repacking does not establish game acceptance or loose-file precedence.

### O8 — Consolidation, verification and handoff

Fix integration regressions, remove dead interim paths, refresh generated bindings, and run relevant Rust tests, changed-crate/shell checks and frontend type/build/tests. Run formatting/clippy appropriate to changed scope; distinguish pre-existing failures without reformatting unrelated code. Repeat expensive checks only after relevant changes. Recheck anchor hashes and ensure source fixtures were untouched.

Produce one consolidated receipt at docs/receipts/league-overnight-2026-09-17.md beginning with a short layperson summary. Include completed packages, developer checks and exact commands, remaining external blockers, changed-file hashes, implementation choices, UI evidence versus API-only evidence, per-target status, cleanup results and the best next task for Codex review. Do not claim independent acceptance. If all feasible work completes, stop; do not expand into teams/assets/audio or unrelated feature queues merely to use remaining tokens.

## Durable progress and cleanup

Maintain docs/receipts/league-overnight-checkpoint.md with queue state (not started/in progress/implemented+checked/blocked), last verified source state, next action and package ownership. Update it at meaningful checkpoints, not every tool call. Child receipts may be concise sections in the consolidated receipt; no human stop is needed between them.

Adopt the L1.1-A scratch root documented in its receipt only for O1 mapping verification: H:/TEMP/claude/e--Repositories-nhl-modding-studio/39435b70-83f0-4d5f-989f-713865ad26da/scratchpad/l11a. Inspect scripts before execution. Preserve necessary reproducible checks as durable repo tests/evidence, then remove this exact obsolete scratch once O1 no longer needs it.

Use package-owned scratch under local/runs/league-overnight-2026-09-17 and a declared build-cache subdirectory. Transfer a cache to the next package only for an actual compile/test dependency. Clean completed packages' obsolete copied targets, exports and builds as work advances; do not retain everything for morning review. Preserve unresolved recovery material until resolved, then clean verified owned paths. Final handoff includes exact removed/retained paths and reasons. Preserve source/tests/receipts and user files; no broad temp sweeps or git clean/reset. Stop owned app/test processes when no longer needed.
