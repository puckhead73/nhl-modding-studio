# ONB-002 — persistence design and changed contracts (developer note)

Developer note for [ONB-002](onboarding-20261001-project-workspace.md), written before implementation
(delivery step 1). It records the minimal model the package implements and every contract it changes.
It is not evidence of acceptance.

## 1. Target / source / destination model

| Term | Meaning | Where it lives |
|---|---|---|
| **Target** | One of the 27 shipping `(game_id, platform)` pairs. | `studio_domain::targets::SHIPPING_TARGETS` (new; the frontend catalog stays the UI source). |
| **Source** | Where a project's starting bytes came from: `fresh` (bundled baseline) or `import` (user files). Sources are read once at creation and never written. | Manifest `source`. |
| **Baseline** | Immutable copies of the source files with their identities, kept under the project. | `<project>/baseline/`, `BASELINE.json`. |
| **Working** | The copies the edit session opens and the project save rewrites. A `tdb-txn` copied-target root. | `<project>/working/` (marker `COPIED-TARGET.tdb-txn`). |
| **Destination** | Where a deployment writes: the game installation (database folder, archives, loose assets) and the save location (profile / slot). Chosen on the path screen, revalidated before every deployment. | Manifest `destinations`. |
| **Overlay** | Project-owned outputs of loose-asset routes (texture replace, nameplate, kit clone, Hockey Shop), stored install-relative. | `<project>/working/overlay/<install-relative path>`. |

## 2. Bundled baseline inventory

- `baselines/INVENTORY.json` (schema `nhlms-baselines/1`) is generated from `anchors/` by
  `cargo run -p studio-api --bin baseline-inventory -- --write` and lists, per shipping target, the
  approved components (`main_db`, `meta`, `dictionary`, `dictionary_meta`, `roster_save` files), each
  with its path relative to the files root, length, SHA-256, provenance status and a witness note, plus
  the target's explicit gaps. Save components carry the codec probe (`tdb-savedata` container
  version / `is_ps4_save`, writable, experimental).
- The approval table is in the generator source (`crates/studio-api/src/bin/baseline_inventory.rs`)
  so a reviewer sees exactly which anchor is admitted for which target and why.
- Runtime resolution (`commands::resolve_baselines`): `<resource_dir>/baselines/INVENTORY.json` with
  files under `<resource_dir>/baselines/files/`; in a development run, the repository's
  `baselines/INVENTORY.json` with files under `anchors/`. The release assembler copies the approved
  files into `baselines/files/` and the inventory beside them; `verify.py` covers them through the
  manifest. The release-layout test opens a title from the bundled store.
- Every copy out of the store re-hashes the file and refuses a mismatch (a damaged or substituted
  bundle never becomes a baseline).

Provenance statuses: `verified-vanilla` (anchor hashed against the install it is labelled as, or an
anchor-only Xbox 360 title with no installed counterpart), `cross-platform-schema` (PS3 NHL 10 meta
from the byte-identical Xbox 360 database), `derived` (Legacy Recompiled: Xbox 360 Legacy anchors,
witnessed by handoff R128), `unavailable` (listed, not bundled).

## 3. Project folder layout and manifest

```
<project>/
  nhl-studio-project.json   manifest (schema nhlms-project/1), written temp → rename
  baseline/                 immutable source copies + BASELINE.json
  working/                  tdb-txn copied-target root: db/, loc/, saves/, overlay/, .tdb-txn/
  deployments/<seq>/        record.json + backups/ (one per explicit deployment)
  exports/                  default Save As location
```

Manifest fields: `schema`, `project_id` (random), `created`, `game_id`, `platform`, `title`,
`source {kind, inventory_version?, components[]}`, `working {main_db, meta, dictionary?, roster_save?}`
(project-relative), `destinations {picked_root, install_dir?, game_dir?, save_root?, roster_save?,
localization_path?, main_db_path?}`, `legacy_sidecar` (path of a `.nhlmodproj` seen in the install;
read only, never migrated or deleted), `extraction {choice, at}`, `saves[]` (transaction ids),
`deployments[]` (seq, state).

Folder rules: the project folder must not be inside, equal to, or contain the game root, install
dir, game dir, save root or any import source, compared on canonicalised paths (junctions and symlinks
resolved, Windows spelling), and must not itself be a reparse point. A non-empty folder that is not a
Studio project is refused. Conflicts are reported; no other folder is chosen.

## 4. Session, save and reopen

- `Registry::project_open(project_dir, defs)` builds the same workspace shapes `open_workspace` builds
  (gen7: main database + aligned roster mirror; PS4: roster save as the single session or the template
  database alone) **on the working copies**, attaches the project's dictionary, and binds the
  project id to the folder. Nothing in the installation or save location is opened for writing.
- **Save project** = `copied_target_review` + `copied_target_apply` on `working/`. That is the existing
  journaled transaction (database, roster, content headers, dictionary, settings-CSV members, archive
  texture members, PS3 board loose pairs). Archives an edit needs are materialised into `working/`
  from the installation first, verified against the identity the edit was planned on. A failed or
  refused save leaves the session's edits and the previous working files untouched; the project is
  never marked saved unless the engine committed.
- **Reopen** = `project_open` again: the working files are the baseline of the new session. Undo/redo
  history is not persisted (ONB-003 will disclose this); the saved edits are.
- The old `open_workspace` no longer writes `.nhlmodproj`. Detection reports an existing sidecar
  read-only.

## 5. Write routing

| Route | Before | Now |
|---|---|---|
| Save (in place) | live files | the project's working files (`project_save`) |
| Save As | any path, default = live db | refused inside any destination root; default `<project>/exports/` |
| Texture replace / nameplate | `install_dir/<rel>` + `.bak` | `working/overlay/<rel>` (prior copied from the install once); staged file recorded |
| Kit texture clone | beside the sources | `working/overlay/<same rel>` |
| Hockey Shop install | `rendering/<folder>` | refused while a project is open (needs a redirected rendering root; recorded gap) |
| Extraction | install (fs::write, overwrite) | install (authorised exception), hardened — §6 |
| Schedule export, asset exports | any path | refused inside destination roots |
| Copied-target, disc-image routes | user-chosen roots | unchanged |

## 6. Extraction service

One service (`studio_domain::extract_service`) behind onboarding and Settings → Tools:
`preflight(root)` (read-only: archives, members, name collisions, existing files, estimate, markers),
`start(root)` (background job with progress and cancel), `status`, `cancel`. Writes go through a
temporary sibling and rename; an existing equal file is retained; an existing different file is never
overwritten (recorded as a conflict); a member name claimed by two archives with different bytes is
blocked for both (precedence unknown); entry names and resolved parents are contained in the archive's
directory with reparse points refused. A per-archive marker (`<archive>.nhlms-extracted.json`) records
the archive identity and per-member outcomes; preflight reports `complete` only when every supported
archive's marker matches and every recorded member still exists. One job per destination.

## 7. Deployment

`project_deploy_plan(id)` maps working outputs to destinations (main database → discovered database
path, roster save → chosen slot or "to be created", dictionary → discovered dictionary, overlay files
→ install-relative, materialised archives → the installation's archive), with each destination's
current identity, whether it changed since the baseline or the last deployment, and the database cell
diff. `project_deploy_apply(id, handle)` refuses unsaved edits, writes backups under
`deployments/<seq>/backups/`, checks free space, then publishes through `in_place_save::publish`
(sibling temp file, rename, verify, reverse rollback; split state reported). `project_deploy_restore`
puts the backups back under the same publisher. Cancel before apply is non-mutating.
