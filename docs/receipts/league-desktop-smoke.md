# LEAGUE-SMOKE-1 — desktop smoke receipt (2026-09-17)

**Disposition: desktop interaction gate EXECUTED on the real Tauri build (driven over the Chrome
DevTools Protocol against the app's own WebView2), with five integration defects found and fixed.
Not an in-game run; not independently reviewed; nothing staged, committed or pushed.**

**Plain-language summary.** The Leagues, Pending Edits and copied-target save screens were used for
real, in the actual desktop application, on disposable copies of three games (NHL 21 PS4 with a real
copy of its 777 MB game archive, NHL 18 PS4 with its own language dictionary, NHL 14 PS3). Browsing,
searching, jumping to a member team, renaming Allsvenskan's name key, undo/redo, reviewing and
applying the save into the copy (loose file, and loose file + archive together), reopening from
disk, exact restore, save-in-place, dictionary lookup and the older-console "no editable name" case
all behaved as designed, and every copied file was verified back to its original bytes afterwards.
The important thing that did **not** work: pressing **Cancel** during a long save had no effect —
the save finished anyway — because those operations ran on the application's main thread and
blocked every later request (and the window) until they returned; they now run off the main thread
and a cancel pressed 0.6 s into a two-minute save stops it with nothing changed. Also fixed: the
panel showed its own freshly prepared review as a red "blocked" root offering "Run recovery"; copied
roots were displayed with Windows' internal `\\?\` prefix; an over-long name key was refused with an
internal message; the Leagues search box was squeezed to a sliver; and closing a project never
released it server-side. Still unavailable: no game was run (in-game acceptance stays a separate
task), NHL 21's own dictionary was not supplied so its keys remain unverified text, and every edit or
review in the debug build takes 1.5–3 s (longer operations take minutes) because those commands still
run synchronously.

---

## 1. Package, disposition, scope

* **Package:** LEAGUE-SMOKE-1 per [league-r2-acceptance-and-desktop-smoke-dispatch](../validation/league-r2-acceptance-and-desktop-smoke-dispatch.md).
  Desktop smoke of the league editing / pending review / copied-save / restore flows plus directly
  discovered integration fixes with targeted regressions. No new phase, no live installation, no
  staging/commit/push; the pre-existing dirty tree (overnight + R1 + R2 work, unrelated streams) is
  preserved and was built on.
* **Desktop interaction — how it was done, and what it is not.** The verified launch (`npm run dev`
  at the repository root → `tauri dev` → `npm --prefix app run dev` → `http://localhost:1420`) was
  started with `CARGO_TARGET_DIR` set to the transferred cache and
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`. WebView2 then exposes the
  app's *own* page over CDP, so a small Node driver (scratchpad, no dependencies) sent **real input
  events** (`Input.dispatchMouseEvent` / `insertText` / `dispatchKeyEvent`) into the running
  desktop window, read the rendered DOM, and captured screenshots with `Page.captureScreenshot`.
  Every command went through Tauri IPC into the real Rust registry; nothing was mocked and no jsdom
  or direct-API call is presented as desktop evidence. What this is **not**: a human at a mouse, a
  release build, or a game. Clicks landed by coordinates on the live layout; one native control
  (the OS folder picker behind *Browse…*) was bypassed by typing the path into the same input the
  picker fills.
* **Disposition:** steps 1–7 executed. The interaction gate is **passed for the exercised flows**
  (§3), five defects were **fixed** with regressions (§4) and re-verified on the desktop, and the
  in-game / runtime gates remain **not executed**.

## 2. Fixtures, identities, originals

Created through the engine's own helper (`tdb_txn::engine::create_copied_target_root`, called from
a throw-away scratch crate) as marked copied roots; every copy's SHA-256 was recorded at setup and
re-checked at closeout. Anchors and the installed game were read only (`git status anchors` empty;
anchor hashes below unchanged after the run).

| Label | Copied to (under `local/runs/league-smoke-1/fixtures/`) | Bytes | SHA-256 (= original) |
|---|---|---|---|
| nhl21-db | `nhl21-ps4/CUSA18182/nhl/data/common/gamemodes/database/t3db/nhl_game.db` | 11 209 672 | `90ee1ec49d36dbb0f6d9cefe75e259d9661e3aa05338d03347b4842e33c8e9f9` |
| nhl21-meta | `…/t3db/nhl_game-meta.xml` | 495 259 | `0b262e4abafdf5fa…` |
| nhl21-archive (**real** copy of the installed `H:\Emulators\ShadPS4\Games\CUSA18182\databigs\inxp_00.big`) | `nhl21-ps4/CUSA18182/databigs/inxp_00.big` | 777 373 328 | `1c41f578a595dbd5aae7ad921445626636f3a842d8fb869bc054d35e3e849e87` |
| nhl18-db | `nhl18-ps4/CUSA07569/…/t3db/nhl_game.db` | 18 171 964 | `65eaa6e82c6dbad7…` |
| nhl18-meta | `…/t3db/nhl_game-meta.xml` | 492 716 | `b4d3c39c1c575d1a…` |
| nhl18-dict (the title's **own** supplied dictionary, at its discovered path) | `nhl18-ps4/CUSA07569/nhl/data/kettle/loc/eng_us.db` | 2 250 244 | `12f1af46906c0646…` |
| nhl14-db | `nhl14-ps3/dev_hdd0/game/BLUS31179/USRDIR/nhlng.db` | 2 820 656 | `84e1bdff74281102…` |
| nhl14-meta / nhl14-loc | `…/USRDIR/nhlng-meta.xml`, `…/USRDIR/nhl_eng_us.db` | 444 989 / 1 679 348 | `d1f9318e50008161…` / `d7bf2a6f7b579f20…` |
| nhl14-roster | `nhl14-ps3/dev_hdd0/home/00000001/savedata/BLUS311790200 - ROSTER/SYS-DATA` | 2 215 156 | `e54e9621f070402f…` |

Marked roots: `nhl21-ps4/CUSA18182`, `nhl18-ps4/CUSA07569`, `nhl14-ps3` (the picker was pointed at
the parent so discovery resolved the copies exactly as it would an emulator tree). The archive
evidence is **real** (a byte copy of the installed archive); no synthetic archive was used.

## 3. Desktop interactions and results (observed log)

Screenshots: `local/runs/league-smoke-1/screenshots/NN-*.png` (51 files, retained — see §7).
Timings are the debug build on this host.

### 3.1 NHL 21 PS4 — the dispatched sequence (step 3)

| # | Interaction (real input into the desktop window) | Observed |
|---|---|---|
| 00–02 | Start → *NHL 21* card → typed the copied root → auto-detect → *Open project* | Detected `nhl_game.db` + metadata; "art still packed" offer (not taken); opened at Teams (433 rows) |
| 03 | *Leagues* tab | 31 rows keyed by code; keys shown in mono amber (`NHL`, `AHL`, `elitserien`, … `TXT_ROW_LEAGUE`, `HUT`); toolbar "keys shown" + dictionary path/attach; **search box squeezed to ~30 px** (defect D5) |
| 04 | Typed `allsv` in search | 1 / 31 → `8 Allsvenskan 14` |
| 05 | Clicked the row | `/app/database/leagues/8`: banner `Allsvenskan · code 8 · native · 14 teams` + "Shown as its localization key…"; Identity card (`name` editable, 25-byte note, `leagueid` read-only); 14 member links |
| 06 | Clicked member `Team #265` | Team AIK header: `Allsvenskan · Conference group 63 · Division group 63` |
| 07–08 | Back via breadcrumb; clicked `name`, Ctrl+A, typed `TXT_ROW_LEAGUE`, Enter | Editor opened focused with `Allsvenskan`; commit round-trip ≈ 2.3–2.6 s (pending "…"); list row became `8 TXT_ROW_LEAGUE 14`; header pill *Pending Edits 1* + *Save* |
| 09 | *Pending Edits* | `unsaved changes · undo depth 1 · redo depth 0`; **one** cell: `TXT_ROW_LEAGUE / stockleagues / name / Allsvenskan → TXT_ROW_LEAGUE`; integrity: 1 table header; copied root prefilled — as `\\?\E:\…\CUSA18182` (defect D3); status *ready* |
| 10–11 | *Undo* → *Redo* | 1.7 s: `undo 0 · redo 1`, "no differences from the file on disk"; 2.6 s: cell back |
| 12 | *Review save* | 2.6 s; handle 0 / txn 000001; root and archive controls **disabled**, "Destination settings are locked…"; artifact `nhl_game.db` prior `90ee1ec4…` → output `343404153ae5…`, *unqualified*, loader-acceptance item; **but** status pill red *blocked* + *Run recovery* offered for the panel's own review (defect D2) |
| 13–14 | Second edit on the league page (`orderid` 11 → 12, 2.5 s), back to Pending | Leaving the page disposed of handle 0 (root *ready*, nothing listed); 2 cells shown |
| 15 | *Review save* (handle 1) → *Undo* on the page | "This review is stale: the session changed after the review (an edit, undo, redo or save). Apply is disabled"; Apply disabled; controls still frozen |
| 16 | *Discard review* | Controls re-enabled, root *ready*, 1 cell remains |
| 17 | *Review save* (handle 2 / txn 000003) → *Apply to copied target* | 3.0 s; "committed — written to the copied tree as unqualified artifacts (not an in-game installation)"; destination `→ 11209672 B 343404153ae5…`; "Session baseline: primary rebound (clean)"; after refresh: *saved · undo 1 · redo 1*, no differences. **Disk SHA-256 = `343404153AE5…`** (= the declared output) |
| 18–19 | *Close* → *NHL 21* (root remembered) → *Open project* → Leagues | Fresh session from disk: `8 TXT_ROW_LEAGUE 14`; "Existing project — settings loaded" |
| 20–22 | Pending → *Restore* 000003 | 5.5 s: "restored · nhl_game.db → 11209672 B 90ee1ec49d36… · Project 0: reloaded"; *saved · undo 0*; Leagues `8 Allsvenskan 14`. **Disk SHA-256 = anchor `90EE1EC4…`** |

### 3.2 NHL 21 PS4 — archive toggle and artifact review (step 5, real archive)

| # | Interaction | Observed |
|---|---|---|
| 23 | Edit again → Pending → ticked *Also replace the database member…* (defaults `databigs/inxp_00.big`, `nhl/data/common/gamemodes/database/t3db/nhl_game.db`) → *Review save* | **173 s** to prepare (stages/hashes the 777 MB copy); all three archive inputs frozen. Two artifacts from **one** revision: primary output `343404153ae5…` and `Archive (inxp_00.big)` prior `1c41f578a595…` → `4876cd2b3f4b…` whose entry 2652 carries that same primary output; qualification disclosed item by item (`+16` path hash resolved; `+12` sort key **unresolved**; header word `0x10` **unresolved**; slot replacement and native loader acceptance **unresolved**) — *unqualified*, *live install not allowed* |
| 24–25 | *Apply* (Cancel button visible while applying) | 136 s: committed; both destinations listed; **disk: archive `4876CD2B…`, db `343404…`** — exactly the declared outputs |
| 26 | *Restore* (2 files) | 112 s: `nhl_game.db → 90ee1ec4…`, `inxp_00.big → 1c41f578…`, "Project 0: reloaded". **Disk: both originals** |

### 3.3 Save-in-place, refusal, roots, switching (steps 4–5)

| # | Interaction | Observed |
|---|---|---|
| 27 | Edit → Pending → *Save (in place)* | "Wrote 1 file in place."; *saved · undo depth 1 · redo 0* (history kept); no differences; Save/Review disabled; header pill turns neutral. Disk `343404…` — byte-identical to the transaction engine's output |
| 28 | *Undo* past the save | *unsaved changes · undo 0 · redo 1*; **reverse** pending cell `Allsvenskan / name / TXT_ROW_LEAGUE → Allsvenskan` |
| — | *Save (in place)* again | Disk back to the anchor `90EE1EC4…` (writer round trip exact) |
| 29 | Typed a 26-byte key (`ABCDEFGHIJKLMNOPQRSTUVWXYZ`), Enter | Refused in the row, value unchanged, not dirty — text was `invalid string length: 26` (defect D4) |
| — | Prepared a review (handle 4), then *Close* with it prepared → opened NHL 18 → later reopened NHL 21 | NHL 18's panel resolved **its own** root, *ready*, no foreign handle listed; NHL 21 reopened: *ready*, no leftover handle, no restorable transactions, disk = anchor. (The old NHL 21 workspace itself stayed registered server-side — defect D6.) |
| 40 | Root input := an **unmarked** directory | "not a copied-target root created by this engine (missing COPIED-TARGET.tdb-txn)" |
| — | Root input := a **missing** directory | "resolving …/does-not-exist: The system cannot find the file specified. (os error 2)" |
| 41 | *Create a new copied root…* → typed path → *Create*, then *Create* again | Created + marked (`COPIED-TARGET.tdb-txn` on disk); second attempt: "already exists; a copied-target root is only ever created as a new, empty directory" — with the earlier "Created…" note still standing beside it (defect D7) |
| 42 | Root := the empty UI-created root; *Review save* with a dirty NHL 21 | Refused: "the workspace's main file … is not inside the selected copied-target root …; the root is never widened implicitly" — no review, edits kept |
| 43–44 | Archive review (186 s) → *Apply* → **Cancel apply** clicked at +0.6 s | **Defect D1:** "Cancellation requested." never appeared; after 131 s the panel showed "Nothing to cancel for that handle." and the transaction had **committed** (disk = the edited outputs). Restored afterwards (000006, 114 s, both originals) |

### 3.4 NHL 18 PS4 — the target's own dictionary and unknown keys (step 5)

| # | Interaction | Observed |
|---|---|---|
| 30–31 | Opened `nhl18-ps4` (detected db, metadata **and** `kettle/loc/eng_us.db`); on-load jersey autocorrect notice dismissed and its edit undone; Leagues | 26 rows, "names via eng_us dictionary (33,396 rows)"; resolved text (`NHL®`, `SHL`, `HockeyAllsvenskan`, `Rest Of World` for `TXT_ROW_LEAGUE`); keys absent from the dictionary stay keys (`CUSTOM_TEAMS`, `TXT_LEAGUE_MASCOT`, `TXT_LEAGUE_3ON3`); search box full width (no attach control) |
| 33 | League 8 | `HockeyAllsvenskan · code 8 · key Allsvenskan · native · 14 teams` |
| 34 | `name` := `TXT_SMOKE_UNKNOWN` (2.3 s) | Banner switches to the mono key `TXT_SMOKE_UNKNOWN` with "Shown as its localization key…"; pending cell `Allsvenskan → TXT_SMOKE_UNKNOWN`; undone (saved · undo 0) |

NHL 18's dictionary was **not** attached to NHL 21; NHL 21's key existence/runtime text remains
unverified (no NHL 21 dictionary fixture).

### 3.5 NHL 14 PS3 — older layout (step 4)

| # | Interaction | Observed |
|---|---|---|
| 36 | *NHL 14* → *PS3* → copied RPCS3-shaped root → *Open project* | db + metadata + `nhl_eng_us.db` + 1 roster save detected; opened with the roster mirror (*Main DB / Roster save* toggle) |
| 37 | Leagues | 15 codes, all labelled from `gen7-nhl12-15` (`NHL`, `AHL`, `SHL (Sweden)`, … `Winter Classic`); no raw-code note, no attach control |
| 38 | Code 2 | `SHL (Sweden) · code 2 · corroborated · 12 teams`; evidence "gen7-nhl12-15 (corroborated): 12 rows; stockleagues 2 = elitserien (dictionary text SHL on NHL 15 PS4)"; Identity card = the truthful blocker ("This title stores no league name in its database…"), **no** editable control; 12 named member links |
| 39 | Member AIK | Header `SHL (Sweden) · Conference group 20 · Division group 20`; Team Data *League* cell shows `SHL (Sweden)` |
| — | Pending Edits | Two pending files (`Main database (nhlng.db)`, `Roster save (SYS-DATA)`), no differences; own marked root resolved, *ready* |

### 3.6 Re-verification after the fixes (same desktop, rebuilt by `tauri dev`)

| # | Check | Result |
|---|---|---|
| 45 | Leagues search box | 320 px wide; toolbar wrapped above it |
| 46 | 26-byte key | `name: the text is 26 bytes but this field holds at most 25 bytes; the edit was not applied` |
| — | Copied root display | `E:\…\fixtures\nhl21-ps4\CUSA18182` (no `\\?\`), status query round-trips |
| 47 | Review prepared | green **review prepared** pill with the engine's guidance; **no** *Run recovery* button; Apply enabled; *ready* after discard |
| 49 | Archive review (199 s) → *Apply* → **Cancel apply** at +0.6 s | "Cancellation requested." at **+0.64 s**; apply ended at +40 s with "transaction 000008 … cancelled at BeforeStageReplacement("primary"); Aborted; nothing was changed"; root *ready*; edits kept (*unsaved · undo 1*); **disk: both originals** (`1C41F578…`, `90EE1EC4…`) |
| — | Restore of 000006 issued while the command ran off the main thread | the webview kept answering throughout the 114 s restore (the CDP poll loop never stalled) |

## 4. Defects found and their disposition

| ID | Defect (desktop-observed) | Fix | Regression |
|---|---|---|---|
| **D1 (P1)** | Cancel during a copied-target apply is impossible: `copied_target_apply/review/…` were plain `#[tauri::command]`s, which Tauri runs on the **main thread**; every later IPC call (the cancel, status polls, the window's own events) queued behind the 2-minute apply. The registry logic was correct (§3.3 #43: cancel answered "nothing to cancel" only after the commit). | `crates/studio-api/src/commands.rs`: `copied_target_review`, `_review_archive_member`, `_apply`, `_discard`, `_recover`, `_restore`, `_reload` are now `#[tauri::command(async)]` (Tauri's "sync_threadpool" context — the sync body runs in a spawned task off the main thread; the registry runs the engine without its locks held, so the still-sync status/cancel never wait on them). Module note added above `copied_target_review`. | `crates/studio-api/tests/command_execution_context.rs` (new): asserts each of those declarations carries `#[tauri::command(async)]`. The behaviour itself: desktop re-verification §3.6 #49; the engine-level cancel paths remain covered by the existing `copied_target` cases. |
| **D2 (P2)** | The panel rendered its **own** prepared review as a red *blocked* root and offered *Run recovery* (recovery on a prepared set would abort the review the panel still holds). | `CopiedTargetPanel.tsx`: `activeHandle()` / `needsRecovery()` — when the root's active transaction is a live handle the pill reads *review prepared* (own) / *review pending* (foreign) / *applying* (in flight) and *Run recovery* is hidden; a genuinely interrupted transaction (no handle) still shows *blocked* + *Run recovery*. | `CopiedTargetPanel.test.tsx` "status presentation": 3 cases (own prepared; foreign pending + in-flight; interrupted keeps recovery). |
| **D3 (P3)** | Copied roots were displayed with the Windows verbatim prefix (`\\?\E:\…`) in the root field, the "Created …" note and the reopened panel. | `registry/copied_target.rs`: `copied_target_root_for` and `copied_target_create_root` return `display_path(...)` like every other root string. | `league_copied_target.rs`: no `\\?\` prefix on either result, and each round-trips through `copied_target_status` (existing root assertion updated to the display spelling). |
| **D4 (P3)** | An over-long name key was refused with the internal `invalid string length: 26`. | `tdb-edit`: new `EditError::TextTooLong { field, bytes, max }` ("{field}: the text is {bytes} bytes but this field holds at most {max} bytes; the edit was not applied"), raised by `EditSession` with the field's metadata name. | `studio-domain/tests/leagues.rs` asserts the exact sentence; `studio-api/tests/leagues.rs` updated. |
| **D5 (P3)** | Leagues list: the "keys shown / dictionary path / attach" toolbar squeezed the search box to ~30 px. | `EntityList.tsx`: the toolbar/search row wraps (`flex-wrap`) and the box keeps `min-w-[8rem]`; the compact select toolbars elsewhere still share the row. | Visual (§3.6 #45); no component test (layout). |
| **D6 (P3)** | *Close* (and the corruption-guard escape) only dropped the shell's reference; the server-side workspace was never released (`close_project` existed and was registered but was absent from the typed client map and never called). | `AppShell.tsx` Close and `OpenProject.tsx` `forcePicker` fire `close_project`; `client.ts` gains the command entry. The panel's unmount discard and `close_project`'s own discard race harmlessly (whichever runs second finds nothing). | Existing registry `close_project` behaviour; type-checked wiring. Not observable from the DOM, so no desktop assertion is claimed. |
| D7 (nit) | A refused root creation left the earlier "Created …" note beside the error. | `createRoot.onError` clears the note. | `CopiedTargetPanel.test.tsx` "drops the earlier 'Created …' note". |

**Observed but not changed (recorded for Codex):**

* Every other command is still synchronous on the main thread: an edit, undo or pending review
  takes 1.5–3 s in the debug build and the window is unresponsive for that long. Making the
  remaining 100+ commands async is a broader change than this package allows; the archive
  review/apply/restore times (2–3 min) are dominated by hashing/copying the 777 MB archive and are
  now at least cancellable and non-blocking.
* The engine's own cancel/abort message still embeds a canonical `\\?\` recovery path and an empty
  ``stage failed at `` `` location (`tdb-txn` text, outside this package's seam).
* The pending review names a league cell by its **after** value (`TXT_ROW_LEAGUE`) — as the API test
  asserts; arguably the before-name reads better. Left as designed.
* *Close* with unsaved edits asks nothing (no confirm step); out of scope here.
* The picker persists the last-used root per game/platform into the app's saved settings; the smoke
  overwrote `nhl21:ps4` / `nhl18:ps4` and added `nhl14:ps3`. Restored exactly to the pre-run values
  (`H:\Emulators\ShadPS4`, no `nhl14:ps3` entry) before closing the app.

## 5. Checks executed (developer, this host; none independent)

All Rust with `--target-dir local/runs/league-overnight-2026-09-17/target`.

| Class | Command | Result |
|---|---|---|
| Desktop | `npm run dev` (root) + CDP driver, all of §3 | executed as recorded; app relaunched by `tauri dev` after the Rust fixes (15:54) and re-verified (§3.6) |
| Domain | `cargo test -p studio-domain --test leagues --test referential --test pending --test session` | 7 / 5 / 3 / 11 ok |
| API | `cargo test -p studio-api --test league_copied_target --test copied_target --test command_execution_context --test leagues --test league_offline_campaign` | 7 / 15 / 1 / 3 / 1 (28 rows) ok |
| Full crates | `cargo test -p tdb-edit -p studio-domain -p studio-api` | **58 `test result: ok` lines, 0 failures** (includes the anchor-wide matrices and the pre-existing `zz_diag`) |
| Structural | `export-bindings -- --check`; `cargo clippy -p tdb-edit -p studio-domain -p studio-api --tests`; `cargo check -p nhl-studio`; `rustfmt --check` on the four new/changed test files | "Bindings are in sync" (no DTO changed); no warnings; shell compiles (and was rebuilt/launched by `tauri dev`); clean |
| Component | `cd app && npx vitest run` | 5 files, **43 passed** (39 + 4 new) |
| Frontend | `npx tsc --noEmit -p app`; `npx vite build` | clean; built |
| Integrity | anchors: `git status anchors` empty; NHL 21/18/14 dbs, NHL 18 dictionary and NHL 14 roster re-hashed = originals; every fixture copy re-hashed = original at closeout; installed game read only | ok |
| In-game | — | **not executed, not authorized** |

Fixture identities used by the tests: committed anchors (NHL 21 `90ee1ec4…`, NHL 18, Legacy) and
per-test temp copies, as before.

## 6. Remaining blockers and limitations

* **Runtime:** nothing is game-verified; the PS4 archive output stays structurally unqualified
  (`+0x0C` row word, header `0x10`, slot replacement, loader acceptance all *unresolved* — and the
  UI says so on every review). Loose-vs-archive precedence on PS4 is untested.
* **NHL 21 dictionary:** no fixture; keys unverified; NHL 18's dictionary was only used for NHL 18.
* **Older consoles:** no league name in the database (the UI states the blocker); code → dictionary
  key binding remains Phase 1.3.
* **Performance:** debug-build timings only; the synchronous-command latency above is a real desktop
  limitation for release qualification.
* **Desktop evidence class:** CDP-driven real webview, not a human run; the native folder picker
  was not exercised.

## 7. Temporary artifacts, processes, cleanup

Package root: `E:\Repositories\nhl-modding-studio\local\runs\league-smoke-1\` (untracked).

| Item | Purpose | Disposition |
|---|---|---|
| `fixtures/` (2 405 569 508 bytes: three marked roots, the real archive copy, 8 terminal transaction journals + recovery backups, UI-created root, profile sidecars) | disposable copied targets | **removed** after hash verification (every copy = original), absolute-path containment and reparse-point check; `Test-Path` false afterwards |
| `screenshots/` (51 PNG, 6.2 MB) | desktop evidence referenced in §3 | **retained** (evidence for Codex's review of this receipt; remove with the package root after review) |
| `settings.pre-run.json` | exact pre-run app settings, used for the restore in §4 | retained with the screenshots (documents what was restored) |
| `build.log`, `dev.out.log`, `dev.err.log`, `cargo-tests.log` | build/dev-server/test transcripts | transcribed here; **removed** (`cargo-tests.log` after the 58/58 count above was recorded) |
| Scratch crate `H:\TEMP\…\scratchpad\smoke-setup\` and CDP driver `…\scratchpad\cdp\` | fixture setup (via the engine helper) and the desktop driver | session scratchpad, outside the repo; the driver source is the only way to reproduce §3 exactly — retained in the scratchpad for this session only |
| `local/runs/league-overnight-2026-09-17/target/` (transferred cache, now also holding `release/league-smoke-setup.exe` + the debug `nhl-studio.exe`) | compile/launch the candidate; rerun the checks | **retained** with a named dependency: Codex's review rerun of the D1–D4 regressions and the Tauri shell build (≈30 min to rebuild). Remove after that review; no second cache was created |
| App saved settings (WebView2 localStorage, `nhl-modding-studio.settings.v1`) | overwritten by the picker during the smoke | **restored** to the pre-run values (verified) |
| Engine staging `%TEMP%\nhl-modding-studio\copied-target-staging\` | per-review staging | empty at closeout (the entries seen mid-run belonged to the cargo test process and were settled by the tests themselves) |
| Processes | `nhl-studio.exe`, the Tauri CLI node, the Vite node and their `cmd` launchers | **stopped** (none matching remain; port 9333 closed) |

No `git clean`/`reset`; no shared `local`/`target` tree deleted; index empty; nothing staged.

## 8. Next review action

Codex: review D1 (the async command context and its source guard) first — it changes how seven
commands execute — then D2–D4; rerun `cargo test -p studio-api --test command_execution_context
--test league_copied_target --test leagues`, `cargo test -p studio-domain --test leagues`, and
`cd app && npx vitest run src/components/CopiedTargetPanel.test.tsx`. If accepted, the natural
follow-ups are (a) the in-game NHL 21 test of a copied `inxp_00.big` (separately authorized) and
(b) a bounded package moving the remaining synchronous commands off the main thread. Stopping here.
