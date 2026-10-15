# Receipt 20 (F1 / F2) — the weight, rating, birth-date and cap-hit encodings now come from a schema-family contract, never from the cells; every cap-hit unit witnessed by several public contracts

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-f1f2-encoding-contract`. Clock: 13:20 → 14:40 (MDT, 25 Sep 2026).
Direction: [development audit 2026-09-25](../../validation/development-20260925-misrouted-prompt-audit.md)
F1 and F2, which replaced the misrouted prompt's queue.

## 1. Plain-language summary

Until this package Studio decided how a player's weight, ratings and birth date were stored by
looking at the numbers currently in the file: the middle weight, the share of ratings above 49.
That reads a stock roster correctly but breaks on an edited one — set every player to 170 lb on a
title that stores pounds minus 120 and the same code would re-read the file as pounds minus 150,
then write the next edit with the wrong base. The stored format never changed; only the values
did. Studio now decides these encodings from the **shape of the database** (which draft-year field
the title carries and how wide it is, the equipment table's widths, the byte order), which no cell
edit can move, and cross-checks that shape against the title the project was opened as. A file
whose declared title disagrees with its schema refuses to write those cells in words; a schema no
known title matches shows them raw and read-only. The old value probes stay as diagnostics that
say when a file's cells look unlike its title's stock data.

The cap-hit unit is now bound to families witnessed by several independent public contracts each
(six to twenty-five players per family, not three players seen many times): NHL 08's unit, which
receipt 17 had inferred from its neighbours, is now measured against seven contracts; Crosby's
NHL 08 figure is EA's own and simply does not match his cap hit — receipt 17's "pre-extension"
explanation is withdrawn. The real NHL 20 database (private fixture, recorded hash) carries the
same contract as the NHL 19 base copy that sits under `anchors/ps4/nhl20`.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/encoding.rs` (**new**) | `SchemaFamily` (07 / 08 / 09 / 10 / 11–Legacy / PS4) classified from the schema shape; `PlayerEncoding` per family (weight base, six-bit rating offset, birthdate index, cap-hit unit — each with an `Evidence` level); `encoding_conflict` (declared title vs schema); `is_contract_encoded`; `content_diagnostics` (the old probes, reporting only) |
| `crates/studio-domain/src/view.rs` | `read_field`: rating offset, weight base and birthdate index from the contract; unresolved schema → raw display with the note, not editable; `CompanionReader::build` and the header use the contract; `cap_hit_scale` returns the family's witnessed unit; `sixbit_rating_offset` renamed `sixbit_rating_population_offset` (diagnostic) |
| `crates/studio-domain/src/session.rs` | `prepare_field_edit` refuses an encoded cell (`EditError::Refused`, in words) under a title / schema conflict or an unresolved schema |
| `crates/studio-domain/src/import.rs` | `push_semantic_int` skips an encoded field (reason recorded) when no contract is resolved instead of lowering with offset 0 |
| `crates/studio-domain/src/assets.rs` | player-asset slug uses the contract's birthdate index (probe only as the unresolved fallback); probe docs marked diagnostic |
| `crates/studio-domain/src/lib.rs` | module + exports |
| `crates/studio-domain/tests/encoding_contract.rs` (**new**) | family + contract per anchor (27) with the diagnostics agreeing; bulk weight edits across the old threshold (PS3 15, X360 09, PS4 17) — display, edit form, re-applied human value, unrelated cell, undo, redo, save → reopen; all-zero weights; bulk rating edits (X360 14, PS3 12 lowered; X360 09 raised); title / schema conflict refusal in words with bytes and history intact; the import seam's offset; the real NHL 20 witness (`--ignored`, private fixture, hashes pinned) |
| `crates/studio-domain/tests/contract_scale_probe.rs` | candidate table: 52 public cap hits × 27 anchors against `contractdollars` **and** `salary`; ≥ 2 independent matches asserted per witnessed family |

No DTO, binding or frontend change.

## 3. Evidence

- [`f1-encoding-contract-red.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f1-encoding-contract-red.log)
  — the new cases against the population probes: bulk 170 lb on PS3 NHL 15 → base 150 ("the base
  moved with the population"); all-zero weights on X360 NHL 09 → 120; PS3 NHL 12 ratings lowered →
  +50; the import seam → 150; no refusal under a wrong title.
- [`f1-encoding-contract-green.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f1-encoding-contract-green.log)
  — 6 passed after the contract; [`f1-encoding-contract-families.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f1-encoding-contract-families.log)
  — the 27-row family / contract / evidence table, diagnostics "agree" on every anchor.
- [`f2-contract-witness-candidates.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f2-contract-witness-candidates.log)
  — every rostered candidate per anchor with raw → dollars and the match verdict; matched
  independent players per family: NHL 07 **6**, NHL 08 **7**, NHL 09 **8**, NHL 10 **11**,
  NHL 11–Legacy **25**, PS4 **24**. `ubPc` carries no `salary` column on any title (the 13 / 14-bit
  `salary` fields of the metadata live in other tables); `contractdollars` is the AAV column.
- [`f2-real-nhl20-encoding.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f2-real-nhl20-encoding.log)
  — real NHL 20 (`1e200478…` / meta `2fac0e73…`): family PS4, contract 120 / 0 / ×$5 000, no
  conflict as `nhl20`, diagnostics empty, McDavid $12,500,000.
- [`f1-domain-focused-suites.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f1-domain-focused-suites.log),
  [`f1-api-sweeps.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f1-api-sweeps.log),
  [`f1-domain-lib.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f1-domain-lib.log).

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-domain --test encoding_contract` | **6 passed**, 1 ignored (private fixture) |
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test encoding_contract -- --ignored` | **1 passed** (real NHL 20) |
| `cargo test -p studio-domain --test contract_scale_probe` | 3 passed |
| `cargo test -p studio-domain --test weight_base --test rating_scale_probe --test view --test nhl07_encodings --test birthdate_encoding --test import_lower --test import_lower_sweep --test ratings_write_codec --test roster_overall_column --test field_profile --test nhl10_meta_smoke --test capabilities` | 58 passed |
| `cargo test -p studio-domain --lib -- --skip zz_diag` | **222 passed, 0 failed, 2 ignored** |
| `cargo test -p studio-api --test player_identity --test player_ratings --test player_containment --test roster_full_refusal` | 6 passed (27-anchor sweeps) |
| `rustfmt` on the two new files | applied |

Repo-wide fmt / clippy remain the recorded C0.2 baseline defects; not re-measured. Bindings unchanged.

## 5. Contract table (what the code now asserts, with its evidence)

| Family (schema shape) | Weight base | Six-bit ratings | Birth m / d | Cap-hit unit |
|---|---|---|---|---|
| NHL 07 — `yeardrafted` 6-bit | 150 (in game) | +50 (in game) | 1-indexed (measured) | ×$25 000 (in game; 6 witnesses) |
| NHL 08 — `yeardrafted` 7-bit | 150 (measured) | +50 (**inferred**, provisional) | 1-indexed | ×$25 000 (**measured**, 7 witnesses — was unwitnessed) |
| NHL 09 — 8-bit, `helmetbrand` 4-bit | 150 | +50 (inferred) | 1-indexed | ×$25 000 (8) |
| NHL 10 — 8-bit, `helmetbrand` 8-bit | 150 | +50 (inferred) | 1-indexed | ×$25 000 (11) |
| NHL 11 – Legacy — `draftyear` | 120 | +36 (in game 14 / 15 / Legacy) | 0-indexed | ×$5 000 (25) |
| PS4 — little-endian | 120 | stored 0..99 | 0-indexed | ×$5 000 (24; real NHL 20 included) |

Legacy Recompiled follows the Legacy row by identity (no independent anchor). The NHL 08–10 "+50"
rating reading remains a distribution inference; a console reading of one known rating would
close it (card open since receipt 13).

## 6. Files saved by earlier builds — what could be wrong, how to tell, what not to do

Attribution is by code history, not by a build number: no release exists and the working tree of
25 Sep is uncommitted, so "build" below means a checkout of the named commits.

| Period | Weight | Six-bit ratings | Cap hit |
|---|---|---|---|
| commits before `0c1f410` (26 Jun – 20 Jul 2026) | fixed +120 everywhere → NHL 10 (the only pre-11 title then open) displayed 30 lb light, a staged weight written 30 lb **heavy** | fixed +36 → NHL 10 displayed 14 low, written 14 **high** | raw, display only |
| `0c1f410` (20 Jul) → HEAD `8d60a6d` (16 Sep) and the working tree until package 11 (25 Sep 05:50) | base chosen by the rating probe: **PS4** displayed 30 lb heavy, written 30 lb **light**; **NHL 08** displayed 30 lb light, written 30 lb **heavy**; NHL 07 / 09 / 10 / 11–Legacy correct | until package 13 (06:45): **NHL 08** displayed 14 low, written 14 **high** (a typed 86+ overflowed the 50..99 scale) | display only; NHL 08–10 raw |
| packages 11 / 13 → this package | content-probed: correct on stock rosters; movable by a bulk edit (F1) — no evidence any file was affected | same | display only |

Operations that wrote through the offset: the Bio weight cell (edit form), any Attributes-tab
rating cell, the roster-builder ratings import and the historical import's weight / rating
lowering (`push_semantic_int`), all through `FieldView.edit_offset`. The cap hit was never written
through a unit.

Detection: (1) `content_diagnostics` (exposed in the domain; not yet on a UI surface) reports a
whole population that looks unlike the title's stock encoding — it catches a bulk shift, not a
single cell; (2) against the title's committed vanilla anchor, a weight cell that differs from
vanilla by exactly ±30, or a rating cell by ±14, is a **candidate** for a mis-offset write — not
proof: an intended edit of that size is indistinguishable, and a created or imported player has
no vanilla counterpart at all. Recovery: reopen the file with this build and re-enter the intended
human value; the write now goes through the contract. **Nothing is repaired automatically and no
edit history is inferred from a plausible value** — the diagnostic and the anchor diff are the
user's evidence, the re-entry is the user's decision.

## 7. Anchors and independence (F2)

The sweeps enumerate **27 anchor directories**, not 27 independently verified titles:
`anchors/ps4/nhl20` is an NHL 19 base copy (memory / R88); the real NHL 20 witness is the private
fixture case above (`databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db`, SHA-256
`1e2004784bccfdd6bed6e82533a67595798379f2c2d763c0636d9d370977a57b`; meta `2fac0e73…`); NHL 07 PS3
is a research copy; Legacy Recompiled has no independent anchor. Receipt 17's "28 public cap hits"
were 3 players × anchors; the candidate table replaces that with 6–25 distinct players per family.

## 8. Observations (for research / the coordinator)

- NHL 08 stores Crosby's `contractdollars` as 280 (×$25 000 = $7 000 000); his 2007-08 cap hit was
  $850 000 and his extension $8 700 000 — EA's figure matches neither; the receipt-17 explanation is
  withdrawn, the row stays in the table as "differs".
- NHL 15 / PS4 15 store Kane and Toews at 2 100 ($10 500 000), their 2015-16 figure, a season early;
  NHL 15 stores Karlsson twice (1 300 and 132) and Subban at 575 — rows that differ are listed, not
  explained.
- PS4 18 carries duplicate rows at 0 for nine of the candidates (template / second rows); PS4 21
  stores Perry at 300 ($1 500 000, his Montréal deal).
- `attributes::is_gen6_style_schema` (3-bit style field) is true on NHL 07 **and** NHL 08; it
  cannot name NHL 07 alone (the classifier uses the draft-year width instead).
- Not moved to the contract: `dXSB` main-team indexing (probe `probe_main_team_offset`-style logic in
  its own module) and `birth_year_base` (width-keyed, schema-derived already).

## 9. Cleanup

Scratchpad: `patch_encoding.py`, `patch_encoding_stage1.py` deleted by literal path at the end of
this package; `nhlms-encoding-*` temp roots removed on drop (verified absent); no processes.

## 10. Next action

The audit's sequence: **title / source-scoped equipment resolution** across the stick / glove /
helmet surfaces (scope on the enum definitions, resolved with the project's title; unknown ids stay
raw and round-trip; a label never authorises a write), then the completion-ledger refresh against
R88–R95 and the corrected blocker map, then the League / Team queue.
