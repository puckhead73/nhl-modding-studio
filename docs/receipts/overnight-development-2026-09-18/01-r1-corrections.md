# Receipt 01 — LEAGUE-PARITY-COMPLETION-R1 (F1–F5), developer-verified

> **R2 correction (18 September, [OVERNIGHT-DEV-R2](../../validation/overnight-2026-09-18-review-and-r2-dispatch.md) F1):** the F1 writer described below omitted the destination whose rename had already happened from its rollback when the read-back failed or found the wrong identity, and this receipt's claim that "a crash between renames leaves a split the next open detects through identities" was not backed by durable evidence. Both are corrected in receipt [08](08-r2-write-recovery-and-image-ownership.md); the text below is left as written.

Overnight developer lane, 18 September 2026. Dispatch:
[league-parity-completion-review-and-r1-dispatch](../../validation/league-parity-completion-review-and-r1-dispatch.md).
Status: **implementation complete / automated verified / Codex review pending.** Nothing here is
game-verified; no game, emulator or live install was touched. NHL21-E1 stays accepted and closed.

## Plain-language summary

The five review findings are fixed and each has a regression that reproduces the original defect
first. **F1:** an in-place save now checks every file it is about to write — main database, roster
save, content-header sidecar and dictionary — *before* it writes anything, then publishes them
through temporary files with a verified rollback; a dictionary that changed underneath refuses the
whole save with the database untouched, a dictionary write that fails after the database was
written puts the database back, and the one case a rollback cannot fix is reported as exactly the
split it is (which file holds which bytes) instead of being hidden. **F2:** a reload that cannot
re-read the attached dictionary (missing or checksum-invalid) fails before adopting anything and
keeps the edits, the dictionary, the pending rename, the history and the revision; dropping a
dictionary is now its own button. **F3:** the rename panel binds every async result to the
project, the request generation and the inputs it was made for, and the shared project-summary
boundary refuses a summary for a project that is no longer open, so a late response can never pair
project B's paths with project A's summary; a preview disappears when its inputs or the dictionary
change. **F4:** the NHL 19 quarantine helper derives a per-file recovery plan, so an interruption
after the first move (Codex's reproduction: exit 2, `MIXED`, nothing moved) now restores or
continues from either direction, never overwrites unknown bytes, refuses through junctions, and
does not depend on its manifest. **F5:** `traced` binding evidence is scoped to the target whose
executable was actually read — NHL Legacy Xbox 360 — and NHL Legacy PS3 / Legacy Recompiled show
`by-reference`; the record and the desktop-screenshot description are corrected without rewriting
the screenshot.

## Findings and dispositions

| Finding | Disposition | Where |
|---|---|---|
| F1 dictionary validation after DB writes | **fixed** — two-phase in-place publication (`in_place_save` module; `Workspace::save_in_place_with`): preflight packages every file and verifies every destination against the loaded/attached identity (`stale_baseline` also refuses); publish = temp file + rename + read-back per destination; rollback in reverse from in-memory prior bytes, verified; `Outcome::Split` names `holding_new` / `restored` / `unknown` and marks the baseline stale until an explicit reload. Sidecar creation is planned (`plan_content_header`) not written ad hoc, and is removed on rollback. | `crates/studio-domain/src/in_place_save.rs` (new), `workspace.rs` |
| F2 failed dictionary reload discards edits | **fixed** — `reload_from_disk` prepares DB, roster **and** dictionary before adopting any; a dictionary failure returns an actionable error and changes nothing (stale reason recorded with origin). New deliberate `Workspace::detach_dictionary` → `Registry::detach_dictionary` → Tauri command `detach_dictionary` → Leagues toolbar "detach". Stale reasons now carry an origin (`Databases` / `Dictionary` / `Split`) so a re-attach or detach clears only dictionary-originated staleness. | `workspace.rs`, `studio-api/src/{registry.rs,commands.rs,lib.rs}`, `app/src/lib/client.ts`, `app/src/pages/Leagues.tsx` |
| F3 late rename responses overwrite another project | **fixed** — `ProjectProvider.update` adopts a summary only when `project_id` matches the open project (shared boundary). `DictionaryRenamePanel`: generation ref bumped on unmount / project change / input change / dictionary identity change; every mutation captures `(gen, forProject)` and completions are ignored unless still current; preview is stored bound to `(key, donor, dictionarySha256)` and shown only while those are current; summaries are requested for the originating project and published only if it is still open. `LeagueDetail` passes the dictionary identity. | `app/src/lib/project.tsx`, `app/src/components/DictionaryRenamePanel.tsx`, `app/src/pages/LeagueDetail.tsx` |
| F4 quarantine cannot restore an interrupted two-file operation | **fixed** — per-file states (`home` / `away` / `both-recorded` / `missing` / `unknown-at-install` / `unknown-in-quarantine` / `not-a-file`) → per-file plans; `quarantine` is all-or-nothing at *plan* level (a refused file moves nothing) but continues from a partial state; `restore` is per file and idempotent (an `away` file comes home even when its neighbour is refused); no overwrite ever; containment + ancestor reparse checks on both roots at plan time and at each move; `QUARANTINE.json` written before the first move and refreshed after each, never required; status prints the per-file plans; fixture-only `-FixtureEmuAfter N` simulates the emulator starting after N moves. **No live quarantine run.** | `local/runs/league-parity-batch/quarantine.ps1`, `tests/run-quarantine-tests.ps1` |
| F5 Xbox-only trace shown as proven on PS3/Recompiled | **fixed** — vocabularies may declare `[key_witness] traced_on = [{game, platform}]`; `LeagueVocabulary::key_evidence_for` resolves per target: `traced` only on a witnessed (game **and** platform), otherwise `by-reference` with a note naming the witness; a `traced` claim without a witness, or a catalog built without a platform, is never `traced`; `hypothesis` passes through. Session catalogs pass the project platform (`build_for_target`). Legacy TOML carries the witness (`nhllegacy` / `xbox`). UI describes the class in plain words. Record §3 legend and rows 8/10 corrected; §6 marks the PS3 screenshot as showing the pre-R1 defect. NHL 07 wording unchanged: label source *untraced*, not proven absent. | `crates/studio-domain/src/leagues.rs`, `session.rs`, `defs/leagues/gen7-nhllegacy.toml`, `defs/leagues/README.md`, `app/src/pages/LeagueDetail.tsx`, `docs/validation/league-parity-completion-2026-09-18.md` |

Design notes. The copied-target engine (`tdb-txn`) is not reused for in-place saves because it
refuses any root without the copied-target marker by design (a live tree must never be pointed
at); the in-place publication keeps the same two-phase/rollback shape without journaling. No
crash/power-loss atomicity across files is claimed for in-place saves: a crash between two
renames leaves a split the next open detects through identities. DTOs unchanged; bindings in
sync (`export-bindings --check`).

## Regressions added (each reproduces the defect against the pre-fix code path)

- `crates/studio-domain/tests/in_place_save.rs` (7): F1-A changed dictionary + real DB edit +
  pending rename → refused, DB identity unchanged, session working bytes / loaded identity /
  revision / undo depth / pending cells / pending rename / dictionary baseline all unchanged, no
  stale state; then the same save succeeds after the file is put back. F1-B changed database →
  refused before the dictionary is written. F1-C dictionary write failure after the DB was written
  (directory at the temp name) → DB verified back, "nothing on disk changed", then succeeds.
  F1-D injected rollback failure → `Split` error naming the DB as holding new bytes, nothing
  adopted, saves refused until an explicit reload clears it. F2-A missing dictionary, F2-B
  CRC-invalid dictionary → reload fails, everything preserved, dictionary still attached; restored
  file → reload succeeds. F2-C detach is explicit and clears only dictionary staleness.
- `crates/studio-domain/src/in_place_save.rs` unit tests (4): prior-read expectations, all-or-
  nothing with a created sidecar, temp-name obstruction, split reporting.
- `crates/studio-domain/tests/leagues.rs::key_evidence_is_scoped_to_the_witnessed_target`: Xbox
  360 `traced`, PS3 `by-reference` with the witness note, Recomp `by-reference`, no platform →
  never traced, NHL 14 stays `hypothesis` on both platforms.
- `crates/studio-api/tests/dictionary_rename.rs`: PS3 now asserts `by-reference` (+ evidence
  text); new Xbox 360 (`traced`, `default.xex` named) and Recompiled (`by-reference`) assertions
  on real anchors.
- `app/src/components/DictionaryRenamePanel.test.tsx` (5, deferred responses): preview dropped
  on input change; obsolete preview ignored after inputs changed while pending; preview dropped on
  dictionary identity change; late "Hold rename" summary for a replaced project never published
  while project B's own requests still publish B; nothing published after unmount.
- `app/src/lib/project.test.tsx` (1): `update` adopts only the open project's summary.
- `local/runs/league-parity-batch/tests/run-quarantine-tests.ps1` (27, was 8): interruption
  after the first move in each direction, continue-forward and restore-back, repeated recovery,
  emulator running / starting between moves, unknown bytes at either destination preserved,
  junction ancestor refused, restore without the manifest.

## Checks run (developer evidence)

| Check | Result |
|---|---|
| `cargo build -p studio-domain`, `-p studio-api` | clean |
| `cargo test -p studio-domain` (full, on the F1/F2 state) | 263 passed, 0 failed, 5 ignored |
| `cargo test -p studio-domain --test leagues --test in_place_save --test workspace --test pending --test copied_target` (final state) | 6 + 7 + 8 + 3 + 10 passed |
| `cargo test -p studio-api` (full, final state) | 80 passed, 0 failed, 3 ignored |
| `cargo run -p studio-api --bin export-bindings -- --check` | in sync (84 roots, 155 files) |
| `npx tsc --noEmit -p app` | clean |
| `npx vitest run` (app) | 8 files, 63 tests passed |
| `run-quarantine-tests.ps1` (fixture, process-scoped ExecutionPolicy Bypass) | 27 passed, 0 failed; fixture root removed |

Not run: the desktop app, any game or emulator, any live install, `cargo fmt`/`clippy` repo-wide
(known baseline defects, C0.2).

## Changed paths

New: `crates/studio-domain/src/in_place_save.rs`, `crates/studio-domain/tests/in_place_save.rs`,
`app/src/components/DictionaryRenamePanel.test.tsx`, `app/src/lib/project.test.tsx`, this
directory. Modified: `crates/studio-domain/src/{lib.rs,workspace.rs,leagues.rs,session.rs}`,
`crates/studio-domain/tests/leagues.rs`, `crates/studio-api/src/{registry.rs,commands.rs,lib.rs}`,
`crates/studio-api/tests/dictionary_rename.rs`, `app/src/lib/{project.tsx,client.ts}`,
`app/src/components/DictionaryRenamePanel.tsx`, `app/src/pages/{LeagueDetail.tsx,Leagues.tsx}`,
`defs/leagues/{gen7-nhllegacy.toml,README.md}`,
`docs/validation/league-parity-completion-2026-09-18.md` (§3 legend, rows 8/10, §6 note),
`local/runs/league-parity-batch/{quarantine.ps1,tests/run-quarantine-tests.ps1,tests/quarantine-results.txt}`.
`SHA256SUMS.txt` of the batch package is now stale for `quarantine.ps1` and the two test files
(deliberately not regenerated: the package's other 129 entries are unchanged and the candidates
were not affected — no candidate is regenerated by R1).

## Per-target readiness (unchanged by R1 except the evidence class)

The runtime matrix in `league-parity-completion-2026-09-18.md` §3 stands; the only row changes
are the binding class on NHL Legacy PS3 (`by-reference`) and Legacy Recompiled (`by-reference`).
No candidate identity changed. NHL 19's quarantine remains a runtime-session decision and is now
recoverable from any interruption.

## Cleanup

Fixture roots (`tests/fxq`, temp copied roots) removed by the tests themselves; no builds or
processes left; scratch edit helpers live only in the session scratchpad outside the repo.

## Unresolved

- Independent Codex review of F1–F5 (this receipt is developer evidence).
- Whether `SHA256SUMS.txt` in the batch package should be regenerated for the helper/test files
  (left stale on purpose; coordinator call).
