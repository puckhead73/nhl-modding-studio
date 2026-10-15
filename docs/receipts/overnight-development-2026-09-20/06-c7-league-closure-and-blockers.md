# Receipt 06 — C7: remaining League closure — whole-group scope, PS4 locales, NHL 15 patch DB; Priority-5 blocker record

Developer lane, overnight 2026-09-20. **Developer-verified; not Codex-accepted.** Baseline `8d60a6d`.

> **Correction (OVERNIGHT-DEV-20260920-R1, [record](../../validation/overnight-dev-20260920-r1-corrections.md)):**
> §4's whole-group scope accepted contradictory plans on one block (R1 F1 — fixed, refused at
> engine, domain and API); §6's blocker table is stale in three rows — the NHL 07 row overstates
> the label source (the executable holds the `CHTM_LEAGUE_*` **keys**, not label text; no text
> source was located — research 2026-09-19 handoff 05 §1, 2026-09-20 handoff 04 §4), the audio
> row's "bank index undecoded" is superseded by research 2026-09-20 handoff 02 (index decoded;
> encoders and the gen7 event link remain the blockers), and the schedule-generation row's "stock
> format not established" is superseded by handoff 06 (per-era quotas measured; generation
> contract proposed, not accepted). See the record's §4 for the reconciled table.

## Plain-language summary

Three League items that still had a contract were closed: (1) on the PS4 titles where one logo block
serves several paths (a logo, its French twin, a scouting-region logo), the archive dialog now asks
**which scope** you mean — this member only (a private copy, the default) or the whole group (every
path shows the new image, rewritten in place when it fits, otherwise appended once with every path
re-pointed) — and never widens the group on its own; (2) the 99 non-English and patch-archive
dictionaries of the seven PS4 titles (seven locales each; NHL 21 adds the template locale; NHL 15's
and NHL 19's patch archives carry copies) were extracted from the installs and every dictionary
operation Studio has, including the new rebuild, runs on each of them; (3) NHL 15's patch database,
the copy the console is believed to read first, is now tested for the competition-group editor.
Everything else left in Phase 1 is research-blocked or a runtime question; the exact blockers are
listed below so nothing is quietly called complete.

## 1. Package, disposition, scope

C7 — delivered, developer-verified. Priority 1 leftovers (whole-group scope; PS4 locales) and
Priority 5 closure (NHL 15 patch DB) plus the blocker record. Not built: anything in §6.

## 2. Changed paths (SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-archive-big/src/stage.rs` / `stage/tests.rs` | `a515eca486d5605a` / `451d6f6d09c7b1e5` | `SharedSpanRule::WholeGroup`, `plan_scoped(.., whole_group: &[usize])`, `PlannedEdit.sibling_row_edits` (applied by `patched_prefix`, checked by `verify` incl. the siblings' re-pointed rows), `StageError::SharedGroupIrregular`; +1 unit case (fit in place, append once, irregular refusal, index list vs plan-wide rule) |
| `crates/studio-domain/src/asset_member_edit.rs` | `fcfbe451c800d312` | `whole_group` on the plan/planner (irregular-group refusal `group-irregular`; whole-group placement predictions) |
| `crates/studio-domain/src/copied_target.rs` | `7a15bdae2aeb5a7c` | `TextMemberReplacements.whole_group` (member paths) |
| `crates/studio-api/src/registry/copied_target.rs` | `0750aace248a39c3` | `stage_asset_members` plans with `plan_scoped` (private copy for everything shared unless the member chose the group) |
| `crates/studio-api/src/{registry,commands,dto,convert}.rs` | `6e54b31d…` / `1c46bd2b…` / `8c3b18cc…` / `06e2b7b8…` | `asset_member_preview_scoped`; command `wholeGroup`; `AssetMemberPlanDto.scope` |
| `app/src/bindings/AssetMemberPlanDto.ts` | generated | `--write`; `--check` in sync |
| `app/src/components/ArchiveMemberReplace.tsx` (+test), `LeagueImageryCard.test.tsx`, `lib/client.ts` | `087c3e60…` / `182a7ec0…` / `f23fcfc1…` / `84673bad…` | scope radio for shared members (re-preview under a new generation; exact `wholeGroup` argument asserted); +1 jsdom case |
| `crates/studio-api/tests/asset_member_route.rs` | `0805f57095900502` | +1 route case: whole group in place (both rows keep the offset, take the new size, nothing appended) and appended once (both re-pointed, old block left), restore byte-exact; default scope unchanged |
| `local/runs/overnight-development-2026-09-20/extract_ps4_locale_fixtures.py` | `518f0c5c031eb652` | **new** — developer extractor (read-only on the installs; imports the 2026-09-18 research EB reader) |
| `E:\nhl-studio-fixtures\loc-ps4\**` + `manifest.json` (`0254a5a0d51dc7fa`) | — | **new private fixtures** (250 MB): 99 locale members of 14 archives |
| `crates/tdb-loc/tests/locale_coverage.rs` | `a04807c3739afd9c` | +1 `--ignored`: the 99 PS4 locale members through re-point, re-text, growth, no-op rebuild (identity minus slack) and new-glyph rebuild |
| `crates/studio-domain/tests/ps4_groups.rs` | `d670703c59555f2f` | +1 `--ignored`: NHL 15 patch DB binds by structural signature, fits with no conflict, takes and undoes membership moves |
| `crates/studio-api/src/bin/competition_groups.rs`, `defs/leagues/groups/ps4-nhl15.toml` | `b517550f8a4320b3` / `5bfd690095c0b57a` | the NHL 15 witness note updated (generator + rendered table; `--check` in sync) |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | (updated) | cells for C1–C7; campaign banner |
| `docs/receipts/overnight-development-2026-09-20/MORNING-REVIEW.md` | new | cards by title |

## 3. Measured facts (developer evidence)

- **PS4 locale members:** every title ships seven locales (`cze_cz`, `eng_us`, `fin_fi`, `fre_fr`,
  `ger_de`, `rus_ru`, `swe_se`) in `inxp_00.big`; NHL 21 adds `lng_lg`; NHL 15's `patch_inxp00.big`
  and NHL 19's `patch_inxp_51 … 56.big` carry a full set each (research 20 §6 said "NHL 15 and 19 also
  carry copies in `patch_inxp*.big`" — measured as complete locale sets; research 02/01's "NHL 19's
  twelve patch archives carry none" was about the CSV catalogue). All 99 are class A; the no-op rebuild
  is the input minus its trailing slack on every one.
- **Whole-group blocks on the synthetic archive** behave as research 02 §4.1 describes; on the real
  PS4 archives the groups measured by research (dedup rows sharing exact spans) are the ones the
  `group-irregular` check accepts — a partial overlap (different offset/size) refuses.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `cargo test -p tdb-archive-big --lib stage::tests` | 30 passed (+1) |
| `cargo test -p studio-api --test asset_member_route` | 4 passed (+1), 2 ignored |
| `NHL_STUDIO_FIXTURES=… cargo test -p tdb-loc --test locale_coverage every_ps4_locale -- --ignored` | **1 passed** (1 945 s; 99 files; locales as listed) |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-domain --test ps4_groups real_nhl15_patch -- --ignored` | 1 passed |
| `cargo test -p studio-domain --test ps4_groups` / `cargo run -p studio-api --bin competition-groups -- --check --fixtures …` | 10 passed / in sync (12 rendered) |
| `cargo run -p studio-api --bin export-bindings -- --write` then `--check` | 1 written / in sync (117 roots) |
| `cd app && npx tsc --noEmit -p . && npx vitest run` | clean / **17 files, 128 passed** |
| aggregate (this checkpoint): `studio-domain` copied_target 6, in_place_save 7, leagues 11, membership 5, pending 3, schedule_edit 12 (+1 ign), settings_csv_edit 6 (+1), settings_edit 8 (+1), txn_coordination 3, workspace 10; `studio-api` copied_target 15, copied_target_contention 5, dictionary_rename 5, iso_route 5 (+1), league_copied_target 7, leagues 3, membership 5, schedule_edit 1, settings_csv_route 2 | all passed |

## 5. Coverage after C7

- 1.4 archive placement, PS4 16–21: **whole-group scope impl+tested** (synthetic; the real-archive
  sweep exercises the private-copy scope).
- 1.3 locales, PS4: **impl+tested** on the 99 members (was research unknown / not extracted).
- 1.2 competition groups, PS4 15: the patch database **impl+tested** (was "untested").

## 6. Priority-5 blocker record (League database / rules / audio) — nothing quietly complete

| Item | State | Exact blocker / next evidence |
|---|---|---|
| NHL 07 league identity (1.1/1.3) | blocked | labels exist only as literals in `default.xex` (research 05 §1, bounded search); a rename needs an executable literal edit (out of scope) or a class-B key insertion (refused by contract); card R5-07-1 first |
| League/competition row creation or resizing (1.2) | not impl | engine tolerance of a grown league table is research unknown; no schema/dependency contract for the created row's consumers |
| Unnamed/inferred competition groups (PS4 ids 77/78, NHL 17 mechanism) | research unknown | the eboot table / id mechanism; shown by number with the hypothesis, never as the name |
| Tournament bindings (`championsschedule`, `spenglerschedule`, `wcschedule`, `worldchamp`/`worldjunior`) | research unknown | slot-count vs stem-league mismatch not resolved (P1-B) — browse only |
| Rules / season / playoff settings on gen7 & PS3 | blocked | compiled tables in the executables (research 21/22); no data-file copy |
| Stored defaults on NHL 16–21 / NHL 15 | impl+tested (D3/D4) | runtime consumption/precedence: cards P1BS-1 / P1BS-CSV-1 |
| Schedule generation beyond the balanced round robin (NHL / Swiss / AHL) | blocked | the stock division-weighted format is not established; inferring it from stock seasons is forbidden |
| Mode-related data editing / new-mode survival of an edited schedule | research unknown | no save-side evidence (P1-B) |
| Audio: bank reader / catalogue / preview / reassignment (1.5) | blocked (format) | SND10 bank index and per-platform sample codecs undecoded (research 04 / 19 §2; Q-R4-2); no contract sufficient for even a read-only catalogue of samples — nothing speculative built |

## 7. Temporary artifacts / processes

- Scratchpad `c7_*.py` edit scripts — disposable. No temp dirs left (`nhlms-d8-api-*` removed by `Drop`).
- Retained: `loc-ps4` fixtures (private root; input of the `--ignored` sweep; regenerable by the extractor).

## 8. Runnable commands

```sh
cargo test -p tdb-archive-big --lib stage::tests
cargo test -p studio-api --test asset_member_route
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p tdb-loc --test locale_coverage every_ps4_locale -- --ignored --nocapture
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test ps4_groups real_nhl15_patch -- --ignored
cd app && npx vitest run src/components/ArchiveMemberReplace.test.tsx
```
