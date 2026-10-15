# Receipt 49 — Presentation 5.2: read-only gen7 venue-art gallery (board ads, bench ads, jumbotron, power ring, glass)

**Developer-verified; not Codex-accepted; nothing launched; nothing written.** HEAD `8d60a6d` (unchanged; nothing staged).
Claim `dev-venue-board-art-gallery`. Research consumed: R127 handoff 20 §6–§9 (R127c / f / g). Research's active claim
`research-q4-board-art-writer-boundaries` covers replacement beyond this gallery; nothing here touches it.
[Observation for research](observations/49-R127f-slot1-past-block.md).

## 1. Plain-language summary

- **The Arena page has a "Venue art" card.** It lists the arena's venue packages, found through its `arenaartid` as the
  Arena assets card already does. For each package it shows every board-ad, bench-ad, jumbotron, power-ring and glass
  material by name, with the picture its data points to (size and format), and every picture in the package.
- **On Xbox 360 you can view the pictures.** Click a material's picture to see it. The Winnipeg arena's `adbd_WPG` shows
  the rink-board sponsor strip (EA Sports, Easton, NHL, Bauer, CCM and others).
  - A preview only works while the file still has the hash the listing showed; otherwise it is refused.
  - PS3 packages show sizes only, because how their pictures are stored is not measured.
- **Only research-measured games are read:** Xbox 360 NHL 10 / 13 / 15 / Legacy, and PS3 NHL 15 / Legacy. Other titles,
  and NHL Legacy Recompiled, say "not measured" and nothing is parsed.
- **It claims nothing beyond the data.** A material named "adbd" pointing at a picture is parsed data, not proof of what
  the game draws. Nothing can be replaced, re-keyed or resized from here.

## 2. Files

| Path | Change |
|---|---|
| `crates/tdb-rx2/src/venue.rs` (**new**), `lib.rs`, `texlib.rs` (`DESC_FETCH_TILED_BIT` public) | venue reader: table bounds; name dictionary (entries outside the table or file skipped and counted); rasters (X360: dims / format, `DW8` → pixel entry checked for index, `0x10031` tag and size; PS3: size and signature only); materials (the preceding chunk must be `0xEF0001`; slot words read only inside it; a word is a raster only if it names a `0x20003` descriptor); `raster_linear`; 3 unit tests on a hand-built package |
| `crates/studio-domain/src/venue_art.rs` (**new**), `lib.rs` | `MEASURED` title / platform pairs; `venue_art` (package listing, classes by name, `used_by`); `venue_raster_png` (SHA-256 must match the listing) |
| `crates/studio-api/src/{dto,convert,registry,commands,lib,bindings}.rs` | `VenueArtDto` family (6 bindings), `arena_venue_art`, `arena_venue_raster` (only `stadium/stadium_<this arena's art id>_container_<n>` of the attached tree; measured titles only); root count 170 |
| `app/src/components/ArenaVenueArtCard.tsx` (+ test), `pages/ArenaDetail.tsx`, `lib/client.ts` | the card |
| tests (**new**) `tdb-rx2/tests/venue_r127f.rs`, `studio-domain/tests/venue_art.rs`; `studio-api/tests/arena_assets.rs` +1 | below |

## 3. Evidence

- **`venue_r127f` 1 / 1 (`--ignored`, six real loose trees, read only).** Every R127f package: 361 packages and
  4 116 presentation materials (1 327 board ads), with equal material order, slot-0 / slot-1 rasters (descriptor chunk,
  ordinal, size, format) and raw words.
  - **Exception: 1 737 slot-1 words.** Research read these past a 288-byte PS3 parameter block. Studio refuses to read
    outside the chunk. None was a raster (observation 49).
  - **X360 rasters:** 10 637 of 10 684 have a validated pixel entry.
  - **First runs failed, twice (recorded):**
    - a label difference for unnamed PS3 signatures (normalised in the test);
    - then the past-block words, now explained and counted rather than hidden.
- **`studio-domain venue_art` 1 / 1 (`--ignored`):**
  - X360 Legacy `stadium_1`: 39 rasters and 13 presentation materials across all five classes, plus 51 other materials.
    `adbd_WPG` → 512 × 1024 DXT1, previewed as a 128 × 256 PNG, viewed and correct;
  - a wrong hash is refused;
  - PS3 Legacy: the same material at the same size, not previewable, with the reason;
  - NHL 12 and Recompiled: not measured.
- **API `venue_art_gallery_through_the_registry` 1 / 1 (`--ignored`):** the arena with `arenaartid` 1 on the NHL Legacy
  anchor, with the real tree attached. The preview works. Refused:
  - another arena's package;
  - a `..` path;
  - a changed hash;
  - an unmeasured title's listing and preview.
- **Other suites:** `tdb-rx2 --lib venue` 3 / 3; vitest `ArenaVenueArtCard` 3 / 3; `tsc` clean;
  `export-bindings --write` 6 new.

## 4. Limits

- **No runtime claim:** which raster the game draws, and whether it loads the loose package or the `cacherender.big`
  copy, are unobserved (R127 §8–§9). There is no replacement route.
- **PS3 rasters are listed without previews.** Their `83f1` / `83f2` signatures are not decoded, and there is no measured
  descriptor → pixel link.
- **Not measured, so not read:** X360 NHL 09 / 11 / 12 / 14, PS3 NHL 13 / 14, and Recompiled.
- **The card reads every package of the arena when it opens** (one or two files of 11–14 MB). This is not cached across
  sessions.
