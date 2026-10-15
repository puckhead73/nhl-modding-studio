# 20 September campaign — first independent review and correction dispatch

Disposition: campaign not accepted in full. Start a fresh developer session for OVERNIGHT-DEV-20260920-R1. This review checks selected write/UI paths; it does not certify the canonical dictionary serializer, every codec, all real-archive sweeps or Phase 2 coverage.

## Independent checks

- `cargo test -p tdb-archive-big --lib`: 69 passed, 1 ignored.
- ArchiveMemberReplace, ColorField and TeamLogoCard component suites: 17 passed.
- `cargo test -p tdb-loc --lib`: 7 passed. These seven tests do not exercise the new canonical rebuild contract.
- Two additional reviewer reproductions failed as described below. No game/desktop launch, live write, commit or staging. The temporary reviewer tests were removed after recording the reproductions; shared build cache preserved except artifacts attributable to the temporary Rust test.

## F1 — P1: conflicting whole-group replacements produce an invalid plan and staging panic

`crates/tdb-archive-big/src/stage.rs::plan_scoped` rejects duplicate entry indices but does not reject overlapping effective row/payload ownership introduced by whole-group edits. Two distinct entries can be aliases of the same span. Workspace coalescing is keyed by archive/member path, so distinct paths remain separate pending plans.

Independent reproduction reuses the existing `stage/tests.rs` Synth builder: point entry 1's first twelve row bytes at entry 0's; bind; request replacements for indices 0 and 1 using different `noise(40, 3)` / `noise(40, 5)` chunkzip payloads; call `plan_scoped(..., SharedSpanRule::PrivateCopy, &[0, 1])`. Planning succeeds. Calling stage then panics at `patched_prefix` (stage.rs:1474 at review), because the first edit already changed the old row expected by the second. This is a reproduced debug-build panic, not a claim of live corruption or tested release behavior.

Resolve overlapping effective ownership before accepting pending edits and before producing an archive plan. Refusal with the conflicting paths is sufficient; do not silently choose a winner or widen/narrow scope. Preserve previously pending edits on refusal. Retain an engine-level guard even if the UI/domain catches the conflict first.

Regressions: whole-group vs whole-group; whole-group vs private sibling in both orders; identical and different requested bytes; fit and append; separate non-overlapping groups; domain/Registry pending/revision/undo invariants. No accepted plan may reach an internal assertion or defer a known contradictory request to staging. Verify successful non-conflicting output and exact restoration.

## F2 — P1: colour draft can migrate to a different team

`app/src/components/ColorField.tsx` resets text only when the decoded colour string changes. FieldProfile keys cells by label, not target identity. Changing record/project with the same stored colour leaves the old draft alive; commit uses the new props.

Independent jsdom reproduction: render project 7, record 3, packed `feprimarycolor = 0xdd9b26`; enter `#123456` without committing; rerender record 8 with the same colour; blur. Actual dispatch: `set_cells_int { projectId: 7, table: "kOtt", record: 8, writes: [{field: "feprimarycolor", value: 1193046}] }`. The draft intended for record 3 is sent to record 8. Four existing colour tests passed; the added target-switch test failed.

Bind draft and mutation lifecycle to project, table, record and component-field identity. Reset obsolete draft/error state even when the two targets have equal colours. Capture mutation arguments, prevent duplicate commit dispatch, and keep refresh/completion attached to its owning context. The shared ProjectProvider already rejects foreign-project summaries; do not describe that existing guard as absent.

Add deferred component cases for team/project/field switches with equal colours, in-flight completion, unmount, and Enter/blur duplicates. Assert dispatched arguments and shared state, not only text. No desktop reproduction is claimed.

## F3 — P2: reconcile remaining work and stale blockers

The campaign delivered substantial offline implementation; Phase 1 itself remains open. Retain explicit gaps: BIG4 ordered reflow, NHL 07 mirror cascade, unsupported resize families, audio and other competition/localization questions. Do not label every remaining feature research-blocked without identifying its missing contract.

Reconcile the final receipt's PS4 vault binding and chunklzx statements with the latest research. Research has parsed typed PS4 vault links; that is not a production reader/writer or complete runtime selector. Distinguish decoder framing research from promotion into Studio, and both from an encoder/accepted substitute. Use exact handoff paths because multiple campaigns have a handoff 07. Recheck the NHL 07 label-source claim against the bounded search evidence rather than asserting a located executable text source.

Open a provisional Phase 2 operation-by-target matrix now for work already delivered. Mark developer evidence separately from independent acceptance; a capability flag, hash identity or listing test does not establish edit/save/restore parity. Keep this an accounting task, not permission to expand features during this correction.

## Next developer prompt

Implement F1 and F2, reconcile F3, then run the affected engine/domain/API/component tests, binding check if contracts change, and type check. Preserve copied-target recovery, private-copy behavior, prior undo history and unrelated dirty files. No new feature work or live writes in this correction package.

Move the campaign's retained sweep/aggregate logs to a durable evidence location with a manifest and link them from receipts; then clean obsolete package-owned edit scripts using verified containment. Do not delete shared caches, unrelated scratch or private inputs needed by tests.

Receipt: plain-language summary first, exact fixes, regression evidence, changed identities, omissions, cleanup and remaining independent-review areas. Stop for Codex review. Existing NHL21-E1 and earlier scoped acceptances remain closed.
