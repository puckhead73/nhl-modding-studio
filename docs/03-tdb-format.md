# 03 — EA TDB Database Format

> **Superseded for PS3 by the VERIFIED spec:** [formats/tdb-v6-format.md](formats/tdb-v6-format.md)
> (reverse-engineered and validated 100% vs the meta XML). This draft remains for general/cross-platform
> context; use the verified spec for the actual byte layout.

> Engine home: the `tdb-core` / `tdb-defs` crates and the **(title, platform) schema/codec
> registry** in [DESIGN.md](DESIGN.md#cross-cutting-the-title-platform-registry). The reference app
> proved the format for **Legacy / 14 / 15**; per-title schemas for the rest of NHL 10–21 are
> reversed in [ROADMAP.md](ROADMAP.md) Phase 10.
>
> **Verification status: DRAFT.** This document captures intent and a place to record confirmed
> facts. EA's TDB format has real nuances that vary by **year (NHL 10–21)** and **platform**.
> **Do not treat any binary-layout specific here as fact until it is confirmed** against the prior
> reference implementation or against real (locally held, never-committed) game data. Where a claim
> is unverified, it is marked `[UNVERIFIED]`.

## What TDB is

EA's NHL titles store much of their editable game data — players, teams, rosters, attributes, and
related tables — in a proprietary database format referred to as **TDB**. NHL Modding Studio's
`tdb` crate is responsible for reading and writing these databases losslessly.

## Why a version-aware codec is required

Across NHL 10 → 21 the databases differ in ways that matter:

- **Schema drift** — tables and fields are added, removed, resized, or reordered between years.
- **Field encodings** — attribute scales and bit-packing can change.
- **Endianness** — big-endian on PowerPC platforms (Xbox 360 / PS3) vs little-endian on PS4 `[UNVERIFIED for specific titles]`.
- **Packaging** — how the TDB is embedded/packed within the title's files can differ by platform,
  including the **Legacy Recompilation** (Xbox-derived).

The codec layer must therefore resolve the correct layout from **(title, platform)** rather than
assuming one universal format.

## Facts to confirm and record here

As each is verified, replace the checkbox with the confirmed detail and cite the source
(reference impl / observed data):

- [ ] TDB file/container magic bytes and header layout, per era.
- [ ] Table directory / index structure (how tables and records are located).
- [ ] Record/field descriptor format (types, widths, bit-packing).
- [ ] String storage / pooling scheme.
- [ ] Endianness rules per platform and per title.
- [ ] Checksums / integrity fields that must be recomputed on write.
- [ ] How the TDB is packed within each platform's title files.
- [ ] Per-title schema deltas (10 → 11 → … → 21).
- [ ] Legacy Recompilation differences vs the stock Xbox build.

## Design requirements for the `tdb` crate

- **Lossless round-trip:** read → write with no edits must reproduce the original bytes
  (or a documented, intentional normalization).
- **Validated writes:** recompute any checksums/lengths; reject writes that would corrupt.
- **Schema registry:** declarative per-(title, platform) schema definitions, versioned.
- **Tolerant reads where safe:** surface unknown/extra tables rather than failing outright, so new
  titles degrade gracefully.

## Mechanics confirmed by the reference engine

The reference app's `tdb-core` already encodes these as verified (port faithfully, fold specifics
in here as they are documented): **CRC recomputation** on serialize, **Huffman type-66 pool rebuild**
(string/localization pools), **RawId namespace reversal** (byte-reversed namespace bridge), and
endian handling for the PowerPC platforms. PS4 little-endian paths extend this (net-new).

## References

- Prior rough-draft application (ask the user for its path) — the best source of confirmed format
  details to fold in here. Its design brief is captured in [rebuild-design.md](rebuild-design.md).
