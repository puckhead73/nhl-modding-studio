# Receipt 21 (review item 8e) — equipment catalogues label only the title they were established on; every other title shows the stored id as a number, with the reason

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p8e-equipment-scope`. Clock: 14:40 → 15:10 (MDT, 25 Sep 2026).
Direction: the coordinator's review-item 8e decision (title-scoped vocabularies; ids observed / labels
established / values permitted kept apart; unknown ids visible and untouched; no borrowed label, no
silent default, no silent unsupported write) as bounded by the development audit ("not in the
shipped catalogue" is not proof an id is invalid; a label alone never authorises a write).

## 1. Plain-language summary

The stick, glove, helmet, skate, visor, tape and tuck pickers used to show NHL Legacy's model names
on every title from NHL 08 to PS4 NHL 21, because the vocabulary files bound to a table and field
with no idea which game they were read from. Each of those ten vocabulary files now carries a
scope — NHL Legacy, the title whose textures and catalogue CSVs the labels were read from — and the
registry resolves a scoped vocabulary only for a project opened as that title. Everywhere else
the Equipment tab shows the stored number, a small "no catalogue for this title" marker, and a
tooltip saying whose labels exist and why they are not applied; a project with no declared title
gets no equipment labels either. Nothing about writing changed: any id inside the field's range is
accepted and round-trips untouched through unrelated edits, save and reopen, on Legacy as on every
other title — a modder's own asset ids are not refused for being absent from EA's catalogue, and a
label never authorises or forbids a write. Where a scoped catalogue does apply, the tooltip names
its provenance, so a shared definition keeps its source.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-enums/src/lib.rs` | `EnumScope { games, platforms }` (`[scope]` in the TOML); `EnumDef.scope`, `applies_to`, `provenance`; `EnumRegistry` keeps every def per binding site, `set_scope` / `scoped` (game id + platform), `for_field` / `label` / `validate` resolve the applicable def, `def_named`, `vocabulary_note`; unit tests: scope parsing, unknown / other title → no labels, a label shared by two titles resolves to each title's own id, platform lists honoured |
| `defs/enums/{stick-model,glove-model,helmet-model,skate-model,visor-style,visor-tint,stick-tape-color,jersey-tuck,sock-tuck,pant-length}.toml` | `[scope] games = ["nhllegacy"]` with the decision recorded in a comment (labels read off Legacy PS3 textures / catalogue CSVs; Xbox 360 shares the id → file grammar; Recompiled by identity; no platform split — the evidence does not require one) |
| `crates/studio-domain/src/project.rs` | the registry is scoped to `(None, platform)` at load and to `(game id, platform)` in `set_game_id` |
| `crates/studio-domain/src/view.rs` | `FieldView.vocabulary_note` (provenance when a scoped catalogue applies; "no catalogue for <title>" when the site's only catalogues belong to other titles; `None` otherwise) |
| `crates/studio-domain/src/equipment_slots.rs` | `slot_enum_label` resolves the vocabulary by name **under the registry's scope** |
| `crates/studio-api/src/dto.rs`, `convert.rs` | `FieldViewDto.vocabulary_note` |
| `crates/studio-api/src/registry.rs`, `commands.rs` | the hockey-shop / equipment-file label lookups take the project's title (or the caller's `game` / `gameId`) and scope the bundled registry; `list_equipment_slot_assets` and `hockey_shop_sync` gain an optional game id |
| `app/src/bindings/FieldViewDto.ts` | regenerated (`export-bindings --write`; in sync) |
| `app/src/components/FieldRow.tsx` | "no catalogue for this title" marker beside an uncatalogued number (reason in the tooltip); the label tooltip carries the vocabulary note with the raw id behind it |
| `app/src/lib/client.ts`, `app/src/pages/HockeyShop.tsx` | optional `game` / `gameId` on the two shop commands; the shop passes the install's game id |
| `crates/studio-domain/tests/equipment_scope.rs` (**new**) | 27-anchor sweep: labels only on Legacy (PS3 / X360), numbers + note elsewhere and with no title; the range-top stick id written, kept through an unrelated edit, saved, read back from the bytes; every skater-equipment vocabulary carries exactly the Legacy scope and nothing else is scoped |
| `app/src/components/FieldRow.test.tsx` (**new**) | marker + tooltip on an out-of-scope title; label + provenance on the catalogue's title; nothing for a field without a vocabulary |

## 3. Evidence

- [`p8e-equipment-scope-sweep.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p8e-equipment-scope-sweep.log)
  — the 27-row table: "labelled (NHL Legacy catalogues)" on `ps3/nhllegacy` and `xbox360/nhllegacy`
  only; "numbers + note" on the other 23 titles with an equipment companion; NHL 07 has none; the
  range-top stick id (190 / 31 / 127 / 255 / 511 / 1023 by title) round-trips everywhere.
- [`p8e-tdb-enums.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p8e-tdb-enums.log)
  — 5 unit + 2 integration cases (the shipped registry still loads 30 defs and cross-checks the anchors).

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p tdb-enums` | 7 passed |
| `cargo test -p studio-domain --test equipment_scope` (27 anchors) | **2 passed** |
| `cargo test -p studio-api --test player_sticks` (27 anchors) | 1 passed (labels asserted only where a vocabulary applies) |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-api --test equipment_files -- --ignored` (real Legacy + NHL 11 Xbox 360 installs, read-only) | 1 passed |
| `cargo run -p studio-api --bin export-bindings -- --write` then `--check` | wrote 1 (`FieldViewDto.ts`); in sync (277 files) |
| `npx vitest run` (full) | **39 files, 239 passed** (FieldRow 3 new) |
| `npx tsc --noEmit` | clean |

Repo-wide fmt / clippy remain the recorded C0.2 baseline defects; not re-measured.

## 5. What is and is not established

| Concept | Where it lives now |
|---|---|
| ids observed in a file | the cell value, shown as a number wherever no catalogue applies; never rewritten |
| labels established by evidence | the scoped TOML entries (Legacy only today; per-title catalogues for other titles would be new `[scope]`d files) |
| values permitted for editing | the field's metadata range (`tdb-defs`); unchanged — a catalogue neither widens nor narrows it |
| asset-backed ids | `check_player_equipment_files` (the install's rendering tree), unchanged; now labelled under the project's title only |

- **Platform split:** none. The labels were read on PS3 Legacy; Xbox 360 Legacy shares the
  id → file grammar (`texlib_<id>.rx2` vs `.rpsgl`) and the real Legacy Xbox 360 install passes the
  equipment-file check, but no one has compared a painted Xbox texture to its PS3 twin. The scope
  covers both platforms on that basis; a platform list can be added to a file the moment evidence
  requires it.
- **Other titles:** NHL 08 – PS4 21 have no catalogue. Whether Legacy's id 17 is the same stick
  on NHL 10 or PS4 19 is not established (receipt 19 §5) and is not claimed.
- **Goalie equipment (`fMVl`)** carries no shipped vocabulary; unaffected.
- **Appearance vocabularies** (skin / eye colour, player / goalie style) were also promoted on
  Legacy but re-verified against the anchor distributions (Spike S4) and gen6-overridden where they
  differ; they are outside this decision and stay unscoped — flagged for the coordinator (§7).
- The label → id resolution used by the historical import (`resolve_enum`) goes through the same
  scoped `for_field`, so a corpus label can only resolve to the target title's own vocabulary.

## 6. Coverage recorded (Phase 4 matrix, 4.1 correction row)

Stick / glove / helmet / skate / visor / tape pickers: **labelled on NHL Legacy (PS3, X360; Recomp by
identity)** — impl+tested; **numbers with the no-catalogue note on NHL 08–15 (both consoles) and
PS4 15–21** — impl+tested; n/a on NHL 07. Card V-61 is corrected (V-63).

## 7. Observations for the coordinator

- Item 8e is implemented as decided; the same mechanism is ready for per-title catalogues if any
  are ever established (a new `[scope]`d TOML per title, no code).
- Open question: should the appearance vocabularies (skin colour, eye colour) be scoped the same
  way? Their evidence is Legacy-only too, but their codes are small and the anchor distributions
  agreed on 21 anchors; left unscoped here so as not to widen the change without a decision.

## 8. Cleanup

Scratchpad `patch_scope.py`, `patch_scope2.py` deleted by literal path (scratchpad empty); no temp
roots (in-memory copies); no processes.

## 9. Next action

The audit's sequence continues: refresh the League / Team completion ledger against R88–R101 and the
corrected blocker map, then take the highest-impact ready League / Team item — on current reading the
gen7 `.abk` bank-tree listing (R89 / R94) and `.ast` stream framing + diagnostic decode (R91 / R92 /
R98 / R100) for the arena and team audio rows, within their measured populations.
