# LEAGUE-R2 acceptance and desktop smoke dispatch

Claude session: start a new session. The reference-validation correction is accepted for the tested source/domain/API scope; the next package is actual desktop workflow verification.

Reviewed by Codex, 17 September 2026. No remaining blocking finding in this bounded correction. This closes LEAGUE-R2, not Phase 1.1 or runtime qualification of the overnight features.

## Independent evidence

Receipt SHA-256 matches e44e878bedba12e824dba8dd85410510a3338d6ff22748720d8d4a77b36736fa. All five implementation hash prefixes in the receipt match the working files.

Reviewed referential.rs and its integration into EditSession: new violations are compared by multiset identity (metadata row key plus referenced value), not total count. The inspected regressions cover equal/lower counts, pure repairs, unrelated edits and ordered-shift/swap-pop row movement. The documented keyless-table fallback remains a limitation; this review does not generalize it to future schemas.

Fresh independent checks with `--target-dir local/runs/league-overnight-2026-09-17/target`:

- `cargo test --locked --offline -p studio-domain --test referential`: 5 passed.
- `cargo test --locked --offline -p studio-api --test league_copied_target`: 7 passed, including shared archive snapshot, valid save/restore and the new Registry-level invalid-substitution case.
- Existing reviewer harness rerun unchanged (source SHA-256 d87e71a8eecc23a5d092819029869ffc0b1e5c057e3ed0497dbdde68a2fcbf48): both original and substitution edits refused.

```text
SET TEAM 0 LEAGUE TO ABSENT 31: Err(... league = 31 refers to no stockleagues row ...)
NHL18 LEAGUE 30 EXISTS: false
CHANGE INVALID 31 TO INVALID 30: Err(... league = 30 refers to no stockleagues row ...)
TEAM 0 AFTER: Some(31)
```

The original harness's following `Ok("committed")` is a no-op save after the rejected NHL 21 edit. The NHL 18 source remains the deliberately seeded pre-existing-invalid fixture, unchanged by the rejected API edit. No anchors or installed games were modified. No application source changes, staging, commits or pushes by Codex.

Prior LEAGUE-R1 source/API/component findings remain as recorded; its 21 Rust and six component tests were not rerun or counted as fresh here. Desktop and in-game work remain unexecuted. Native desktop UI control is unavailable in this Codex task's exposed computer-use interface; no mocked browser or API interaction is being presented as desktop evidence.

## Cleanup and transfer

Reviewed copied fixtures under local/reviews: league-invalid-reference-52016, -74364 and this review's -5012. Every journal in those fixtures was terminal (committed); engine staging root was empty and review/test processes finished. Minimal reproductions now live in durable domain/API regression tests and the prior review documents.

Remove the three exact fixture directories and league-overnight-probe after this durable record is written; record actual outcome below. Preserve unrelated historical S2-A/S2-B scratch and unowned temp directories. The cache at E:/Repositories/nhl-modding-studio/local/runs/league-overnight-2026-09-17/target transfers explicitly to **LEAGUE-SMOKE-1**, solely to compile/launch the same corrected Tauri candidate and verify any smoke fixes. Set CARGO_TARGET_DIR to that absolute path when launching. Remove it after smoke closeout unless a named active correction needs it; do not create another duplicate cache or retain it without a dependency.

## Executable prompt — LEAGUE-SMOKE-1

Cleanup outcome: all four exact reviewer directories listed above were removed successfully after absolute-path containment and recursive reparse-point checks. Durable tests and reviews remain. Only the named build cache transfers to the smoke task.

You are the sole Studio developer in E:/Repositories/nhl-modding-studio. Start a fresh session. Read CLAUDE.md, the working agreement, this acceptance, the overnight receipt and LEAGUE-R1/R2 receipts. Preserve the existing dirty tree. Scope is desktop smoke verification and directly discovered integration fixes, not another feature phase.

1. Inspect available desktop interaction tooling and launch configuration. The verified launch is `npm run dev` from the repository root, whose script runs `tauri dev`; beforeDevCommand runs `npm --prefix app run dev`, and the configured URL is http://localhost:1420. Use the explicitly transferred CARGO_TARGET_DIR. Do not claim an app launch is an interaction pass. If native interaction is unavailable, do not manufacture desktop evidence from jsdom/API calls: finish feasible launch/setup checks, preserve exact manual steps and report that gate blocked. Do not spend the whole package repeatedly trying unavailable drivers.
2. Create disposable, clearly owned marked copied roots and copy source fixtures into them through the established setup API/test helper. Keep anchors and installed games read-only. Record fixture identities and originals for hash comparison. Use NHL 21 PS4, plus one older target with a different league-code layout (for example NHL 14 PS3). Use existing schema-known scratch archives for archive UI exercise if real copied archive access is impractical; label synthetic versus real evidence explicitly.
3. Exercise the actual desktop UI: open copied DB; Leagues list/search/detail; member-team navigation; edit Allsvenskan's name key to TXT_ROW_LEAGUE; verify one intended pending change; undo/redo; review save; frozen config; subsequent edit makes review stale; discard and re-review; apply; refresh/reopen; exact restore. Verify original hashes after restore and record screenshots or a concise observed-interaction log. NHL 21 dictionary key existence/runtime text remains unverified without its own dictionary.
4. Exercise lifecycle cases in the running UI where feasible: leave the page with a prepared review, switch projects, return to inspect pending handles, wrong/non-marked root errors, cancellation and failed operations. Confirm old-project responses do not overwrite the current project or silently select an old target. Verify older targets show truthful non-editable league-name behavior and correctly scoped/candidate/raw labels. Where a race requires automated testing rather than manual timing, cite the existing component/API regression separately.
5. Verify save-in-place baseline behavior and save→undo pending differences on copies. Exercise a supplied target's own dictionary attachment (e.g. NHL 18) and unknown-key display; do not attach NHL 18's dictionary as verified NHL 21 evidence. Exercise the archive toggle and artifact review, observing that loose and archive payloads belong to one revision and qualification is disclosed. No in-game deployment.
6. Fix reproducible integration defects within these flows, add targeted regressions, and rerun affected checks. Avoid redesigning the editor or expanding localization writing, league topology, assets/audio, Projects/Builds or runtime deployment. Generate bindings through the exporter when contracts change. Do not stage, commit or push.
7. Start the receipt with a short plain-language summary; give exact desktop interactions and results, API/component evidence separately, changed hashes, checks, unresolved blockers and the next recommended action. No intermediate updates or preference questions needed. Clean completed package-owned temp fixtures, exports, logs/builds and processes after retaining durable evidence. Retain unresolved recovery material only with a specific dependency. Stop with the receipt for Codex review.

Deliver docs/receipts/league-desktop-smoke.md. If desktop interaction is unavailable, title its disposition blocked/not executed rather than passed, and leave a concise ready-to-run manual checklist. In-game execution remains a later separately authorized task.

## Roadmap

LEAGUE-R2 accepted for its correction scope. Next: LEAGUE-SMOKE-1, actual desktop workflow verification. Phase 1.1 remains open for UI and per-target runtime evidence; no new Phase 1.2 or localization-writing package is dispatched. Projects/Builds remain final optional stretch.
