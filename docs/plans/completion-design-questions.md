# Completion design questions — one at a time

> **27 September beta scope update:** [October 1 beta decisions](beta-20261001-scope-decisions.md) govern the named beta exclusions and bundled XMA-helper distribution. League and team creation/cloning/deletion and executable patching are outside the beta; executable patching remains final-release scope. Preserve the rest of the agreed requirements and distinguish beta readiness from final completion.

> **17 September 2026 supersession:** the [entity-editing parity roadmap](entity-editing-parity-roadmap.md) is authoritative for current sequencing and revised scope. The Project/Build decisions below, including PB-D1–PB-D9 and canonical Project portions of Q06–Q08/Q43, are preserved for final optional stretch implementation, not active prerequisites or release gates. Shared transactional safety remains required for direct edits. Named arena/ice/presentation operations in the new roadmap supersede their previous blanket stretch exclusion. The earlier foundation specifications are archived behind their existing links.

Prepared 2026-09-05. Companion to [the completion roadmap](project-completion-roadmap.md).
These are queued design decisions, not a request to answer the whole document now. No recommendation
below is an accepted requirement until discussed. Existing explicit requirements remain in force.

## Discussion procedure

Ask only the next unresolved question. Record the answer, its acceptance criteria and affected
milestones, then revise subsequent questions to avoid asking something already answered. Expand
technical follow-ups only when that answer requires them. Keep a decision log at the end. Do not
block unrelated implementation on a later question, and do not treat silence as a scope decision.

For each tool, the resulting brief must specify: user goal, inputs, supported targets, workflow,
preview and edit controls, outputs/install behavior, failure/recovery behavior, exclusions and
automated/manual acceptance cases. Prove format behavior from fixtures; user answers define product
intent, not binary-layout facts.

## Settled repository contracts — do not ask again

- Player asset slugging is implemented in `tdb-core::dev_key`: `lastname_firstname_dd_mm_yyyy`,
  lowercase ASCII transliteration, name punctuation/separators removed, complete zero-padded date.
- User-facing identities are resolved names; raw IDs stay behind details.
- Headless Blender is bundled; remaining work is packaging/licensing/layout and harness completion.
- Portrait matching uses the universal slug with explicit review for ambiguity. Logo generation emits
  the target game's required variants.
- Existing full-kit/clone/assignment workflows and verified game font atlases are the baseline;
  reusable kit presets and custom font import are not implicit release requirements.
- Localization's defined release surface is individual string editing within verified pool-fit
  constraints. Strings remain keyed by their existing hashes: editing updates that hash's value for
  every reference, with no cloning or reference rewiring. Pool growth and bulk translation are not
  implicit scope.
- Audio previews, exports and replaces existing menu/in-game/PBP/PA slots. It does not add cues/slots,
  and user-facing identities follow the repository's human-readable-surface rule.
- Legends eligibility, franchise assignment, three-best-season composite, roster capacities,
  residual teams and deterministic tie rules are implemented operator specifications.
- Planned bulk tools are blank templates, global field set, career clear/rebuild and asset backfill.

## Completion boundary and QA access

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q01 (answered through Q01a–Q01b) | Which game/emulator environments can we use for actual-game QA today, including exact titles/platforms and whether edited files can be installed and saved again? | The runtime/title inventory and baseline boot/play/save capability are confirmed. Edited-output acceptance remains an execution QA gate for every exact target. |
| Q02 (answered) | Should we release verified subsets along the way while retaining the full catalog as the completion target? | Decision: no release until the entire agreed project is complete. Verified subsets are internal QA milestones only. |
| Q03 (answered) | Which save modes must full completion support beyond roster saves? | Required: Franchise, Season, normal roster files and settings. Be A Pro and other game modes are stretch goals, not release requirements. |
| Q03a (answered) | Which Franchise/Season data must be editable beyond the existing roster/player/team tools? | Source schedule editing/import/generation is required. No further mode-specific editing is required at this time. |
| Q03a.1 (answered) | Should the schedule tool edit/import existing schedules, generate new schedules from rules, or support both? | Both are required, using the source-default workflow and shared validation/preview. |
| Q03a.1a (answered through follow-ups) | Should schedule generation use the loaded game's team/league structure and season length, or also support custom team counts, alignments and games per team? | League restructuring is stretch scope per Q03a.1b. Both normal and supported custom season lengths are required per Q03a.1c. |
| Q03a.1b (answered) | Must custom league sizes block release, or should they remain a research/stretch goal while required schedule tools operate within each game's verified league structure? | Decision: stretch goal; league restructuring does not block release. Required schedule workflows use verified league structures. |
| Q03a.1c (answered) | Should generated schedules allow a custom number of games per team where supported, or preserve each game's normal season length? | Both: normal length and a custom games-per-team option where supported. Validate custom counts through mode initialization, season completion and playoff transition where applicable. |
| Q03a.2 (answered) | Beyond source schedules, which additional Franchise/Season-specific controls are required? | None at this time. Existing player/team/roster tools remain required; standings, accumulated records, finances and other live mode state are outside release scope. |
| Q03b (answered) | Which settings must be editable for completion? | Every setting that can safely be identified. Use evidence-based per-target metadata; unknown/ambiguous values remain read-only and guessed meanings do not qualify. |
| Q04 (answered) | Must save output be directly deployable to original consoles, or are documented emulator/decrypted-file and external repack workflows acceptable for some platforms? | Original-console packaging is required alongside emulator/decrypted workflows. Cover Xbox 360, PS3 and PS4 packaging and verified deployment; an inner payload alone is insufficient. |
| Q05 (answered) | Which desktop operating systems must the finished application ship on? | Windows is required for the initial release. Linux and macOS are stretch goals and do not block it. |

## Shared editing and project workflow

**Additional accepted foundation decision (PB-D9):** use SQLite `project.db` with unique Studio-specific identifiers independent of source-game IDs. Final output IDs/slots are resolved at Build execution separately for each target, permitting the same canonical entity to occupy different native slots across games. Preserve source provenance, rewrite dependent references through the output allocation map, and retain that map in each immutable Build. See the foundation roadmap for allocation and validation requirements.

**Additional accepted foundation decision (PB-D8):** Develop New Project opens a separate empty Project environment without selecting a game. Users browse onboarded game databases/assets in familiar source-specific views, onboard additional games in place, and send leagues/teams/players/assets into persistent Project-owned data. Team import prompts for identity/assets only or rostered players plus team/player assets. Sources also include the Historical DB and Studio-native data/assets. Closing or switching source games preserves all imported Project content. See the foundation roadmap's PB-D8 contract and mixed-source PB3 acceptance gate.

**Additional accepted foundation decisions (PB-D5–PB-D7):** vanilla backup, Project and generated Build paths are independently configurable. Custom assets are copied into Studio-managed storage with originals untouched; external changes require explicit reimport. Multi-target graduation completes successful targets and reports blocked/failed targets with reasons, retaining progress for targeted retries. See the foundation roadmap for acceptance requirements.

**17 September 2026 foundation decisions:** [Projects and Builds foundation roadmap](projects-builds-foundation-roadmap.md) records PB-D1–PB-D4. Edit one game-independent Project with entry/asset count ceilings based on the maximum supported across games per type. Select destination game/platform pairs at graduation and review inclusion/exclusion and compatibility separately for each target. No named Project variants are requested. Initial game onboarding captures an immutable vanilla backup for target file-change attribution. Implement managed-installation Build switching/restoration first. Saves/settings use separate profiles per mod, scoped to game/platform and user/account, with explicit reuse between compatible Build revisions. Qualify isolation per target and expose manual steps where necessary. Existing canonical data, immutable history and target-specific Build requirements remain in force.

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q06 (answered) | Should database edits and asset replacements be reviewed and applied together as one project change set? | Yes. One pending review and coordinated transaction spans databases, saves and assets, with dependency-aware validation and full recovery on failure. |
| Q07 (answered) | What must a named Build snapshot preserve? | Split the concepts: a Build is a named deployable game-file set; a Project is the overarching editable workspace and history that produces Builds. |
| Q07a (answered) | Can one Project produce Builds for multiple game/platform targets, or is each Project bound to one target? | A Project can map to multiple targets. Each target requires user-reviewed reconciliation/approval for differences such as league/team counts, attributes and bio fields. |
| Q07a.1 (answered) | What is the Project's authoritative data model: a target-neutral canonical roster/asset model, or a designated primary game Build whose data maps outward to other targets? | Use a target-neutral canonical Project model. Comprehensive per-target DB schemas and asset profiles drive automatic output conversion; target-absent fields are omitted and reported for approval. |
| Q07a.2 (answered) | How should a new canonical Project be created: blank, imported from an existing game/save, cloned from another Project, or all three? | All three. Each path produces the same validated canonical model; imports retain provenance/unmapped source details and clones receive independent identity with ancestry. |
| Q07a.3 (answered) | How should canonical values invalid for a present target field be resolved? | Provide an Invalid Values tab with inline edits and an automated Clear to Defaults action. Use only verified target defaults; preview changes and block unresolved values. |
| Q07a.3a (answered) | Should fixes in the Invalid Values tab apply only to that target's export mapping, or update the canonical Project value too? | Apply only to that target's Build mapping. Canonical values remain unchanged; Project edits happen separately. |
| Q07b (answered) | Are Builds immutable outputs, or can regenerating a named Build update it in place? | Builds are immutable. Regeneration creates a distinct revision/identity; QA and deployment records bind to exact hashes. |
| Q08 (answered) | Before generating an immutable Build, must pending Project edits be committed as a named Project revision, or may Studio generate from a captured pending snapshot? | Edits must be committed to Studio's Project history. Builds cannot capture an uncommitted snapshot. This is separate from Git. |
| Q08a (answered) | Should a Standard-mode editing session be promotable into a new canonical Project without losing its current edits? | Yes. Promotion preserves staged edits and source provenance, previews canonical mappings/unmapped data, and leaves original files untouched. |
| Q08b (answered) | Should the user-facing name be Advanced mode or Developer mode? | Developer Mode. Use this consistently in navigation, settings, onboarding, tutorials and documentation. |

## Texture Lab

Existing intent: cache index, decode preview, used-by lookup, replace-from-PNG, bulk export.
Codec work exists; the route is a placeholder. The route's hint also mentions audio, which conflicts
with the dedicated audio design and should be corrected when the boundaries are settled.

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q09 (answered) | Is Texture Lab primarily a browser/converter/replacer, or must it include image editing such as painting, compositing and channel manipulation? | Browser, converter and replacer only. Inspection is supported; painting, compositing, retouching and channel authoring are outside scope. |
| Q10 (answered) | Should texture replacement enforce original dimensions/format, or allow compatible resizing/reformatting when the target supports it? | Enforce original target dimensions and format. Higher-resolution installed textures are a stretch research goal requiring separate qualification. |
| Q11 (answered) | Should batch replacement match exported paths/manifests automatically or use a manual assignment queue? | Manual one-at-a-time assignment/replacement for the initial release. Automatic matching and batch replacement are stretch goals. |

## Portrait and logo import

Existing intent: ranked surname+DOB matching, fragment fallback, manual browse, per-target format
conversion, and generated logo variants. These are not proposals to restart the completed decoder.

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q12 (answered) | What filename convention identifies player-specific source assets? | Portraits and headmeshes use the universal slug `lastname_firstname_dd_mm_yyyy`. Both resolve through one canonical player identity parser/index. |
| Q14 (answered) | Should imported portraits have standardized crop/background treatment, or preserve the supplied artwork? | Sources are already normalized. Apply no crop/background treatment; only scale to the destination game's dimensions and encode its required format. |

## Uniforms and nameplates

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q16 (answered) | Must curved nameplate baselines generate installable output for completion, or is the supported feature set limited to straight layouts? | Curved nameplates must produce installable output and match the editor preview in-game. |

## Appearance and canonical headmeshes

Settled direction: game headmeshes are provided in one consistent canonical database, supplemented
by Character Creator 5 / Headhost 3 generated meshes in the same exported format. Studio automatically
converts any canonical entry into every supported game's native output. Existing NHL24/CC5 paths and
the source-target matrix are implementation evidence for target adapters, not separate user workflows.

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q23 (answered) | Are hair/facial-hair grafting and texture baking required production tools, or are catalog selection and enum edits sufficient? | Hair and facial-hair components are supplied in a canonical database for selection. Studio converts/installs them; grafting and baking authoring tools are not required. |

## Hockey Shop and arena assets

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q25 (answered) | Must Hockey Shop convert equipment across games, or is same-target import/clone/install sufficient? | Required release scope is same-game Hockey Shop workflows. A canonical cross-game library remains the intended stretch architecture. |
| Q25a (answered) | Must canonical cross-game equipment conversion for every supported target block release? | No. It is a stretch goal, independently gated by per-slot/target UV and in-game qualification. |
| Q27 (answered) | Which arena assets should be editable beyond the current Arena database fields? | Arena asset editing is stretch scope. A future community-editor integration is intended to cover crowd textures, power rings, scoreboard animations and center ice logos. |
| Q28 (answered) | Does arena editing need a dedicated arena-centered preview/workflow, or can Texture Lab provide the required experience? | No in-Studio arena asset workflow is required for the initial release. Defer workflow design until the community arena editor integration enters active scope. |

## Localization

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q31 (answered) | Should changing a shared string update all references automatically, or allow an independent copy where the format supports it? | Update the value attached to the existing hash. Every reference to that hash receives the change; do not clone strings or rewire references. |

## Audio

Existing requirement: preview/export/replace for menu, in-game, PBP and PA; no waveform authoring.
The codec and bank-layout spike is still required before implementation estimates are trustworthy.

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q35 (answered) | Should Studio automatically convert sample rate/channel layout and normalize volume, or reject incompatible imports for external preparation? | Automatically convert to the target bank's verified sample rate and channel layout. Loudness normalization is an explicit user choice. Reject before staging when no verified conversion exists. |
| Q37 (answered) | Should music workflows include playlist order and loop settings, or only clip replacement? | Replace audio in existing slots only. Playlist ordering and loop-point editing are outside the initial release scope. |

## Historical, Legends, bulk tools and delivery

| ID | Question to ask | Recommendation / effect |
|---|---|---|
| Q43 (answered) | Which exports and mod-sharing outputs must users be able to produce? | Support both canonical Project packages for editable project exchange and target-specific change packages for compact distribution and reproducible Build generation. |
| Q44 (answered) | Should the historical corpus and large runtime components work entirely offline after installation, or be provisioned separately? | Download large corpora, canonical asset libraries and optional conversion runtimes when first needed. Cache installed versions locally for later offline use. |
| Q46 (answered through Q46a–Q46e) | What installer/update/signing and project-license choices should define the release? | In-app user-approved updates, no-cost updater signing, deferred paid Authenticode, and an MIT open-source release define the initial distribution. Audit bundled dependency/resource notices against actual contents. |
| Q46a (answered) | Should application updates run inside Studio or require manually downloaded installers? | Provide update checking, download, verification and installation within the app. |
| Q46b (answered) | Should available application updates install automatically, or require the user to start installation? | Notify the user and require explicit approval before downloading and installing the update. |
| Q46c (answered) | Must the Windows installer, application executable and update packages be code-signed for release? | Defer paid Windows Authenticode signing so the initial release has no certificate/service cost. Retain the no-cost project-controlled Tauri updater signature and published hashes for update integrity. |
| Q46d (answered) | Should the Studio source be released under an open-source license, or remain proprietary/all-rights-reserved initially? | Release NHL Modding Studio as open-source software. Select the exact license separately and audit every bundled dependency, corpus and asset under its own applicable terms. |
| Q46e (answered) | Which open-source license should cover the Studio-owned source code? | Use the standard MIT License for Studio-owned source code. Keep third-party components, data and assets under their own terms. |

## Decision log

Q01 runtime availability answered: installed runtimes, title coverage, playable gameplay and save
creation are user-confirmed for every shipping target. Exact builds/paths and edited-output
deployment and acceptance remain execution checks in C0/C2/C9, not assumptions of support.

| ID | Agreed decision | Acceptance criteria | Roadmap changes | Date |
|---|---|---|---|---|
| Q01 (partial) | User confirms Xenia, RPCS3, shadPS4, and Legacy Recompiled are installed. | Record exact builds, installed game titles, deployment paths, and successful load/save checks before selecting qualified test targets. Installation alone is not game acceptance. | C0/C2/C9 runtime inventory now has four confirmed installed environments. | 2026-09-05 |
| Q01a | Xenia: NHL07–15 and Legacy. RPCS3: NHL08–15 and Legacy. shadPS4: NHL15–21. Legacy Recompiled is also installed. | Installed coverage is confirmed by the user; verify boot, usable gameplay, save creation and edited-file deployment separately for each target. | C0/C2/C9 can plan against all 27 shipping game/platform targets, including Recompiled; no NHL07 PS3 research copy is included. | 2026-09-05 |
| Q01b | User confirms all listed installations, including Legacy Recompiled, boot into playable gameplay and create saves. | Treat baseline runtime availability as user-confirmed. Record build/path details during setup and collect exact edited-output acceptance evidence during the QA campaign; no game-verified ledger promotion follows from baseline playability. | C0/C2/C9 have no reported baseline boot/play/save blocker across the 27 shipping targets. | 2026-09-05 |
| Q02 | No release until the entire agreed project is complete. | Every required feature and target passes its implementation, automated QA, manual game acceptance where applicable, and release gates before release. | Roadmap scope and C10 prohibit subset releases; staged verification remains internal. | 2026-09-05 |
| Q03 | Required save targets are Franchise, Season, normal roster files and settings. Be A Pro and other game modes are stretch goals. | Required modes must pass implementation and QA gates wherever present; inventory older Franchise-equivalent mode names per title. Define actual Franchise/Season/settings editing controls in Q03a/Q03b. | C2 includes required multipart and non-TDB settings work; optional modes do not block release under Q02. | 2026-09-05 |
| Q03a | Franchise/Season scope is the existing player/team/roster tools plus source schedule editing, import and generation. No further mode-specific editing is required at this time. | Edited source schedules initialize correctly in new modes; required existing editors work against supported mode saves. Standings, accumulated records, finances and other live state are excluded. | C2/C6 scope is bounded; in-save schedule and other mode-state editors are removed from the release gate. | 2026-09-05 |
| Q03a.1 | Both editing/importing existing schedules and generating new schedules from rules are required. | Both routes produce validated source schedules used by newly created Franchise/Dynasty and Season modes; generator rules and import format remain to be designed. | C6 includes both workflows and generator-specific QA. | 2026-09-05 |
| Q03a.1a (partial) | User wants to determine and change league team/division/conference sizes, but identifies deep game-binary and database reverse engineering as an outstanding prerequisite. | Establish target-specific feasibility and structural constraints before promising custom topology. Release-gate status and season-length customization remain undecided. | C6 tracks league-structure discovery separately from schedule generation; no assumption of feasibility or automatic scope exclusion. | 2026-09-05 |
| Q03a.1b | League restructuring is a stretch goal. | Custom team/division/conference sizes and their reverse engineering do not block release; required schedule editing/import/generation use verified league structures. | C6 separates optional restructuring from required schedule tools; custom season length remains unresolved. | 2026-09-05 |
| Q03a.1c | Both normal season length and custom games-per-team counts where supported are required. | Offer both options; validate custom counts against target format/runtime constraints, including season completion and playoff transition where applicable. | C6 includes both length modes and rejects unsupported counts before installation. | 2026-09-05 |

| Q03a.2 | No additional Franchise/Season-specific editing is required at this time. | Existing player/team/roster tools plus source schedules define required functionality. | Standings, accumulated records, finances and other mode-state editing do not block release. | 2026-09-05 |

| Q03b | Every setting that can safely be identified is required. | Maintain an evidence-backed per-target field disposition; identified settings use semantic controls and pass round-trip plus in-game effect checks. Unknown/ambiguous values remain read-only. | C2 includes metadata-driven TDB and non-TDB settings editing rather than a fixed category subset. | 2026-09-05 |

| Q04 | Original-console packaging is required. | Produce console-deployable outputs for Xbox 360, PS3 and PS4 required workflows, including applicable outer metadata/integrity and documented unavoidable transfer/signing steps; verify on original hardware. Emulator/decrypted outputs remain supported too. | C2 retains STFS and adds PS3/PS4 console-package discovery, implementation and original-hardware QA. | 2026-09-05 |

| Q05 | Windows is the required initial desktop platform; Linux and macOS are stretch goals. | Installer, bundled runtimes, filesystem/recovery behavior and packaged-app QA pass on supported Windows versions. Cross-platform support is not claimed without its own qualification. | C5/C7/C10 target Windows packaging; portable seams remain desirable, but Linux/macOS do not block release. | 2026-09-05 |

| Q06 | Database edits and asset replacements use one pending-changes review and coordinated project transaction. | Validate all temporary outputs before installation; detect source conflicts; apply with backups and a durable journal; restore the complete pre-apply state or expose resumable recovery after failure. | C3/C8 unify review, application and recovery across DB/save/asset files. | 2026-09-05 |

| Q07 | Builds contain the actual deployable game files; Projects separately contain working project files and history. | Every Build has a file/hash manifest linked to the exact Project revision and target. Projects preserve editable state, formulas, import/mapping/library state, sidecars, validation and transaction history needed to reproduce Builds. | C8 separates Project management from Build generation/deployment and defines both workflows. | 2026-09-05 |

| Q07a | Projects can map to multiple game/platform targets, with manual user reconciliation and approval for target differences. | Each target Build shows mappings, transformations, omissions, conflicts and blockers for league/team structure, capacities, fields/enums/scales, relationships and assets. Approval is versioned and invalidated when relevant inputs change. | C8 adds target adapters and a required reconciliation gate before deterministic Build generation. | 2026-09-05 |

| Q07a.1 | The target-neutral canonical Project model is authoritative. Target export ignores target-absent/invalid fields and automatically converts supported data and assets using comprehensive game/platform schemas and output profiles. | Registry covers physical encodings and semantic transforms, including offsets, ranges, enums, relations and asset container/format rules. Omissions/lossy conversions are reported in the required reconciliation review; present-but-invalid values require an explicit conversion rule or block. Generated Builds are deterministic and do not mutate canonical data. | C0 adds canonical/schema/adapter architecture; C1 completes DB semantics; C3–C7 complete asset converters; C8 becomes the canonical editor and target export pipeline. | 2026-09-06 |

| Mode split | Projects/Builds are an opt-in Developer Mode with a dedicated tutorial. Standard mode remains the default direct save/roster editing experience. | A new user can open/edit/save a normal target-native file without creating a Project. Developer Mode's first-entry tutorial covers canonical data, reconciliation, conversion, Builds, deployment, history and recovery and is repeatable. Both modes share validation and transactional safety. | C8 onboarding and navigation split Standard from Developer Mode; Projects/Builds do not complicate the default path. | 2026-09-06 |

| Q08a | A Standard-mode editing session can be promoted into a canonical Project with its current edits preserved. | Snapshot baseline and staged state; preview mappings, target-native extensions, ambiguity and missing assets; create transactionally without modifying the source; retain provenance and history. | C8 adds a Standard-to-Project promotion workflow and fidelity test back to the source target. | 2026-09-06 |

| Q08b | The user-facing name is Developer Mode. | Use “Developer Mode” consistently across navigation, settings, onboarding, tutorials, errors and documentation. | C8 terminology is settled; no mixed Advanced/Developer labels. | 2026-09-06 |

| Q07a.2 | New canonical Projects support blank creation, import from an existing game/database/save, and cloning another Project. | All paths produce the same validated canonical model. Imports preserve source hashes, provenance and target-native extensions while reporting ambiguity; clones have independent identity and recorded ancestry. | C8 implements and tests all three creation workflows. | 2026-09-06 |

| Q07a.3 | Target reconciliation has an Invalid Values tab with inline editing and an automated Clear to Defaults action. | List every invalid mapped value with entity, field, value, target constraint and reason. Automated resolution uses only verified target defaults, previews all deltas, supports undo/redo and leaves unknown-default rows blocking export. Target-absent fields remain a separate omissions list. | C8 adds the tab and target-validation workflow; schema adapters must carry verified defaults and provenance. | 2026-09-06 |

| Q07a.3a | Invalid-value corrections apply only to the selected target's Build mapping. | Preserve canonical data unchanged; store target overrides with prior value, resolution method and schema/adapter provenance; revalidate/invalidate overrides when inputs change. | C8 separates target reconciliation overrides from canonical Project edits. | 2026-09-06 |

| Q07b | Builds are immutable outputs. | Files, manifest, target, Project revision, reconciliation, adapters and hashes cannot change after generation. Regeneration creates a new Build identity/revision; mutable installation/annotation state remains separate. | C8 Build history, comparison, deployment rollback and QA evidence bind to immutable revisions. | 2026-09-06 |

| Q08 | Pending Project edits must be committed before Build generation. | Studio Project commits are immutable validated revisions with parent, description and change-set identity; Build manifests reference the exact revision hash. This is unrelated to Git commits. | C8 adds Project commit review and prohibits Builds from staged workspace state. | 2026-09-06 |

| Q09 | Texture Lab is a browser, converter and replacer only. | Support discovery/search, metadata/channel/mip inspection, preview, export, verified conversion, replacement, install and restore. Exclude image painting, compositing, retouching and channel authoring. | C4 scope is bounded around asset IO and target conversion. | 2026-09-06 |

| Q10 | Texture replacements preserve the original target dimensions and encoded format. | Converters may adapt source interchange images into the exact existing specification with preview; validate mip/channel/compression/swizzle/atlas constraints. Higher-resolution output is experimental stretch scope and cannot block release. | C4 removes variable installed texture specifications from release scope and adds isolated high-resolution research. | 2026-09-06 |

| Q11 | Texture replacement is manual and one-at-a-time for the initial release; automated matching and batch replacement are stretch goals. | Required UI resolves the exact target and affected references, previews source and converted output, and stages one replacement safely. Future batch support remains experimental until matching and atomic rollback are proven. | C4 removes batch replacement from the release gate while retaining bulk export. | 2026-09-06 |

| Q12 | Player-specific assets, including portraits and headmeshes, use `lastname_firstname_dd_mm_yyyy`. | One shared parser/index resolves the full name and birthdate to canonical player identity; preserve original filenames; malformed, duplicate or conflicting slugs require review. | C4/C5 share player-asset identity rather than separate portrait and head matching systems. | 2026-09-06 |

Queue audit (2026-09-06): Q12a, Q13, Q15, Q17, Q18, Q22, Q24, Q26, Q29, Q30,
Q32–Q34, Q36, Q38–Q42 and Q45 were removed because repository contracts, implemented behavior or
existing scoped plans already answer them. Do not reopen them without conflicting evidence or an
explicit request.

| Q14 | Portrait sources are normalized before Studio receives them; target conversion only scales to destination dimensions and encodes the native format. | Preserve canonical source unchanged; no crop, pad, reframe or background/style processing. Reject sources outside the canonical aspect/dimension contract and verify scale, alpha/color and target variants. | C4 portrait conversion becomes a deterministic scale-and-encode adapter. | 2026-09-06 |

| Q16 | Curved nameplate baselines must produce installable output. | Bake/encode the edited curve and game-font layout into the verified target representation; support staging, undo and re-edit; verify 2D/3D preview fidelity and in-game rendering across representative curves/names. | C4 promotes curved baselines from preview-only experiment to a required production write path. | 2026-09-06 |

| Q19–Q21 | Headmeshes from every game are provided in a consistent canonical database; CC5/Headhost 3 generated meshes use the same exported format. Any canonical entry must export automatically to every supported game. | Replace user-facing pair selection with canonical-head -> target adapters. Fully describe and verify each target's mesh, rig/weight, UV/texture, companion, ID/container and install output; test representative game-origin and generated entries per target. | C5 makes all supported target adapters release requirements and treats current pair/path work as reusable implementation evidence. | 2026-09-06 |

| Q23 | Hair and facial-hair assets are provided in a canonical database for selection. | Validate the supplied component catalog, preview compatible selections, and automatically convert/install them through every supported target adapter. Exclude in-Studio grafting and texture baking authoring. | C5 replaces graft/bake scope with canonical component selection and target conversion. | 2026-09-06 |

| Q25 (partial) | A canonical cross-game equipment library is the intended design, contingent on full UV-conversion testing. | Preserve canonical item sources/provenance; build independently gated slot/target adapters; preview UV/texture results and refuse unverified conversion. Whether this blocks release remains Q25a. | C5 expands Hockey Shop architecture while keeping its completion boundary explicit. | 2026-09-06 |

| Q25a | Canonical cross-game equipment conversion is a stretch goal. | Same-game browse/preview/store/import/delete/clone/assign/install/recovery remains required. Cross-game adapters stay experimental until each slot/target passes UV, texture and in-game gates. | C5 removes complete cross-game equipment coverage from the release gate while retaining the canonical architecture. | 2026-09-06 |

| Q27–Q28 | Arena asset editing is a stretch goal; integrate a community arena editor later. | Keep existing Arena database fields in required database scope. Defer crowd textures, power rings, scoreboard animations, center ice logos and the dedicated arena workflow to the future integration. | C6 and the QA plan exclude arena art/presentation assets from the initial release gate. | 2026-09-06 |

| Q31 | Localization strings remain linked to their existing hashes. | Edit the value for the selected hash and show all affected references before staging. Do not create independent copies or change reference hashes. | C6 treats hash identity as immutable and verifies propagation to every reference. | 2026-09-06 |

| Q35 | Audio imports automatically conform to the destination bank's verified sample rate and channel layout; loudness normalization is optional. | Preview the converted result and any normalization adjustment before staging. Refuse imports when a verified target conversion is unavailable. | C7 includes automatic format conformance and an explicit normalization control in the import pipeline and QA matrix. | 2026-09-06 |

| Q37 | Audio editing replaces clips in existing slots only. | Preserve each slot's existing identity, cue linkage, ordering and loop metadata. Do not expose playlist ordering, loop-point editing, new cues or new slots in the initial release. | C7 narrows authoring and QA to safe replacement of existing audio slots. | 2026-09-06 |

| Q43 | Support both canonical Project packages and target-specific change packages. | A Project package carries the editable canonical model, source assets, Project history, target mappings and dependency manifests. A change package binds changed payloads/deltas, base-file compatibility hashes, reconciliation decisions and installation metadata to one exact game/platform target; Studio validates and applies it to generate a new immutable Build. | C8 makes both package formats required, versioned, integrity-checked and round-trip tested. Existing data exports remain separate utilities. | 2026-09-07 |

| Q44 | Large corpora, canonical asset libraries and optional conversion runtimes are downloaded when needed. | Resolve components through versioned manifests, verify size/hash/signature before activation, support resumable download and configurable cache storage, and preserve installed versions for offline reuse and reproducible Builds. Missing network/components produce an actionable prompt without damaging Project state. | C8/C10 add an on-demand component manager and test first-use, interruption, corruption, update, rollback, cache relocation and subsequent offline operation. | 2026-09-07 |

| Q46a | Application updates are handled within Studio. | Check a versioned update feed, present release information, download and verify signed packages, close or checkpoint active work safely, apply through a restart flow, migrate compatible state and recover or roll back after failure. | C10 requires a qualified in-app update path rather than manual installer-only upgrades. | 2026-09-07 |

| Q46b | Studio notifies the user and asks before updating. | Show the available version, release notes, download size and compatibility/migration warnings. Do not download or install until the user approves; preserve active work and allow dismissal or deferral. | C10 makes updates user-initiated while retaining the complete in-app download/install path. | 2026-09-07 |

| Q46c | Paid Windows Authenticode signing is deferred. | Do not incur certificate or signing-service costs for the initial release. Sign updater packages with the project-controlled Tauri key, publish hashes, document the expected Windows unknown-publisher warning and keep Authenticode as a future distribution improvement. | C10 removes Authenticode from the initial release gate while retaining cryptographically authenticated in-app updates. | 2026-09-07 |

| Q46d | NHL Modding Studio will be open source. | Publish the Studio-owned source with a recognized OSI-approved license and the required copyright/license files. Inventory third-party code, tools, fonts, corpora and assets separately; include their notices and do not imply that user-supplied or copyrighted game content is covered by the Studio code license. | C10 makes open-source release and a complete distribution-rights audit part of final acceptance. | 2026-09-07 |

| Q46e | Studio-owned source code uses the MIT License. | Add the standard MIT license text with the correct copyright holder, mark package metadata consistently and retain complete third-party license/notice separation. | C10 replaces the unresolved license choice with MIT and adds consistency checks across release artifacts. | 2026-09-07 |
| Q47 | **Legacy Recompiled reuses the Legacy Xbox 360 asset profile.** The user explicitly confirms (2026-09-15, C0-REG5 §C) that Legacy Recompiled's asset files are identical to Legacy Xbox 360's and directs that the same asset profile be reused rather than a second one written. | Accepted as the architectural direction, **attributed to the user; not a measured fact**: no file hash, member identity or corpus is invented for Recompiled, and no `byte-identical-to-archive-member` bridge is declared (that method needs a recorded comparison). Each asset family's source-side blocks (identity, container, compression, path families, resolution chains, variants) are to be **shared by reference** from the `nhllegacy-xbox360` target profile, never duplicated. **Kept separate and Recompiled-specific:** destination installation paths and roots (`discovery`, `install`), deployment behaviour, `runtime_lookup`, and every qualification / evidence cell — none inherits from Legacy Xbox 360, and Recompiled's own native filesystem/deployment path (roadmap §1) remains what is qualified. Database equivalence is **not** inferred from this decision (FR4: no Recompiled anchor; the database registry keeps `legacyrecomp-recomp` absent). The external headmesh repository is not inspected. | Registry: the asset-profile contract gains the smallest follow-up that makes the reuse executable — an optional per-target `reuses = "<target id>"` reference on `[[target_profile]]` (validator: the referenced profile is declared in the same file and attributable; the referencing profile may declare only the destination-side and qualification blocks; nothing operative inherits across the reference without its own scope) — recorded in [C0-REG5](../receipts/c0-reg5-early-generation-adapters.md) §C and **implemented by [C0-AS5](../receipts/c0-as5-shared-legacy-asset-profiles.md)** as `asset-output-profile/5` (`reuses` on `[[target_profile]]`; inheritance boundary fixed in `asset_reuse.rs`: shared = identity / formats / naming / variants / chains / operations, destination = discovery / install / runtime_lookup, never inherited; conflicts refused, not merged). `legacyrecomp-recomp` is registered by reference in all three committed families with every inherited block in its source state and every destination block unknown — an asset *registration*, not an asset support claim. Committed at `cc913f9` (C0-AS5-CLOSE, after correcting validation sequencing so cross-field checks run on the effective profile). | 2026-09-15 |

The scoped design-question queue is complete. Remaining unknowns are reverse-engineering findings,
fixture availability, implementation choices bounded by these decisions and manual QA results; track
those as execution evidence or blockers rather than reopening settled product questions.
