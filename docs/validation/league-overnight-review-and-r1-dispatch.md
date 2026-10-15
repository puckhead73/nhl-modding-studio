# League overnight independent review and correction dispatch

Claude session: start a new session. The overnight queue is complete and broad; use a fresh correction session with this bounded review instead of carrying the entire implementation history forward.

Reviewed by Codex, 17 September 2026. Acceptance withheld for the three findings below. Existing copied-target acceptance predates this new archive producer and frontend integration; it does not certify them automatically.

## Evidence and scope

Receipt SHA-256 matches 7352982c88ec72a4a237134824604290fc9e5388503c4e89b1356c0b6e7e80d4. Of its 70 hash-listed files, 69 match the recorded prefixes. Only docs/receipts/league-overnight-checkpoint.md differs (current SHA-256 2ddc047fd27613010bbd7dce316510a803d3c2a7fd480c790e251a95e32b8de2); it now contains the short closed-run pointer. Correct that bookkeeping in the next receipt. All hash-listed implementation files match.

Fresh independent checks, all using local/runs/league-overnight-2026-09-17/target:

- `cargo test --locked --offline -p studio-api --test leagues --test league_copied_target --test league_offline_campaign`: 3 + 3 + 1 passed. The campaign reran the 28 fixture/status rows, not game runtime.
- `cargo test --locked --offline -p studio-domain --test leagues --test pending`: 6 + 3 passed.
- `cargo test --locked --offline -p tdb-loc`: 2 unit + 4 integration tests passed.

Total: 22 passing tests. Read the new archive producer, frontend review lifecycle, pending baseline implementation, localization parser and campaign evidence. No independent desktop/browser interaction or in-game run was performed; real-archive ignored tests and full developer checks were not rerun. Source-inspected findings below are distinguished from the executable API reproduction. No application source changes, staging, commits or pushes by Codex.

## F1 — archive and loose database can come from different revisions (P1; source inspection)

crates/studio-api/src/registry/copied_target.rs:940 packages primary bytes inside one with_workspace call, releasing the registry's inner mutex afterward. It later calls copied_target_review_with, which reacquires the workspace at line 1068 and stages the loose DB and plan revision. Its extension closure uses the earlier captured packaged bytes for the archive replacement.

An edit between those two critical sections yields archive revision A and loose DB/plan revision B. Apply only validates B against the current workspace; it cannot detect that the archive payload came from A. Both artifacts can therefore be internally well-formed and committed in one transaction while disagreeing about the roster/database content. This is a code-path finding; the concurrent interleaving was not dynamically forced in this review.

Required: derive the archive member bytes from the same immutable packaged artifact/snapshot used by stage_workspace, or capture and validate one common revision across all production. Avoid a second unconstrained session read. Add a deterministic concurrency regression with a controlled edit at the original capture/stage boundary; assert either refusal before writes or byte-identical packaged primary and decoded archive member from the accepted revision. Include save/rebind and cancellation/error cleanup in the existing transaction checks as relevant.

## F2 — orphan PS4 league references can be saved (P1; reproduced)

O1's brief required target-gated league-to-team relationship validation. The receipt explicitly substitutes catalog reporting for validation. That is insufficient: the generic editor can create a new reference to no stockleagues row and copied-target save commits it.

Independent reviewer probe on a disposable NHL 21 anchor copy:

```text
LEAGUE 31 BEFORE: false
SET TEAM 0 LEAGUE TO ABSENT 31: Ok(())
APPLY INVALID REFERENCE: Ok("committed")
```

The operation is Registry::set_field(project, "kOtt", 0, "xMjj", FieldValue::Int(31)), followed by copied_target_review/apply. The 5-bit field permits 31, but the target's stockleagues rows cover only 0..30. A UI picker that omits 31 does not protect API, bulk or generic editing paths.

Required: enforce the evidenced PS4 foreign-key relationship using existing validation patterns, gated to layouts with the native league table. Refuse a newly introduced dangling reference without changing working bytes, pending state or destination files. Test generic/bulk editing and save validation as appropriate. Preserve raw/candidate browsing of pre-existing unknown values; do not silently repair them or impose PS4 rows on older titles. Add valid-reference and pre-existing-violation cases to avoid overblocking legitimate edits.

Probe: E:/Repositories/nhl-modding-studio/local/reviews/league-overnight-probe/src/main.rs, SHA-256 0050b3460c40c977b992b5dc59d57c6e5a9318b7e08cedbe53e0d2ed0a5c4197. Command: `cargo run --offline --manifest-path local/reviews/league-overnight-probe/Cargo.toml --target-dir local/runs/league-overnight-2026-09-17/target`. The first independent dependency resolution failed on a cache package; copying the existing repo lockfile into this isolated harness allowed the successful offline run without changing the repository lockfile. Fixture: local/reviews/league-invalid-reference-52016. Only a copied DB was written; no source anchor or installed game changed.

## F3 — frontend review ownership does not follow its configuration/lifetime (P2; source inspection)

app/src/components/CopiedTargetPanel.tsx:

- Lines 31–56, 187–220: root, archive-enabled, archive path and member path remain editable with an existing review. The stale calculation only compares undo depth, redo depth and dirty. Changing the destination/options leaves Apply enabled for the old backend handle. Example: prepare a loose-only review, enable archive replacement, then Apply; the existing handle still contains only the loose file. Changing root similarly changes displayed status without changing the root the handle will write.
- Lines 67–86 and 110–115: unmount cleanup discards only a handle already held in handleRef. If navigation/unmount happens while review creation is pending, cleanup sees no handle; the later response has no mounted owner to dispose of the backend set. The component lacks a request-generation/unmounted guard. Project switches and late refresh callbacks also need ownership checks so an old project's response cannot update the current project's context.
- Undo/redo depths are not a content revision: undo followed by a different edit can return to the same depths/dirty value. Backend checks prevent that stale write, but the promised frontend stale detection is unreliable.

Required: bind a prepared review to project, server revision and the entire destination/archive configuration; invalidate/discard it when any binding changes, or freeze those controls until disposal. Guard asynchronous review completion across navigation/project switches, disposing late handles safely and surfacing cleanup failures through a durable owner/status path. Do not let an old refresh overwrite a new project summary. Preserve cancel-while-applying behavior.

Add component-level interaction tests with deferred API responses for late review creation/unmount/project switch and configuration changes, plus edit→undo→different edit. Verify no stale apply, no lost prepared ownership and no cross-project update. Perform a real desktop smoke run if tooling permits after the corrections; mocked component tests are not desktop or in-game evidence. No frontend runtime reproduction is claimed by this review.

## Next executable prompt — LEAGUE-R1

You are the sole Studio developer in E:/Repositories/nhl-modding-studio. Start a fresh Claude session. Read CLAUDE.md, the working agreement, the overnight brief/receipt and this review. Preserve the dirty tree and unrelated work.

Implement **LEAGUE-R1**, addressing F1–F3 together without expanding feature scope. Correct the shared snapshot boundary, enforce native PS4 league-reference validation and make the frontend review lifecycle robust. Add the precise regressions above; do not merely hide bad choices in the UI or rely on happy-path tests. Keep existing S2-C reservation/recovery behavior and pending baseline semantics intact.

Run focused domain/API/localization/archive checks appropriate to touched code, existing copied-target regressions, the offline league campaign, generated-binding checks where changed, frontend interaction tests/type/build checks and the thin shell check. Independently distinguish structural/API/component/desktop/game evidence. If desktop tooling is unavailable, record it as an unexecuted gate and provide exact manual smoke steps; do not substitute DTOs for UI evidence. No live installation or game-file modification outside disposable copied fixtures.

Correct receipt overclaims: not every title/code has a confirmed label; gen7 dictionary decoding does not establish code-to-key resolution; NHL 19/21 dictionaries are missing from supplied fixtures, not absent from the shipped games. Update the checkpoint hash. Do not mark Phase 1.1 or the overnight queue independently accepted.

Use durable receipts and self-verification without intermediate preference/review questions; no progress updates are needed during the correction. Begin the final receipt with a short layperson summary, then findings, changed-file hashes, fresh checks, limits and exact scratch ownership. Stop after this bounded correction for Codex review. The prior overnight queue authorization does not authorize unrelated phases or live deployment. No staging, commits or pushes.

## Cleanup and next roadmap step

The overnight target cache remains an active LEAGUE-R1 compilation/review dependency, not abandoned scratch. Retain the reviewer probe and local/reviews/league-invalid-reference-52016 only for this correction's reproduction; the fixture's committed copied-target transaction is disposable after durable regression evidence is retained and review closes. Engine staging root was empty after independent tests/probe. No probe/test processes remain. The reported pre-existing nhlms-p3-op-24084 directory remains unowned; do not delete by prefix or age.

No package-closeout cleanup is claimed while corrections remain active. On acceptance remove obsolete owned cache/probes/copied fixtures safely, preserving durable tests/receipts. Next is corrected UI smoke verification, then a separately scoped runtime test plan. In-game execution is not authorized by this review. Phase 1.1 remains open; Phase 1.2 and wider league localization work have not been dispatched. Projects/Builds remain optional final stretch.
