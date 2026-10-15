# LEAGUE-SMOKE-R1 — close-flow correction receipt (2026-09-17)

**Plain-language summary.** Closing a project no longer takes the user's word for it: the shell now
asks the server to release the project and only leaves the screen once the server has confirmed.
If a save into a copied game folder is still running, the server refuses, and the app now stays
exactly where it was — the running save, its progress and its **Cancel** button all remain in
reach — with the reason shown under a **Retry close** button. The same holds when a review cannot be
discarded or the request itself fails, and the "escape a damaged project" path on the start screen
behaves the same way instead of silently dropping the project. This was verified in the real desktop
app: Close pressed during a genuine two-minute save was refused with the running save's name, the
save could then be cancelled, and Close succeeded afterwards, with the server confirming the
project was gone. Still unavailable: pressing Close while a save is in its first half-minute is
answered late (about 32 s) because the save briefly holds a lock the close request waits on — the
app does not lose anything, it just freezes for that long; that belongs to the separate
responsiveness task. No game was run.

---

## 1. Package, disposition, scope

* **Package:** LEAGUE-SMOKE-R1 per [league-smoke-review-and-r1-dispatch](../validation/league-smoke-review-and-r1-dispatch.md)
  (one blocking P2: Close ignored backend refusal). Bounded to the two Close paths, the typed close
  outcome they need, regressions, and the comment correction. The accepted smoke fixes (cancel /
  status pill / messages / search box) are untouched. No broader responsiveness rewrite, no new
  phase, no live install, no staging/commit/push; the pre-existing dirty tree is preserved.
* **Disposition:** the finding is fixed with the requested component and API regressions and
  re-verified on the real desktop; developer-verified, awaiting Codex review.

## 2. The finding and its disposition

### Close disposed of its owner before (and regardless of) the server's answer — fixed

**Typed outcome (backend).** `Registry::close_project_outcome(id) -> CloseProjectDto { project_id,
outcome, reason }` with `outcome` ∈ `closed` (released now) · `already_closed` (no such open
project — idempotent) · `busy` (a copied-target apply / restore / recovery holds the session; the
reason names the holder: "apply of review handle 1 (project 1) still needs this session; wait for
it to finish or cancel it, then close again") · `failed` (a prepared review could not be discarded;
the project stays open and the staging is listed under the root's status). The boolean
`close_project` remains as `outcome == "closed"` for existing callers/tests. The Tauri command
`close_project` now returns the DTO (new export root; bindings regenerated with `--write`).

**Shared hook (frontend)** — `app/src/lib/closeProject.ts` `useAcknowledgedClose(projectId,
onClosed)`: one request in flight at a time (a ref guard — repeated clicks in one tick send one
request); each request is bound to the project id and a request generation; a response that arrives
after the project changed or after a newer request is ignored, never applied to another session;
`onClosed` (context disposal + navigation) runs only for `closed` / `already_closed`; `busy`,
`failed` and a rejected IPC call keep the owner and expose a plain-language reason.

**Shell Close** — `CloseProjectButton`: *Close* → *Closing…* (disabled) → on acknowledgement the
context is cleared and the picker shown; on refusal the button becomes **Retry close** with the
reason in a popover under it (the header layout is untouched, the page — and a running apply's
*Cancel apply* — stay as they were).

**Corruption escape** — `LingeringProjectNotice` on the picker (rendered only for the
`pick: true` navigation the CorruptionGuard uses): asks the server once on mount; "Closing the
previous project…" while pending; on acknowledgement clears the owner **without any redirect** (the
picker route stays put); on refusal/error shows the reason with **Retry close** and **Return to
project** (a plain navigation back into the still-open shell — never a re-open), and while the
project lingers the picker's *Open project* is disabled with a note, so a new session can never
silently replace the one that failed to close. No unsaved-edit confirmation was added (not in
scope).

**Comment correction.** The note above `copied_target_review` in `commands.rs` no longer claims
that sync status/cancel "never wait" on the async commands. It now records the actual constraint:
the apply's engine run holds no registry lock (hence cancel/status/close answer during it), but the
review commands hold the workspace mutex while packaging/staging, the apply's preflight holds the
registry lock while it verifies bound inputs, and every still-synchronous command that needs those
locks blocks the main thread for as long as it waits — an async attribute alone does not remove
lock contention. The desktop measurement in §4 is an instance of exactly that.

## 3. Regressions

| Suite | Cases |
|---|---|
| `app/src/components/CloseProject.test.tsx` (new, jsdom, deferred responses) | **Shell Close:** `closed` after deferred ack (nothing disposed while pending; route unchanged until ack); `already_closed` completes; `busy` keeps owner + route, shows reason, *Retry close* then succeeds (2 requests); `failed` keeps owner with the discard reason; rejected IPC keeps owner with retry; repeated clicks → one request; late response after a project switch closes neither the old nor the new project. **Corruption escape:** one request on mount, owner cleared only on ack, no redirect; refusal keeps owner on the picker with *Retry* and *Return to project* (navigation only, no re-open, no second request); retry completes without an automatic second request; rejected IPC leaves owner + retry; renders nothing when no project lingers. 12 cases. |
| `crates/studio-api/tests/copied_target.rs` (`r2_reservations_cover_aliasing_workspaces`, extended) | during the in-flight apply both projects answer `busy` naming "apply of review handle … cancel it"; after the apply `close_project(b)` succeeds, a second close of `b` and a never-opened id answer `already_closed`, a fresh project answers `closed` then `already_closed`. |
| `crates/studio-api/src/bindings.rs` | export-root count 80 → 81 (`CloseProjectDto`). |

## 4. Desktop re-exercise (real Tauri build over CDP, same method as the smoke; NHL 21 PS4 copy with the real 777 MB `inxp_00.big` copy; not a human run, not a game)

| Case | Observed |
|---|---|
| Ordinary prepared-review close | edit → review prepared (handle 0 / txn 000001) → *Close*: picker after **308 ms**. Backend seam (`__TAURI_INTERNALS__.invoke` from the page): `close_project(0)` → `already_closed`; `project_summary(0)` → "no project with id 0"; root status: no pending handles, *ready* (the prepared handle was discarded with the session). Screenshot `50-r1-ordinary-close.png`. |
| Close during a real apply (run 1) | archive review (169 s) → *Apply* → *Close* clicked ≈0.3 s in → refusal shown at **+32.8 s**: popover "The project is still in use by a running operation, so it was not closed. apply of review handle 1 (project 1) still needs this session; wait for it to finish or cancel it, then close again"; URL unchanged; header still the project; **Cancel apply visible**; *Retry close* visible. Seam during the apply: `close_project(1)` → `busy` (same reason); status `pending_handles=[{handle 1, in_progress: true}]`, `busy_projects=[1]`. *Retry close* while still applying → refused again, stays. *Cancel apply* → "Cancellation requested." → apply aborted (+72 s) → root *ready*, edits kept → *Retry close* → picker after **317 ms**. Seam after: `already_closed`, no handles, no busy projects, no reservations. Both fixture files at their originals (`1c41f578…`, `90ee1ec4…`). Screenshots `51-r1-close-refused-during-apply.png` (first layout: the reason spilled across the header tabs — fixed to the popover), `52-r1-after-cancel.png`, `53-r1-closed.png`. |
| Close during a real apply (run 2, popover layout) | same sequence: refusal at **+32.7 s** rendered as the popover under *Retry close*; cancel → settled at +35.7 s; *Retry close* → picker after 310 ms; seam `already_closed`, `busy_projects=[]`. Screenshot `54-r1-refusal-popover.png`. |

**Measured limitation (recorded, not fixed here):** the synchronous `close_project` (and any other
sync command) is answered only once the apply's preflight releases the registry lock it holds while
verifying the 777 MB bound input's identity (~32 s on this host) — during that window the main
thread, and therefore the window, is blocked; the click is not lost and nothing is disposed of. This
is the lock contention the review scoped to the separate bounded responsiveness task.

The corruption-escape path was not reproduced on the desktop (it needs a structurally corrupt
database); it is covered by the component cases above.

## 5. Changed sources — complete SHA-256 manifest (working tree after this correction; LEAGUE-SMOKE-1 files included since its receipt only listed prefixes)

```text
55f3dd9a8e182eee28668383d4f4d202e909b0c18738e963797ae10c3575402d  crates/studio-api/src/commands.rs            (R1: typed close command, lock-constraint note)
3eb88de44a7181d485f4cc569e86feacec2ffa8093cb7af4d80ae2f3c5830271  crates/studio-api/src/registry.rs            (R1: close_project_outcome)
75a3d77f5a9e52995e1fe5c006fd4e6aea587d318faff89f494ecdf970d8be1c  crates/studio-api/src/dto.rs                 (R1: CloseProjectDto)
68d79f137caea9eafa915960e6c40d102df73c4b813f8afd07f2212f64c8b284  crates/studio-api/src/bindings.rs            (R1: export root + count)
0a0e242511da1a796dbf4666118fb191c09c47e69f318404ec32d04de0ca0eda  crates/studio-api/tests/copied_target.rs     (R1: outcome regressions)
a1016879daf795cbaca64ba35ba3a43cf01f49540e95d642e0076e15a7c89090  app/src/bindings/CloseProjectDto.ts          (R1: generated, --write)
481260e037ebd62eb4411b56f1f10497d5f153fdbaa9262bda41bf4f59d8266f  app/src/lib/client.ts                        (R1: close_project ret type)
2ca38b8e8f205cebf569e8a70594c865333df0d6df3c6e6fb3621f9b8cbeaa07  app/src/lib/closeProject.ts                  (R1: new hook)
d25265ace662fce47c0a8b4e723820eaf1788667120a21f3c13bf185a41e9053  app/src/components/CloseProjectButton.tsx    (R1: new)
427359c7f25f46096194fb802e50e2b1104635cb0d96062753aac6ce75cbaf77  app/src/components/LingeringProjectNotice.tsx (R1: new)
582b6c1624d5246fdd627306365b2575518aeecae9b5d27c2702ecccfe3e12e2  app/src/components/CloseProject.test.tsx     (R1: new, 12 cases)
7c079685510e92582ca95050afb99e96fb46fc3281f266b28c8c2e5210355483  app/src/AppShell.tsx                         (R1: CloseProjectButton)
c771383c0c69b00e8548c7d2ed570dafeaa94de99f13a82cde81e7c52c9fbd58  app/src/pages/OpenProject.tsx                (R1: LingeringProjectNotice, escapeBlocked)
7c8be44674f9da6f19eef7fec8bc2fc41eb2a4ff5e02648874bcd41c7730f775  crates/studio-api/src/registry/copied_target.rs (SMOKE-1, unchanged by R1)
dad6fe8d16db3dbfcbb0443adb12b64e0fb9211890c7c14007ace2823c32ac17  crates/studio-api/tests/command_execution_context.rs (SMOKE-1)
28ff9a31a163011e972a8fabb8a3cec65bf2350f209811ddcc5a8dcec96f5d27  crates/studio-api/tests/league_copied_target.rs (SMOKE-1)
737bf73e6e4f4e27a03b052a746d2779f744de76e4986bb4fd5481dfebc28aa8  crates/studio-api/tests/leagues.rs           (SMOKE-1)
f0e95101a266a2836ae6bd471ed380364007030fd6358fff224a494650c984a1  crates/studio-domain/src/session.rs          (SMOKE-1)
9f031902fc665964453149014737c44f7d8bcf42d9b8700b3f7e9af34db3d9dd  crates/studio-domain/tests/leagues.rs        (SMOKE-1)
e8dc3fe89a30e55f036e723870c847324a412608170ea859895f29523a9f4779  crates/tdb-edit/src/lib.rs                   (SMOKE-1)
7f1085f3254a54fdda5f8ab8e438e27d6859dc3c2f9621facf89da6e498dd646  app/src/components/CopiedTargetPanel.tsx     (SMOKE-1)
8dccf61c2b1290781434df1d731e7f01c25a266ad23e82561fe7b69321b07a1d  app/src/components/CopiedTargetPanel.test.tsx (SMOKE-1)
0f0fd630cbb047d54877aeb3a137635d4a2055b23d1578d2120a3185ba94a0fd  app/src/components/EntityList.tsx            (SMOKE-1)
```

Baseline `8d60a6d` + the overnight/R1/R2/SMOKE-1 working tree; index empty; nothing staged.

## 6. Checks executed (developer, this host, `--target-dir local/runs/league-overnight-2026-09-17/target`)

| Class | Command | Result |
|---|---|---|
| API | `cargo test -p studio-api --test copied_target --test command_execution_context --test league_copied_target --test leagues` | 15 / 1 / 7 / 3 ok (close-during-apply regression passing with the new outcome assertions) |
| API | `cargo test -p studio-api --lib`; `--test registry --test roster_builder` | 31 ok (drift gate + root count 81); 6 / 1 ok |
| Bindings | `export-bindings -- --write` then `-- --check` | 1 new root installed; "Bindings are in sync" |
| Structural | `cargo clippy -p studio-api --tests`; `cargo check -p nhl-studio`; `rustfmt --check` on the changed test file | no warnings; shell compiles (and was rebuilt/launched by `tauri dev`); clean |
| Component | `cd app && npx vitest run` | 6 files, **55 passed** (12 new) |
| Frontend | `npx tsc --noEmit -p app`; `npx vite build` | clean; built |
| Desktop | §4 | executed as recorded |
| Not rerun | studio-domain full suite, the 28-row campaign, real-archive `#[ignore]` cases | no domain/engine change in this package |
| In-game | — | not executed, not authorized |

## 7. Limitations

* Sync commands still wait on the workspace/registry mutexes (the ~32 s close answer during an
  apply's preflight above); separate bounded task.
* Close with unsaved edits still asks nothing (no confirmation policy was requested).
* The desktop evidence is CDP-driven, debug build, no native picker, no game; the corruption escape
  is component-tested only.

## 8. Scratch, ownership, cleanup

| Item (absolute) | Purpose | Disposition |
|---|---|---|
| `E:\Repositories\nhl-modding-studio\local\runs\league-smoke-1\fixtures\` (recreated for §4: 3 marked roots incl. the real archive copy; 3 aborted journals) | disposable copied targets | **removed** after every copy re-hashed = original, containment + reparse-point checks; `Test-Path` false |
| `E:\Repositories\nhl-modding-studio\local\runs\league-smoke-1\screenshots\` (51 + 5 new = 56 PNG) | desktop evidence (§4 + the smoke receipt) | **retained** for this correction's review |
| `E:\Repositories\nhl-modding-studio\local\runs\league-smoke-1\settings.pre-run.json` | exact pre-run app settings; used again to restore the picker's saved paths after §4 (`nhl21:ps4` → `H:\Emulators\ShadPS4`, no `nhl14:ps3`; verified) | **retained** with the screenshots |
| `E:\Repositories\nhl-modding-studio\local\runs\league-smoke-1\r1-dev.out.log`, `r1-dev.err.log` | dev-server transcripts | **removed** |
| `H:\TEMP\claude\e--Repositories-nhl-modding-studio\e66483ae-2c51-4f71-9688-ef42374df4fa\scratchpad\smoke-setup\` (fixture setup crate, uses the engine helper) and `…\scratchpad\cdp\` (CDP driver + step scripts `s0–s33`, `r1-a…r1-d`) | reproduce §4 exactly | **retained** while this correction is under review (session scratchpad, outside the repo); remove with the package |
| `E:\Repositories\nhl-modding-studio\local\runs\league-overnight-2026-09-17\target\` | transferred build cache (candidate build + `release\league-smoke-setup.exe`) | **retained** — dependency: Codex's rerun of this correction's checks; no second cache created |
| App saved settings (WebView2 localStorage) | overwritten by the picker during §4 | **restored** (verified) |
| Engine staging `%TEMP%\nhl-modding-studio\copied-target-staging\` | per-review staging | empty at closeout |
| Processes (`nhl-studio.exe`, Tauri CLI node, Vite node, `cmd` launchers) | §4 | **stopped**; port 9333 closed; none remaining |

No `git clean`/`reset`; anchors untouched; installed game read only; index empty.

## 9. Next review action

Codex: review `close_project_outcome` and the two close paths; rerun
`cargo test -p studio-api --test copied_target r2_reservations_cover_aliasing_workspaces`,
`cargo test -p studio-api --lib bindings`, and `cd app && npx vitest run
src/components/CloseProject.test.tsx`. Then the bounded responsiveness/lock-scope task (the ~32 s
preflight window and the review-time workspace mutex are its first two measurements) or the
separately authorized runtime plan. Stopping here.
