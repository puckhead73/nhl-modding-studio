# Receipt70 / r69 review — 2026-10-01

Disposition: N21-001-R1 accepted for bounded offline correction; **r69 is the next internal user-verification candidate**. User acceptance of PS4 onboarding remains pending. No new blocking finding in this focused review. Development remains frozen; no additional implementation or candidate requested. No app/game launch, original save open, deployment, publication or cross-session message this turn.

## Independent checks

| Check | Result |
|---|---|
| Existing onboarding UI suite plus unmodified preserved coordinator reproduction | 15/15 passed: 10 current +5 preserved. Runner duration6.65s. |
| Domain unreadable-profile/title/library tests | 2/2 passed, no ignored selected cases. |
| API ps4_onboarding_paths suite | 4/4 passed;1 real-install case ignored explicitly. Opens used suite-owned copied fixtures. |
| r69 reviewed source identity | OpenProject.tsx, its tests, discovery.rs and ps4_onboarding_paths.rs match r69 build-source manifest. |
| r68→r69 source-manifest comparison | Exactly the six reported changed entries; both manifests contain974 entries. |
| Extracted r67, r68 and r69 bundles | Each:619 manifest entries matched,620 files including manifest. Whole ZIP/executable hashes match published receipt values. |

Commands executed:

```text
npm --prefix app test -- src/pages/OpenProject.test.tsx src/pages/N21CoordinatorReviewR69.test.tsx
cargo test --offline -p studio-domain --lib ps4_unreadable -- --nocapture
cargo test --offline -p studio-api --test ps4_onboarding_paths
```

Existing tdb-loc lifetime and studio-domain unused-function compiler warnings appeared; no failure. No full cargo/frontend suite, fmt/clippy, Windows CI, type/binding checks or archive selftests independently rerun. Their developer results remain separate evidence. ZIPs were whole-file hashed, not independently member-streamed.

## Candidate identities

Current:

- Program: `E:\nhlms-review-build-r69-2026-10-01\NHL Modding Studio rc-20261001-r69\nhl-studio.exe`
- Program SHA-256: `051681c2661537acc123392218801fd799199356bb21a89cea1a357bac8b4968`
- ZIP: `E:\nhlms-release-candidate-20261001-r69\NHL Modding Studio rc-20261001-r69.zip`
- ZIP SHA-256: `322b9b9a6872e1889101945f0661be67333cab8e1b661be59e1cb7eb352c5627`

Preserved:

- r67 exe `60eee0adbd16df95692c4249f9926d7aff998855cf04958ba897fa12d37bbe65`; ZIP `6297cda973d2b9368d5809c086d71eb9f72c0ec14aa463493083f73fdfaa0b4a`.
- r68 exe `bd47527691548ac974ab002dbab7ece45bd35f6b82962c6db003c2a7559726a3`; ZIP `67de2eb2b1888ad6304a9bf7c6fc2b21d71d3dd44e4fe176b4ad79cb9647c5fd`.

## Accepted scope and remaining limits

The reproduced stale-selection case now passes. With a stale remembered selection, alternatives require a new selection, and no alternatives require explicit database-only consent. Current tests assert actual backend arguments and stored settings. Backend stale selection and valid/first-time selections remain covered. Failed nested directory listings are disclosed as incomplete inventory in the measured tests.

Database-only selection while recognized rosters remain is not added; accepted as outside this bounded correction, not a new product-wide scope exclusion. Metadata-dependent game identity, title-folder-dependent save identity, bounded scans and separate update-folder candidates retain receipt69/70 limitations. No unlimited resilience or runtime support claim.

Opening writes the inherited `.nhlmodproj` sidecar. Detection can be performed read-only on named originals, but opening/extraction/editing tests require explicitly owned game/database and save copies with baseline/restore established. A remembered startup auto-open can also write the sidecar; inspect/default-control this before starting a test session rather than assuming launch is read-only. No shared settings/cache reset authorized.

## User reruns and next step

Use r69 for new findings; retain prior r67/r68 records as historical, not overwritten.

1. NHL21 A01/A02: separate game and save roots, actual title/patch, profiles and source paths; establish minimum owned copy layout before Open/extraction.
2. Remembered roster: valid selection, stale selection with zero/one/multiple alternatives, and startup behavior on owned fixtures only.
3. Save-root/profile/title-level selection; supported roster versus unsupported Profile0 distinction.
4. DB/selected-save opening and reopen, verifying the actual editing target.
5. Asset/File Browser/speech root attachment, and extraction only when needed on owned targets.
6. Continue NHL21-CHECKLIST.md using r69 after onboarding succeeds. No blanket repeat of unaffected work; no auto-upgrade of earlier results.

NHL21-E1 remains accepted for its actual identity/durability/restore scope. r67 startup pass remains historical; r69 startup is untested. r67 onboarding failure and r68 stale-selection regression remain recorded. N21-001 is offline-corrected and awaiting user confirmation, not fully closed.

## Artifact/process disposition

Reviewer-created temporary `app/src/pages/N21CoordinatorReviewR69.test.tsx` was an exact copy of the preserved r68 probe; removed after checking its hash still matched. Original probe remains at `r68-review/N21CoordinatorReview.test.tsx.txt`. No feature source changes or processes left running by this review. Shared test/build caches retained. Existing bundles and developer instances untouched.

Developer next instruction: remain frozen. No further implementation, rebuild or candidate is needed unless user verification identifies and explicitly dispatches another bounded correction.
