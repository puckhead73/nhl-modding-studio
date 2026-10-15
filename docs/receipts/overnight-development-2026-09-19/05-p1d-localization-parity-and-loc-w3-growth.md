# Receipt 05 — D5 / P1-D localization parity and LOC-W3 growth (preserving serializer)

Developer lane, overnight 2026-09-19. **Developer-verified; not Codex-accepted.**

## Plain-language summary

The dictionary tools now cover every title, platform and locale Studio has a file for: the two existing
operations (point a key at another key's text; re-write a text in place) are exercised on all 25 committed
English dictionaries and, with the private fixtures, on all 100 loose locale dictionaries of the Xbox 360
installs and PS3 discs plus the six install-bound English files — each with its own compression tree, the
PS3 files as themselves. On top of that Studio can now **grow** a dictionary: give an existing key a longer
text, add a new key at its sorted place, or set a text to empty, by appending to the file and leaving
everything that exists exactly where it is. Titles whose rows are not in hash order (NHL 08/09/10 on Xbox,
NHL 09/10 on PS3, the `lng_lg` template) refuse the new-key case and say why. A grown dictionary flows
through the pending review, the in-place save and the copied-target save (raw or compressed archive
member), and restores byte-exactly. Not built: rebuilding the compression tree for a symbol it lacks
(refused, symbols named). Whether a game loads a larger dictionary or finds a new key is a user test.

## 1. Package, disposition, scope

- **D5 / P1-D + LOC-W3 rev. 3 (P serializer)** — delivered, developer-verified, pending independent
  review. Bounded checkpoints: (1) per-title/platform/locale LOC-W1/W2 coverage; (2) LOC-W3 preserving
  growth (O-T, O-K class A, O-N) with Studio integration; (3) **canonical rebuild / new glyph (O-G,
  serializer C, proofs C1/C2): not built** — the P path refuses `needs-rebuild` with the missing symbols.

## 2. Changed paths and hashes (baseline `8d60a6d`; prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-loc/src/growth.rs` | `1b507dfc78cfb490…` | **new** — `GrowthOp`, `OrderClass`, `plan_growth`, `apply_growth` (§4-P), `GrowthRefusal` kinds |
| `crates/tdb-loc/src/lib.rs` / `Cargo.toml` | `8f88749b…` / `20a24fd3…` | module + exports; dev-dep `serde_json` |
| `crates/tdb-loc/tests/growth.rs` | `cd4b387481580733…` | **new** — 9 cases (T1a, T2/T3/T6/T9/T10 accounted on LE + BE, T4 incl. the shipped pair, T5 class B, T7 sharer semantics, T8-P, T9 no-op, T12 capacity + verify refusal) |
| `crates/tdb-loc/tests/locale_coverage.rs` | `8d5c33ded45a9a45…` | **new** — 1 default sweep (25 anchors) + 2 `--ignored` sweeps (100 locale files, 6 install dictionaries) |
| `crates/studio-domain/src/dictionary_edit.rs` | `5521af1cab4ff1db…` | `DictionaryPlan::Growth`, `DictionaryGrowthPreview`, `preview_growth`, `apply_growth`, warnings, transaction detail |
| `crates/studio-domain/src/workspace.rs` | `db30600d19d35bee…` | `dictionary_growth_preview`, `set_dictionary_growth` (consumer notes appended) |
| `crates/studio-domain/src/lib.rs` | `ae15c9b1c2200ca1…` | re-exports `tdb_archive_codec`, `tdb_loc` |
| `crates/studio-api/src/registry/copied_target.rs` | `eb8d7cead2fc4c64…` | archive-member route: a **chunkzip** member takes the artifact re-encoded natively (decode-checked); raw unchanged; chunklzx refused |
| `crates/studio-api/src/{dto,convert,registry,commands,lib,bindings}.rs` | `0eaa5507…` / `69f8d94d…` / `ca00f76e…` / `f839216c…` / `31ca4310…` / `35e4850e…` | `DictionaryGrowthOpDto`, `DictionaryGrowthPreviewDto` (+ insert/edit DTOs); `dictionary_growth_preview` / `dictionary_growth_set`; export roots 111 → 113 |
| `crates/studio-api/tests/dictionary_growth.rs` | `fc04c6210a32cc7f…` | **new** — 3 route cases |
| `app/src/bindings/*` | generated | `--write`; `--check` clean |
| `app/src/lib/client.ts` | `4827276e641c4dbb…` | command typings |
| `app/src/components/DictionaryRenamePanel.tsx` (+test) | `86c51b33…` / `058c8a56…` | modes "free text (append, file grows)" and "new key"; growth preview block; +2 jsdom cases |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | `3740243b105372cc…` | 1.3 rows: LOC-W2 coverage, growth, locales |
| `local/runs/overnight-development-2026-09-19/extract_locale_fixtures.py` | `fd58873e4d9db47b…` | fixture extraction (read-only on the installs/discs) |

Private fixtures added (read-only inputs): `E:\nhl-studio-fixtures\loc\<platform>-<title>\*.db` — **100
loose locale dictionaries** (53 Xbox 360: 07 1, 08 1, 09 7, 10 2, 11 2, 12–Legacy 8 each incl. `lng_lg`;
47 PS3: 08 1, 09 2, 10 2, 11 2, 12–Legacy 8 each) with `loc\manifest.json` (source path, size, SHA-256;
`9705e2161b1e4fdc…`), 126 MB.

## 3. Behavior and decisions

- **Coverage (P1-D).** `locale_coverage::exercise` runs, per file: `verify` + `serialize` round trip,
  LOC-W1 re-point, LOC-W2 re-text with a strict prefix of an existing text (always fits, always in the
  tree) with every other row asserted identical, LOC-W3 P no-op identity, text growth (the text doubled)
  and — on class-A files whose tree encodes the key — a new key at its sorted position; class-B files
  get the `baseline-order-anomalous` refusal for insertion. Order classes match research 20's census;
  one census fact is now exercised in code: the **PS3 NHL 08 disc dictionary is class A** (0 descents —
  insertion offered) although the committed "ps3 nhl08" anchor (an Xbox file by identity) is class B.
- **LOC-W3 P.** Exactly §4-P: `k` inserted 16-byte records at `count(hash < new)` (+ inserts before),
  re-pointed 4-byte slots, the original pool copied verbatim (tree, strings, unreachable storage, slack)
  shifted by `16k`, new strings appended after the physical pool end (`[key][text]` gen7 / `[text][key]`
  PS4; edits' texts after the inserts), `db_size` / `max_records` / `current_records` patched, the four
  CRC scopes recomputed. Independent verification inside `apply_growth`: re-parse, `verify`, decoded rows
  equal the requested result (input rows with the plan's edits, inserts at their positions), order class
  preserved (class A stays A; class B keeps its exact descent set), pool bytes verbatim. Tests derive
  the expected layout on their own (`account`) and check the serializer's `changed_ranges` claims
  against it.
- **Refusals.** `exists`, `hash-collision` (full multiset incl. the shipped `TXT_STATS_RANK` pair and two
  new keys with one hash), `baseline-order-anomalous` (descent rows listed), `capacity` (100 B / 4 000 B /
  16-bit record limit), `needs-rebuild` (symbols listed), `not-found`, `nothing-to-change`, `invalid`
  (NUL, non-16-byte records, spare slots), `verify` (bad CRC) — nothing written on any of them.
- **Sharers.** O-T on a LOC-W1 baseline reports the sharers before mutation; "this row only" (default)
  appends a string for the row and leaves the sharers on the old one; "all sharers" re-points them all
  to one appended string and preserves the old string (unreachable) — T7 (a) and (b) from the same
  baseline. LOC-W2's zeroed slack is copied verbatim (T8-P).
- **Studio.** `DictionaryPlan::Growth { plan, ops }` re-plans from the ops on the attached bytes at
  packaging time (must equal the held plan), like the other two kinds; one pending dictionary operation
  at a time; the pending review shows `growth` with the first edited/inserted key and the byte count.
  Placement: the in-place save writes the longer loose file; the archive-member route appends and
  retargets a raw member (the old bytes stay in place, dead) and re-encodes a chunkzip member with the
  native encoder (decode-checked, native framing checked); the disc-image route is untouched (same-length
  only, so a grown dictionary is refused there by construction). After a committed copied-target save
  the grown dictionary is the baseline; a further growth plans on it.
- **UI.** Two new modes on the rename panel; the growth preview shows the edited/inserted rows, the
  order class and descents, appended bytes, sharers with the choice, and the warnings (runtime gates
  R-A/R-B, loader-order-compatibility unknown); a class-B refusal surfaces verbatim.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `cargo test -p tdb-loc` (default) | 7 + 5 + 9 + 1 + 4 passed; 8 ignored by name |
| `NHL_STUDIO_FIXTURES=… cargo test -p tdb-loc --test locale_coverage -- --include-ignored` | **3 passed** (25 anchors; 100 locale files, classes per census; 6 install files — PS3 NHL 08 class A, NHL 09 class B, PS4 A); 322 s |
| `cargo test -p tdb-loc --test growth` | 9 passed |
| `cargo test -p studio-api --test dictionary_growth` | **3 passed** (raw member append-and-retarget with the old bytes left in place, chunkzip member native re-encode + decode check, in-place loose save; class-B refusal through the route) |
| `cargo test -p studio-api --lib bindings` / `export-bindings --check` | 25 passed (roots 113) / in sync |
| `cd app && npx tsc --noEmit -p .` / `npx vitest run` | clean / **14 files, 108 passed** |

`cargo test -p studio-api --test dictionary_rename` (LOC-W1/W2 routes, unchanged): 5 passed. Not run: full workspace suite; `zz_diag`; clippy/fmt; desktop; any game.

## 5. Remaining blockers / limitations

- Canonical rebuild (O-G) and its proofs C1/C2 — not built; any text or key needing a symbol the tree
  lacks is refused with the symbols.
- Runtime gates R-A (larger file loads, appended string read through an existing key) and R-B (new key
  found) — user cards; glyph gate — user card.
- PS4 locale members other than `eng_us` are not extracted into fixtures (same code path; research
  unknown per title).
- Which copy the console loads (loose vs archive vs disc image) remains research unknown; the growth
  write refuses the same-length image route by construction.

## 6. Temporary artifacts / processes

- Session scratchpad `d5_growth.py`, `d5_api_ui.py`, `d5_panel.py` — disposable.
- Fixtures above — retained (regenerable by the extraction script).
- Test temp dirs `nhlms-locw3-*` — removed by `Drop`; no processes left.

## 7. Next

D6 — league asset parity (P1-E), consuming research 02 (packaging) and 16/18 (codecs), then G/H as
research readiness allows.
