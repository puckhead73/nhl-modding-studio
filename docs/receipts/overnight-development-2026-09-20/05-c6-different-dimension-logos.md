# Receipt 05 — C6: different-dimension league logos and imagery (Priority 4)

Developer lane, overnight 2026-09-20. **Developer-verified; not Codex-accepted.** Baseline `8d60a6d`.

## Plain-language summary

A league logo or presentation image can now be replaced at a **different size** on the titles where
Studio knows every byte that has to follow: the Xbox 360 NHL 12–Legacy (and Recompiled) packages, at
the two widths their GPU descriptor is measured for (64 and 256 wide, any height in steps of 8), and
the PS3 NHL 12–Legacy packages (any size). Besides the texture itself, the package's two layout
members are rewritten — the six-vertex quad in the geometry record and the size words in the Apt
object — so the art is drawn at its new size, 1:1. Where the game draws a *smaller or rotated* quad
than the texture (trophies, broadcast trophies, MUD logos, NHL 07's oddly sized art) the size
relationship is not understood and a resize is refused with that reason; the RW4 packages of NHL
07–11 (both consoles) and the PS4 GNF members stay same-size only because their texture headers are
not regenerated. Same-size replacement is unchanged everywhere. Whether a game accepts a resized quad
is a runtime question for the user.

## 1. Package, disposition, scope

C6 — delivered for the supported families, developer-verified; the blocked families are named with
their blocker. Bounded developer probe (no research handoff existed for the record's exact layout):
the 288-byte record and member 0 were measured on all 141 fixture packages (77 logos + 64 imagery,
every gen7 title/platform/family); the XPR2 descriptor words on the 8_8_8_8 slots of NHL 12/Legacy.

## 2. Changed paths (SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/studio-domain/src/assets.rs` | `4c221c513295dbfd` | `replace_portrait_texture_noted`: dimension-change gate + `rewrite_apt_geometry` (record vertices and member-0 size words verified against the old size, rewritten to the new, both through the re-plan); `package_texture_dims`; `DdsArgb` resize (standard DDS header words: height/width/pitch/mips) |
| `crates/tdb-rx2/src/xpr.rs` | `84ad626baacfe0a3` | `replace_resized` for `Argb8888` at the measured widths (64 / 256; DW7 second byte 0x80 / 0x00 as measured; DW9 `height/8-1` + width word; `data_size`); `XprError::ResizeUnmeasured` |
| `crates/studio-domain/tests/league_logo_packages.rs` | `4ec6c1442c2aa33a` | +2 `--ignored`: the Apt geometry sweep over all 141 packages; the resize cases (2× / ½ or 64↔256 on XPR2 8_8_8_8 + DXT5, PS3 DDS uncompressed + DXT5, division logos) with every other member/trailer/tail verbatim, plus the refusals (RW4 families, a display-crop package, an unmeasured XPR2 width) |
| `app/src/components/TextureReplace.tsx` | `c02badeb8918a9b6` | the resize note states the supported scope and the runtime question |

## 3. Measured facts (developer evidence; not in the research handoffs)

- **Apt geometry record (`<n+1>`, 288 bytes), all 141 packages:** header words `288, 1, 0, 16, 272, 1,
  0, 16, 256, 1, 0, 0, 0xFFFFFFFF, <n>`; a 2×2 identity at +64; **six vertices at +96, 32-byte stride:
  `(0,H) (0,0) (W,0) (W,0) (W,H) (0,H)`** (x, y as big-endian floats; the remaining six words of each
  vertex zero). Research 16 §2 described "four corner slots"; the measured record is a two-triangle
  quad of six vertices.
- **Member 0 (`Apt Data:1:5:4`), four length variants (240 / 256 / 264 / 440 B):** the word at +16 is the
  offset of its object table (28 / 44 / 52 / 228); the quad size is stored at **table + 20 as two
  integers and at table + 96 as two floats**, equal to the record's `(W, H)` on every package.
- **Quad vs texture:** 94 packages draw the texture 1:1 (every league-logo family of NHL 08–Legacy on
  both consoles, `divisionlogos` 08–Legacy, `mudleaguelogos` on Xbox); 47 draw a smaller or even
  **transposed** quad (`trophies` 225×375 on a 376×228 texture; `trophiesbroadcast` 175×162 on 256×256;
  `mudleaguelogos` 128×128 on 256×256 on PS3; NHL 07 `leaguelogos` 100×100 on 100×104,
  `leaguelogoswide` 512×247 on 512×248, `divisionlogos` 557×401 on 560×408). The texture-coordinate
  rule of those is not established → resize refused there.
- **XPR2 8_8_8_8 descriptor:** DW7 byte 0 = `0x80 | width/128` as for DXT; DW7 byte 1 = `0x80` on the
  64-wide slots, `0x00` on the 256-wide ones (unknown meaning; no other 8_8_8_8 width is shipped, so
  widths other than 64/256 are refused); DW9 = `00, height/8 − 1, 0xE000 | (width − 1)`; `data_size`
  = w × h × 4.
- **The 16-byte package tail is unchanged by a resize** (it is a per-family constant independent of
  the texture — receipt 02 §3); whether the loader keys anything on it is unknown.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-domain --test league_logo_packages -- --ignored apt_geometry dimension_change` | **2 passed**: 141/141 packages carry the measured layout (94 1:1 / 47 smaller); resizes on 6 packages × 2 sizes decode at the new size with the Apt words rewritten and everything else verbatim; refusals as stated |
| `cargo test -p tdb-rx2` | 112 passed (26 ignored) — unchanged |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-api --test loose_texture_route -- --ignored` | 4 passed (the same-size sweeps: 77 + 34 + 64 + 22, unchanged behaviour) |
| `cargo test -p studio-api --test asset_member_route --test gen7_asset_member_route --test big4_asset_member_route --test registry` | all passed (default cases) |
| `cd app && npx tsc --noEmit -p . && npx vitest run src/components/TextureReplace.test.tsx` | clean / 6 passed |

## 5. Coverage (matrix 1.4 "replace with different dimensions")

| X360 07–11 (RW4xb2) | X360 12–Legacy / Recomp (XPR2) | PS3 08–11 (RW4ps3) | PS3 12–Legacy (DDS) | PS4 15 (PNG) | PS4 16–21 (GNF) |
|---|---|---|---|---|---|
| **blocked** — the Xenos texlib descriptor is not regenerated (same-size only) | impl+tested at widths 64 / 256 (8_8_8_8) and any 4-aligned size (DXT5), 1:1-quad families | **blocked** — RW4ps3 slot/descriptor not regenerated | impl+tested (any size; 1:1-quad families) | impl (PNG carries its own size; registry claim from receipt 06) — not separately tested here | **blocked** — GNF header/tiling not regenerated |

Display-crop families (`trophies`, `trophiesbroadcast`, `mudleaguelogos` on PS3, NHL 07): refused.
Archive placement of a resized package: through C3/C4 (the member length changes; re-plan + stager).

## 6. Limitations / open

- Runtime acceptance of a resized quad (and of DW7 byte 1's meaning for the 64↔256 change) — user card.
- Only widths 64 and 256 for Xbox 8_8_8_8; DXT5 resizes reuse the portrait descriptor rule.
- The PS4 PNG dimension change is not exercised by a dedicated test (the PNG branch has no size gate).

## 7. Temporary artifacts / processes

Scratchpad `c6_*.py` probes/edit scripts — disposable. Probe test `zz_xpr_probe.rs` created under
`crates/tdb-rx2/tests/` then `crates/studio-domain/tests/` for one measurement and **removed** (verified
absent). No temp dirs left (`nhlms-logos-*` removed by the tests).

## 8. Runnable commands

```sh
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test league_logo_packages -- --ignored apt_geometry dimension_change
cargo test -p tdb-rx2 --lib xpr
```
