# Studio development roadmap — entity editing and cross-game parity

> **27 September beta scope update:** [October 1 beta decisions](beta-20261001-scope-decisions.md) govern the named beta exclusions and bundled XMA-helper distribution. League and team creation/cloning/deletion and executable patching are outside the beta; executable patching remains final-release scope. Preserve the rest of the agreed requirements and distinguish beta readiness from final completion.

Accepted sequence: 17 September 2026. NHL 21 Legends supplies the use cases; direct editing in each game supplies the execution path. Projects/Builds are deferred to a final optional stretch implementation, after the five phases and required product acceptance.

## Authority and scope change

This is the current feature-development sequence. It supersedes the Project/Build-first prerequisite and the earlier overhaul-led ordering. Preserve the detailed technical requirements in [project completion requirements](project-completion-roadmap.md) where consistent with this direction, and the implementation ownership in [ownership plan](parallel-completion-plan-2026-09-16.md).

Archived snapshots, copied and hash-verified before this change:

- [Previous overhaul roadmap — superseded](nhl21-legends-development-roadmap.superseded-2026-09-17.md).
- [Project/Build foundation roadmap — superseded, retained stretch design](projects-builds-foundation-roadmap.superseded-2026-09-17.md).

The canonical Project editor, SQLite project.db, Studio-specific IDs, multi-source Project composition, Project history, graduation, native Build-ID allocation, immutable Builds, Project/change packages and Build/profile switching are **stretch features**, not implementation prerequisites or release blockers. Preserve their code/research and accepted design for later; do not delete or rebuild them during direct-editing work. General parsing, field/asset mappings, validation, dependency handling, safe writes, backup/restore and source provenance remain required where used by active workflows. The old C0/C8 Project clauses do not override this change.

The requested arena details/ice textures, boards, scorebug and other named presentation work are active roadmap items. This supersedes the previous blanket arena-art stretch exclusion for these named operations. It does not implicitly add an unrestricted 3D arena authoring suite. Editing supported league structure is inventoried in Phase 1; arbitrary engine-level league expansion remains stretch unless separately approved.

Retain the existing 27 shipping title/platform targets, including independently tested Legacy Recompiled. NHL 07 PS3 is research-only. No Xbox One support or full NHL 24 editor is added. The product's no-partial-public-release policy remains, applied to the revised required scope rather than deferred Projects/Builds.

## Working loop — finish one step across games

**Latest developer campaign gate:** [Overnight developer first review / R1](../validation/overnight-development-2026-09-19-review-r1.md). 26 selected tests passed independently (one additional case ignored); reviewer probe reproduced loss of the preceding pending texture on undo. Correct archive-dialog async ownership/global summary and pending texture history before new feature work. The full campaign is not yet independently accepted.

**Latest campaign research review:** [19 September overnight research review](../validation/overnight-research-2026-09-19-review.md). Useful packaging/imagery/audio/Teams evidence delivered; placement generalizations, anomaly counts, runtime verdicts and search bounds need correction before consuming the proposed rules. Developer campaign completion is reported in its STATUS but has not been independently accepted by this research review.

**Latest research input:** [P1-BS-CSV reviewed; overnight addendum](../validation/p1bs-csv-review-and-overnight-addendum.md). CSV mappings and existing archive-planner capability are ready for consumption under the stated contract clarifications. Research reconciles those at its first overnight checkpoint, then advances to R2; developer still starts with settings decoding.

**Latest developer acceptance:** [P1-C-R2 accepted offline](../validation/p1c-r2-acceptance-2026-09-19.md), with 31 focused domain/API tests independently passed. The [overnight developer brief](overnight-development-2026-09-19.md) now starts at D1 scoped settings decoding; the [research brief](overnight-research-2026-09-19.md) continues CSV parity independently. Earlier P1-C correction banners below are historical.

**Latest settings acceptance:** [P1-BS-R1 accepted with coordinator clarifications; CSV parity research next](../validation/p1bs-r1-acceptance-and-csv-parity-dispatch.md). The NHL 15 stored-default contract is accepted, with 21 explicit setting IDs and exact checksum preservation. P1-C-R2 remains active; scoped settings decoding, reader and writer follow in sequence. Research can advance NHL 16–21 CSV defaults and shared packaging independently.

**Latest developer review:** [P1-C-R1 review; narrow P1-C-R2 correction](../validation/p1c-r1-review-and-r2-dispatch.md). Prior pairing, title-selection and project-switch regressions pass. One API case silently discards a conflicting division on single-group targets; correct that before the queued settings decoder package.

**Latest settings review:** [P1-B-SETTINGS review and contract correction](../validation/p1bs-review-and-contract-correction-dispatch.md). Catalogue evidence and the four-field parser finding are usable; research must correct the proposed writer's precedence and runtime verdicts. A scoped decoder fix is queued after P1-C-R1 closes; no second implementation lane.

**Latest research step:** [P1-B revision 3 accepted; settings-catalogue research next](../validation/p1b-r2-acceptance-and-settings-research-dispatch.md). The inventory and stock-save correspondence are accepted research inputs; rules/default editing and modified-source runtime behavior remain open. P1-C-R1 development is independent.

**Current research status:** [P1-B-R2](../validation/p1b-r1-review-and-r2-dispatch.md). Revised stock-save correspondence and NHL 15 PS4 settings/default catalogue provide useful leads. Coverage presentation, residual completeness and test verdicts need correction before consuming the proposed bindings. P1-C-R1 development continues independently.

**Latest developer gate:** [P1-C-R1 corrections](../validation/p1c-review-and-r1-dispatch.md). Eight focused PS4 cases and four UI cases pass, but independent probes reproduced invalid conference/division pairing and incorrect-title catalogue acceptance. Membership summary ownership also needs correction. P1-A remains accepted offline; P1-B research continues separately.

**Current developer step:** [P1-A-R2 accepted; P1-C dispatched](../validation/p1a-r2-acceptance-and-p1c-dispatch.md). The bounded offline schedule workflow is accepted with its documented limitations; runtime cards remain separate. Start a fresh developer session on PS4 competition groups while research completes P1-B-R1. This supersedes the P1-A correction status below.

**Latest research review:** [P1-B-R1](../validation/p1b-review-and-r1-research-dispatch.md). The inventory supplies useful observed structures and stock-save correspondences; edited-schedule survival and calendar causation are not established by those comparisons. League-rule editing remains research-blocked, not removed from scope based on possible storage location.

**Latest developer review:** [P1-A-R2](../validation/p1a-r1-review-and-r2-dispatch.md). Independent schedule tests passed (12 domain including private NHL 20, 1 API, 9 UI); a separate probe reproduced cross-workspace review-ID collisions. P1-A remains open for this bounded correction. Research P1-B continues independently.

**Research, 18 September:** [LOC-W3 revision 3 is accepted as an implementation contract](../validation/loc-w3-r3-acceptance-and-p1b-research-dispatch.md); growth implementation and runtime acceptance remain pending. Research advances independently to P1-B competition rules/mode initialization while developer P1-A-R1 remains active.

**Current package, 18 September:** [P1-A-R1 schedule workflow corrections](../validation/p1a-review-and-r1-dispatch.md). Initial normal-path tests pass, but independent review reproduced stale-preview overwrite; UI lifecycle corrections and remaining per-target offline coverage are required before P1-A closes. Same developer session; no live execution authorized by this review.

**18 September overnight execution:** follow the [two-agent research/development campaign](overnight-two-agent-campaign-2026-09-18.md). Two fresh Claude sessions may work through corrections and this roadmap without intermediate review stops, completing implementation parity at each numbered step while batching visual/manual acceptance for the user's return. This overrides older overnight scope/stop instructions for this run only; no pending visual gate may be reported as passed. Xbox 360 install discovery must include `X:\Games\Emulation\Roms\XBOX360`.

Codex leads, coordinates and reviews; Claude is the primary implementation developer. Follow the [working agreement](coordinator-developer-working-agreement.md) for package cleanup, plain-language receipts and session/prompt/status handoffs. The S2-C-R4 lifecycle correction and its review scratch cleanup are complete within the reviewed copied-target API scope. Begin [L1.1-A league identity mapping and existing-workflow assessment](../validation/s2c-r4-review-and-l11a-dispatch.md) in a fresh Claude session; this does not yet mark Phase 1.1 implemented or qualified across games.

For each numbered step below:

1. **Choose a concrete change.** Identify a named NHL 21 entity and a small intended before/after result, starting within existing entries/slots. Define all affected data, localization, assets and references before applying it.
2. **Inspect and map.** Confirm source title/platform/version, baseline hashes, relevant records/files and runtime lookup. Reuse existing metadata, codecs, editors and findings; investigate only missing facts needed by the change.
3. **Complete the Studio workflow.** Browse, preview, edit/import/replace, validate, stage, review, apply/save-as/repack and undo/restore using direct target-native sessions. No Project creation is required. Research scripts may establish facts, but do not count as finished UI functionality.
4. **Verify NHL 21.** Reopen the output, compare intended changes and unrelated-data preservation, install via the verified path, inspect/play in game and exercise save/reload where relevant. Restore exact baseline content, including removal of newly added files.
5. **Replicate the equivalent change in each applicable game/platform.** Use that target's own metadata, limits, formats and native IDs; preserve meaning rather than copying raw values or bytes. Early-generation, PS3, Xbox 360 and Recompiled differences receive explicit tests.
6. **Close the step.** Record per-target results, unresolved differences and user-visible support behavior. A representative target never certifies its neighbors. A missing fixture is blocked/untested, not not-applicable. Do not mark the step complete while applicable targets remain unresolved.

Default cadence is one active numbered step. If an external fixture/runtime dependency prevents closure, record the blocker and the decision to proceed before advancing; do not quietly accumulate a second unfinished campaign. Ready support work may continue without implying the blocked step passed. Planning documentation does not dispatch new implementation agents or change code ownership.

**Overnight authorization, 17 September 2026:** follow the [league overnight execution brief](league-overnight-execution-2026-09-17.md). L1.1-A is accepted as planning input with the corrections there. Claude may implement the league workflow and its specified localization-reader/archive prerequisites sequentially without intermediate user review stops or progress updates. Preserve explicit blockers and developer-versus-independent verification status; this does not close Phase 1.1 or authorize unrelated phase expansion.

Track title/platform, step, entity, source hash, field/file binding, operation, Studio result, structural validation, runtime result, save/reload, restore result and evidence location. Use the existing QA contract/status vocabulary rather than replacing it. Operation scope matters: replacing an existing asset does not prove adding/registering one; changing membership does not prove expanding a league.

## Phase 1 — Leagues

**Latest gate:** [R3 accepted; P1-A schedule workflow dispatched](../validation/overnight-dev-r3-acceptance-and-p1a-dispatch.md). Independent domain/API/UI checks passed; the correction dispatches below are historical. Start a fresh developer session for the next Phase 1 matrix package. Live image Write cards remain withheld for separately scoped execution. NHL21-E1 remains closed.

**Post-overnight review, 18 September:** Phase 1 implementation is partially delivered, not closed across targets. [OVERNIGHT-DEV-R2](../validation/overnight-2026-09-18-review-and-r2-dispatch.md) is active: correct post-write rollback accounting and image-plan/operation ownership, then reconcile the operation-by-target coverage. Schedule workflow, multi-format logo replacement/archive placement and league-audio consumer questions remain open; visual review is separately pending. Preserve the Phase 2.3 opening slice without expanding Phase 2 ahead of this gate. NHL21-E1 remains accepted.

**Coverage statement (R2 F3, developer, 18 September):** the operation-by-target status of every Phase 1 step — implemented + tested, implemented + test missing, not implemented, research unknown, evidence-backed not applicable, blocked; user review pending throughout — and the remaining work packages P1-A … P1-I that would close the phase live in [phase-1-coverage-matrix-2026-09-18.md](../validation/phase-1-coverage-matrix-2026-09-18.md). Active development priority is those packages, in order of the matrix, before any Phase 2 expansion. The overnight receipts' "Phase 1 implementation complete" wording is superseded by that matrix.

Current gate: NHL21-E1 is accepted by the user as a comprehensive pass for the tested identity edit, durability, consistency and restoration. No repeat is required. Phase 1.1 remains open across other targets. The coordinated parity completion queue is delivered; its [R1 lifecycle and integration corrections](../validation/league-parity-completion-review-and-r1-dispatch.md) are active before consolidated runtime review.

| Step | Work | Acceptance focus |
|---|---|---|
| 1.1 Identity data | Identify/edit league or competition names, codes, abbreviations and identity fields that exist in each target. Trace direct text versus localization references. | A controlled league identity change persists and appears on the intended surfaces; identifiers and references stay valid. Display text requiring localization is mapped here and completed in 1.3, not guessed as a literal DB name. |
| 1.2 Additional database entries | Inventory membership links, competition categories, divisions/conferences where exposed, rules, season/tournament/playoff settings, schedule sources and mode initialization. Distinguish fields, related records, and row creation as different operations. | Supported changes survive new-mode creation, schedule validation and progression. Required source schedule editing/import/generation is included within verified topology. Unsupported expansion or hidden engine limits are explicit. |
| 1.3 Localization | League names/abbreviations, competition labels and explanatory text; trace every affected consumer, key, token and supported locale. | Correct strings render in menus and relevant game screens without breaking shared references or formatting. |
| 1.4 Assets | League logos, selection tiles, textures, imagery and league-specific presentation graphics. Identify frontend versus in-game variants and shared fallbacks. | Preview, import/replace and install the correct variants; verify every scoped consumer and restore. |
| 1.5 Audio | League/competition-related announcements, presentation or music references where they exist. Identify shared banks and event bindings. | Replace/reassign a supported existing cue and trigger it in game; evidence-backed absence is recorded per target. |

## Phase 2 — Teams

| Step | Work | Acceptance focus |
|---|---|---|
| 2.1 Identity data | Team name/city/abbreviations/codes, colors, type and relevant identity/art references. | Intended changes are consistent across roster and selection views, preserving valid links. |
| 2.2 Additional database entries | League membership, affiliations/rivals, roster membership and player-index links, lines/depth charts, captains, numbers/retired numbers, strategies and supported team settings. | Roster and lineup operations remain coherent, including goalies, special teams, duplicate membership and capacity edges; play and save/reload succeed. |
| 2.3 Localization | Team/city names, short labels, scoreboard abbreviations and other dictionary-driven team text. | Long and short forms agree across menus and gameplay, with no unrelated shared-string damage. |
| 2.4 Assets | Logos/crests, menu imagery, team branding and relevant textures; asset-ID bindings and all required variants. | A team rebrand resolves correctly in frontend and gameplay after restart. |
| 2.5 Arena details and ice | Arena DB identity and team assignment; supported venue parameters; center-ice marks, rink/ice textures and associated asset bindings. Inventory further arena dependencies when encountered. | Correct venue and ice appear for the selected team; shared arena users are disclosed. In-game close/wide views and restore verified. Full arena geometry authoring is not implied. |
| 2.6 Jerseys, pants and kits | Home/away/alternate entries, ordering and selection; jersey/pant/sock components, fonts, numbers, nameplates, recoloring and kit-art/style/variant relationships. | Complete kits agree in previews and on ice, including goalie contexts and menu thumbnails. Qualify additional entries separately from replacements. |
| 2.7 Audio | Goal horns, team PBP/PA calls, arena music and other team-specific cue assignments. | Correct cues trigger for the edited team; shared banks, fallback names and timing/loop constraints are respected. |

## Phase 3 — Players

| Step | Work | Acceptance focus |
|---|---|---|
| 3.1 Identity data | Names, birth date, nationality/birthplace where supported, position, handedness and physical measurements. | Human-readable values agree with actual game display/behavior; per-title encodings are verified. |
| 3.2 Additional database entries | All supported skater/goalie ratings, styles/traits, overall consistency, contracts/draft/career data, player indexes, companion records, roster operations and supported animation/stance/celebration assignments. Include historical import, formula profiles and bulk editing through the same native workflow. | Imported/created/cloned/edited players retain coherent records and references. Recompute/preview agree; meaningful skater and goalie cases play and reload correctly. |
| 3.3 Localization | Player-related UI labels and any genuinely localized player text; distinguish direct biography strings and audio-name references. | Only appropriate consumers change; no fabricated assumption that every player name lives in a locale dictionary. |
| 3.4 Assets and appearance | Portraits, head/face geometry and textures, hair, eyebrows, facial hair, skin/eye appearance, relevant thumbnails and player-associated imagery. Integrate the independently developed head converter when ready. | Correct likeness appears in UI, close-ups and gameplay; imported heads/assets obey target bindings and rig/texture requirements. Missing conversion capability remains visible. |
| 3.5 Audio | Player name calls, PBP/PA references and supported related voice slots. | Intended name/cue triggers in the relevant contexts; unavailable names have explicit handling. New speech synthesis or arbitrary cue creation is not implied. |

## Phase 4 — Equipment

Apply the complete catalog → label → database assignment → geometry/material/texture → thumbnail → archive → runtime chain to each category. Existing entries first; qualify clone/new-entry registration independently. Same-game import/assignment/editing is required; arbitrary cross-game equipment conversion remains stretch.

| Step | Work | Acceptance focus |
|---|---|---|
| 4.1 Hockey sticks | Models, textures/recoloring, catalog options, handedness/orientation, blade/shaft tape and supported accessories. | Picker, player assignment, on-ice appearance and saved-player reload agree. |
| 4.2 Gloves | Models, textures, colors, branding, sizing/variants where supported. | Correct materials and assignments in preview and play. |
| 4.3 Helmets | Models, textures, colors, visors/tints, ear protection and applicable accessory relationships. | Accessory combinations and dependencies render correctly and persist. |
| 4.4 Skates | Models, textures, colors, guards and supported appearance options. | No mismatched catalog/asset IDs; proper preview and on-ice rendering. |
| 4.5 Goalie gear | Pads, blocker, catcher, mask/paint, goalie stick, neck protection and associated appearance choices. | Complete goalie set works; independent goalie-stick and model/paint bindings are established. |

## Phase 5 — Miscellaneous presentation

| Step | Work | Acceptance focus |
|---|---|---|
| 5.1 UI | General menu branding, textures/icons, fonts, prompts and supported layouts/navigation; identify compiled or executable dependencies. Include Legends unsupported-Franchise messaging and NHA guidance when implementing the overhaul. | Startup through play remains navigable, text/glyphs fit, and existing accessibility/settings controls remain usable. Novel UI logic is scoped explicitly before implementation. |
| 5.2 Boards | Board/glass-related imagery and rink-side advertising where present; arena-specific/shared variants. | Correct venue, camera, material and repeated/animated variants; no accidental global replacement. |
| 5.3 Scorebug | Overlay textures/layout/text/team-logo bindings, scoreboard values and relevant broadcast overlays. | Score, clock, period, penalties and team identity remain correct during play and replay contexts. Cosmetic editing does not imply rewriting gameplay logic. |
| 5.4 Menu music | Music slots/banks, codec/sample constraints, playlists/selection where supported, looping and volume behavior. | Track selection, transitions, looping and restart persistence work; replace/restore succeeds. |

Shared localization/audio/texture infrastructure is developed when first needed and reused in later phases. Repeating an entity category does not mean rebuilding the underlying editor.

## Requirements carried through every phase

- **Safe direct editing:** identified vanilla/source backups, change review, validation, atomic/recoverable file operations, external-change detection, save-as, archive packaging and exact restore. These do not depend on canonical Projects or immutable Builds.
- **Save and mode coverage:** roster, Season, Franchise/Dynasty/Be A GM and identifiable settings as applicable. Preserve the distinction between source DBs and mode-local copies. The Legends overhaul may exclude Franchise while Studio still supports its required edits.
- **Asset dependencies:** catalog registration, frontend/in-game variants, thumbnails, strings, shared assets, archive precedence and runtime lookup accompany each asset edit. No silent missing assets.
- **Add/remove operations:** inventory and qualify create/clone/delete separately where already in scope. Capacity rules and reference cleanup must be proven before adding records or assets; field editing alone does not prove expansion.
- **Bulk/historical tools:** retain source provenance, rating/formula correctness, batch preview, duplicate/identity review, capacity checks and rollback. Integrate these into team/player steps; defer canonical multi-source Project composition only.
- **Settings and gameplay behavior:** expose safely identified existing settings/rules with verified ranges. Arbitrary gameplay-code changes, new animations and new game modes are not implicitly included. Animation/stance assignment is distinct from authoring animations.
- **Application completion:** onboarding, discovery, source/library management, optional components, usable errors, cancel/progress, Windows packaging, performance and accessibility remain required acceptance work. Final cross-phase testing covers a fresh install, complete Season/tournament progression, long sessions, save/reload and recovery.

Potential presentation additions to inventory while working: loading screens, replay/wipe graphics, crowd branding and arena scoreboard animations. Record whether they fall within UI/team/arena work; do not promise cinematics/video editing or new animation authoring without a scope decision. Do not add a sixth content phase solely for this inventory.

## First work package — 1.1 League identity

**Goal:** identify and safely edit one existing league/competition identity in NHL 21, then reproduce the same semantic operation across every applicable shipping target.

1. Select a reversible existing league example from the NHL 21 Legends use case. Inventory its ID, name/code/abbreviation fields, localizable references and consumers; record fields that do not exist independently on that target.
2. Trace reads/writes in the existing Studio league/entity surfaces. Define which direct DB values can be changed now; retain stable identity keys for the first rename test. Reassigning IDs is a separate operation requiring complete reference mapping.
3. Produce a concise target matrix: native table/field, displayed meaning, encoding/limits, references, editable/read-only disposition, current UI, fixture/runtime availability and expected result. Use the [knowledge-gap register](builder-knowledge-gap-register.md) to source blockers, not as a canonical-builder prerequisite.
4. Complete the minimum Studio editing path and prove save/reopen and unrelated-data preservation. If an intended displayed name is exclusively localization-driven, record that dependency for 1.3; do not mark an invisible DB change as a successful displayed rename.
5. Test the available direct identity behavior in NHL 21, then each other target with a suitable equivalent league. Record exact values and evidence; restore after each test.

**Exit:** all applicable direct league identity fields in the agreed step have mapped semantics and a tested editing path, or explicit unresolved blockers. No claim of league resizing, new competition registration or localization completion is made by passing a DB identity edit. Advance to 1.2 after the step's target review.

## Final optional implementation — Projects and Builds

Only after the active editing campaign and required completion checks, revisit the archived foundation design: SQLite project.db, Studio-owned IDs, empty Project environment, mixed-source imports, target selection/reconciliation, native output-ID allocation, immutable outputs and managed profile switching. Reuse the field, dependency, asset and runtime evidence produced by Phases 1–5. This is stretch work and does not gate release of the revised required Studio scope.
