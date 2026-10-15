# Handoff R30 — A codec-preserving storage route for `chunklzx` members: LZX uncompressed blocks inside the pinned framing (round-trips 50 / 50 shipped members)

Research lane, 21 September 2026, Priority 1.2 ("chunklzx encoding or a genuinely supported alternate storage route"). Claim
`research-r30-chunklzx-store`. Nothing game-verified; **loader acceptance is not established** — this is a syntactic route validated
against the research decoder that was itself validated byte-for-byte on shipped data; no writer into any archive is proposed here.

Machine-readable: [`data/r30-chunklzx-store-roundtrip.json`](data/r30-chunklzx-store-roundtrip.json). Encoder
`local/runs/overnight-research-2026-09-21/probes/chunklzx_store.py` (library + CLI, ≈ 90 lines), round-trip probe
`r30_chunklzx_store_roundtrip.py` (≈ 1 s).

## Plain-language summary

The gen7 Xbox 360 `rendering/*` textures (rink and centre-ice art, jersey and logo texlibs, `attribdb`) are stored in a container
(`chunklzx`) whose inside is Microsoft LZX compression — readable since the 2026-09-18 framing work, but not writable because nobody has an
LZX compressor. LZX itself allows a block that carries its bytes uncompressed. This handoff provides an encoder that writes any file as a
valid `chunklzx` member using only such blocks, keeping the container identity the game expects (same magic, chunk headers, flag 3 sub-frame
framing, trailer), and shows that the research decoder reads every one of the 50 previously validated members back byte-for-byte after
re-encoding, plus 16 synthetic sizes (1 byte to 3 chunks + 5). The price is size: a texlib that ships compressed to 5–9 kB becomes 200 kB
(the plaintext + 74–182 bytes), so any use of this route is a *changed-length* archive write under the existing EB placement rules.
Whether the game's own LZX decoder accepts uncompressed blocks in these members is the one remaining question, and it is a runtime one.

## 1. What is written (all measured against the 2026-09-18 framing handoff 40 and the 50 vectors)

- Header: `chunklzx │ u32 2 │ u32 fullsize │ u32 0x40000 │ u32 n_chunks │ 00000010 00000000 00000000 00000000` (the 16-byte tail is the constant
  observed on all 50 vectors).
- Chunks of 0x40000 plaintext bytes (8 frames of 32 768), each at `H = ((cursor − 8 + 15) & ~15) + 8` with `u32 zsize │ u32 3`, then the
  sub-frames `[u16 BE csize][bytes]` (full frames) and `[0xFF][u16 BE usize][u16 BE csize][bytes]` (last frame), then 5 zero bytes;
  `zsize = Σ(2 + csize) + (5 + csize_last) + 5` as pinned.
- The LZX stream per chunk: 1 header bit (E8 translation off), then per frame `3 bits type = 3 │ 24 bits length`, the mspack "drop to bytes"
  alignment (partial word discarded; a whole word emitted when the header ends on a word boundary, because mspack discards one there),
  `R0 R1 R2` = `1 1 1` (LE u32), the frame's raw bytes; a pad byte after an odd block only when another block follows. Each sub-frame's
  `csize` is exactly its frame's stream bytes (full frames: 32 784 B).

## 2. Verification (researcher-verified; `r30-chunklzx-store-roundtrip.json`)

| Set | Result |
|---|---|
| 50 shipped members' plaintexts (NHL 11–Legacy X360 `attribdb/*` and `rendering/logo/texlib_*.rx2`, SHA-256 re-checked against the manifest) | encode → `chunklzx.decompress` → **50 / 50 byte-equal**; 1–2 chunks; overhead 74–182 B over the plaintext; ratio vs the shipped compressed member 1.8× (`audioevents.vlt`) to 209× (`texlib_27_0_3`, a mostly-blank raster that ships in 960 B) |
| Synthetic lengths 1, 2, 3, 15, 16, 17, 32 767, 32 768, 32 769, 65 535, 65 536, 262 143, 262 144, 262 145, 262 144 + 32 768 + 1, 3 × 262 144 + 5 | **16 / 16** byte-equal (odd lengths, exact frame / chunk multiples, > 2 chunks) |
| Empty input | the encoder emits one chunk with a 0-byte last frame; the decoder contract of handoff 40 refuses `fullsize 0` — the writer must refuse empty members |

The decoder used is the one whose identity (`chunklzx.py`, `xex_image.py` SHA-256) is recorded in the vector manifest; its LZX core is an
mspack-semantics port validated on the 50 shipped streams. The type-3 path of that core (`drop_to_bytes`, R0–R2, odd-length pad) follows
mspack; the round trip therefore establishes conformance with mspack's reading of LZX uncompressed blocks, not with the console's decoder.

## 3. What it enables and what it does not

- **Enables (static):** a `chunklzx` member with changed content and any length can be produced without an LZX compressor, with the
  container fields the loader inspects (`fullsize`, chunk count, `zsize`, flag 3, sub-frame counts) all consistent. Combined with the EB
  `0x400` placement rules (2026-09-20 handoff 03 §3: append-at-end, patched table words, whole-archive hashes) this is a complete static
  path for gen7 X360 `rendering/*` replacement — the developer's stated Phase 2.5 blocker "(a) chunklzx decoder promoted **and** an
  encoder or a loader-accepted substitute codec".
- **Does not establish:** that the Xbox 360 loader's LZX decoder accepts type-3 blocks in these members (it is standard LZX, and the
  Xbox `XMemDecompress` LZX family implements the format, but nothing was run); the 16-byte header tail's meaning (copied verbatim); whether
  a member that grows from 5 kB to 200 kB fits the title's memory budget at runtime (unobserved). Card **R30-LZX-1** below is the
  discriminating test; it supersedes nothing in card R2-LZX-1 (chunkzip-in-place-of-chunklzx), which remains a separate, less codec-faithful
  candidate.
- **Better later:** a real LZX compressor (no maintained Rust crate is known to this lane; the mspack C library and libmspack-based tools
  compress LZX for CAB) would remove the size penalty; the store encoder is the minimal correct writer until then.

## 4. Card R30-LZX-1 (NHL Legacy Xbox 360; deferred; needs a copied-target write which this lane does not perform)

Baseline: `cacherender.big` member `rendering/logo/texlib_27_0_0.rx2` (Leafs current crest, vector row in the 2026-09-18 manifest; SHA-256
there). Treatment A: the same plaintext re-encoded with `chunklzx_store.py` (content unchanged, only the storage form) placed by the
developer's EB append route into a **copied** archive. Observable: the rink centre logo at Toronto renders unchanged. Control: the
untouched archive. Treatment B (only after A passes): a visibly edited crest through the same route. Verdicts: A renders = the loader
accepts uncompressed LZX blocks in this member family (nothing more); A fails / blank = refuse the route for this family (and try B with the
existing chunkzip substitution as the next discriminator); A renders but B does not = the loader accepted the container but the
texture edit is wrong (texlib rules), not the codec. Restore: replace the copied archive with the original (hash-verified).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r30_chunklzx_store_roundtrip.py docs/research/overnight-2026-09-21/data/r30-chunklzx-store-roundtrip.json   (≈ 1 s)
python local/runs/overnight-research-2026-09-21/probes/chunklzx_store.py <plaintext> <member.chunklzx>                                             (CLI)
```
Inputs: the loose plaintexts named in `docs/research/overnight-2026-09-18/data/chunklzx-vectors/manifest.json` (X: trees, read-only).
Executed: the round trip once (all 50 + 16). Not executed: any archive write, launch, or decode with the console's decoder.
