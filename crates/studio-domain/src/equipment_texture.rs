//! Zone-mask **equipment texture** compositor — turns a glove/helmet's zone
//! masks plus its `hHbv` zone colours into a finished diffuse for the 3D viewer.
//!
//! Ported from the reference app's `equipment_preview` (Heck DB Editor v1.2's
//! `_build_colored_texture`). NHL doesn't ship flat glove/helmet diffuses: each
//! `_tm` file is a **zone mask** whose R/G/B channels flag which of up to 9 body
//! zones a pixel belongs to, and the team's `hHbv` record supplies the colour for
//! each zone. We paint by **channel dominance**:
//!
//! ```text
//! for mask file f, channel c ∈ {R,G,B}:  zone = f*3 + c
//! pixel := zone_colors[zone]  where mask.channel[c] > 128 AND the other two < 64
//! ```
//!
//! An optional detail map (`_dm`) is alpha-blended on top. The helmet's
//! UV2-corrected sampling (reference app) is out of scope here — our pinned
//! helmet mesh ships no UV2 — so the helmet uses the standard path with its main
//! colour as the background (rather than transparent), which keeps it from
//! reading as mostly-black where the single mask leaves gaps.

use std::path::{Path, PathBuf};

use crate::assets::{decode_texture_file, DecodedImage};
use crate::head_preview::{base64_encode, PreviewTexture};

const HIGH: u8 = 128;
const LOW: u8 = 64;

/// Nearest-sample the source pixel at canvas `(x, y)` when a mask/detail differs
/// in size from the canvas (in practice a set is one resolution — this is safety).
#[inline]
fn sample(src: &DecodedImage, x: u32, y: u32, cw: u32, ch: u32) -> [u8; 4] {
    let sx = if src.width == cw { x } else { x * src.width / cw };
    let sy = if src.height == ch { y } else { y * src.height / ch };
    let i = ((sy * src.width + sx) * 4) as usize;
    [src.rgba[i], src.rgba[i + 1], src.rgba[i + 2], src.rgba[i + 3]]
}

/// Channel-dominance zone paint. Canvas size = the first mask. `default_bg`
/// pre-fills every pixel (helmet main colour); `None` leaves it transparent
/// black (gloves), matching the reference app.
fn composite_zone_masks(
    masks: &[DecodedImage],
    zones: &[[u8; 3]],
    default_bg: Option<[u8; 3]>,
) -> Option<DecodedImage> {
    let first = masks.first()?;
    let (w, h) = (first.width, first.height);
    let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
    if let Some([r, g, b]) = default_bg {
        for px in out.chunks_exact_mut(4) {
            px[0] = r;
            px[1] = g;
            px[2] = b;
            px[3] = 255;
        }
    }
    for (fi, mask) in masks.iter().enumerate() {
        for y in 0..h {
            for x in 0..w {
                let [r, g, b, _] = sample(mask, x, y, w, h);
                let ch = [r, g, b];
                for c in 0..3 {
                    let dominant =
                        ch[c] > HIGH && (0..3).all(|k| k == c || ch[k] < LOW);
                    if !dominant {
                        continue;
                    }
                    if let Some(&[zr, zg, zb]) = zones.get(fi * 3 + c) {
                        let oi = ((y * w + x) * 4) as usize;
                        out[oi] = zr;
                        out[oi + 1] = zg;
                        out[oi + 2] = zb;
                        out[oi + 3] = 255;
                    }
                }
            }
        }
    }
    Some(DecodedImage {
        width: w,
        height: h,
        rgba: out,
    })
}

/// Alpha-blend a detail/material map over the zone-coloured base (source-over).
fn blend_detail(base: &mut DecodedImage, detail: &DecodedImage) {
    let (w, h) = (base.width, base.height);
    for y in 0..h {
        for x in 0..w {
            let [dr, dg, db, da] = sample(detail, x, y, w, h);
            if da == 0 {
                continue;
            }
            let a = da as f32 / 255.0;
            let oi = ((y * w + x) * 4) as usize;
            for (c, &dv) in [dr, dg, db].iter().enumerate() {
                base.rgba[oi + c] =
                    (dv as f32 * a + base.rgba[oi + c] as f32 * (1.0 - a)).round() as u8;
            }
            base.rgba[oi + 3] = base.rgba[oi + 3].max(da);
        }
    }
}

/// Build one equipment piece's diffuse: decode its zone masks, paint them with
/// `zones` (RGB per zone), blend the optional detail map, and hand back a
/// [`PreviewTexture`] ready for the kit viewer. `None` when no mask decodes.
pub fn composite_equipment_texture(
    mask_paths: &[PathBuf],
    detail_path: Option<&Path>,
    zones: &[[u8; 3]],
    name: &str,
    default_bg: Option<[u8; 3]>,
) -> Option<PreviewTexture> {
    let masks: Vec<DecodedImage> = mask_paths
        .iter()
        .filter_map(|p| decode_texture_file(p, None).ok().map(|(_, img)| img))
        .collect();
    let mut base = composite_zone_masks(&masks, zones, default_bg)?;
    if let Some(dp) = detail_path {
        if let Ok((_, detail)) = decode_texture_file(dp, None) {
            blend_detail(&mut base, &detail);
        }
    }
    Some(PreviewTexture {
        name: name.to_string(),
        width: base.width,
        height: base.height,
        rgba_base64: base64_encode(&base.rgba),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(w: u32, h: u32, px: [u8; 4]) -> DecodedImage {
        DecodedImage {
            width: w,
            height: h,
            rgba: px.repeat((w * h) as usize),
        }
    }

    #[test]
    fn channel_dominant_pixel_takes_its_zone_color() {
        // A pure-red mask → every pixel is zone 0 (file 0, channel R).
        let masks = [mask(2, 2, [255, 0, 0, 255])];
        let zones = [[10, 20, 30], [0, 0, 0], [0, 0, 0]];
        let out = composite_zone_masks(&masks, &zones, None).unwrap();
        assert_eq!(&out.rgba[0..4], &[10, 20, 30, 255]);
    }

    #[test]
    fn non_dominant_pixel_stays_background() {
        // A grey mask (no channel dominates) → keeps the default background.
        let masks = [mask(1, 1, [100, 100, 100, 255])];
        let zones = [[10, 20, 30]];
        let transparent = composite_zone_masks(&masks, &zones, None).unwrap();
        assert_eq!(&transparent.rgba[0..4], &[0, 0, 0, 0]);
        let filled = composite_zone_masks(&masks, &zones, Some([7, 8, 9])).unwrap();
        assert_eq!(&filled.rgba[0..4], &[7, 8, 9, 255]);
    }

    #[test]
    fn second_mask_maps_to_zones_three_through_five() {
        // File index 1, green channel → zone 1*3 + 1 = 4.
        let masks = [mask(1, 1, [0, 0, 0, 255]), mask(1, 1, [0, 255, 0, 255])];
        let mut zones = [[0, 0, 0]; 6];
        zones[4] = [90, 91, 92];
        let out = composite_zone_masks(&masks, &zones, None).unwrap();
        assert_eq!(&out.rgba[0..4], &[90, 91, 92, 255]);
    }

    #[test]
    fn legacy_x360_real_glove_masks_composite() {
        let gdir = std::path::Path::new(
            r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering\glove",
        );
        if !gdir.is_dir() {
            eprintln!("skipping: {} not present", gdir.display());
            return;
        }
        let masks: Vec<std::path::PathBuf> =
            (0..3).map(|i| gdir.join(format!("feglove_15_{i}_tm.rx2"))).collect();
        if !masks.iter().all(|p| p.is_file()) {
            eprintln!("skipping: feglove_15 mask set incomplete");
            return;
        }
        // Nine distinct, saturated zone colours so each painted zone is findable.
        let zones: Vec<[u8; 3]> = (0..9).map(|i| [(i * 28) as u8, 200, 40]).collect();
        let tex = composite_equipment_texture(&masks, None, &zones, "glove_cm", None)
            .expect("real glove masks composite");
        assert!(tex.width >= 64 && tex.height >= 64, "sane dims {}x{}", tex.width, tex.height);
        // Decode the base64 back and confirm at least one zone colour was painted
        // (the masks are real, so channel-dominant regions must exist).
        let raw = {
            // minimal base64 decode for the assertion
            let t: Vec<u8> = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
                .iter()
                .copied()
                .collect();
            let inv = |c: u8| t.iter().position(|&x| x == c);
            let bytes: Vec<u8> = tex.rgba_base64.bytes().filter(|&b| b != b'=').collect();
            let mut out = Vec::new();
            for chunk in bytes.chunks(4) {
                let mut n = 0u32;
                let mut k = 0;
                for &c in chunk {
                    if let Some(v) = inv(c) {
                        n = (n << 6) | v as u32;
                        k += 1;
                    }
                }
                n <<= 6 * (4 - k);
                for j in 0..(k - 1) {
                    out.push((n >> (16 - j * 8)) as u8);
                }
            }
            out
        };
        let painted = raw.chunks_exact(4).any(|px| px[3] == 255);
        assert!(painted, "at least one zone was painted onto the glove");
    }

    #[test]
    fn detail_overlay_alpha_blends() {
        let mut base = mask(1, 1, [0, 0, 0, 255]);
        let detail = mask(1, 1, [200, 0, 0, 128]); // ~50% red
        blend_detail(&mut base, &detail);
        assert!((base.rgba[0] as i32 - 100).abs() <= 1, "got {}", base.rgba[0]);
    }
}
