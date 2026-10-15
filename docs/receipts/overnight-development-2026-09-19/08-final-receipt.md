# Final receipt — developer lane, overnight 2026-09-19

> **Correction note (OVERNIGHT-DEV-R1, 19 Sep).** Left as written below. The first independent review
> (write routes) found two regressions in the D8 route — UI ownership (F1) and undo history (F2) — both
> corrected; and narrowed §1's coverage wording (F3): bullet 4 is a codec round trip on a per-title fixture
> set, not a complete UI flow on every installed build; bullet 5's imagery is listed, not round-tripped on
> real files; bullet 6 is proven on one real NHL 21 member inside a synthetic archive. §2's "`tsc --noEmit`
> clean" did not hold when rerun under R1: `LeagueImageryCard.test.tsx` (the D8 jsdom case) had a type
> error that vitest does not check; fixed under R1. The campaign remains under review; the areas the
> first review did **not** cover are listed in the R1 record §5. Record:
> [overnight-dev-r1-corrections-2026-09-19.md](../../validation/overnight-dev-r1-corrections-2026-09-19.md).

**Developer-verified; not Codex-accepted; nothing game-verified.** No commits, staging, pushes, resets or
clean operations; no writes to installs, disc images, anchors or profiles; private fixtures were read-only
inputs; NHL21-E1 untouched. Baseline `8d60a6d`, branch `feat/phase5-asset-tooling`, the pre-existing dirty
tree preserved. Per-package detail: receipts [01](01-p1bs-1-scoped-settings-decoding.md) →
[07](07-p1f-ps4-archive-member-route-and-p1-accounting.md); cards: [MORNING-REVIEW.md](MORNING-REVIEW.md);
lane state: [STATUS.md](STATUS.md).

## 1. What users can now do (offline, in Studio)

- **Read every mode-setup setting** of NHL 15–21 PS4 as a catalogue with lineage names, contexts, choice
  labels and per-context stored defaults; NHL 15's four offset-binary sites decode correctly; every other
  target shows the exact reason it has no data-side catalogue. (D1–D2)
- **Edit one stored default** on NHL 15 PS4 (one cell of the open database, ticketed preview → apply,
  byte-accounted, CRC recomputed) and on NHL 16–21 PS4 (one or two cells of the `setting_csv` members,
  re-compressed with a byte-faithful native chunkzip encoder and placed into `databigs/inxp_00.big` by the
  copied-target save, fit-in-slot or append-and-retarget, restore byte-exact). (D3–D4)
- **Grow a localization dictionary**: longer text, a new key at its sorted position (class-A files), null
  texts — on every title, platform and locale Studio has a file for (25 anchors + 100 loose locale files +
  6 install dictionaries), through the pending review, in-place and copied-target saves. (D5)
- **Preview and replace every league logo package** on every chain — Xbox 07–Legacy (RW4xb2 / XPR2), PS3
  08–Legacy (RW4ps3 / DDS), PS4 15–21 (PNG / GNF) — at the same dimensions; two Xbox decoder faults fixed.
  (D6)
- **List and replace the league-keyed presentation imagery** (playoff-tree logos, trophies, MUD logos; PS4
  large logos, Memorial Cup, Champions tile, region aliases) per title. (D6 / P1-G)
- **Rewrite a PS4 logo or imagery member inside its packed archive** through the copied-target save, with a
  private copy for deduplicated members and the siblings named. (D8 / P1-F)

## 2. Tests (developer, focused; all green unless stated)

| Area | Suite(s) | Result |
|---|---|---|
| Settings decoding | `tdb-defs` settings_sites | 10 (+1 `--ignored`, passed on the patch DB) |
| Catalogue / editor / CSV | studio-domain settings_catalogue 13, settings_edit 9, settings_csv_edit 6 (+1 real NHL 16 archive), studio-api settings_csv_route 2 | passed |
| Chunkzip native | tdb-archive-codec chunkzip_native 6 (53/53 CSV + 2/2 stored-chunk members byte-identical) | passed |
| Localization | tdb-loc growth 9, locale_coverage 1 (+2 `--ignored`: 100 locale files, 6 install files), studio-api dictionary_growth 3, dictionary_rename 5 | passed |
| League assets | tdb-rx2 112 (+3 texlib_ps3), studio-domain league_logo_packages 2 (+2 `--ignored`: 77 packages PS3 = Xbox exact, 34 PS4 members), league_imagery 3, league_logos 3 | passed |
| Archive-member route | tdb-archive-big stage 29 (+ private copy), studio-api asset_member_route 2 (+1 `--ignored` real NHL 21 GNF) | passed |
| Phase 1 gaps (D7) | studio-domain membership local-bits sweep, league_logos NHL 07 | passed |
| Bindings | `export-bindings --check` in sync; roots 116 | passed |
| Frontend | `tsc --noEmit` clean; vitest 15 files / 112 tests | passed |
| Broader affected suites | studio-domain `--lib` 176; tdb-archive-big 57; studio-api `--lib` 31; the D8 batch (see STATUS closing note) | passed |

Not run: the full workspace suite; `zz_diag` (excluded deliberately); clippy/fmt (recorded baseline
defects); the desktop; any game.

## 3. Unresolved (named, with the next action — receipt 07 §5 has the per-cell table)

- Canonical dictionary tree rebuild (O-G) — not built; refused with the missing symbols.
- Different-dimension logo replace — not built (Apt record floats + container resize; runtime unknown).
- gen7 placement of a changed-length logo package into BIG4 / `cache.big` / `.bh` — not built (inner BIGF
  span-preserving re-plan + BIG4 writer).
- Runtime gates: settings consumption (P1BS-1 / P1BS-CSV-1), dictionary growth (R-A / R-B), loose-vs-archive
  precedence, PS4 appended-member acceptance (R3-IMG-2), the imagery readings — user cards.
- Research unknowns carried: gen7 id ↔ league mapping for division logos / trophies; NHL 07 dictionary
  labels; league row growth tolerance; audio bank index (blocked, format).

## 4. Cleanup

- Removed: the scratch example `crates/studio-domain/examples/logo_probe.rs`; test temp dirs
  (`nhlms-*`) are dropped by the tests. Session scratchpad scripts (`d1_*` … `d8_*`, `Cargo.lock.before-d4`,
  `d1_affected.log`) are disposable and live outside the repo.
- Retained (durable evidence): the five generators under `local/runs/overnight-development-2026-09-19/`,
  the private fixtures listed in STATUS (all regenerable; read-only), `defs/settings/ps4-nhl15.toml`,
  `defs/league_imagery.toml`, the regenerated `docs/audits/data/mutation-matrix.csv` (+1 record, D1).
- No background processes remain; `zz_diag` was never launched.

## 5. Runnable commands (all from the repo root)

```sh
cargo test -p tdb-defs --test settings_sites
cargo test -p studio-domain --test settings_catalogue --test settings_edit --test settings_csv_edit --test league_logo_packages --test league_imagery --test league_logos --test membership
cargo test -p tdb-loc --test growth --test locale_coverage
cargo test -p tdb-archive-codec --test chunkzip_native
cargo test -p tdb-archive-big --lib stage::tests
cargo test -p studio-api --test settings_csv_route --test dictionary_growth --test asset_member_route --lib bindings
cargo run -p studio-api --bin export-bindings -- --check
cd app && npx tsc --noEmit -p . && npx vitest run
# private fixtures (read-only), opt-in:
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test league_logo_packages -- --include-ignored
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test asset_member_route -- --ignored
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p tdb-loc --test locale_coverage -- --include-ignored
```

## 6. Continuation

Codex review of receipts 01–07 (nothing is named for commit yet). If development resumes before review:
the next bounded items in priority order are (a) the gen7 inner-BIGF span-preserving re-plan for
changed-length logo packages, (b) the canonical dictionary serializer C with proofs C1/C2, (c) the
different-dimension logo replace behind a runtime card. Phase 2 (teams) begins only once those are either
done or recorded as external/manual blockers, per the brief.
