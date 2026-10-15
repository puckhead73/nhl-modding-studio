# LEAGUE-RESP-1 — desktop responsiveness during copied-target review/apply (2026-09-17)

**Plain-language summary.** The desktop window no longer freezes while a copied-game save is being
prepared or written. Before this change the app's window stopped answering for the whole
preparation of a save that includes the 777 MB game archive (about 3½ minutes) and for about half a
minute at the start of writing it, and pressing **Close** during either was answered only when the
lock cleared — during a preparation the answer then *closed the project and threw the prepared save
away*. Now Close, the status display, **Cancel** and edits on other open projects are answered in
about a third of a second throughout: Close is refused with a plain reason ("review of project …" /
"apply of review handle … still needs this session"), the window's own message loop never stalls
(measured at the native level: worst round trip 3 ms during the long operations, versus 211 s and
34 s before), and a cancel pressed during the first second of a save is honoured within about 0.6 s
with nothing written. A save that runs to completion still installs exactly the declared bytes and
restores exactly the originals, verified on the real archive copy. Two limits remain: editing a
project *while its own save is being prepared* is now refused at once with the reason (previously
the edit silently waited and then made the review stale), and the player/team/asset screens outside
this package's scope still issue their commands on the window's thread. No game was run and nothing
was installed live.

---

## 1. Package, disposition, scope

* **Package:** LEAGUE-RESP-1 per
  [league-smoke-r1-review-and-responsive-dispatch](../validation/league-smoke-r1-review-and-responsive-dispatch.md).
  One bounded infrastructure package: the dispatch/lock scope of the commands the league, pending
  and copied-target screens (and the shell/picker on the same flows) dispatch. Not a migration of
  every command; no lock redesign beyond the reservation model already in place; no Phase 1.2/1.3,
  no Projects/Builds, no live install, no in-game run; nothing staged, committed or pushed. The
  pre-existing dirty tree (overnight/R1/R2/SMOKE-1/SMOKE-R1 and unrelated streams) is preserved and
  built on; LEAGUE-SMOKE-R1 stays accepted and untouched in its scope.
* **Disposition:** the measured ~32 s close stall and the review-time workspace-lock case are both
  resolved on the desktop with measured values; contention regressions with barriers are in place;
  one newly visible frontend staleness (a cancelled apply briefly shown as a foreign "review
  pending") was found by the measurement and fixed with a component regression. Developer-verified,
  awaiting Codex review.

## 2. Before/after map: what the screens dispatch, where it runs, what it waits on

Commands traced from the typed client map as used by `AppShell` / `CloseProjectButton` /
`LingeringProjectNotice`, `Leagues`, `LeagueDetail` (+ `EditableField`), `PendingEdits`,
`CopiedTargetPanel` and the picker's open path.

| Command group | Before (thread · lock waits) | After (thread · lock waits) |
|---|---|---|
| `close_project`, `project_summary`, `validate_project`, `set_edit_override`, `set_display_roster`, `autocorrect_jerseys`, `repair_roster_blocks`, `save_workspace` (shell) | **main thread**; `inner` | blocking pool (`off_main` → `spawn_blocking`); `inner` — the window never waits |
| `list_leagues`, `league_view`, `attach_dictionary`, `set_field`, `list_arenas` (leagues) | main thread; `inner` | blocking pool; `inner` |
| `pending_changes`, `undo`, `redo`, `save_project` (pending) | main thread; `inner` | blocking pool; `inner` |
| `copied_target_status`, `copied_target_cancel`, `copied_target_root_for`, `copied_target_create_root` | main thread; `copied` + `inner` (status), `copied` (cancel) | blocking pool; same locks, now never held long (§3) |
| `copied_target_review`, `_review_archive_member`, `_apply`, `_discard`, `_recover`, `_restore`, `_reload` | `#[tauri::command(async)]` on a sync body = **inline on a tokio runtime worker** (pins a worker for minutes); review held `inner` for its whole duration; apply held `inner` across `verify_plan_inputs` | blocking pool; review holds `inner` only to package the sessions, apply holds `inner` only for the binding checks + reservation (§3) |
| `detect_install`, `open_workspace` (picker, close → reopen) | main thread | blocking pool |
| everything else (players, teams, assets, Hockey Shop, historical, formulas, `load_project`, `open_project`, `extract_big_files`) | main thread | **unchanged** — remaining limit (§5) |

Lock order is unchanged and still `copied` → (released) → `inner`, never nested the other way; the
engine's root lock is fail-fast (never waits) and `tdb_txn::status` takes none. Registry code
never holds `inner` across engine work or whole-file hashing any more.

**Why `off_main` and not the `async` attribute.** `tauri-macros` 2.6.3 `body_async` calls the
sync body *inside* the `async move` block it hands to `respond_async_serialized` → the body runs
on a runtime worker, not a blocking thread. `off_main` wraps the registry call in
`tauri::async_runtime::spawn_blocking`, reaching the registry through the `'static` `AppHandle`
(`app.state::<Registry>()`), so no borrowed `State` crosses into the pool.

## 3. Lock/dispatch decisions (registry, `crates/studio-api/src/registry/copied_target.rs`)

* **Apply preflight split.** One short `inner` critical section does the binding checks (project
  present, revision == plan, no stale baseline), the cheap layout check (destination parents exist —
  new `verify_plan_layout`, split out so its message is preserved) and computes the reservation
  keys, then **reserves**. `inner` is released; the whole-file identity verification (every
  destination prior, every staged output, every bound input — ~1.5 GB on the real archive, the
  measured ~32 s) runs with **only the reservation held**, via the new
  `verify_plan_inputs_unless(plan, cancelled)` that polls the cancel flag between files. Then a
  final revision re-check under `inner` (defensive: the reservation already excludes every registry
  writer — `with_workspace`, `register`, Save-As, close), then the engine with no lock. A skipped
  (cancelled) verification is never treated as passed: the engine then only ever runs to abort at
  its first checkpoint, exactly as a cancel before the apply. `r2_same_project_edits_cannot_slip…`
  still holds: an edit lands before the revision check (apply refuses) or is refused by the
  reservation — no third outcome.
* **Review reservation.** The review packages the sessions under `inner` (`stage_workspace`, a
  few seconds — the same order as an ordinary edit) and, in the same critical section, takes a
  reservation "review of project N (handle H)" on the project, its aliases and its files (the
  archive-member variant reserves the archive up front; the generic seam grows the reservation
  after `extend` through `Inner::extend_reservation`, refused if another reservation covers a
  file). `inner` is released; archive staging + re-verification and the engine's `prepare`
  (minutes) run under the reservation only. Final revision validation under `inner`, then the
  handle is registered **before** the reservation is released (no moment where the root's active
  set has no owner). Any failure releases the reservation and removes the staging (or settles the
  prepared set through the existing `settle` path).
  *Consequence, deliberate:* an edit/save/reload/close/alias-open of a project while its review is
  in progress is refused at once with the holder named, instead of waiting ~3 minutes on the mutex
  and then invalidating the review. Close answers `busy` with that reason.
* **Preserved:** single authoritative snapshot (the plan is bound to the revision packaged under
  `inner`), external-change detection (identity verification before the engine + the engine's own
  re-check), ordered locking, alias reservations (`projects_on` at reserve time), rollback,
  recovery, acknowledged close (`close_project_outcome` unchanged), the S2-C-R3 staging
  obligations. No cached status claims completion: status still derives from journals + live
  handles/reservations. DTOs unchanged (bindings in sync, 81 roots).
* **Frontend (found by the measurement).** With cancellation settling in ~0.6 s, the panel's old
  order (clear the owner → `invalidateQueries()` for everything → error state) showed the cached
  pre-apply status for ~3.7 s: its own, already-aborted handle as a foreign "review pending" with a
  *Discard* button, the abort text arriving only afterwards. `CopiedTargetPanel.tsx`: the apply's
  success/error paths now re-read the root status (`refetchQueries(["copied-status", root])`)
  **before** dropping ownership (`settleAppliedHandle`), the error path no longer waits for the
  summary refresh, and `refresh()` no longer awaits the invalidate-everything chain. Desktop
  re-probe: one clean transition 325 ms after the cancel click (pill *ready* + abort text, no
  foreign row).

## 4. Regressions and checks

### New / changed tests

| Suite | Cases |
|---|---|
| `crates/studio-api/tests/copied_target_contention.rs` (**new**, barriers via channels + per-call deadline threads, timings printed) | `close_status_cancel_and_edits_answer_promptly_during_apply_preflight` — apply held at `ApplyPhase::Reserved` (reservation held, `inner` free): close(a)/close(alias b) → `busy` naming the apply; status lists the in-flight handle, `busy_projects=[a,b]`; edits on a/b refused ("locked by"); unrelated c edits and closes; second apply / review-during-apply refused; cancel acknowledged — all **< 1 ms – 300 ms**; released → aborted (cancel honoured), nothing written, edits kept, a/b close cleanly. `cancel_before_apply_aborts_without_verifying`. `close_status_and_edits_answer_promptly_during_review_staging` — review held in the staging hook after packaging: close(a/b) → `busy` "review of project a"; status shows the reservation (3 files, projects [a,b]) and no handle; edits/competing reviews/alias open refused promptly, Save-As into held file refused, unrelated project fine; released → handle on the packaged revision, applies (alias reloaded), both close. `a_failed_review_releases_its_reservation_and_staging`. `concurrent_reviews_on_one_root_contend_at_prepare_never_on_the_mutex` — second project (different file, same root) reviews to a handle while the first is held (715 ms); the first is refused by the engine at `prepare` (busy root), reservation/staging released, edits kept; second applies, first then reviews+applies. **5 cases.** |
| `crates/studio-api/tests/command_execution_context.rs` (rewritten) | source guard for the **30** off-main commands: `pub async fn`, plain `#[tauri::command]`, body forwards through `off_main(app, …).await`, no `State` borrowed; `off_main` uses `spawn_blocking`. Explicitly *not* the evidence of contention resolution (the file says so). |
| `app/src/components/CopiedTargetPanel.test.tsx` (+1) | "re-reads the root status before dropping a cancelled apply's handle — never the pre-apply cache": status re-read counted, abort text shown while the summary refetch is still pending, no *Cancel apply*, no "other prepared reviews", no "review pending", pill *ready*, no discard issued. |
| `copied_target_apply_observed` / `ApplyPhase` (doc-hidden seam), `copied_target_review_with` (unchanged signature, now the review-time seam) | the barriers above |

### Checks executed (developer, this host, `--target-dir local/runs/league-overnight-2026-09-17/target`)

| Class | Command | Result |
|---|---|---|
| API | `cargo test -p studio-api --test copied_target_contention -- --test-threads=1` (and default threads) | 5 ok, 31 s; every held call answered in 0.07–0.9 ms (open-alias refusal 293 ms, unrelated edit 306 ms) |
| API | `--test copied_target --test command_execution_context --test league_copied_target --test leagues --test registry --test roster_builder --lib` | 15 / 1 / 7 / 3 / 6 / 1 / 31 ok (drift gate + 81 roots) |
| Domain | `cargo test -p studio-domain --test copied_target --test txn_coordination` | 6 / 3 ok |
| Full | `cargo test -p studio-domain -p studio-api` (final code) | 52 ok lines, 355 passed, 0 failed, 6 ignored (§9) |
| Bindings | `export-bindings -- --check` | "Bindings are in sync" (no DTO changed) |
| Structural | `cargo clippy -p studio-domain -p studio-api --tests`; `cargo check -p nhl-studio`; `rustfmt --check` on the 5 changed Rust files | no warnings; shell compiles (and was rebuilt/launched by `tauri dev`); clean |
| Component | `cd app && npx vitest run` | 6 files, **56 passed** (55 + 1) |
| Frontend | `npx tsc --noEmit -p app` | clean |
| Desktop | §4.2 | executed as recorded |
| Not rerun | tdb-edit/tdb-txn suites, the 28-row campaign, real-archive `#[ignore]` cases | no engine change |
| In-game | — | not executed, not authorized |

### 4.2 Desktop measurements (real Tauri build over CDP against the app's own WebView2; NHL 21 PS4 copy with the real 777 MB `inxp_00.big` copy; debug build; not a human run, not a game)

Method as in LEAGUE-SMOKE-1, plus two new probes. **Main-thread heartbeat:** a PowerShell loop sends
`WM_NULL` to the app's main window with `SendMessageTimeout` (5 s cap) every 100 ms and logs the
round trip — the window's UI thread answers only when it pumps its message loop, which is what
"the window accepts input" means at the native level, independent of the renderer process (which
keeps answering CDP even while the host thread is blocked). **IPC probe:** from the page, every
500 ms, a trivial sync command (`corpus_open`, no registry lock — IPC-level main-thread
availability) and `copied_target_status`. Command response, cancellation acknowledgement and
final settlement are measured separately from the click. **Baseline** = the pre-change
`nhl-studio.exe` from the transferred cache (18:13, R1 build) launched directly against a Vite dev
server; **after** = the candidate rebuilt by `tauri dev` (19:56). Ordinary edit/read baseline
included.

| Measurement | Baseline (pre-change build) | After (candidate) — two runs |
|---|---|---|
| Ordinary league edit (`name` commit → header pill) | 2 173 ms | 2 020 / 3 259 ms (2 017 in the commit run) |
| Ordinary reads, idle: `league_view` / status / trivial cmd | 229 / 3 / 2 ms | 255 / 6 / 2 ms · 297 / 12 / 2 ms |
| Main-thread stall during an ordinary edit (heartbeat) | 1.4–2.0 s per edit | none ≥ 1 s (max 233 / 274 ms across the whole run) |
| **Close clicked +5 s into the archive review** | answered after **206.6 s** — and it **closed the project** once the review finished (review wasted, txn 000001 aborted) | **refused in 310 ms** naming "review of project N (handle H)"; review continued and prepared (189 / 207 s) |
| Status command during the review | max **192.6 s** (blocked from the first poll) | max 1 087 / 1 533 ms (one sample each, during the seconds-long packaging under `inner`), median 6 / 13 ms |
| Main-thread heartbeat during the review | **blocked 211 s** (from +0.6 s, the first status poll) | max round trip **1 / 3 ms** |
| **Close clicked +0.3 s into the apply** | refused after **38.7 s** (R1 measured 32.8 s) | **refused in 311 / 307 ms** (+0.6 s into the apply) |
| Main-thread heartbeat during the apply | **blocked 33.9 s** | max **0 / 1 ms** |
| Status during the apply | max 38.9 s | max 189 / 120 ms |
| Cancel acknowledgement ("Cancellation requested.") | 305 ms — but only reachable after the 38.7 s refusal | **307 / 313 ms**, at +1.0 s into the apply |
| Final safe cancellation (apply settled, abort shown) | +0.8 s after cancel (the preflight had already finished) | **569 / 574 ms** after cancel (preflight cancelled between files; engine aborted at `BeforeStageReplacement("primary")`; nothing changed) |
| Retry close after cancel → picker | 433 ms | 312 / 554 ms |
| Backend seam during the apply | `close_project` → `busy` (2 ms once the main thread was free) | `close_project` → `busy` in 2 ms; status 9–16 ms; `busy_projects` `[0]` / `[1,3]` (run 2 had a lingering alias, see below) |
| Screenshots | `baseline-01…04` | `after-01…04`, `after2-01…04`, `p1/p2-…-probe-after-cancel`, `commit-01/02` |

**Commit + restore on the candidate (one cycle, heartbeat running):** archive review 169.9 s;
apply with Close clicked at +0.3 s → refused in 307 ms → committed at **135.4 s**; disk
`nhl_game.db` = `343404153ae5…`, `inxp_00.big` = `4876cd2b3f4b…` — exactly the outputs the smoke
declared for this edit; "Session baseline: primary rebound (clean)". Restore of 000007: **106.0 s**
→ `90ee1ec49d36…` / `1c41f578a595…` (both originals), "Project 0: reloaded". Heartbeat max 2 / 1 / 1
ms across review / apply / restore; status max 1 082 ms (median 13). Final close 312 ms; no open
projects, no handles, no reservations, no leftovers.

**Incidental alias evidence (run `after2`).** Vite's HMR reloaded the page during the diagnostic
probe, leaving project 1 open server-side without a shell owner. The next run's apply (project 3)
reserved it as an alias (`busy_projects=[1,3]`); `close_project(1)` answered `busy` naming the
apply and `closed` after it — the reservation model observed on the desktop.

**Post-run verification:** every fixture copy re-hashed = original after the final restore
(`90ee1ec4…`, `0b262e4a…`, `1c41f578…`); seven journals (`000001–000006` aborted, `000007`
restored); engine staging `%TEMP%\nhl-modding-studio\copied-target-staging\` empty; saved app
settings restored exactly to the captured pre-run snapshot (`nhl21:ps4` → `H:\Emulators\ShadPS4`,
21 keys, no default) and verified; installed game read only; `git status anchors` empty; no
`nhl-studio`, Tauri CLI, Vite or heartbeat process remaining; ports 1420/9333 closed.

## 5. Remaining limits and blockers

* **Out-of-scope commands still run on the main thread** (players, teams, arenas detail, assets,
  Hockey Shop, historical, formulas, `load_project`/`open_project`/`extract_big_files`): each
  waits behind whichever registry lock it needs and blocks the window for that long — the same
  1.5–3 s per edit the smoke recorded. Migrating them is mechanical through `off_main` but outside
  this bounded package.
* **Packaging still holds `inner` for seconds.** `stage_workspace` (serialize → re-parse → hash the
  sessions) runs under `inner` at review time (≈1–1.5 s on the 11 MB database, up to 5.5 s in the
  smoke's NHL 18 case): a status poll or edit issued in that window waits that long (never on the
  main thread). Moving it out would need a two-phase serializer in studio-domain; not needed for
  the responsiveness target and not done.
* **A review now refuses concurrent edits on its own project** (and its aliases) with the holder
  named, for its whole duration. This is the honest form of the previous behaviour (the edit waited
  ~3 min and then invalidated the review). If a "queue the edit" UX is wanted later it needs a
  design decision; the panel already disables its own controls while reviewing.
* **`copied` is still held across `settle`/`drop_staging`** (journal abort + staging removal, sub-
  second in every run: status max ≈1.1 s once per apply/commit). Not restructured.
* **Engine text** still embeds a `\\?\` recovery path and an empty ``stage failed at ` ` `` location
  in the abort message (tdb-txn, outside this seam; recorded by SMOKE-1).
* **Evidence class:** CDP-driven real webview, debug build, no native picker, no game; the
  heartbeat is a native message-loop probe, not a human at a mouse. No in-game run.

## 6. Changed sources — SHA-256 (working tree after this package)

```text
f58cee79cb4da5731acec4e2b4983485a4c4911323a5282423b873744c7d5d53  crates/studio-domain/src/copied_target.rs        (verify_plan_layout, verify_plan_inputs_unless, PlanVerification)
75c6d600250cb1e621119674c2d66b5bab79c477b69dfa814469a4e79eec845d  crates/studio-api/src/registry/copied_target.rs  (lock scope: review reservation, apply preflight split, extend_reservation, ApplyPhase seam, docs)
447c2e958a11405a0eb67d2750841349d386a99928348967e9fc8904315dbad2  crates/studio-api/src/registry.rs                (re-export ApplyPhase)
4bac3df6d47291a1d134eb7cb2347fd8a5b9a21c19de9e5d1f091ace2d573eaa  crates/studio-api/src/lib.rs                     (re-export ApplyPhase)
515c1d0f7a322387774fca44f6f7066787f6b844bc4b8f77b040e033121ebf3c  crates/studio-api/src/commands.rs                (off_main; 30 commands → async + spawn_blocking; notes)
a3c893c42eaba373941d51bf8391a2f4140a705229ee57f7665838d7fb3f718a  crates/studio-api/tests/command_execution_context.rs (rewritten guard)
3d56d6a625b8d6cd0c6a81862ae325ad546e59d3d9d53e11d703a7e64dde8159  crates/studio-api/tests/copied_target_contention.rs  (new, 5 cases)
a1a915e4dc03d5a35706ccc9b723902e313310d81a49a5a9ca17bb533fd97d47  app/src/components/CopiedTargetPanel.tsx         (settleAppliedHandle; refresh no longer awaits invalidation)
d374f1117698272b1bc9fe289a7fd6b7fb391f2cfcc64267d9b9864779999ecd  app/src/components/CopiedTargetPanel.test.tsx    (+1 case)
```

Baseline `8d60a6d` + the pre-existing dirty tree (446 status lines; `src-tauri/src/lib.rs`'s
modification is the earlier S2-C-R2 teardown hook, not this package); index empty; nothing staged.
No DTO/binding changed.

## 7. Temporary artifacts, processes, cleanup (absolute paths)

| Item | Purpose | Owner / disposition |
|---|---|---|
| `E:\Repositories\nhl-modding-studio\local\runs\league-resp-1\fixtures\` (NHL 21 marked root: anchor db + meta + real archive copy; 7 journals incl. the 777 MB recovery backups of 000007; 1 526 MB) | disposable copied target for §4.2 | **removed** after every copy re-hashed = original, containment + reparse checks (`Test-Path` false) |
| `E:\Repositories\nhl-modding-studio\local\runs\league-resp-1\screenshots\` (16 PNG + `SHA256SUMS.txt`) | desktop evidence | **retained** for Codex review (3.5 MB with logs) |
| `E:\Repositories\nhl-modding-studio\local\runs\league-resp-1\logs\` — `*-markers.jsonl`, `*-probe-*.json`, `*-heartbeat.csv` (baseline / after / commit; 22 317 samples), `*-run.out.log`, `fixtures-MANIFEST.tsv`, `final-cargo-tests.log` | the measurements behind §4.2 | **retained** for Codex review |
| `E:\Repositories\nhl-modding-studio\local\runs\league-resp-1\settings.pre-run.json` | fresh pre-run app settings (2 762 bytes), used for the restore | **retained** with the evidence (documents what was restored); remove with the package root |
| Dev-server / Vite / app transcripts under `…\league-resp-1\logs\` | launch transcripts | **removed** (transcribed) |
| `E:\Repositories\nhl-modding-studio\src-tauri\local\runs\league-overnight-2026-09-17\target\` (**stray**, 19.8 GB, 17 203 files) | created by my first `tauri dev` launch: a *relative* `CARGO_TARGET_DIR` resolved from `src-tauri/`, and because it sat inside the watched `src-tauri/` tree the second launch looped rebuilding | **removed** (containment + reparse checks). Lesson recorded: pass an absolute target dir to `tauri dev` |
| `E:\Repositories\nhl-modding-studio\local\runs\league-overnight-2026-09-17\target\` (transferred cache; now holds the candidate `debug\nhl-studio.exe` 19:56 and `release\league-resp1-setup.exe`) | build/launch the candidate, rerun the checks | **retained** — dependency: Codex's rerun of this package's checks and desktop regression; no second cache remains |
| `H:\TEMP\claude\e--Repositories-nhl-modding-studio\a3ac1ed4-…\scratchpad\resp1-cdp\` (`cdp.mjs`, `lib-edit.mjs`, `heartbeat.ps1`, `run.mjs`, `probe-cancel.mjs`, `commit-restore.mjs`, `close-now.mjs`, `closeout.mjs`) and `…\resp1-setup\` (fixture setup crate, NHL 21 only) | reproduce §4.2 exactly (successor of the transferred helpers; the 57 obsolete step scripts were not carried over) | **retained** in this session's scratchpad while the package is under review; remove with the package |
| `H:\TEMP\…\e66483ae-…\scratchpad\smoke-setup\` and `…\cdp\` (transferred helpers) | superseded by the successors above | **removed** (dependency ended; the unrelated `edit.py`, `n.txt`, `o.txt` in that folder were not mine and are untouched) |
| `H:\TEMP\…\a3ac1ed4-…\scratchpad\edit_*.py` | one-shot source edit scripts | session scratch; no dependency; remove with the package |
| App saved settings (WebView2 localStorage `nhl-modding-studio.settings.v1`) | overwritten by the picker during the runs | **restored** exactly (verified after each session) |
| Engine staging `%TEMP%\nhl-modding-studio\copied-target-staging\` | per-review staging | empty at closeout |
| Processes: `nhl-studio.exe` (baseline and candidate), Tauri CLI node, Vite node, `cargo`/`rustc` of the looping launch, heartbeat `powershell` | §4.2 | **stopped**; none remaining; ports 1420/9333 closed. Pre-existing unrelated `node` processes (13–16 Sept) were not touched |

No `git clean`/`reset`; anchors untouched; installed game read only; no live recovery backup
touched; index empty; nothing staged.

## 8. Next review action

Codex: review the two lock-scope changes (`review_reserving`, `copied_target_apply_observed`)
and `off_main`; rerun `cargo test -p studio-api --test copied_target_contention -- --nocapture
--test-threads=1`, `cargo test -p studio-api --test copied_target --test command_execution_context`,
`cd app && npx vitest run src/components/CopiedTargetPanel.test.tsx`, and
`cargo run -p studio-api --bin export-bindings -- --check`. The desktop measurement can be
reproduced with `resp1-setup` + `resp1-cdp/run.mjs after` (absolute `CARGO_TARGET_DIR`). Stopping
here for review; no Phase 1.2/1.3, no Projects/Builds, nothing staged.

## 9. Final full-suite result

`cargo test -p studio-domain -p studio-api` on the final code (after the formatting fixes):
**52 `test result: ok` lines, 355 passed, 0 failed, 6 ignored** (the private-fixture cases), exit 0
— `local/runs/league-resp-1/logs/final-cargo-tests.log`.
