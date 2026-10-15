# October 4 background review build

Work resumed with the user watching the second monitor. No application windows were opened, moved or focused, and no emulator input was sent. The recovery heartbeat remains paused from the user's earlier pause; it has not been re-enabled to take over the desktop later.

## Save destination correction

Xbox 360 and Recompiled projects no longer automatically link a default-discovered roster save when the optional save-data location is blank. Selecting a location still links an available writable roster; clearing that location removes the link and remains cleared after reopening the project. Explicit roster selection and PS3 selected-HDD discovery retain their behavior. Onboarding now explains the blank-field behavior.

Validation: the destination unit test passes. All nine project integration tests pass (73.28 seconds), including a real copied NHL14 roster link/clear/reopen case that checks the source save remains unchanged, baseline creation/opening, project isolation, and reviewed deployment/restore. The initial empty-asset test fixture was correctly refused; the corrected fixture contains an asset and passes.

## Archive qualification

Latest archive: `target/internal-candidates/nhlms-release-candidate-20261004-qa3/NHL Modding Studio QA 20261004.zip`

- Size: 608,722,139 bytes.
- SHA-256: `5e6f152b3df4070c6b96ea72a757f9f79a702361d8c53d3ed2dfa3b911c64c15`.
- Completed native release build: `target/overnight-20261003/review-release-build-3.log`.
- Includes the save-destination correction, bundled baseline resources, historical database snapshot, fixed WebView2 runtime, audio helper, and the qualified PS4 package extractor with pinned sources/notices.
- Archive integrity check passes after extraction into `review-unpacked-3`. Extracted-resource integration passes: bundled definitions resolve, baseline files verify, and a fresh project opens from bundled resources. The known NHL20 baseline alias remains unavailable, not substituted.
- All 63 frontend test files / 347 tests pass with two workers. Nine localization integration tests pass: rename, retext, growth/rebuild, copied loose/archive targets, save/reopen, and restore.

The build record is an after-build input snapshot, not a before/after reproducibility assertion. Earlier qa1/qa2 outputs remain preserved. This archive is an internal review artifact and has not been published. The assembler retains its existing external-release blockers: the Studio copyright-holder record and rights records for embedded artwork. No desktop launch of this latest executable was performed during the monitor restriction.

## Remaining feedback and acceptance limits

[GitHub issue 1](https://github.com/puckhead73/nhl-modding-studio/issues/1) was reread on October 4: NHL08 PS3 extraction/assets and Santala career dates remain the reported symptoms. The title reports contain actual source-route, texture and date-regression evidence; this does not close runtime acceptance. The Discord source includes the separate arena reassignment/full-ice complaint, not merely onboarding.

NHL07's schema explicitly stores the home-arena selector in `StEO`: `vfEq` joins the exhibition team, `buDa` selects the arena index, and `RREB` selects stock/custom. A complete reassignment must validate and change the selector pair together and account for multiple selectors resolving to one venue. Current reassignment remains explicitly refused; editing venue names is not evidence of successful reassignment or changed in-game arena art. No selector implementation was changed in this review.

Localization still holds one pending operation; save between separate string edits. Passing transaction tests does not establish a multi-edit queue. PS3 mesh decoding, early Xbox smaller mip levels, measured arena/kit/center-ice mappings, PS4 base/patch collision precedence and missing compatible saves remain in the per-title reports. NHL22–24 remain excluded as requested. No edited gameplay has been verified, and no launch result is reported as edited-game acceptance.
