# Handoff R132 (P28) — League 1.4 / Teams 2.4 different dimensions on the X360 NHL 08 → 11 logo packages: the Apt quad rule of NHL 12 → Legacy holds unchanged, and the RW4xb2 ARGB8 descriptor follows one measured rule on 6 926 / 6 926 rasters

Research lane, 27 September 2026, published 06:05 MDT (system clock). The developer ledger's open item "RW4-in-BIGF: whether the loose RW4
descriptor rule + the Apt quad rewrite apply inside the 07–11 package form is **unmeasured** (research)" (current-authority table, row 1.4 /
2.4; receipt 13 of 22 Sep refuses every 07–11 target as "same-size only (package form)"). Claim `research-p28-old-logo-resize-inputs`.
Read-only over the Xbox 360 loose trees of NHL 08, 09, 10, 11 (NHL 12 as the control); Studio's rule read from `assets.rs`
(`rewrite_apt_geometry`, the loose texlib replace). Nothing written or re-encoded.

Machine-readable: [`data/r132-old-logo-package-resize-inputs.json`](data/r132-old-logo-package-resize-inputs.json) (per title and family:
wrappers, texture forms, Apt classes, descriptors by size, samples), [`data/r132b-rw4-argb8-descriptor-rule.json`](data/r132b-rw4-argb8-descriptor-rule.json)
(every RW4 raster of every `fe/ion/artassets` family, NHL 08 → 11).

## Plain-language summary

Studio can already give a team or league logo a new size on NHL 12 and later; on NHL 07–11 it refuses, because nobody had checked whether the
older packages are built the same way. They are. The small geometry record that tells the game how big to draw the logo sits in the same
place with the same layout, and on 3 530 of 4 419 logo packages it spans the whole picture, exactly the case Studio already handles; the rest
are cropped displays (every calendar logo) that Studio refuses on the newer titles too. The pictures themselves are stored uncompressed in the
older texture format, whose size fields follow one simple rule on every one of the 6 926 pictures measured. So a resize on these titles needs
two developer steps — that texture rule, and re-wrapping the compression some titles use — plus the usual in-game check.

## 1. Apt geometry (Studio's 12 → Legacy rule applied unchanged)

Rule (from `assets.rs::rewrite_apt_geometry`): the `<n+1>` member of 288 bytes carries six vertices at +96 (stride 32) = (0,H) (0,0) (W,0)
(W,0) (W,H) (0,H) for the texture's (W, H), and member `0` (`Apt Data:`) carries W, H as u32 at table +20 and f32 at table +96 (table =
16 + u32 @ 16).

| Family (X360) | NHL 08 | NHL 09 | NHL 10 | NHL 11 |
|---|---|---|---|---|
| team logos | 134 full / 1 crop / 1 size-word differs (of 136) | 216 / 1 / 1 (218) | 215 / 1 / 1 (217) | 297 / 1 (298) |
| small | 135 / 0 | 217 / 0 | 216 / 0 | 297 / 0 |
| wide | 132 / 3 | 214 / 3 | 219 / 3 | 297 / 0 |
| dynasty | 132 / 1 | 215 / 1 | 214 / 1 | 297 / 1 |
| calendar | 0 / 135 (all crops) | 0 / 217 | 0 / 216 | 0 / 297 |
| league logos / small / wide | 6 / 1 · 1 · 2 | 10 / 1 · 1 · 2 | 11 / 1 · 1 · 10 | 13 / 1 · 12 · 14 |

Totals NHL 08 → 11: **3 530 full-texture quads**, 886 display-crop quads (every calendar package plus a few per family — the same pattern
as receipt 13 on 12 → Legacy), 3 packages whose quad fits but member `0`'s size words differ (refuse). On NHL 11 member `0` is sometimes
RefPack-wrapped (unwrapped before the check; 172 packages).

## 2. Containers and wrappers

| Title | Package file | Texture member | Member `0` |
|---|---|---|---|
| NHL 08 | **whole-file RefPack** → BIGF | stored RW4xb2 | stored |
| NHL 09 / 10 | plain BIGF | stored RW4xb2 | stored |
| NHL 11 | plain BIGF | **RefPack** → RW4xb2 | stored or RefPack |
| NHL 12 (control) | plain BIGF | RefPack → XPR2 | — (Studio's resize route) |

Every texture member is one raster: ARGB8 (`0x86`) on every logo family except the small league logo (DXT5).

## 3. RW4xb2 descriptor rule (0x38-byte `0x20003` descriptor; X360; every front-end family, NHL 08 → 11)

| Field | ARGB8 (6 926 rasters, 73 sizes) | DXT5 (1 638, 58 sizes) |
|---|---|---|
| DW2 at +36 | `(w − 1) \| (h − 1) << 13` (Xenos packing) — all | same |
| pitch = DW0 at +28, bits 22–30 × 32 texels | **= align(w, 32) on 6 926 / 6 926** (a power-of-two reading fails on 311, e.g. 208 → 224, 400 → 416) | = align(w, 128) on the discriminating sizes listed (376 → 384, 388 → 512, 1 832 → 1 920) |
| payload (`0x10031` size) | **= pitch × align(h, 32) × 4 on 6 926 / 6 926** (e.g. 100 × 88 → 128 × 96 × 4) | not tested here |
| all other descriptor bytes | **one variant** for the format (constant) | one variant |

So for ARGB8 a resized descriptor is: keep every byte, set the pitch bits of DW0 to align(w, 32) / 32, set DW2 to (w−1) | (h−1) << 13, and
size the payload chunk to align(w, 32) × align(h, 32) × 4 (the RW4 file table's chunk size and file size follow, as in Studio's loose DXT
resize). The texel order inside the payload is whatever Studio's existing same-size ARGB8 writer uses (not re-derived here). Studio's loose
route refuses ARGB8 resizes today ("uncompressed (8_8_8_8) raster is same-size only"); this rule is the missing evidence for that case.

## 4. Developer-consumable contract input (not a writer; runtime acceptance separate)

1. ARGB8 RW4xb2 resize per §3 (static rule; verify with a decode → re-open round trip like the DXT route).
2. Package route for NHL 08 → 11: apply the existing Apt rewrite (§1) unchanged; refuse display-crop quads and size-word mismatches as on
   12 → Legacy; re-wrap per §2 (whole-file RefPack on NHL 08; texture-member RefPack on NHL 11; member `0` in its own wrapper).
3. Card: resized team logo on one NHL 10 and one NHL 11 package in game (the receipt-13 card T-3 covers which screen draws which family).

Not measured: PS3 NHL 08 → 11 (`RW4ps3`), X360 NHL 07 (root `t*.big` packages), archive-held copies of these packages.

```
python -B local/runs/overnight-research-2026-09-26/probes/r132_old_logo_package_resize_inputs.py <out.json>   (≈ 1 min)
python -B local/runs/overnight-research-2026-09-26/probes/r132b_rw4_argb8_descriptor_rule.py <out.json>        (≈ 1.5 min)
```
Executed: R132 three times (member-0 unwrap and descriptor capture added), R132b twice (the first read the pitch from the wrong word and
was discarded).

## 5. Addendum 06:08 — PS3 NHL 08 → 11 (R132c, [`data/r132c-ps3-old-logo-packages.json`](data/r132c-ps3-old-logo-packages.json))

Same families and the same Apt check on the PS3 trees (Studio already resizes loose `RW4ps3` rasters; receipt 13 refuses the package form):

| Title | Package location | Texture member | Apt classes (all families) |
|---|---|---|---|
| PS3 NHL 08 | members of `data0.big` (BIG4; no `fe.big`), plain BIGF | stored `RW4ps3`, ARGB8 (league small DXT5), 1 mip | full quad on every package except the calendar family (all crops), 6 other crops (team 1, wide 3, dynasty 1, league 1), 1 size-word mismatch |
| PS3 NHL 09 / 10 | members of `fe.big` (BIG4), plain BIGF | **RefPack** → `RW4ps3` ARGB8, 1 mip | same pattern |
| PS3 NHL 11 | loose `fe/ion/artassets/<family>/*.big`, plain BIGF | **RefPack** → `RW4ps3` ARGB8, 1 mip | same pattern |

Totals PS3 NHL 08 → 11: **3 533 full-texture quads**, 883 display crops, 3 size-word mismatches — the Xbox 360 figures within a few packages.
So the package route's refusal on PS3 08 → 11 is also a *form* question (archive member re-placement in BIG4 + member RefPack), not an Apt
one; the `RW4ps3` resize itself is Studio's existing loose route.

## 6. Addendum 06:10 — Xbox 360 NHL 07 (R132d, [`data/r132d-x360-nhl07-logo-packages.json`](data/r132d-x360-nhl07-logo-packages.json))

NHL 07's loose `fe/ion/artassets/<family>/*.big` are plain BIGF with a **stored** single-raster `RW4xb2` ARGB8 texture (on the loose tree; receipt 13's RefPack'd-package wording may describe
archive-held or fixture copies, which were not compared). Apt classes: team logos 56 full / 1 crop (57),
small 56 / 0, league small 1 / 0; **wide 0 / 56, calendar 0 / 56, league logos 0 / 2, league wide 0 / 2 are display crops** — NHL 07's wide
family is a crop, unlike NHL 08 → Legacy. Some NHL 07 texture members carry no `0x10031` payload chunk in their file table (their payload
chunk type was not resolved here) — a resize writer must refuse those until their layout is read.
