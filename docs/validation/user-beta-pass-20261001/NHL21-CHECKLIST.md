# NHL 21 PS4 — user checklist (current candidate r69)

Build update: r69 accepted for internal verification after receipt70; see R69-REVIEW.md. Earlier r67/r68 findings retain their original build identity. Complete focused onboarding/source/save reruns before continuing below. Opening writes a profile sidecar: use owned copies, including for remembered startup auto-open.

User requested a full batch checklist on 2026-10-01, superseding the one-step-at-a-time presentation preference. Startup after desktop-context relaunch is user-observed pass; install detection is not yet confirmed. NHL21-E1 remains accepted for its prior league identity/durability/restore scope and is not repeated here.

Report each ID as PASS, FAIL, BLOCKED, MISSING or NOT TESTED, plus relevant observations. Do not mark a whole group passed when only one sub-operation worked. Use subletters for split results. Missing UI is an observation, not automatically proof of technical non-applicability. Screenshots needed only for ambiguity/errors/visual defects.

All items below are new N21 batch tests, adapted from the roadmap and documented workflows. Existing source cards are credited where used; no invented existing-card instructions. UI control names are only prescribed where documented. If a function is absent, record that rather than hunting for hidden controls or bypassing a refusal.

## Setup and common edit procedure

- **A01 — Identify the target.** Record NHL21 PS4 install path, input database/save path, title ID and patch if known. Check Studio identifies NHL21/PS4, with readable player/team/league names. Report dictionary warnings verbatim.
- **A02 — Establish copies before editing.** Record untouched source, dedicated working-copy destination and baseline/restore method. Database-only work needs its DB and relevant metadata/dictionary; asset writes need the affected native files/archives in a copy. Do not edit an attached original install. Do not duplicate the whole install by default. If the required file set or destination is uncertain, complete read-only checks and mark dependent writes BLOCKED for follow-up.
- **A03 — Choose records.** Prefer Sidney Crosby, Marc-André Fleury, Pittsburgh, NHL and one non-NHL league; substitute records actually present. Record baseline values. Use a small change: one rating point, one pound, one character, or another listed existing asset. Keep game-ready outputs separate from intentionally invalid diagnostic copies.
- **A04 — About/licences.** Inspect About & Licences and open two listed licence texts. Expect internal-review status, MIT-holder/artwork limitations and populated third-party notices (V-82).
- **A05 — Navigation.** Search/filter, switch players/teams/leagues, return to a prior record and resize the window. Expect correct selection/data, readable labels, no stale previews or clipped essential controls. Record keyboard/focus issues.
- **A06 — Missing inputs.** Using a disposable DB-only copy without attached assets/dictionary, note missing-source messages. Expect honest missing/unavailable information, not invented names/previews. Do not move or rename original or bundle files.

For every editable item: record before/after; inspect Pending; apply; test undo and redo; save to a new owned copy; close/reopen; verify the change and one nearby unaffected record; restore. Record these outcomes separately if any differ. Do one feature per output until coordinated saving is tested. Never save unknown IDs or negative-test values into a runtime candidate. Stop the affected route for lost edits, wrong-target writes, corruption or failed recovery; retain its copy/backups.

## Core lifecycle

- **B01 — Stage/cancel.** Stage Crosby's weight +1 lb, inspect the correct named change, then cancel/reject. Expect no retained change or disk write.
- **B02 — Apply/undo/redo.** Repeat that change. Each action should affect only the intended field and restore/reapply the exact value.
- **B03 — Save/reopen.** Save a new copy and reopen. Changed value persists; original input and a neighboring player remain unchanged. Record output path.
- **B04 — Unsaved changes.** Make another temporary edit, then attempt a record/session switch and close. Check how unsaved changes are handled; cancel leaving first, then deliberately discard. Expect no silent loss.
- **B05 — Range/string refusal.** Try one above the displayed maximum and text beyond the displayed limit on a disposable copy. Expect clear refusal, no wrapping/clamping or partial change.
- **B06 — Combined changes.** After single edits work, combine one player and one team edit; inspect both, undo/redo and save/reopen. If later adding a dictionary/asset edit, report that transaction separately—DB-only success does not qualify it.
- **B07 — Restore.** Reopen the baseline and verify original values. Check restored asset/archive identities when those routes are eventually used. A successful undo is not by itself disk restoration.

## Leagues and schedules

- **C01 — Identity/membership display.** Browse NHL and a non-NHL league; verify names, abbreviations, member teams and exposed groups. Prior NHL21-E1 rename remains accepted; do not redo it.
- **C02 — Groups/relationships.** On a copy, move one selected team between valid exposed groups, inspect all related changes, then restore. Invalid conference/division pairing must refuse or require correction.
- **C03 — Localization.** Change one separate league label through the supported dictionary workflow, using a one-character suffix within limits. Shared-string users must be disclosed; save/reopen and restore. Record locale and glyph issues.
- **C04 — Schedule browsing/export.** Browse one league's schedule and export if offered. Teams, dates, home/away and counts should agree between views/export.
- **C05 — Schedule edit/import.** Move one game to an unused date on a disposable copy. Review, save/reopen and restore; export/import the controlled change if offered. Unrelated games must retain values and positions.
- **C06 — Generation.** Use only a topology explicitly admitted by Studio. Record league, participants, dates, quotas/groups and counts. Preview before applying; retained games stay in place. If no safe recipe is clear, BLOCKED for a focused follow-up.
- **C07 — Schedule refusal.** In preview only, introduce a duplicate game/team-date collision or incomplete host selection. Expect a clear diagnosis and no mutation. Austrian phased generation remains withheld.
- **C08 — Tournament schedules.** Browse Champions Hockey League and Spengler Cup. Expect slot/seed explanations and read-only treatment for these documented tournament tables (V-70), not invented club assignments.
- **C09 — Stored defaults.** Inspect available season/playoff/rule defaults. If an admitted editor and copied target exist, change one displayed value by its smallest increment, inspect all paired-source changes, reopen and restore. Runtime precedence is a separate test.
- **C10 — League branding/audio.** Record separately: imagery listing, actual preview, replacement availability, and audio listing/playback. Only exercise replacement on an owned target through an admitted route. Missing writer/consumer stays open.

## Teams, arenas and uniforms

- **D01 — Identity/colors.** Pittsburgh: test name/city/abbreviation and one color independently. Readable references and other clubs must remain intact; restore each.
- **D02 — Relationships.** Inspect league/affiliation/rival links; change one valid existing relationship on a copy and restore. Check both sides where shown.
- **D03 — Roster operations.** Move a selected player between two suitable rosters and back; inspect membership and duplicate/capacity warnings. Do not infer other operations from one successful move.
- **D04 — Lines/captains/numbers.** Test one line assignment, captain/alternate and unused jersey number separately. Inspect special teams/goalie handling. Record dressed/scratched behavior separately; its semantics need confirmation.
- **D05 — Other team records.** Test one supported strategy, retired-number entry and draft-pick trade independently; restore. Record unavailable operations.
- **D06 — Team logos.** Inspect menu/small/scoreboard or other listed variants. On a copied target, replace one admitted image with an opaque, same-size test image; inspect shared consumers, save/reopen and restore. Resize and new registration remain separate qualifications.
- **D07 — Arena assignment.** Select another existing arena; expect old/new tenant disclosures before assignment. Cancel once, then apply/undo on the copy (V-47).
- **D08 — Arena/ice.** Inspect venue identity, center-ice/rink imagery and listed dependencies. Distinguish actual previews from folder listings or candidate bindings. Test a replacement only if the NHL21 route is explicitly supported; otherwise report missing/blocked.
- **D09 — Uniform variants.** Inspect home/away/alternate entries, order, thumbnails and assigned components. Test a supported reorder/assignment and restore; do not infer new-entry support.
- **D10 — Kit components.** Report jersey, pants, socks, fonts, numbers and nameplates separately: listing, preview, edit/recolor/replace, save/reopen and restore. Do not count a jersey preview as the whole kit.
- **D11 — Team mask slots.** Inspect Uniforms → Helmet → Team goalie mask paints. NHL21 should expose nine slots; note shipped exceptions and unsupported descriptions (V-80). Slot display is not proof of runtime mask mapping.
- **D12 — Team audio.** Inspect name calls, horns and goal/win/power-play/intro-song references. Report resolved names, sample playback and assignment/replacement availability independently.

## Players

- **E01 — Biography.** On the selected skater, test name, birth date, nationality, handedness/position, height and weight independently using small changes. Header/editor/reopened values must agree. Restore before the next case.
- **E02 — Nationality boundaries.** Disposable offline copy only: stored code 200 should survive save/reopen as a numeric unknown; 256 must refuse rather than wrap (V-73). Restore; never deploy this output.
- **E03 — Skater ratings.** Increase one sub-maximum rating by 1; check Attributes, roster Overall and any recalculation/preview agree with their stated behavior. Test style/potential/traits separately if exposed. A single-point edit need not change rounded Overall.
- **E04 — Goalie ratings.** Repeat with Fleury's goalie rating. Consistency should show raw 0/1/2, not a rating-scale offset; its meaning must not be invented (V-69).
- **E05 — Contracts.** Check displayed cap hit/term/status; change cap hit by one supported increment and restore. Stock NHL21 reference: Crosby $8.7m, McDavid $12.5m (V-60); edited rosters may legitimately differ.
- **E06 — Related records.** Inspect draft data, player indexes and companion records. Report missing/orphan warnings. NHL21 career-stat tables are absent per the Phase3 ledger; do not attempt to manufacture them.
- **E07 — Player lifecycle.** On a disposable DB copy, test create, clone and delete a test player separately where exposed. Check unique identity, equipment/related records and roster links; save/reopen. Delete only the newly created test record; preserve source players.
- **E08 — Historical import.** If the corpus is available, import one explicitly selected historical player; record source person, duplicate handling, generated ratings and roster placement. Review before committing and restore. Missing corpus or unresolved package identity means BLOCKED.
- **E09 — Bulk/formulas.** Inspect bulk editing/formula profiles/Legends builder independently. Use only preview initially; inspect one modest two-player change if a working UI exists. Record calibration/provenance and absent UI. Engine/API existence is not a UI pass.
- **E10 — Portraits.** Inspect actual portrait, containment and missing/unknown distinctions. Reassign to a listed shipped portrait through Check/Apply, then complete lifecycle and restore. Missing IDs must refuse.
- **E11 — Heads.** On a suitable copied skater record, check shared pool head 60000. Expect title-vault evidence and a coordinated head/head-texture change, one undo, and persistence. Check 60211 as the documented stock-source negative control (V-94); if your patch differs, report source mismatch rather than bypassing it.
- **E12 — Stale reassignment.** After a valid portrait/head Check, alter that same player's corresponding field, then attempt the old Apply. It must refuse stale review without overwriting the newer value (V-95). Restore both test edits.
- **E13 — Hair.** New test derived from receipt66: if Paul Stastny is present with hair 0, Check/Apply hair 1. Otherwise choose an actually listed alternative and record before/after. Expect only hair assignment to change, with helmet/color limitations disclosed. Undo/redo/reopen/restore; actual rendered hair is separate.
- **E14 — Other appearance.** Inspect supported skin/accessory/stance/celebration fields. Use only known listed choices for changes; unidentified animation IDs remain BLOCKED. Head conversion/new-head integration remains unqualified, not waived.
- **E15 — Player audio.** Inspect recorded-name/PBP/PA references and available sample chips. Report correct-name playback, missing calls and assignment availability separately; preview does not prove replacement.

## Equipment

For F01–F05 use an existing listed alternative; test pending/apply/undo/redo/save/reopen/restore for each component separately. If only numbers exist, record that and avoid claiming a verified item mapping.

- **F01 — Sticks.** Model, handedness/orientation, blade/shaft tape styles/colors and supported curve/flex/accessories.
- **F02 — Gloves.** Model, colors, branding and exposed variants.
- **F03 — Helmets.** Helmet, visor type/tint, ear protection and exposed accessories; inspect dependencies.
- **F04 — Skates.** Model, colors and any supported guards/options.
- **F05 — Goalie set.** Pads, blocker, catcher, goalie stick, mask shell/paint and neck protection independently.
- **F06 — Catalogue names/pictures.** Gear catalogue (PS4): inspect names and pictures for each skater category (V-88/V-91). Expect NHL21's own names. A Creation Zone picture is distinct from a model/material preview and runtime mapping.
- **F07 — Mask pair.** Inspect a shipped goalie's Mask pair (PS4) and picture. Distinguish list-backed images from labeled naming-rule candidates (V-90/V-91). Report pair and source description.
- **F08 — Mask negative control.** On an offline copy, set helmet type to 99: expect “helmet type not in the catalogue” disclosure, not necessarily a write refusal (V-87). Undo; do not deploy. This differs from true numeric out-of-range refusal.
- **F09 — Asset authoring/Hockey Shop.** Report which NHL21 equipment import/recolor/replace/new-entry/install operations are actually offered. Do not apply the X360 Hockey Shop recipe to PS4. Unknown PS4 writer admission means BLOCKED pending review.

## Presentation and audio

- **G01 — General UI/localization.** Inspect menus, icons, prompts, fonts and long/accented text on available surfaces. Test supported edits only on copies; missing editors remain open.
- **G02 — Boards/glass.** Record NHL21 browse/preview/replacement availability separately. Receipt67's PS3 board writer does not establish a PS4 route; no forced board writes.
- **G03 — Scorebug.** Inspect supported overlay/team-logo/text bindings. Record editing/preview availability and shared consumers; game values are checked later.
- **G04 — Speech playback.** On an admitted NHL21 sample, listen for wrong voice, clipped onset/tail, clicks, pitch or length. Record bank/sample identity. Unsupported codec should be named; use a clearly audible sample, not silence as a fidelity control.
- **G05 — Music/sounds.** Inspect catalogue labels, available playback and missing-source messages. Test stop/cancel and selection switching; old audio/results must not appear for the next selection.
- **G06 — Loop/multichannel.** Where offered, listen across loop joins and each channel pair; record exact sample and audible result. Absence of a playable loop/pair remains BLOCKED, not passed.
- **G07 — Audio authoring.** Record replacement/reassignment, playlist and transition availability separately. Do not infer a writer from a successful preview; incomplete encoder/consumer contracts remain open.

## Runtime and persistence — conditional, user-operated

Do these only for outputs whose corresponding offline tests passed, with exact deployment location and baseline restore established. Record runtime/emulator version and game patch. Isolate or specifically back up shared saves/cache before launch. If any of those prerequisites are unknown, mark this group BLOCKED for the next guided step. No intentionally invalid diagnostic output goes into the game.

- **H01 — Load controlled output.** Confirm the exact copied output loads and one known changed field is visible.
- **H02 — Player appearance/equipment.** Check skater and goalie close-ups, portraits, heads/hair and each changed gear component. Menus/thumbnails and on-ice rendering are separate observations.
- **H03 — Team/presentation.** Check edited names/colors/logo variants, home/away/alternate kit components, arena/ice and any admitted board changes. Include shared users and camera distances.
- **H04 — Gameplay/audio.** Check lineup/captain/number behavior, name calls/horns/music and relevant loop/transition behavior. Check scorebug team identity, score, clock, period and penalties, including replay context.
- **H05 — Roster save.** Load the native roster container, save, restart and reload. Intended changes persist; other records remain intact.
- **H06 — Season.** Separately test mode creation, edited schedule initialization/progression and save/reload. Record source versus mode-local behavior.
- **H07 — Franchise.** Separately test its supported container/edit/save/reload route. Roster success is not a Franchise pass; lack of fixture/runtime is BLOCKED.
- **H08 — Settings/tournament.** Separately test identifiable settings persistence/default precedence and tournament progression where applicable. Do not conflate them with Season results.
- **H09 — Final restore/stability.** Restore original content and any backed-up saves/cache, restart and confirm baseline behavior. Report accumulated slowdowns, freezes or resource growth during the session.

## Kept out of this self-guided batch

Destructive fault injection, archive byte corruption, changing packaged defs/resources, forced process termination during writes, archive re-keying and unadmitted writers. These require a separately prepared owned fixture. League/team create/clone/delete and executable patching are outside beta; Projects/Builds deferred. All other unavailable agreed functions stay recorded, not silently excluded.

## Findings format

Source/install + patch:
Working copy + output paths:
Baseline/restore method:
Records selected:

ID — PASS / FAIL / BLOCKED / MISSING / NOT TESTED — before → after; observation; save/reopen outcome; restore outcome; screenshot/error if useful.

Example: E04 — FAIL — goalie rating 85 → 86; editor changed but reopen showed 85; original untouched; output path …

List other findings separately with the screen, action and expected/actual result. No need to capture screenshots of ordinary passes.


Onboarding scope update (2026-10-01): ONB-002 supersedes the initial setup flow above for the next candidate. See docs/plans/onboarding-20261001-project-workspace.md. User is reviewing all-game onboarding first; detailed NHL21 tests remain queued until the revised workflow is delivered and reviewed.
