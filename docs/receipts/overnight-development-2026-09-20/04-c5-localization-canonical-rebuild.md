# Receipt 04 — C5: localization canonical rebuild (LOC-W3 serializer C, O-G) — Priority 3

Developer lane, overnight 2026-09-20. **Developer-verified; not Codex-accepted.** Baseline `8d60a6d`.

## Plain-language summary

Until now a dictionary text that needed a character the game's compression tree did not have (say
`~`, or a letter the English tree never used) was refused outright. Studio can now **rebuild** the
dictionary: it repacks every string from the decoded rows in the game's own order and builds a new
compression tree that includes the missing characters. Nothing is reordered, empty texts stay empty,
strings that earlier edits made shared stay shared, and every difference from the original file is
accounted for line by line. The rebuild is never chosen for you: the default path still appends and
touches nothing that exists; when it refuses because of a missing character, the panel offers the
rebuild as an explicit switch, previews it with its own facts, and holds it as the same pending
dictionary change that the save writes and the archive routes place. Two things the rebuild cannot
prove: that the game loads a rebuilt file, and that its fonts draw the new character — those stay
user cards. Along the way a small fact about EA's files was measured that research had not stated:
every stored string carries one extra byte past its last full byte; with that rule Studio's repack of
every shipped dictionary (131 files) is byte-identical to the original apart from the trailing slack.

## 1. Package, disposition, scope

C5 — delivered, developer-verified. Contract: research handoff 20 rev. 3 §4-C (accepted 18 Sep).
Covers: new symbols (O-G) with the tree rebuilt; existing-key growth and new-key insertion under the
rebuild; shared pointers preserved as sharing (sharer choice honoured); null texts; duplicate-hash rows
carried (row identity = the hash multiset, unchanged policy); original *and* Studio-touched inputs
(LOC-W1 / LOC-W2 outputs); class-B files (rebuild allowed for text edits, insertion still refused);
PS4 non-English locales (**correction:** the 106-file sweep of §4 covers the 100 loose gen7 locale files
and the six install `eng_us` files only — the PS4 locale members were not in the fixture set; they were
extracted afterwards in C7 (99 members of 14 archives: seven locales per title, `lng_lg` on NHL 21,
copies in NHL 15's and NHL 19's patch archives) and swept separately — see receipt 06); native container
packaging via the existing dictionary artifact routes; UI integration. Separated, as required: (1) valid
serialization — proven; (2) packaging/reopen — proven offline; (3) new-key lookup in game — **not**
claimed; (4) font glyph availability — **not** claimed (stated on every rebuild preview).

## 2. Changed paths (SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-loc/src/rebuild.rs` | `504a4673731849a3` | **new** — `build_tree_bytes` (Huffman over the byte histogram; serialised in the measured pool format: 4 bytes per internal node, leaf marker 0 + symbol, internal marker = 1-based read position + 0, right child enqueued first), `plan_rebuild`, `apply_rebuild` (per-row pool order gen7 `[key][text]` / PS4 `[text][key]`, sharing by source, nulls, reserved record bytes and header/descriptors verbatim, CRCs recomputed, independent verification: rows = requested result, sharing relation, opaque metadata), `RebuildAccounting` (tree kept/rebuilt, discarded ranges classified trailing-slack / unreachable, moved strings, re-pointed records, `pool_prefix_identical`), `stream_bytes` |
| `crates/tdb-loc/src/growth.rs` | `a77248469ba918f5` | `PlanMode`, `plan_growth_with` (rebuild mode collects missing symbols instead of refusing), `RebuildInfo`, `GrowthPlan.rebuild`; `apply_growth` refuses rebuild plans; the P serializer's `encode` uses EA's `bits/8 + 1` stream length |
| `crates/tdb-loc/src/lib.rs` | `1f509bdc3919b181` | exports |
| `crates/tdb-loc/tests/rebuild.rs` | `99b80d2480b75aa3` | **new** — 7 default cases (T1b on the 25 anchors, T11, refusals, T8-C, T7-C, forced tree, tree builder) + 1 `--ignored` (106 private dictionaries) |
| `crates/studio-domain/src/dictionary_edit.rs` | `13230c7e5903e2b2` | `GrowthSerializer`, `preview_growth_with` / `apply_growth_with`, `DictionaryPlan::apply_plan` rebuild branch (re-plans and re-applies at save), `rebuild_changed_ranges`, preview fields `serializer` / `new_symbols` / `rebuild_accounting`, rebuild wording in the warnings (runtime + glyph gates) |
| `crates/studio-domain/src/workspace.rs` | `e23d5a1946b08aa1` | `dictionary_growth_preview_with` / `set_dictionary_growth_with` |
| `crates/studio-api/src/{dto,convert,registry,commands,bindings}.rs` | `8ba825d4…` / `2685927e…` / `3d0816f5…` / `ee3d6fc2…` / `8a1574e5…` | `DictionaryGrowthOptionsDto { serializer: preserving / rebuild / rebuild-tree }`, preview DTO fields; `dictionary_growth_preview_with` / `_set_with`; commands take `options`; export roots 116 → 117 |
| `app/src/bindings/*` | generated | `--write`; `--check` in sync |
| `app/src/lib/client.ts` | `a9484b80d3ca6ebb` | typings |
| `app/src/components/DictionaryRenamePanel.tsx` (+test) | `91323d14c3d12078` / `252323a43fb17feb` | "Rebuild the dictionary" switch (off by default; the held preview is dropped when it flips), the `needs-rebuild` offer, the rebuild facts card (symbols, accounting); +1 jsdom case |
| `crates/studio-api/tests/dictionary_growth.rs` | `dc572854be8fb51d` | +1 route case: refusal → explicit rebuild → preview → hold (pending listed) → in-place save → reopen → preserving growth on the rebuilt file → forced tree rebuild accepted; unknown serializer refused; the rebuilt dictionary placed into a raw archive member (append-and-retarget) and restored |

## 3. Behavior, decisions and measured facts

- **Tree format (measured, all anchors):** internal child marker = the child's 1-based position in the
  breadth-first read order (right child enqueued before left), value 0; leaf marker 0, value = symbol.
  A byte alphabet needs ≤ 255 internal nodes, so markers fit. The builder's output re-parses through
  the crate's own parser; the codes are prefix-free by construction.
- **EA's stream length is `bits / 8 + 1` bytes** — one byte past the last full byte, so a stream ending
  exactly on a byte boundary carries one extra zero byte. Research 20 B7 ("byte-aligned, contiguous,
  0-byte gaps") is correct *only* under this rule: with `ceil(bits/8)` packing, 1 in 8 strings left a
  1-byte zero gap (956 of 7 904 strings on NHL 07; 8 356 of 66 792 on PS4 NHL 18). With the rule, the
  no-op rebuild's repacked pool equals the input pool byte for byte up to the trailing slack on **all 25
  anchors and all 106 private dictionaries** (`pool_prefix_identical`), and the P serializer's appended
  strings are EA-shaped. Developer evidence; the research doc is not edited.
- **Never silent:** the preserving path still refuses `needs-rebuild`; the rebuild is an explicit
  serializer choice (`options.serializer = "rebuild"` / `"rebuild-tree"`), stated in the preview
  (`serializer`, `new_symbols`), the warnings (rebuild wording + glyph gate) and the pending card's detail
  (`serializer`, `new_symbols`, `tree_forced`). The tree is kept when the alphabet did not grow unless
  forced.
- **Save path:** `DictionaryPlan::apply_plan` re-plans on the attached bytes at save and refuses if the
  plan differs (as the preserving path does); the existing loose in-place save and both archive-member
  routes (raw member append-and-retarget; chunkzip re-encode) take the rebuilt bytes unchanged.
- **Class B:** insertion still refused (`baseline-order-anomalous`); text edits with new symbols rebuild
  and keep the descent set.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `cargo test -p tdb-loc` (all suites) | growth 9, rebuild **7**, retext 7, dictionaries 5, locale_coverage 1 (+2 ignored), private 0 (+6 ignored), lib 4 — all passed |
| `NHL_STUDIO_FIXTURES=… cargo test -p tdb-loc --test rebuild noop_rebuild_and_new_glyph -- --ignored` | **1 passed** (544 s): 106 files — no-op rebuild = input minus slack (379 slack bytes in total), new-glyph rebuild decodes on each (105 trees rebuilt; `xbox360-nhl09/nhl_ger_de.db` already had `~`) |
| `cargo test -p studio-api --test dictionary_growth` | **4 passed** (incl. the new rebuild route case) |
| `cargo test -p studio-api --lib bindings` / `export-bindings --write` then `--check` | 25 passed (roots 117) / in sync |
| `cd app && npx tsc --noEmit -p .` / `npx vitest run src/components/DictionaryRenamePanel.test.tsx` | clean / **8 passed** |

Not run: PendingEdits jsdom (no change there); desktop; any game.

## 5. Coverage after C5 (matrix 1.3 "tree rebuild (O-G, new glyphs)")

| X360 08–15/Legacy | X360 07 | PS3 08–15/Legacy | PS4 15–21 | Recomp |
|---|---|---|---|---|
| impl+tested (anchors + 53 loose locale files incl. `lng_lg`) | blocked (labels not in the dictionary — unchanged) | impl+tested (anchors + 47 loose locale files + the 08/09 disc fixtures) | impl+tested (anchors + NHL 19 base/patch, 20, 21 install `eng_us`; the 99 non-English / patch locale members: receipt 06) | impl+tested by identity with Xbox Legacy |

Runtime gates R-A / R-B, the rebuilt-pool loader gate and the glyph gate: user cards (§6 of the morning
review). "Which copy the console loads" (loose vs archive vs patch): research unknown, unchanged.

## 6. Limitations / open

- The rebuild's Huffman tree is a valid tree, not EA's tree-building algorithm; identity is not required
  (research 20 C2) and not claimed.
- `max_records` follows the P rule (`+ k`); shipped files have `max == current`, so the rebuilt file does
  too.
- The pending card shows the rebuild's detail; the copied-target review shows the dictionary artifact as
  before (its producer string does not distinguish P from C — small follow-up).

## 7. Temporary artifacts / processes

Scratchpad `c5_*.py` edit scripts — disposable. Probe tests `zz_tree_probe.rs` / `zz_gap_probe.rs` were
created in `crates/tdb-loc/tests/` for the two measurements and **removed** (verified absent). No temp
dirs, no processes.

## 8. Runnable commands

```sh
cargo test -p tdb-loc --test rebuild
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p tdb-loc --test rebuild -- --ignored --nocapture
cargo test -p studio-api --test dictionary_growth
cd app && npx vitest run src/components/DictionaryRenamePanel.test.tsx
```
