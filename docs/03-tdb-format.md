# 03 — EA TDB Database Format

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

## References

- Prior rough-draft application (ask the user for its path) — likely the best source of confirmed
  format details to fold in here.
