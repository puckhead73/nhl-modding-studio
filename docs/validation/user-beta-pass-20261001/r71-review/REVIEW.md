# r71 coordinator review — changes required

2026-10-01. ONB-003 is **not accepted**. Candidate identity checks pass, but six owned-copy regression probes reproduce five findings. No original game/save was opened or written, no application/game was launched, and no implementation was changed. This is automated/source review, not user or runtime acceptance.

## Findings

### R1 — P1: package edits bind to row numbers and can edit the wrong player

`crates/studio-api/src/registry/package.rs:154, :701, :734, :1067` serialize and replay physical record indexes. The entity label is display text; it does not bind the recipient entity. A matching field value is not proof of matching identity.

Reproduction: author two NHL21 database edits using the existing test helper. On an independent recipient, swap complete player rows 0 and 7, including their keys, then recompute CRCs. Both players originally have the same position value. Preflight accepts without conflict, and Apply changes Travis Zajac (key 22), now at row 0, instead of Paul Stastny (key 5), now at row 7. The unrelated player's position changes from raw 0 to 5. This is not a cross-year request: both inputs are NHL21 PS4.

Required: stable, verified entity bindings and role-specific preconditions. Resolve recipient keys and relationships before replay. For operations without a proven mapping, reject incompatible identity/layout explicitly. Matching author/target *bundled* hashes does not establish the recipient's layout. Conflict override must never authorize an identity guess.

Probe: `coordinator_reordered_player_is_not_edited_by_old_row_number`.

### R2 — P1: file roles are not enforced; edits spread or disappear

`package.rs:608–635, :679–765` derives requirements from whether *all* cells are roster-only and builds a workspace that may mirror edits automatically.

Two reproductions:

- A Legacy PS3 project imports only `nhlng.db`. Its package explicitly contains only `main_db` operations. Installing on an owned recipient that also has a roster changes that roster. Roster SHA-256 changes from `c079ec4b5ae7ce2aa5b678373a5cb250c5762eb333f7dfe8bb05a35080744cb9` to `cde2b7fb2f2b7d308460e63efc871badebf8f930d3d636d544ad942f09f2867a`.
- A fresh Legacy PS3 project creates `both` operations. Remove the owned recipient roster before preflight: the plan is still installable, with no blocking reasons. A required half of the package has been dropped.

Related source path needing regression coverage: PS4 picks a roster as primary whenever one is present, even for `main_db` operations. Non-PS4 discovery can pick the first roster automatically. Role-specific edits need independent targets and must preserve explicit selection/stale-choice safeguards.

Required: derive complete dependencies from every selected operation; main-only never writes roster, roster-only never writes main, both requires and checks both. Do not merge operations with different prior values or identities into a lossy `both` record. Missing or unaligned required targets block the whole plan. Unrelated saves must not become dependencies of DB-only/asset-only packages.

Probes: `coordinator_main_db_only_does_not_change_recipient_roster`, `coordinator_both_roles_requires_roster`.

### R3 — P1: restore can strand a mixed result and reject recovery

`package.rs:1014–1060` publishes restored existing files first, then deletes newly created outputs outside that coordinated operation. A failed removal exits before updating the record.

Reproduction: install a Legacy package on an owned Recompiled content tree. The actual installer creates the missing save-index header. Hold that header open with Windows delete sharing disabled, first confirming deletion is denied. Restore reverts the DB and roster, fails to remove the header, and leaves a committed record. Release the handle and retry: Restore refuses the already-restored DB as a foreign change. The error is produced by the installer's own previous partial restore.

Additionally, `package_installs` offers recovery only for `committed`; `package_restore` rejects `publishing` and `split`. Merely retaining backups is not the checked recovery workflow required by ONB-003. The equivalent remove-after-publish pattern also exists in project deployment restore (`registry/project.rs:1237`); correct the shared recovery behavior rather than adding a competing transaction engine.

Required: coordinate writes and deletions, durable per-item state, interruption/failed-rollback recovery and safe retry. Preserve true foreign changes. Include required header outputs in reviewed preflight, not only at Apply.

Probe: `coordinator_restore_can_recover_after_sidecar_removal_failure`.

### R4 — P2: asset-only Project Save invokes the database workspace

`registry/project.rs:889–893` calls `with_workspace` for all projects. `AppShell.tsx` routes project Save to this API.

Reproduction: create NHL21 asset-only project, stage a supported loose PNG replacement, then call Project Save. It fails with “this project has no main database or roster save, so database editing is unavailable”. This does not establish that staged texture bytes were lost; it establishes that the advertised Save operation rejects a supported project mode.

Required: make Save valid for asset-only projects and prove save/close/reopen/package durability without importing a dummy database. Already-durable staged assets may produce an accurate no-op save result.

Probe: `coordinator_ps4_texture_package_uses_game_asset_root` fails at Save, before reaching its texture install assertion.

### R5 — P2: PS4 texture installer searches the DB folder as the asset root

`package.rs:636, :785–794` prefers `report.install_dir`; PS4 discovery assigns that to the DB's parent (`discovery.rs:608`).

Reproduction: repeat with a DB plus asset input to pass Save. Package a supported loose PNG at `fe/ion/artassets/leaguelogos/l0.png`; put an identical original at that relative path under an independent recognized NHL21 game root. The package builds as supported, but preflight claims the existing extracted file is absent. The installer resolves under `nhl/data/common/gamemodes/database/t3db` instead of the asset root.

Required: explicit platform-correct asset roots and placement recipes; prove preview, install and restore on owned extracted trees. A synthetic PNG can cover this routing bug now; representative native texture/uniform container evidence is still required for broader claims.

Probe: `coordinator_ps4_texture_with_database_uses_game_asset_root`.

## Evidence and boundaries

- ZIP SHA-256 independently matches `a51b907ac3311a833bef58c07bdf104b70edb4097b5f578914fd3528a8902309`.
- EXE SHA-256 independently matches `1359e186dff2584a304e5bd074f3ecb876c01adadf05be64988db38dd03ddee8`.
- Independently checked all 771 entries of the extracted bundle's `MANIFEST.sha256`: zero failures.
- Reviewed source files match r71's `dirty-inputs-before.txt`: package registry, project registry, domain package/discovery, original package/input tests and AppShell.
- Copied the existing four package integration tests to a temporary sibling and added six probes. The four original cases passed. The final six-probe run: **0 passed, 6 failed**, all at the reported defect assertions/error. Final output: [coordinator-probes.txt](coordinator-probes.txt).
- [probe-output.txt](probe-output.txt) preserves the earlier combined run: original four pass; the recovery probe initially stopped on its author-platform declaration setup, corrected before the definitive final run. Do not count that initial setup failure as evidence of R3.
- Complete runnable reproduction: [onb003_coordinator_probe.rs.txt](onb003_coordinator_probe.rs.txt). Copy to `crates/studio-api/tests/onb003_coordinator_probe.rs`, run `cargo test -p studio-api --test onb003_coordinator_probe coordinator_ -- --nocapture`. The file includes the original helper/tests and uses only temporary owned write targets. Coordinator removed the temporary test sibling after preserving it.
- Did not rerun all domain/frontend/API suites, archive selftest, or Windows CI. Did not independently rehash every older candidate tree. None was changed by this review. No user checks promoted.

## Scope still requiring qualification

The matrix explicitly declares cross-platform textures unsupported. That is honest, but the requested Legacy roster-plus-logos/uniforms multi-platform use case remains incomplete. Positional cell replay is also not stable semantic project/package identity. Archive-member/native-only/structural/settings-CSV/schedule/dictionary-growth exclusions remain documented limitations, not accepted implementations of those package paths. Missing codecs can remain explicit blockers; do not mark ONB-003 wholly complete by relabeling them as passed.

Keep r71 frozen for evidence. Correct the named paths, qualify supported behavior on owned targets, produce the next unused candidate, and return for review before the user package/install checklist. See [CORRECTION-HANDOFF.md](CORRECTION-HANDOFF.md).
