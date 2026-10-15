# 69 — N21-001 PS4 onboarding paths (candidate r68)

**Summary.** PS4 onboarding (shared by NHL 15–21) no longer assumes that the game and the saves sit inside the shadPS4 folder.
- **Game folder:** you pick it directly, wherever it is and whatever it is called. Studio identifies the game from its own
  package metadata (`sce_sys/param.sfo`), never from the folder name.
- **Save-data folder:** you pick it separately. Studio suggests this Windows user's `%APPDATA%\shadPS4\savedata` and accepts
  the savedata root, one profile folder or the game's title folder. Every matching save of every profile and regional title id
  is listed with its profile and title id.
- **Several roster saves:** you choose which one to open, and Studio remembers the choice.
- **Older setups:** a remembered r67 setup that points at the shadPS4 folder still opens.
- **Problems:** a missing game, a wrong game, an unreadable location, missing saves and another title's saves each get their
  own message.

This is the bounded correction the user dispatched on 2026-10-01, the only exception to the
[r67 verification freeze](68-r67-verification-freeze.md). It ships as **internal candidate r68**; r67 is unchanged byte for
byte. Nothing was committed, pushed, published, launched or installed; no original game or save was written. Defect report:
[BUG-N21-001](../../validation/user-beta-pass-20261001/BUG-N21-001-onboarding-paths.md) — its initial r67 failure stands as
evidence and is not edited. No user check is marked passed here.

## 1. Changes

| Area | Change |
|---|---|
| **Game location** (`studio-domain` `discovery.rs`) | The PS4 root is the game folder itself, or a library holding game folders (`<root>/Games/<game>` or `<root>/<game>`, any folder name; one level only). Each candidate is identified from its own `param.sfo`: the TITLE must be an NHL title of the requested year, and a CONTENT_ID, when present, must name the same year. TITLE_ID, CONTENT_ID (region prefix) and APP_VER are reported for every candidate. Exactly one match resolves; none, a wrong title, an unidentifiable folder or several matches resolve nothing and say why (`game_not_found` / `game_wrong_title` / `game_unidentified` / `game_ambiguous`). The database, metadata, dictionary, extraction and art all resolve from the matched game folder only. |
| **Save location** | `save_path` is now used for PS4. It is read at exactly one level: the title folder (`<TITLE_ID>/<save>/DATA`), a profile (`<profile>/<TITLE_ID>/…`) or the savedata root (`<root>/<profile>/<TITLE_ID>/…`). Saves are kept for the title's known regional ids plus the matched install's own TITLE_ID, across every profile; each slot carries `profile`, `title_id` and, when it can't be opened, `problem` (unreadable, or a format the codec doesn't recognise). Other titles' saves are never listed; selecting only those gives `saves_other_title`. With no save location, the older layout under the root (`user/savedata`, `savedata`, root-as-profile) is searched, which keeps a remembered emulator-folder entry working. |
| **Suggestion** | `ps4_default_save_root()` = `%APPDATA%\shadPS4\savedata` from the environment (like the existing Xenia default from `USERPROFILE`); command `ps4_save_root_suggestion` reports it and whether it exists. A suggestion only: it is never scanned unless it is the chosen save location. |
| **Opening** (`studio-api` `registry.rs`) | `open_workspace` gains `roster_save`. PS4: a chosen roster must be one of the recognised roster saves in the current inventory; with no choice, a single recognised roster opens and several are refused, naming each one by save folder, profile and title id. The choice is settled before the profile sidecar is written, so a refused open writes nothing. A PS4 open whose minimum isn't met reports the discovery problems. Other platforms ignore `roster_save`. |
| **Profile sidecar** | Records `game_dir` and `save_root` (serde defaults, so older sidecars still load). |
| **Onboarding page** (`OpenProject.tsx`) | PS4: "Game folder" with metadata-identity copy (no shadPS4.exe wording), and an optional "Save-data folder" prefilled with the suggestion for a fresh setup. A remembered setup is left as it was, and a "Use it" link offers the suggested folder. Game candidates are listed with title, title id, region and version, with "Use this folder" when several match. Problems are listed separately. A roster-save radio list appears whenever recognised rosters exist, and Open stays disabled while several remain unchosen. A remembered choice no longer listed is flagged. Extraction targets the matched game folder. The paths and roster choice are remembered (`PathEntry.saveFile`) and sent on auto-open. |
| **Settings page** | PS4 rows show "Game folder" and an optional "Save-data folder"; changing either clears the remembered roster choice. |
| **Bindings** | `SaveSlotDto` +3 fields; `InstallReportDto` +4; new `GameCandidateDto`, `DiscoveryProblemDto`, `SaveRootSuggestionDto` (export roots 173 → 174). Regenerated with `export-bindings --write`; `--check` in sync. |
| **Hardening** | `sfo_string_value` no longer panics on an out-of-range key offset (`get` and checked adds); it now reads untrusted metadata from any picked folder. |

Not changed: PS3, Xbox, Recomp and ISO discovery and opening; the PS4 codec gate (`tdb_savedata::is_ps4_save` decides writability; a listed `DATA` is never support); the PS4 minimum (database + metadata; a roster save remains optional); `install_dir` semantics (the database folder), so asset roots, speech and archive features still derive from the game folder exactly as before.

## 2. Decisions to review

1. **Game identity is metadata-only.** A PS4 game folder without a readable `sce_sys/param.sfo` is refused (`game_unidentified`), even under a known CUSA name. r67 accepted a known-id folder name alone. Every installed title on the user's machine carries one (§4).
2. **Same-number titles no longer match.** r67's year-only check let FIFA 16 (`CUSA02138`, in the user's library) pass as NHL 16; it is now refused.
3. **Several recognised roster saves need a choice.** r67 silently opened the first by path order. This also applies inside one profile (`RosterCustom0` / `RosterCustom1`).
4. **Save identity is the title-id folder name.** shadPS4 names each title's save folder by TITLE_ID. A renamed or copied title folder can't be confirmed and is reported as another title's or not found. Save `param.sfo` contents were not used (not verified here).
5. **For a product decision, not changed:** opening still writes the `.nhlmodproj` profile sidecar into the game's database folder. This predates N21-001 and is now visible because the game folder is chosen directly. Preserved as an existing requirement.

## 3. Checks

| Suite | Result |
|---|---|
| `studio-domain` `discovery::` + `profile::` (incl. 7 new N21-001 cases and the existing PS3 / Xbox / Recomp / PS4 cases) | 21 / 21 |
| `studio-domain --lib` (all) | 234 / 234 (2 ignored, pre-existing) |
| `studio-api --test ps4_onboarding_paths` (owned copies of the genuine NHL 16 PS4 anchor; PS3 control) | 3 / 3, plus the read-only real-install case `--ignored` 1 / 1 |
| `studio-api --lib` (export root count 174) | 42 / 42 |
| bindings `--check` | in sync (174 roots, 304 files) |
| frontend vitest (all; `OpenProject.test.tsx` new, 5 cases) | 283 / 283 |
| `tsc --noEmit` | clean |
| r68 `verify.py check` · `selftest` · extracted-bundle tests from `E:\tmp` | ok · 21 / 21 refused · 3 / 3 (§5) |

**What the cases cover:**
- **Locations and identity:**
  - an arbitrary game folder with unrelated roaming-shaped saves;
  - a game folder not named after its title id, identified by metadata;
  - the wrong title, an unidentified folder, a content id naming another year, and FIFA 16;
  - the older `Games/CUSA` + `user/savedata` layout and an arbitrarily named library;
  - several matching installs (none chosen).
- **Save selection:**
  - savedata-root, profile and title-level selection, plus a custom / portable root;
  - two profiles × two NHL 20 regional ids, complete and ordered;
  - another title's title folder refused.
- **Missing and unreadable:**
  - a missing game location, a missing save location, empty saves and a hollow game folder;
  - a `DATA` that is a directory, and a `DATA` held open with no sharing (`couldn't be read`);
  - a recognised roster versus an unrecognised payload.
- **Opening, end to end through the Registry:**
  - both paths reach detection and opening;
  - refusal with two rosters and no choice, with no sidecar written;
  - refusal of a choice outside the inventory;
  - the chosen profile-2 roster is the edited file, the save's bytes are unchanged on open, and the dictionary attaches;
  - the sidecar records `game_dir` / `save_root`;
  - reopen; a remembered roster that disappeared is refused; a single remaining roster opens;
  - the r67 emulator-folder configuration opens;
  - NHL 16 opened as NHL 17 is refused;
  - PS3 ignores `roster_save`.
- **UI:**
  - the PS4 fields and the prefilled suggestion;
  - `detect_install` / `open_workspace` arguments carry both paths and the chosen roster;
  - the remembered entry is written and an old entry kept with the offer;
  - several candidates with "Use this folder", and problem kinds kept distinct;
  - a stale remembered roster;
  - the Xbox route unchanged.

**Real installs, read only (detection only; no open, since an open writes the sidecar):**
- **NHL 21 game folder:** `H:\Emulators\ShadPS4\Games\CUSA18182` was picked directly, with this user's roaming savedata.
  - NHL™ 21, CUSA18182, EP region.
  - Database at `databigs\Extracted\…\t3db\nhl_game.db`.
  - Profile 1: `RosterCustom0` recognised; `Profile0` listed as unreadable by Studio.
- **NHL 15–21:** each game folder picked directly identifies from its own `param.sfo`. NHL 20 is the genuine `CUSA15439` install, not the committed NHL 19-base anchor.
- **FIFA 16:** refused as NHL 16.

None of this is runtime qualification of any title; the shared contract is tested on one genuine title (NHL 16) plus detection on the seven real installs.

**Not run:**
- full `cargo test --all`;
- fmt / clippy (C0.2 baseline);
- Windows CI;
- any app launch, game run or real save open.

## 4. Candidate r68

`r68` was unused: no folder, archive, evidence directory or receipt carried it.

| Item | Value |
|---|---|
| Archive | `E:\nhlms-release-candidate-20261001\NHL Modding Studio rc-20261001-r68.zip`, SHA-256 `67de2eb2b1888ad6304a9bf7c6fc2b21d71d3dd44e4fe176b4ad79cb9647c5fd`, 620 files, 46 166 893 bytes |
| Executable | `E:\nhlms-review-build-r68-2026-10-01\NHL Modding Studio rc-20261001-r68\nhl-studio.exe`, SHA-256 `bd47527691548ac974ab002dbab7ece45bd35f6b82962c6db003c2a7559726a3` (50 996 224 bytes) |
| Inputs | HEAD `8d60a6d` + 974 dirty inputs, manifest `d69b430a…c208`, identical after the build; built 02:58:31 → 03:03:12 MDT |
| Changed inputs vs r67 | the N21-001 set in §1 (16 changed existing files, including the untracked `bindings.rs` and two generated bindings; 5 new files: three generated bindings, `OpenProject.test.tsx`, `ps4_onboarding_paths.rs`) plus the unrelated pre-existing `Cargo.toml` dev/test profile change, whose release profile is unchanged (receipt 68 §2) |
| defs / frontend / helpers | `20c3b183…cf8b` (unchanged) / `f0d16272…8cf9` / `1caa5f4d…f5c4e` (unchanged); helper `dc3533ed…8c21`; FFmpeg source `7138d28c…78a3` |
| Record | `internal-review`, `release_ready: false`; blockers unchanged (MIT holder pending; 53 artwork files, rights unrecorded) |
| Evidence | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r68/` |

## 5. Bundle verification

- `verify.py check`: ok. Every manifest entry matches, nothing extra, layout, pinned helper and FFmpeg source, licences, not-ready status.
- `selftest`: refuses all 21 broken variants (its scratch removed itself).
- Extracted-bundle tests, run from `E:\tmp` with `NHL_STUDIO_XMA2_HELPER` unset:
  - `release_archive_layout`: 1 / 1; defs resolve beside the executable;
  - `about_notices`: 1 / 1; every file the notices name exists;
  - `ea_xma_real_vectors`: 1 / 1; 83 vectors through the bundled helper, no failures.
- **r67 preserved byte for byte:** the extracted r67 tree was hashed before and after this package (620 files, tree `7271eafe…5fc9`, identical). The r67 zip still hashes `6297cda9…0b4a` with its 2026-09-28 21:14:26 timestamp. r68 went to new folders only.

## 6. Rerun list and invalidated evidence

**Rerun on r68 (focused):**
1. **PS4 onboarding:** NHL 21 A01 / A02. Pick the real game folder directly, with the save-data folder as suggested.
2. **Source and save selection:** savedata root, profile folder and title folder; your actual save inventory, with a roster choice when there are several.
3. **Remembered reopening:** startup default and returning to the title, both with the new pair and with an old r67 emulator-folder entry.
4. **Database and save opening:** the chosen roster opens as the edited file; the database-only open without saves.
5. **Asset-root attachment:** Assets / File Browser / speech cards after a directly picked game folder, plus extraction from onboarding, which now targets the game folder.

**Invalidated by an actual changed dependency:**
- r67 PS4 onboarding, detection, open and remembered-path results: the PS4 discovery, open and onboarding code changed.
- Any r67 evidence that relied on a silent first-roster choice.
- PS4 onboarding extraction: its root is now the matched game folder.

**Not invalidated:**
- Accepted NHL21-E1 identity: path selection only; it is not reopened.
- PS4 database / save editing after opening: session, codec and save paths are unchanged.
- PS3, Xbox, Recomp and ISO onboarding: their code paths are unchanged and covered by the regression cases. One quick open on a non-PS4 title remains a sensible smoke check, because the shared page was edited.
- Every non-onboarding r67 card.

## 7. Remaining limitations

- **Untested real-world layouts:**
  - Save identity follows the shadPS4 title-id folder layout (§2.4).
  - Patch / update folders laid out as separate game folders appear as separate candidates (a choice is required); their precedence is not modelled.
- **Discovery has no recursion beyond its fixed bounds:** one library level for games; two levels below the save selection.
- **Carried forward unchanged from receipt 68:** every limitation listed there (speech read routes, PS3 board coverage, DXT1 opacity, missing Recompiled features, withheld gates, display caches, external-release gates).

## 8. Cleanup

- **Removed:**
  - every test's owned scratch (`nhlms-n21-*`, `nhlms-*` discovery trees; their guards removed them, none remain);
  - the selftest scratch;
  - my session scratch files (splice script, block draft, `param.sfo` dump script, r67 tree manifests).
- **Kept:**
  - r67 (zip and extracted bundle, untouched) and the corpus package;
  - the r68 zip, its extracted bundle and the assembler's `stage` folder in `E:\nhlms-release-candidate-20261001`, as for earlier candidates;
  - the shared `target\` cache, toolchain, fixtures and `E:\tmp`.
- **Not mine, left running:** a development run of Studio (`cargo run --no-default-features` → `target\debug\nhl-studio.exe`, started 02:52). No process of this package is running.
- E: has 127 GiB free.
- The claim `dev-n21-001-ps4-onboarding` is `ready`.

**Next:** none. r68 waits for the user's focused reruns; development returns to the freeze.
