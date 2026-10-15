# ONB-003 — portable projects, resumption, mod package builder and installer

Revised user direction, 2026-10-01. This replaces the earlier resume-only ONB-003 specification. It follows ONB-002 delivery/review and reuses its project-owned storage, extraction and transaction infrastructure.

The user's explicit request includes flexible input-only projects, platform-agnostic saving, a single-file distributable mod package builder and the built-in package installer in this work. It overrides the future-release phrasing in the supplied vision for those named functions. **Cross-game/year conversion, generic destination-independent NHL projects and automatic removal of unsupported teams/leagues/edits remain deferred.** Same-game cross-platform application is in scope through verified adapters; Legacy Xbox360, PS3 and Recompiled are the user's concrete example. An emulator is a runtime for a target platform, not a format identifier.

The same-day release goal does not change the required evidence or authorize publishing. Preserve all numbered candidates and earlier observations; keep unrelated development frozen. This is a specification/developer handoff, not implementation or a message sent to another session.

## 1. Required input combinations and onboarding

Support a project with any one of the following, or compatible combinations:

| Inputs | Usable work |
|---|---|
| Extracted game asset files, no roster save | Supported asset browse/preview/edit/import without requiring a roster. Do not impose a DB requirement on an asset route that does not need one. |
| One recognized roster save only | Supported roster/database edits without a game installation or asset extraction. |
| One native main database only (nhlng.db / nhl_game.db or the target's actual name) | Default-game-database edits without a roster save or assets. |
| Any compatible combination | Combined supported work; retain each source's role and provenance. |
| Start fresh | Selected target's verified bundled vanilla databases and usable save templates from anchors; no bundled game archives/assets. |

Determine game/platform/variant from reliable file evidence or an explicit user selection and validate compatibility. Resolve needed schemas/metadata from verified bundled resources where appropriate; do not secretly require a full install for roster-only/database-only input. If an exact schema/codec is unavailable, explain that specific limitation rather than treating the input as supported.

The input modes must be working application paths, not UI placeholders. An asset-only session needs a first-class project context independent of a DB-backed workspace. Establish related identity data only for an operation that requires it; do not auto-import a fabricated roster to satisfy an old minimum.

Retain the launch choices **Start fresh**, **Use my own files** and **Open existing project**. Opening a mod package is a fourth distinct action, **Install mod package**, also reachable from the relevant Tools area. Game/platform selection for new projects retains ONB-002's existing catalog UI.

All27 targets still show the project/game-install/save-destination fields. **New precedence over ONB-002:** only the project folder and the selected input prerequisites are mandatory for project creation. Game/save deployment locations can be unset until an operation actually needs them. Suggest detected paths, label them as final installation destinations and allow Browse/edit. A roster-only user must not have to install/extract a game to start editing. Destination selection never changes which source file is being edited.

Keep independent own-file source selectors. Preserve multiple-source distinctions without importing arbitrary mixtures across game years. Incompatible sources refuse with a specific explanation; no silent target conversion.

Full extraction remains optional under ONB-002, offered only when a relevant install exists and the chosen operation needs extracted content. Its Settings tool, explicit installation write, conflict handling and per-operation gates remain unchanged. Missing assets disable dependent features, not independent roster/database work. Unimplemented codecs/writers must not be described as merely needing extraction.

## 2. Portable, durable project representation

Use the ONB-002 project layout/manifest where viable, extending it with an explicit versioned schema and migration rules. Do not create a second unrelated project format or a second installer transaction engine.

A project contains:
- A stable project identity, name and manifest/schema version.
- Its target game identity and source platform/variant information; explicit compatible target/platform declarations with evidence.
- Relative paths to project-owned baselines, source assets, working files and outputs, with hashes and provenance.
- Durable database/roster/dictionary operations and relationships, including original/expected values, current staged values, operation order/dependencies and stable semantic entity bindings.
- Editable source assets and their replacement/assignment intent: for example original DDS/PNG textures, GLB or other actually supported mesh interchange sources, and supported audio source files plus relevant parameters.
- Shared-asset consumers, catalog/variant mappings, dimensions, mips, material/mesh/rig or audio-loop/channel constraints needed to reproduce an admitted target-native result.
- Staged-versus-deployed state, coherent transaction/recovery history and last meaningful Studio context.
- Separate machine-local installation/save/profile bindings that can be relinked and are not exported as recipient destinations.

Platform-agnostic means durable intent/source data is not replaced by one platform's packed output. Preserve original editable asset files and explicit conversion recipes; produce native textures/containers/DBs at build/install time using supported target adapters. Native baselines/caches may coexist for fidelity/recovery, but cannot be the sole representation advertised as portable.

Raw source field offsets, row numbers, asset IDs and whole-file binary patches are not intrinsically platform independent. Store their source scope and verified semantic mapping. For fields/entities with no verified portable binding, mark the operation target-specific and limit package compatibility. Never guess that an ID, enum, endian layout, rating scale or asset path is identical across platforms.

Do not promise arbitrary DDS/GLB/audio conversion merely because that extension can be stored. Preserve sources losslessly; actual conversion/installation requires a supported encoder/adapter, measured material/rig layout and native writer. Incomplete external head-conversion integration remains a concrete blocker for those operations.

All edit routes save into the project before deployment, including textures, meshes, audio, imports, dictionaries and archive-member replacements. Do not leave asset tools writing into the live installation while database edits are staged safely. Allow incremental project saves without committing to the game.

Persist accepted/staged work atomically or recoverably; protect concurrent writers. Changing project views, normal closing and reopening must not lose work. Resolve invalid/incomplete field drafts explicitly rather than dropping them or bypassing validation. Keep undo/redo history durable where supported; accurately disclose a history limitation without losing the working state. Lost history is not permission to discard edits.

Bundled vanilla material comes from a versioned verified anchor inventory, independent of the developer checkout. Ship needed databases/save templates/metadata, **not .big archives or all games' assets**. Reconcile known exact-title gaps (including the NHL20 alias) and usable save-format coverage before advertising a fresh baseline. Never mutate anchors or substitute another title's baseline silently.

## 3. Open existing project and resume

Select the persisted project manifest/folder, validate it and resume directly in Studio. This is not importing its files into a new project.

Restore the same project, game/platform/variant, linked install/save profile/slot, edited file state, pending changes and deployment status. Restore the last meaningful page/record/view where valid; otherwise explain the fallback. Do not use newly bundled vanilla versions to reset an existing project.

Relative internal paths must permit moving a self-contained project to another directory/machine. External paths can be unavailable; retain project work, allow independent work and explicitly relink affected resources. Never choose the first other save/profile/game in place of a missing prior selection.

Reconstruct live review handles from durable state and revalidate sources/targets. Opening must not install outputs, resume an interrupted deployment/extraction automatically, run the game, or write the legacy install-side .nhlmodproj.

Malformed/incompatible manifests, missing project payloads, interrupted saves and concurrent project opens require specific non-destructive errors/recovery. Legacy r67/r69 install-side profiles are not full project snapshots; any migration must say what can actually be recovered and preserve the original.

## 4. Build a single-file distributable mod package

Provide a real **Build mod package** workflow:
1. Choose the included project changes/assets and package identity/version/description.
2. Show supported game/platform/variant targets and per-edit compatibility; distinguish verified adapters from unsupported conversions.
3. Validate every selected operation, payload, dependency and intended target before producing a package.
4. Build one versioned, self-contained installer-package file that another Studio installation can inspect and install without the author's project or original paths.

Use a documented data-only container (for example a ZIP-based format with a dedicated extension), not an executable/self-extracting installer or user-supplied scripts. A recipient installs it through Studio. No network upload or distribution action is part of package creation.

The package carries a manifest, required Studio/package schema version, target-game compatibility, payload hashes/sizes, semantic edits, source assets, per-target conversion/placement requirements, dependencies and conflict/precondition information. Keep build/version/adapter provenance sufficient to diagnose a produced output.

Include only selected authored/modified payloads and required edit data; do not dump the author's baseline installations, vanilla asset archives, whole working directory, recovery backups, save-account metadata or absolute machine-local paths into the package. Where a native replacement is genuinely required, mark it as exact-target data with its base requirements, not a universal edit.

A package for a given game may declare several platforms, with explicit component coverage and supported adapters. Same-title pack/build differences need measured compatibility. Do not infer PS3 or Recompiled support from an Xbox360-only test.

If any selected operation cannot be represented or installed on a declared platform, report the item and reason before build. The author may explicitly narrow supported platforms or revise selected contents. Never silently omit an edit, downgrade the target list or claim a universal package from partial mappings. Cross-game automatic omission remains deferred.

Builder failures must leave the prior good package intact and not mutate the project or deployment targets. Provide an inspectable summary of contents and target support. A file existing is not proof the installer can apply it.

## 5. Built-in package installer

Provide **Install mod package**, available without the author's project and without first importing it as one. The recipient chooses a package, then a supported destination game/platform and any required game/save/profile paths.

Read/validate the package before writes. Show mod identity/version, declared targets, included categories, required resources, compatibility results and exact planned destination changes. Preserve r69's explicit save selection and stale-choice safeguards. A database-only or asset-only package requires only its actual target dependencies, not an unrelated roster.

Never trust package-supplied destination paths or code. Enforce safe relative paths, size/count/decompression limits, no traversal/absolute/drive/UNC escapes, duplicate/case-insensitive entry collision detection and reparse containment. Reject unsupported manifest versions, missing or mismatched hashes/payloads, invalid operations and malformed formats before installation. Hashes establish integrity, not publisher authenticity.

Use the selected platform's verified adapters to materialize:
- DB/roster/dictionary edits against the recipient's compatible base and native schema/container.
- Texture/model/audio payloads into supported native formats.
- Correct archive/loose placements, shared variants and lookup bindings for that exact target.

Do not replace a recipient's whole customized roster just to apply a rating change. Represent a deliberate whole-roster replacement separately and disclose its overwrite scope. Save-account/profile/container requirements are validated, not assumed transferable from the author's machine.

Staging/conversion happens in an installer-owned workspace outside live targets. Preview/preflight is non-mutating. Before Apply, verify target identity/content, required disk space, existing mods/collisions, complete dependencies and current source state. Changed targets invalidate the plan. An unsupported selected edit blocks that installation plan; no automatic partial installation.

Install only after explicit user Apply. Use the existing coordinated transaction/journal/backup/restore engine for the complete reviewed change set. Account for multi-file/paired/archive writes and separate save/game destinations. Retain recoverable state if rollback fails. Never silently continue after uncertainty about committed files.

Record what was installed, where, package/version, before/after identities and recovery locations. Provide restore/uninstall of this installation through a checked recovery route; it must not overwrite later foreign changes or another mod's work. Explain conflicts and preserve evidence when restore is unsafe. Do not delete unrelated files or consume pre-existing backups.

Do not run a game, execute package code or automatically download dependencies as part of install. Distribution through Discord/forums remains the user's separate action.

## 6. Scope boundaries and genuine completion

In scope now: flexible input combinations; fresh bundled data; durable project-owned editable sources/operations; resume; package builder; same-game supported-platform installer; reviewed deployment and recovery.

Deferred:
- One generic NHL project installable into arbitrary game years.
- Cross-year roster/entity/league/asset conversion and automatic dropping of unsupported teams/leagues/attributes.
- Automatic resizing/expansion of engine league/team capacities.
- Unmeasured generic model/audio/texture conversions or arbitrary cue synthesis.
- Previously excluded league/team creation/deletion/expansion and executable patching.
- Unrelated canonical authoring features beyond the model actually required for this package.

Retain all27 target identities and inventory support per operation/adapter. Build a complete functional package workflow; do not present manifest scaffolding or a same-platform file copier as finished portable installation. Where a required adapter/codec is absent, identify the exact blocker, continue independent work and report unsupported scope honestly. A packaged source asset alone is not an install pass.

The Legacy 1992–93-style scenario is an acceptance use case for the already-supported edits (roster data, logos, uniforms, etc.), not permission to create missing engine capabilities or silently omit requested content. Independently qualify Legacy Xbox360, PS3 and Recompiled where those edits are admitted.

## 7. Implementation and review sequence

Do not alter an in-flight ONB-002 build silently. After its delivery/review, inspect its actual format and services, then implement this expanded ONB-003 in the same authorized development lane:

1. Input admission/capability model: asset-only, roster-only, native-DB-only and combinations; remove global all-files-present requirements while keeping actual codec/operation requirements.
2. Extend durable project/source/semantic-change representation and resume/migration handling.
3. Implement package schema, builder, content validation and compatibility inventory.
4. Implement installer preflight/adapters/staging/transaction/restore UI end to end.
5. Prove the full workflow with isolated targets and package/relocation/adversarial cases; package a review candidate.

These are internal implementation milestones, not permission to ship an incomplete product as complete. Keep implementation incremental and report genuinely blocking input/adapter questions. No parallel sessions or messaging other chats authorized.

## 8. Required evidence

Meaningful checks, using owned copies/synthetic fixtures for all writes:
- Every allowed input combination opens and permits its actual independent operations; absent resources gate only dependent features. No installation required for supported roster-only/default-DB-only work.
- Fresh and imported projects save/reopen with exact DB/save/dictionary/asset edits, pending state and source provenance; originals/targets unchanged.
- Two projects with the same game and two different-game projects remain isolated. Moved/copied projects reopen without the original absolute root; broken external links do not substitute another target.
- Invalid draft, interrupted save, no space, corrupt manifest, concurrent opens and incompatible version preserve recoverable data.
- Build a package, move it to an unrelated directory, make the author's project inaccessible, and install against an independent owned compatible base. Confirm complete intended output and unrelated-data preservation, then restore baseline.
- Verify at least one admitted database/roster edit and representative texture/uniform asset edits across declared same-game platform variants. Independently test PS3, Xbox360 and Recompiled; no alias qualification. Mesh/audio conversions require their own actual adapter evidence.
- Inspect package contents: no whole baseline/archive dump, private absolute paths or account/save secrets accidentally included; payload list equals selected changes.
- Wrong game/year/platform/base; unknown adapter/codec; renamed/stale profiles; missing dependencies; cross-game package; unsupported edit: refuse with an actionable reason and zero live writes. No silent omitted teams/leagues/edits.
- Package traversal, reparse, case collision, corrupt/missing/hash-mismatched entries and excessive payloads: reject before installation.
- Destination change after review, collision with existing modifications, mid-write failure, failed rollback and restore after foreign modifications: exact coordinated outcome and preserved recovery.
- Targeted routing across all27 catalog entries; genuine NHL20 source and independently admitted Recompiled baseline. Distinguish synthetic, copied-fixture, user-observed and runtime evidence.
- Packaged Studio runs outside the checkout, resolves included baselines/defs/helpers, and loads an installer package without development dependencies.

Do not force audio/mesh tests through unsupported adapters and call a clean refusal a completed conversion. No original game/save writes, full-install extraction, game launches or external publication for developer validation.

## 9. Delivery and user verification

Produce the next unused numbered internal candidate after ONB-002's actual candidate, never a guessed number that overwrites an existing build. Keep prior bundles unchanged. Deliver exact source/build manifests, executable/ZIP hashes, resource/package verification, developer receipt and operation/target compatibility matrix. Record tests, skips, adapter/fixture gaps and retained recovery artifacts. Do not edit generated capability ledgers or user-result records to promote support.

No commits, pushes, publication, uploads or original deployment authorized. Freeze after delivery for coordinator review.

Changed dependencies now include project admission/opening, persistence, edit routing, package conversion and installation/recovery. Identify focused reruns; preserve earlier accepted evidence for its historical scope instead of erasing it or treating it as new-platform/package acceptance.

First user pass: create a small NHL21 project from one supported input; stage a known change; close/reopen; build a package; inspect its declared target/content; install and restore against an explicitly owned test destination. Follow with separately scoped same-game multi-platform qualification. The user's release-today intention is a goal, not a completion claim or permission to skip these checks.
