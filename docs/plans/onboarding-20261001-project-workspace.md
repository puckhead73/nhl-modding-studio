# ONB-002 — project-folder onboarding and optional extraction

Authority: explicit user direction on 2026-10-01, after acceptance of r69 for internal verification. User subsequently confirmed that **Start fresh uses bundled verified vanilla files from `E:\Repositories\nhl-modding-studio\anchors`**. This is a requested workflow revision, not another N21-001 bug fix.

**Subsequent scope update:** [expanded ONB-003](onboarding-20261001-resume-projects.md) adds flexible asset-only/roster-only/database-only input admission, portable editable project data, resumption, a distributable mod package builder and built-in same-game platform installer. It explicitly permits game/save destination fields to remain unset until needed; only the project folder and actual input dependencies are mandatory. Its newer requirements take precedence over conflicting all-files/path requirements below. Keep this package's delivery/review boundary rather than silently changing an in-flight ONB-002 candidate. Cross-game/year conversion remains deferred.

This direction supersedes the earlier direct-to-install onboarding and the deferral of project folders/staged output **for the operations specified here**. It lifts the freeze for this package only. It does not revive the entire canonical project.db/Studio-ID/multi-source composition/immutable-build programme; those unrelated features remain deferred. All other feature development stays frozen. Claude owns implementation; this document is a coordinator dispatch, not evidence of implementation or user acceptance.

Preserve r67, r68 and r69. Keep old verification results and scope. The new candidate must identify the changed dependencies and reruns; do not silently relabel prior direct-edit results as project-workflow acceptance.

## 1. Required user flow

### First screen: starting data and project folder

At programme launch the initial new-project flow presents:

1. **Start fresh** — use bundled, verified vanilla databases and saves for the selected title/platform/variant.
2. **Use my own files** — import the user's chosen database/save inputs into the project; preserve originals.

Both choices require selection of the project folder, where working files, persisted staged edits and produced outputs live. This selection is made here and carried forward to the later path screen; do not ask for a different folder a second time. Permit review/change there.

**Sequencing clarification from the user:** the separate user-facing **Open existing project** flow follows this package in the expanded [ONB-003](onboarding-20261001-resume-projects.md), alongside portable project saving and package build/install. ONB-002 still establishes durable project state and tests persistence/reopen at the service level; do not postpone that foundation or conflate project resumption with importing database/save files. Existing remembered auto-open must not bypass the new workflow or open live saves under the old write model. Do not destroy/reset stored settings to implement the transition.

### Second screen: game/platform

Keep the existing game/platform selection screen unchanged. Use the actual27 shipping combinations: Xbox360 NHL07–15/Legacy; PS3 NHL08–15/Legacy; PS4 NHL15–21; Legacy Recompiled. No NHL07 PS3 shipping target or Xbox One target.

### Third screen: paths

Every title/platform shows these fields, in this order, with Browse buttons:

1. **Project folder** — destination for working/staged edited files.
2. **Game installation folder** — final in-game deployment destination and source of installed asset resources.
3. **Save-data location** — final save deployment destination, with applicable profile/slot selection.

Use existing discovery rules to suggest/prepopulate values, but allow confirmation/correction on every platform. The game field represents the actual selected game's installation, not merely the RPCS3/shadPS4 executable folder; existing emulator/library discovery may resolve a suggestion to that game's installation. Keep the platform-aware adapters and r69 explicit ambiguity/stale-choice handling.

The save destination may be absent for a new project/slot. Distinguish a destination to create later from existing import inputs; do not require an original live save solely to create a project with a supported bundled baseline. Required profile/account/container metadata must be validated before deployment. Never fabricate a signed/usable console save from an unsupported input.

For **Use my own files**, include explicit source-file selection/review in this flow. Source database/save paths are not replacements for the three destination fields and must not be silently treated as deployment destinations. Existing selected source saves can suggest a destination, but the role remains visible and editable. This implements the requested own-files choice without conflating read sources and write targets.

### Fourth screen: extraction, only when needed

After Next on the path screen, preflight the selected installed files without writes. Determine which supported containers/dependencies have actually been extracted. Do not infer completeness solely from a folder existing or from one database being present.

If extraction is needed, present a separate choice:

**Extract game assets now?**

> Extract the game's supported containers once to make their contents available for direct asset editing, including textures, models and audio. This can take a few minutes for Xbox360/PS3 games and significantly longer for PS4 games, depending on the installation and storage. You can skip this now and run it later from Settings → Tools.

Actions: **Extract now** and **Skip for now**. Duration is guidance, not a guaranteed estimate. Show the resolved extraction destination and progress/cancellation; explain partial/failed extraction accurately.

If already complete, continue without a redundant prompt. Existing archive-native capabilities that do not require bulk extraction remain available; do not disable them just because another asset family is unextracted.

### Load Studio

Both choices load the project whenever its own database/save prerequisites are satisfied. Skipping bulk extraction must not prevent independent database editing. If a selected own-file source is still packed, use an existing supported targeted read/extract-to-project route for required DB/metadata rather than demanding full asset extraction. If the source cannot be opened by any supported route, name that exact prerequisite; do not promise unavailable readers.

Grey out only edits whose actual dependencies require unavailable extracted content. On hover use the requested explanation, for example **“Container extraction is required to edit this asset. Run Extract game containers in Settings → Tools.”** Also expose the reason on keyboard focus/accessibility text; disabled controls alone cannot reliably receive hover/focus. This is a dependency gate, not a blanket lock on every field in a record.

An unsupported writer/codec/feature remains labeled unsupported; do not tell users extraction will implement it. Partial extraction unlocks only proven available operations. No missing requirement is counted as complete by displaying a tooltip.

### Settings

In the primary Settings tab add a **Tools** section with **Extract game containers**. It operates on the current project's confirmed game installation and uses the same preflight, extraction service, progress, cancellation, result and capability refresh as onboarding. It must remain available after Skip. Do not launch a second extraction against the same destination while one is running.

## 2. Sources, project persistence and deployment

This is functional isolation, not a renamed `.nhlmodproj` sidecar.

- Package an explicitly versioned, hashed inventory of approved anchor files beside the application. Runtime must work outside the developer checkout. Never use a hardcoded repository path or download another tool's data.
- On fresh creation, copy the chosen valid vanilla database/save/metadata/dictionary set into the project. On import, validate and copy the user-selected inputs. Preserve native formats and title/platform/variant identities.
- Keep baseline identities/provenance and recoverable working copies under the project. A fresh project must not mutate anchors; an imported project must not mutate source files.
- Persist every staged database/dictionary/save/asset change in the project before deployment. Reopening a project must restore its staged intent, file dependencies and review state; do not rely on live-session memory or ephemeral review handles. Revalidate stale sources/destinations when reconstructing a plan.
- Audit every reachable write route used by the project: generic save, save-as defaults, dictionary writes, image/texture/nameplate tools, uniform/player/equipment import/replacement, archive members and coordinated transactions. All edit outputs go to the project. A hidden direct writer into the installation/save destination defeats this requirement.
- Reading installed assets for previews is allowed; editing requires project-owned output. Copy/materialize only what the native operation needs, lazily for large archives where feasible. Do not duplicate a whole installation by default.
- Move the active project profile/state to the chosen project folder. Opening/discovery must no longer create or overwrite `.nhlmodproj` in the live game database folder. Existing sidecars may be read as legacy configuration; do not delete or migrate them silently.
- A clearly separate, explicit final deployment review maps project outputs to game and save destinations. Show exact changed files and changes, detect externally changed targets, preserve backups and reuse established coordinated transaction/recovery infrastructure. Cancel is non-mutating. A refusal leaves project work intact.
- Destination writes require the final user action; ordinary edit, stage, save-project, reopen or browse must not deploy. Disk-space/copy/write failures must not destroy the baseline or previously saved project state.
- Prevent project storage from aliasing/overlapping the game/save/source roots in a way that makes staging mutate originals; account for Windows case/path aliases and reparse points, not just string inequality. Report an actionable folder-selection conflict rather than silently choosing another destination.

## 3. Extraction is the explicit installation-write exception

The user specifically requests extraction **into the game installation folder**, retaining each container's internal structure. Do not redirect full extraction to the project and call the requirement satisfied.

- Extract only after the user chooses Extract now or the Settings tool and reviews its destination. Original containers remain intact.
- Preserve the established per-format/per-title extraction root mapping. An archive-relative path may need an archive-parent base rather than flattening every member under one root. Show the destination and retain native relative hierarchy; no path traversal outside the selected install or through unexpected reparse targets.
- Before writes, account for existing loose files, member-name collisions across containers, patch/base precedence and differing duplicates. Never overwrite pre-existing edited files merely because extraction was requested. Equal existing bytes may be retained; conflicting bytes need an explicit safe disposition or refusal with project-independent recovery evidence.
- Avoid silent “last archive wins”. A container with unsupported entries does not make the whole install extracted; retain per-source/per-file completion and failure records. Unknown precedence remains blocked, not guessed.
- Stream large PS4 containers, estimate/check free space where possible, report useful progress and support cancellation. Preserve/recover owned partial outputs; do not delete pre-existing files on cancellation or retry.
- Recognize pre-existing valid extraction and support safe rerun after a partial/interrupted run. Revalidate availability when source containers change; “one-time” does not mean permanent unconditional success.
- Detect-only, Skip, opening a project and saving staged edits must not alter installation/save destinations. Full extraction writes are separate from deployment and do not mean project edits have been committed.

The present extractor uses direct `fs::write` into archive-relative outputs and a coarse best-effort summary. Audit/harden those behaviors within this package before exposing them as the requested repeatable extraction workflow. Reuse format readers; this is not authorization to invent unsupported decompression/writer formats.

## 4. Anchor and platform coverage

The user identifies `anchors` as the source repository (552 files currently inventoried by filename count). **File presence, correct identity, vanilla provenance, readable format and usable save support are different claims.** Inventory them separately for all27 targets and required DB/save categories before wiring Start fresh.

Known evidence requiring reconciliation, not silent substitution:

- `docs/audits/anchors-manifest.md` records the NHL20 PS4 main-DB anchor as an NHL19-base alias. Use genuine verified NHL20 data; never bundle the alias as a verified NHL20 baseline. Preserve the historical alias/provenance unless its replacement is explicitly reviewed.
- That manifest records missing PS4 NHL19/NHL21 localization databases. Use established exact-title authorized sources if already available, record their provenance, or keep the dependency explicit; never borrow a neighboring title's dictionary.
- NHL21 anchor save directories currently include BeAGM0, BeAPro0, Profile0, Season0 and ThreeOnThree0; a usable vanilla roster save is not established merely by those names. Inspect actual container capabilities rather than generating or relabeling a roster.
- Recompiled requires its own admitted baseline/mapping and evidence even where bytes derive from Legacy Xbox360. NHL07 PS3 remains research-only.

A gap must appear as a specific unavailable fresh-start component/route, not a fake generated save, a silent downgrade to the installed user's data, or a claim that all27 are ready. Continue independent supported implementation and report exact missing inputs/contracts. File bundling remains internal-only under existing artwork/data decisions; no external release authorization is implied.

## 5. Implementation sequence and acceptance

Implement one owned development lane, with reviewable internal steps; no parallel implementation sessions or messages to other chats authorized.

1. Establish the target/source/destination model and baseline inventory. Reuse existing native sessions/transactions; document the minimal persistence design and changed contracts.
2. Implement project-owned initialization, staged persistence/reopen and routing of writes. This precedes claiming the new first screen is functional.
3. Wire the new launch → unchanged game/platform → three-path flow for all27 targets, including own-file source selection, existing project reopen and r69 selection/error behavior.
4. Implement extraction preflight/choice and Settings tool using one service, with partial-completion capability gates and safe destination handling.
5. Connect final reviewed deployment and recovery of project outputs. Validate project save separately from deployment, and preserve independent native save-category limits.
6. Run meaningful focused tests and assemble a separately numbered internal candidate; freeze after delivery.

Required checks (developer-owned copies/synthetic fixtures only for writes):

- Fresh baselines and imported inputs create isolated projects; untouched source and destination hashes remain unchanged through stage, save, close and reopen.
- Staged DB, dictionary and asset edits survive restart and remain coherent with undo/redo/cancel; no project is marked saved when persistence failed.
- Different projects sharing one game destination cannot overwrite each other's staging or profiles. Old configurations do not silently auto-open/edit live files.
- Explicit final deployment changes only reviewed targets; stale destination, no-space, cancellation and failure exercise accurate refusal/rollback/recovery. Source game/save originals are never the test targets.
- Platform-combination UI/path smoke matrix covers all27 entries. Test actual PS3/Xbox/PS4/Recompiled adapter differences; new all-platform fields must not bypass r69 remembered roster/ambiguity guards.
- Fresh-source inventory includes exact identity and codec capability for each advertised baseline; genuine NHL20 and Recompiled distinctions enforced.
- Skip extraction permits independent database editing; required assets are disabled with actionable accessible reasons; archive-native supported previews remain usable.
- Pre-extracted, partial, unsupported, corrupt, conflicting loose-file and patch/base extraction cases; traversal and reparse containment; progress/cancel/resume/free-space refusal. No blanket completion after partial success.
- Settings extraction after Skip refreshes affected availability without losing staged work or changing project outputs.
- Standalone packaged execution resolves bundled baselines/defs/helpers without checkout dependence.

Do not claim broad parity from a representative title. No autonomous game launch or original install/save extraction/deployment as validation. Preserve proprietary fixtures outside the tracked repository except already-authorized committed anchors. Do not hand-edit generated bindings/capability ledgers.

## 6. Delivery and freeze

Deliver a developer receipt with concrete behavior, source changes, baseline inventory/remaining gaps, tests and skipped cases, extraction/deployment limits, recovery evidence and artifact ownership. Produce **r70 if unused**, otherwise the next unused candidate; use new output paths, exact executable/ZIP hashes, resource/source manifests and package checks. No commits, pushes, external publication or cleanup of shared/unowned resources.

This revision changes onboarding, opening, persistence/write routing and extraction across all27 targets. Requalify those changed workflows and dependent editing/save/deployment checks on the new candidate. Preserve prior accepted NHL21-E1 as historical evidence for its actual build and route; require reruns only where an identified dependency changed, not by blanket erasure.

User-verification next step after review: inspect the new source-choice and project-folder screen, then NHL21 first. The remaining per-game checklist stays active. Do not call the beta complete from this package.
