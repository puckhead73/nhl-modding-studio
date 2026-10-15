# DB-03 — Advanced Player Editing

Status: **PASSED — user sign-off, 2026-10-07**, for the supported NHL07 Xbox360 existing-player assignment workflow. User confirmed “DB-03 passees” after the review handoff. Generic/bespoke classification and complete gear/audio file lookup remain outside this acceptance; other titles/platforms require separate qualification. See the [acceptance record and implementation evidence](../validation/db-03-nhl07-review-2026-10-07.md). Next work package: DB-04 Advanced Team and League Editing.

First qualification target: **NHL 07, Xbox 360, existing players in the default game database**. MS-01 through MS-03 and DB-01/02 have user sign-off for this target. Other games/platforms require their own qualification. Save Editor is separate.

## Intended result

A mod author can select an existing player, change their supported equipment and appearance assignments using understandable controls, review/revert those edits in Project Overview, save/reopen the project, and build/install the changes. Assignments must produce the intended in-game result before they are called verified.

The package uses existing game assets. Creating, importing, replacing or converting asset files remains AA-01/AA-03/AA-05 and AUD-01. Basic confirmed appearance values and generic/bespoke selection belong here where NHL07 supports them; full hair-card editing, mesh/texture authoring and cross-title appearance conversion do not.

## Starting evidence

- The player page already has an Equipment tab and generic field-editing infrastructure. Existing code is a starting point, not DB-03 qualification.
- The field view supports early-title equipment fields sourced from `ubPc`, as well as later-title `xmja`/`fMVl` companions. NHL07 must use its actual source records.
- The committed NHL07 metadata includes stick, skates, blocker, helmet/mask, gloves/trapper, visor/pads, tape/tuck/pad options, custom-equipment flags, `artid`, `headid`, `audioid`, `defaultaudioid`, skin tone and separate left/right eye colors.
- Names such as `helmetbrandorgoaliemask`, `glovesortrapper` and `visorcolororgoaliepads` indicate role-dependent meanings that must be checked separately for skaters and goalies. Metadata alone does not establish the complete value vocabulary or runtime effect.
- Several asset references are deliberately hidden from the generic Player Data editor. Their dedicated editing routes and shared-reference behavior must be audited before exposing new controls.
- Existing equipment filename rules cover other title formats. NHL07 file matching must be demonstrated against its own game source, including packed assets.

## Implementation order

| Step | Work | Completion evidence |
| --- | --- | --- |
| DB03-01 — Field and asset inventory | Inventory NHL07 player equipment, appearance and presentation fields; resolve source records, storage ranges, enum meanings, skater/goalie differences, defaults and shared references. Trace which files or database rows each supported assignment selects. | A field/support matrix distinguishing confirmed, unresolved, absent and dependent-on-later-work features. No later-game vocabularies assumed valid. |
| DB03-02 — Skater equipment | Complete selectors for existing sticks, gloves, helmets, skates and supported visor, tape, tuck and pad options. Use NHL07 labels and applicable controls only. | Writes hit the correct player record; named options remain visible while editing; invalid selections are refused; unrelated attributes remain unchanged. |
| DB03-03 — Goalie equipment | Complete selectors for existing masks/helmets, pads, blockers, trappers and other confirmed goalie options. Resolve the custom-equipment flags and any required companion assignments. | Goalie meanings are independently qualified; linked fields change atomically; skater controls and labels do not leak into goalie editing. |
| DB03-04 — Appearance assignments | Support existing generic/bespoke head selection and confirmed skin/eye settings. Determine the actual selector/sentinel behavior and necessary dependencies before offering a toggle. Surface only appearance parameters proven meaningful for this title. | Head selection preserves coherent supporting values; supported settings can be saved/reverted; unknown combinations are not presented as working presets. Full appearance authoring remains AA-05. |
| DB03-05 — Portrait and audio assignments | Provide selection of existing portraits and player commentary/name audio. Establish the relationship between art identity, portrait lookup, active audio and default audio before exposing edits. Prefer known asset or donor-player labels over unexplained ID boxes. | Correct reference changes without unintended re-identification of the player or overwriting a shared file. A preview is included only where the existing decoder works; unavailable preview is reported accurately. |
| DB03-06 — Editing and reference checks | Resolve known choices from the correct game/source; distinguish Available, Missing and Not verified. Explain sharing where it affects the edit. Reuse immediate feedback, validation rollback and queued writes. | Missing source/unextracted content is not mislabeled as a missing asset. Existing unknown values remain preserved. Verified invalid references are refused; uncertainty is explained without inventing labels. |
| DB03-07 — Project integration | Group edits beneath the player in Overview, with readable before/after values and editor links. Extend × reverts for any linked DB-03 assignments, keeping them atomic. Verify Undo/Redo, Save Session, autosave, reopen, developer package export/import, build, activation and deactivation. | Saved and packaged state matches the intended edits; reverts do not leave half-restored assignments; original assets and game source remain unchanged by Studio editing. |
| DB03-08 — Qualification and handoff | Complete automated checks on disposable NHL07 projects, repair failures, prepare a controlled review project/build and exact expected results. Deliver one consolidated user verification run. | Implementation recap, automated results, outstanding native checks, and concrete player/option instructions. Do not claim a visible/audible result from database round trips alone. |

## Verification performed before user review

1. Read/write the confirmed field set for representative skaters and goalies, including known defaults and boundary/refusal cases.
2. Verify role-dependent labels and values against NHL07 evidence; preserve unknown existing values without silently remapping them.
3. Exercise assignment dependencies, shared references and missing/unavailable source cases. Check references in packed content when applicable.
4. Test immediate UI feedback, failed-write rollback, serialized Save/Undo behavior and navigation while edits complete.
5. Test atomic Undo/Redo and Overview × for simple and linked assignments, both before and after save/reopen; preserve unrelated edits.
6. Round-trip the developer project archive and confirm all assignments survive import.
7. Build/install into disposable game copies, compare the resulting database changes and verify deactivation restores the original bytes.
8. Check that saves, unrelated players, equipment records and source files are unaffected except for explicitly required dependent writes.

## Single-pass user verification plan

The final handoff will replace these categories with exact players, option names, original values and expected results selected from confirmed NHL07 data. There are no prerequisite manual experiments requested by this roadmap.

| Check | User action | Expected result |
| --- | --- | --- |
| Skater equipment | Change one test skater's supported gear and equipment options. | Controls are named correctly and update promptly. |
| Goalie equipment | Change a test goalie's mask, pads, blocker and trapper where supported. | Correct goalie options; no skater labels or unintended linked changes. |
| Head/appearance | Assign a known head and exercise the confirmed generic/bespoke and skin/eye controls. | Choices and dependencies remain coherent in the editor. |
| Portrait/audio | Assign known existing portrait and commentary/name choices. | The intended assignments appear, with accurate availability and sharing information. |
| History/reverts | Undo/redo an assignment, then use × on an Overview edit while another edit remains. | Linked values revert together; unrelated edits remain. |
| Persistence/sharing | Save, close/reopen, export a developer package and import it as a separate project. | Assignments, details and Overview state are preserved. |
| Native visuals | Build/activate, restart NHL07, restore default rosters and inspect the selected skater/goalie in menus and gameplay. | Equipment, head and applicable appearance settings match the supplied expected results. |
| Native portrait/audio | Open the relevant player screens and trigger the selected player's commentary using the supplied scenario. | The intended portrait and audio are used. If audio cannot be triggered, record it as unverified, not passed. |
| Restore | Deactivate, restart and restore default rosters. | Original assignments and presentation return. |

## Dependencies and deferred acceptance

| Area | DB-03 responsibility | Later work |
| --- | --- | --- |
| Portraits/heads | Assign existing compatible assets and qualify their use. | Import/export/replacement, mesh/material/rig editing: AA-01. |
| Equipment | Select existing supported gear and confirmed parameters. | New gear, textures, custom designs and model authoring: AA-03. Team-wide colors: AA-02. |
| Appearance | Existing head associations and confirmed basic parameters. | Full Appearance Studio, hair/facial-hair systems, Card A/B changes and cross-game conversions: AA-05. |
| Audio | Select existing player audio and qualify the actual reference. | Export/replacement/editing and full PBP tools: AUD-01. |
| Historical/player creation | Maintain reusable assignment handling. | Historical appearance/equipment packages: DB-05. New player records/slots: DB-08. |
| Overview | Database assignment summaries and coherent database reverts. | Asset-file, archive and structural-record reverts remain separate work. |

DB-03 closes for NHL07 Xbox360 once its supported assignments pass automated and user verification, with unresolved or absent features explicitly recorded. A feature needing later tools cannot be marked passed; any deferral discovered during implementation must be named in the handoff. NHL07 has no PS3 qualification target.

Execution agreement: when implementation is authorized, work through this package and all available verification before handing over one consolidated review. Request user action only when progress genuinely depends on it, with a concise, specific instruction.
