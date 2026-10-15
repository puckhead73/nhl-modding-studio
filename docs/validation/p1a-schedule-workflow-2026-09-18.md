# P1-A — cross-title source schedule workflow (receipt)

18 September 2026. Package [P1-A](overnight-dev-r3-acceptance-and-p1a-dispatch.md) under the
[working agreement](../plans/coordinator-developer-working-agreement.md). Status:
**R1 (§9) and R2 (§10) corrections applied 18 September / developer-verified / Codex review pending.** Nothing
launched, no live install or image written, no staging, no commit, no push.

> **Correction note (18 September, R1 F1).** The original receipt claimed that "preview → apply
> races are refused rather than misapplied". That was **false**: Apply recomputed the plan from
> the current inputs and only the import path compared a slot-map fingerprint, which does not
> cover schedule contents or a changed file — the reviewer reproduced a previewed no-op import
> silently overwriting a later edit. §9 records the fix: previews are now held server-side under
> a review id bound to the workspace revision (primary + roster + dictionary identities), the
> operation inputs and, for imports, the file bytes; Apply consumes the id and never recomputes.
> Sections 3–4 below describe the original package and are left as written; where they disagree
> with §9, §9 is current.
>
> **Correction note (18 September, R2).** §9.1 claimed that a "foreign project id" was refused.
> That was **too broad**: review ids were plain per-workspace counters starting at 1, so a ticket
> minted by another workspace (or by an earlier instance of the same file) could alias a locally
> held review and apply the *local* plan — the reviewer reproduced it with two Legacy workspaces.
> The R1 test only proved that an *unknown* id is refused. §10 records the fix: tickets are opaque
> strings namespaced by the issuing workspace instance, checked before lookup.

## Plain-language summary

Every league's season schedule can now be edited in Studio, not just looked at. On the league
page you can change a game's home team, away team or date and see, before anything is written,
whether the season still holds together (games per club, no team playing itself, real calendar
dates, the running order); you can export the whole season as a CSV or JSON file, edit it
outside Studio and bring it back through a team-mapping step that asks you explicitly whenever a
team in the file cannot be matched; and for leagues whose season is a whole number of
round-robins (the Swedish, Czech, German and NHL 07 tables, for example) you can generate a
fresh balanced season. The NHL's own 82-game season is *not* generated — its division-weighted
rules are not established, and Studio says so with the exact numbers instead of guessing. NHL 07,
previously "blocked", turned out to be a Studio classification gap: its four season-length tables
now edit like any other. Every change is one undo step and shows up in the pending review; a
refused change writes nothing. Nothing here has been seen in a running game yet.

## 1. Package, disposition, scope

P1-A as dispatched, items 1–6, all delivered within limits. Scope: source schedule tables of the
main database on every family that ships them (Xbox 360 07–Legacy, PS3 08–Legacy, PS4 15–21,
Recompiled by file identity). Out of scope and untouched: mode-local copies
(`leaguescheduletable`), Season/Franchise initialization, row-count changes, the stock NHL
scheduling rules, runtime acceptance.

## 2. Changed paths (baseline `8d60a6d`, dirty working tree preserved)

The working tree carried ~560 modified/untracked paths from earlier packages before this one; none
were touched except as listed. Hashes are SHA-256 of the working-tree file, CRLF as on disk.

| Path | Change |
|---|---|
| `crates/tdb-edit/src/txn.rs` | + `EditTransaction::set_cells_int_many` — many cells across many records in **one** repairing transaction (the per-cell path re-clones and re-parses the database per write; a 1 231-row import needs one parse, one audit) |
| `crates/studio-domain/src/session.rs` | + `set_cells_int_rows` (metadata-validated multi-row transaction, one undo entry), `schedule_model`, `schedule_export_context` |
| `crates/studio-domain/src/workspace.rs` | + `set_cells_int_rows` (roster mirror rules as `set_cells_int`), `schedule_edit_preview/apply`, `schedule_export`, `schedule_import_preview/apply`, `schedule_generation_options`, `schedule_generate_preview/apply`; apply refuses a plan with errors and a stale slot-map fingerprint |
| `crates/studio-domain/src/schedules.rs` | `SlotMap`/`SlotEntry` (abbrev, export key, global index, FNV fingerprint), `ScheduleLayout` (actual bit widths), NHL 07 thin-table join for slot resolution, NHL 07 variant-stem fix (`nhlschedulelongnoleap` classified as stem `nhl`), `intlelitserien`/`intlsmliiga` stems, exact-then-containment league key matching (PS4 `TXT_LEAGUE_O2_EXTRALIGA` etc.), `champions`/`spengler` deliberately **unbound** |
| `crates/studio-domain/src/schedule_edit.rs` | **new** — proposal model, whole-table validation (errors vs warnings), single-game edit plan, CSV/JSON export + parser, import key resolution + explicit mapping + stale-fingerprint refusal + exact-row-count refusal, deterministic circle-method round-robin generation with kept rows merged by season date; file format documented in the module header |
| `crates/studio-domain/src/pending.rs` | schedule rows labelled `nhlschedule game 0 (Oct 7)` in the pending review instead of `#0` |
| `crates/studio-domain/src/lib.rs` | `pub mod schedule_edit` |
| `crates/studio-domain/tests/schedule_edit.rs` | **new** — 11 cases (10 anchor, 1 private NHL 20) |
| `crates/studio-api/src/{dto.rs,convert.rs,registry.rs,commands.rs,lib.rs,bindings.rs}` | 16 DTOs (11 export roots), converters, 8 registry methods (export writes to `out_path`; import reads inline text or a `path`), 8 Tauri commands; root-count guard 99 → 110 |
| `crates/studio-api/tests/schedule_edit.rs` | **new** — registry seam: preview writes nothing, apply = one undo step named in the pending review, export→file→import no-op, explicit mapping, refusals, generation |
| `app/src/bindings/*.ts` | regenerated with `export-bindings --write` (16 new `Schedule*Dto.ts`, `ScheduleViewDto.ts` changed); `--check` clean |
| `app/src/lib/client.ts` | 8 command signatures + request arg shapes (plain numbers for the u64 fields) |
| `app/src/components/LeagueScheduleCard.tsx` | row editor (home/away pickers from the slot map, calendar date, Preview → Apply), Export CSV/JSON (save dialog), Import… (open dialog, key → team mapping table, preview/apply), Generate… (options, blocker, participants, preview/apply) |
| `app/src/components/LeagueScheduleCard.test.tsx` | **new** — 4 jsdom cases |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | §1.2 schedule rows rewritten per target family with the named cases; new binding row; P1-A package row |
| `docs/receipts/overnight-development-2026-09-18/MORNING-REVIEW.md` | schedule cards per title updated to the P1-A surfaces (Studio-only, no placement); runtime card 2a |
| `docs/validation/p1a-schedule-workflow-2026-09-18.md` | this record |

Hashes: see §7 (computed at the end of the session; the file list above is the authority).

## 3. Behaviour and decisions (per dispatch item)

1. **Target availability.** Every anchor family has schedule tables; NHL 07's four NHL length
   variants and three `intl*` tables were misclassified by Studio (stem `nhlschedule` instead of
   `nhl`; no `intlelitserien`/`intlsmliiga` stems; slot resolution read the thin `kOtt` record
   instead of the stock row) — fixed, so NHL 07 is now "impl+tested", not "blocked". The real NHL 20
   database (private fixture `1e200478…`) is exercised in an `--ignored` case, never the mislabelled
   anchor. PS4 league keys such as `TXT_LEAGUE_O2_EXTRALIGA` now match by exact key first, then
   containment, so Czech/Swiss/Austrian/ECHL resolve on PS4 18+. Tables whose stem binding is
   contradicted by the data (`championsschedule`: 32 slots vs an 8-member league; `spenglerschedule`:
   12 vs 6) and the tournament tables (`wcschedule`, `worldchamp`, `worldjunior`) are left **unbound**
   and browse read-only — a research question, recorded in the matrix, not an invented mapping.
2. **Editor.** Slots = rank within the league by the team table's global `index`, never id/index/
   record order (verified on the NHL 10 AHL and NHL 11 OHL exceptions, both platforms). Each field's
   width comes from the table's own descriptor (5-bit gen7/PS4 15–17, 6-bit PS4 18+, 4-bit European);
   a slot that does not fit is a refusal. Validation separates **errors** (refuse: impossible date,
   self-game, slot out of width, unresolved slot *in the touched row*, non-member team) from
   **warnings** (disclose: Feb 29 in a yearless table, out-of-season-order rows, uneven counts,
   zero-game members, home/away imbalance, non-zero round/status, pre-existing oddities in untouched
   rows). Season order is the yearless Sep → Aug key; no season year is ever invented.
3. **Import/export.** Format `nhl-studio-schedule/1` (CSV with header or JSON; calendar month/day;
   teams by key = abbreviation, or `ABBR#id` when not unique; `SLOT#n` for unresolved slots). Keys
   resolve as `explicit` (mapping) → `auto` (exact key, numeric team id, unique abbreviation) →
   `ambiguous`/`unmapped` (refused until mapped). Two keys → one team is refused. The mapping is
   pinned to the slot-map fingerprint (FNV over rank:team:index) — a membership/ordering change in
   between refuses the apply. The file must hold exactly the table's active rows; short or long
   files are refused with both counts and the reason (no truncation, no growth, no filling of PS4
   spare rows). Fields not in the file (`round`/`status`) keep their current values; unrelated
   rows/tables are untouched (only cells whose value changes are written).
4. **Generation.** Balanced round-robin (circle method; venue by `(cycle + a + b)` parity so an
   even number of round-robins is exactly balanced) for an explicit participant set, `rounds`, start
   date and matchday spacing; rows involving a non-participant slot (All-Star row) are kept verbatim
   and merged by season date; the running index is rewritten 0..N-1; `round`/`status` written as 0.
   The result must equal the table's active rows exactly, else the plan is refused with the arithmetic
   (`14 participants × 3 = 273 … 364 active rows`). The NHL (1 230 / 435 pairings) and Swiss (300 /
   66) tables are blocked with that message; edit/import remain. No stock scheduling rule and no
   mode-local survival is claimed anywhere in the UI text or docs.
5. **Tests** — see §4.
6. **Docs** — matrix and review cards updated to the tested scope only; no runtime acceptance marked.

Architecture: all writes go through the existing `EditSession::run` harness (repairing transaction,
delta referential gate, undo/redo, pending diff, every save/placement route). Previews are pure
functions over a `ScheduleModel` read from the working bytes; apply re-derives the plan and checks
the fingerprint again, so preview → apply races are refused rather than misapplied.

## 4. Checks executed (developer; no prior independent review of this package)

| Check | Result |
|---|---|
| `cargo test -p studio-domain --test schedule_edit` | 10 passed, 1 ignored (NHL 20 private) |
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test schedule_edit -- --include-ignored` | **11 passed** (real NHL 20 `nhl_game.db` `1e200478…`, meta `2fac0e73…`, identity asserted) |
| `cargo test -p studio-domain --lib -- schedule_edit` | 3 passed (dates, circle method, parser refusals) |
| `cargo test -p studio-domain --test schedules --test leagues --test membership` | 2 / 11 / 4 passed |
| `cargo test -p studio-api --test schedule_edit` | 1 passed |
| `cargo run -p studio-api --bin export-bindings -- --check` | in sync after `--write` |
| `cargo test -p studio-domain -p studio-api -p tdb-edit --no-fail-fast` | see §7 (run at the end of the session) |
| `npx tsc --noEmit` | clean |
| `npx vitest run` | 11 files, 83 passed (incl. the 4 new card cases) |

Anchors exercised by name: Xbox 360 Legacy/15/11/09/07, PS3 Legacy/12/11/10/08, PS4 15/16/17/18/21,
plus the private NHL 20 install DB. Recompiled is covered by file identity with Xbox Legacy only.
Per-title cases the suites do **not** contain (recorded as "impl, test missing" in the matrix):
import/generation on X360/PS3 08–11, generation *apply* on PS4, NHL 07 import.

Untested claims: none are made about the runtime. The UI was exercised in jsdom only, not in the
desktop shell.

## 5. Blockers and limitations

- **Row count is fixed.** Whether the engine reads `current` or `max` on a shorter table is
  unverified (research 12 §1.4), and PS4's spare rows have no established "active" representation;
  Studio therefore refuses any proposal that does not fill exactly the active rows. Custom season
  lengths on gen7 need table growth (`tdb-edit` capacity question) *and* that runtime answer.
- **NHL/AHL/Swiss/Finnish/Austrian generation is blocked** by arithmetic (not a whole number of
  round-robins) — the stock division-weighted formats are not established. `nhlfutureschedule`
  (second Franchise season) is not regenerated with `nhlschedule`; the plan discloses the sibling.
- **Unbound tables:** `championsschedule`, `spenglerschedule`, `wcschedule`, `worldchampschedule`,
  `worldjuniorschedule` (browse only; research question). `ahl31/ahl32/nhl32` expansion placeholder
  slots are disclosed as unresolved and never refused when untouched.
- **Mode-local copies** and Season/Franchise survival: not measured (P1-B / runtime card 2a).
- The pending review labels a schedule row by game index and date; slot → team names stay on the
  card (the review has no league catalog at hand).

## 6. Temporary artifacts and processes

| Path | Purpose | Owner | Disposition |
|---|---|---|---|
| `H:\TEMP\claude\…\d9e98828…\scratchpad\patch_*.py` (5 files), `cargo-test-full.txt` | patch scripts used to edit source; test log | this session | disposable; removed at closeout (§7) |
| `%TEMP%\nhl-studio-p1a-<pid>\`, `%TEMP%\nhl-studio-p1a-api-<pid>\` | temp copies written by the copied-output tests | the test cases | removed by the tests themselves |
| `target\debug` | existing shared build cache reused; no new target tree created | shared | retained (not package-owned) |
| `crates/studio-domain/tests/zz_dbg.rs` | throwaway probe (NHL 07 / PS4 league keys) | this session | **removed** during the session |

No processes spawned beyond `cargo`/`node`; none left running. `local/runs/*` untouched. Disk: E:
had ~5.3 GB free at start; no new large trees.

## 7. Closeout figures

Full suites, `--no-fail-fast`, run after every edit in this record (developer results):

| Crate | Binaries | Passed | Failed | Ignored |
|---|---|---|---|---|
| `tdb-edit` | 7 | 41 | 0 | 0 |
| `studio-domain` | 45 | 334 | 0 | 8 (private-fixture cases, explicit) |
| `studio-api` | 16 | 90 | 0 | 3 (private-fixture cases, explicit) |

The ignored cases are the explicit private-fixture skips; the P1-A NHL 20 case among them was
additionally executed with `NHL_STUDIO_FIXTURES` set (§4). The pre-existing untracked
`crates/studio-domain/tests/zz_diag.rs` (7 Sep, not this package's) runs inside the domain suite
and was left in place.

SHA-256 (first 16 hex) of the changed working-tree files at closeout:

```text
f2d361c6a18015ea  crates/tdb-edit/src/txn.rs
bfb14e288cac52cc  crates/studio-domain/src/session.rs
a12a1eabcd0ad0cb  crates/studio-domain/src/workspace.rs
8a441beb5f030662  crates/studio-domain/src/schedules.rs
3cd26abc1c087720  crates/studio-domain/src/schedule_edit.rs
85413267cf4df552  crates/studio-domain/src/pending.rs
93cc191ec0e40344  crates/studio-domain/src/lib.rs
170c8eaf92c5aa1b  crates/studio-domain/tests/schedule_edit.rs
efb3a36ec953e632  crates/studio-api/src/dto.rs
bfad671e84a654e3  crates/studio-api/src/convert.rs
fb65bd6cb334dec2  crates/studio-api/src/registry.rs
bd7875f09f4596c0  crates/studio-api/src/commands.rs
2bec9a88e5f8a50c  crates/studio-api/src/lib.rs
c489318bbed6319c  crates/studio-api/src/bindings.rs
17bd69aff6a98c53  crates/studio-api/tests/schedule_edit.rs
8ef6299246c2928e  app/src/lib/client.ts
f3e4ec4d9fc51b89  app/src/components/LeagueScheduleCard.tsx
31392cfdfe75605b  app/src/components/LeagueScheduleCard.test.tsx
ce94c741b2b61367  app/src/bindings/ScheduleViewDto.ts
8630ecf770060e15  docs/validation/phase-1-coverage-matrix-2026-09-18.md
f938f41b7a43318e  docs/receipts/overnight-development-2026-09-18/MORNING-REVIEW.md
```

Scratch cleanup: the five `patch_*.py` scripts and the three `cargo-test-*.txt` logs under the
session scratchpad were removed after this record was written (containment: all under
`H:\TEMP\claude\e--Repositories-nhl-modding-studio\d9e98828-…\scratchpad\`); the test-owned
temp directories were removed by the tests. Nothing retained; no successor dependency.

## 8. Next review action (superseded by §9.7)

## 9. R1 corrections (18 September 2026, [dispatch](p1a-review-and-r1-dispatch.md))

### Plain-language summary

Two things were wrong and one was unfinished. First, pressing Apply used to redo the work from
the current inputs instead of applying what you had just looked at — so if anything changed in
between (another edit, a moved team, a re-saved import file), the result could differ from the
preview; now every preview is kept by Studio under a ticket, Apply hands back the ticket, and
Studio refuses it if the project or the file moved since. Second, the schedule card could show a
preview that no longer matched the inputs, or refresh the wrong project, when answers came back
late; the card now drops every late answer that belongs to older inputs or another project/league/
table, and locks the controls while an apply runs. Third, import and generation were only
exercised on a few titles; the same checks now run on every committed target (26 title/platform
combinations, Recompiled through its own adapter) plus the real NHL 20 database.

### 9.1 F1 — Apply bound to the previewed review

* `Workspace` holds up to 16 `ScheduleReview`s (`review_id`, table, `WorkspaceRevision` at
  preview time, the `ScheduleOperation` — edit inputs / import format + text snapshot + SHA-256 +
  source path + mapping / generation config — and the plans). `schedule_edit_preview`,
  `schedule_import_preview`, `schedule_generate_preview` return `(review_id, plan)`.
* `Workspace::schedule_apply_review(review_id, source_text_now)` refuses **before any mutation**:
  an id not held by this workspace (foreign project, consumed, or evicted), a plan with errors, a
  revision that moved (any edit, undo/redo, membership change, reload, attach — the identity
  covers the primary, roster and dictionary bytes), and an import whose re-read source bytes no
  longer hash to the previewed snapshot. It writes the **stored** plan through
  `set_cells_int_rows` (atomic, one undo entry) and then drops every held review.
* API: `schedule_apply_review(project_id, review_id)` replaces `schedule_edit_apply`,
  `schedule_import_apply`, `schedule_generate_apply`; the registry re-reads a path-based import's
  file for the hash check; plans carry `review_id`; `ScheduleApplyResultDto` (`kind` edit/import/
  generate + the previewed plans + outcome + fresh view). Bindings regenerated (`--check` in
  sync; export roots 110 → 108).
* Regressions (`studio-domain/tests/schedule_edit.rs::apply_is_bound_to_the_previewed_review_and_refuses_anything_that_moved`,
  `studio-api/tests/schedule_edit.rs`): the reviewer's reproduction (previewed no-op import after a
  later edit → refused, the edit survives) on both seams; edit review after an edit on another
  table; membership move; undo/redo; tampered import bytes (refused, then the untouched snapshot
  applies exactly as previewed); two generation configurations → the first review applies its
  own configuration and the second is dropped; two import mappings → each review holds its own;
  foreign project id; consumed id; plan with errors. Each refusal asserts bytes, undo/redo depth,
  pending cell count and the revision unchanged.

### 9.2 F2 — UI request ownership

`LeagueScheduleCard.tsx`: a context key (`project/league/table`) in a ref, a per-panel input
generation, and one `useReview` lifecycle for the three panels — a preview response is kept only
if its generation and context are still current; any input change clears the shown plan at once;
Apply sends only the held `review_id`, locks the inputs, ignores a second click, and reports back
to the card even if the panel is gone (the card refreshes queries, updates the project summary
only if that project is still open when the summary returns, and shows the notice only in the
same context); the file dialog's result is dropped when the context moved; the picked table is
scoped to the project/league it was picked in. Panels are keyed by context so a switch never
shows another context's state. Tests (`LeagueScheduleCard.test.tsx`, 9 deferred-promise cases):
preview resolving after an input change; out-of-order previews; mapping change while a preview is
pending; project switch during preview/apply/summary refresh (no foreign `update`, no notice);
league switch resets the picked table and drops a pending preview; file dialog returning after a
switch; double Apply; generation input change invalidates the plan — each asserting the displayed
state and the exact dispatched arguments (`schedule_apply_review` with `{projectId, reviewId}`).

### 9.3 F3 — per-target offline parity

`import_and_generation_parity_on_every_committed_target` runs the same import (export CSV → swap
two clubs through an explicit mapping → apply → read back → undo to the baseline) and generation
(apply the first source table that is a whole number of round-robins and check pairs/venues/
dates/index/games-per-participant from the bytes; else every source table must name the
arithmetic blocker) on **26 committed targets**: Xbox 360 07/08/09/10/11/12/13/14/15/Legacy,
Legacy through `Platform::Recomp`, PS3 08/09/10/11/12/13/14/15/Legacy, PS4 15/16/17/18/19/21 —
and the private real NHL 20 database in the `--ignored` case. NHL 19's source is the committed
`anchors/ps4/nhl19` (the NHL 19 base, correctly labelled); NHL 20 is never read from
`anchors/ps4/nhl20`. Result per target (developer run, `--nocapture`):

| Target | Import swap | Generation |
|---|---|---|
| NHL 07 X360 | `nhlschedulelongnoleap` | `intldelschedule` 14 × 4 = 364 (blocked: 4 other tables) |
| NHL 08, 09 X360 / PS3 | `nhlschedule` | **blocked on every source table** (NHL 1 230 / 435, AHL 1 160 / 406 — no whole round-robin) |
| NHL 10 X360 / PS3 | `nhlschedule` | `czechschedule` 14 × 4 = 364 (blocked: 4) |
| NHL 11–15, Legacy X360 / PS3; Legacy Recomp | `nhlschedule` | `czechschedule` 14 × 4 = 364 (blocked: 6–7) |
| NHL 15–19, 21 PS4 | `nhlschedule` | `czechschedule` 14 × 4 = 364 (blocked: 6–11) |
| NHL 20 PS4 (private install DB) | `nhlschedule` + Austrian no-op | `czechschedule` 14 × 4 = 364; Austrian blocked (270 / 11) |

Shared bytes (Recompiled = Xbox Legacy files) are opened through the Recomp adapter, not
substituted. Unsupported topologies remain explicit arithmetic blockers; the unbound tournament
tables remain research questions (P1-B).

### 9.4 Checks (developer; see §9.6 for the closeout totals)

| Check | Result |
|---|---|
| `cargo test -p studio-domain --test schedule_edit` | 11 passed, 1 ignored |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-domain --test schedule_edit -- --include-ignored` | see §9.6 |
| `cargo test -p studio-api --test schedule_edit` | 1 passed |
| `cargo test -p studio-api --lib bindings` | 25 passed (root count 108; `--check` in sync) |
| `npx tsc --noEmit`; `npx vitest run` | clean; 11 files, 88 passed (9 card cases) |

### 9.5 Changed paths (R1)

`crates/studio-domain/src/workspace.rs` (review model), `crates/studio-domain/src/lib.rs`
(exports), `crates/studio-domain/tests/schedule_edit.rs` (helpers, F1 regression, parity case),
`crates/studio-api/src/{dto.rs,convert.rs,registry.rs,commands.rs,lib.rs,bindings.rs}`,
`crates/studio-api/tests/schedule_edit.rs` (rewritten), `app/src/bindings/` (`ScheduleApplyResultDto.ts`
added; `Schedule{Change,Import,Generation}ResultDto.ts` removed; `ScheduleChangePlanDto.ts`),
`app/src/lib/client.ts`, `app/src/components/LeagueScheduleCard.tsx` (rewritten),
`app/src/components/LeagueScheduleCard.test.tsx` (rewritten), this record, the coverage matrix.

### 9.6 Closeout figures and cleanup

Full suites after the last R1 edit (developer results, `--no-fail-fast`):

| Suite | Result |
|---|---|
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test schedule_edit -- --include-ignored` | **12 passed** (26-target parity + real NHL 20: `czechschedule` 14 × 4 applied, Austrian blocked) |
| `cargo test -p studio-api` | 17 binaries, 90 passed, 0 failed |
| `cargo test -p studio-domain` | 45 binaries, 335 passed, 0 failed, 8 ignored (explicit private-fixture skips) |
| `npx tsc --noEmit`; `npx vitest run` | clean; 11 files, 88 passed |

SHA-256 (first 16 hex) of the R1-changed working-tree files:

```text
f6f7b1b8b43cfb3c  crates/studio-domain/src/workspace.rs
14c688a2e0c42def  crates/studio-domain/src/lib.rs
e109b7729ef8ffda  crates/studio-domain/tests/schedule_edit.rs
27ed5e080b0bb6f8  crates/studio-api/src/dto.rs
7e69608157b4a63b  crates/studio-api/src/convert.rs
8f84e65a217c7739  crates/studio-api/src/registry.rs
3748b8e82198f09c  crates/studio-api/src/commands.rs
8015277009946a67  crates/studio-api/src/lib.rs
0a2a979081f54611  crates/studio-api/src/bindings.rs
9872596c4b60943e  crates/studio-api/tests/schedule_edit.rs
47081398292f5332  app/src/bindings/ScheduleApplyResultDto.ts
12b8379c0388e9f1  app/src/bindings/ScheduleChangePlanDto.ts
0edd0d0a0a2432c0  app/src/lib/client.ts
d369911db139781a  app/src/components/LeagueScheduleCard.tsx
b689111c71bb85ef  app/src/components/LeagueScheduleCard.test.tsx
8b04711178b9954c  docs/validation/phase-1-coverage-matrix-2026-09-18.md
```

Removed bindings: `ScheduleChangeResultDto.ts`, `ScheduleImportResultDto.ts`, `ScheduleGenerationResultDto.ts`
(generator-owned, removed by `--write`). Scratch: the R1 patch scripts and test logs under the
session scratchpad were removed after this section was written (containment: all under
`H:\TEMP\claude\e--Repositories-nhl-modding-studio\d9e98828-…\scratchpad\`); test temp
directories removed by the tests; the shared `target` cache reused; nothing retained, no successor
dependency; no process left running.

### 9.7 Next review action (superseded by §10.5)

## 10. R2 — ticket ownership (18 September 2026, [dispatch](p1a-r1-review-and-r2-dispatch.md))

### Plain-language summary

A preview ticket from one open project could be mistaken for a ticket of another, because
tickets were just counters that every project started at 1 — so handing project B a ticket from
project A would apply B's own plan under that number. Tickets are now opaque strings stamped
with the identity of the project instance that issued them; any other instance — including the
same file closed and reopened — refuses them outright, before looking anything up. Everything
from R1 (exact stored plans, revision and file checks, single use, bounded retention, nothing
written on refusal) is unchanged.

### 10.1 Change

* `Workspace` gains `review_namespace` — `<construction-time nanos hex>-<process-wide sequence
  hex>`, minted once per constructed instance (both constructors) — and tickets are
  `"<namespace>/<sequence>"` strings. `owns_schedule_ticket` checks the namespace prefix;
  `schedule_review` and `schedule_apply_review` refuse a ticket from another instance with
  "issued by another project (or by an earlier instance of this one)" before any lookup or
  mutation; a ticket from this instance that is no longer held (applied, evicted, dropped by a
  change) is refused as "no longer held". The R1 protections (stored plan, `WorkspaceRevision`,
  import byte hash, single use, 16-review retention, unchanged-on-refusal) are untouched.
* Representation is lossless end to end: `ChangePlan`/`ScheduleReview.ticket: String`,
  `ScheduleChangePlanDto.ticket` / `ScheduleApplyResultDto.ticket` (`string` in the generated
  bindings), `schedule_apply_review(project_id, ticket: String)`, the client sends the string as
  received; no `Number` conversion anywhere (`review_id: u64` is gone from the schedule surface).
* Bindings regenerated (`ScheduleChangePlanDto.ts`, `ScheduleApplyResultDto.ts`); `--check` in
  sync; root count unchanged (108).

### 10.2 Regressions

* Domain — `tickets_never_alias_across_workspace_instances`: two Legacy workspaces mint tickets
  concurrently with the **same local sequence** (the reviewer's reproduction: A day = 1, B day =
  2); each other's unmodified ticket is refused in both directions with bytes, revision,
  undo/redo depth and pending cell count unchanged and both tickets still held; each then applies
  exactly its own plan once (single use asserted). Repeated for an import ticket (A) against a
  generation ticket (B) on different tables, and for a **replacement instance** opened on the same
  file (old ticket refused by the new instance even though it minted a same-sequence ticket; the
  old instance's ticket still applies at home). The F1 case keeps a plain unknown-ticket refusal.
* API — `studio-api/tests/schedule_edit.rs`: two projects (Legacy X360, Legacy PS3) mint
  concurrent tickets; each refuses the other's unmodified ticket ("issued by another project"),
  both stay clean, each applies its own once (second apply refused); a project closed and
  reopened refuses the closed instance's ticket and applies its own fresh one.
* UI — the 9 card cases now carry string tickets (`"ws-a/<n>"`) and assert
  `schedule_apply_review` is dispatched with `{ projectId, ticket }` verbatim.

### 10.3 Checks (developer)

| Check | Result |
|---|---|
| `cargo test -p studio-domain --test schedule_edit` | 12 passed, 1 ignored |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-domain --test schedule_edit -- --include-ignored` | see §10.4 |
| `cargo test -p studio-api --test schedule_edit` | 1 passed |
| `export-bindings --check` | in sync |
| `npx tsc --noEmit`; `npx vitest run` | clean; 11 files, 88 passed |

### 10.4 Closeout figures, incident and cleanup

Full suites after the last R2 edit (developer results, `--no-fail-fast`):

| Suite | Result |
|---|---|
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test schedule_edit -- --include-ignored` | **13 passed** (incl. the collision case, the 26-target parity case and real NHL 20) |
| `cargo test -p studio-api` | 17 binaries, 90 passed, 0 failed |
| `cargo test -p studio-domain` | 45 binaries, 336 passed, 0 failed, 8 ignored (explicit private-fixture skips) |
| `npx tsc --noEmit`; `npx vitest run` | clean; 11 files, 88 passed |

SHA-256 (first 16 hex) of the R2-changed working-tree files:

```text
4d82b3122b72d01c  crates/studio-domain/src/workspace.rs
5ee9b00b33c4c6be  crates/studio-domain/tests/schedule_edit.rs
b97e600ce8097b0d  crates/studio-api/src/dto.rs
de2dd9004346e97a  crates/studio-api/src/convert.rs
b707c0dc6ab30c42  crates/studio-api/src/registry.rs
c89b26990a9a3522  crates/studio-api/src/commands.rs
8a7c45103f36f76a  crates/studio-api/tests/schedule_edit.rs
622e9740f314b55c  app/src/bindings/ScheduleApplyResultDto.ts
d3db5c9feb707a62  app/src/bindings/ScheduleChangePlanDto.ts
89625fdadcda65ca  app/src/lib/client.ts
a1539da06628a2a5  app/src/components/LeagueScheduleCard.tsx
88cbd2f31cd06d58  app/src/components/LeagueScheduleCard.test.tsx
```

**Incident (recorded for the reviewer).** While applying the R2 patch, a scratch script opened
`crates/studio-domain/src/workspace.rs` for writing before its transform had succeeded, and an
assertion in the transform left the file **empty**. The file is untracked-modified (the R1 state
was never committed), so it was recovered from a dangling git blob
(`337704f5b5bb79873d57ae013be2bdad724111fc`, 2 169 lines) whose content hashed to exactly the R1
closeout identity `f6f7b1b8b43cfb3c` recorded in §9.6, then re-patched (write-after-transform).
No other file was affected; the current identity is in the table above. The scratch scripts were
corrected to transform first and write only on success.

Scratch cleanup: the R2 patch scripts, test logs, hash list and the `workspace.rs.r1-backup` copy
under the session scratchpad were removed after this section was written (containment: all under
`H:\TEMP\claude\e--Repositories-nhl-modding-studio\d9e98828-…\scratchpad\`); test temp
directories removed by the tests; shared `target` cache reused; nothing retained; no process left
running.

### 10.5 Next review action

Codex review of §10 and the R2 paths; no further package started; P1-B stays with the research
session.
