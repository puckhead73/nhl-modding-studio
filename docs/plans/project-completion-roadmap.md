# Project completion roadmap

> **Scope and sequence supersession — 17 September 2026:** [Entity editing and cross-game parity](entity-editing-parity-roadmap.md) is the current execution roadmap: leagues → teams → players → equipment → miscellaneous presentation, one step replicated across applicable targets. Projects/Builds and their canonical editor, history, graduation, packaging and Build/profile management requirements are now final optional stretch work, not prerequisites or release gates. C0/C8 clauses below requiring those features are retained historical design and apply only if that stretch work resumes. Shared direct-editing validation, backup/recovery, save packaging and required tools remain active. Arena details/ice textures and the other presentation operations explicitly named in the new roadmap are now active scope; their former blanket arena-art stretch exclusion is superseded. Other compatible requirements below remain in force.

Prepared 2026-09-05 against the current working tree. Planning document; no implementation or QA
promotion is implied. This is the completion sequence; [full-game QA](full-game-qa-editability.md)
defines the detailed verification procedures. [ROADMAP.md](../ROADMAP.md) retains the original phase
history. [Design questions](completion-design-questions.md) contains the decisions to resolve in order.

Updated 2026-09-07 with the NHL 19 BIG Studio tester review and required Ignite archive-backed
texture integration (C3a/C4). This update specifies implementation work; it does not promote archive
writes to game-verified support.

## 1. Finish line and scope

**Scheduling update 2026-09-16:** the user's three-stream ownership and current execution queue
are recorded in [parallel completion plan](parallel-completion-plan-2026-09-16.md). Studio has one
code writer; universal head conversion has its own repository/developer; user-assisted game research
produces Markdown outside both implementation trees. That plan supersedes the older discovery-first
ordering below. C0–C10 feature scope and acceptance requirements remain binding.

Complete the NHL Modding Studio product described in DESIGN.md: database and roster editing,
historical and Legends roster construction, supported save packaging, production asset workflows,
onboarding, project/build management, and distributable desktop releases.

Target the 17 catalog entries spanning NHL 07–21, Legacy, and Legacy Recompiled on the actual
catalog platform combinations. This is not a Cartesian promise of every title on every console.
NHL 07 PS3 is a research copy, not a shipping target. The 27 main-database anchors include that
research copy; Recompiled needs independent runtime/save evidence despite sharing Legacy's schema.
NHL24 is currently a headmesh source, not a new full-editor target. Do not imply Xbox One support
from the old README's wording.

“Fully complete” means all agreed applicable feature/target cells pass their completion gates,
including game acceptance for writes. Missing fixtures, blocked implementations, and experimental
features are unfinished work. **Release policy (Q02): nothing releases until the entire agreed
project is complete.** Verified subsets are internal development/QA milestones only, not incremental
releases. A scope exclusion requires an explicit recorded decision;
technical difficulty alone cannot turn a required feature into `not-applicable`.

**Required save scope (Q03):** normal roster files, Franchise, Season, and settings. Map older
Dynasty/Be A GM naming to Franchise coverage through per-title inventory; verify each actual mode
and layout rather than assuming equivalence. Required modes gate full completion wherever present.
Be A Pro and other game modes are stretch goals and do not gate release. Standalone profile editing
is not a primary goal; any profile/package handling necessary for a required workflow remains part
of that workflow. **Settings scope (Q03b): expose every setting whose identity and encoding can be
safely established.** Build a per-title/platform settings catalog from verified evidence rather than
a fixed category list. Identified settings receive friendly names, semantic controls, valid ranges or
enums, defaults where proven, and read/write qualification. Unknown or ambiguous values may appear
only in a clearly marked technical/raw view and remain read-only; guessed labels do not satisfy scope.
Settings support must cover non-TDB payloads where required.

**Deployment scope (Q04): original-console packaging is required**, alongside emulator/decrypted
workflows. Studio must produce or complete the full supported packaging/repackaging process needed to
deploy edited roster, Franchise/Season, settings and source files to Xbox 360, PS3 and PS4 targets.
This includes outer-container metadata and integrity where applicable. Document any unavoidable
console-side transfer/signing step, but do not treat an edited inner payload alone as completed console
support. Legacy Recompiled uses its verified native filesystem/deployment path.

**Schedule editing (Q03a, partial):** required feature. The preferred workflow edits the source
schedule used when creating a new Franchise/Dynasty or Season, so the edited schedule becomes the
default for new games. Locate and verify that source and initialization behavior per title/platform;
do not assume schedules live in the main database or a particular archive. In-progress save schedule
editing is not required. **Q03a.2 decision:** no further Franchise/Season-specific editing is required
at this time. Existing player, team and roster tools plus the source schedule workflows define the
release scope; standings, accumulated season records, finances and other live mode state are excluded.

Bound otherwise open-ended work before estimating: complete every canonical-head target adapter, arena
asset operations, supported save modes, bulk tools, and desktop operating systems. Proposed defaults
are in the question brief and are not accepted scope changes. Existing deferred research remains
separate unless explicitly brought in: recomp development, NHL2K10 RE, exclusive-path head-import
research, standalone rig-audit/weight-remap projects, FMT plugins, EBOOT editing, mesh authoring, and
audio waveform authoring. Inspect the existing CC5 path as current work, without silently expanding
the product to every arbitrary authored mesh.

**Desktop platform scope (Q05): Windows is the required initial release platform.** Linux and macOS
are stretch goals and do not block full completion of the initial release. All bundled Blender/audio
runtime, filesystem, installer, upgrade, signing, recovery and packaged-app acceptance requirements
therefore target supported Windows versions. Keep platform seams portable and avoid needless Windows
coupling, but do not count untested cross-platform builds as supported.

**Unified change model (Q06): database, save and asset edits share one pending-changes review and
apply as one coordinated project transaction.** The review resolves every target to friendly entities
and files, shows validation findings and dependency/order information, and permits reverting an item
only when the resulting change set remains valid. Apply writes all outputs to validated temporary
artifacts first, checks source hashes and available space, then installs with a transaction journal
and recoverable backups. A failure must restore the complete pre-apply project state or leave an
explicit resumable recovery record; mixed old/new DB, save, reference and asset state is never success.

**Projects and Builds (Q07) are separate concepts.** A Project is the overarching editable workspace:
source/baseline identities, staged and committed working changes, formulas, import history, resolved
mappings, library assignments, sidecars, validation evidence, transaction history and the information
needed to reproduce outputs. A Build is a named set of actual deployable game files generated from a
specific Project state for a specific target/deployment path. Each Build carries a manifest tying its
files and hashes to the Project revision, target, generation inputs and QA state. Project management
covers create/open/rename/duplicate/archive, history and recovery; Build management covers generate,
validate, activate/install, export/package, compare and restore. Remaining questions will define target
cardinality, Build versioning and pending-edit behavior.

**Multi-target Projects (Q07a):** one Project may produce Builds for different game/platform targets.
Every Build remains target-specific. Before generating a target Build, Studio probes the destination
schema and runtime capabilities, lowers the Project state into a reconciliation plan, and requires
explicit user review/approval of every material mismatch: league/team counts and identities, roster
capacity, attributes and bio fields, enums/scales, relationships, save/container support and asset
availability/conversion. The preview lists exact mappings, transformations, omissions, conflicts,
fallbacks and blockers using friendly names, with raw identifiers in details. No lossy omission,
automatic remap or target-specific default is silently accepted. Approved reconciliation is versioned
with the source Project revision and target, can be reused only while its inputs remain unchanged, and
is invalidated by relevant Project, schema, formula, mapping or capability changes. Build generation
must be deterministic from that recorded plan.

**Canonical Project model (Q07a.1):** the Project is the target-neutral source of truth. Users build
their intended roster, league content, schedules, settings references and assets in this canonical
model; no game/platform database is the master representation. Canonical values preserve the richest
known semantics and source assets so exporting to a poorer target never erases Project data. Each
game/platform has a versioned, evidence-backed output adapter. Export automatically maps and encodes
supported values, converts canonical source assets into the target's required formats/layouts, and
omits fields that do not exist for that target. Every omission and lossy conversion is
listed in the Q07a reconciliation report and requires the already-agreed user approval before Build
generation. A value for a field that does exist but violates a proven target constraint must be
resolved through the Invalid Values workflow below or block export; it is not silently discarded as
though the field were absent.

**Invalid Values workflow (Q07a.3):** target reconciliation includes a dedicated tab listing every
canonical value that maps to a present target field but violates its proven type, range, enum,
relationship, capacity or other constraint. Each row shows the friendly entity/field, canonical value,
target constraint, reason, proposed output value and raw/schema details behind disclosure. The user
can edit resolutions inline or run an automated **Clear to Defaults** action for selected rows or all
eligible rows. Automation replaces values only with evidence-backed target defaults defined in the
schema adapter, previews the full delta, participates in undo/redo, and still requires reconciliation
approval. If no safe default is known, the row remains unresolved and blocks Build generation. Target-
absent fields are reported as omissions in a separate category; they are not mislabeled as invalid.
**Q07a.3a decision: every correction applies only to that target's Build mapping.** The canonical
Project value is never changed by reconciliation. Manual edits and Clear to Defaults create explicit
target overrides stored with the target mapping, including provenance, prior value, resolution method
and adapter/schema version. Overrides may be reused for later Builds of the same target while their
inputs remain valid, and are revalidated or invalidated when canonical data, target schemas or adapter
rules change. Editing canonical data remains a separate deliberate action in the Project editor.

The adapter system requires a comprehensive schema registry for every target. Database entries record
table and field identity, semantic canonical property, type, endianness, bit/byte offset and width,
signedness, range, enum vocabulary and stored code, scale/offset/index origin, null sentinel, string
encoding/pool behavior, update/key/structural status, foreign-key relationships, companion/cardinality
rules, capacity and presence/applicability. Every read transform must have a verified inverse write
transform. Asset profiles record discovery and identity rules, archives/indexes, paths/slugs/IDs,
raster formats, dimensions, channels, mipmaps, compression, swizzle/tiling, atlases/pools, mesh vertex
layouts, UV conventions, rigs/bones/weights/palettes/LODs, audio banks/codecs/sample constraints, and
repack/install rules. Facts carry fixture provenance, confidence and automated/manual qualification.

Canonical asset sources stay non-destructive and high fidelity. Target adapters derive portraits,
logos, uniform/equipment textures, heads/meshes and audio outputs automatically, including resizing,
channel transforms, mip generation, compression, tiling/swizzling, atlas placement, mesh conversion,
bank encoding and archive rebuild as applicable. Generated derivatives are cached by source hash,
adapter version and settings, but remain reproducible outputs rather than canonical Project data.

**Product modes:** Projects/Builds and the canonical multi-target workflow are an opt-in
**Developer Mode**. The default Standard mode preserves the current direct workflow: open a
normal database/save, edit the roster and other supported target-native data, review changes, and
save/save-as/repack without first creating a canonical Project or understanding target adapters.
Standard mode uses the same verified parsing, validation, conversion and transactional foundations,
but hides project history, cross-target reconciliation, adapter/schema diagnostics and Build
orchestration. Its UI must not become a disguised setup wizard for the advanced system.

Developer Mode has a dedicated first-entry tutorial and persistent help that explains canonical
Project data, source inputs, target schemas, reconciliation and omissions, automatic asset conversion,
Build generation, deployment, history and recovery. The tutorial uses a safe sample or dry run,
includes an end-to-end Project -> reconciliation -> Build example, can be paused/restarted/revisited,
and requires no prior knowledge of binary formats. Developer controls remain progressively disclosed;
raw schema IDs and offsets are diagnostic details rather than primary labels. **Q08b decision: the
user-facing name is Developer Mode.**

**Standard-to-Project promotion (Q08a):** a user can promote the current Standard session into a new
canonical Project without losing staged edits. Studio snapshots the untouched source and current
staged state, imports both target-native database/save data and associated assets into canonical
properties, and carries the edit history/provenance forward. Before creation, a preview identifies
confident mappings, target-specific values retained as provenance/extensions, ambiguous mappings,
unsupported data and missing assets. Promotion never modifies the original source or implicitly
applies pending edits to it. The resulting Project records its source target/schema hashes and can
immediately regenerate an equivalent Build for that target, subject to byte-versus-semantic fidelity
rules and the normal reconciliation approval. Promotion is transactional and cancelable before commit.

**Project creation (Q07a.2) supports three entry points:** blank creation, import from an existing
game/database/save, and cloning another Project. Blank creation selects an initial canonical league
and roster structure without binding the Project to an output target; templates are canonical inputs,
not disguised game databases. Import uses verified source adapters to lift semantic data and assets,
preserves source hashes/provenance and target-native extensions, and reports anything ambiguous or
unmapped. Clone creates an independent Project identity while retaining an explicit ancestry link and
the chosen canonical state, mappings, assets and formulas; generated Builds and deployment state are
not mistaken for editable source. All paths end at the same validated canonical model and can target
multiple games through the same reconciliation/export pipeline.

## 2. Evidence baseline

Sources: STATE-OF-THE-REPO.md; DESIGN.md; EDIT-MODEL.md; the full-game QA and Phase 6 plans;
qa-ledger.md; save-container-audit.md; current routes; headmesh-matrix.toml; git status.
This was a planning inspection, not a fresh execution of all tests or a full implementation audit.

| Workstream | Baseline | Completion work |
|---|---|---|
| TDB engine, staged edits, validation, save spine | Implemented; automated evidence exists | Regression baseline, known defects, full semantic and game acceptance coverage |
| Database UI and roster operations | Implemented through original Phase 4 | Remaining parity features, field dispositions, UI and game verification |
| Historical import/formulas | Implemented | Cross-target correctness, corpus distribution, data-quality and import QA |
| Legends | Engine/API/UI work exists, substantially uncommitted | Product rules, deterministic selection, checkout integration and game QA |
| Save containers | Raw and several inner-container round trips proven | v1 integrity, NHL15 PS4, PS4 topology, STFS, console packaging, multipart implementation |
| Portraits/logos/textures | Decode/export and selected replace paths implemented | Import matching, conversion, icon generation, archive install qualification |
| Uniforms/nameplates | Full-kit preview and editing implemented | Persisted output/install, curved text decision, per-target visual verification |
| Hockey Shop | Browse/preview/store/clone/install implemented | Slot conversion recipes, collision/recovery and in-game coverage |
| Appearance/headmesh | Catalog/preview and conversion groundwork | Enum writes, CAP library, Blender delivery, pair-specific installation and validation |
| Texture Lab | Placeholder route over existing codecs | Complete workflow and UI |
| Ignite `.big` texture editing | Tester implements direct EB entry writes and append relocation; Studio has EB readers and experimental writers | C3a: archive-backed texture identity, streaming writes, codec/metadata qualification and full transactional restore |
| Localization | Discovery only | String editing and safe type-66 pool handling |
| Audio | Unimplemented | Format discovery, codecs, bank editing, preview/export/replace/install |
| Arena assets | Present in the legacy QA scope | Stretch: integrate a community arena editor later; exclude arena art/presentation editing from the initial release gate |
| Shell/builds/settings | Partial | Onboarding, snapshots, jobs, bulk tools, exports, recovery and release packaging |

Ledger baseline: 392 rows: 203 missing-fixture, 80 edit-roundtrip-pass, 53 discovered, 27 read-pass,
19 not-applicable, 10 blocked; zero game-verified. These are feature-family rows, not 392 individual
tests or 80 fully supported game targets. Asset fixtures outside anchors must be reconciled with
the ledger before interpreting its missing-fixture count as a complete inventory of local evidence.
Headmesh definition notes include limited manual observations (including CC5 UV mapping); these are
leads for evidence collection, not full pair sign-off.

**Tester review evidence (2026-09-07):** inspected [README.txt](../../Nhl19BigStudio-Tester/README.txt)
and statically decompiled the managed assemblies embedded in `Nhl19BigStudio.Gui.exe` using ILSpy.
Executable SHA-256: `0108399FAF416B6182DABEECDB46EC3FD596E94AC1A8B1E2292D9C763604717E`.
The supplied directory contains a self-contained .NET bundle, not the original C# project. Findings
below are implementation evidence from `Nhl19BigStudio.Core`, `Nhl19BigStudio.Gui` and
`PS4GnfStudio.Core`; no tester write or game acceptance run was performed in this review.

| Reviewed capability | Observed implementation and limits |
|---|---|
| Archive discovery/read | README and GUI support one `.big` or a folder of archives, folder/category navigation, search, entry preview and extraction. `EbArchive.Open` reads EB headers and name/folder/file tables, then seeks payloads on demand. This is EB support, not proof of every archive carrying a `.big` extension. |
| EB layout | `EbArchiveHeader` derives 16/64-byte blocks from flag `0x200` and 16/20-byte table rows from `0x10000`; offsets use block index times block size. `EbFileEntry.DeclaredSpan` selects row `+4` when nonzero, otherwise `+8`. These field interpretations need fixture qualification; the class names alone do not prove size-field semantics. |
| Chunked read/write | `EbChunkedAsset.Decode` handles compact and extended chunk headers, Deflate chunks and extended type-4 stored chunks. Signature detection only checks the `chun` prefix; this does not establish LZX decoding. `EbChunkedAssetEncoder.Encode` emits one Deflate chunk in a 48-byte `chunkzip` wrapper (version 2). The writer converts any detected chunk wrapper to this representation, rather than preserving every source codec. |
| Texture preview/export/import | Noesis supplies GNF preview/decoded DDS; orbis-image2gnf supplies metadata and image-to-GNF conversion using the first original texture's pixel-format string. Already encoded GNF bypasses conversion. `GnfDdsExporter` attempts format-preserving recompression (BC1/2/3/7 UNORM/sRGB and BC4/5 UNORM/SNORM), with a reported decoded DDS fallback. Export regenerates mips from decoded pixels; it is not original compressed-block extraction. |
| Fit-in-slot write | `EbInPlaceWriter.Replace` encodes first, compares stored length to `DeclaredSpan`, records undo, overwrites the original span and zero-fills slack. It updates the big-endian row `+8` word; other entries and archive length stay in place. |
| Growing replacement | Relocation defaults to enabled: append at EOF aligned to the header's block size, update this row's block offset and `+8` size, and update the 64-bit archive-size field at `0x18`. Existing payloads are not reflowed; the former payload remains unused. This is an archive edit, but not a same-offset write. |
| Undo and validation | `EbUndoRecord` saves the old payload span and complete table row in `.nhl19-undo`; the GUI offers Undo last write and reopens after replacement. Restore does not save/restore the archive-size header or truncate appended data. The GUI replacement path does not enforce original dimensions, mip layout or descriptor compatibility, and the writer leaves row `+4`, `+12` and optional `+16` untouched. No source-hash check or coordinated transaction is present in these methods. |

Studio reuse points are [EB archive code](../../crates/tdb-archive-big/src/eb.rs),
[archive codecs](../../crates/tdb-archive-codec/src/lib.rs),
[asset replacement](../../crates/studio-domain/src/assets.rs) and
[GNF encoding](../../crates/tdb-rx2/src/gnf.rs). `EbArchive` already streams table/payload reads;
`EbFile` materializes the archive and offers slot replacement or full reflow, leaving the `+12` word
unchanged. Its emitter reconstructs gaps as zero, so arbitrary source padding is not preserved.
`replace_texture_in_file` dispatches extracted asset bytes, while `replace_gnf_texture` updates only
the base mip and preserves old trailing mips. Existing code therefore supplies useful primitives,
but does not complete the tester's archive workflow or Q10's mip requirements. The Legacy Xbox 360
probe comments also report crashes for raw/reflow and chunkzip/in-place experiments; these are a
reason to qualify codec substitutions per target, not evidence against PS4 writes as a whole.

## 3. Execution sequence

Use the milestone IDs below for completion tracking; original roadmap and QA phase numbers remain
historical references. A milestone is complete only after its exit gate. QA runs during feature
development, not after all coding. Calendar estimates follow fixture/runtime inventory and design
decisions; each discovery spike produces a bounded implementation estimate or a specific blocker.

### C0 — Establish a reproducible baseline and evidence ownership

1. Inventory tracked and untracked work without resetting, cleaning, or committing it. Identify
   missing build inputs, generated DTOs, diagnostic-only files, the corpus, and both new crates.
2. Run existing fmt, clippy, Rust tests, frontend build, DTO and QA drift checks as available. Record
   baseline failures separately from later regressions. Prove an isolated build can include the
   intended uncommitted source without depending on accidental local files.
3. Reconcile README, CLAUDE, DESIGN scope, compatibility documentation, original roadmap, and asset
   plan with the current evidence. Preserve audit history and don't promote statuses by prose.
4. Separate generated automated evidence from durable manual records. Define a deterministic merge
   so `qa-inventory` cannot erase manual sign-off or silently overwrite later asset evidence.
5. Create a fixture/runtime register: exact title/platform/build, save modes, archive/asset variants,
   available runtime, owner, fixture hashes, storage policy, blocker, and next action. Include
   crate-local asset fixtures and extracted trees; do not require private data to enter git.
6. Resolve the first scope/runtime questions. Map every completion item to a milestone, evidence
   cell/case, accountable implementer or manual tester, and unresolved decision.
7. Specify the canonical Project schema and versioned target-adapter contract. Expand the existing
   schema snapshots into the comprehensive semantic/physical registry defined by Q07a.1, and create
   the equivalent per-family asset output profiles. Every fact must link to fixture/audit evidence;
   unknown facts become explicit blockers or unsupported target capabilities, never inferred defaults.

Exit: baseline report; reconciled target register; reproducible QA evidence generation; every
required feature has an owner and a test route or an explicit unresolved fixture/runtime blocker.
The canonical schema/adapter contract is versioned and can express every proven DB field and asset
constraint. No commit or push is part of this milestone unless requested.

**C0 status (reconciled 2026-09-08, corrected 2026-09-09, package
[C0-CLOSE1](../receipts/c0-close1-remaining-work.md) + R1; **current-status cells corrected
2026-09-13 by [C0-NEXT1](../receipts/c0-next1-evidence-activation.md)**): closure candidate —
architect review pending. Not complete.** This block supersedes the 2026-09-07 C0-BULK1 statement; superseded claims
are listed and attributed in [C0-CLOSE1 §2.2](../receipts/c0-close1-remaining-work.md), and the
conclusions C0-CLOSE1 itself got wrong — chiefly that registry population was not C0 work — are
withdrawn in its §0. No historical receipt was rewritten. Implemented, tested, architect-accepted,
committed and activated are separated deliberately; they are not the same thing.

| C0 item | Implemented | Tested | Accepted | Committed | Activated | Record |
|---|---|---|---|---|---|---|
| 1 inventory | yes | n/a | yes | **yes** | n/a | [c0-01](../validation/c0-01-baseline-inventory.md) |
| 2 baseline execution | yes | yes | yes | **partly** — B1–B3 repairs are working-tree only; B4–B7 and Windows CI open | n/a | [c0-02](../validation/c0-02-baseline-execution.md), [c0-int1](../validation/c0-integration-01-baseline-repairs.md), [c0-dt1](../validation/c0-dt1-dto-drift-gate.md), [c0-fx1](../validation/c0-fx1-explicit-fixture-tests.md) |
| 3 documentation reconciliation | yes | n/a | yes | no | n/a | [c0-03](../validation/c0-03-guidance-reconciliation.md) |
| 4 generated vs manual evidence, deterministic merge | **yes — S1, S2 and S3.** The four matrix emitters **are wired** (parallel `observe`, journalled before the first case) under **`QA_EVIDENCE_ROOT`**, which enables observation capture; **`QA_WRITE` is a separate switch controlling legacy CSV regeneration**. `qa-evidence check` **has been validated against a real non-empty tree held outside the repository** | yes | S1/S3 accepted; the S2-INT1 R1–R5 chain has no acceptance record here; C0-MIG-REVIEW1 accepted; [C0-MIG-ID1](../receipts/c0-mig-id1.md) + [R1](../receipts/c0-mig-id1-r1-isolated-verification.md) implemented, tested and isolation-verified, acceptance pending | **evidence core: yes — `819940a`** ([C0-COMMIT-EVID1](../receipts/c0-commit-evid1.md), 2026-09-13): the `tdb-qa` crate, its tests, `defs/qa/scopes.toml` and one lock line. **Emitter integration and fixture freshness: yes — `52a12fd`** ([C0-CLOSE-EMIT](../receipts/c0-close-emit-development-transition.md), 2026-09-15): the four `studio-domain` matrix suites capture executed observations with fixture identity under `QA_EVIDENCE_ROOT`; discovery/build/check verify fixture identity ([EMIT1](../receipts/c0-emit1-observation-integration.md), [R1](../receipts/c0-emit1-r1-freshness-coverage.md), [R2](../receipts/c0-emit1-r2-fixture-freshness.md), [R3](../receipts/c0-emit1-r3-attribution-path-guards.md)). No published evidence, no registry | **no — parked by direction (2026-09-15) until the feature-complete initial RC QA transition (C9).** Reproducibility is demonstrated on external and isolated HEAD-based trees; live migration, publication, consumer transition, vocabulary/variant freeze, storage/retention and the CI gate are all **inactive** and remain open tasks at that gate — none is complete | [c0-04a](../validation/c0-04a-evidence-contract.md), [receipts](../receipts/), [c0-s2-int1](../receipts/c0-s2-int1-emitter-integration.md) + R1–R5 |
| 5 fixture/runtime register | register yes | measured counts | through C0-BULK1 | no | n/a | [c0-05](../validation/c0-05-fixture-runtime-register.md) — **18 identifiers, 17 still open** (FR18 closed, FR16's schema half closed). Its **storage-policy** half (U3, FR2, FR3) is **outstanding** |
| 6 completion mapping | mapping yes (2,016-row mechanised universe) | regenerates | yes | no | n/a | [c0-06 §6](../validation/c0-06-canonical-project-and-adapters.md) — **no item has an owner** |
| 7 canonical schema/adapter contract **and the comprehensive registry** | **contract: format complete, both halves.** **Registry: 4/28 database units committed (`nhl15-xbox360`, `nhl10-xbox360`, `nhllegacy-xbox360` at `1ede5ba` — [C0-REG1a](../receipts/c0-reg1a-legacy-xbox360-adapter.md) / [R1](../receipts/c0-reg1a-r1-legacy-evidence-correction.md) / [R2](../receipts/c0-reg1a-r2-representation.md), committed by [C0-COMMIT-REG1](../receipts/c0-commit-reg1.md); partial: 10 / 2 965 fields, `player.shoots` and `player.team` blocked — and `nhl15-ps3` at `58d1bc0` — [C0-REG1b](../receipts/c0-reg1b-nhl15-ps3-adapter.md), committed by [C0-COMMIT-REG1b](../receipts/c0-commit-reg1b.md); partial: 10 / 2 969 fields, same two mappings blocked, capacities from its own PS3 ROSTER fixture); **updated 2026-09-15: 14/28 units** — plus `2ec9fde` (six, C0-REG2), `26d5e44` (four PS3, C0-REG3), `6ba9af4` (NHL 10 quarantine); **`c5e3a2b` (2026-09-15, C0-REG4-CLOSE): +7 PS4 units** (`nhl15-ps4` … `nhl21-ps4`, [C0-REG4](../receipts/c0-reg4-ps4-database-adapters.md) + [R1](../receipts/c0-reg4-r1-attribution-and-verification.md), 449 canonical cases on the exact staged tree) → **21/28 committed: 19 attributed + 2 registered as representation only** (`nhl19-ps4`, `nhl20-ps4`: byte-identical anchors, attribution unresolved, nothing established); **`039de62` (C0-REG5-CLOSE): +6 early-generation units** (`nhl07-xbox360`, `nhl08-xbox360`, `nhl09-xbox360`, `nhl08-ps3`, `nhl09-ps3` attributed and partial; `nhl07-ps3` the research copy as representation only — [C0-REG5](../receipts/c0-reg5-early-generation-adapters.md), 519 cases on the exact staged tree) → **27/28 database units registered: 24 attributed partial adapters + 3 representation-only; Legacy Recompiled absent (FR4)**. **Asset side, `cc913f9` (C0-AS5 + CLOSE, [receipt](../receipts/c0-as5-shared-legacy-asset-profiles.md)):** `asset-output-profile/5` shared definitions by reference; `legacyrecomp-recomp` registered by reference in all 3 committed families (Q47), nothing operative, destination blocks unknown; 532 cases. Next: asset-family population, not further reuse-contract expansion — **and, of the 7 asset-bearing families, 3 committed** (`assets-portraits` populated; `assets-logos` and `assets-jerseys` at `0251b57` / `c3f4181`, represented not populated) **plus `assets-equipment` at `9e02acf` (C0-AS6-CLOSE, 2026-09-16, [receipt](../receipts/c0-as6-asset-family-coverage.md)) — 4 of 7 committed; the other 7 matrix families are database/save/onboarding workflows and carry no asset profile** (see item 3 below) — the numbered deliverable is not met** | yes (366 canonical cases at `c3f4181` incl. anchor-snapshot cross-checks, a target/snapshot identity guard and per-target fixture cases for 12 adapters) | mixed; see [C0-CLOSE1 §4.5](../receipts/c0-close1-remaining-work.md) | **contract half: yes** — `ef0fdd7` committed `crates/tdb-canonical/**`, `defs/canonical/`, `defs/adapters/` (2 adapters, 1 asset profile) per [C0-CAN-COMMIT2](../receipts/c0-can-commit2.md); `1ede5ba` committed the third adapter and its target-bound verification; `58d1bc0` committed the fourth (`nhl15-ps3`) and the shared `target_evidence` test mechanics; **registry population: partial, not met** (as of 2026-09-16 / `9e02acf`: 1 database unit — Recompiled, FR4 — and 3 of the 7 asset-bearing families have nothing committed; the committed profiles beyond portraits are represented, not populated) | n/a | [c0-06 §5](../validation/c0-06-canonical-project-and-adapters.md), [c0-can1](../receipts/c0-can1-adapter-constraints.md)+R1/R2, [c0-as1](../receipts/c0-as1-asset-profile-constraints.md), [c0-as2](../receipts/c0-as2-r1-build-mapping-overrides.md)+[R2](../receipts/c0-as2-r2-exact-approval-binding.md), [c0-as3](../receipts/c0-as3-scoped-asset-variants.md)+[R1](../receipts/c0-as3-r1-source-backed-variants.md)+[R2](../receipts/c0-as3-r2-observation-authority.md) |

**Met:** the inventory and baseline (items 1–2); the documentation reconciliation (item 3); the
register itself (item 5); the milestone/evidence-cell/test-route mapping (item 6); a defined,
deterministic merge whose reproducibility is demonstrated on external trees (item 4); and a
**versioned contract that can express** every constraint proven to date — `target-adapter/2` covers
all 20 database constraints Q07a.1 names, `asset-output-profile/3` all 26 asset constraints, each
carrying evidence scope and force (item 7's contract half). C0 requires no commit.

**Unmet — four deliverables and one review, none of which closes C0 alone:**

1. **Item 6 — ownership.** Every row in both registers is `owner-unassigned` (FR12). Architect
   decision.
2. **Item 5 — storage policy.** U3 (evidence tree location and commit policy), FR2 (mounted
   installs behind the fixture root) and FR3 (outer-package acquisition and storage) are named in
   item 5's own text and remain undecided. Architect decision; FR3 also a user decision.
3. **Item 7 — the comprehensive registry.** Item 7 requires expanding the existing schema snapshots
   into the Q07a.1 semantic/physical registry **and creating the equivalent per-family asset output
   profiles**. **As of 2026-09-15 (C0-AS4-CLOSE), 14 of 28 database units have a committed partial
   adapter and 14 have none; 3 asset families have a committed profile.** **Corrected 2026-09-16
   (C0-AS6): the family axis has 14 entries but only 7 are asset-bearing — the other 7 are
   database (`db-presentation`, `db-operations`; C1), save (`save-raw-db`, `save-inner-container`,
   `save-outer-package`; C2) and onboarding (`onboarding-install`, `onboarding-localization`;
   C1/C6) workflows whose registry object is the database adapter, not an asset profile, and a
   profile filed under one is now refused (`asset-family-not-asset-bearing`). Of the 7:
   `assets-portraits` populated, `assets-logos` / `assets-jerseys` represented (committed),
   `assets-equipment` represented (`9e02acf`, C0-AS6), `assets-heads` absent (external
   conversion workstream; Studio integration need recorded), `assets-arenas` absent (stretch, by
   decision), `assets-audio` absent (C7 audit first). "11 of 14 missing" was a category error; no
   scope is dropped — every family keeps its QA-matrix row.**
   Committed adapters: `nhl15-xbox360`, `nhl10-xbox360`, `nhllegacy-xbox360` (`1ede5ba`),
   `nhl15-ps3` (`58d1bc0`); `nhllegacy-ps3`, `nhl14-xbox360`, `nhl14-ps3`, `nhl13-xbox360`,
   `nhl12-xbox360`, `nhl11-xbox360` (`2ec9fde`, [C0-REG2](../receipts/c0-reg2-database-asset-registry.md));
   `nhl10-ps3`, `nhl11-ps3`, `nhl12-ps3`, `nhl13-ps3` (`26d5e44`,
   [C0-REG3](../receipts/c0-reg3-database-asset-design.md)). Twelve of the fourteen are measured on
   their own anchor and ROSTER fixture (capacities, discriminator, companion rule); `nhl15-xbox360`
   and `nhl10-xbox360` remain structural with audit-derived capacities. **Committed at `c5e3a2b`
   (C0-REG4-CLOSE, 2026-09-15):** `nhl15-ps4` through `nhl21-ps4`, each measured on its own PS4 anchor
   (main-DB directory capacities — the gen8 main DB is not packed — with the ROSTER save's own
   directory compared through the committed PS4 codec on NHL 16–19, blocked on NHL 15 and
   missing-fixture on NHL 20/21; 7-bit identity ratings; `team.name` unresolved on NHL 16). Two
   evidence gaps surfaced and recorded, not resolved: the committed NHL 19 and NHL 20 PS4 anchors
   are byte-identical (main DB, metadata and auxiliary databases) with no capture provenance for
   either — **C0-REG4-R1 makes both adapters non-operative** (every measurement a candidate under an
   unknown block, every mapping blocked; registered representation, not coverage, until an
   independent witness attributes the bytes) — and NHL 18–21 carry un-rostered main-team pointers
   outside the team key set.
   **Uncommitted, for review (C0-REG5, 2026-09-15):** the six early-generation units —
   `nhl07-xbox360` (gen6: two-companion rule, thin team index with names on `stockteamtable`, the
   audit's gen6 rating offset 50), `nhl08-xbox360` / `nhl08-ps3` and `nhl09-xbox360` / `nhl09-ps3`
   (as-is main-team pointer with no absence code, 1-origin birth dates, a rating regime **no audit
   row names** — the probe's result a candidate; NHL 09's ROSTER fixtures carry the ceiling, NHL 07's
   and NHL 08's decode to a TDB without the roster core so their capacities are unknown, not
   substituted; NHL 09 holds four companion-less bio rows), and the NHL 07 PS3 research copy
   registered under `nhl07-ps3` as **representation only** (byte-identical to the Xbox 360 anchor,
   no PS3 capture, not a shipping target). Remaining without any adapter after REG5 (1 of 28):
   Legacy Recompiled (no anchor, FR4). Its **asset** side is now registered by reference: Q47
   (user-confirmed identity of the asset files) is implemented by C0-AS5 as
   `asset-output-profile/5` — `legacyrecomp-recomp` reuses `nhllegacy-xbox360` in the portrait,
   logo and jersey profiles, inheriting identity / formats / naming / variants / chains /
   operations in their source states (nothing operative) while discovery, install and
   runtime_lookup stay Recompiled's own and unknown; no measured hash invented, no database
   equivalence inferred, no qualification inherited (committed at `cc913f9`, C0-AS5-CLOSE).
   **NHL 07 team-name join — a canonical execution requirement, not implemented:**
   `nhl07-xbox360` maps `team.name` to `stockteamtable.fullname` and its named case measures the
   `kOtt.(table, teamid) -> stockteamtable.index` join on the anchor, but the contract has no
   join-path block and the direct-field round trip does **not** implement the join; resolving a
   team's name on NHL 07 through the canonical registry requires executing that join (as the
   engine's `team_name_source` thin shape does today) and is an open C1 requirement.
   **C0-AS6 (2026-09-16, committed at `9e02acf` by C0-AS6-CLOSE, [receipt](../receipts/c0-as6-asset-family-coverage.md)):**
   `assets-equipment` represented on `nhllegacy-xbox360` and `nhllegacy-ps3` (+ Recompiled by
   reference) — the eight equipment id fields measured on the byte-identical Legacy anchors
   (widths 7/7/8/7/7/7/7/9 = the HEAD `SLOT_SPECS` ceilings, which are Legacy widths carried as
   code: on NHL 08 the stick is 8 bits and the other present slots 4–5, on NHL 21 PS4 all are
   10), grammar from HEAD source, the PS3 operator stick
   record (74 `texlib_N.rpsgl`, 106 thumbnails, an on-ice test) as asserted candidates; the
   portrait profile's `[identity]` gains a measured candidate on all three declared targets
   (`ubPc.lOnr` on the anchors — 5 450 distinct ids on Legacy, 350 above every observed bucket
   range; 7 244 on NHL 21 PS4; still unknown, no id placed beside a file); the 19
   asset-referenced database fields recorded on all 27 anchors.
   **C0-AS7 (2026-09-16, committed at `8d60a6d` by C0-AS7-CLOSE after Codex review passed — [receipt](../receipts/c0-as7-asset-evidence-measurements.md)):**
   the D1 + D4 + D3 batch from the C0-AS6 receipt §D, on the two already-identified copies, through
   the committed EB reader and a new pure `tdb_canonical::asset_census` module: `cacherender.big`
   and `nocacherender.big` listed in full (every member under `rendering/`, all `chunklzx`; no
   `fe/` member, so portraits/crests/thumbnails are in one of the eleven archives not opened) and
   joined to the Legacy anchor — jerseys 698/698, all eight measured equipment fields total, logos
   180/218 under the declared spelling (218/218 under `texlib_{id}_{a}_{b}`), and a ninth
   comparison — goalie-stick members against the `xmja.wXFx` field HEAD source says the goalie
   stick shares — contradicted; the PS3 portrait
   census per id (18 663 files hashed): 4 500 of 5 449 anchor ids resolve to a loose file, 949 do
   not (350 outside the observed regular ranges 12403..12814, 599 inside), `patch.big` (read; 83
   portrait members, 23 colliding with differing loose content; committed `chunkzip` fails on its
   257 multi-chunk members) supplies 33. Discovery candidates upgraded to measured corpus
   invariants but not established (one root per target vs families split across two archives);
   identities stay candidates; no readiness change; 552 canonical + 28 archive cases on the isolated
   candidate after review findings P2 (loose-file absence = `NotFound` only) and P3 (summaries
   reconciled to the records) were closed in place.
   (`docs/receipts/data/c0-as6-asset-db-references.csv`, drift-guarded). Nothing became
   operative. The smallest measurement batch that would change that is named in the receipt
   (§D: Legacy Xbox 360 `cacherender.big` / `nocacherender.big` member tables; a per-id PS3
   portrait census; `patch.big` opened).
   Every adapter is partial
   (9–10 of ~1 770–2 970 fields), `player.shoots` and `player.team` are blocked on all, and the
   NHL 10 six-bit rating regime is **quarantined on both platforms** (`6ba9af4`: the ratings audit
   and the production probe disagree on the anchor — a C1 finding awaiting one in-game witness).
   Adapter presence is not target completion; none is game-verified.
   Committed profiles: `assets-portraits` (`ef0fdd7`; three targets, the only family with any
   established block — a few specimen-level portrait facts on Legacy PS3) and, **represented but
   not populated**, `assets-logos` (`0251b57`) and `assets-jerseys` (`0251b57`, moved to
   `asset-output-profile/4` at `c3f4181`, [C0-AS4](../receipts/c0-as4-composite-resolution.md)):
   every block unknown on `nhllegacy-xbox360`, the jersey composite id measured on the anchor
   (703 `xjgw` rows → 698 distinct ids) and carried as a candidate, the thumbnail fallback chain a
   candidate from working-tree source. **Represented ≠ populated ≠ supported:** a profile file
   enumerates the gap in the contract's terms; populated support would be operative blocks at a
   scope that speaks for the target, and no family has one beyond the portrait specimens.
   **Registering the target-specific evidence this repository already holds — the 27 physical
   snapshots, the 28 audit documents, the anchors manifest, `defs/enums/` — is C0 work and must not
   wait on fresh measurement.** Resolving genuinely missing facts and proving runtime compatibility
   is later measurement work. FR4 bounds the database half to 27 of 28 units.
4. **Item 4 — activation sequencing.** Decided 2026-09-15 (direction update below): migration
   installation, publication, consumer transition and the CI gate are **parked until the RC QA
   transition (C9)**. They remain open tasks at that gate — this is a sequencing decision, not a
   scope reduction, and none of them is complete.
5. **Review.** Several packages behind items 4 and 7 carry no acceptance record in this repository.
   That is not evidence no review occurred, and no blanket acceptance is asserted either way.

Registered fixture and runtime blockers (FR1, FR4–FR11, FR13–FR17) are what the exit criterion
permits in place of a test route and do **not** block C0 closure; each blocks a later milestone.

**What the registries do and do not say.** The one populated asset profile declares **7 established
constraints on 1 of its 3 targets**, every one an `observed-source-format` read of one inspected
corpus, and `target_required_constraints` is pinned at **0** by test: **no asset constraint reaches
`supported-target-constraint`**, so none states a demonstrated target requirement. That is a
statement about the **asset** registry — the database adapters do carry established target-specific
facts (61 blocks over two units, 30 measured and 31 derived, each linked to anchor or audit
evidence), and the two must not be collapsed. Provenance bridges are **declarations** requiring
separately reviewed evidence, and none is populated; `[runtime_lookup]` is unknown on every target;
operator-entered observations are representable but cannot decide a variant. An `unknown` block —
including one carrying a candidate — is **not** established coverage, a profile consisting of
unknowns is representation rather than population, and no cross-title asset compatibility is implied
anywhere.

**What is committed (corrected 2026-09-13).** Items 1–2; since `ef0fdd7`
([C0-CAN-COMMIT2](../receipts/c0-can-commit2.md), 2026-09-09), the **canonical contract slice** —
`crates/tdb-canonical/**`, `defs/canonical/`, `defs/adapters/`; and since `819940a`
([C0-COMMIT-EVID1](../receipts/c0-commit-evid1.md), 2026-09-13), the **evidence core** — the
`tdb-qa` crate, its tests, `defs/qa/scopes.toml` and one lock line. The statement that nothing beyond
items 1–2 was committed was true when written and is superseded. Since `52a12fd`
([C0-CLOSE-EMIT](../receipts/c0-close-emit-development-transition.md), 2026-09-15), the **emitter
integration and fixture-freshness slice** — the four matrix suites' capture wiring, the freshness
scopes, fixture identity verification and one further lock line. **Still uncommitted:**
`docs/receipts/`, the validation records written since C0.2, and **this roadmap itself** — all in a
mixed working tree together with the repo-wide `fmt`/`clippy` repairs. The evidence core's boundary
was enumerated and isolation-verified by [C0-NEXT1 §2–§3](../receipts/c0-next1-evidence-activation.md)
and committed as exactly that set; the emitter slice's 20-path boundary by EMIT1-R3 §5.
None of the remaining uncommitted work is recoverable from HEAD. No capability is promoted, and **no cell anywhere is
game-verified**. The ordered remaining queue, its dependencies and the decisions it needs are in
[C0-CLOSE1 §6–§7](../receipts/c0-close1-remaining-work.md).

**Historical direction update 2026-09-15 (scheduling superseded on 2026-09-16 by the
[parallel completion plan](parallel-completion-plan-2026-09-16.md); retained for provenance;
the evidence-activation deferral remains current).** User-directed, recorded by
[C0-CLOSE-EMIT](../receipts/c0-close-emit-development-transition.md). Codex accepted the
EMIT1/R1/R2/R3 behaviour within its demonstrated scope and it is committed at `52a12fd`. From here
the sequence is:

1. **Comprehensive database and asset registry development is the primary development track**
   (item 7's registry half, as of 2026-09-16: 1 database unit (Recompiled, FR4) and 3 of the
   **7 asset-bearing** families still have nothing committed — the other 7 matrix families are
   database/save/onboarding workflows with no asset profile to write; the 27 committed adapters
   are partial, `assets-portraits` is populated on one target and the other committed profiles are
   represented, not populated; C0-REG4's seven PS4 adapters are committed at `c5e3a2b` and C0-REG5's six early-generation adapters at `039de62`; C0-AS5's `/5` reuse and the Recompiled by-reference registration at `cc913f9`; C0-AS6's family reconciliation, equipment profile and anchor measurements at `9e02acf`; C0-AS7's measurement batch D1 + D4 + D3 at `8d60a6d` (Legacy Xbox 360 render-archive member tables joined to the anchor, PS3 per-id portrait census joined to the anchor, PS3 `patch.big` read — file halves observed, nothing promoted, [receipt](../receipts/c0-as7-asset-evidence-measurements.md)); next: list `cache.big` / `nocache.big` with the same tool to locate the portrait, crest and thumbnail members and decide per-family discovery on complete evidence).
2. Consolidate ownership, fixtures and ingestion requirements (items 5–6: FR12 owners, U3/FR2/FR3
   storage policy).
3. Complete the broader baseline and Windows hardening (C0.2 B4–B7, R6).
4. Continue feature completion (C1 onward).
5. **Resume evidence-infrastructure integration and activation at the feature-complete initial RC
   QA transition (C9).** Evidence activation no longer blocks development.

Ordinary tests, fixture identities and development receipts continue throughout. **Parked, not
complete, at that gate:** storage/retention policy for evidence trees (U3, layer-A retention),
migration installation and first publication, consumer transition of `game-qa-matrix.csv`,
`matrix_guard` and the emitter CSVs under R-TRANSITION, the `conversion.class`/recipe-id vocabulary
and variant freeze (U2, D8, D9), and the `qa-evidence check` CI gate. None of these is marked done
by this update, and no further evidence-hardening package is planned unless a new defect
materially blocks development.

**Asset registration must describe both ingestion and destination output.** Every asset unit in the
registry states, per family and target: canonical identity; storage/archive location; formats and
constraints; references and dependencies; conversion; installation; and validation. A profile that
describes only the source format is representation, not registration.

**External completed-mod ingestion is required.** Ingesting a completed external mod compares it
against a matching vanilla baseline where one is available, preserves provenance and every
unresolved file, creates an independent canonical Project, and reports portability limitations
explicitly. It never infers compatibility with a target the evidence does not cover.

**Previously deferred asset operations stay deferred** until a recorded decision expands them; no
package may widen them as a side effect. Headmesh conversion remains external until native
integration (C5 direction update).

### C1 — Correctness defects and database parity closure

1. Reproduce and fix the dressed/scratched mapping to player-type `ZBac.RREB`; derive the correct
   status field per supported schema from evidence. Disable the control where semantics remain unknown.
2. Make overall display consistent across player lists, details, historical views, and Legends
   ranking. Gate uncalibrated regressions; distinguish stored overall from computed estimates.
3. Verify goalie-type codes on `hHbv`; replace guesses with audited mappings or disable editing.
4. Finish skater/goalie companion creation on position flip, multi-select transactions, and clone
   name editing. Batch operations must preview conflicts and roll back without partial rosters.
5. Check manual player construction, lines/best-lines, captaincy, retired numbers, contracts, draft
   picks, strategies/mirrors, memberships, capacity normalization, and repair flows against the
   binding edit contract. Existing matrix success does not prove a missing operation exists.
6. Complete the field-disposition inventory: curated, raw-editable, read-only, or intentionally
   omitted with reasons. Unknown semantics and identity/structural fields stay protected.

QA: scalar and relational tests across every applicable anchor, all distinct encodings and
representative entity shapes; invalid-input atomicity, undo/redo, save/reopen/second no-op save,
audit deltas, capacity boundaries, and human-readable pending changes. Add focused regressions for
the known defects. Start the manual database campaign immediately on available targets.

Exit: no unresolved critical/high database defect; agreed database operations implemented; automated
matrix green; controls reflect both schema presence and write qualification. Broader manual
qualification continues through C9.

### C2 — Finish save/container and runtime write paths

1. Resolve NHL10/11 v1's unverified `tag_1c` integrity behavior using controlled fixtures and game
   acceptance. Preserving a stale word is not a verified write algorithm.
2. Audit NHL15 PS4 roster framing, implement its write path, and qualify NHL16–21 separately.
   Verify the DATA/INDEX/NAME relationships and the actual deployment/reimport process.
3. Implement complete original-console packaging for Xbox 360, PS3 and PS4 required workflows.
   For Xbox360 STFS, cover package identity, metadata, integrity/hash updates, payload replacement,
   and console acceptance. Audit the equivalent PS3 and PS4 outer-container, encryption/signing and
   transfer requirements; implement every part that can be performed by Studio and a guided handoff
   for any unavoidable console-side step. Distinguish inner payload, emulator/decrypted output and
   console-deployable packages in the UI, documentation and QA ledger.
4. Inventory required Franchise (including verified older Dynasty/Be A GM equivalents), Season,
   roster and settings fixtures, including multipart DATA000N cases. Implement segment reassembly,
   schema-safe editing, resegmentation and coordinated writes where those workflows require them.
   Audit settings payloads separately and implement a metadata-driven editor for every safely
   identified setting even when the format is non-TDB. Classify every encountered value as editable,
   read-only, unknown, structural or intentionally omitted, with evidence. Verify friendly labels,
   types, units, ranges/enums, inverse encoding, defaults/reset behavior where known, preservation of
   unknown bytes, and actual effect in-game. For Franchise/Season, expose the existing player/team/roster tools supported by
   each mode's schema; the source schedule workflow is handled in C6. Standings, accumulated season
   records, finances and other mode-specific state are outside required scope. Supporting a container
   does not by itself complete the required editing workflows. Track Be A Pro and
   other game modes as optional stretch work outside the release gate.
5. Obtain Recompiled-specific install/save fixtures and validate its runtime paths and persistence.
   Sharing the Xbox database schema does not prove Recompiled installation or save handling.
6. Verify raw DB workflows and any PS3/PS4 external preparation or reimport steps required by the
   supported environment. Document the complete user journey, including tool boundaries.

QA: every framing, console package and save mode; real scalar and structural edits rather than arbitrary payload
mutation alone; opaque-region preservation; checksums; compressed semantic equality; deterministic
output; malformed/truncated input; locked files; external edits; backup restore; interruption during
multi-file replacement. Validate emulator/decrypted workflows and real original-console deployment:
package -> transfer/import -> console game load -> inspect -> play/simulate -> console save -> export
or transfer back -> reopen in Studio. Record any platform tooling and signing identity used.

Exit: each required save path is implemented and has automated plus runtime evidence. Missing
runtime access leaves a named blocker, never an assumed pass.

### C3 — Shared production asset writes and fixture coverage

1. Register representative asset/archive fixtures by title/platform/family/format and origin.
   Reuse existing codec fixtures; add verified local fixtures where gaps remain.
2. Unify discovery, identity resolution, used-by references, format capability reporting, and
   source-versus-installed asset tracking. Keep unsupported formats local to the affected operation.
   Define canonical source representations and a target output profile for each asset family; retain
   originals and record all automatic conversion parameters.
3. Close archive replacement/repacking/index-update paths, including size-changing replacements,
   nested containers, texture pools/atlases, and supported compression variants.
4. Implement the Q06 unified preview -> pending review -> validate -> apply transaction across DBs,
   saves and assets. Include dependency-aware ordering, validated temporary outputs, backups, source
   hash and disk-space checks, a durable journal, cancellation before the commit boundary, crash
   recovery and complete restore. Partial failure cannot leave dangling assignments or mixed project
   state. The same change-set identity follows emulator/decrypted and console-package outputs.
5. Standardize jobs: progress, cancellation, timeouts, useful errors/logs, temp cleanup, and bounded
   memory. Define per-format byte identity versus semantic equality and visual tolerances.

Exit: one full discover/export/reimport/replace/install/restore path proven in a copied game tree;
fixture and operation matrices established for every family. This foundation gates production
installation for C4–C7; read/preview development can proceed earlier.

### C3a — Required Ignite EB archive-backed texture editing

Implement the tester's useful read/replace workflow natively in Studio as a required part of C3,
starting with NHL19 PS4 fixtures and qualifying every applicable PS4 NHL15–21 target separately.
Treat Ignite as the requested workflow scope, not a format signature or a blanket Xbox One support
claim. Keep classic BIGF/BIG4 and Legacy EB variants behind their own proven profiles. This work
does not expand the agreed arena-authoring or mesh-authoring scope.

1. **Archive-backed identity and browsing:** open a `.big` or scan an archive folder without mandatory
   whole-game extraction. Index friendly categories, internal paths, types and texture metadata;
   seek/decode only requested entries with bounded memory and cancelable jobs. Identify a target by
   game/platform, archive identity/hash, entry index/path and nested texture index, not basename alone.
   Preserve that identity through preview, export, pending changes, apply and reopen. Detect duplicate
   paths across archives and shared/overlapping payload spans before choosing a writable target.
2. **Read/codec parity:** reconcile the tester's flag-derived alignment/row sizes and size-field
   interpretations with Studio's current EB profiles. Add hashed fixtures for 16/64-byte alignment,
   16/20-byte rows where encountered, archives beyond 4 GB, raw entries, compact/extended chunkzip,
   multiple chunks and stored chunks. Validate table bounds, arithmetic, declared versus consumed
   spans and decompression limits. Dispatch exact codec signatures; unsupported LZX or other wrappers
   remain explicit operation blockers. Qualify source-preserving encoders or any required chunkzip
   substitution against the actual target game; never infer interchangeability from tester behavior.
3. **Streaming write planner:** lower the validated replacement into encoded bytes and choose either
   fit-in-slot overwrite/padding or aligned append-and-retarget when compressed size grows. Growth
   in stored bytes is required even under Q10's unchanged texture dimensions/format. Preserve unrelated
   entries, ordering, opaque fields, padding and old payload regions; do not use full archive reflow
   as the default replacement path. Prove which row size/integrity fields must change, including
   nonzero `+4`, `+8`, `+12`, optional `+16`, block offsets and header size. Refuse unqualified layouts
   or shared-span edits until an explicit safe isolation/reference policy exists. Use checked 64-bit
   offsets and validate representability in on-disk fields before producing output.
4. **Transactional archive apply:** execute the plan against a streamed temporary archive artifact,
   validate/reopen it, then install through C3/Q06 with source conflict checks and a durable journal.
   Coalesce pending edits to the same archive into one plan; calculate subsequent spans/offsets from
   staged state and keep coordinated DB/reference edits in the same transaction. Standard mode saves
   back to the selected archive or Save As without a Project prerequisite; Developer Mode derives
   immutable Build files from the committed Project revision. Retain full recovery of original bytes,
   table rows, size header and physical file length, including append relocation. Tester-style
   single-entry undo alone does not meet this gate.
5. **Dependencies and integration:** reuse `tdb-archive-big`, `tdb-archive-codec`, `tdb-rx2` and the
   domain/API/job layers rather than requiring the tester executable. Noesis/orbis tools are reference
   implementations; any retained external dependency needs a verified integration contract and C10
   license/distribution disposition, with approved components delivered through Q44. Detect missing
   tools explicitly, bound process duration, capture failures and support cancellation. Do not copy
   the tester's silent missing-dependency behavior into Studio.

QA cases must cover unchanged raw replacement/no-op byte identity, smaller/exact-fit replacements,
growth requiring relocation, repeated edits and multiple pending entries, incompressible input,
nonzero secondary size fields, opaque metadata/padding preservation, aliased spans, truncated input,
out-of-range offsets, locked files, external modification, low disk space and interruption at payload,
table, header and installation boundaries. Verify unchanged-entry hashes and complete archive hash
restoration, including original EOF after relocation; compare decoded pixels semantically only when
recompression is intended. No-op detection must avoid unnecessary decode/re-encode drift.

Exit: C3a read/fit/relocate/restore cases pass automatically on representative fixtures, and each
applicable exact target completes C9 using a visibly changed texture loaded from the edited `.big`,
including a replacement that requires relocation. Reopening in Studio or the tester is necessary
diagnostic evidence but cannot establish game acceptance. Unqualified metadata/codec paths remain
named blockers; archive-backed texture writes cannot be marked complete through loose-file output.

### C4 — Complete the 2D asset workflows

Texture Lab: implement indexed search/filtering, decoded preview, channel/alpha and mip inspection,
used-by lookup, bulk export, replace-from-PNG, target-format conversion preview, validation, pending
writes, install and restore. Reuse the codec and job layers. **Q09 decision: Texture Lab is a browser,
converter and replacer only.** It may inspect channels, alpha, mip levels, metadata and encoded versus
decoded output, but it does not paint, composite, retouch or author channels. Source artwork changes
happen in external image tools and return through import/replacement. Image-authoring features are
outside the release scope rather than a hidden unfinished part of Texture Lab.

**Archive workflow (C3a):** expose open archive/folder -> browse/search -> select exact entry/texture
-> inspect/export -> import replacement -> before/after conversion preview -> stage -> review/apply
-> reopen/restore. Raw native extraction/replacement and DDS interchange export should distinguish
original stored bytes, decoded native payloads and recompressed interchange images in their contracts.
Report decoded DDS fallback explicitly. Native GNF import must pass the same validation as image
conversion; a `GNF ` magic match is insufficient. Show archive destination, affected references and
whether output fits or requires relocation in the review. Keep manual single-target replacement under
Q11; folder browsing and bulk export do not imply batch replacement.

Complete GNF mip-chain regeneration in the original layout for changed images; the current base-mip
patcher is insufficient for textures with lower mips. Preserve/validate descriptor fields, texture
count/type, tiling, channel mapping, UNORM versus SNORM, linear versus sRGB, alpha and normal-map
semantics alongside dimensions/format. Surface unsupported arrays/cubemaps/multi-texture layouts
locally until their adapters are qualified. DDS format-preserving export is semantic recompression,
not a byte-lossless round trip; provide native payload export for original-byte preservation.

**Texture output constraints (Q10):** release replacements must preserve the target asset's original
dimensions and encoded format. The converter may accept a supported source interchange image and
produce those exact required dimensions/format, with the resize/crop/pad result previewed, but it may
not change the installed texture specification. Validate mip count/layout, channels, compression,
tiling/swizzle, atlas/pool placement and encoded size constraints in addition to width/height and
container label. Higher-resolution installed textures are a stretch research goal, isolated behind
experimental capability gating and requiring per-title/platform archive, memory, rendering and
in-game stability evidence before any future promotion.

**Texture replacement workflow (Q11):** the initial release requires manual, one-at-a-time target
selection and replacement. The user selects the exact resolved texture/usage, chooses a source file,
reviews the decoded and converted before/after preview plus affected references, and stages that
single replacement into the unified change set. Bulk export remains available, but automatic path or
manifest matching and batch replacement are stretch goals. Any future batch workflow must prove
deterministic identity matching, collision handling, complete preview and atomic rollback before it
can leave experimental status.

Portraits: reuse the established universal player slug and existing EA/FHM identity normalization;
implement the remaining source library index, portrait import/match review, persistent mappings and
conversion into each verified target format. Never silently bind an ambiguous match.

**Universal player asset slug (existing implemented contract, confirmed in Q12):** player-specific
assets use `lastname_firstname_dd_mm_yyyy`. `tdb-core::dev_key` already defines and tests the exact
normalization: lowercase ASCII transliteration, punctuation/whitespace/hyphens removed from each name,
and zero-padded `dd_mm_yyyy`; it also validates plausible complete dates. EA and historical adapters
already derive this bridge key, Player Assets exports it, and the headmesh library recognizes it.
Portrait import must call this implementation rather than introduce another parser or normalization
question. Preserve the original source filename and surface absent/malformed/duplicate keys for manual
resolution.

**Portrait normalization (Q14):** portrait library inputs are already normalized to a consistent
composition/background state. Studio must not recrop, reframe, remove/replace backgrounds or apply
style treatment during import/export. The target adapter scales the canonical image to the destination
game's exact portrait dimensions and encodes its required native format, color/channel behavior and
container. Preserve the canonical source unchanged. If an input violates the canonical dimensions or
aspect contract, report it as invalid instead of silently cropping or padding it. Test scaling quality,
alpha/color preservation and both menu/detail and HUD-size variants where the target carries them.

Logos: generate and preview the required per-title icon variants/atlas placements, preserve alpha
and color semantics, manage team references, and install the complete variant set atomically.

Uniforms/nameplates: finish any missing preview-to-output path, including font atlas usage, zones,
placement, generated textures and jersey identity/clone references. **Q16 decision: curved baselines
must produce installable output for release.** Convert the editable baseline/control points and real
game-font glyph layout into the target's actual nameplate texture/zone representation, stage it in the
unified change set, rebuild its native container and install it. The saved result must match the 2D and
3D previews within documented rasterization tolerance; preview-only curved data is not completion.
Support undo/reopen/re-edit, preserve unrelated zones/glyphs, and refuse geometry that clips or cannot
be represented safely by the target.

QA: manual target selection, unchanged reimport, visibly changed replacement, all mip levels, alpha edges, color channels,
aspect ratios, normal/BC4/BC5 semantics where relevant, oversized inputs, atlas/pool boundaries,
wrong-target prevention, ID collisions, transaction rollback and archive reopen. Stretch batch work
adds ambiguity/collision queues and whole-batch rollback. Inspect logos/portraits in
menus and uniforms on ice, at relevant LODs and camera distances, on each applicable target.
For portraits, assert scale-only conversion from the normalized canonical source with no crop,
background or composition changes.
For curved nameplates, test short/long and accented supported names, multiple curve directions,
captain/number/patch coexistence, atlas edges, jersey seams, relevant LODs and in-game camera views.
Assert every release replacement retains its original target dimensions and encoded format; keep
higher-resolution experiments outside the supported matrix.
Extend the existing slug tests only at the new portrait-import integration boundary, covering exact
identity matches, malformed keys, collisions and preservation of original filenames.

Exit: no Texture Lab placeholder; agreed import/generation workflows complete; each claimed 2D
write/install operation qualified independently.

### C5 — Mesh runtime, Appearance Studio, Hockey Shop and headmesh conversion

1. Finish the Blender harness: pinned runtime, delivery layout, dependency notices, invocation
   contract, structured results, progress, cancellation/timeouts, output validation and cleanup.
   Inspect existing work before replacing it. Prove mesh I/O golden round trips and conversion
   tolerances; use byte identity only for operations where that is the actual contract.
2. Appearance enums: audit hair/facial hair/eyes/skin values and offsets per schema; add friendly
   controls, validation, pending changes and game-verified assignment.
3. Canonical head library: ingest the provided database of consistently exported game headmeshes and
   additional Character Creator 5 / Headhost 3 generated heads. Index every entry by the established
   universal player slug, preserve source/provenance and companion assets, and implement catalog
   metadata, thumbnails, inspection, assignment, install and removal/restore. Studio consumes the
   canonical format; it does not require users to repeat source-game extraction during import.
4. Appearance catalog: ingest the supplied canonical database of hair and facial-hair components;
   verify component identity, mesh/texture pairing, compatibility metadata, missing assets,
   rescan/cache invalidation and preview. **Q23 decision: Studio selects from this canonical database;
   it does not graft or bake hair/facial hair as an authoring operation.** Target adapters automatically
   convert and install the selected component into each supported game's verified native representation.
5. Hockey Shop: close per-slot UV/atlas conversion recipes; verify store sync, import/delete,
   clone/new IDs, assignment, installation and recovery. Test skater and goalie equipment.
   **Q25 direction:** evolve the store into a canonical cross-game equipment library. Canonical items
   retain source geometry/textures, slot identity and provenance; target adapters convert them into
   each game's native mesh/texture/container/ID arrangement. UV conversion is the principal open gate:
   establish audited projection/atlas recipes per equipment slot and target, preview converted UVs and
   textures on the actual target mesh, and prohibit export where the recipe is unverified. Test seams,
   orientation, scale, normals, logos, wear/color channels, mip behavior, animation/attachment and
   in-game LODs. **Q25a decision: canonical cross-game conversion is a stretch goal and does not
   block release.** Required Hockey Shop scope remains same-game browse/preview/store/import/delete,
   clone/new-ID handling, assignment, install and recovery for every supported target. Keep the
   canonical library model and experimental adapter gates so verified cross-game slots can be added
   without changing that release boundary.
6. Replace the user-facing source-to-target pair model with canonical-head -> target adapters for
   every supported game/platform. A canonical entry's originating game or CC5/Headhost 3 provenance
   does not change the import workflow. Automatically convert geometry, coordinates, UVs, textures,
   weights/rigging, palettes, companions, LODs, identifiers and containers into the selected target's
   verified representation. Existing NHL24, CC5, donor-transfer, UV and pack work becomes adapter
   implementation/evidence rather than separate product modes.
7. Complete and qualify all target adapters, including PS4 skeleton wiring, mesh/GNF write and roster
   binding, pre-NHL11 layout/ID schemes, gen7 multi-skin copies, PS3/Xbox distinctions, archive rebuild
   and console deployment. An unsupported target adapter blocks full completion because headmeshes are
   required for every game Studio supports; it cannot be hidden as an unlisted pair.

QA: validate the supplied canonical head and appearance-component database schemas, file integrity,
identity uniqueness and
source provenance. For every supported target, import representative canonical heads originating from
multiple games plus CC5/Headhost 3 and select representative canonical hair/facial-hair components.
Check geometry/UV/bone/weight/palette bounds; quantization; scale/orientation; companion eyes/mouth/hair;
helmet interaction; LODs; animation and facial behavior where supported; texture seams and clipping;
multiple representative heads, not just one success. Reopen generated containers, install on emulator
and original console paths, and verify in gameplay. Reject invalid canonical entries before writing.

Exit: packaged mesh jobs work in the supported Windows release; required same-game equipment and
canonical-head/appearance workflows pass; every canonical head and compatible canonical hair/facial-hair component can be
automatically exported through every supported target
adapter, and each target adapter is installed and game-verified with representative corpus and
CC5/Headhost 3 entries. Cross-game equipment conversions remain separately marked experimental or
stretch and do not block release.

**Direction update (2026-09-13, recorded by [C0-NEXT1](../receipts/c0-next1-evidence-activation.md)).**
Game-to-game headmesh conversion is now being completed in a **separate repository**, which owns
that work until it is complete; its results will then be integrated **natively** into Studio. For
this roadmap that means:

- **Deferred until that integration:** Studio-side implementation and generalization of headmesh
  conversion — items 6 and 7 above as *conversion* work (the source→target adapter matrix, donor
  fitting, UV/weight transfer, pack recipes). No new conversion or donor-fitting work is scheduled
  in Studio in the meantime.
- **Preserved, not removed:** the existing `tdb-headmesh` and `blender-job` crates, `defs/appearance/*`,
  the N19/Mogilny/Demitra/Turgeon fixtures, scripts and validation records, the headmesh audits and
  every receipt. They are the evidence and code the integration will land against.
- **Still in Studio's completion scope:** the native integration itself, and its **qualification** —
  every target adapter must still be installed and game-verified through Studio's evidence system
  (C9) before any headmesh cell can promote. The external work does not qualify anything here.
- **Inspection authorized 2026-09-16:** the user supplied
  `E:\Projects\Repositories\nhl-head-mesh-converter` for review and coordination. Its developer
  owns conversion implementation; Studio consumes the completed converter through a reviewed
  integration package. No duplicate Studio-side converter development or premature code transfer.
- **Continues now:** shared asset infrastructure that does not depend on the external
  implementation — C3/C3a recoverable writes and Ignite archives, per-family asset profiles,
  evidence keys and the pair-key registry contract (U2), fixture and runtime registration.
- **Evidence stays honest:** headmesh-specific gaps — the uncommitted `defs/appearance/headmesh-matrix.toml`,
  the ignored real-registry integration case, the open U2 class vocabulary, `recipe = unknown` on
  every pair key — remain **explicit blockers**. No identity is fabricated and no coverage is
  promoted to make C0 or C5 appear further along than the evidence supports.

**User-reported runtime successes, retained as fixture-specific facts.** The Mogilny-over-Eichel
(2026-09-10) and CC5 Demitra revision 2 (2026-09-11) head imports were reported by the user as
successful **in NHL 19 PS4 (game build CUSA11126 / APP_VER 01.60) running on shadPS4** — see
[docs/MESH_UV_TEMPLATE_TRANSFER.md](../MESH_UV_TEMPLATE_TRANSFER.md) and
[validation/2026-09-11-n19-demitra-cc5-success.md](../validation/2026-09-11-n19-demitra-cc5-success.md).
These are retained exactly at that scope: **one title, one platform, one game build, named
fixtures**. `CUSA11126 / APP_VER 01.60` identifies the **game build**, not the emulator: the exact
shadPS4 executable version is **not recorded** anywhere in this repository yet (FR6 remains open). They are not layer-B evidence records, they carry no coverage key, and they promote **no**
cell — not `nhl19-ps4/assets-heads/*`, and nothing on any other title.

### C6 — Source schedules and localization

Source schedule editor: audit where each title obtains its initial Franchise/Dynasty and Season
schedule, how team/league identities resolve, and whether the source is read directly, transformed,
or generated during mode creation. Both editing/importing existing schedules and rule-based generation
are required (Q03a.1). Define import formats and generation rules through the question brief, including
team/league structure, games per team, home/away balance, matchup distribution and calendar constraints.
Desired league restructuring (Q03a.1a): determine and change team, division and conference sizes.
The user identifies this as requiring deep reverse engineering of game binaries and database
structures that has not yet been done. Track an explicit discovery workstream: locate structural
definitions and engine limits, distinguish database-driven values from binary constraints, and
verify impacts on mode initialization, schedules, standings, playoffs and save persistence per target.
Do not assume a schedule edit can change league topology. **Q03a.1b decision: league restructuring
is a stretch goal; its research and implementation do not block release.** Required schedule editing,
import and generation operate within each target's verified league structure. **Q03a.1c decision:
provide both normal season length and custom games-per-team counts where the target supports them.**
Default to the normal count; validate custom counts against proven format and runtime constraints.
Test season completion and playoff transition where applicable in addition to initial schedule
loading; reject unsupported counts before installation. Any binary investigation stays within authorized
game material and this repository's clean-room rules.

Use a shared validated preview for imported, manually edited and generated schedules. Generation
must report unsatisfiable constraints without installing partial output; preserve reproducible inputs
and any random seed. Implement the agreed controls, friendly team references, staged writes,
source installation and restore. Cover dates, home/away assignments, game counts, duplicate/conflicting
fixtures and format-specific constraints as established by evidence. If source replacement cannot
control initialization on a target, record the blocker and discuss an alternative; do not silently
substitute in-save editing for the requested workflow.

Schedule QA: source parse/edit/serialize/reopen, import round trips, generator constraint checks,
reproducibility, unsatisfiable-input rejection, unrelated-data preservation and backup/restore;
create a fresh Franchise/Dynasty and a fresh Season where supported and compare their resulting
schedules with the intended source changes. Play/simulate through edited fixtures, save and reopen
to verify stability. Check that installing a new default does not unexpectedly rewrite existing
careers. Per-target new-game initialization is the acceptance gate, not merely opening a source file.

Localization: discover locale databases; browse/search by resolved meaning/key; edit the value bound
to an existing hash with encoding and placeholder validation; expose pool usage; reject overflow
safely; show every affected reference before staging; preserve the hash and propagate its updated
value to every reference without cloning or rewiring; stage/undo/redo/save/reopen. Confirm whether
pool-fit only or verified expansion is required and whether import/export and multi-language
propagation belong in the agreed tool. Acquire missing
NHL19/21 PS4 localization fixtures or establish an evidence-backed non-applicability decision.

Arena asset editing is a stretch goal and does not block the initial release. The existing Arena
database fields remain part of C1 database editing and QA. A later integration with the community
arena editor is the intended path for crowd textures, power rings, scoreboard animations and center
ice logos. Define its interchange, version compatibility, validation and install contract when that
integration enters active scope. Arena audio remains governed by the general audio scope in C7.

QA: localization pool boundaries, duplicate/shared hashes, propagation to every reference, immutable
hash identity, Unicode policy, placeholders, UI length, font glyph support in game and unchanged
entries. Arena database fields retain their
normal database QA coverage; arena art/presentation asset rows are explicitly stretch scope.

Exit: agreed source schedule and localization operations implemented and verified, with no unexplained
“localization discovered” status standing in for editable support. Arena asset rows are classified as
stretch scope and excluded from this exit gate.

### C7 — Audio production tools

1. Audit menu music, in-game audio, play-by-play and PA bank/index layouts for each required target;
   identify codecs, alignment, slot IDs, duration/sample/channel constraints and rebuild rules.
2. Prove both decode and encode; settle runtime/codec packaging from actual requirements, including
   availability, desktop compatibility and distribution notices. Existing plan names are candidates,
   not proof that every encoder exists or can be bundled.
3. Implement bank/slot browsing with meaningful names, playback, export, source import, conversion
   preview, replacement, validated bank rebuild, install and restore. Add ID/reference editing only
   where required by the agreed scope and supported by evidence.
4. Automatically convert imported audio to the destination bank's verified sample rate and channel
   layout, with a preview of the converted result. Make loudness normalization an explicit user
   choice and preview its adjustment before staging. Reject imports when no verified conversion
   exists. Preserve the destination slot's identity, cue linkage, ordering and existing loop metadata.
   The initial release replaces clips in existing slots only; it does not edit playlist order or loop
   points and does not add cues or slots. Define remaining behavior for length mismatch and unsupported
   banks. Leave waveform authoring outside current scope unless explicitly added.

QA: unchanged and changed audio, decoded content checks, index/offset/alignment integrity, automatic
sample-rate/channel conversion, normalization enabled and disabled, size and duration boundaries,
preservation of existing cue/order/loop metadata, distortion/clipping, channel mix, cancellation,
unrelated slots, and
real event-triggered playback for music/PBP/PA. Hearing a clip in Studio does not verify its game cue.

Exit: each required audio family has working preview/export/replace/install and per-target in-game
playback evidence; unavailable encoders are resolved blockers rather than hidden export-only parity.

### C8 — Historical/Legends finish and complete desktop workflows

Historical/import: verify corpus completeness claims, provenance/versioning, name quality, missing
data, era normalization and target-specific scales. Test identity matching, overwrite/create,
duplicate-season policy, registry/cache recovery, preview==commit, batch rollback, capacity, undo
and reimport. Itemize omitted target fields. Provision the roughly 800 MB corpus on demand through
the versioned component system without an accidental development-directory dependency.

**On-demand components (Q44):** keep the base Windows installation small and download the historical
corpus, canonical asset libraries and optional conversion runtimes when a workflow first needs them.
Resolve exact versions through manifests, verify size, cryptographic hash and publisher signature
before activation, and never leave a partial download visible as an installed component. Support
progress, cancellation, resumable downloads, retry, configurable cache storage and safe cache
relocation. Retain installed versions for offline reuse and pin the exact dependency versions in
Projects, packages and Builds so old outputs remain reproducible. Component updates install side by
side until dependent Projects are migrated or explicitly repinned; a failed download or unavailable
network leaves Project state unchanged and reports the required component and recovery action.

Legends: settle franchise lineage, peak season/career selection, era eligibility, roster composition,
duplicate policies, ranking/ties, goalie handling and manual overrides. Show selection reasons and
computed-rating provenance. Prove cache invalidation on corpus/formula/rule changes and route final
rosters through the same validated import pipeline. Test empty/small franchises and boundary cases.

Onboarding: first-run platform/title selection, path validation, install/save discovery, archive
extraction with progress/cancel/resume policy, metadata/localization resolution, source versus output
locations, and clear recovery from absent or unsupported inputs. Default into Standard mode and take
the user directly through target-native save/database editing. Developer Mode is explicitly
enabled and launches its own guided tutorial; its state and tutorial progress are remembered without
locking users out of Standard mode.

Projects: create/open/rename/duplicate/archive multi-target overarching workspaces; preserve working files,
formulas, import/mapping/library state, sidecars, validation, transaction history and reproducibility
metadata. Provide history inspection and recovery with clear storage and migration behavior. For each
target, generate a user-approved reconciliation plan covering structural, schema and asset differences;
version approvals and invalidate them when inputs change.
Allow promotion from the active Standard session: preview the canonical lift, preserve staged edits,
history and source provenance, retain unmapped target-native details, then create the Project
transactionally without changing the opened source files.
Support blank canonical Projects, verified import from existing game/database/save sources, and
cloning an existing Project into an independent lineage. Run all three through the same canonical
validation and target-preview contracts.
Provide staged-versus-committed status and a Project commit review. Require a validated immutable
Project revision before target reconciliation can be finalized or a Build generated; let users select
an earlier revision without discarding newer work.

Canonical editor: edit the target-neutral Project model directly. Import existing game data and
historical/library content through source adapters without making any imported game's schema the
Project schema. Preserve canonical fields/assets that some targets cannot represent. Target preview
uses the complete DB schema and asset profile to show mapped output, automatic conversions, omitted
properties, capacity/identity conflicts and blockers before approval.
Provide the Invalid Values tab for each target preview with filtering, entity navigation, inline
editing, multi-select, Clear to Defaults, undo/redo and before/after counts. Revalidate incrementally
after each resolution and refuse Build generation while blocking rows remain. Store resolutions as
target mapping overrides without mutating canonical values; show canonical and override values together.

Builds: generate named target-specific deployable game-file sets from a specific Project revision and
approved reconciliation plan through the target's versioned database and asset adapters; record target,
adapter versions, input/output hashes and validation/QA state; validate, compare, activate/install, export/package and
restore them. Handle dirty Project state, interrupted generation/copy, disk-space checks and external
file conflicts without confusing Project history with deployed artifacts. Portable settings/path
relocation applies to both layers. Specify and finish bulk operations (blank templates,
global field changes, career clear/rebuild, asset backfill) and data exports after inventorying what
exists.

**Portable package exports (Q43):** support both package types. A canonical Project package carries
the editable target-neutral model, canonical source assets, Project revision history, target mappings,
approved overrides and versioned dependency manifests needed to resume or branch the work on another
Studio installation. A target-specific change package is bound to one exact game/platform target and
carries changed payloads or deterministic deltas, compatible base-file hashes, adapter/schema
requirements, approved reconciliation decisions, install destinations and rollback metadata. Importing
a change package validates its base installation and dependencies, then generates a new immutable
Build; the package itself is not treated as an already-installed Build. Both formats require explicit
format versions, content hashes, corruption/incompatibility rejection, reproducible round trips and
clear reporting of external dependencies. CSV and other data exports remain separate utilities rather
than substitutes for either package.

**Build immutability (Q07b):** after successful generation, a Build's deployable files, manifest,
target, source Project revision, approved reconciliation, adapter versions and hashes are immutable.
Regenerating after any change creates a distinct Build revision/identity, even when it shares a user-
facing series name. Installation state, annotations and archival state may change as metadata without
altering the artifact. QA evidence binds to the exact Build identity and hashes. Comparison, rollback
and reproduction use immutable revisions; deleting or pruning artifacts is an explicit lifecycle
operation and never rewrites history to make another Build appear equivalent.

**Build source revision (Q08):** Build generation accepts only a committed Project revision; it never
captures an uncommitted or staged workspace snapshot. Here, “commit” means an immutable revision in
Studio's Project history, separate from a Git commit. If pending changes exist, the user reviews and
commits them or selects an earlier committed revision before reconciliation and generation. The
Project commit records the unified change set, validation result, parent revision, timestamp and user
description. Build manifests reference its immutable revision ID/hash. Committing Project changes does
not write to an installed game; deployment happens only through the subsequent Build workflow.

UX: session header/undo/redo/save/discard, one unified pending review for DB/save/asset changes,
dependency and file-impact disclosure, capability explanations,
keyboard navigation, focus, screen-reader labels, contrast, narrow-window layouts and actionable
errors. Test Standard and Developer Mode navigation independently: Standard contains no Projects/Builds
prerequisite, while Developer Mode makes target/reconciliation/build state unambiguous. Set measurable
load/scan/import/preview/memory budgets from representative hardware and data.

Exit: a fresh user can install and edit a standard save without entering Developer Mode. After the
tutorial, a user can create/manage a canonical Project, create/import a roster, customize supported
assets, export/import canonical Project packages, exchange target-specific change packages, reconcile
targets, generate named Builds, install, restore and switch Builds without developer intervention. Historical and Legends outputs
also pass the target game campaign. No dead-end placeholder remains for an agreed production tool.

### C9 — Full game QA qualification campaign

Start after C0 with available database paths; extend as each feature clears its automated gates.
The first pilot should use the accessible target with the broadest fixtures (Legacy Xbox-derived
is a candidate), then PS3, early gen7/v1, gen6, PS4 variants including NHL15, and Recompiled. Final
coverage is every applicable exact target, not one test per generation.

For each feature/target:

1. Select hashed baseline fixtures and record Studio build/source identity plus runtime/game build.
2. Execute the actual UI operation, inspect the preview/pending changes, save, and reopen in Studio.
3. Install into a recoverable test copy through the supported deployment path.
4. Load the game; verify changed values/assets and unaffected control cases. Exercise the behavior:
   roster/lines during play, animation for meshes, actual announcement triggers for speech.
5. Play/simulate enough to generate a new game save; reopen it in Studio and verify persistence and
   integrity. For install-only assets without saved state, record why the save check is not applicable.
6. Restore the original and verify recovery. Record steps, expected/actual results, hashes,
   screenshots/video/audio as appropriate, tester, date, case ID, defects, and evidence path.
7. Promote only the proven cell/operation and conversion path. A parser check or third-party tool
   opening a file cannot substitute for game/runtime acceptance.

Maintain child cases beneath broad family rows so one scalar edit cannot qualify all roster
operations, and one portrait cannot qualify jerseys. Reuse a session to collect multiple cases, but
retain separate results. The assistant prepares fixtures/cases, automates checks and consolidates
evidence; the user or designated tester performs unavailable hardware/runtime interactions.

Exit: all agreed applicable write cells have current game-verified evidence; read-only features have
their appropriate read/UI gates; non-applicability has a reviewed reason; no unresolved critical/high
defect. No missing-fixture/blocked/experimental cell is hidden by an overall green title label.

### C10 — Release engineering and final acceptance

1. Enforce fast, anchor-read, anchor-edit, container, asset and manual-required CI suites; generated
   DTO/schema drift; publish machine-readable results and the support matrix. Shard costly suites.
2. Treat corrupt/truncated/untrusted file handling, archive path containment, allocation limits,
   external process arguments, and filesystem rollback as concrete robustness tests for this toolkit.
3. Build installers from an isolated source snapshot with all required base resources. Test clean
   install, upgrade, config/sidecar migration, uninstall preserving user work, first-use component
   downloads, interruption/resume, corrupt-package rejection, cache relocation, later offline use of
   cached components, paths with spaces/non-ASCII text, missing optional components, restricted
   permissions, and low disk space.
4. Provide application update checking, package download, updater-signature/hash verification and installation
   within Studio. Notify the user of an available version and show release notes, download size and
   compatibility/migration warnings. Require explicit user approval before downloading or installing;
   allow dismissal and deferral. Safely checkpoint or close active work, apply via a controlled
   restart, migrate compatible settings/Projects and roll back or recover after failure.
   Test interrupted downloads, invalid signatures, insufficient space, files in use, elevation needs,
   failed migration and restart recovery.
5. Defer paid Windows Authenticode signing for the initial release; it does not block completion.
   Continue to sign update packages with the project-controlled Tauri updater key and publish hashes,
   which adds no certificate fee. Protect and rotate that private key through the release process,
   reject invalid update signatures, and document the expected Windows unknown-publisher warning.
   Track Authenticode as a future distribution improvement if funding becomes available.
6. Release NHL Modding Studio's owned source code under the MIT License. Add the standard license text
   with the correct copyright holder and align repository, Rust, npm/Tauri and packaged metadata. Audit
   Studio-owned code, third-party dependencies and tools, fonts, historical data, canonical/community
   asset libraries and other downloadable components separately. Ship required copyright, license and
   attribution notices; exclude user-supplied and copyrighted game content from the Studio code
   license; make README/resource statements match what each package actually contains and may
   redistribute.
7. Publish user workflows, recovery/troubleshooting, compatibility matrix, limitations, release notes,
   fixture/test instructions and a reproducible release report. Publishing itself requires the user's
   release instruction; this roadmap does not authorize deployment or commits.
8. Run a final packaged-app regression on supported Windows versions and repeat impacted game cases
   after the release build's last relevant change. Confirm there are no untracked runtime dependencies.
   Track Linux and macOS ports separately as stretch work; they do not block the initial release.

Exit: releasable artifacts and acceptance report satisfy the agreed scope; packaging/recovery tests
pass; all relevant automated checks and current manual gates are green across the entire agreed
project. No partial or subset release is permitted under Q02; unfinished required work blocks release.

## 4. Dependency and scheduling rules

Critical dependencies: C0 -> C1/C2 -> broad database acceptance; C0 -> C3 -> C4/C5/C6/C7 asset install
qualification; C1 target codecs -> C8 import acceptance; all agreed feature gates + C9 -> C10 sign-off.
C9 starts early and continues throughout. C10 CI enforcement starts at C0; packaging experiments
start when Blender/corpus delivery is decided. Localization and audio discovery can proceed while
mesh work is blocked, and database QA should never wait for new asset UI.

Recommended delivery order: stabilize and start game QA; close save blockers; finish shared asset
writes and 2D workflows; finish mesh/appearance/equipment; localization; audio;
finish shell/historical/Legends integration; complete the exact-target campaign and release gates.
Reorder independent work based on available fixtures, not by silently relaxing verification.

## 5. Continuous QA and defect policy

| Layer | Required coverage | Trigger |
|---|---|---|
| Fast | Formatting, lint, types/DTOs, focused units, frontend build | Every implementation change |
| Anchor read/edit | Capability/field dispositions, codecs, scalar and relational mutations | Changes to schema, metadata, domain, validation, serialization |
| Saves | Every framing/mode, fidelity, semantic edits, atomic recovery | Save/filesystem changes |
| Assets | Codec and archive variants, reference integrity, conversion, restore | Asset/mesh/audio changes |
| UI/end-to-end | Actual commands and workflows, invalid inputs, dirty state, accessibility | Affected surfaces and packaged candidates |
| Manual game | Exact target, operations, install, persistence, behavior, restore | Initial promotion and relevant implementation changes |
| Distribution | Install/upgrade/on-demand components/cache/offline reuse/migration/recovery | Packaging/runtime changes and candidates |

Keep anchors immutable and run mutations on copies. Use byte equality for lossless no-op contracts;
use documented semantic equality and tolerances for recompression/resampling/conversion. Verify
unrelated data preservation in both cases. A deliberate model conversion is not exempt from QA
because it cannot be byte-identical.

Every defect records severity, exact target/build/hash, operation, expected/actual, reproducer,
evidence, owner and regression case. Critical = corruption/data loss; high = wrong edit or unusable
required workflow; medium = misleading/missing surface; low = cosmetic/documentation. Critical/high
defects block promotion and demote affected support. Medium/low issues need explicit disposition;
they cannot hide missing agreed functionality.

Manual evidence is invalidated by changes to the relevant parser, schema/metadata, codec, edit path,
validation, conversion recipe, package writer or runtime topology. Keep the old evidence as history,
mark it stale, and rerun impacted cases. Shared-path changes require review of every dependent target;
adjacent titles do not inherit a pass.

## 6. Completion tracking and immediate next steps

Runtime inventory update (user-confirmed 2026-09-05):

| Installed runtime | Installed titles | Shipping target count |
|---|---|---|
| Xenia | NHL07–15 and NHL Legacy | 10 |
| RPCS3 | NHL08–15 and NHL Legacy | 9 |
| shadPS4 | NHL15–21 | 7 |
| Legacy Recompiled | Legacy Recompiled | 1 |

Installed coverage spans all 27 shipping game/platform targets. This count is distinct from the
27 database anchors, which include the NHL07 PS3 research copy and lack an independent Recompiled
anchor. The user also confirms that every listed installation boots into playable gameplay and
creates saves. Baseline runtime availability is therefore user-confirmed across all 27 targets.
Exact runtime builds, deployment paths, and acceptance of Studio-edited outputs still need recorded
evidence during C0/C2/C9. Baseline playability does not promote any write path to game-verified.

Track each deliverable with: ID, scope/target, implementation state, decision IDs, dependencies,
fixture/runtime owner, automated cases, manual cases, evidence freshness, defects and exit gate.
Report implemented, automated-proven, and game-verified separately; do not invent a single completion
percentage from the 392 ledger rows.

Next (direction update 2026-09-16): follow the
[parallel completion plan](parallel-completion-plan-2026-09-16.md). Studio completes working
features from established findings while the converter and Markdown-only game research proceed
separately. First bound the existing source baseline and known multi-chunk codec defect, then the
shared transaction/archive path and a complete Texture Lab slice. Registry population accompanies
the consuming implementation. Evidence-infrastructure activation resumes at the RC QA transition (C9).
Scope-independent correctness work can proceed while later tool designs are discussed.
For Ignite textures, begin C3a by registering the reviewed tester hash and copied NHL19 PS4 EB/GNF
fixtures, reproducing fit/relocate/undo behavior with byte-diff receipts, and resolving size/integrity
field semantics before connecting the streaming writer to Texture Lab. Track C3a archive writes and
C4 GNF mip/conversion work separately; both gate completion of this required workflow.
