# S1 — `chunkzip` multi-chunk decoding: reproduction, correction, regressions

> **Package:** S1 of the [parallel completion plan](../plans/parallel-completion-plan-2026-09-16.md)
> (§3, "reproducible baseline and concrete codec defect"). **Date:** 2026-09-16.
> **Author role:** developer. Reviewer: Codex (architect). **Branch:** `feat/phase5-asset-tooling`
> at `8d60a6d` (C0-AS7-CLOSE) with the pre-existing dirty working tree preserved.
> **Nothing staged, committed or pushed.** No registry schema, no LZX, no archive write or
> installation, no live-game change, no evidence-infrastructure activation, no QA generator run,
> no `capability.csv` edit, no receipt data record added or changed.

---

## 1. Result in one paragraph

The 257 `patch.big` failures recorded by C0-AS7 (§A.3) are one defect, already understood on
PS4 and repaired **uncommitted** in the working tree before this package: the committed padded-chunk
walker stepped `off += 8` to reach the next chunk header and gave up after two steps, which cannot
work when a chunk's compressed length is not a multiple of 8. On the PS3 NHL Legacy `patch.big`
every residue mod 8 occurs, so every multi-chunk member whose first chunk happened to end off the
8-byte grid failed with the recorded `TruncatedHeader` (`needed N, have M` with `N < M` — the
stepping loop's own message, not a real bounds failure). The corrected arithmetic
(`align_up_16(off − 8) + 8`) decodes all 329 `chunkzip` members of `patch.big` to exactly their
declared `fullsize`, the 72 members that decoded before give identical digests, and an independent
Python walk of the same bytes agrees chunk for chunk (1 817 chunks). Checking the same walker
against five small PS4 **NHL 21** (`CUSA18182`) archives (5 420 `chunkzip` members) exposed **a second framing
variant the codec ignored**: the per-chunk word after `zsize` is a coding flag — `1` = raw deflate
(every other observed chunk), `4` = **stored** — and two `overlays_21.big` members end in a stored
chunk. That is now decoded, byte-identical to the externally extracted files. The walker also no
longer returns a silently truncated file when the chunks stop short of `fullsize`. Eleven synthetic
regressions pin the measured layouts and the malformed cases. `chunklzx`, the `chunkzip` *writer*,
and archive installation are untouched and stay open.

## 2. Inputs (bounded; no repository-wide audit)

| Input | Where | Role |
|---|---|---|
| C0-AS7 receipt §A.3, §F.5 | [c0-as7-asset-evidence-measurements.md](c0-as7-asset-evidence-measurements.md) | the recorded defect: 257 / 329 fail, all `dummy = 2`, `n_chunks ≥ 2` |
| C0-AS7 member record | `docs/receipts/data/c0-as7-ps3-patch-members.{csv,json}` | per-member header words, stored digests and the exact committed-codec error text — the baseline reproduction |
| Specimen | `H:\Emulators\RPCS3\dev_hdd0\game\BLUS31540\USRDIR\patch.big`, SHA-256 `d2d26273…9bbe0`, 60 596 976 bytes — re-hashed this package, identical to the AS7 record | the PS3 multi-chunk corpus |
| Second corpus (regression check only) | `H:\Emulators\ShadPS4\Games\CUSA18182\databigs\{boot,overlays_21,overlays_22,overlays_23,overlays_24}.big` and the `Extracted/` tree beside them (the C0-POR1 PS4 root) — **NHL 21**, per that copy's `param.sfo` (`TITLE NHL 21`, C0-POR1 §1.2) | the walker comment cites PS4 **NHL 19** `game_90-008.big` / `patch_game_9x.big` from a separately attributed, older copy that has not been rerun; this copy does not contain them, and these five are the small `EB\0\3` archives it does |
| Tool | `crates/tdb-archive-big/examples/as7_eb_members.rs` (committed, unchanged) | per-member decode outcome and digests; all runs written to the session scratchpad, **not** to `docs/receipts/data` |
| Codec | `crates/tdb-archive-codec/src/lib.rs` | the only source file changed |

**Baseline state of the codec, stated precisely.** HEAD carries the stepping walker (the one AS7
measured). The working tree already carried, uncommitted, the `align_up_16` rewrite plus three
regressions (`padded_chunkzip_decodes_chunks_with_unaligned_compressed_lengths`,
`prior_stepping_alignment_cannot_reach_unaligned_chunk_headers`,
`padded_chunkzip_rejects_truncation_around_chunk_alignment`), motivated by PS4 NHL 19 and never
run against the PS3 specimen. This package verified that rewrite on the AS7 specimen, then extended
the codec for the second variant and the short-output case. The working-tree diff of this file
against HEAD therefore contains both the earlier uncommitted rewrite and this package's changes;
§4 lists which is which.

**Relevant baseline failures.** `cargo fmt --check` / `clippy -D warnings` fail repo-wide at HEAD
(C0.2, open) — this file is now `rustfmt`-clean and both `tdb-archive-codec` and `tdb-archive-big`
pass `clippy --all-targets -D warnings`. `eb::tests::chunklzx_decodes_against_extracted_oracle` is
`#[ignore]`d (LZX framing not pinned). `eb::tests::writer_round_trips_real_data0_big` reads a
mounted absolute path and reports `ok` — a recorded C0.2 silent-skip pattern, not touched here.

## 3. Reproduction and measurement

### 3.1 The recorded failure, explained

Every failing row in the AS7 record has `dummy=2; w16=131072; w20≥2; w24=16` and the error
`chunkzip header is truncated: needed N bytes, have M`. In 238 of the 257 rows `N < M`, which no
bounds check can produce: it is the stepping loop's `steps > 2` exit, reporting `off + 4` as
"needed" after failing to land on `off % 16 == 8` (the other 19 stepped off the end of the member
first). A replay of the removed loop over the stored bytes gets stuck on all 257. The 16
multi-chunk members that *did* decode at HEAD are exactly the ones in which **every** chunk's
`zsize` is `≡ 0 (mod 8)`, so the stepping loop happened to land.

### 3.2 Independent characterisation of the PS3 framing

A Python walk (zlib, no project code) over the 329 `chunkzip` members of `patch.big`:

| Property | Measured |
|---|---|
| header words | `dummy = 2`, `chunk_size = 131072`, `w24 = 16`, bytes 28..40 zero — all 329 |
| `n_chunks` histogram | 1: 56 · 2: 54 · 3: 83 · 5: 18 · 6: 59 · 11: 39 · 22: 7 · 29: 13 |
| chunk header offset | `≡ 8 (mod 16)` — all 1 817 chunks |
| `zsize mod 8` | every residue 0–7 present (151–330 chunks each) |
| inter-chunk padding | every width 0–15 present (77–111 chunks each); **all padding bytes zero** |
| flag word | `1` — all 1 817 chunks |
| deflate framing | raw deflate — all 1 817 (zlib fallback never needed) |
| middle chunks | inflate to exactly `chunk_size`; last chunk to exactly the remainder — all 329 |
| bytes after the last chunk | 0 — all 329 (stored `size` ends at the last chunk's end) |

### 3.3 Specimen checks with the working-tree tool

| Archive | `chunkzip` members | HEAD codec (AS7 record) | before this package's changes | after |
|---|---|---|---|---|
| PS3 `patch.big` | 329 | 72 ok · **257 fail** | 329 ok | 329 ok |
| PS4 NHL 21 `boot.big` (SHA-256 `1b794ece…`) | 4 952 | — | 4 952 ok | 4 952 ok |
| PS4 `overlays_21.big` (`cdcf3889…`) | 254 | — | 252 ok · **2 fail** (`corrupt deflate stream` on chunk 1) | 254 ok |
| PS4 `overlays_22.big` (`0ccd4bb9…`) | 90 | — | 90 ok | 90 ok |
| PS4 `overlays_23.big` (`6a119596…`) | 89 | — | 89 ok | 89 ok |
| PS4 `overlays_24.big` (`098e3742…`) | 35 | — | 35 ok | 35 ok |

Cross-checks on every run: stored bytes and paths identical to the AS7 record (459/459 rows);
decoded size `== fullsize` for every decoded member; the 72 members decodable at HEAD keep their
AS7 digests; every previously decodable PS4 member keeps its digest between the two runs.

### 3.4 The second variant

`overlays_21.big` members 5 and 246 (`nhl/ui/swfassets/kettle/overlays/guestinthebooth/
guestinthebooth{arrived,teaser}.gfx`): two chunks each, chunk 0 flag `1` inflates to 131 072,
chunk 1 carries **flag `4`** with `zsize` equal to the remaining plain size (6 839 and 35 120) and
bytes that are not a deflate stream. Treating a flag-`4` chunk as stored yields exactly `fullsize`
bytes, and both results are **byte-identical to the corresponding files in the `Extracted/` tree**
(SHA-256 `f516e40e4c5fe70c…` and `a19e9e61c718fbd9…`). Across all six archives the flag word is
`1` on 8 195 chunks and `4` on these two; no other value was observed.

## 4. Changes — `crates/tdb-archive-codec/src/lib.rs` only

Already in the working tree before S1 (verified here, kept):

- padded walker: `off = align_up_16(off − 8) + 8` replaces the capped `off += 8` loop;
  `align_up_16` helper; three regressions listed in §2.

Added by S1:

| Change | Detail |
|---|---|
| per-chunk flag | the word after `zsize` is read as the coding flag. `FLAG_STORED = 4` copies `zsize` bytes verbatim, refused with `StoredChunkTooLong` if they exceed what `fullsize` still owes; every other value takes the existing raw-deflate → zlib path, so nothing that decoded before stops decoding. The legacy (`dummy > 2`) header has no flag and passes `FLAG_DEFLATE` |
| short output is an error | `finish_output` requires `out.len() == fullsize` in both walkers; `ShortOutput { declared, produced, n_chunks }` replaces the silent `truncate(fullsize)` return of a partial file |
| `DecompressFailed` names the flag | new `flag: u32` field, shown as `(flag N)` in the message so an unobserved variant is recorded rather than lost |
| module docs | the flag semantics and the corpora they were measured on; the exact-size rule |
| tests (8 new, 38 total in the crate) | `padded_chunkzip_covers_every_inter_chunk_gap_width` (17 stored-block chunks; asserts gaps `{0..=15}` are all reached, then decodes), `padded_chunkzip_decodes_stored_chunks` (the observed deflate-then-stored shape at the real 131 072 / 6 839 split; stored-first; stored-only), `stored_chunk_longer_than_the_declared_remainder_is_rejected`, `short_output_is_rejected_not_truncated` (too few chunks; last chunk short; legacy header), `chunk_inflating_past_fullsize_is_rejected`, `unknown_flag_is_tried_as_deflate_and_named_on_failure`, `malformed_padded_framing_fails_fast` (`n_chunks = u32::MAX`, `zsize = u32::MAX`, zero chunks with non-zero size, zero/zero, a cut inside the flag word), `padded_header_dummy_0_1_2_share_the_walker` |

`compress_chunkzip` (the writer) is unchanged. It still writes `0` in the flag position; the reader
treats `0` as deflate, so its round trip holds — but every observed deflate chunk carries `1`.
See §7.

## 5. Checks run

| Check | Result |
|---|---|
| `cargo test -p tdb-archive-codec` | 38 passed, 0 failed (30 before S1) |
| `cargo test -p tdb-archive-big` | 28 passed, 1 ignored (`chunklzx` oracle) — unchanged |
| `cargo test -p studio-domain --lib` | 148 passed, 1 ignored — unchanged |
| `cargo test -p tdb-rx2`, `-p tdb-canonical`, `-p studio-domain --tests` | see §5.1 |
| `rustfmt --check` on the changed file | clean |
| `cargo clippy -p tdb-archive-codec -p tdb-archive-big --all-targets -- -D warnings` | clean |
| specimen runs (§3.3) | 6 archives, 5 749 `chunkzip` members, 0 decode errors; oracle match on the 2 stored-chunk files |

Missing fixtures, stated as such: the PS4 archives named in the walker's comment
(`game_90-008.big`, `patch_game_91.big`, `patch_game_93.big`) are not in this copy — the NHL 19
alignment claim rests on the earlier session's note (its own specimens, not rerun); this package's
five-archive check is on the NHL 21 copy and is a separate observation. The Xbox 360 Legacy archives hold `chunklzx`, not `chunkzip`, so they say nothing
about this codec. No private-fixture (`NHL_STUDIO_FIXTURES`) case exists for `chunkzip`; the
specimen checks above were run by hand with the committed tool and are not automated.

### 5.1 Longer suites

| Suite | Result |
|---|---|
| `cargo test -p tdb-rx2` | 109 passed, **26 ignored** — the `NHL_STUDIO_FIXTURES` cases; not run (no root set), i.e. missing-fixture skips, not passes |
| `cargo test -p tdb-canonical` | 552 passed (the four AS7 specimen-record cases included: the committed records are untouched, so the re-derived joins still match) |
| `cargo test -p studio-domain --tests` | all targets pass: lib 148 passed / 1 ignored; 19 integration binaries, 0 failures, 5 of them wholly `#[ignore]`d (mounted-data cases, skipped — not passes). No new regression; nothing here reads the codec beyond the two `assets.rs` cases, which passed |

## 6. What remains (codec)

1. **`chunklzx`** — unchanged, still refused by the public path; the LZX bitstream framing is not
   pinned (`lzx.rs`, ignored oracle test). Every Xbox 360 Legacy in-archive rendering payload is
   this form.
2. **Flag values other than `1` and `4`** — none observed on 8 197 chunks; the codec tries deflate
   and reports the flag on failure. No further action until one appears.
3. **`dummy` 0 and 1** — accepted by the same walker (a test pins it) but never observed on a real
   member: all 5 749 carry `2`. Whether they exist, and on which titles, is unknown.
4. **Legacy header (`dummy > 2`)** — synthetic coverage only; no real member of that shape has been
   measured in this repository.
5. **Writer flag word** (§4, §7).

## 7. Questions for the architect

- **Q-S1.1** Should `compress_chunkzip` write flag `1` (the value on every observed deflate chunk)
  instead of `0`? It is a one-word change on the write path, which S1 excludes; a reader that
  honours the flag (this one) accepts either, and whether the *game's* loader does is unmeasured.
- **Q-S1.2** Should the two stored-chunk `.gfx` members and a handful of PS3 multi-chunk members be
  cut out (stored bytes only, a few hundred KB) into `NHL_STUDIO_FIXTURES` with an `#[ignore]`d
  case under the C0-FX1 mapping, so §3.3 becomes a rerunnable check instead of a hand run?

For the research agent, none. Nothing here needed a game fact that was missing.

## 8. Suggested next bounded package

Toward the shared archive transaction and Texture Lab (S2): **the C3/C3a change-set/apply boundary
for one `EB\0\3` archive**, before any UI —

- a planner over `eb::EbWriter` that takes `{entry index → new stored bytes}` and reports, per
  entry, *fits in place* vs *relocates* (growth), with the reflowed table and the exact byte
  ranges it will touch;
- apply = write to a staged copy, verify by re-parsing and re-decoding every touched entry
  (`decode_payload` now being trustworthy for every observed `chunkzip` shape), then swap;
  recovery = the untouched original plus a journal of the ranges;
- failure tests: truncated write mid-payload, table/payload disagreement, a decode that fails after
  write (the codec's new `ShortOutput` / `StoredChunkTooLong` are the signals), power-loss
  simulation by cutting the staged copy;
- the PS3 `patch.big` (now fully decodable, 60 MB) as the first attributable specimen, with the
  `patch.big` precedence question (AS7 §F.4) carried as an explicit unknown rather than decided.

Texture selection → preview → export → import then sits on that boundary instead of on a
one-off write route.

## 9. Handoff summary

- **Fixed:** multi-chunk padded `chunkzip` (the AS7 defect — 257 → 0 failures on the recorded
  specimen); stored (`flag 4`) chunks; silent short output.
- **Not fixed, by scope:** `chunklzx`; the writer's flag word; archive installation; anything
  runtime.
- **Files changed:** `crates/tdb-archive-codec/src/lib.rs`; this receipt. Nothing under
  `docs/receipts/data`, `defs/`, `app/`.
- **Blockers:** none for S1. For S2, none codec-side for `chunkzip`; `chunklzx` remains the block
  for any Xbox 360 rendering payload.

---

## 10. Closure after Codex review (2026-09-16, [s1-review-and-s2a-dispatch](../validation/s1-review-and-s2a-dispatch.md))

Attribution corrected above and in the codec comments: the `CUSA18182` `boot.big` / `overlays_2x.big`
corpus is **NHL 21** (its `param.sfo`, C0-POR1 §1.2), not NHL 19. The NHL 19 `jack_eichel.mcd`
alignment note in the walker stays as the separately attributed origin of the `align_up_16` rewrite,
marked as an unrerun copy. Neither statement infers complete build attribution or pristine status.

| Item | Done |
|---|---|
| Stale codec docs | encoder existence (unqualified for game writes) replaces "write path unimplemented"; the padded word is documented as the chunk flag, not a gap, in the chunkzip paths (the `chunklzx` walker's comment now says it reads past a flag it does not interpret — LZX itself untouched); the `DecompressFailed.flag` comment no longer claims the legacy helper reports `0` (it supplies `FLAG_DEFLATE`) |
| Encoding damage | 17 mojibake runs in `lib.rs` (em dashes and one arrow, from the S1 session's cp1252 round trip) restored to UTF-8; nothing else in the file reformatted beyond `rustfmt` on the touched file |
| **Q-S1.1 encoder flag** | `compress_chunkzip` writes `FLAG_DEFLATE` (`1`) at `0x2C`. `compress_chunkzip_header_bytes_and_independent_decode` asserts every header field at its offset (magic, `dummy 2`, `fullsize`, `chunk_size`, `n_chunks 1`, word `16`, 12 reserved zeros, `zsize`, flag `1`, payload ending the buffer) and inflates the chunk **through `flate2` directly**, then through the decoder, for empty / short / 70 000-byte noise / 200 000-byte runs. `historical_flag_zero_encoder_output_still_decodes` rewrites the flag to `0` and decodes, and decodes a flag-`0` hand-built payload. Not qualified by this: native loader acceptance, `chunk_size = fullsize` sizing, empty-input handling by the game, or the 128 KiB chunking real members use. Unchanged native payloads are never recompressed |
| **Q-S1.2 fixture set** | `crates/tdb-archive-codec/src/fixtures.rs` (`#[cfg(test)]`): six recorded members — four PS3 `patch.big` multi-chunk members with distinct first-chunk `zsize mod 8` residues (5, 2, 6, 4) and chunk counts (3, 2, 6, 11), and the two NHL 21 stored-tail `.gfx` members — each with archive path + SHA-256, member index/path/offset, stored size + SHA-256, expected decoded size + SHA-256 and the oracle (PS3: independent zlib walk of the same bytes, no external file agrees; PS4: the extracted file, byte-identical). A selected case loads the bytes, **refuses bytes whose digest is not the recorded one**, decodes and compares. Root/key resolution (`unset`, empty, missing root, root-is-a-file, missing file, empty file, wrong bytes, directory-in-place) is covered by 5 self-contained cases. `sha2 0.10` (already in the lock) added as a **dev-dependency** of the codec crate |
| Fixture execution | root provisioned **outside the repository** at `E:\tmp\nhl-studio-fixtures` (six files, 318 749 bytes, cut from the two archives by offset/size and verified against the recorded stored digests); run with the variable passed to that one command, never exported: `NHL_STUDIO_FIXTURES=E:/tmp/nhl-studio-fixtures cargo test -p tdb-archive-codec -- --ignored` → **6 passed, 0 failed**. Negative check: `NHL_STUDIO_FIXTURES=E:/tmp/does-not-exist … --ignored <one case>` → **fails** naming the variable, key and root. Default suite: 45 passed, 6 ignored |

Codex's independent run (38 codec / 28 archive) predates the encoder and fixture additions; the
codec file's SHA-256 in the review record is therefore stale.
