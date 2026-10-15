# LEAGUE-R2 — correction receipt (2026-09-17)

**Plain-language summary.** The safety check that stops a team from being pointed at a league
that does not exist had a loophole: it only looked at *how many* bad references there were, so
swapping one bad league code for a different bad one (or fixing an old bad reference while
creating a new one somewhere else) slipped through because the total did not grow. The check now
compares the actual bad references — which team, which value — before and after every edit, so
any newly introduced bad reference is refused even when the count stays the same or drops.
Databases that already contain a bad reference still open and browse normally, other edits to
those teams still work, fixing the bad value still works, and the check still does not exist on
the older consoles that have no league table. The reviewer's NHL 18 reproduction is now refused.
Also corrected: the manual desktop launch command written in the previous receipt did not exist;
the real one is documented below. No desktop or in-game run was performed.

---

## 1. Package, disposition, scope

* **Package:** LEAGUE-R2 per [league-r1-review-and-r2-dispatch](../validation/league-r1-review-and-r2-dispatch.md)
  (one blocking P1 in the referential gate). Confined to reference validation and its regressions;
  the accepted F1 (archive snapshot) and F3 (frontend lifecycle) changes are untouched. No new
  features or phases; no live install; no staging/commit/push. Dirty tree preserved.
* **Disposition:** the finding is fixed with the requested regressions; developer-verified;
  awaiting Codex review. Phase 1.1 remains open.

## 2. The finding and its disposition

### Equal-count dangling substitutions bypassed the gate (fixed, reproduced with the reviewer's harness)

The comparison is now a **multiset of violation identities**, never a count. It lives in a new
pure function `studio_domain::referential::new_dangling_references(before, after, relations,
meta)`; `EditSession::run` calls it after every committed operation (every seam: `set_field`,
`set_cells_int`, structural ops, imports) and refuses the op with the same human message when it
returns anything.

* **Identity = `(row key, value)`.** The row key is the source table's `key="True"` field values
  from the title's metadata (`kOtt.vfEq` on PS4). Keys are never editable through the scalar seam
  and structural operations mint fresh ids, so an untouched violation keeps its identity when a
  row-removal *moves* its record index (ordered shift or swap-pop). Tables whose metadata marks no
  key fall back to the record index. Identities are counted, so duplicates cannot mask each other.
* **Rule:** any identity whose post-count exceeds its pre-count is a new violation. Substituting
  invalid 31 → invalid 30 on the same row (equal count) is refused; repairing team A while giving
  team B the same absent code (equal count) is refused; repairing two old violations while adding
  one new one (lower count) is refused. Unchanged violations, unrelated edits on a violating row,
  pure repairs and legitimate record movement are accepted. Titles without `fsLK` still have no
  rule (the audit skips the edge).
* **Row movement inspected, not weakened:** with a record-index identity a removal would have
  turned every later pre-existing violation into a "new" one; the key-based identity is why the
  removal cases below pass without any allowance for structural ops. Genuinely new violations
  after a removal are still caught.

Reviewer harness rerun (unchanged source, SHA `d87e71a8…`):

```text
SET TEAM 0 LEAGUE TO ABSENT 31: Err(Domain(Validation("Team Anaheim Ducks®: league = 31 refers to no stockleagues row on this title (team-league relationship); the edit was not applied")))
NHL18 LEAGUE 30 EXISTS: false
CHANGE INVALID 31 TO INVALID 30: Err(Domain(Validation("Team Anaheim Ducks®: league = 30 refers to no stockleagues row on this title (team-league relationship); the edit was not applied")))
TEAM 0 AFTER: Some(31)
```

### Regressions added

`crates/studio-domain/tests/referential.rs` (new, NHL 18 anchor; asserts codes 30/31 have no
`stockleagues` row):

| Case | What it asserts |
|---|---|
| `equal_count_substitution_is_a_new_violation` | 31→30 and 30→31 on one row each report exactly that new identity; an unrelated cell change on the violating row reports nothing |
| `repair_plus_introduction_at_equal_and_lower_counts_is_refused` | repair A + introduce B (equal) → B only; repair two + introduce one (lower) → the one; pure repairs → nothing; swapping two invalid values between rows → both identities |
| `unchanged_violations_survive_record_movement` | `remove_record_ordered` (violation moves 5→4) and swap-pop `remove_record` (tail violation moves to 0) report nothing; a new violation after the move is still reported |
| `titles_without_the_league_table_have_no_rule` | Legacy (no `fsLK`) reports no team-league violation |
| `nhl18_invalid_to_invalid_substitution_refused_on_every_edit_seam` | `set_field` and `set_cells_int` refuse 31→30; working bytes, undo (0, nothing to undo/redo), pending (not dirty) and the destination file unchanged; unrelated edit and valid repair accepted; re-introduction refused |

`crates/studio-api/tests/league_copied_target.rs` — `nhl18_equal_count_invalid_substitution_is_refused`:
the reviewer's case through `Registry::set_field` on a copied NHL 18 fixture inside a marked
copied-target root: refused; summary not dirty, undo/redo 0, `workspace_revision` unchanged,
pending cells 0, team still shows 31, destination SHA-256 unchanged; unrelated edit and repair
accepted; 30 and 31 both refused afterwards; destination still unchanged.

Multi-row "repair + introduce" states are exercised through the validator function on hand-built
before/after bytes, as the dispatch allows; no production multi-row API was added.

### Manual smoke launch command (corrected, not executed)

`app/package.json` has no `tauri` script (`dev`, `build`, `preview`, `test` only). The repository
root `package.json` has `"dev": "tauri dev"` with `@tauri-apps/cli` installed (`npx tauri
--version` → `tauri-cli 2.11.3`), and `src-tauri/tauri.conf.json` sets `beforeDevCommand` to
`npm --prefix app run dev` and `devUrl` to `http://localhost:1420`. The executable command is
therefore **`npm run dev` from the repository root** (equivalently `npx tauri dev` there). The
R1 receipt's smoke paragraph now points here. The remaining smoke steps are unchanged and remain
an unexecuted gate.

## 3. Changed paths (SHA-256 prefix)

```text
9c89e7dcd1a295e1  crates/studio-domain/src/referential.rs           (new: identity-based delta gate)
ae4288de52710289  crates/studio-domain/src/session.rs               (run() delegates; count shortcut removed)
54d7e219cec5e713  crates/studio-domain/src/lib.rs                   (pub mod referential)
5f6ad8aa824aafdd  crates/studio-domain/tests/referential.rs         (new, 5 cases)
dea247b0d1e1ccb3  crates/studio-api/tests/league_copied_target.rs   (+1 regression)
docs/receipts/league-r1-correction.md                                (smoke command struck and redirected)
```

No frontend, binding, DTO, relationship def, archive or copied-target source changed.

## 4. Fresh checks (developer, this host, `--target-dir local/runs/league-overnight-2026-09-17/target`)

| Evidence class | Command | Result |
|---|---|---|
| Domain / validation (full) | `cargo test -p tdb-relations -p tdb-domain-validation -p studio-domain` | every suite ok, 0 failures (incl. `referential` 5, `leagues` 7, `session` 11, `pending` 3, `copied_target` 3; `tdb-relations` 4+4+3; `tdb-domain-validation` 2+14) |
| Focused rerun after the last edit | `studio-domain --test referential --test leagues --test session --test pending` | 5 / 7 / 11 / 3 ok |
| API (focused) | `cargo test -p studio-api --test copied_target --test leagues --test league_offline_campaign --test league_copied_target` | 15 / 3 / 1 (28 fixture rows) / 7 ok |
| Reviewer harness | `cargo run --offline --manifest-path local/reviews/league-overnight-probe/Cargo.toml …` | both edits refused (see §2) |
| Structural | `cargo clippy -p studio-domain -p studio-api --tests`; `export-bindings -- --check`; `cargo check -p nhl-studio`; `rustfmt --check` on the touched Rust files | no warnings; "Bindings are in sync"; shell compiles; clean |
| Component (untouched, rerun) | `cd app && npx vitest run` | 5 files, 39 passed |
| Desktop | — | **not executed**; launch command verified from config only (§2) |
| In-game | — | not executed, not authorized |

## 5. Limits

Automated evidence on committed anchors and a copied NHL 18 fixture only. The gate covers the
confirmed relationship defs; `leaguegroup`/`conferencegroup`/`divisiongroup` ids remain ungated
(no evidenced target table). A table whose metadata marks no key field falls back to record-index
identity, where a row removal ahead of a pre-existing violation would be reported as new — every
current confirmed binding's source table (`kOtt`, `ZBac`, `ubPc`, `eGlu`, `fMVl`) carries a key on
the PS4 metadata checked. PS4 archive output stays structurally unqualified; runtime acceptance
untested.

## 6. Scratch and ownership

* Retained (review dependencies, untouched): `local/runs/league-overnight-2026-09-17/target/`,
  `local/reviews/league-overnight-probe/`, `local/reviews/league-invalid-reference-52016/`,
  `local/reviews/league-invalid-reference-74364/`.
* Removed by this correction (literal path): `local/reviews/league-invalid-reference-87084/`, the
  fixture my harness rerun created (NHL 21 no-op transaction + copied NHL 18 db).
* Test temp dirs (`nhlms-league-ct-*`, `nhlms-league-r2-*`) removed by the tests themselves;
  `%TEMP%\nhl-modding-studio\copied-target-staging` empty; `%TEMP%\nhlms-p3-op-24084` and
  `%TEMP%\nhlms_ps3_dds_portrait` unowned, untouched.
* Anchors untouched (`git status anchors` empty); index empty; no processes left running.

## 7. Next review action

Codex: review `referential.rs` and the identity rule; rerun
`cargo test -p studio-domain --test referential` and
`cargo test -p studio-api --test league_copied_target`; rerun the harness. Desktop smoke
(`npm run dev` at the root) on disposable copied fixtures if tooling permits. Stopping here.
