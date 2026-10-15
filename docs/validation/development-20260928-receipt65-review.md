# Coordinator review — receipt 65 checkpoint, 28 September 2026

## Disposition

**Original F1 backup-deletion defect closed for the tested in-memory recovery scope.** Source now retains the original backup on failed restoration and reports its path, or attempts a separate recovery copy if no suitable backup exists. Eleven Hockey Shop unit tests passed independently, including the original failed-restore shape, pre-existing backup, create rollback, temporary-name refusal and failed rename. This does not establish crash safety or protection against concurrent external writers.

**Head reassignment needs source revalidation before this path is reused for hair.** See F2 below. Normal PS4 happy-path tests do not cover a metadata-preserving source replacement. r64 remains an internal review candidate, not an accepted release.

Next dispatch: [development-20260928-after-receipt65.md](../plans/development-20260928-after-receipt65.md).

## F2 — catalogue cache can bypass current source identity at Apply

`Registry::ps4_head_catalogue` (reviewed lines 1308–1318) caches by title, install path, archive length and modification time. The relevant identity manifest/defs root and archive/member content are not in that key. `player_reassign` calls it for Check and Apply; the plan digest then contains the cached catalogue's old evidence. The current cache can therefore return an old identity-checked catalogue after relevant source bytes change while preserving length/time, or after relevant identity definitions change.

A reviewer scratch probe wrote different four-byte contents to one path and restored the timestamp: `metadata_key_unchanged=true`, `content_changed=true`. This confirms the key collision, **not a full Registry stale-vault execution**. The unsafe reuse follows from source inspection of the cache hit and Apply call path. Require a public-path regression and current content/identity validation at Apply; do not rely on length/mtime alone. Parsing caches may remain when keyed/bound safely.

The test of a different-title install covers cold-load mismatch, not this warm-cache case. The affected operation is a database reassignment validated against attached assets; the reviewed asset source must be current. Withhold blanket acceptance of that guarantee until corrected.

## Additional bounded corrections

- `WriteTxn::write` calls `write_new`, then on **any** failure removes the temporary path if it exists. Because `write_new` returns the same `io::Result` for open/create failure and a later write failure, an `AlreadyExists` result after the initial precheck is not proof that this transaction owns the file. Preserve creation ownership explicitly before cleanup. This is source inspection, not a reproduced concurrent-install failure. It does not reopen original F1, but qualifies the claim that unowned files can never be removed.
- The old `assets.rs` test at `nhlms_ps3_dds_portrait` reuses a fixed temp directory. Give future runs exclusive owned scratch; leave the existing unowned directory alone.
- R15 identifies `ge_nhl_skaterhairtexturepack` as the class and `GenericHairStyle` as the typed attribute. Hair continuation is authorized within the measured NHL 17–21 existing-id scope after the source-check corrections, with runtime appearance unclaimed.
- Internal artwork permission and external replacement remain settled; no repeated artwork/holder-choice question was sent. No broad release or earlier-receipt acceptance is granted here.

## Review scope and independent evidence

Read consolidated receipt 65 and package receipts 61/63; inspected transaction write/rollback helpers, relevant unit regressions, head catalogue cache/load/type join, Registry Check/Apply, reassignment fingerprint, R15's head/hair binding and the reused-temp test location.

- `CARGO_INCREMENTAL=0 cargo test -p studio-api --lib hockey_shop`: **11 passed, 0 failed**, 0.14 s; existing lifetime/dead-code warnings noted.
- r64 `verify.py check` on the actual zip and a fresh reviewer-owned extraction: **passed**. No full selftest, bundle helper vectors, About frontend, Windows CI or full workspace suite rerun this turn.
- Metadata/content collision probe: as F2 above.
- PS4 head/public Recompiled integration results are recorded in the completion addendum below.

No app/game launch or listening. No live install, anchor, private fixture, application-source, index or commit modification. Rust tests use the developer's disposable-copy routes. Source hashes and measured summary are in `development-20260928-receipt65-evidence.json`.

## Cleanup

Reviewer-owned metadata probe and archive extraction are confined to `local/runs/coordinator-r65-review`; removed at closeout after preserving compact evidence and checking exact containment and reparse points. Shared caches, existing bundles, installed tools and unowned H:\TEMP folders remain untouched. No claim is made that unrelated developer scratch was independently audited.

## Completion addendum

The independently executed PS4 head integration test passed (1/1, 211.68 s), followed by the Recompiled public-edit integration test (1/1, 36.72 s), using the configured private-fixture root and disposable copies. Combined with the 11 Hockey Shop unit tests, this review executed 13 passing tests with no failures. These passing paths do not cover F2's warm-cache metadata-preserving source replacement. The developer must add that regression and correction before hair continuation. No broader receipt acceptance is implied.

