# Receipt 08 — Priority 3 / 2.5: the `chunklzx` store route (gen7 Xbox 360 `rendering/*` members become writable)

**Developer-verified; not Codex-accepted; nothing game-verified — and this route's loader
acceptance is explicitly unestablished (card R30-LZX-1).** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Real archives read only; writes on synthetic / temp copies. Claim:
`dev-p3-chunklzx-store-route`. Research consumed: 2026-09-21 handoff 06
(`research-r30-chunklzx-store`, `ready`) §1–3 and `r30-chunklzx-store-roundtrip.json`.

## 1. Plain-language summary

Receipt 04 made the LZX-wrapped Xbox 360 members (rink and centre-ice art, jersey / logo texture
libraries, pucks, attribute databases) readable but left them unwritable: nobody has an LZX
compressor. The research lane pointed out that LZX itself allows blocks that carry their bytes
uncompressed, and wrote an encoder that produces a valid `chunklzx` member from any file using
only such blocks. Studio now has that encoder natively: a replaced `chunklzx` member is written
back **in the same container the game expects**, just larger (the plaintext plus ~100 bytes
instead of a compressed few kilobytes). The replace preview says exactly that, with the size
growth and the open question: whether the console's LZX decoder accepts uncompressed blocks in
these members has **not** been run. Nothing is substituted by `chunkzip`; a BIGF-reader member
that used to be refused as "LZX not implemented" is now decoded.

## 2. What users can now do

- Replace a texture inside a `chunklzx`-stored member of the gen7 Xbox 360 EB archives
  (`cache.big`, `cacherender.big` / `rendercache.big`, `boot.big`) through the existing
  archive-member route: preview → pending changes → one copied-target save, with the qualification
  in the preview text. Stored form shown as `chunklzx-store`.
- Read every `chunklzx` member through the BIGF `Archive::read_entry` path as well (it used to
  return "LZX decompression is not yet implemented").

## 3. Implementation

| Component | Change |
|---|---|
| `crates/tdb-archive-codec/src/lib.rs` | `compress_chunklzx_store(plain)`: handoff 06 §1 byte-for-byte — header `chunklzx | 2 | fullsize | 0x40000 | n_chunks | 16-byte tail`; chunk headers at `((cursor − 8 + 15) & !15) + 8` with `zsize | 3`; sub-frames `[u16 csize]` / `[FF][u16 usize][u16 csize]`; 5 zero trailer bytes; per frame `[E8 bit] 3-bit type 3 | 24-bit length`, mspack drop-to-bytes (partial word discarded; a whole word emitted on a boundary), `R0 R1 R2 = 1 1 1`, raw bytes, one pad byte after an odd block when another follows. Empty input refused (`Malformed`). |
| `crates/tdb-archive-codec/src/lzx.rs` | `align_to_word` now follows libmspack exactly ("read 1-16, not 0-15, bits": one whole word is discarded when the block header ends on a word boundary). The 50 real vectors still decode byte-exact after the change (the boundary case does not occur in them or in the store route, whose headers are 27 / 28 bits). |
| `crates/tdb-archive-big/src/stage.rs` (+ `stage/tests.rs`) | the stager no longer refuses a new payload by its `chunklzx` form — `check_decoded` verifies it through the promoted decoder like any other form (a malformed one is `DecodeFailed`; a store-encoded one plans). `UnsupportedNewPayload` text generalised. |
| `crates/tdb-archive-big/src/lib.rs` | BIGF `read_entry` decodes `chunklzx` (`ChunkLzxDecodeFailed` on a malformed one; the `ChunkLzxUnsupported` variant is gone — no other crate used it). |
| `crates/studio-domain/src/asset_member_edit.rs` | stored form `chunklzx-store`: encode with the store route, verify by an independent decode, and append the qualification to the placement prediction (growth `from → to` bytes, "NOT established — card R30-LZX-1"). No `chunkzip` substitution. |
| `crates/studio-domain/src/copied_target.rs`, `crates/studio-api/src/registry/copied_target.rs` | the copied-target re-verification accepts `chunklzx-store` (expected form `ChunkLzx`, independent decode); the packaged-artifact route (dictionaries / databases inside archives) store-encodes a `chunklzx` member instead of refusing. |

## 4. Regression evidence (developer)

| Case | Asserts |
|---|---|
| `tdb-archive-codec` `chunklzx_store_tests::store_route_round_trips_the_research_synthetic_lengths` | the research's 16 synthetic lengths (1 … 786 437, odd / exact-frame / exact-chunk / > 2 chunks) → **exactly the research encoder's sizes** (`r30` `synthetic.encoded`) → decode byte-equal through `decompress_chunklzx` and `decompress_if_wrapped`; empty refused |
| `tdb-archive-big/tests/chunklzx_vectors.rs` (`--ignored`, real archives on `X:`) | for each of the 50 shipped members: decode byte-exact (as receipt 04) **and** store re-encode = the research's `store_encoded_size` exactly, decodes back to the plaintext, `is_chunklzx` |
| `stage/tests.rs::plan_refuses_ambiguous_and_conflicting_replacements` | a malformed `chunklzx` payload → `DecodeFailed`; a store-encoded one plans |
| `studio-api/tests/gen7_asset_member_route.rs::a_chunklzx_member_is_written_back_through_the_store_route` (`--ignored`, private Legacy X360 logo fixture) | a store-encoded real XPR2 package as a synthetic `cache.big` member: preview form `chunklzx-store`, prediction carries "uncompressed blocks" + "R30-LZX-1" and no "chunkzip"; apply committed; the installed member is `chunklzx`, its `fullsize` = decoded length, decodes to a stock-shaped package with the marked pixels (MAD 0); unrelated members untouched; session clean; restore byte-exact |

Runs: `cargo test -p tdb-archive-codec --lib chunklzx_store` (1 passed);
`cargo test -p tdb-archive-codec -p tdb-archive-big --lib` (46 + 80 passed);
[p3-chunklzx-store-vectors.log](../../validation/evidence/overnight-development-2026-09-21/logs/p3-chunklzx-store-vectors.log)
(1 passed, 50/50, 0.8 s);
[p3-chunklzx-store-route.log](../../validation/evidence/overnight-development-2026-09-21/logs/p3-chunklzx-store-route.log)
(1 passed, first run); aggregate default run of the four crates after receipts 07 + 08:
[p3-aggregate-default.log](../../validation/evidence/overnight-development-2026-09-21/logs/p3-aggregate-default.log)
(see §4a). Not run: any launch; no write into a real archive.

### 4a. Aggregate

Filled in from the log once the run completes (one run; ignored = fixture / mount-gated, never
counted as passing).

## 5. Coverage effect

- Phase 2 matrix 2.5 (gen7 X360 `rendering/*` members): **write is implemented** via the store
  route, developer-tested on a synthetic archive around a real package; **loader acceptance
  unqualified** — card **R30-LZX-1** (MORNING-REVIEW) is the discriminating test, before which no
  cell may read better than "implemented, not loader-qualified". Card R2-LZX-1 (chunkzip
  substitution) is superseded as the primary candidate; it stays listed as the fallback
  discriminator the research names.
- Memory-budget effect of a 200 kB member replacing a 5 kB one at runtime: unobserved (research
  §3); part of the same card.

## 6. Open / omissions

- A real LZX compressor would remove the size penalty; none is known in Rust (research §3).
- The real-archive apply of a store-encoded member (the `--ignored` "every real gen7 EB archive"
  case) was **not** extended tonight — the synthetic archive case covers the placement path with
  the same stager; the real copies (~450 MB per target) are a morning run.
- The BIGF reader's decode is exercised only by the unit tests of the codec (no BIGF fixture with
  a `chunklzx` member in the tree).

## 7. Cleanup

Scratchpad `store_a.rs`, `store_b.rs`, `lzx_test.rs` deleted. No temp roots (the tests remove
theirs), no processes.
