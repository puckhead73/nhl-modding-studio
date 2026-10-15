# Receipt 02 — D2 / P1-BS-2: read-only settings catalogue in Studio

Developer lane, overnight 2026-09-19. **Developer-verified; not Codex-accepted.**

## Plain-language summary

Studio now shows the game's mode-setup settings catalogue for every PS4 title — a new **Game Settings**
tab under Game Database. For NHL 15 it reads the five catalogue tables of the open database; for NHL 16–21
it reads the CSV text files inside the install's `inxp_00.big`. Each row shows the setting, its range or
choice codes with the labels they map to, the stored global default, the stored default of every screen
that has one, and whether the row is in the accepted stored-default editing contract. Names the title does
not store itself (NHL 15 screen names, NHL 15/16 choice labels) are marked as inferred; NHL 18–21's two
stored copies of the global default are shown as a pair with their agreement. Nothing is editable yet and
nothing on the page claims what the game offers or applies. Xbox 360, PS3 and NHL 07–09 show the reason no
catalogue exists there.

## 1. Package, disposition, scope

- **D2 / P1-BS-2** — delivered, developer-verified, pending independent review.
- Scope: NHL 15 five-table catalogue and NHL 16–21 `setting_csv` catalogue (three schema variants) through
  domain → API → UI; row identity = legacy id; shared identities informational; raw codes, inferred /
  explicit labels, anomaly disclosure; async UI results bound to project + install root; component/API/
  domain coverage. No writer, no claims about runtime behavior.

## 2. Changed paths and hashes (baseline `8d60a6d`)

| Path | SHA-256 | Change |
|---|---|---|
| `defs/settings/ps4-nhl15.toml` | `c2d768009227abf94adbf3bac7641bd6ab66ad517a3d4aa46ba4c11f3aeee3c9` | **new, generated** — NHL 15 PS4 support data: 57 context lineage names (NHL 16 key / NHL 21 name), 79 group lineage names, 133 label arrays (the executable descriptor records that carry one; array order = code, inferred), the 21 eligible ids with roles, 16 exclusion reasons, witness identities |
| `local/runs/overnight-development-2026-09-19/gen_settings_defs.py` | `012136f1…` | the generator (inputs: research handoff 22 data JSONs; deterministic) |
| `crates/studio-domain/src/settings_catalogue.rs` | `12716cd6507f302bc20f96f3449f9e3358444f2c81e368f136462465ab175562` | **new** — defs loader, unified model, NHL 15 DB reader, CSV member reader (EB streaming / directory / texts), subset CSV parser, variant detection, blockers |
| `crates/studio-domain/src/lib.rs` | `72c25c07…` | `pub mod settings_catalogue` |
| `crates/studio-domain/src/project.rs` | `ea1de22e…` | `Project.settings` (`defs/settings`), `settings_defs()` / `set_settings_defs()` |
| `crates/studio-domain/src/session.rs` | `f5f6d6e5…` | `EditSession::settings_catalogue(install_root)` routing: NHL 15 tables → DB reader; PS4 16–21 → `<root>/databigs/inxp_00.big`; else blocker |
| `crates/studio-domain/tests/settings_catalogue.rs` | `bc29e5f7…` | **new** — 13 cases (2 `--ignored`) |
| `crates/studio-api/src/dto.rs` | `ddcbbaa2…` | 11 DTOs (`SettingsCatalogueResultDto` root) |
| `crates/studio-api/src/convert.rs` | `58f93663…` | `settings_catalogue_result_dto` |
| `crates/studio-api/src/registry.rs` / `commands.rs` / `lib.rs` | `35092672…` / `47b30dd3…` / `c7160e84…` | `settings_catalogue(project_id, install_root)` command |
| `crates/studio-api/src/bindings.rs` | `c31bb44a…` | export root added; root count 108 → 109 |
| `app/src/bindings/{SettingChoiceDto,SettingContextDefaultDto,SettingContextDto,SettingGroupDto,SettingRowDto,SettingsCatalogueDto,SettingsCatalogueResultDto,SettingsMemberDto,SettingsSourceDto,SettingsTableDto,StoredDefaultDto}.ts` | generated | `export-bindings --write`; `--check` clean |
| `app/src/lib/client.ts` | `0f7f7a77…` | `settings_catalogue` typing |
| `app/src/pages/GameSettings.tsx` | `cc115f1c…` | **new** page |
| `app/src/pages/GameSettings.test.tsx` | `350e46ea…` | **new** — 5 jsdom cases |
| `app/src/routes.tsx` / `app/src/nav.ts` | `eeaf0816…` / `566b9dcf…` | route `database/game-settings`, subtab "Game Settings" |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | `a1e9f0dc…` | 1.2 "Rules, season/tournament/playoff settings" row rewritten per family |

Private fixtures added (read-only inputs, outside the repository, extracted from the installs with
`local/runs/overnight-development-2026-09-19/extract_setting_csv_fixtures.py`, which reuses the 2026-09-18
research probes unmodified): `E:\nhl-studio-fixtures\databases\nhl{16,17,18,19,20,21}-ps4-<cusa>\inxp_00.big\setting_csv\*.csv`
— 65 plain members; every plain SHA-256 equals the research census (`p1bs-csv-sources.json`, 65/65);
manifest with archive/entry/offset/stored+plain hashes at `…\databases\ps4-setting-csv-manifest.json`
(`62c3b6b9…`).

## 3. Behavior and decisions

- **One model, two readers.** `SettingsCatalogue { source, contexts, groups, settings, absent_eligible,
  anomalies, notes }`; a `SettingRow` is keyed by the legacy id and carries `global: StoredDefault`,
  `context_defaults`, `choices`, `label_source`, `eligibility` (`Eligible{role}` / `Excluded{reason}` /
  `OutOfScope`) and row anomalies. Every stored default names its exact cell(s): `stocksettings record r
  (legacyid l) field default`, `contextsetting.csv line n (context c, setting s) column default_value`.
- **NHL 15.** Requires the five tables *and* the P1-BS-1 scoped sites to be active on the declaration
  (`declaration-mismatch` blocker otherwise — tested with a re-declared `min`). Copy identity by whole-DB
  SHA-256 against the defs witness (`base 01.00` / `patch 01.06` / `unrecorded …` for an edited working
  copy). Context/group names are lineage (`NameSource::Lineage`), labels inferred; the `subgrouporder`
  structural-only distinction is carried into the notes when `tdb-defs` reports it. Stock anomalies
  reported once and preserved: dangling 236 ×4, contexts 49–52 without layout rows, containers 36/74/75,
  duplicate (80, 94), 59 rows in no group, global defaults outside their own range, label-array/size
  mismatches, executable-vs-DB setting-id disagreement (none in stock).
- **NHL 16–21.** Variant detected from `setting.csv` columns (16: `key`+`remap_id`; 17: `legacy_id` without
  `default_value`; 18+: `default_value`+`potential_values`). Global default: 16 = one `setting.csv` cell; 17 =
  the reserved `_DEFAULT_` (−2) row; 18–21 = **pair** (`paired`, `values`, `consistent`; disagreeing or
  different-list pairs flagged, a missing `_DEFAULT_` row reported). The −2 context is `reserved` and never
  listed among a setting's screens. Per-context rows carry their own list when `potential_values` differs
  (88's tournament rows, list 12), `is_enabled`/`is_visible`, NHL 16 override presence ("not
  interpreted"), null cells, values outside the row's domain. Labels: 17–21 explicit from `choice.csv`
  (`name_loc`, `value` = code; a list with `id ≠ value` is reported ambiguous and not applied); 16 = the
  NHL 15 arrays by lineage only when the array length equals the row's span (else withheld with the
  reason — `StartingRosters` 0..1 vs 3 entries). NHL 17 siblings by `legacy_id`; 16/18–21 note that no
  identity column exists.
- **CSV subset parser** (handoff 01 §3.4, read side): BOM, any `"` byte, non-ASCII/NUL → member refused
  with the reason (browse-only); duplicate required header → refused; CRLF or LF records; ragged
  records skipped and reported; extra columns carried by name.
- **Members.** `SettingCsvMembers::from_eb_archive` streams only the front tables and the
  `setting_csv/` entries (chunkzip decoded through the existing codec; chunklzx refused); `from_dir` for
  fixtures; `from_texts` for synthetic cases. Origin, plain length/SHA-256 and stored form per member are
  reported in the source block.
- **Routing.** PS4 + five tables → DB reader (title identity by tables, not by name). PS4 16–21 without an
  install root → `no-install-root`; archive absent → `archive-missing`. Xbox/PS3/Recomp/07–09 →
  `unsupported-title` with the research reason. No gen7 executable-table reader in this checkpoint.
- **UI.** Contract-first scope (eligible + excluded) with an "all N settings" switch and search; row list
  (id, name — dictionary text when attached, key otherwise — role, range, global default with
  inferred/lineage pills and the pair state, screen count, contract pill with the exclusion reason on
  hover); detail card (cells, per-screen table with lineage marking, flags, domain notes, anomalies, codes
  with labels and their source, groups, dependents); disclosures (notes + source anomalies). Query keyed by
  `(projectId, installRoot)`; selection reset on project/root change; a late response for a replaced
  project never repaints (react-query key isolation, asserted).
- **EDIT-MODEL note (for D3).** The five NHL 15 tables declare no `update="True"`; the generic cell editor
  already accepts them (fields default to updatable). D3 records the deliberate exemption as E1 did.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test settings_catalogue -- --include-ignored` | **13 passed** (incl. the NHL 15 patch DB and the real NHL 16–21 members: 267/351/376/389/409/427 setting rows, 61/102/111/111/117/120 contexts, 570/8 306/8 944/10 232/10 703/10 969 contextsetting rows, 21/16/16/16/16/16 eligible present, no ambiguity, 72 global 3/4, context-3 = 4 everywhere, 18–21 pairs consistent on every eligible row, 88's tournament list of 8 with default 2, cup type 2 entries, tie-break authentic 7, NHL 16's 88 empty defaults) |
| `cargo test -p studio-domain --test settings_catalogue --test ps4_groups --test leagues` | 10 + 11 passed (3 ignored by name) |
| `cargo test -p studio-api --lib bindings` | 25 passed (root count updated to 109) |
| `cargo run -p studio-api --bin export-bindings -- --check` | "Bindings are in sync." after `--write` (11 new files) |
| `cd app && npx tsc --noEmit -p .` | clean |
| `cd app && npx vitest run` | **13 files, 102 passed** (5 new) |

Not run: the full workspace suite, `zz_diag`, clippy/fmt (baseline defects), desktop launch, any game.
Visual review of the page is a morning card.

## 5. Remaining blockers / limitations

- Runtime: which context a screen reads, precedence, `is_enabled`/`is_visible`/`usage` meanings — unknown
  (cards P1BS-1 / P1BS-CSV-1, user execution).
- NHL 15 shows lineage names and inferred labels only; NHL 16 labels are the NHL 15 arrays by lineage.
- Gen7 (10–15/Legacy) executable table: not read (recorded reason); 07–09: nothing found; PS3 by reference.
- The page needs an install-root project for NHL 16–21 (file-pick projects get the `no-install-root` reason).
- Dictionary label resolution is only as good as the attached dictionary (setting `stringid`/`name_loc`
  keys resolve exactly or by hash; unresolved keys display as keys).

## 6. Temporary artifacts / processes

- Session scratchpad `d2_api.py` — disposable.
- `local/runs/overnight-development-2026-09-19/{gen_settings_defs.py, extract_setting_csv_fixtures.py}` —
  retained (small, regenerable generators; owner: developer lane; needed to regenerate the defs file and
  the fixtures).
- Private fixtures (above) — retained, read-only inputs of the `--ignored` cases.
- No new build tree, no processes left.

## 7. Next

D3 — one-cell stored-default editing on NHL 15 PS4 (handoff 22 §2 with the acceptance note), then the
NHL 16–21 extension gated on D4 packaging.
