# NHL Legacy — Xbox 360 — platform limitations (write path)

> **Scope correction (2026-08-03):** the studio's write target is the **extracted file formats**
> (the decompressed `RW4`/`RX2` texlibs, XPR2 portraits, mesh containers), **not** repacking the
> `.big` archives — the user's own pipeline handles getting edited extracted files back into the
> game. So the EB-archive writer and the `chunklzx`/LZX codec below are **not required for the goal**
> (kept as parked/optional work — see the note at the end). Read-side (discover / identify / preview
> / export) is confirmed — see [`ASSET_PROVENANCE.md`](ASSET_PROVENANCE.md) and
> [`IDENTITY_MAP.md`](IDENTITY_MAP.md).

## Extracted-format write — SUPPORTED

Editing a texture in an extracted `.rx2`/texlib and writing the file back works through
`tdb_rx2::texlib::{replace_same_size, replace_with_resize, replace_with_mip_chain,
replace_base_keep_tail}` (portraits: `xpr::Xpr2Texture::{replace_resized, replace_base_mip_linear}`;
meshes: the `container` encoder). Verified end-to-end by
`assets::tests::legacy_x360_extracted_texlib_write_roundtrip`: on the real extracted Red Wings logo
`rendering/logo/texlib_10_0_0.rx2` it (a) rewrites an unchanged base mip and re-decodes it
**byte-for-byte** (lossless), and (b) recolors it and re-decodes the edit from the rewritten `.rx2`.
`+12` and `.big` framing are irrelevant here — the file being written is the loose extracted asset.

**Portraits — supported (2026-08-03), both platform encodings.** `assets::replace_portrait_texture`
picks the largest decodable slot in the extracted `p{id}.big` and branches on its container:

- **Xbox 360 `XPR2`** — RGBA → DXT via `texpresso` → `xpr::Xpr2Texture::{replace_base_mip_linear,
  replace_resized}` → RefPack → BIGF rebuild. Verified by
  `assets::tests::legacy_x360_extracted_portrait_write_roundtrip` (real `playerheadssmall/p1.big`
  XPR2 replaced with a gradient, rebuilt + written, re-`decode_portrait`ed intact).
- **PS3 `DDS `** — RGBA → DXT full mip chain (`compress_bc_mip_chain`) → `encode_dds` →
  RefPack → BIGF rebuild. The read side gained the matching branch (`rasters_in` decodes a RefPack→`DDS `
  slot via `tdb_archive_codec::dds_base_mip`). Verified by
  `assets::tests::ps3_dds_portrait_write_roundtrip_synthetic`: a synthetic PS3-style portrait `.big`
  (DXT5 DDS) is replaced, and the rebuilt entry is confirmed to be a **full-mip-chain DDS** (proving the
  DDS arm ran, not XPR2) that re-`decode_portrait`s with the new gradient intact.

Both pipelines are ported from the prior app, whose portrait packing was proven against real EA
descriptors. The prior app's **PS3 portrait export rode this same `DDS → RefPack → BIGF` pipeline** and
shipped — so the PS3 branch is **proven-ported**, not net-new. (In the prior app the `\x89RW4ps3`
container used by NHL 11–14 was read/export-to-PNG only, never a replace target — matching our current
NHL-Legacy-PS3 = `DDS ` scope.) Our own coverage adds a synthetic round-trip
(`ps3_dds_portrait_write_roundtrip_synthetic`) proving the port's mechanics; a real extracted-PS3-file
test mirroring the Xbox mount-gated one would strengthen it further once a PS3 portrait path is provided
(path-intake gate). `game-verified` stays the standing manual gate, as for every family.

Remaining write gaps (extracted formats): **PS4 GNF** has no encoder yet (read-only). `game-verified`
stays a manual gate (and depends on the user's own repack/install pipeline).

---

## Below: `.big` archive repacking — PARKED (not needed for the goal)

The following was investigated before the scope correction above. It is retained for reference but is
**not on the critical path** for editing extracted files.

## Container packaging — the `EB\0\3 0x0000_0400` archive

Assets ship inside two-level `EB\0\3` `.big` archives (`cacherender.big`, `nocacherender.big`,
`cache.big`, …); see [[../../../../audits/data/assets/nhllegacy/xbox360/archives.csv]].

**Writer — DONE (write path, 2026-08-03).** `tdb_archive_big::eb::EbFile` parses a whole archive
retaining the header/table prefix + every payload, re-emits **byte-identically** when unmodified
(verified against the real `data0.big`), and `replace_payload` swaps one entry's bytes and reflows all
later blocks (16-byte-aligned, contiguous; file table `block`/`size` + header `archive_size` patched).
Proven by `eb::tests::writer_{round_trips_synthetic_byte_identical, replace_payload_reparses_and_reflows,
round_trips_real_data0_big}`. So the container **`rebuild`** mechanism exists; a modified archive
re-parses cleanly with the independent reader.

## Remaining blocker — the payload codec `chunklzx` (LZX)  [game-verified, 2026-08-03]

Three game-verified probes on `boot.big` (recoloring the boot splash `rendering/boot/texlib_0.rx2` +
`texlib_5.rx2` and loading in Xenia) settled the write path:

- **Probe #1** — raw payload + reflow (archive grew): **crash.**
- **Probe #2** — `chunkzip` payload, **in-place, same length, `+12` kept**: **crash.** (Byte-identical to
  stock except the two payloads.)
- **Probe #3** — original valid `chunklzx` payloads, **only the `+12` word corrupted**: **crash.**

Probe #3 shows the loader **validates `+12`** — but the `+12` *values* prove it is a **filename/path
hash, not a content checksum**: for `rendering/boot/texlib_N.rx2` the word is exactly
`0x15675BB6 + N × 0x121881` (perfectly linear across N=0..6, though the payload sizes vary from 462 B to
184 KB). **So `+12` is automatically correct for any edit that keeps the filename** — it is *not* a
blocker for replacing existing assets (`EbFile` already preserves it). Probe #3 only broke because it
corrupted the hash artificially.

That leaves Probe #2 — a correct `+12`, everything byte-identical except a `chunkzip` payload — which
still crashed. Therefore **the loader requires `chunklzx` (LZX) specifically**; it does *not* accept a
substituted `chunkzip`/raw payload. **Implementing LZX (chunklzx) (de)compression is the single
remaining blocker** for the write path (and LZX decompression separately unblocks reading straight from
the `.big`, removing the extracted-tree dependency). The chunklzx wrapper is
`[version=2][uncompressed_size][window=0x40000][n_chunks][hdr]` + a per-chunk table.

`install` / `game-verified` remains the manual gate; never auto-promoted from an automated round-trip.

## Summary

| Operation | State |
|---|---|
| discover / identify / preview / export | **pass** (read side confirmed) |
| EB container rebuild (byte-identical + in-place / reflow replace) | **done** (`EbFile`) |
| `+12` checksum | **solved** — filename hash, auto-correct on replace (preserved verbatim) |
| replace-modified (end-to-end texture) | **blocked** — game requires `chunklzx` (LZX); need LZX compression |
| install / game-verified | **blocked** — manual emulator/console gate (also needs the above) |

Nothing here is PS3 mesh (that title/platform has its own categorical mesh disable); on **Xbox 360**
mesh editing is not categorically blocked — it is simply out of scope for the current texture-first
families (5.1–5.3).
