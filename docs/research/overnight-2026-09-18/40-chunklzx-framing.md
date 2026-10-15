# Handoff C-X — `chunklzx` framing pinned and validated (Xbox 360 EB archive payloads)

Research lane, overnight 2026-09-18. Claim `codec-chunklzx-framing`. Consumer: developer (`tdb-archive-codec`
`decompress_chunklzx` / `lzx.rs`, currently reported unsupported because "the LZX bitstream framing isn't pinned");
unblocks previews of every `chunklzx` member (rink logos, ice/jersey/puck texlibs, attribdb `.bin/.vlt`, …) on
NHL 11–Legacy Xbox 360. Probe: `probes/chunklzx.py` (LZX core in `probes/xex_image.py`, validated on the XEX).
Vectors: [`data/chunklzx-vectors/`](data/chunklzx-vectors/) (hashes only — no game bytes stored).

Evidence: **observed and validated byte-for-byte** — R1 regeneration (2026-09-18): **50 members from six
titles (NHL 11–15, Legacy Xbox 360)** decode to exactly the loose extracted plaintexts in the X: trees
(`attribdb/{renddb.bin, feattribdb.vlt, crowds.vlt, crowds.bin, audioevents.vlt}` of every `boot.big` and
three/four `rendering/logo/texlib_*.rx2` of every `cacherender.big`/`rendercache.big`); full identities in
`data/chunklzx-vectors/manifest.json` (§2). The overnight claim of three plaintext matches is subsumed.

## 1. Framing (all integers big-endian)

```
0x00 'chunklzx'   0x08 dummy=2   0x0C fullsize   0x10 window=0x40000 (bytes per chunk)   0x14 n_chunks   0x18 0x10 …
chunk i header at H (H % 16 == 8; first H = 40; after a chunk, H = the first offset ≥ end with H % 16 == 8, i.e. `((end - 8 + 15) & ~15) + 8`; zsize need not be a multiple of 4):
    zsize u32, flag u32 (=3 observed), then zsize payload bytes
payload = sub-frames:
    full frame:  [u16 csize][csize bytes]                       → 32 768 output bytes
    last frame:  [0xFF][u16 usize][u16 csize][csize bytes]      → usize output bytes (usize may be 32 768)
    then 5 trailing zero bytes (observed on every specimen; not consumed)
```

The **concatenated sub-frame bytes of one chunk are one LZX stream** (mspack semantics): leading 1-bit Intel-E8
header (0), **window bits 17** (34 position slots; the header's `window` is the chunk size, not the LZX window —
2^18 does not decode), 32 KiB frames with 16-bit realignment at each frame end; the whole chunk is typically
one aligned-offset block (block length = chunk output size). Each chunk starts a fresh LZX stream. Output is
exactly `fullsize`; no zero padding is needed.

What Studio's walker gets wrong today (`decompress_chunklzx`): (a) it feeds the raw payload — including the
u16 / `FF u16 u16` sub-frame prefixes — to the LZX decoder; (b) it advances to the next chunk header in steps
of 8 with a 2-step limit, which can never reach a header from an offset ≡ 12 (mod 16) (`crowds.vlt`: chunk 0
ends at 29 356, chunk 1 header at 29 368); (c) it derives the window bits from the header word (18) instead of
17. The LZX core (`lzx.rs`, "verified byte-for-byte against `gamedata/physics/refcoll.xml`") is presumably fine.

## 2. Vectors (regenerated 2026-09-18 R1 with the final decoder — supersedes the overnight table)

The single regression reference is [`data/chunklzx-vectors/manifest.json`](data/chunklzx-vectors/manifest.json):
50 members from **six titles (NHL 11, 12, 13, 14, 15, Legacy Xbox 360)** — five `attribdb` members of each
`boot.big` and three or four `rendering/logo/texlib_*.rx2` members of each `cacherender.big`
(`rendercache.big` on NHL 11) — every one **byte-equal to the loose extracted plaintext in the same X: tree**
(`plaintext.equal == true` on all 50; no zero padding anywhere; `fullsize == output length`). Per vector the
manifest stores: archive path + full SHA-256 + EB version + entry count, entry index, absolute offset,
stored size, `w4`/path-hash words, input SHA-256, the `chunklzx` header words, per-chunk `{header_offset,
zsize, flag, subframes, full_frames, last_frame_usize, last_frame_csize, stream_bytes, produced, trailer}`,
output length + SHA-256, and the plaintext path/length/SHA-256. `decoder_identity` carries the SHA-256 of
`chunklzx.py`, `xex_image.py` (LZX core), `eb_list.py` and `chunklzx_vectors.py` that produced it. The
README beside it repeats the compact table; the overnight `*.decoded.sha256` file and the earlier README
rows (a 4 096-byte zero-padded `texlib_27_0_3` result and a 196 608-byte unpadded run) were **superseded and
removed** — they came from the decoder before the last-frame rule was pinned.

Coverage of framing variants (all with flag 3 and a 5-byte zero trailer): single last-frame chunks
(`feattribdb.vlt`, `audioevents.vlt`, `crowds.bin`); multi-frame chunks ending in a partial last frame
(`renddb.bin` 4 sub-frames, texlibs 7 sub-frames with `usize` 4 096); a last frame whose `usize` is a full
**32 768** (`crowds.vlt` chunk 0 on every title, `renddb.bin` NHL 12); **two-chunk** members (`crowds.vlt`
everywhere, `renddb.bin` NHL 11/12); and chunk sizes ≡ 2 (mod 4) (about half the set), which is what broke
the step-by-4 alignment in the overnight probe — the chunk header rule is `H = ((end − 8 + 15) & ~15) + 8`.
Byte accounting holds on every chunk: `zsize = Σ_full(2 + csize) + (5 + csize_last) + 5`.

Not covered (no such member found in the sampled sets): a stored (`flag == 0`) chunk, members with more than
two chunks, a chunk without a last-frame marker. NHL 07–10 do not use `chunklzx` (BIG4 era). This qualifies
the framing on the six sampled titles' `attribdb` and `rendering/logo` members — not every member family.

Reproduce (any row): `python probes/eb_list.py "<archive>" --extract <member> m.bin` →
`python probes/chunklzx.py m.bin out.bin` → compare `sha256(out.bin)` with `output_sha256` / the loose file;
or regenerate the whole manifest with `python probes/chunklzx_vectors.py probes/chunklzx_vectors.spec.json
out.json --archive-hashes`.

## 2b. Required decoder behaviour and malformed-input cases (for `tdb-archive-codec`)

Required:

1. Header: magic `chunklzx`, `dummy` (=2 observed) @8, `fullsize` @12, `window` @16 (chunk size, 0x40000
   observed), `n_chunks` @20; data from @40. Reject `fullsize == 0` / `n_chunks == 0` with fullsize > 0.
2. Chunk header at `H = ((cursor − 8 + 15) & ~15) + 8` (first `H = 40`): `zsize` u32 BE, `flag` u32 BE.
   `flag == 3` = LZX sub-frames (below); `flag == 0` = stored `zsize` bytes (mirrors chunkzip's stored flag;
   **not observed**, keep as a defensive path); any other flag → error naming the chunk.
3. Sub-frame walk over the `zsize` payload: `[u16 BE csize][csize bytes]` = a full 32 768-byte frame;
   `[0xFF][u16 BE usize][u16 BE csize][csize bytes]` = the last frame (`usize ≤ 32 768`); then 5 trailer bytes
   (ignore, but require `Σ == zsize`, i.e. the trailer is exactly what is left).
4. The concatenated sub-frame bytes are **one LZX stream per chunk**: E8 header bit present, window bits
   **17** (fixed — never derived from the header word), 32 KiB frames with 16-bit re-alignment at each
   frame boundary; decode exactly `full_frames × 32 768 + usize` bytes.
5. Output must equal `fullsize` after the last chunk; never zero-pad, never truncate silently (chunkzip's
   `ShortOutput` semantics).

Malformed-input cases to test (synthetic fixtures, no game bytes): truncated header (< 40 B); `zsize` running
past the end of the payload (`TruncatedChunk`); a sub-frame `csize` running past the chunk; a last-frame
`usize` > 32 768; a chunk whose sub-frames sum to more than `fullsize − produced` (overrun) or fewer bytes
than declared (short output); `n_chunks` larger than the data can hold; a chunk header that is not 16-byte
aligned by the rule above (should be reached, not stepped past — the step-by-8 walk that never terminates or
mis-lands); a zero-length block inside the LZX stream (guard against an infinite loop); window word ≠
0x40000 (decode must not depend on it); LZX position-slot table sized for 17 bits when the stream needs 18
(should error, not read out of bounds).

## 3. What it unlocks

- Every `chunklzx` member on NHL 11–Legacy Xbox 360 becomes readable (rink logos, `centerlogo_*`, ice
  components, jersey `texlib_*`, pucks, attribdb). Rink logo texlibs each carry **three rasters**
  `logo_{id}_{a}_{b}_{0,1,2}.Raster` (DXT5 256×128 / 128×64 / 128×128), rendered tonight with a port of
  `texlib.rs` (`probes/rx2_preview.py`, `previews/rinklogos/`): `27_0_0` current Leafs crest, `27_1_6` vintage
  crest, `27_0_3` blank — `a` = set (current/vintage), `b` = variant slot (inferred).
- Write-back still needs either an LZX encoder (not attempted) or the existing `chunklzx → chunkzip`
  substitution (`compress_chunkzip`); runtime acceptance of a substituted codec stays a roadmap question.
- Family check: NHL 12 `cacherender.big` `rendering/logo/texlib_0_0_0.rx2` decodes with the same framing (200 704 B
  RW4); NHL 11 members are `chunklzx` too (not decoded). **PS3 does not use it**: the PS3 Legacy `cacherender.big`
  and `boot.big` members are `chunkzip` (`texlib_0_0_0.rpsgl`, `attribdb/renddb.bin`), which Studio already decodes.
