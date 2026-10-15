# Receipt 07 — C8: Phase 2 opening — Team identity data (2.1): names and colours

Developer lane, overnight 2026-09-20. **Developer-verified; not Codex-accepted.** Baseline `8d60a6d`.

## Plain-language summary

Phase 2 (Teams) opens with the identity step. The team page now shows the name cells each title
really carries — on PS4 NHL 17–21 the split **City** / **Team Name** plus the **Nickname**, on PS4 16
the short name plus nickname, elsewhere the full/abbreviated names as before — and a new **Colours**
section: the team's primary and secondary colours on NHL 11–15 / Legacy (both consoles) and PS4 15,
six front-end / presentation / overlay colours on PS4 16, nine on PS4 17–21. Each colour is a swatch
with a `#rrggbb` box. Under the hood the colour is stored two different ways depending on the title
(three separate red/green/blue bytes, or one packed number), and the byte order of the packed number
was **measured** on the anchors, not assumed: the same clubs carry the same colour on NHL 16 (three
bytes) and NHL 17 (one word). A colour edit writes all of its parts as **one** operation, so a colour
can never be left half-changed, and one undo restores all of it. NHL 08–10 have no colour on the
team row, so they get no Colours section — nothing is shown that the title cannot store. Nothing was
written to a game; the user cards ask whether the game picks the changed colour up on its screens.

## 1. Package, disposition, scope

C8 — delivered, developer-verified. Roadmap 2.1 ("Team name/city/abbreviations/codes, colors, type
and relevant identity/art references"), the name and colour part. **Not** in this package: `artid` /
`index` re-identification (research handoff 07 §1 lists the cascade — `stockteamjerseys.teamartid`,
`stockawardstable.teamartid`, vault packs, registry files for `artid`; every `→ index` column plus the
schedule slots for `index` — a refuse-or-cascade contract is not yet accepted), `forceteamcolors`
semantics (ambiguous per the census), the stanchion / goal-post / visor colours (presentation, 2.5),
localization of the names (2.3; `locstring` keys per research handoff 23 are a separate operation
already available through the dictionary panel).

Eligibility: the remaining Phase 1 work is research-blocked or runtime-blocked (receipt 06 §6) or a
background evidence sweep (§5 below); research handoff 07 (teams, delivered 09:40 — read, not edited)
proposes that an identity edit keeping `index` and `artid` is dependency-free at the database level,
which is exactly the scope built here.

## 2. Changed paths (SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/studio-domain/src/field_profile.rs` | `1068a00bc3986298` | `CellControl::Color`; `ProfileCell.color_components: Vec<FieldView>`; `TeamProfileCtx::color(label, stems)` (packed word `<stem>` else `<stem>_r/_g/_b` triple on the raw `kOtt` row); Identity cells **City** (`cityname`), **Team Name** (`teamname`), **Nickname** (`nickname`); **Colours** section for the eleven stems, emitted only when the title carries at least one |
| `crates/studio-api/src/dto.rs` | `1ea33be53615171a` | `FieldCellDto.control` gains `"color"`; `FieldCellDto.color_fields: Vec<FieldViewDto>`; new `CellWriteDto {field, value}` |
| `crates/studio-api/src/convert.rs` | `493d061115f455d2` | `"color"` mapping; `color_fields` from `color_components` |
| `crates/studio-api/src/registry.rs` | `33aaef4bff401bea` | `Registry::set_cells_int(id, table, record, writes)` — one workspace op (existing `Workspace::set_cells_int`, roster-mirrored like `set_field`); empty write refused |
| `crates/studio-api/src/commands.rs`, `lib.rs` | `8ea96167576911e7` / `ca0042c3e8b8e0df` | Tauri command `set_cells_int` (async, off-main), registered (167 command entries in `lib.rs`) |
| `crates/studio-api/src/bindings.rs` | `29f5a599654564a2` | root `CellWriteDto` (count 118) |
| `app/src/bindings/FieldCellDto.ts`, `CellWriteDto.ts` | generated | `--write`; `--check` in sync (226 files) |
| `app/src/components/ColorField.tsx` | `73c0194ce8a7dd9e` | **new** — swatch + hex box; `hexOf` / `writesFor` (packed ↔ triple); one `set_cells_int` per commit; invalid / unchanged input never dispatches; read-only when a component is not editable |
| `app/src/components/ColorField.test.tsx` | `4ecc1cba75de4389` | **new** — 4 jsdom cases |
| `app/src/components/FieldProfile.tsx`, `app/src/lib/client.ts`, `app/src/pages/TeamDetail.tsx` | `f0cc88e3de37cfa3` / `8537d45d9d131ad6` / (final hash in the closeout receipt) | `"color"` control dispatch; `set_cells_int` typing; the Misc-fields list treats a colour cell's `_g`/`_b` components as curated (no raw duplicates; player cells carry no colours) |
| `crates/studio-domain/tests/field_profile.rs` | `2206213405429441` | `assert_profile_targets_exist` now checks colour components; +2 tests: per-title colour/name shapes on 9 anchors; packed-word byte order proven across NHL 16 ↔ 17 |
| `crates/studio-api/tests/registry.rs` | `f0c98eadfa90b178` | +1 route case: Legacy primary triple written as one op (3 edits), profile reads it back, one undo restores all three, empty write refused |

## 3. Measured facts (developer evidence, anchors)

- **Colour storage shape per title** (team row `kOtt` / `exhibitionteams`): NHL 08–10 — none;
  NHL 11–15, Legacy (Xbox 360 and PS3) and PS4 15 — `primarycolor_{r,g,b}`, `secondarycolor_{r,g,b}`;
  PS4 16 — `feprimarycolor`, `fesecondarycolor`, `presprimarycolor`, `pressecondarycolor`,
  `overlayprimarycolor`, `overlaysecondarycolor`, each as an `_r/_g/_b` triple (six colours);
  PS4 17–21 — those six plus `fetertiarycolor`, `prestertiarycolor`, `overlaytertiarycolor` as one
  packed word each (nine colours). NHL 07 (gen6) is not profiled for colours (no stems).
- **Packed word byte order** `(r << 16) | (g << 8) | b`: on the NHL 16 and NHL 17 PS4 anchors at least
  20 clubs (matched by abbreviation) carry the identical front-end primary when NHL 16's triple is
  packed in that order (`packed_colour_word_is_r_g_b_big_endian_order`); the earlier probe showed
  Boston `dd 9b 26` ↔ `0xdd9b26` and NHL 15's `primarycolor` `ff ad 0f` ↔ NHL 17's `presprimarycolor`
  `0xffad0f`.
- **Name fields:** PS4 17–21 carry `cityname` + `teamname` + `nickname`; PS4 16 `shortname` + `nickname`;
  other titles `fullname` / `abbrname` / `shortname` / `artabbr` as before. Which of the split names each
  screen composes is **research unknown** → card.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `cargo build -p studio-api` | clean (pre-existing warnings only) |
| `cargo test -p studio-domain --test field_profile` | 8 passed (+2) |
| `cargo test -p studio-api --test registry` | 7 passed (+1) |
| `cargo test -p studio-api --lib bindings` / `export-bindings --check` | 25 passed / in sync (118 roots, 226 files) |
| `cargo test -p studio-api --test membership --test leagues`; `-p studio-domain --test workspace --test pending` | 5 / 3 / 10 / 3 passed |
| `cd app && npx tsc --noEmit -p . && npx vitest run` | clean / **18 files, 132 passed** (+1 file, +4) |

Not run: full workspace suite, clippy / fmt (baseline defects C0.2), the `zz_diag` suite.

## 5. Background real-archive sweeps (from receipt 03 §4 — results)

- **PS4 `inxp_10*.big` (C2):** NHL 15, 16, 17, 18, 19 (receipt 03) and now **NHL 20** (3 757 204 192 B,
  37 690 entries; staged 1 047 s, committed 1 870 s, restored 2 596 s) and **NHL 21** (2 057 349 808 B,
  21 181 entries; 679 / 1 092 / 1 472 s) — **7 / 7 passed**: logo (`size256/l0`) + imagery
  (`size512/l0`) private copies appended, `_fr` twins left on the shared bytes, restore byte-exact.
- **gen7 EB (C3) and BIG4 (C4) queues:** running at the time of this receipt (Xbox Legacy `cache.big`
  fit-in-slot lines already logged); results in the closeout receipt.

## 6. Coverage after C8 (Phase 2, 2.1 — first entries)

| Operation | X360 07 | X360 08–10 | X360 11–15 / Legacy | PS3 08–10 | PS3 11–15 / Legacy | PS4 15 | PS4 16 | PS4 17–21 | Recomp |
|---|---|---|---|---|---|---|---|---|---|
| Team names (full / abbr / short / art abbr) | impl+tested (stock/custom row) | impl+tested | impl+tested | impl+tested | impl+tested | impl+tested | impl+tested (+ nickname) | impl+tested (city / team name / nickname) | by identity with Xbox Legacy |
| Team colours | — (no stems) | — (no fields on the team row: measured) | **impl+tested (C8)** — 2 triples | — | **impl+tested (C8)** — 2 triples | **impl+tested (C8)** — 2 triples | **impl+tested (C8)** — 6 triples | **impl+tested (C8)** — 9 packed words | by identity |
| `artid` / `index` re-identification | not impl — cascade contract (research 07 §1) not accepted | ← | ← | ← | ← | ← | ← | ← | ← |

Runtime: which screens consume which colour (front-end vs presentation vs overlay; `forceteamcolors`)
and which composed name each screen shows are user cards (MORNING-REVIEW T-1 / T-2).

## 7. Temporary artifacts / processes

- Scratchpad `c8_*.py` edit scripts — disposable; the two probe tests (`zz_team_fields_probe.rs`,
  `zz_color_probe.rs`) were removed before this receipt. No temp dirs; no new fixtures.
- Background sweep runners (`c3_run.sh`, `c4_run.sh` + copied exes/logs) stay until their results are
  recorded in the closeout receipt, then are removed.

## 8. Runnable commands

```sh
cargo test -p studio-domain --test field_profile
cargo test -p studio-api --test registry team_colour
cargo run -p studio-api --bin export-bindings -- --check
cd app && npx vitest run src/components/ColorField.test.tsx
```
