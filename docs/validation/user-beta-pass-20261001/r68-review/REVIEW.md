# Receipt69 / r68 coordinator review — 2026-10-01

Disposition: correction required before adopting r68 as the next user-verification baseline. Continue the same developer session for N21-001-R1 only. No implementation, dispatch message or app/game launch performed by reviewer.

## P1 — stale remembered roster silently replaced in manual onboarding

Source: `app/src/pages/OpenProject.tsx:98–100`, Open button at line607 and radio fallback around line748.

Reproduction: remember profile3/RosterCustom0/DATA; discovery now returns exactly one recognized roster, profile1/RosterCustom0/DATA. The UI warns that the remembered file is absent and asks for a new choice, but auto-checks the remaining roster and enables Open. `rosterArg` supplies that different path, so the backend's stale-choice validation never receives the missing remembered path. The alternative is also persisted as the new choice after successful open.

Expected: explicit reselection before substituting a different roster. A first-ever setup with one roster may retain its documented convenience selection. A stale remembered selection must not use that fallback. With zero remaining rosters, require explicit acknowledgment/DB-only choice rather than silently switching source type. Startup auto-open currently passes the stale path to the backend and refuses; manual onboarding must preserve the same protection.

Independent regression: copied the existing OpenProject.test.tsx to a reviewer-owned temporary sibling, changing only the stale-roster test's mocked inventory to `[roster("1")]`. The existing assertion `expect(openButton().disabled).toBe(true)` fails: actual false. Original onboarding suite 5/5 passed; probe suite 4/5 passed, 1 failed. Combined 9 passed, 1 failed, runner duration20.01s. Command: `npm --prefix app test -- src/pages/N21CoordinatorReview.test.tsx src/pages/OpenProject.test.tsx`. No real save opened or modified; this is component-level reproduction, not an observed wrong-save disk write.

Evidence: `N21CoordinatorReview.test.tsx.txt` retains the exact probe. Temporary `app/src/pages/N21CoordinatorReview.test.tsx` removed after results. The first attempted probe construction stopped on a newline-pattern mismatch before creating a file; the second constructed and executed the probe successfully. No feature source modified.

## Independent identity checks

- r68 executable: bd47527691548ac974ab002dbab7ece45bd35f6b82962c6db003c2a7559726a3
- r68 ZIP: 67de2eb2b1888ad6304a9bf7c6fc2b21d71d3dd44e4fe176b4ad79cb9647c5fd
- r67 executable: 60eee0adbd16df95692c4249f9926d7aff998855cf04958ba897fa12d37bbe65
- r67 ZIP: 6297cda973d2b9368d5809c086d71eb9f72c0ec14aa463493083f73fdfaa0b4a
- Both extracted bundles: all619 manifest entries match;620 total files. Whole ZIPs hashed; ZIP members/selftests not independently rerun.
- Reviewed OpenProject.tsx, its existing tests, catalog.ts, discovery.rs and registry.rs match their exact r68 dirty-source manifest hashes.

No independent full domain/API/frontend run, build, game observation or runtime acceptance. Developer totals remain developer evidence.

## Decisions and limitations

Metadata-required game matching is acceptable for the supported intact game-folder route; an extracted DB folder without package metadata is not equivalent. Separate saves and explicit multi-roster choice are appropriate. Save identity based on unrenamed title directories remains a disclosed limitation, not proof of payload identity across regions. Do not generalize detection into writable-format support.

The inherited `.nhlmodproj` sidecar write remains outside this bounded implementation change. It means even Open is not read-only: use an owned copied game/database location and copied saves before opening in verification. Detection against originals is separate. No permission to write that sidecar into originals or to reset shared app settings has been added.

Source inspection additionally shows nested `sorted_dirs`/`has_save_with_data` collapse directory-listing failures into empty collections. The developer's unreadable-DATA cases do not establish complete unreadable-profile/title-directory disclosure. Retain as a review limitation; during R1, cover an unreadable nested profile/title directory and report partial inventory explicitly if it is currently omitted. This concerns the existing N21-001 unreadable-location requirement, not new feature scope.

## Bounded developer follow-up

Fix stale remembered-roster handling in manual onboarding, with zero/one/multiple recognized alternatives covered. Preserve stale-path identity until the user selects a replacement or explicitly chooses DB-only; do not auto-check/submit/persist another profile's save. Test actual open_workspace arguments, not only warning text/button state. Keep initial one-roster setup and valid remembered selection working; retain backend stale-path refusal and startup auto-open behavior.

Reconcile incomplete unreadable nested-directory disclosure with focused owned-fixture tests and accurate partial-inventory messages. Do not drop unreadable paths silently or promote no-saves from an incomplete scan.

Run focused UI/API/discovery and type/binding checks as affected. Preserve r67 and r68; create r69 if unused, otherwise the next unused candidate. Hash and manifest-verify the new package, name precise changed inputs, and deliver a developer receipt with reruns. No unrelated refactor, sidecar relocation, game/save mutation, commit, publication or autonomous launch. Freeze again after delivery. This document is a review handoff, not a message sent to another session.
