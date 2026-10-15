# Receipt 58 — PS3 venue-art pictures; four more titles in the venue-art gallery

**Developer-verified; not Codex-accepted; nothing launched or written.** HEAD `8d60a6d` (unchanged; nothing staged). Claim
`dev-venue-ps3-previews`. Research consumed: 2026-09-28 handoff 03 §1 and §8 (R137). This is the read-only half of B13's next
action. Board-art replacement stays withheld.

## 1. Plain-language summary

- **On PS3 you can now view venue pictures** on the Arena page's "Venue art" card (PS3 NHL 14, 15 and Legacy). Before, PS3 showed
  sizes only.
- **The gallery now covers Xbox 360 NHL 11, 12 and 14 and PS3 NHL 14**, which research measured this morning. Before, it covered
  X360 NHL 10 / 13 / 15 / Legacy and PS3 NHL 15 / Legacy.
- The PS3 and Xbox copies of the Winnipeg board strip (`adbd_WPG`) decode to the same image. Studio's PS3 bytes equal the Xbox bytes
  once those are untiled.
- Nothing can be replaced. Previews still refuse a file that changed since it was listed. PS3 NHL 11–13 keep their venue packages only
  inside archives and are not listed.

## 2. Files

| Path | Change |
|---|---|
| `crates/tdb-rx2/src/venue.rs` | PS3 descriptors: `83f1` / `83f2` / `83f3` → DXT1 / DXT3 / DXT5; the descriptor's first u32 names a `0x10030` entry, used only when its size equals the plain chain for the level count at +52 (cube maps are named, not previewed); `ps3_plain_chain`; `raster_linear` returns the stored base level on PS3 |
| `crates/studio-domain/src/venue_art.rs` | `MEASURED` adds X360 NHL 11 / 12 / 14 and PS3 NHL 14; scope text |
| `crates/studio-domain/tests/venue_art.rs` | PS3 `adbd_WPG` previews and equals the X360 base; new `r137_trees_list_and_ps3_descriptors_preview`; unmeasured example now NHL 09 |
| `crates/tdb-rx2/tests/venue_r127f.rs` | R127f spells `83f1` / `83f2` as signatures; the comparison maps Studio's new names back |
| `crates/studio-api/tests/arena_assets.rs` | unmeasured example NHL 12 → NHL 09 |

## 3. Evidence (real loose trees, read only)

- **New sweep.** Package and descriptor counts equal R137 on every tree:
  - PS3 NHL 14: 64 packages / 3 033 descriptors · PS3 NHL 15: 64 / 3 033 · PS3 Legacy: 64 / 3 035
  - X360 NHL 11: 51 / 2 060 · X360 NHL 12: 57 / 2 567 · X360 NHL 14: 64 / 3 033
  - **Every PS3 descriptor previews: 9 101 of 9 101.**
  - X360 board-ad materials reach rasters on 11 / 12 / 14.
  - The gallery lists arena art id 1 on all six trees.
- **`adbd_WPG` (Legacy `stadium_1`):** PS3 512 × 1 024 DXT1 previews. Its base blocks are byte-identical to the X360 base after
  untiling and swapping (R137 §1). All 39 rasters of the PS3 package preview.
- `venue_r127f` 1 / 1 (every receipt-49 comparison unchanged; label mapping only). `tdb-rx2 --lib venue` 3 / 3. API `arena_assets`
  5 / 5 and `arena_venue` pass.

## 4. Limits

- On X360, 51 NHL 11 descriptors (one each on NHL 12 and 14) fail receipt 49's pixel-entry checks. They show "not previewed" with the
  check that failed. Which check fails was not broken down in this package, and R137's "entry ≥ tiled base" covers board-ad rasters only.
- Pictures are parsed data. Which picture the game draws is not observed.
- Included in the r59 bundle (after r56).
