# 70 — N21-001-R1: stale remembered roster and unreadable folders (candidate r69)

**Summary.** Two bounded corrections from the [r68 coordinator review](../../validation/user-beta-pass-20261001/r68-review/REVIEW.md).
1. **A remembered PS4 roster save that has disappeared is no longer replaced silently.** r68 auto-checked the only remaining
   roster, enabled Open, sent that other profile's save and remembered it. Now nothing is pre-checked, sent or remembered
   until you pick a replacement. When no roster is left, Open stays disabled until you explicitly tick "Open the database
   only". A first-time single roster keeps its convenience selection; a valid remembered roster stays selected; startup
   auto-open still sends the stale path, which the backend refuses.
2. **Unreadable folders are reported, not skipped.** An unreadable profile or title folder in the save-data location, or an
   unreadable folder in a game library, used to read as empty. It is now named in an "incomplete" problem. When nothing was
   found among the readable folders, Studio says the scan was incomplete instead of claiming there are no saves, or no game.

Internal candidate **r69**. r67 and r68 are byte-for-byte unchanged. Nothing was committed, pushed, published, launched or
installed. No original game or save was opened or written: every open in a test used owned copies, since an open writes
`.nhlmodproj` (sidecar location unchanged). Defect lineage: [BUG-N21-001](../../validation/user-beta-pass-20261001/BUG-N21-001-onboarding-paths.md) → [receipt 69](69-n21-001-ps4-onboarding-r68.md) / r68 → review → this receipt. No user check is marked passed.

## 1. Changes (vs r68)

| File | Change |
|---|---|
| `app/src/pages/OpenProject.tsx` | `rosterArg` falls back to the single roster only when nothing was remembered (`!rosterStale`); the radio is pre-checked under the same condition. `rosterChoiceMissing` also blocks a stale choice while any roster remains, or while none remains and `dbOnly` is unset. A new `dbOnly` state and checkbox, "Open the database only (no roster save is edited)", is shown only for a stale choice with no roster left; it toggles, is reset by any rescan or roster pick, and after a database-only open the remembered `saveFile` is cleared. The stale remembered path stays stored until an explicit replacement or database-only open. The stale notice and legend now name the action needed. |
| `crates/studio-domain/src/discovery.rs` | An `Unreadable` collector (path → error) is threaded through `sorted_dirs`, `has_save_with_data`, `ps4_title_dirs`, `ps4_save_dirs` and `ps4_library_children`, so a failed listing is recorded instead of becoming an empty result. New problem kinds: `save_inventory_incomplete` names each unreadable folder; it replaces `saves_not_found` / `saves_other_title` when no save was found, and is added beside a partial list otherwise. `game_location_incomplete` does the same for library folders and replaces `game_not_found`. An unreadable folder named for another title id (`CUSA#####`, not applicable) is not reported. The game location's own folders aren't counted as save-location gaps. |
| `crates/studio-api/src/dto.rs` | Documentation of the new problem kinds (regenerated `DiscoveryProblemDto.ts`). |
| Tests | `OpenProject.test.tsx` (stale cases: one, several and no alternatives; a valid remembered roster; the first-time single roster; startup auto-open), `discovery.rs` (2 unreadable-folder cases), `ps4_onboarding_paths.rs` (database-only open + incomplete inventory through the Registry). |

Unchanged:
- the backend's stale-choice refusal and its open rules;
- the sidecar location;
- the PS4 codec gate;
- PS3 / Xbox / Recomp / ISO behaviour;
- everything outside onboarding.

Not added: a database-only open while recognised rosters remain. The backend opens a single unchosen roster by design, so a "database only despite rosters" choice would need a new backend flag. It is outside this correction, and the checkbox is offered only when no roster is left.

## 2. Checks

| Suite | Result |
|---|---|
| `OpenProject.test.tsx` | 10 / 10 (5 previous + 5 net new: the old stale case replaced by zero / one / several, plus valid-remembered, first-time single and startup auto-open). Each asserts `open_workspace` arguments and the stored `nhl21:ps4` entry. |
| Coordinator probe `r68-review/N21CoordinatorReview.test.tsx.txt`, run unmodified as a temporary sibling (removed after) | 5 / 5 (failed 4 / 5 on r68) |
| frontend vitest (all) | 288 / 288 |
| `tsc --noEmit` | clean |
| `studio-domain --lib` (incl. 2 new unreadable-folder cases) | 236 / 236 (2 ignored, pre-existing) |
| `studio-api --test ps4_onboarding_paths` | 4 / 4; read-only real-install detection `--ignored` 1 / 1 |
| `studio-api --lib` | 42 / 42 |
| bindings `--write` · `--check` | 1 rewritten (`DiscoveryProblemDto.ts`), 303 unchanged · in sync (174 roots) |
| r69 `verify.py check` · `selftest` · extracted-bundle tests from `E:\tmp` | ok · 21 / 21 refused · 3 / 3 (83 XMA vectors, no failures) |

**How the unreadable cases are made:** each case owns a directory held open with no sharing
(`FILE_FLAG_BACKUP_SEMANTICS`, share mode 0), so its listing fails with a sharing violation until the handle drops. Each case
first asserts that the listing really fails. No ACL or permission is changed.

**What they cover:**
- an unreadable second profile next to a readable one (partial list plus a named gap);
- the only title folder unreadable, selected from the savedata root and from the profile (`save_inventory_incomplete` only,
  never `saves_not_found`);
- another title's unreadable folder (ignored);
- an unlistable save folder whose `DATA` is readable, which still lists: save folders aren't listed, their `DATA` is opened
  by path, and an unreadable `DATA` stays a per-slot problem;
- an unreadable library folder beside a readable match (it resolves, with the gap reported);
- a library whose only folder is unreadable (`game_location_incomplete`, never `game_not_found`).

**Not run:**
- full `cargo test --all`;
- fmt / clippy (C0.2 baseline);
- Windows CI;
- any launch, game run, or opening of an original save.

## 3. Candidate r69

`r69` was unused (no folder, archive or evidence directory). It is in its own output folder so r68's folder, including its
`stage`, is not touched.

| Item | Value |
|---|---|
| Archive | `E:\nhlms-release-candidate-20261001-r69\NHL Modding Studio rc-20261001-r69.zip`, SHA-256 `322b9b9a6872e1889101945f0661be67333cab8e1b661be59e1cb7eb352c5627`, 620 files, 46 166 867 bytes |
| Executable | `E:\nhlms-review-build-r69-2026-10-01\NHL Modding Studio rc-20261001-r69\nhl-studio.exe`, SHA-256 `051681c2661537acc123392218801fd799199356bb21a89cea1a357bac8b4968` (51 006 464 bytes) |
| Inputs | HEAD `8d60a6d` + 974 dirty inputs, manifest `bb8f2914…afec6`, identical after the build; built 14:25:58 → 14:28:44 MDT |
| Changed inputs vs r68 | `app/src/pages/OpenProject.tsx`, `crates/studio-domain/src/discovery.rs`, `crates/studio-api/src/dto.rs` (documentation only), `app/src/bindings/DiscoveryProblemDto.ts` (regenerated documentation), `app/src/pages/OpenProject.test.tsx`, `crates/studio-api/tests/ps4_onboarding_paths.rs` — nothing else |
| defs / frontend / helpers | `20c3b183…cf8b` (unchanged) / `84abba13…2d31` / `1caa5f4d…f5c4e` (unchanged); helper `dc3533ed…8c21`; FFmpeg source `7138d28c…78a3` |
| Record | `internal-review`, `release_ready: false`; blockers unchanged (MIT holder pending; 53 artwork files, rights unrecorded) |
| Evidence | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r69/` |

**Preserved:**
- **r67:** the extracted tree is identical before and after (620 files, `7271eafe…5fc9`); the zip is `6297cda9…0b4a`.
- **r68:** the extracted tree is identical before and after (620 files, `60b3d2f9…7ff9`); the zip is `67de2eb2…c5fd`; its
  folder (zip and `stage`) keeps its 03:03 / 03:04 timestamps.

## 4. Focused rerun list (on r69)

1. **Stale remembered roster** (owned copies only). Remember a roster, then remove or rename that save in a copied save-data folder.
   - With one other roster: nothing is pre-checked and Open is disabled until you pick it; then that save opens and is remembered.
   - With several others: same, with an explicit pick.
   - With none: Open is disabled until "Open the database only" is ticked; the database opens and no roster is remembered.
2. **Normal paths unchanged:** a first-time single roster opens directly; a valid remembered roster stays selected; startup auto-open with a stale remembered roster refuses and leaves the choice remembered.
3. **Incomplete inventory** (optional; needs a folder the user can't list): the save list or game list names the unreadable folder and never says "no saves" or "no game" for that scan.
4. **The receipt 69 §6 PS4 reruns,** on r69 instead of r68: onboarding, source and save selection, remembered reopening, database and save opening (owned copies), asset-root attachment.

**Invalidated by an actual changed dependency:** any r68 result for remembered-roster handling (the selection logic changed) and any r68 result that relied on "no saves" / "not found" wording for a partly unreadable location (discovery changed). Everything else from r68 holds; nothing outside PS4 onboarding is affected.

## 5. Cleanup

- **Removed:**
  - the temporary coordinator-probe copy (`N21CoordinatorReviewR1Probe.test.tsx`), right after its run;
  - the selftest scratch;
  - the r67 / r68 preservation manifests in session scratch;
  - every test's owned temp tree (their guards removed them; none remain).
- **Kept:**
  - r67, r68 (with its `stage`) and the corpus package;
  - r69's zip, its extracted bundle and its assembler `stage`;
  - the shared `target\` cache, toolchain, fixtures and `E:\tmp`.
- **Processes:** the separately running development instance noted in receipt 69 was not touched; it was no longer running
  at the end of this package, and no process of this package is running.
- The claim `dev-n21-001-ps4-onboarding` is `ready` (R1 recorded).

**Next:** none. Freeze again; r69 waits for the focused reruns.
