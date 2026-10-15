//! Head-UV fingerprinting — the measurement side of the UV-compatibility spike (HM-S1).
//!
//! The working hypothesis ("NHL 11 → NHL 24 share one head UV atlas; pre-NHL11 differs")
//! is unconfirmed in either repo, so the conversion matrix consumes **measured UV
//! classes** only. This module produces a per-submesh fingerprint that is comparable
//! across container formats and vertex orderings:
//!
//! * counts (vertices, triangles, unique quantized UVs) and the UV bounding box;
//! * an **order-independent multiset hash** of UVs quantized to a 1/2048 grid — two
//!   meshes whose UV sets are identical up to reordering hash equal even when their
//!   vertex order (or container) differs;
//! * per-vertex UV distance stats when two fingerprints share a vertex count and
//!   ordering provenance.
//!
//! Findings land in `docs/audits/headmesh-uv-compat.md` and
//! `defs/appearance/head-uv-classes.toml`; the raw fingerprints are the evidence.

use serde::{Deserialize, Serialize};

/// UVs are quantized to this many steps per axis before hashing — coarse enough to
/// absorb u16-storage rounding across containers, fine enough to distinguish atlases.
pub const QUANT_STEPS: f32 = 2048.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UvFingerprint {
    /// Where this came from (file path, for the audit trail).
    pub source: String,
    pub submesh: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub unique_uv_count: usize,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    /// FNV-1a 64 over the sorted quantized UV multiset (order-independent).
    pub uv_multiset_hash: String,
}

pub fn fingerprint(
    source: &str,
    submesh: &str,
    uvs: &[[f32; 2]],
    triangle_count: usize,
) -> UvFingerprint {
    let mut quantized: Vec<[u32; 2]> = uvs
        .iter()
        .map(|uv| {
            [
                (uv[0].clamp(0.0, 1.0) * QUANT_STEPS).round() as u32,
                (uv[1].clamp(0.0, 1.0) * QUANT_STEPS).round() as u32,
            ]
        })
        .collect();
    quantized.sort_unstable();
    let unique = {
        let mut u = quantized.clone();
        u.dedup();
        u.len()
    };

    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for q in &quantized {
        for axis in q {
            for byte in axis.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }

    let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
    for uv in uvs {
        for a in 0..2 {
            min[a] = min[a].min(uv[a]);
            max[a] = max[a].max(uv[a]);
        }
    }
    if uvs.is_empty() {
        min = [0.0; 2];
        max = [0.0; 2];
    }

    UvFingerprint {
        source: source.to_owned(),
        submesh: submesh.to_owned(),
        vertex_count: uvs.len(),
        triangle_count,
        unique_uv_count: unique,
        uv_min: min,
        uv_max: max,
        uv_multiset_hash: format!("{hash:016x}"),
    }
}

/// Verdict of comparing two fingerprints.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum UvMatch {
    /// Same quantized UV multiset — same atlas layout (up to vertex reordering).
    Identical,
    /// Same vertex count but differing sets; carries mean/max quantized-grid distance of
    /// the sorted pairing as a rough drift magnitude.
    SameTopologyDrifted { mean_steps: f64, max_steps: u32 },
    /// Different vertex counts — different topologies; atlas identity undecidable by
    /// hash alone (needs the visual wireframe pass).
    DifferentTopology,
}

pub fn compare(a: &UvFingerprint, b: &UvFingerprint) -> UvMatch {
    if a.uv_multiset_hash == b.uv_multiset_hash && a.vertex_count == b.vertex_count {
        return UvMatch::Identical;
    }
    if a.vertex_count != b.vertex_count {
        return UvMatch::DifferentTopology;
    }
    UvMatch::SameTopologyDrifted {
        mean_steps: f64::NAN, // filled by the raw-UV comparison below when data is at hand
        max_steps: 0,
    }
}

/// Distance stats between two same-length raw UV sets, compared as sorted quantized
/// multisets (order-independent).
pub fn drift_stats(a: &[[f32; 2]], b: &[[f32; 2]]) -> Option<(f64, u32)> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let quant = |uvs: &[[f32; 2]]| {
        let mut q: Vec<[i64; 2]> = uvs
            .iter()
            .map(|uv| {
                [
                    (uv[0].clamp(0.0, 1.0) * QUANT_STEPS).round() as i64,
                    (uv[1].clamp(0.0, 1.0) * QUANT_STEPS).round() as i64,
                ]
            })
            .collect();
        q.sort_unstable();
        q
    };
    let (qa, qb) = (quant(a), quant(b));
    let mut sum = 0.0f64;
    let mut max = 0i64;
    for (pa, pb) in qa.iter().zip(qb.iter()) {
        let d = (pa[0] - pb[0]).abs().max((pa[1] - pb[1]).abs());
        sum += d as f64;
        max = max.max(d);
    }
    Some((sum / qa.len() as f64, max as u32))
}

/// Side length of the UV coverage raster used for cross-topology atlas comparison.
pub const COVERAGE_N: usize = 256;

/// Rasterize the UV triangles into a `COVERAGE_N`² occupancy bitmask. Two heads with
/// the same atlas *layout* cover (nearly) the same texels even when their topologies
/// differ — which the multiset hash cannot see.
pub fn coverage_mask(uvs: &[[f32; 2]], triangles: &[[u32; 3]]) -> Vec<u64> {
    let n = COVERAGE_N;
    let mut mask = vec![0u64; (n * n).div_ceil(64)];
    let mut set = |x: usize, y: usize| {
        let bit = y * n + x;
        mask[bit / 64] |= 1u64 << (bit % 64);
    };
    let to_px = |v: f32| (v.clamp(0.0, 1.0) * (n as f32 - 1.0)).round() as i64;
    for tri in triangles {
        let Some([a, b, c]) = tri
            .iter()
            .map(|&i| uvs.get(i as usize).copied())
            .collect::<Option<Vec<_>>>()
            .map(|v| [v[0], v[1], v[2]])
        else {
            continue;
        };
        let (ax, ay) = (to_px(a[0]), to_px(a[1]));
        let (bx, by) = (to_px(b[0]), to_px(b[1]));
        let (cx, cy) = (to_px(c[0]), to_px(c[1]));
        let (min_x, max_x) = (ax.min(bx).min(cx), ax.max(bx).max(cx));
        let (min_y, max_y) = (ay.min(by).min(cy), ay.max(by).max(cy));
        // Scanline point-in-triangle over the bounding box (edge functions).
        let edge = |px: i64, py: i64, x0: i64, y0: i64, x1: i64, y1: i64| {
            (x1 - x0) * (py - y0) - (y1 - y0) * (px - x0)
        };
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let e0 = edge(px, py, ax, ay, bx, by);
                let e1 = edge(px, py, bx, by, cx, cy);
                let e2 = edge(px, py, cx, cy, ax, ay);
                if (e0 >= 0 && e1 >= 0 && e2 >= 0) || (e0 <= 0 && e1 <= 0 && e2 <= 0) {
                    set(px as usize, py as usize);
                }
            }
        }
    }
    mask
}

/// Fraction of `a`'s set texels lying within Chebyshev radius `radius` of a set texel
/// of `b`. Raw IoU saturates: a 1-texel border jitter (u16-vs-f32 UV storage) already
/// costs ~7% IoU on a head atlas, indistinguishable from a genuinely reworked layout.
/// With a small tolerance radius, same-layout-with-jitter reads ≈ 1.0 while moved or
/// re-cut islands still fall out.
pub fn coverage_within(a: &[u64], b: &[u64], radius: usize) -> f64 {
    let n = COVERAGE_N;
    let get = |mask: &[u64], x: usize, y: usize| {
        let bit = y * n + x;
        mask[bit / 64] >> (bit % 64) & 1 == 1
    };
    // Two-pass Chebyshev dilation of b (rows then columns).
    let mut rows = vec![false; n * n];
    for y in 0..n {
        for x in 0..n {
            let lo = x.saturating_sub(radius);
            let hi = (x + radius).min(n - 1);
            rows[y * n + x] = (lo..=hi).any(|xx| get(b, xx, y));
        }
    }
    let mut dilated = vec![false; n * n];
    for y in 0..n {
        let lo = y.saturating_sub(radius);
        let hi = (y + radius).min(n - 1);
        for x in 0..n {
            dilated[y * n + x] = (lo..=hi).any(|yy| rows[yy * n + x]);
        }
    }
    let mut total = 0u64;
    let mut hit = 0u64;
    for y in 0..n {
        for x in 0..n {
            if get(a, x, y) {
                total += 1;
                if dilated[y * n + x] {
                    hit += 1;
                }
            }
        }
    }
    if total == 0 {
        return 0.0;
    }
    hit as f64 / total as f64
}

/// Symmetric tolerant agreement: the worse direction of [`coverage_within`].
pub fn coverage_agreement(a: &[u64], b: &[u64], radius: usize) -> f64 {
    coverage_within(a, b, radius).min(coverage_within(b, a, radius))
}

/// Intersection-over-union of two coverage masks. Same atlas layout ⇒ near 1.0;
/// unrelated layouts land far lower.
pub fn coverage_iou(a: &[u64], b: &[u64]) -> f64 {
    let mut inter = 0u64;
    let mut union = 0u64;
    for (wa, wb) in a.iter().zip(b.iter()) {
        inter += (wa & wb).count_ones() as u64;
        union += (wa | wb).count_ones() as u64;
    }
    if union == 0 {
        return 0.0;
    }
    inter as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_iou_separates_same_vs_disjoint_layouts() {
        // One triangle in the lower-left quadrant.
        let uvs_a = vec![[0.05, 0.05], [0.45, 0.05], [0.05, 0.45]];
        // Same triangle, retopologized (extra vertex, two triangles).
        let uvs_b = vec![[0.05, 0.05], [0.45, 0.05], [0.05, 0.45], [0.25, 0.05]];
        // Same-shape triangle moved to the upper-right quadrant.
        let uvs_c = vec![[0.55, 0.55], [0.95, 0.55], [0.55, 0.95]];
        let tri = vec![[0u32, 1, 2]];
        let tri_b = vec![[0u32, 3, 2], [3, 1, 2]];
        let ma = coverage_mask(&uvs_a, &tri);
        let mb = coverage_mask(&uvs_b, &tri_b);
        let mc = coverage_mask(&uvs_c, &tri);
        assert!(
            coverage_iou(&ma, &mb) > 0.95,
            "retopologized same layout ≈ 1"
        );
        assert!(coverage_iou(&ma, &mc) < 0.05, "disjoint layout ≈ 0");
        assert_eq!(coverage_iou(&ma, &ma), 1.0);
    }

    #[test]
    fn hash_is_order_independent_and_layout_sensitive() {
        let uvs_a = vec![[0.1, 0.2], [0.5, 0.5], [0.9, 0.1]];
        let uvs_b = vec![[0.9, 0.1], [0.1, 0.2], [0.5, 0.5]]; // reordered
        let uvs_c = vec![[0.1, 0.2], [0.5, 0.5], [0.9, 0.15]]; // moved
        let fa = fingerprint("a", "head_", &uvs_a, 1);
        let fb = fingerprint("b", "head_", &uvs_b, 1);
        let fc = fingerprint("c", "head_", &uvs_c, 1);
        assert_eq!(fa.uv_multiset_hash, fb.uv_multiset_hash);
        assert_ne!(fa.uv_multiset_hash, fc.uv_multiset_hash);
        assert_eq!(compare(&fa, &fb), UvMatch::Identical);
        assert!(matches!(
            compare(&fa, &fc),
            UvMatch::SameTopologyDrifted { .. }
        ));
        let (mean, max) = drift_stats(&uvs_a, &uvs_c).unwrap();
        assert!(mean > 0.0);
        assert!(max >= 100); // 0.05 * 2048 ≈ 102 steps
    }

    #[test]
    fn different_counts_are_different_topology() {
        let fa = fingerprint("a", "head_", &[[0.1, 0.2]], 0);
        let fb = fingerprint("b", "head_", &[[0.1, 0.2], [0.3, 0.4]], 0);
        assert_eq!(compare(&fa, &fb), UvMatch::DifferentTopology);
    }
}
