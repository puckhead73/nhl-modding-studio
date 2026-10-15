# C0-POR1 inspection tooling — preserved for review and re-derivation

The C0-POR1 portrait findings were produced by throwaway scripts in a session scratch directory.
C0-POR1-R1 preserved them here so the findings can be checked or challenged rather than taken on
trust. **This is a location choice, not an established convention** — if `docs/receipts/data/` is
the wrong home for executable inspection tooling, say so and it moves.

## What is here

| File | Role |
|---|---|
| `por1_formats.py` | The inspection parsers and decoders: RefPack, `BIGF`, `DDS`, `XPR2`, `GNF`, DXT5, Xbox 360 untiling |
| `por1_census.py` | Walks the three supplied roots and writes the reconciled counts |
| `por1_specimens.py` | Hashes the named specimens and runs the PS3/Xbox 360 block-stream comparison |

## Running them

Both scripts are read-only with respect to every source location, take an output directory, and
write only aggregate JSON. Python 3.12 was used; they have no third-party dependencies.

```sh
# Reconciled counts by root, folder, extension, subfolder, group and structural tuple.
python docs/receipts/data/c0-por1-tools/por1_census.py docs/receipts/data

# Specimen hashes; --compare additionally re-runs the cross-platform comparison.
python docs/receipts/data/c0-por1-tools/por1_specimens.py docs/receipts/data --compare
```

Outputs land at `docs/receipts/data/c0-por1-census.json` and
`docs/receipts/data/c0-por1-specimen-hashes.json`. Each embeds the SHA-256 of the scripts that
produced it, so a report can be tied to the exact tooling revision that generated it.

## The source roots are private and are not in this repository

The three roots are hardcoded at the top of each script (`POR1_ROOTS`, and `PS3`/`X360`/`PS4`). They
are user-supplied, private, uncommitted locations on one machine. **No game bytes are written to the
repository by any of this** — the outputs carry counts, header field values, digests and at most one
example relative path per observed group.

These paths are recorded so the run is reproducible on that machine. They are **not** a fixture
declaration: nothing in the build or test tree reads them, and no test depends on them. See
`docs/validation/c0-05-fixture-runtime-register.md` §5.7.

## What these tools are, and what they are not

They are **inspection** parsers written for this package. A file reported as `accepted` is one where
these routines found the fields they look for at the offsets they expect. That is **not** semantic
validation, and it does not establish that a file is well-formed for the game.

They are **not** this repository's parsers. `tdb-rx2`, `tdb-archive-big` and `tdb-archive-codec` were
**not exercised**, so nothing produced here qualifies them — with one documented exception:

> `por1_formats.xbox360_tiled_offset` is **transcribed** from
> `crates/tdb-rx2/src/texlib.rs::xg_address_2d_tiled`. Results depending on it are **not** independent
> of this repository. It is kept here so the transcription itself is reviewable.

## Independent versus shared stages

This matters for reading the PS3 ↔ Xbox 360 comparison:

```
PS3   chain: parse_bigf -> refpack_decode -> parse_dds  -> decode_dxt5
X360  chain: parse_bigf -> refpack_decode -> parse_xpr2 -> swap16 -> untile -> decode_dxt5
```

`parse_bigf`, `refpack_decode` and `decode_dxt5` are the **same code** on both chains. Agreement
between a PS3 and an Xbox 360 file therefore exercises only the stages that differ — `parse_xpr2`,
`swap16` and the transcribed tiling formula — and **cannot detect a fault shared by both**. It is
also a statement about the specimens compared and about nothing else: it does not establish that
either tree is authentic, unmodified, or the set the game ships.
