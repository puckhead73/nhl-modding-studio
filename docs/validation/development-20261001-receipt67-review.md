# Receipt 67 review and verification freeze — 1 October 2026

Disposition: r67 is a hash-verified **internal user-verification baseline**, not a fully qualified external beta. Freeze feature development at the user's direction. No continuation into Recompiled portrait/head previews or other queued features while the user tests. Incomplete agreed scope stays recorded; this freeze does not waive it.

## Independent work this turn

Read receipt 67 and relevant PS3 board planner/UI and Recompiled speech sources. Stream-checked every r67 ZIP manifest entry and corresponding existing extracted file: 619 entries match, 620 archive files including the manifest. Archive SHA-256 `6297cda973d2b9368d5809c086d71eb9f72c0ec14aa463493083f73fdfaa0b4a`; executable SHA-256 `60eee0adbd16df95692c4249f9926d7aff998855cf04958ba897fa12d37bbe65`. Exact results: `development-20261001-receipt67-evidence.json`. No full verifier/selftest or build drift replay this turn. Source has subsequent storage-profile/documentation changes; do not identify the dirty checkout with the frozen binary merely by HEAD.

No Rust compilation or 755-second real-archive transaction suite was rerun after the authorized cache cleanup. Those remain developer evidence. No app/game launch or audio playback. No independent full acceptance of the new paired writer is claimed from source inspection or UI tests.

## Findings and decisions

1. **Read-path error/ambiguity handling remains incomplete.** `gen7_speech_preview::locate_member` returns `Option`, collapsing open/table/sidecar failures into absence and using the first path match. `read_member_unwrapped` inherits that behavior. `bank_from_archives` chooses the first match; `archive_beside` silently prefers a direct archive when both roots exist. Catalogue comparison across archives does not resolve within-archive duplicate ambiguity or ensure preview uses the same selection. Source finding, not a reproduced public-path failure. Research-8's amendments are not fully implemented.
2. **Partial loose directories still suppress archive fallback.** `AudioDir::open` commits to Loose if the directory exists. An absent required file then errors rather than falling back; optional loose read errors are treated as absence. Record these as incomplete read-route robustness. They do not establish a data-loss defect and need not block stock-install user verification. They must be triaged before a claim that these boundary cases are complete.
3. **DXT1 opacity:** accepted as the narrowly disclosed board-writer policy for this internal pass. It drops alpha, not composites onto a chosen background; hidden RGB may become visible. Prefer opaque input images for the test and verify the disclosure for transparent input. Do not claim DXT1 generally cannot encode transparency or change generic texture behavior. No new codec change requested.
4. **PS3 board coverage:** NHL 11–13 has no UI gallery/entry point; admission census is not a write qualification for NHL 11/12/14/15. Record backend-only and tested target distinctions. Use Legacy copies for V-100 first. The 64 archived versus 67 loose package discrepancy is reported, not independently reconciled here; do not claim every loose package has a usable pair.
5. **Recompiled missing features remain missing.** Portrait/head previews, header logo, league imagery, kits, arena art, equipment previews and Hockey Shop installation are not completed by this receipt. The Hockey Shop refusal is the intended guard, not proof equipment editing is complete.
6. **Loose-loader card:** research handoff 15's original card has a pending correction from research-receipt-8 review. Do not run its shared-cache/two-guessed-location procedure as written. This experiment is not a prerequisite for ordinary r67 UI verification.

No new reproduced corruption blocker was established this turn. This is not proof that none exists. The proposed action is to gather user evidence on an immutable r67 baseline, not to expand development until every unfinished feature is built.

## Freeze and release distinction

Use the exact extracted r67 directory and its packaged resources. Keep baseline install/save copies and recovery space. Stop a write-path test if unrelated bytes change, save/reopen loses edits, recovery fails, or originals become unavailable; preserve the affected copy and diagnostics for a bounded fix. Never overwrite r67 with a correction build.

External release remains gated by unresolved agreed functionality, target verification and review, plus the missing MIT holder and artwork replacement/rights record. The archive's two packaging blockers are not an exhaustive product-release checklist. Internal artwork permission stands. E: currently has approximately 132 GiB free; receipt 67's 1.9 GiB statement predates cleanup.

Dispatch: `docs/plans/development-20261001-verification-freeze.md`. No staging, commit, publication, application-source edits or cleanup occurred this turn.

## Focused UI execution

ArenaVenueArtCard.test.tsx: 4 passed, 0 failed (19.16 s runner duration), independently run this turn. The first command targeted nonexistent BoardReplace.test.tsx and exited with no tests found; the corrected command ran the actual suite. This is UI regression evidence only, not paired-writer or runtime acceptance.

