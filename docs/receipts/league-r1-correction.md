# LEAGUE-R1 — correction receipt (2026-09-17)

**Plain-language summary.** Three problems found in the overnight league work are fixed. First, when
a save writes both the loose database and a copy of the game archive, both now come from exactly
the same packaged snapshot, so they can never disagree. Second, Studio now refuses to point a team at
a league that does not exist on a PS4 title — the edit is rejected with a plain message and nothing
changes — while older consoles (which have no league table) and databases that already contain such
a stray value keep working as before. Third, the "save to a copied target" panel now locks its
settings while a review is prepared, recognises a stale review by the server's own content
revision, and disposes of reviews that finish after the panel was closed or the project switched;
any handle it cannot dispose of stays visible with a Discard button. Still not done: no desktop or
in-game run was performed (the panel was exercised only under an automated browser-like harness),
and the review's other limits remain — this correction does not close Phase 1.1.

---

## 1. Package, disposition, scope

* **Package:** LEAGUE-R1 per [league-overnight-review-and-r1-dispatch](../validation/league-overnight-review-and-r1-dispatch.md)
  (F1 P1, F2 P1, F3 P2). Bounded correction; no feature expansion, no new phase, no live install,
  no staging/commit/push. Dirty tree and unrelated work preserved.
* **Disposition:** all three findings addressed with the requested regressions; developer-verified;
  awaiting Codex review. Not independently accepted; Phase 1.1 and the overnight queue remain open.

## 2. Findings and their disposition

### F1 — archive and loose database from one snapshot (fixed)

`Registry::copied_target_review_archive_member` no longer reads the session a second time. Inside
the review's extension closure — which runs under the workspace lock, after `stage_workspace` has
packaged the sessions — the archive member's bytes are read from **the plan's own primary
artifact** (`plan.artifact(KEY_PRIMARY).artifact.replacement`), its identity is re-checked against
the artifact's recorded `replacement_identity`, the artifact must be the `Primary` role, and the
bytes must parse as a bare TDB (a packaged save container or PS4 roster stream is refused as an
archive member). There is no earlier capture, so no interleaving can produce two revisions; the
existing apply-time revision check still refuses an edit made after review.

Regression `archive_member_bytes_come_from_the_plan_snapshot` (`crates/studio-api/tests/league_copied_target.rs`):
revision A reviewed with the archive → edit → apply refused, loose file and archive untouched, root
free; revision B reviewed and applied → decoded archive member ≡ loose destination ≡ the plan's
primary output identity; session clean afterwards. Existing S2-C regressions (`copied_target`, 15)
rerun green; the closure signature gained the staging directory (three test call sites updated).

### F2 — dangling PS4 league references (fixed, reproduced with the reviewer's probe)

* New `defs/relationships/team-league.toml`: `kOtt.xMjj → fsLK.aQrQ`, `no_null_sentinels = true`
  (0 is the NHL row, 31 is a plain code — neither is a null). `RelBinding` gained the
  `no_null_sentinels` flag and `audit_edge` honours it; every other edge keeps the generic rule.
  The edge is table-gated by the audit itself: `fsLK` exists only on PS4, so gen6/gen7 databases
  never see it.
* `EditSession::run` now applies a **delta referential gate** after every committed operation (single
  cell, multi-cell, structural ops, imports all go through `run`): if any confirmed edge gained a
  dangling reference relative to the pre-op bytes, the op is refused with a human message
  ("Team ANA: league = 31 refers to no stockleagues row on this title (team-league relationship);
  the edit was not applied") and working bytes, undo history and pending state stay untouched.
  Pre-existing dangling values are preserved and browsable (raw code, member count), unrelated
  edits and edits to the dangling row itself are accepted, and moving a row to a valid code then
  back is a *new* violation (refused) — the same delta semantics the op-time audit already uses.
* Reviewer probe rerun (unchanged source, SHA `0050b346…`): `SET TEAM 0 LEAGUE TO ABSENT 31:
  Err(Domain(Validation("Team ANA: league = 31 refers to no stockleagues row …")))`; the following
  apply committed a no-op set.

Regressions: `dangling_league_reference_is_refused_before_it_can_be_saved` (API: refusal, no
dirty/undo/pending/destination change, valid reference accepted, pre-existing violation cases),
`gen7_league_codes_are_not_subject_to_the_ps4_rule` (Legacy), and
`nhl21_dangling_league_reference_refused_on_every_edit_seam` (domain: `set_field` and
`set_cells_int`). `tdb-relations` registry test updated (9 defs; the new binding resolves with the
flag).

### F3 — frontend review ownership (fixed)

`app/src/components/CopiedTargetPanel.tsx` rewritten around a **bound review**: `{review,
projectId, revision, config}` where `revision` is the new `ProjectSummaryDto.workspace_revision`
(the server's content revision, the value the handle is bound to) and `config` is the full root /
archive-enabled / archive path / member path set. While a review is prepared every configuration
control is disabled (frozen); a review is stale when project, revision or configuration differ
(Apply disabled with the reason; the undo-depth heuristic is gone). A request generation
(bumped on unmount and project change) plus mounted/project refs make a review that completes late
dispose of its handle immediately instead of adopting it; `refresh` never applies a summary whose
`project_id` is not the current one; apply/discard errors and late-disposal failures are recorded
under "cleanup failures" and any handle the panel does not own is listed from the root status with
its own Discard button (the durable owner path). Cancel-while-applying is unchanged.

Component tests (`CopiedTargetPanel.test.tsx`, jsdom, deferred API mocks): late review after
unmount → discarded, never applied; late review after project switch → discarded, no new-project
update; controls frozen while prepared and unfrozen on discard, archive option then routes to the
archive command; revision change with identical undo/redo depths → stale, Apply inert; cancel
available during apply and the refresh targets the owning project only; failed disposal surfaced
and foreign handles listed. Dev tooling added for this: `jsdom`, `@testing-library/react`,
`@testing-library/dom` (devDependencies).

### Receipt corrections (dispatch)

The overnight receipt now carries a correction banner: labels are confirmed only from PS4 league
tables (gen7 labels are candidate/corroborated, some codes raw); gen7 dictionary decoding does not
establish code→key resolution; NHL 19/21 dictionaries are missing from the supplied fixtures, not
absent from the games; the checkpoint file's hash is `2ddc047fd27613010bbd7dce316510a803d3c2a7fd480c790e251a95e32b8de2`.

## 3. Changed paths (SHA-256 prefix)

```text
e39286469b0d27e2  crates/studio-api/src/registry/copied_target.rs   (F1; extend closure gets staging dir)
99b8b2ed8d1fe18f  crates/studio-domain/src/session.rs               (F2 delta gate, entity labels)
6175309d00f3d23e  crates/tdb-relations/src/lib.rs                   (no_null_sentinels)
51287a8b9e31e091  crates/tdb-relations/tests/relations.rs
58e9acf05a71907d  crates/tdb-domain-validation/src/lib.rs           (audit_edge honours the flag)
323474fbb8cb56dc  defs/relationships/team-league.toml               (new)
57b5fdb69be58325  crates/studio-api/src/dto.rs                      (workspace_revision)
032b0a2cda714db7  crates/studio-api/src/convert.rs
0adc1bf8dbcb5453  crates/studio-api/tests/league_copied_target.rs   (+3 regressions, closure arity)
e6ed74faf4913909  crates/studio-domain/tests/leagues.rs             (+1 regression)
327a3481b105f1d0  app/src/components/CopiedTargetPanel.tsx          (rewritten)
6172e9a006d6a9c6  app/src/components/CopiedTargetPanel.test.tsx     (new, 6 cases)
e3598493b0274d4b  app/src/bindings/ProjectSummaryDto.ts             (generated, --write)
aa0f5f2b3ed465a9  app/package.json                                  (3 devDependencies)
3172a4d7705b28d4  app/package-lock.json
9137f99d673fda9c  docs/receipts/league-overnight-2026-09-17.md      (correction banner)
```

Baseline unchanged (`8d60a6d` + overnight working tree). Bindings installed only via
`export-bindings -- --write`; `--check`: "Bindings are in sync".

## 4. Fresh checks (developer, this host, `--target-dir local/runs/league-overnight-2026-09-17/target`)

| Evidence class | Command | Result |
|---|---|---|
| API / domain (full) | `cargo test -p tdb-relations -p tdb-domain-validation -p tdb-enums -p studio-domain` | 44 ok-lines + `tdb-relations` (count assertion updated, rerun 4+4+3 ok); 0 failures |
| API (full) | `cargo test -p studio-api` | 10 ok-lines, 0 failures (incl. `copied_target` 15, `league_copied_target` 6, `leagues` 3, `league_offline_campaign` 1 — the campaign reran the 28 fixture rows) |
| Focused rerun after formatting | `studio-domain --test leagues --test pending --test session --test copied_target`; `studio-api --test league_copied_target --test copied_target --test leagues` | 7 / 3 / 11 / 6 and 6 / 15 / 3 ok |
| Reviewer probe | `cargo run --offline --manifest-path local/reviews/league-overnight-probe/Cargo.toml …` | edit refused (see F2) |
| Structural | `export-bindings -- --check`; `cargo clippy` (tdb-relations, tdb-domain-validation, studio-domain, studio-api, `--tests`); `cargo check -p nhl-studio`; `rustfmt --check` on touched files | in sync (80 roots; the changed `ProjectSummaryDto` is an existing root); no warnings; shell compiles; clean |
| Component (jsdom) | `npx vitest run` | 5 files, 39 passed (6 new) |
| Frontend build | `npx tsc --noEmit`; `npx vite build` | clean; built |
| Desktop | — | **not executed** (no driver/display automation available in this environment) |
| In-game | — | not executed, not authorized |

**Manual desktop smoke (unexecuted gate, exact steps):** ~~`npm run tauri dev` in `app/`~~ (corrected by LEAGUE-R2: `app/package.json` has no `tauri` script; the launch is `npm run dev` from the repository root, see [league-r2-correction](league-r2-correction.md)); open the
NHL 21 copy under a marked copied root; Leagues → Allsvenskan → set name to `TXT_ROW_LEAGUE`;
Pending Edits: the row appears; in the copied-target panel confirm the root is prefilled, click
Review save → the root/archive controls grey out and the artifact table shows; make any further
edit → the review shows "stale", Apply is disabled; Discard → controls re-enable; Review again →
Apply → outcome shows "committed … unqualified" and the file list; Restore the transaction → the
Leagues page shows Allsvenskan again; navigate away while a review is prepared and confirm the root
status shows no leftover handle for this project.

## 5. Limits

Everything in §2 is API/domain/component evidence; no desktop or in-game run. The PS4 archive
output remains structurally unqualified (unknown `+0x0C` row word) and runtime acceptance is
untested. The referential gate covers the confirmed relationship defs only; the `leaguegroup` /
`conferencegroup` / `divisiongroup` ids have no evidenced target table and are not gated.

## 6. Scratch and ownership

* Retained (review dependencies): `local/runs/league-overnight-2026-09-17/target/` (build cache),
  `local/reviews/league-overnight-probe/` and `local/reviews/league-invalid-reference-52016/`
  (Codex-owned reviewer material, untouched).
* Removed by this correction (literal paths, containment-checked): the fixture my probe rerun
  created, `local/reviews/league-invalid-reference-47460/` (one committed no-op transaction);
  `r1_backend.py`, `r1_tests_append.rs`, `r1-domain-tests.log`, `r1-api-tests.log` under the
  package root. Engine staging root empty; `%TEMP%\nhlms-p3-op-24084` left unowned as instructed.
* Installed games read-only; anchors untouched (`git status anchors` empty).

## 7. Next review action

Codex: review F1–F3 dispositions and the regressions; rerun
`cargo test -p studio-api --test league_copied_target --test copied_target` and
`cd app && npx vitest run src/components/CopiedTargetPanel.test.tsx`; perform the desktop smoke
above if tooling permits. Stopping here.
