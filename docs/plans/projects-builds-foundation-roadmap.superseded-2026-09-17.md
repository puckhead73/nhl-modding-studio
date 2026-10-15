# Projects and Builds foundation — initial architecture roadmap

Prepared 17 September 2026. Updated with accepted Project, onboarding-baseline and deployment decisions from the user. Remaining recommendations are identified explicitly.

The user has made this foundation a prerequisite to starting the NHL 21 Legends overhaul. Deliver a complete, narrow workflow first, then extend it across all required content families and targets. Retain the existing Studio completion scope and implementation ownership.

## Existing decisions and implementation

The accepted Q06–Q08b and Q43/Q44 decisions in [completion design questions](completion-design-questions.md) remain binding:

- Standard mode remains direct target-native editing. Projects and Builds belong to opt-in Developer Mode.
- **Develop New Project** is a separate startup entry into that environment, with an empty canonical Project and no initial game selection. Source games, Historical DB and Studio-native data/assets are accessible within it; selecting a source never selects a Build target.
- A Project is target-neutral and may produce outputs for multiple games/platforms. Original canonical data survives target-specific omissions and conversions.
- One authoritative Project is edited; no per-game Project copies, named content variants or branch/merge system are required for this workflow. Destination games and platforms are selected when graduating a committed revision into Builds.
- Project entry/asset count limits use the highest established supported maximum for each type across supported games, independently of selected output targets. Unknown capacities remain unknown; allocated slots or representable IDs alone do not establish playable maxima. Coupled limits and target-specific constraints are checked during graduation. Registry updates never silently delete existing Project content.
- Completing game onboarding captures an immutable vanilla backup, with game/platform/version identity and file hashes. It is the reference for determining target files affected by Project changes. Capturing an arbitrary installation does not prove it vanilla; record provenance and resolve modified/uncertain sources before accepting a vanilla baseline.
- Prioritize installing and switching Builds in one managed game installation, with exact restoration. Separate playable copies are not the first implementation milestone.
- Create blank, import game/database/save, clone, and promote a Standard session with its pending edits preserved.
- Builds use committed, immutable Studio Project revisions. Autosaving working changes is not committing them. Neither operation installs files into a game.
- Target reconciliation exposes losses, omissions, conflicts and invalid values. Corrections affect target mappings, not canonical values; verified defaults only.
- Builds are immutable file sets with exact input/output identities. Installation, annotations and QA records are separate mutable records bound to those identities.
- Database, save and asset changes share review, validation and recoverable transactions.
- Editable Project packages, target-specific change packages and pinned component dependencies remain required.

Inspection of `studio-domain/src/project.rs` shows that the existing `Project` is a loaded target-native database with registries and packaging information. The frontend context likewise tracks an open database/install session. Preserve those working paths while separating their naming and responsibilities from the new persistent Project.

`tdb-canonical` already supplies schema, adapter/profile validation and mapping-override contracts; its documented scope excludes a Project UI or Build engine. Reuse these contracts. The transaction work also exists but its reviewed correction package must be accepted before depending on it for new consumers. This inspection is not a fresh execution of implementation tests.

## Proposed architecture

| Object | Responsibility |
|---|---|
| Project | Stable identity, canonical entities, source assets, formulas, import provenance, mappings and working state |
| Source browser session | Independently opened game database/assets, Historical DB or Studio library; source-scoped metadata and familiar entity views, distinct from the persistent Project destination |
| Import plan | Selected entities, roster scope, asset dependencies, identity conflicts, conversions and provenance; previewed then atomically copied into Project-owned working state |
| Working changes | Durable editable draft, unified undo/review, crash recovery; not a Build source |
| Project revision | Immutable committed state with parent, description, schema version and content identity |
| Source/dependency manifest | Exact baselines, original assets, corpus, adapters and conversion component versions; local paths resolved separately |
| Vanilla baseline | Immutable onboarding backup of the identified game installation, including a file/hash inventory; shared reference for change detection and output materialization |
| Graduation session | Selected committed revision, game/platform checkbox selections, resumable guided review and separate approved output plan/status per target |
| Target configuration | Game/platform, baseline, output scope and deployment requirements; reusable mappings and explicit overrides |
| Reconciliation plan | Concrete output transformations and findings for exact committed inputs; approval invalidated by relevant changes |
| Build job | Cancellable generation into staging, validation, dependency/cache management and atomic publication |
| Build | Immutable manifest and deployable outputs, tied to revision and approved plan |
| Deployment record | Destination, prior state, installed Build, backups, conflicts and recoverable install/restore history |
| Runtime profile | Mutable saves/settings isolated by mod identity, exact game/platform and user/account context; active Build association, compatibility decisions and recoverable snapshots remain separate from immutable Builds and vanilla backups |

Accepted storage behavior: independently configurable paths for vanilla backups, Projects and generated Builds. Imported custom assets are copied into Studio-managed storage, leaving external originals untouched. Builds and revisions depend on the managed copy; external changes require an explicit reimport. Recommended implementation: a versioned local database for structured state/history metadata plus immutable files addressed by content hash for large assets and outputs. Deduplication remains an implementation choice. Keep machine-specific paths out of portable content identity. Use independent copies or verified copy-on-write mechanisms for mutable deployment; writable hard links must never expose immutable originals to edits.

Persist one committed identity for database and asset assignments together. A durable transaction/journal must bridge metadata and filesystem publication so a crash cannot publish a revision or Build with missing files. Derived caches are disposable; source assets, baselines, dependency versions and committed history are not.

## Implementation sequence

| Package | Work | Completion gate |
|---|---|---|
| PB0 — Contracts and integration map | Inventory current edit/save/asset write paths; define object identities, schemas, service APIs and state transitions. Specify single-Project graduation, per-type maximum limits and vanilla-versus-installed comparisons. Define output scope explicitly: roster-only, source-game modifications, or a combination. | Reviewed contracts distinguish source, draft, commit, graduation, Build and installation. Existing Standard-mode entry points have a migration plan. |
| PB1 — Vanilla onboarding and persistent Project store | Capture and verify immutable vanilla backups; create/open/rename/archive/duplicate Projects; schema migrations; stable entity IDs; storage discovery, relocation and missing-input repair; one writer lock per Project. | Interrupted backup cannot become a valid baseline. Create, close, restart and reopen with intact data; second-writer access and failed migration cannot corrupt the Project. |
| PB2 — Canonical editing and history | Implement actual canonical entity instances and relationships. Connect existing roster/entity and asset-assignment editors through services. Durable draft, unified undo/redo, commit review, immutable revisions, comparison and recovery. | One player edit and one asset assignment survive restart and commit together. Earlier revisions remain inspectable/buildable while newer draft edits are preserved. |
| PB3 — Project environment and composable source import | Separate Develop New Project startup; empty Project; source library/onboarding; familiar game-specific entity/asset tabs; Send to Project at league/team/player/asset level; scoped roster/assets prompt. Integrate Historical DB and Studio libraries, plus whole-source import, clone and Standard-session promotion. | Import NHL 21 Blues, close its source, import an NHL 11 team and Historical DB content, then reopen with all selected entities/assets intact. Identity-only import works without phantom roster dependencies. Originals remain untouched and conflicts/unmapped values are visible. |
| PB4 — Guided multi-target graduation | Game/platform checkbox selector and per-target review; lift/lower adapters, deterministic entity/ID mapping, inclusion/exclusion choices, dependency and capacity/companion validation, automatic asset derivation and Invalid Values UI. Bind approvals to revision, mapping, baseline, adapter/component versions and output settings. | A real NHL 21 target previews and lowers supported edits; another target with different limits gets its own review. Choices preserve canonical content. Stale approvals and unresolved blockers stop that target's generation. |
| PB5 — Immutable Build generation | Preflight dependencies and disk space; stage DB/save/asset outputs; preserve unrelated source data; reopen/validate; hash and publish. Progress, cancel, logs and retry. Build list, compare and exact provenance. | A DB-plus-asset Build can be generated from a committed revision, reopened and reproduced. Cancellation or failure leaves no apparently successful partial Build. Rebuilding creates a new identity, with equivalent deterministic payloads for identical pinned inputs. |
| PB6 — Managed installation switching and restore | Install/switch Builds in one managed installation first. Reuse the accepted transaction engine. Compare desired output with vanilla and separately compare actual installation with last recorded deployment. Remove prior-Build additions and restore prior-Build-only replacements. Keep runtime saves/settings distinct. | Install A, switch to B, restore A and restore vanilla with verified hashes and no leftover mod files. Crash recovery works; external changes are detected rather than overwritten. Demonstrate actual NHL 21 loading and save/reload. |
| PB7 — Full content integration and parity | Route all required roster, historical/Legends, schedule, settings, localization, texture, equipment, appearance and audio producers through the same Project/Build contracts as their implementations become ready. | Each family contributes to shared review and reproducible Builds. Prove a second target before calling the architecture multi-target; later qualify every applicable shipping target. |
| PB8 — Portability and product completion | Project and change-package export/import, external-mod conflict review, dependency acquisition, cache/storage management, retention controls, onboarding/tutorial, diagnostics and packaged Windows acceptance. | Move a Project to a fresh installation, resolve declared dependencies and rebuild. Corrupt/incompatible packages fail clearly. Pruning cannot remove dependencies needed by retained history/Builds. Standard mode remains independently usable. |

PB0–PB6 are the foundation gate before substantive Legends production. Their first complete workflow covers a small NHL 21 roster change and one supported asset family; this is an internal milestone, not a declaration that every Studio feature is finished. PB7/PB8 complete the broader feature and remain required. Portable metadata and package/dependency contracts are designed in PB0, even though their complete UI arrives later.

Advance ready persistence and canonical work alongside transaction corrections; installation acceptance depends on the corrected write engine. Keep the current single Studio implementation owner. No new implementation dispatch is implied by this plan.

## Integration rules

- Share domain operations and validation across Standard and Developer Mode, with explicit session context. A Project edit must never fall through to a direct live-file save handler.
- In Developer Mode, historical imports, formulas and asset tools edit canonical Project state. Target encoding belongs to Build generation. Pin corpus/formula/library inputs needed to reproduce imported results.
- Preserve unknown imported data for round trips to its source target; do not transplant opaque native bytes to another target. Show unsupported conversion as a finding.
- Blank Projects can be edited without a game baseline. Generation requests the verified baseline/container dependencies the chosen target needs; it cannot manufacture unknown required game data.
- Missing adapters or unfinished audio/localization tools block affected output, not Project persistence or unrelated supported workflows. No success state may silently omit requested changes.
- Select any committed revision for generation while protecting newer drafts. Build jobs consume immutable snapshots, so subsequent editing cannot change a running job's inputs.
- Qualify deterministic payload generation separately from timestamps/job IDs and platform-specific signing. Record any external packaging step; inner-file success alone is not original-console deployment support.
- Store deployment and QA records outside immutable Build payloads. Detect tampering before installation. Observe filesystem containment, junctions, path aliases, file locks and cross-volume failure behavior.
- Recommend autosaved drafts with explicit named commits and single-writer Project access initially. Full collaborative editing or merge semantics needs a separate decision rather than accidental support through shared folders.

## First end-to-end acceptance scenario

1. Enter Develop New Project without choosing a game. Open NHL 21 as a source and send the St. Louis Blues with their roster and selected corresponding assets into the empty Project. Close that source; open NHL 11 and import another team, then add Historical DB content. Retain source hashes and provenance independently.
2. Verify the identity/assets-only import path separately. Edit one imported player and assign one replacement portrait or logo through Studio; the Project survives source closure and assets no longer depend on the original live installation.
3. Close and reopen; recover the draft; inspect the unified changes and undo/redo.
4. Commit revision A. Make another draft edit, then build A without losing that draft.
5. Select NHL 21 and a second game/platform via graduation checkboxes. Review each separately, including an over-capacity selection with dependent assets/rosters; resolve an invalid-value case with a target-only override. Prove exclusions preserve Project content and relevant input changes invalidate approval.
6. Generate and validate Build A; retry from the same pinned inputs and compare payloads.
7. Deploy to the managed installation, confirm the changes in game and save/reload.
8. Commit/build B; switch builds, restore A, then restore the original target. Separately test interruption, locked files, changed baselines and insufficient disk space.
9. Reconcile the same canonical sample to a second game/platform with a meaningful schema or asset difference. Confirm canonical data remains unchanged and output differences are reported.
10. Reopen in Standard mode and verify the ordinary open/edit/save workflow still works without Project setup.

## Decisions recorded 17 September 2026

| ID | Topic | Accepted direction |
|---|---|---|
| PB-D1 | Project and target selection | One game-independent Project. Maximum counts per entry/asset type come from the maximum supported across games. Select game/platform destinations at graduation, then guide the user through inclusion/exclusion and compatibility decisions for each. No named content variants or branches are requested. |
| PB-D2 | Vanilla baseline | Game onboarding captures an immutable vanilla backup used to identify the files Project changes affect. Placement and asset ownership are settled in PB-D5/PB-D6. |
| PB-D3 | First deployment | Managed installation switching and exact restoration first (option B). |
| PB-D4 | Saves and settings | Separate profiles per mod, with explicit reuse between compatible Build revisions (option A). Qualify isolation per game/platform and expose required manual steps where automatic isolation is unavailable. |
| PB-D5 | Storage paths | Independently configurable paths for vanilla backups, Projects and generated Builds. |
| PB-D6 | Custom asset ownership | Copy imported assets into Studio-managed storage; leave originals untouched. External changes are adopted only through explicit reimport. |
| PB-D7 | Partial graduation success | Finish successful targets and provide a report identifying blocked/failed targets and why. Preserve review progress and permit targeted retries. |
| PB-D8 | Separate Project environment and source composition | Develop New Project opens an empty game-independent workspace. Browse/open onboarded games or onboard another in place; access their databases/assets alongside Historical DB and Studio-native libraries. Send leagues, teams, players and assets to the Project with reviewed import scope. Persist copied data/assets independently of open source sessions. |
| PB-D9 | Canonical storage and output IDs | Use SQLite `project.db` and unique Studio-specific entity/asset identifiers independent of all source IDs. Resolve final native IDs/slots during Build execution separately for each target; the same Project entity may occupy different slots in different games. |

Explicit duplication remains the previously accepted way to start an independent Project with ancestry; it is not how target outputs or alternative platform constraints are represented. Internal immutable history likewise does not create multiple active content variants.

## PB0 contract: graduation and change tracking

1. Edit and commit the single canonical Project. Capacity guidance comes from a versioned per-type registry of supported maxima, not the currently opened or selected game. When no maximum is established, show that uncertainty rather than inventing a ceiling or declaring unlimited runtime support.
2. Start graduation from one committed revision. Show actual supported game/platform pairs with checkboxes and readiness indicators. Missing baselines, adapters or dependencies identify required setup; they never silently remove a chosen target.
3. For each target, show content counts against limits, attribute/asset compatibility, included and excluded entities, and dependent resources. The developer chooses what fits. Excluding a team/player/asset triggers dependency validation; Studio cannot leave dangling roster, lineup, schedule or asset references. Any automatic proposal requires review.
4. Resolve target-only overrides and conversions, show omissions and final file impacts, then approve the exact plan. Retain these choices for reuse while inputs remain valid. They are output configurations, not copies of the Project.
5. Generate a separate immutable Build for each approved target. A graduation session groups their results but does not pretend they share one binary output. Accepted failure handling: finish and preserve successful Builds, report failed/blocked targets individually with reasons, and allow targeted retries without marking the whole selection complete. Each successful target still requires its own approved reconciliation and validation; partial success does not bypass target gates or trigger automatic installation.
6. Determine target file effects by materializing the approved output against that target's immutable vanilla baseline. Show canonical changes in Project review and exact target file changes in graduation; a target-neutral edit may map to different files in different games. Include added, replaced and removed files plus archive-member detail where known.
7. Install by comparing actual files with the recorded current installation state, not just vanilla. Switching A to B restores files changed only by A and removes files added only by A, while applying B. Recoverable pre-install backups are separate from the never-modified vanilla backup.
8. Game updates establish a new versioned baseline after verification; they never rewrite the baseline pinned by existing Builds. Old Builds remain bound to their original requirements. Missing source versions must be reported explicitly.

## PB1 developer package: persistent sources before content production

**Inputs:** accepted decisions above, existing install discovery, canonical schema contracts and the reviewed transaction interface. No new binary-format guesses are needed for byte-preserving backup capture.

**Deliverables:** a versioned baseline manifest and backup store; staging/verification/atomic publication of onboarding backups; a persistent Project manifest/store with stable IDs and source provenance; create/open/rename/archive/duplicate services; reopenable working state and single-writer locking; UI that distinguishes backup progress, a valid baseline and a Project. Successful onboarding must not be reported before required backup verification completes.

**Backup rules:** verify copied file hashes and detect source changes during capture; preserve installation-relative paths; reject unsafe path escapes; never expose originals through mutable hard links. Insufficient space, cancellation or a crash leaves an incomplete staging record, not a usable vanilla baseline. A modded or uncertain source is reported as such and cannot silently establish vanilla truth. Capture installed game source files; personal saves and mutable runtime settings have a separate lifecycle decision below.

**Acceptance:** capture/reopen a baseline with matching hashes; demonstrate source preservation, interrupted capture/retry and corruption detection; create/edit/reopen a Project without target selection; duplicate with distinct identity and ancestry; relocate/relink storage by verified identity; reject a second writer; demonstrate failed migration recovery. Test two identified game/platform baselines without merging their provenance. These tests qualify persistence, not game compatibility.

## PB-D5–PB-D7 implementation requirements

**Storage (PB1):** expose separate selectors for backup, Project and Build roots. Validate permissions, available space and unsafe overlap with live installations; show the resolved locations before acquisition/generation. Persist object locations independently of current default paths, so changing a default does not strand existing objects or silently move them. Explicit relocation copies, verifies hashes and switches references recoverably across volumes; interruption preserves the existing valid storage. Unavailable drives produce actionable missing-storage states without clearing history.

**Asset import (PB1–PB3):** copy to staging, verify integrity, then publish the managed source and assignment together. Preserve original path/hash as provenance, not as a live dependency. Never modify, move or delete the external original. Editing or deleting that original after import must not affect existing revisions or Builds. Reimport produces a new managed content version and a reviewable Project change; retained revisions continue to reference the old version. Interrupted imports leave no committed reference to an incomplete asset.

**Graduation results (PB4–PB5):** show each selected game/platform as succeeded, blocked, failed, canceled or pending, with successful Build identities and links. Reports identify the affected content or dependency, the constraint/error, and the next resolving action where known. Distinguish a compatibility blocker from an execution failure; include both the summary and inspectable details. Retain review choices for blocked targets, revalidate approvals after relevant changes, and retry only requested targets. Successful outputs are not rebuilt or removed because a different target failed. A partially completed batch must not display an all-target success state.

**Acceptance additions:** configure three different roots and exercise an unavailable drive and interrupted cross-volume relocation; import an asset, edit/delete its external original, and reproduce the retained revision unchanged; explicitly reimport an edited asset and compare revisions; graduate at least one successful and one blocked target, verify the blocker report, resolve/retry that target, and prove the earlier successful Build remains immutable. Inject a target generation failure to verify the same isolation and reporting behavior.

## PB-D4 contract: saves and settings during Build switching

Accepted by the user: separate save/settings profiles per mod, with explicit reuse between compatible Build revisions. Profiles are runtime state, not Project content variants. Scope them to the mod/Project identity, exact game/platform and relevant user/account; never share saves across platforms merely because they originate from one Project.

- Preserve the pre-mod save/settings state in its own profile. Personal progress is not part of the immutable vanilla game backup and must never be overwritten by restoring vanilla source files.
- Associate each active profile with its Build identity and record any approved reuse. A shared mod name or matching save-container format does not prove revision compatibility; relevant roster IDs, mode structure and other established constraints must be checked.
- Before switching, capture current profile changes and retain a recoverable snapshot. Reuse for a compatible revision requires explicit selection. If compatibility is unknown or incompatible, preserve the existing profile and offer a fresh one; do not silently migrate saves. Any future migration needs its own verified operation.
- Coordinate Build deployment and profile activation under one recoverable operation. Failure restores the previous Build/profile pairing or leaves an explicit recovery state. Block switching while affected files are in active use. Do not report success with the new game files and the wrong profile active.
- Qualify profile discovery, capture and activation for each game/platform and deployment path. Where console/account restrictions require manual transfer or selection, present those steps and retain an awaiting-action state until completion is verified. Do not claim automatic isolation from a folder copy alone.
- PB1 defines profile identity and metadata boundaries; PB6 implements and qualifies activation, switching and recovery alongside managed installation. This remains required foundation work.

**PB6 acceptance additions:** play/save under mod A; switch to mod B and verify separation; return to A with progress/settings intact; restore vanilla and its prior profile; explicitly reuse a compatible revision after snapshotting; reject silent reuse for unknown/incompatible revisions; interrupt each transition and recover the previous pairing; verify user/account and target separation. Test documented manual activation where automation is unavailable.

## PB-D8 contract: build a Project from multiple sources

### Entry and navigation

Startup offers the existing game-editing path and **Develop New Project**, with an Open Project path for returning work. The latter creates a genuinely empty canonical database; Studio content is available to browse/import but is not silently prepopulated. Developer Mode remains the umbrella terminology; Develop New Project is the creation action.

The Project stays open while source tabs are opened, closed or switched. Recommended shell: persistent Project navigation, a Sources/library browser and clearly labeled source tabs. Show source game/platform/version in each tab and a distinct Project destination. Source-scoped schemas, capabilities and asset roots must not leak between tabs or override canonical Project controls.

Sources include onboarded installed games, the Historical DB, Studio-native database content and asset libraries. Provide **Add Game to Studio** within the environment, running normal discovery and immutable vanilla capture while preserving the Project draft. Browsing a game source uses the existing league/team/player and asset views, interpreted through that game's own schema. View/export source assets using supported tooling; import them into the Project through explicit selection. Closing a source releases its session, not imported content.

Recommended source behavior is read-only browsing with Send to Project actions; canonical edits happen in the Project. If source editing is later exposed here, it must be a distinctly labeled native session and must not write through to the immutable baseline. Game sourcing does not constrain output targets. Access to a source format does not imply every asset conversion to every destination is already supported.

### Send to Project

For the user's St. Louis Blues example, the team action opens an import prompt with these explicit scopes:

- **Team identity and assets:** team metadata and selected linked team assets; no players or roster memberships imported.
- **Team, rostered players and assets:** team metadata, rostered player records and required companions/relationships, plus selected team/player assets.

The preview names the team, players, roster membership count and asset families, with missing/unsupported items visible. It allows inspection of exactly what will be imported; do not equate “assets” to every file in the game's archives. Resolve portraits, logos, uniforms, equipment, heads and other pertinent references through verified discovery. Unavailable families remain explicit findings. Do not silently substitute assets from another game or drop unresolved content.

League import similarly previews included teams and whether their rosters/assets are included; player import previews biography, supported attributes and related assets. Individual assets may also be imported/exported/viewed without importing a whole team. Preserve relationships inside the selection; references to unselected entities must be disclosed and resolved or retained as explicitly unresolved draft references, never as dangling internal IDs.

An identity-only team is valid unfinished authoring content. Project persistence and commits must permit well-formed incomplete authoring state with findings; build-readiness validation later requires the relationships and resources necessary for the selected output. Structural corruption is not the same as an intentionally unfilled roster.

### Database, asset ownership and identity

Accepted implementation: a versioned SQLite `project.db` stores canonical Project data, with structured entities, relationships, import batches and source provenance, plus Project-owned immutable asset objects referenced by hash. Large asset payloads may live beside the database in managed storage; the database records their identities and assignments. Native game databases are produced at graduation, so NHL 21 field limits and IDs never become the Project's schema by accident.

Each imported or newly created entity/asset receives a stable, unique Studio-specific ID. Canonical relationships use these IDs, never a source game's numeric IDs. Retain original game/platform/database hash, entity type and source entity ID as provenance; never use a bare game numeric ID as cross-game identity. A player or team from NHL 11 with the same numeric ID as one from NHL 21 must not overwrite it. Detect likely real-world identity matches using existing verified matching logic and offer an explicit reuse, distinct historical version or merge review as applicable. No automatic destructive overwrite. Distinct entity/season versions are content within one Project, not Project branches. Asset content hashes identify payloads for integrity/deduplication; they do not replace logical asset IDs.

Import normalizes known values through source adapters and preserves unmapped native details with provenance. Historical imports retain source season/corpus/formula identity. Studio library imports pin their component versions. An imported native asset can remain preserved as source material while a missing canonical conversion is reported; conversion qualification is separate from successful byte-preserving acquisition.

Copy the selected asset payloads and data required by the import into Studio-managed ownership before reporting completion. Preserve source format and high-fidelity originals, with target derivatives generated later. External source files remain unchanged. Deduplication may share immutable objects but retained imports must keep those objects alive after source sessions close or game installations become unavailable. No imported record may depend solely on a temporary extraction directory.

Prepare the complete import before publishing entity changes and asset assignments together. Cancellation/failure leaves the prior Project intact. Successful imports persist as recoverable working changes immediately; they do not require an open source tab, and they do not automatically create a committed Build-source revision. Explicit Project commits remain the Build gate. Undo/review should identify the import batch and affected dependencies.

### PB3 acceptance gate

1. Start a blank Project without a selected game or populated roster. Browse installed sources, Historical DB and Studio libraries.
2. Open NHL 21 and navigate to the Blues using familiar source-native views. Import the team with its roster and selected team/player assets; inspect counts, provenance and assignments.
3. Close NHL 21, open NHL 11 and import another team and players/assets. Verify the first import survives and colliding native IDs remain independent.
4. Import Historical DB content and a Studio-native asset, with source versions and ratings provenance retained.
5. Test identity/assets-only import, league import and single-player import. Verify roster-dependent references are handled correctly and missing assets remain visible.
6. Restart Studio with source installations unavailable; inspect/edit the imported Project content and managed assets. Missing future target baselines can block generation but must not erase imported authoring content.
7. Onboard another game from inside the Project environment without losing pending work. Export a source asset without changing either source or Project; import only through explicit selection.
8. Interrupt/cancel an import and undo a successful batch; prove coherent entity/asset state and untouched originals. Repeat an import to exercise duplicate/conflict review rather than silent duplication or overwrite.

This mixed-source authoring gate is required before Legends production. It does not wait until PB7; broad conversion and per-target game qualification continue there. PB1/PB2 establish storage and canonical editing, then PB3 delivers the user-facing Project environment and import workflow before graduation implementation.

## PB-D9 contract: native ID allocation at Build execution

Project IDs are persistent authoring identities; native IDs are output allocations. Never write a generated game ID back over a Studio ID. Source native IDs are provenance, not reserved output slots.

PB4 previews allocation feasibility, capacity, reserved slots and any semantic replacement choices requiring user review. PB5 resolves and freezes the final allocation during Build execution against the exact approved revision, target baseline, included content, mappings and adapter version. Final allocation must conform to the reviewed plan. If execution discovers a conflict or would change a reviewed replacement/omission, block that target and return it for review rather than silently changing intent.

Allocate deterministically within each target's actual ID namespaces, ranges, reserved/sentinel values and coupled constraints. Check slots occupied by retained vanilla content as well as new content; replacement requires the reviewed mapping. The same entity can therefore map to one slot in NHL 21 and another in NHL 11. A single entity may require multiple native records/IDs, and an asset may have distinct catalog, render, archive-path or slot identifiers; retain all qualified mappings rather than assuming one universal ID column.

Resolve the complete mapping before serializing dependent outputs. Rewrite database foreign keys, roster/lineup references, asset assignments and any supported ID-derived names/paths using that mapping. Unknown required reference semantics block the affected output; never guess or carry an unrelated source ID through. Validate referential integrity and native uniqueness after reopening generated files.

Store the allocation map with the immutable Build manifest: Studio ID, target namespace/role, native ID or slot, baseline/replacement identity where relevant and allocator/adapter version. Pin all allocation inputs so reproducing a Build produces the same mappings. Different revisions may allocate differently; save-profile compatibility must inspect mapping changes and cannot infer compatibility from a shared Project identity.

**PB1/PB2 acceptance:** SQLite uniqueness and foreign-key integrity for Studio IDs; restart-stable identities; source-ID collisions cannot overwrite records; explicit identity reuse/merge retains provenance. **PB4/PB5 acceptance:** generate two target outputs from the same Studio entities with deliberately different available native slots; verify all companion/roster/asset references, deterministic reproduction, reserved/occupied-slot rejection, exhaustion reporting and an unchanged canonical database. Review allocation changes when assessing reuse of saves between Build revisions.
