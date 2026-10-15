# Fresh Codex session — guided initial beta verification

You are my verification partner for NHL Modding Studio. Walk me through the initial beta review **one game/platform at a time, one concrete step at a time**. I will operate Studio and the games and report what happens. Keep development frozen while we do this. Your job is to make the pass comprehensive, practical and resumable, not to add features or ask me to interpret implementation details.

Repository: `E:\Repositories\nhl-modding-studio` (Windows/PowerShell). Read repository instructions. This explicit user-directed manual verification workflow supersedes older instructions that prohibited user-guided app/game testing; it does not authorize autonomous game launches, original-file writes, deployments, feature implementation or publication.

## Start with this fixed baseline

Use **r67**, the internal review candidate:

- Executable: `E:\nhlms-review-build-r67-2026-09-28\NHL Modding Studio rc-20260928-r67\nhl-studio.exe`
- Executable SHA-256: `60eee0adbd16df95692c4249f9926d7aff998855cf04958ba897fa12d37bbe65`
- ZIP: `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r67.zip`
- ZIP SHA-256: `6297cda973d2b9368d5809c086d71eb9f72c0ec14aa463493083f73fdfaa0b4a`

Verify the executable and packaged resources against the bundle manifest read-only before testing. Run the extracted bundle with its own defs/helpers/resources, not the development checkout. The checkout has uncommitted work and subsequent storage-profile changes; HEAD alone does not identify this build. Do not rebuild or replace r67. If the files differ or I choose another build, establish its identity explicitly before proceeding and keep results separated by build.

Read these authorities, resolving old conflicting banners by the latest dated decisions:

1. `docs/plans/development-20261001-verification-freeze.md`
2. `docs/validation/development-20261001-receipt67-review.md`
3. `docs/plans/entity-editing-parity-roadmap.md` and `docs/plans/beta-20261001-scope-decisions.md`
4. `docs/receipts/overnight-development-2026-09-24/67-recomp-routes-and-ps3-boards.md`, `LEDGER.md`, `LEDGER-PHASE3.md` and relevant earlier receipts
5. Phase coverage matrices under `docs/validation/` and research `USER-CARDS.md` files; `docs/research/overnight-2026-09-28/BETA-HANDOFF.md` is supporting research, not release authority and may contain stale status.

Retrieve the actual instructions for each relevant card before using it. If a receipt only names a card, construct a small test from the documented user workflow and label it a new test; never invent an existing card's contents. Deduplicate overlapping cards without dropping distinct title/platform, failure or recovery requirements. Later features do not prove earlier requirements complete.

## How to guide me

Do the document/inventory preparation yourself. Your first user-facing step should briefly state that the baseline is checked (or what prevented it), recommend starting with **NHL 21 PS4**, and ask which title/platform I want first if I prefer another. Do not dump the full checklist or ask me a dozen setup questions.

For each step give:

- A short test ID and purpose.
- The exact action to perform, including a named record and a modest before/after change once we have seen my data.
- The expected visible result.
- What I should report, and the restore action when relevant.

Then wait for my result before giving the next dependent step. Usually give one action or a small inseparable sequence; avoid a page of instructions. On success, record it and continue. On ambiguity, ask the smallest useful question. On failure, gather one useful screenshot/error or reproducible detail at a time. Don't require screenshots of every ordinary pass. Never fabricate screen labels: inspect provided screenshots or available UI state, or ask what I see. Use an available computer-use skill only if needed and follow its tools' permissions and limitations.

Use plain language and visible names. Keep binary field IDs, hashes and technical diagnosis in the record unless they help me act. Give a compact game summary when we finish that game or pause, not a running wall of receipts. Remember earlier answers and paths. If I ask for status, answer briefly and return to the same pending step.

## Coverage and order

Build a private working matrix for **all 27 shipping title/platform targets**:

- Xbox 360: NHL 07–15 and NHL Legacy (10).
- PS3: NHL 08–15 and NHL Legacy (9).
- PS4: NHL 15–21 (7).
- Legacy Recompiled (1).

NHL 07 PS3 is research-only; no Xbox One target. Treat different platforms and patch variants as distinct evidence. NHL 20's committed PS4 anchor is an NHL 19-base alias: use the genuine NHL 20 fixture/install, never qualify NHL 20 from that alias. Recompiled needs its own evidence despite sharing Legacy data. Preserve the previously accepted NHL21-E1 result for its actual scope; don't say there has been no user acceptance, and don't repeat it without a changed dependency.

Suggested order, subject to my preference: NHL 21 PS4; Legacy PS3; Legacy Xbox 360; Legacy Recompiled; then the remaining PS4 titles newest to oldest, PS3 titles newest to oldest, and Xbox 360 titles newest to oldest. Keep the matrix complete even if I reorder it. Finish or explicitly park one target with a blocker before moving on. A representative title never automatically passes its neighbors.

For each game, prepare an operation checklist covering **every applicable roadmap requirement**, not only recent fixes. Mark each as available to test, known unimplemented, blocked, or evidence-backed not applicable before counting results. Include:

1. **Startup and discovery:** correct bundled build, install/title/platform detection, open/import supported database/save inputs, readable names, dictionary errors, navigation, project/record switching, About/licences and missing-source messages.
2. **Core edit lifecycle:** one controlled edit through pending review; cancel/reject; apply; undo and redo; save to a copy; close/reopen; confirm intended values and unrelated data; restore. Check unsaved-change behavior and a safe invalid/out-of-range refusal. Database save success is not automatically a console save-container or Franchise-save pass.
3. **Leagues:** identity/localization, membership and exposed groups/rules, schedule browse/edit/import/generation within admitted topology, preserved games, collision/refusal behavior, branding and relevant audio.
4. **Teams:** identity/colors/localization, league/affiliation/roster relationships, lines/captains/numbers/strategies where supported, logos and variants, arena/ice assignments, kits/uniform variants/fonts/nameplates and team audio.
5. **Players:** identity including dates/nationality/height/weight; skater and goalie ratings/Overall and consistency; contracts and related records; historical import/bulk/roster operations; portraits/heads and applicable hair; other scoped appearance and player-name audio. Include creation/cloning only where the current Players contract supports it—league/team exclusions do not silently exclude player requirements.
6. **Equipment:** sticks, gloves, helmets/visors, skates and complete goalie gear. Exercise selection, actual preview availability, pending review, range refusal, undo/redo and save/reopen. Test Hockey Shop only on admitted targets. A candidate thumbnail is not proof of a correct gear mapping.
7. **Presentation:** supported UI/font/localization surfaces, arena board art, scorebug/team-logo bindings, menu music and sound previews. Distinguish preview from replacement; check looping/multichannel audio by listening where applicable. Missing routes remain missing, not passed through a safe refusal.
8. **Game observation and persistence:** only after the corresponding offline checks pass, guide me through loading the copied output in the exact game, observing the changed feature, exercising relevant save/reload and restoring the baseline. Record runtime/emulator/build and observations. Required save categories remain separate; do not infer Season/Franchise/settings support from a roster test.

Use a small set of controlled records/assets per game to reduce repetition, but keep each operation's outcome separate. Do not invent exhaustive qualification from one successful example. For absent fixtures/runtime access record blocked/untested and continue independent tests.

## Safety and known limitations

Before any write, establish the source path, owned working-copy destination and baseline/restore method. Do not copy a whole install by default: determine what the workflow needs and available space. I choose runtime launch and deployment actions. Never modify the only original, overwrite a pre-existing backup, delete an unexplained folder or change a shared user-data location silently. If a runtime copy still uses shared saves/cache, identify that before launch and establish isolation or a specific backup/restore procedure.

Known r67 distinctions:

- Recompiled Hockey Shop must refuse; test no mutation. The refusal is correct behavior for the guard, but the equipment-install feature remains incomplete.
- Recompiled speech/containment is delivered; portrait/head preview, header logo, league imagery, kits, arena art and equipment previews remain missing as recorded. Test actual behavior without promising those workflows.
- Speech duplicate/unreadable/partial-source handling has source-reviewed gaps. Stock-install success does not close them. Do not manufacture damaged live archives; developer fault tests use owned synthetic/copy fixtures.
- PS3 board UI is available on the measured loose-package titles; NHL 11–13 is backend-only. Start new board writes with Legacy copies (V-100). Other titles have differing evidence; do not generalize Legacy/NHL 13 transaction tests into full parity.
- DXT1 board images are encoded opaque with a disclosure; prefer opaque input first. DXT3 retains its supported alpha. Verify the visible result and every affected shared material where practicable.
- Austrian phased schedules, X360 board writer gates and archive re-keying remain withheld. Record blockers; do not bypass them for testing.
- **Do not run the original R-RECOMP-LOOSE-1 card as written.** The coordinator requires isolation, one location per experiment and valid positive controls. A disappearing failed-open log is not proof a file loaded.
- League/team creation/cloning/deletion and executable patching are outside beta; Projects/Builds deferred. Existing editing remains in scope. Head conversion/other uncompleted agreed functionality is not silently waived.

## Durable records and freeze discipline

Create verification records under `docs/validation/user-beta-pass-20261001/` without changing developer/research-owned ledgers or generated capability files. Use a compact `STATUS.md`, target checklist/results, and `ISSUES.md`; preserve existing records if resuming. Store proprietary screenshots/assets outside the tracked repo in an explicitly owned evidence location and record paths/hashes where useful.

Each result should identify: build, game/platform/patch, test/card ID, source/copy, operation and before/after, user observation, evidence, persistence/restore outcome and status. Distinguish user-observed pass, automated-only, fail, blocked, unimplemented, untested and not applicable. A section with unknowns is not complete. Save the next exact step at every pause so another fresh session can resume without repeating the pass.

For defects, capture reproducible steps, expected/actual, scope, severity and recovery state. Stop the affected write route immediately for lost edits, wrong-target writes, corruption or failed recovery; preserve the copied target and backups. Continue unrelated safe tests where possible. Produce a bounded developer bug handoff; **do not implement or send it to another session unless I authorize that action**. Cosmetic/new-feature work stays queued. Only an explicitly approved correction may produce a new numbered candidate; map exactly which prior results become stale and require rerun. Do not silently change builds mid-pass.

This is comprehensive verification of a frozen internal candidate. It does not imply the external beta is complete. Track all unfinished agreed features, reviews and manual checks separately from the MIT holder and artwork release inputs. Existing artwork is allowed internally; external replacement/rights requirements remain. At the end give a per-target/per-phase result, unresolved release blockers and a clear recommendation grounded in the evidence—no invented completion percentage or blanket release approval.

Begin now: verify the baseline and prepare the matrix, then give me only the first setup step for the first game.
