# Receipt 01 — D1 / P1-BS-1: scoped settings-catalogue decoding

Developer lane, overnight 2026-09-19. **Developer-verified; not Codex-accepted.**

## Plain-language summary

Studio now reads four columns of the NHL 15 (PS4) settings catalogue the way the game stores them — "raw
plus the declared minimum" — instead of as signed numbers. Setting ranges, setting identities and the
screen-group parents now decode to the values the game's own executable carries (for example
`PlayoffSeriesLength` is setting 152 with range 0–4, the advantage sliders run −7…+7). The fix is bound to
exactly that table declaration, so no other title, table or field changes its reading. Nothing is edited
by this step; it is the prerequisite for the read-only settings catalogue (D2) and the one-cell default
editor (D3). Whether the game shows or uses any stored default remains unknown.

## 1. Package, disposition, scope

- **D1 / P1-BS-1** — delivered, developer-verified, pending independent review.
- Scope: the accepted table-plus-field, declaration-shape-scoped offset-binary interpretation for
  `fRvu.SbEu` (`stocksettings.min`), `fRvu.CkjP` (`stocksettings.id`), `HwRX.QHRR`
  (`stocksettingcontext.subgroupparent`), `HwRX.NVBd` (`stocksettingcontext.subgrouporder`);
  regression coverage for the unrelated `qbPU.CkjP`, the anchored career plus/minus sites and other
  signed/sentinel fields; NHL 15 PS4 base anchor and patch 01.06 fixture. No defaults editor, no
  general negative-range rule change.

## 2. Changed paths and hashes (baseline `8d60a6d`, dirty tree preserved)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-defs/src/lib.rs` | `97c55578e8fa64868d64b9d452de94d8542cf3ae121f5896ab2b6c6b8a3f1f5e` | `DeclaredField`, `OffsetEvidence`, `ScopedOffsetSite`, `SCOPED_OFFSET_BINARY_SITES` (4 sites), whole-table shape match in `from_xml`; `FieldMeta.offset_evidence`; `neg1_sentinel()` is false for an offset site |
| `crates/tdb-defs/tests/settings_sites.rs` | `19444696c0e495762011a02b70715cde342bdfcafc0d0adaf6d739c7c8564d27` | new — 10 cases (1 `--ignored`) |
| `crates/tdb-defs/Cargo.toml` | `94e048610245f9ba52551cf2853bcea0d65eea7a29007328977252fbc69e329b` | dev-dependency `sha2 = "0.10"` (already in the lockfile graph) |
| `Cargo.lock` | — | `tdb-defs` dev-dep entry `sha2` (the file was already dirty at baseline with unrelated entries) |
| `docs/audits/data/mutation-matrix.csv` | `212a7ed28cb2d8e109b6b571f33f52b4738bfd06247384ad88e15ee0c2484cb4` | +1 record: `ps4,nhl15,offset-binary,HwRX,NVBd,5,0,-1,15,yes,yes` — regenerated with the documented `QA_WRITE=1` path because the codec classification of NHL 15 PS4 changed as a direct result of this package (not an opportunistic regeneration; the `capability.csv` tree is untouched) |

Private fixture added (read-only input, outside the repository):
`E:\nhl-studio-fixtures\databases\nhl15-ps4-cusa00561\patch_inxp00.big\nhl_game.db`, 16 981 204 B,
SHA-256 `abe36f58fd0a65ab074aff13f83f2e671b707afc539ee57c01df128200c320ff` — entry 0
(`nhl/data/common/gamemodes/database/t3db/nhl_game.db`, raw, not chunkzip) of
`H:\Emulators\ShadPS4\Games\CUSA00561\databigs\patch_inxp00.big`
(`629091b6786cd6878c8f99cd8299a3288cf05b1c08bbbde75b5b62b3dce41ed1`, 563 entries, EB v0x00100400).
Extracted read-only with the previous campaign's `local/runs/overnight-research-2026-09-18/probes/eb_list.py`.
The patch archive has **no `nhl_game-meta.xml` member** (only the DB under `t3db/`), so the anchor metadata
`anchors/ps4/nhl15/databases/nhl_game-meta.xml` (`d9cf1be40ffc8180c173e460bf3a120e6b719b0fcd70a939b8c232d07ac085c3`)
is the declaration for both copies — handoff 22 §1.9 records the same bytes for the patch metadata.

## 3. Behavior and decisions

- **Keying.** A scoped site matches only when (a) the raw table id, (b) the raw field id and (c) the
  table's **complete declared field set** (every raw field id with its depth and legal range,
  order-independent) equal the recorded NHL 15 PS4 declaration. A different depth or range on the site
  or on any sibling field, an extra or missing field, or the same shape under another table id all
  refuse the site and fall back to the previous reading (two's complement / sentinel / plain). This is
  stricter than the field-shape-only minimum: it keeps the reading off any differently declared
  variant while still covering the patch DB and an unseen NHL 15 PS4 build with identical metadata.
- **Evidence preserved.** `FieldMeta.offset_evidence` carries `Anchored` (the two career plus/minus
  sites), `ExecutableWitnessed` (`min`, `id`), `LineageInferred` (`subgroupparent`) or
  `StructuralOnly` (`subgrouporder`). `subgrouporder` is registered **with** its structural-only mark
  (the handoff offered either choice; recorded here): it decodes consistently (−1 on every row that
  has no parent or a 74/75 container parent, 1..6 on the six sub-groups under 36), and the mark lets
  D2 disclose it as unverified externally rather than silently promoting it. No claim that every
  field was externally verified.
- **No general rule.** `jrow.zwIO` (11-bit −512..1535) and every other negative-`rangelow` field keep
  their previous reading; the sweep test asserts exactly the six listed sites take the offset reading
  on NHL 15 PS4 and that no other committed anchor's metadata takes a scoped site.
- **`neg1_sentinel()`** now returns false for an offset site (`subgroupparent` would otherwise have
  classified as a sentinel while decoding as offset).
- `read_inline_str` / `read_u64` in `tdb-core` are unchanged; all decode paths (`view.rs`,
  `session.rs`, `import.rs`, `tdb-edit`) go through `FieldMeta`, so the correction reaches every
  Studio surface automatically.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p tdb-defs -- --include-ignored` | 3 unit + 2 `meta` + **10 `settings_sites` passed** (0 failed, 0 skipped), incl. the patch fixture case |
| `cargo test -p tdb-defs` (default) | 9 passed, **1 ignored** (the patch case, by name) |
| `cargo test -p studio-domain --test mutation_matrix` | failed once on CSV drift (+1 legitimate record, above); after `QA_WRITE=1` regeneration: 1 passed (119 s) |
| `cargo test -p studio-domain --test view` | ran with the above (no failures reported; the view suite reports 0 tests in this target configuration) |
| `cargo test -p tdb-canonical` | 552 passed |

`settings_sites` asserts, through the real parser on the anchor `ae9d1d56…`: 223/373/197/702/25 packed
rows; `size == max − min + 1` on exactly 213 rows; two negative `min` rows (`PlayerAdvantage`,
`GoalieAdvantage` −7..7, size 15); `PeriodLength` 3..20 size 18 default 5; `TXT_MUD_MAX_USERS` 1..24;
`id 152` on legacyids 72/86/116; ids 18/155/156/162 on 68/71/73/70; `SHOOTING_CONTROL_TOGGLE` id −1;
every one of the 21 eligible ids has `min 0` and `size == max + 1`; `subgroupparent ∈ {−1, 36, 74, 75}`
with 36/4/2 rows, `subgrouporder ∈ {−1, 1..6}` exactly on the 36-children; dangling defaults
`(49..52, 236)`; `leaguesimpopups.id` raw == value on every row; base copy: legacyid 82 = id 158 0..1, no
context-9 defaults. Patch `abe36f58…`: 223/379/201/704/25; legacyid 82 → id 186, 0..3, size 4; six
context-9 defaults at records 193–198 (73→0, 82→3, 86→3, 3→5); key order preserved. Raw→logical→raw
identity on every raw value of each site; declaration-mismatch cases (a)–(g).

Not run: full workspace suite, frontend, bindings export (no DTO changed), clippy/fmt (known baseline
defects). No game launch, no live write.

## 5. Remaining blockers / limitations

- Only NHL 15 PS4 builds 01.00 (anchor) and 01.06 (patch) exist on this machine; any other build
  is covered only if its metadata declares the same shape.
- `subgrouporder` remains structurally inferred (no external witness); D2 must disclose it.
- Consumer code, precedence and selected-value storage stay unknown (handoff 22 §9).

## 6. Temporary artifacts / processes

- Session scratchpad: `d1_patch.py` (the edit script), `d1_affected.log` — disposable, session-owned,
  removed at lane close.
- Private fixture (above) — retained, read-only input, owner: developer lane / test
  `nhl15_ps4_patch_01_06_database_decodes_with_the_same_scoped_sites`.
- No new build tree (shared `target/` reused), no processes left.

## 7. Next

D2 — read-only settings catalogue through domain/API/UI. Independent review of this receipt is
requested in the morning; the runnable commands are the two `cargo test` lines above.
