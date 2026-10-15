# Developer observation — R127f slot-1 words read past a short parameter block (28 Sep 2026, receipt 49)

This note is for research. It is recorded by the developer, edits no research file, and is not a contract change.

Studio's venue reader (`tdb_rx2::venue`, receipt 49) agrees with R127f on every listed package: 361 packages and
4 116 presentation materials, with the same materials, order, slot-0 rasters and slot-1 rasters. There is one
systematic difference, and a count difference:

1. **1 737 slot-1 words lie outside their parameter block.**
   - Where the `0xEF0001` block is shorter than 292 bytes (the PS3 packages carry 288-byte blocks), the probe read
     the u32 at +288 from the bytes after the block, belonging to the next chunk.
   - R127f lists those as `raw` values (for example `0x4638` on PS3 Legacy `adbd_ANA`).
   - Studio does not read outside a chunk. It reports "the parameter block (288 bytes) does not reach +288".
   - None of the 1 737 was a raster, so no image is lost. The words are not slot data, and the PS3 slot-1 counts
     in R127f §7 include them.
2. **Package count.** Handoff 20 §7 says 360 packages; `r127f-venue-art-slot-list.json` lists 361. Studio asserts the
   JSON's 361.

Evidence: `crates/tdb-rx2/tests/venue_r127f.rs` (`--ignored`, the six loose trees R127f read).
