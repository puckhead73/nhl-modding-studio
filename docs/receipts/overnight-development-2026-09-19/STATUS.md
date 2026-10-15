# Developer lane status — overnight 2026-09-19

Brief: [overnight-development-2026-09-19.md](../../plans/overnight-development-2026-09-19.md).
Starting revision: `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (branch `feat/phase5-asset-tooling`), dirty working
tree of 585 entries preserved untouched (unrelated dirty files, enum deletions, diagnostics, root zips, evidence).
No staging, commits, pushes, resets or clean operations this lane.

Checkpoints here are **developer-verified, not Codex-accepted**. Visual/manual review is deferred to the user
(`MORNING-REVIEW.md`).

## Completed checkpoints

- D0 — P1-C-R2 accepted offline (read, not repeated).
- **D1 — P1-BS-1 scoped settings decoding** — delivered 04:55, developer-verified: [receipt 01](01-p1bs-1-scoped-settings-decoding.md). Claim `dev-p1bs-1-settings-decoding` complete.
- **D2 — read-only settings catalogue** — delivered 05:02, developer-verified: [receipt 02](02-p1bs-2-settings-catalogue-read-only.md). Claim `dev-p1bs-2-settings-catalogue` complete.
- **D3 — one-cell stored-default editing (NHL 15 PS4)** — delivered 05:18, developer-verified: [receipt 03](03-p1bs-3-stored-default-editor.md). NHL 16–21 extension deferred to D4. Claim `dev-p1bs-3-stored-default-editor` complete.
- **D4 — packaging (native chunkzip + text-member route) and the NHL 16–21 CSV stored-default edit** — delivered 05:52, developer-verified: [receipt 04](04-p1f-packaging-and-csv-default-edit.md). Logo chains (GNF route, BIGF re-plan, RefPack, .bh) not built — recorded. Claim `dev-p1f-packaging` complete.
- **D5 — localization parity (P1-D) + LOC-W3 growth (P serializer)** — delivered 06:22, developer-verified: [receipt 05](05-p1d-localization-parity-and-loc-w3-growth.md). Canonical rebuild (O-G) not built — recorded. Claim `dev-p1d-localization` complete.
- **D6 — league asset parity (P1-E) + P1-G imagery inventory + P1-H matrix** — delivered 07:40, developer-verified: [receipt 06](06-p1e-league-asset-parity.md). Different-dimension replace and gen7/PS4 archive placement of a changed-length package not built — recorded. Claim `dev-p1e-league-assets` complete.

- **D7 — Phase 1 accounting + D8 — P1-F PS4 archive-member texture route** — delivered, developer-verified: [receipt 07](07-p1f-ps4-archive-member-route-and-p1-accounting.md). Private-copy rule in the stager; PS4 logo/imagery members rewritten through the copied-target save; two matrix test gaps closed; remaining Phase 1 open cells named with next actions (receipt 07 §5). `MORNING-REVIEW.md` written (R3-IMG-2 now runnable).

## R1 correction (19 Sep, after the first Codex review)

- [OVERNIGHT-DEV-R1](../../validation/overnight-development-2026-09-19-review-r1.md) F1 (UI ownership of the
  archive-member dialog), F2 (undo history of superseded texture replacements) and F3 (coverage-claim
  reconciliation) delivered — record
  [overnight-dev-r1-corrections-2026-09-19.md](../../validation/overnight-dev-r1-corrections-2026-09-19.md).
  Developer-verified; stopped for Codex review. No new features.

## Active package

- none. Lane closed with the [final receipt](08-final-receipt.md); R1 corrections applied on top (above). Closing regression batch (D8-affected): tdb-archive-big 57 (+1 ignored); studio-domain copied_target 6, txn_coordination 3, settings_csv_edit 6 (+1 ignored), pending 3, workspace 10; studio-api asset_member_route 2 (+1 ignored), copied_target 15, dictionary_growth 3, settings_csv_route 2; bindings in sync — all passed.

## Next queue entry

- Phase 2 research-ahead only if Phase 1 is exhausted (research 05/06 delivered: targeted gaps, PS4 team identity map).

## Blockers

- none blocking; open items are recorded per receipt (§5 of each).

## Claims (this lane)

- `local/coordination/overnight-2026-09-19/claims/dev-p1bs-1-settings-decoding.json` — complete.
- `local/coordination/overnight-2026-09-19/claims/dev-p1bs-2-settings-catalogue.json` — complete.
- `local/coordination/overnight-2026-09-19/claims/dev-p1bs-3-stored-default-editor.json` — complete.
- `local/coordination/overnight-2026-09-19/claims/dev-p1f-packaging.json` — complete.
- `local/coordination/overnight-2026-09-19/claims/dev-p1d-localization.json` — complete.
- `local/coordination/overnight-2026-09-19/claims/dev-p1e-league-assets.json` — complete.

## Retained artifacts (private fixture root `E:\nhl-studio-fixtures`, read-only inputs)

- `databases\nhl15-ps4-cusa00561\patch_inxp00.big\nhl_game.db` (`abe36f58…`, the `--ignored` patch case).
- `databases\nhl{16..21}-ps4-*\inxp_00.big\setting_csv\*.csv` (65 members, census-equal) + `ps4-setting-csv-manifest.json`.
- `chunkzip\nhl21-ps4-cusa18182\overlays_21.big\*.chunkzip` (2 stored-chunk members) and
  `archives\nhl16-ps4-cusa01909\databigs\inxp_00.big` (300 MB, real NHL 16 archive for the `--ignored` round trip).
- `loc\<platform>-<title>\*.db` + `loc\manifest.json` (100 loose locale dictionaries, 126 MB).
- `logos\<platform>-<title>\<family>\l0.big` (77 gen7 packages) + `logos\ps4-<title>\<size>\l0[_fr].{gnf,png}` (34 members) + `logos\manifest.json` (6.2 MB).
- Generators (regenerable): `local/runs/overnight-development-2026-09-19/{gen_settings_defs.py, extract_setting_csv_fixtures.py, extract_locale_fixtures.py, extract_logo_fixtures.py, gen_league_imagery_defs.py}`.

## Continuation instruction

If this session ends mid-package: read this file, the latest numbered receipt and the active claim; rerun the
"runnable commands" listed in the latest receipt; continue from the "Next queue entry".
