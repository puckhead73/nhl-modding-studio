# 71 — ONB-002: project folders, bundled baselines, reviewed deployment, extraction service (candidate r70)

**Summary.** Studio no longer edits the game installation or its saves in place. A project starts from
**Start fresh** (the bundled, verified vanilla files of the chosen title and console) or **Use my own files**
(your database and save files, copied — never written), always into a **project folder** you choose. The
path screen then asks for the three destinations in the same order on every console: project folder, game
installation folder, save-data location. Every edit — database, roster save, dictionary, settings-CSV and
archive-member edits, texture / nameplate / kit-clone outputs — is saved **into the project**; the game and
the saves are written only by an explicit, reviewed **Deploy**, which backs up every replaced file under the
project and can restore the originals. Container extraction, the one deliberate write into the
installation, became a repeatable service with a read-only preflight, a cancellable job, markers, and no
overwriting of anything that already exists; it is offered after the path screen only when something needs
extracting, and again under Settings → Tools after a Skip. A gap in the bundled set (NHL 20 PS4 is the
NHL 19-base alias; NHL 15 PS4's roster save has no codec; some dictionaries are not committed) is shown
as that gap, never filled from another title.

Internal candidate **r70**. r67, r68 and r69 are byte-for-byte unchanged (zips and extracted trees, §5).
Nothing was committed, pushed, published, launched or installed; no original game or save was opened for
writing; every write in a test went to a temp tree the test owns, and the committed anchors were hashed
before and after. Dispatch: [ONB-002](../../plans/onboarding-20261001-project-workspace.md); design note:
[onb-002-persistence-design.md](../../plans/onb-002-persistence-design.md). The user-facing **Open existing
project** screen is [ONB-003](../../plans/onboarding-20261001-resume-projects.md); the durable project
state it needs is in place and tested at the service level here. No user check is marked passed.

## 1. What changed

### 1.1 Bundled baseline inventory (ONB-002 §2, §4)

| Item | Where |
|---|---|
| Shipping-target table (27 rows, no NHL 07 PS3, no Xbox One) | `crates/studio-domain/src/targets.rs` |
| Inventory types, verified store, save-codec probe, atomic writes | `crates/studio-domain/src/baselines.rs` |
| Generator with the **approval table** (which anchor is admitted for which target, provenance, witness); hashes cross-checked against `docs/audits/data/anchors-manifest.csv`; saves probed with `tdb-savedata` | `crates/studio-api/src/bin/baseline_inventory.rs` (`--write` / `--check` / `--bundle <dir>`) |
| The generated inventory (schema `nhlms-baselines/1`, HEAD `8d60a6d`) | `baselines/INVENTORY.json` |
| Packaged beside the executable (`baselines/INVENTORY.json` + `baselines/files/<platform>/<game>/…`), verified by `verify.py`, resolved by `commands::resolve_baselines` (development run: repository `anchors/`) | `tools/release-archive/assemble.py`, `verify.py`, `tools/review-build/manifest.py` |

**Per-target inventory (presence, identity, provenance, codec are separate claims):**

| Target(s) | Status | Components | Gaps (explicit, not substituted) |
|---|---|---|---|
| Xbox 360 NHL 07–15, Legacy (10) | available | main db, meta, dictionary, dictionary meta, roster save — `verified-vanilla` (07–15 anchor-only titles; Legacy re-hashed against the install, REG-1) | — |
| PS3 NHL 11–15, Legacy (6) | available | same roles, `verified-vanilla` (REG-1 disc hashes) | — |
| PS3 NHL 08, 09 | partial | main db, meta, roster save | dictionary: the committed PS3 file is the Xbox 360 dictionary (REG-1); the genuine disc file is a private fixture, not bundled |
| PS3 NHL 10 | partial | main db, dictionary, roster save; meta = `cross-platform-schema` (the committed `ps3/nhl10/nhlng-meta.xml` is the Xbox 360 file `6c335a62…`; the two `nhlng.db` are byte-identical `e87b8c80…`) | dictionary meta not committed (the dictionary attaches without it) |
| PS4 NHL 16, 17, 18 | available | main db, meta, dictionary, dictionary meta, roster save (`DATA` + `INDEX`, `NAME`, `sce_sys/*`) — `verified-vanilla` (REG-1) | — |
| PS4 NHL 15 | partial | main db, meta, dictionary, dictionary meta | roster save: `84f8102f - ROSTER/DATA` is not a container `tdb-savedata` recognises (`is_ps4_save` false) — a codec gap, the database opens alone |
| PS4 NHL 19 | partial | main db, meta, dictionary meta, roster save (NHL 19 base, CUSA11126 01.60) | dictionary: not committed (private fixture) |
| PS4 NHL 20 | **unavailable** | none | the committed `anchors/ps4/nhl20` set is NHL 19 base bytes (REG-1); genuine NHL 20 (`1e200478…`, `2fac0e73…`, `0fc95ec8…`) exists only as private fixtures and is not bundled; no roster save. **Start fresh is refused with this reason.** |
| PS4 NHL 21 | partial | main db, meta, dictionary meta | dictionary: not committed (coordinator decision); roster save: none of BeAGM0 / BeAPro0 / Profile0 / Season0 / ThreeOnThree0 is a roster container — the database opens alone |
| Legacy Recompiled | available | the Xbox 360 Legacy files, `derived` — witness handoff R128 (the registered install's `cacheboot.big` member `db/nhlng.db` = anchor `105d2186…`; raw inner saves of the same layout/size) | — |

26 of 27 targets create and open a fresh project (`every_shipping_target_with_an_available_baseline_creates_and_opens`).

### 1.2 Project folder and persistence (§2)

`crates/studio-domain/src/project_store.rs`: `nhl-studio-project.json` (schema `nhlms-project/1`,
temp → rename), `baseline/` (immutable copies + `BASELINE.json`), `working/` (a `tdb-txn` copied-target
root holding `db/`, `loc/`, `saves/<container>/`, `overlay/`), `deployments/<seq>/`, `exports/`. Folder
rules: canonicalised comparison (junctions / symlinks resolved, case folded), refused inside or containing
the game root, install dir, PS4 game folder, save location or any import source's folder; a reparse point
on the path is refused; a non-empty non-project folder is refused; a failed creation removes only what it
created. A legacy `.nhlmodproj` in the install is recorded read-only (`legacy_sidecar`), never written,
migrated or deleted — **opening no longer writes the install-side sidecar** (`registry.rs`,
`ps4_onboarding_paths.rs` now asserts its absence).

`crates/studio-api/src/registry/project.rs`:
- `project_create_fresh` / `project_import_review` / `project_create_import`: destinations resolved with the
  existing discovery (PS4 roster choice through the r69 `ps4_chosen_roster` rules; gen7 first writable
  roster slot or none), components copied with hash verification, session built on the **working copies**
  (gen7 database + aligned roster mirror; PS4 roster save or the template database alone), dictionary
  attached from the project.
- `project_open` (service level; ONB-003's picker): reopens the working copies as the new baseline;
  destinations are re-discovered read-only (`ProjectOpenDto.discovery`).
- `project_save`: materialises the archives pending member edits need (lazily, free-space checked, identity
  recorded), then `copied_target_review` + `copied_target_apply` on `working/` — the existing journaled
  transaction covering database, roster, content headers, dictionary, settings-CSV members, archive
  texture members and PS3 board pairs. The manifest records the save only after the engine committed.
- Write guards: `refuse_guarded_write` (Save As, schedule export) refuses any path inside an open project's
  destinations or import sources; `project_asset_target` routes texture replace / nameplate / texture
  preview to `working/overlay/<install-relative>` (the installed file is copied there on first use and the
  output is recorded as a staged file); kit-texture clones go to the overlay (`clone_kit_textures_to`);
  **Hockey Shop install is refused while a project is open** (its donor clones, colour map and ledger write
  into the installation's rendering tree and are not yet redirected — a recorded gap, §7).
- Deployment: `project_deploy_plan` (read-only; every output mapped to its destination with the
  destination's current identity and state: already-current / unchanged / changed-externally / absent /
  unresolved), `project_deploy_apply` (refuses unsaved edits, a stale handle, a blocked item; writes backups
  under `deployments/<seq>/backups/`, checks free space, publishes through `in_place_save::publish` — sibling
  temp file, rename, verify, reverse rollback, split state named; plans the Xenia / Recompiled content
  header when a roster destination lacks one), `project_deploy_restore` (destination must still hold the
  deployed bytes), `project_deployments`.

### 1.3 Extraction service (§3)

`crates/studio-domain/src/extract_service.rs` + `crates/studio-api/src/registry/extraction.rs`: `preflight`
(read-only: archives parsed, members mapped beside their archive, pre-existing files, cross-archive name
collisions, per-archive markers, free space), `extract` (background job, progress per member, cancel
between members). Rules: an existing equal file is retained; an existing different file is **left alone**
and reported as a conflict; a member name claimed by two archives with no loose file is **blocked for
both** (patch/base precedence unknown, not guessed); unsafe names and reparse-point parents are refused;
writes go through a temp sibling and rename; a per-archive marker (`<archive>.nhlms-extracted.json`)
records the archive identity and each member's outcome, and preflight reports `complete` only when the
marker is complete, matches the archive, and every recorded member still exists; a changed archive
invalidates its marker; a cancelled or failed run keeps what it wrote and reruns resume. PS3 is
`not_applicable` (archive-native). One job per destination; a destination inside a project folder is
refused. The previous `extract_big_files` command is removed; `Registry::extract_install` remains for its
unit tests only.

### 1.4 Onboarding, Settings, Pending Edits (§1)

- `OpenProject.tsx`: **source + project folder** → the unchanged game and console screens → **paths**
  (project folder, game installation folder, save-data location, with Browse; the import source pickers
  and their review for Use my own files; the bundled-baseline note with its gaps; the r69 PS4 candidate /
  problem / stale-roster handling kept; PS3 explains its saves are found under the RPCS3 folder) → Next
  creates the project and runs the read-only preflight → **Extract game assets now?** (the requested text;
  destination shown; Extract now with progress and Cancel; Skip for now) only when something needs
  extracting → Studio. A remembered default pre-selects the target and pre-fills the paths; it never opens
  anything by itself. Stored settings gain `projectDir`; nothing is reset.
- `components/ExtractionPanel.tsx` (shared by onboarding and Settings → Tools), `DeploymentPanel.tsx`,
  `GateHint.tsx` (the requested reason text on hover and for assistive technology: "Container extraction is
  required to edit this asset. Run Extract game containers in Settings → Tools.").
- Settings → General gains **Tools → Extract game containers** on the open project's confirmed game
  installation (same preflight, service, progress, cancel, result; available after Skip; a second job for a
  running destination is refused by the service).
- Pending Edits: **Save project** (replaces Save in place when a project is bound), the deployment panel,
  Save As defaulting to `<project>/exports/`. The shell's Save button is the project save.
- Asset cards: "not extracted here" pills carry the actionable reason; the Player Assets gate names it.

### 1.5 Bindings and commands

New DTOs (24 generated files, export roots 174 → 186); new commands `baseline_inventory`,
`project_create_fresh`, `project_import_review`, `project_create_import`, `project_open`, `project_info`,
`project_save`, `project_set_extraction`, `project_deploy_plan`, `project_deploy_apply`,
`project_deploy_restore`, `project_deployments`, `extract_preflight`, `extract_start`, `extract_status`,
`extract_cancel`, `extract_running_for` (all off the main thread; listed in `command_execution_context.rs`).
`open_workspace` stays registered (no sidecar write) but the frontend no longer calls it.

## 2. Checks

| Suite | Result |
|---|---|
| `studio-domain --lib` (incl. 18 new: baselines 2, targets 1, project_store 6, disk 1, extract_service 8) | 254 / 254 (2 ignored, pre-existing) |
| `studio-api --test project_workspace` (new, 7 cases: fresh isolation + save + reopen + guards; deployment plan / apply / backups / restore / changed-externally / dirty refusal / stale handle; two projects on one destination; import copies and never writes sources; baseline gaps explicit (NHL 20 refused, NHL 21 / NHL 15 PS4 gaps, Recompiled derived, damaged bundle refused); all 27 targets create (26) / refuse (1); extraction preflight / skip / run / conflict / rerun / project-folder refusal) | 7 / 7 |
| `studio-api --test ps4_onboarding_paths` (updated: no sidecar) | 4 / 4 (1 ignored real-install case) |
| `studio-api --lib` (bindings drift gate, 186 roots) | 42 / 42 |
| `studio-api` all tests | 57 binaries, 421 passed, 0 failed, 55 ignored (fixture-gated, pre-existing) — includes the domain lib (254) run in the same sweep |
| frontend `tsc --noEmit` | clean |
| frontend vitest (all; `OpenProject.test.tsx` rewritten, 7 cases: field order and project-folder carry-over, baseline note, r69 roster guard, fresh creation arguments, preflight-gated extraction screen with Skip and Extract now, import review and refusal, NHL 20 refusal, remembered default never opens, unchanged console screen) | 285 / 285 |
| bindings `--write` · `--check` | 24 written, 304 unchanged · in sync |
| `baseline-inventory --check` | in sync |
| r70 `verify.py check` · `selftest` · release-root tests from `E:\tmp` | ok (772 files; 151 bundled baseline files hash-checked) · 21 / 21 refused · 3 / 3 (defs resolve, the bundled baselines verify and create a Legacy project, 83 XMA vectors through the bundled helper, no failures) |

**Not run:** full `cargo test --all`; fmt / clippy (C0.2 baseline); Windows CI; any app launch, game run,
original install extraction or deployment.

## 3. Candidate r70

| Item | Value |
|---|---|
| Archive | `E:\nhlms-release-candidate-20261001-r70\NHL Modding Studio rc-20261001-r70.zip`, SHA-256 `406a864e7a1bddd58809efb64dd7a8154cdb311a7f590d1f98230b02adc51b75`, 772 files, 106 196 307 bytes |
| Executable | `E:\nhlms-review-build-r70-2026-10-01\NHL Modding Studio rc-20261001-r70\nhl-studio.exe`, SHA-256 `56af771f2e484a95d7202dafe0e8124735d24312ab175e4c9f64a94a6d5ecb09` (52 530 688 bytes) |
| Inputs | HEAD `8d60a6d` + 1011 dirty inputs, manifest `2e570b84…0d9e`, identical after the build; built 16:29:23 → 16:32:50 MDT |
| New in the archive | `baselines/INVENTORY.json` + `baselines/files/` (151 files), verified against the inventory by `verify.py` |
| defs / frontend / helpers / anchors | `20c3b183…cf8b` (unchanged) / `be4a903f…3830` / `1caa5f4d…f5c4e` (unchanged); helper `dc3533ed…8c21`; FFmpeg source `7138d28c…78a3`; anchors tree `170fcc9a…1fb9` |
| Record | `internal-review`, `release_ready: false`; blockers unchanged (MIT holder pending; 53 artwork files, rights unrecorded) |
| Evidence | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r70/` |

## 4. Changed dependencies and focused reruns (on r70)

Changed: onboarding (every console), opening (project folder instead of the live install), persistence
and write routing (project save; guards; overlay), deployment (new), extraction (new service). Unchanged:
the database editing surfaces, codecs, discovery rules, the ISO and copied-target advanced routes.

1. **First screen** (user-verification next step): Start fresh / Use my own files with the project folder;
   the unchanged game and console screens; the three path fields in order on NHL 21 PS4, then one PS3, one
   Xbox 360 and the Recompiled entry.
2. **NHL 21 PS4, Start fresh**: the baseline note names the dictionary and roster gaps; the project opens the
   database alone; the save destination may stay empty.
3. **NHL 21 PS4, Use my own files** (owned copies): source pickers, review, creation; the sources' hashes
   unchanged afterwards.
4. **Edit → Save project → close → reopen** (ONB-003 will add the picker; until then reopen = creating a
   second project is not the same — compare the working file in the project folder).
5. **Deploy** to an owned copy of the install / saves: review, Deploy, the backups under the project,
   Restore originals.
6. **Extraction**: Skip at onboarding, then Settings → Tools; a loose edited file is left alone and reported;
   Cancel and rerun.
7. **r69 reruns still due** (stale remembered roster, incomplete inventory): the same guards on the new path screen.

**Invalidated by a changed dependency:** every r67–r69 result for opening, in-place saving and extraction
(the write model changed). **Not invalidated:** NHL21-E1 (its own build and route), PS4 discovery and roster
selection rules (unchanged code; covered by the kept suites), every non-onboarding database editing card.

## 5. Preservation

r67 zip `6297cda9…0b4a`, r68 zip `67de2eb2…c5fd`, r69 zip `322b9b9a…5627`: unchanged before and after this
package. Extracted trees: each verified against its own `MANIFEST.sha256` (619 / 619 ok) before the build and
re-hashed after (tree manifests in the session scratch: identical). r70 went to new folders only.

## 6. Cleanup

Removed: the selftest scratch (`E:\tmp\nhlms-selftest-r70`, removed by the selftest), every test's owned
temp tree (guards). Kept: r67 / r68 / r69 bundles and zips, r70 zip + extracted bundle + assembler `stage`,
the shared `target\` cache, fixtures, `E:\tmp`. Session scratch (patch scripts, preservation manifests) is
under the session's scratchpad and is not part of the repository. No process of this package is running.

## 7. Remaining gaps and limits

- **Hockey Shop install** is refused while a project is open (not yet routed into the overlay).
- **Asset previews** other than the texture preview read the installation; a staged overlay output is listed
  in the project's manifest and the deployment review rather than previewed in place.
- **Archive member edits** copy the whole installation archive into `working/` at save time (free-space
  checked), and a deployment backs up the whole destination archive; this is the established copied-target
  cost, now disclosed in the plan's byte counts.
- **Roster save creation**: a project with no roster destination (no slot found, or PS4 NHL 20 / 21 with no
  vanilla roster) cannot create one; the plan says `unresolved`.
- **ISO route** and the copied-target advanced routes are unchanged (their own staging / roots).
- **Extraction** buffers BIGF/BIG4 archives in memory (carried over); the stored-size estimate is a lower
  bound for chunkzip members; markers key on length + mtime + a 64 KiB head hash, not a full archive hash.
- **Undo history** is not persisted across a reopen (the saved files are); ONB-003 will disclose this.
- The committed `.nhlmodproj` sidecar code remains for reading; nothing writes it.
