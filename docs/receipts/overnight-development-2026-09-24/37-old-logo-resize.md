# Receipt 37 — logos on NHL 07 → 11 can take a new size (Xbox 360 and PS3 package form)

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD `8d60a6d`
(unchanged; nothing staged or committed). Claim `dev-old-logo-resize` (written after the first edits — a process
slip, recorded here). Direction: [overnight prompt after receipt 31](../../plans/development-20260927-overnight-after-receipt31.md)
§3 / §4 (League 1.4 / Teams 2.4). Research consumed: **R132** (2026-09-26 handoff 25 §1–§4), R132b, R132c (PS3),
R132d (NHL 07). **Not in either review bundle.**

## 1. Plain-language summary

- Replacing a team or league logo with a picture of a **different size** now works on the older titles'
  logo packages too (Xbox 360 NHL 07 → 11 and PS3 NHL 08 → 11), not only on NHL 12 and later. Before, those
  titles refused anything but the original size.
- What makes it safe: research measured how these older pictures describe their own size (one rule, on 6 926
  pictures). Studio's rule reproduces **every one** of the 4 634 single-picture logos on the Xbox 360 NHL 07 → 11
  trees byte for byte before it is allowed to write a new one. The package's display record (the rectangle the
  game draws the logo in) is updated to the new size, exactly as on NHL 12 and later.
- Still refused, with the reason: calendar logos and a few others whose display rectangle is a crop, packages
  without a display record (`bar.big`), pictures with several mip levels or stored unswizzled (nameplates), and
  NHL 07 pictures whose pixel data is not a tagged entry.
- Whether the game shows the resized logo correctly is not observed (card below).

## 2. Files

| Path | Change |
|---|---|
| `crates/tdb-rx2/src/texlib.rs` | `replace_with_resize` accepts a single-level, Xbox-tiled `8_8_8_8` raster (the descriptor rule already covered uncompressed sizes: pitch = align(w, 32) / 32, `DW9` = (w−1) \| (h−1) << 13); the buffer is tiled per texel without byte swap (align(w, 32) × align(h, 32) × 4); a raster whose pixels are not a tagged data entry is refused before anything is built |
| `crates/studio-domain/src/assets.rs` | `argb_resize_refusal` (tiled, single level, 32–8 192 texels) used by the writer and by `plan_texture_replace`; resized ARGB8 writes carry a disclosure; the package route no longer refuses RW4xb2 / RW4ps3 size changes (the texlib writer decides); `rewrite_apt_geometry` handles a RefPack-wrapped member `0` (NHL 11) and patches same-length members **in place** (the NHL 07 → 10 packages page-align their members, which the re-plan does not measure) |
| tests | `studio-domain/tests/x360_old_logo_resize.rs` (**new**, 2); `team_logo_dimension_sweep.rs` (member `0` unwrapped for the Apt check; refusal classes for "no Apt geometry record" and "untagged pixel entry") |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- `r37-old-logo.log`: **descriptor self-check** over every single-level tiled `8_8_8_8` raster in the loose logo
  families of Xbox 360 NHL 07 (230), 08 (683), 09 (1 098), 10 (1 108), 11 (1 515) = **4 634 / 4 634 reproduced**
  (descriptor bytes equal, payload = align(w, 32) × align(h, 32) × 4). NHL 07 is beyond R132's measured population
  (08 → 11); this check is the developer's, and it holds there too. **Route**: `t0.big` team logo 256×256 → 320×128
  on NHL 08 (whole-file RefPack kept), 09, 10 and 11 (RefPack texture member) re-opens at 320×128 with the
  imported pixels (mean difference < 1); the first calendar package of each title refuses "not the texture's own
  size".
- `r37-r38-real.log`: private-fixture sweep `team_logo_dimension_sweep` 1 / 1 over 168 gen7 packages — X360 NHL
  07 → 11 and PS3 NHL 08 → 11 full-quad families now **resize** (team logos, small, wide, dynasty, broadcast) with
  every invariant (new size re-opens, pixels within MAD 14, other members byte-identical, trailer / tail kept, Apt
  size words = new size); calendar / crop packages refuse "display-crop quad"; `misc/bar.big` refuses "no Apt
  geometry record"; NHL 12 → Legacy unchanged.
- `r37-rx2.log`: `tdb-rx2` lib 120 / 120.

## 4. Limits

- The package route does not surface the "resized under the measured descriptor rule" disclosure (only the
  loose texlib route does); the result reports the new size.
- PS3 NHL 08 → 10 packages live inside BIG4 archives; this receipt covers the extracted package (and PS3 NHL 11's
  loose packages). Placing a resized member back into a BIG4 archive is the archive route's concern.
- NHL 07: admitted by the developer's descriptor self-check and the fixture sweep; research measured NHL 07's Apt
  classes (R132d), not its descriptors.
- Runtime display is not observed.

## 5. Card

- **T-3b** (desktop + game, a build with receipt 37, on a copy): X360 NHL 10 and NHL 11 → a team → Logos → team logo →
  Replace with a 320×128 test image → Save → in game, the team select / scoreboard logo shows the test image
  undistorted. A stretched or missing logo = the display rectangle or descriptor is not what the game uses.

## 6. Cleanup

Test temp roots (`nhlms-oldlogo-*`, `nhlms-teamlogo-dims-*`) are removed by their guards; scratch scripts deleted.
