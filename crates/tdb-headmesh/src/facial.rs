//! Facial semantics for the weight transfer — named palette buckets and the mouth crease.
//!
//! A pure closest-point transfer is anatomically blind, and two places on a face punish
//! that badly. Both were reported from in-game footage and both are fixed here:
//!
//! * **The lips.** In the bind pose the mouth is *closed*, so an upper-lip vertex and a
//!   lower-lip vertex are neighbours in space while belonging to opposite halves of the
//!   rig. Euclidean proximity happily sources one from the other, and the result is a
//!   mouth that stays sealed and stretches when the jaw drops.
//! * **The eyelids.** The eyelid bones drive a narrow band of skin. If brow vertices pick
//!   up eyelid weight, a blink moves the brow instead of the lid.
//!
//! The counters are the ones the prior effort arrived at: classify by anatomy, forbid the
//! crossings that are never legitimate, and mask the forbidden buckets off again after
//! interpolation (a donor vertex near the seam can itself carry a little of the other
//! side's weight, so gating the *source* is not sufficient on its own).

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum FacialError {
    #[error("failed to read facial regions: {0}")]
    Io(#[from] std::io::Error),
    #[error("facial regions are not valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("facial regions define no {0:?} bucket — the transfer masks would be silently inert")]
    MissingBucket(&'static str),
}

#[derive(Debug, serde::Deserialize)]
struct RegionsFile {
    legacy: LegacySection,
}

#[derive(Debug, serde::Deserialize)]
struct LegacySection {
    palette_buckets: BTreeMap<String, Vec<u32>>,
}

/// Named palette buckets from `defs/appearance/facial-regions.toml`.
#[derive(Debug, Clone)]
pub struct FacialBuckets {
    map: BTreeMap<String, Vec<u32>>,
}

/// Buckets that must exist for the masks to mean anything. Validated at load so a
/// renamed bucket fails loudly instead of turning every mask into a no-op.
const REQUIRED: [&str; 4] = ["upper_lip", "lower_lip", "tongue", "jaw"];

impl FacialBuckets {
    pub fn load(regions_toml: &Path) -> Result<FacialBuckets, FacialError> {
        let file: RegionsFile = toml::from_str(&std::fs::read_to_string(regions_toml)?)?;
        let b = FacialBuckets {
            map: file.legacy.palette_buckets,
        };
        for name in REQUIRED {
            if b.get(name).is_empty() {
                return Err(FacialError::MissingBucket(match name {
                    "upper_lip" => "upper_lip",
                    "lower_lip" => "lower_lip",
                    "tongue" => "tongue",
                    _ => "jaw",
                }));
            }
        }
        Ok(b)
    }

    /// Build from an explicit bucket table (tests, and callers holding their own defs).
    pub fn from_map(map: BTreeMap<String, Vec<u32>>) -> FacialBuckets {
        FacialBuckets { map }
    }

    pub fn get(&self, name: &str) -> &[u32] {
        self.map.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn upper_lip(&self) -> &[u32] {
        self.get("upper_lip")
    }

    pub fn lower_lip(&self) -> &[u32] {
        self.get("lower_lip")
    }

    pub fn tongue(&self) -> &[u32] {
        self.get("tongue")
    }

    /// Every eyelid slot, both lids and both sides.
    pub fn eyelid_slots(&self) -> Vec<u32> {
        let mut out = Vec::new();
        for name in [
            "upper_eyelid_l",
            "upper_eyelid_r",
            "lower_eyelid_l",
            "lower_eyelid_r",
        ] {
            out.extend_from_slice(self.get(name));
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Which half of the mouth a vertex belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LipSide {
    Upper,
    Lower,
    /// Outside the mouth zone, or no crease measurable — no lip rule applies.
    Neutral,
}

/// The mouth-opening boundary loop, used as the upper/lower divider.
#[derive(Debug, Clone)]
pub struct MouthCrease {
    points: Vec<[f32; 3]>,
    center: [f32; 3],
    /// How far from the crease the lip rules still apply.
    zone_radius: f32,
}

/// Vertices within this distance of the mouth loop are subject to the lip rules. Wide
/// enough to cover the lip body and the rolled inner lip, tight enough to leave the chin
/// and cheeks alone.
pub const DEFAULT_LIP_ZONE_M: f32 = 0.025;

impl MouthCrease {
    pub fn new(points: Vec<[f32; 3]>, zone_radius: f32) -> Option<MouthCrease> {
        if points.len() < 3 {
            return None;
        }
        let mut c = [0f64; 3];
        for p in &points {
            for k in 0..3 {
                c[k] += f64::from(p[k]);
            }
        }
        let n = points.len() as f64;
        Some(MouthCrease {
            center: [(c[0] / n) as f32, (c[1] / n) as f32, (c[2] / n) as f32],
            points,
            zone_radius,
        })
    }

    /// Column-local crease height: the midpoint of the loop's Y range among points within
    /// ±4 mm of `x`. Using a *local* column rather than one global plane matters because
    /// the lip line curves upward toward the corners — a flat split would misclassify the
    /// corners, which is where the two lips actually meet.
    pub fn crease_y_at(&self, x: f32) -> f32 {
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for p in &self.points {
            if (p[0] - x).abs() <= 0.004 {
                lo = lo.min(p[1]);
                hi = hi.max(p[1]);
            }
        }
        if lo.is_finite() {
            (lo + hi) * 0.5
        } else {
            self.center[1]
        }
    }

    /// Squared distance from `p` to the nearest loop point.
    fn dist2_to_loop(&self, p: [f32; 3]) -> f32 {
        self.points
            .iter()
            .map(|q| (0..3).map(|k| (q[k] - p[k]).powi(2)).sum::<f32>())
            .fold(f32::MAX, f32::min)
    }

    /// Classify a point against the crease, or `Neutral` when it is outside the zone.
    pub fn side_of(&self, p: [f32; 3]) -> LipSide {
        if self.dist2_to_loop(p) > self.zone_radius * self.zone_radius {
            return LipSide::Neutral;
        }
        if p[1] >= self.crease_y_at(p[0]) {
            LipSide::Upper
        } else {
            LipSide::Lower
        }
    }
}

/// Are these two lip sides an illegitimate crossing? Upper never sources Lower and vice
/// versa; anything involving `Neutral` is unconstrained.
pub fn lip_sides_conflict(a: LipSide, b: LipSide) -> bool {
    matches!(
        (a, b),
        (LipSide::Upper, LipSide::Lower) | (LipSide::Lower, LipSide::Upper)
    )
}

/// Palette slots a vertex on `side` must never carry weight on.
pub fn forbidden_slots(side: LipSide, buckets: &FacialBuckets) -> Vec<u32> {
    // Head-surface geometry never rides the tongue, whatever its lip side.
    let mut out: Vec<u32> = buckets.tongue().to_vec();
    match side {
        LipSide::Upper => out.extend_from_slice(buckets.lower_lip()),
        LipSide::Lower => out.extend_from_slice(buckets.upper_lip()),
        LipSide::Neutral => {}
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Drop `forbidden` slots from an influence list and renormalize what remains.
///
/// Returns the weight mass that was removed. If masking would empty the vertex the list
/// is left alone — an unweighted vertex is a worse outcome than a slightly wrong one, and
/// it means the classification, not the weights, is what needs looking at.
pub fn mask_and_renormalize(influences: &mut Vec<(u32, f32)>, forbidden: &[u32]) -> f32 {
    if forbidden.is_empty() {
        return 0.0;
    }
    let removed: f32 = influences
        .iter()
        .filter(|(slot, _)| forbidden.contains(slot))
        .map(|(_, w)| *w)
        .sum();
    if removed <= 0.0 {
        return 0.0;
    }
    let kept: Vec<(u32, f32)> = influences
        .iter()
        .copied()
        .filter(|(slot, _)| !forbidden.contains(slot))
        .collect();
    let total: f32 = kept.iter().map(|(_, w)| *w).sum();
    if kept.is_empty() || total <= 0.0 {
        return 0.0;
    }
    *influences = kept
        .into_iter()
        .map(|(slot, w)| (slot, w / total))
        .collect();
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mouth loop shaped like a real one: corners riding higher than the centre, so a
    /// single global split plane would misclassify them.
    fn curved_loop() -> MouthCrease {
        let mut pts = Vec::new();
        for i in 0..21 {
            let t = (i as f32 / 20.0) * 2.0 - 1.0; // -1..1 across the mouth
            let x = t * 0.025;
            let lift = t * t * 0.006; // corners 6 mm higher
            pts.push([x, 1.767 + lift, 0.097]);
            pts.push([x, 1.763 + lift, 0.097]);
        }
        MouthCrease::new(pts, DEFAULT_LIP_ZONE_M).unwrap()
    }

    #[test]
    fn crease_follows_the_local_column_not_a_global_plane() {
        let c = curved_loop();
        let centre = c.crease_y_at(0.0);
        let corner = c.crease_y_at(0.024);
        assert!(
            corner > centre + 0.004,
            "corner crease {corner} should ride above centre {centre}"
        );
        // A point just above the corner crease is Upper even though it sits BELOW the
        // centre-line lips — the exact case a flat split gets wrong.
        assert_eq!(c.side_of([0.024, corner + 0.001, 0.097]), LipSide::Upper);
        assert_eq!(c.side_of([0.024, corner - 0.001, 0.097]), LipSide::Lower);
    }

    #[test]
    fn points_far_from_the_mouth_are_neutral() {
        let c = curved_loop();
        assert_eq!(c.side_of([0.0, 1.90, 0.09]), LipSide::Neutral, "forehead");
        assert_eq!(c.side_of([0.0, 1.60, 0.05]), LipSide::Neutral, "neck");
        assert_ne!(
            c.side_of([0.0, 1.770, 0.097]),
            LipSide::Neutral,
            "upper lip"
        );
    }

    #[test]
    fn only_opposite_lips_conflict() {
        assert!(lip_sides_conflict(LipSide::Upper, LipSide::Lower));
        assert!(lip_sides_conflict(LipSide::Lower, LipSide::Upper));
        assert!(!lip_sides_conflict(LipSide::Upper, LipSide::Upper));
        assert!(!lip_sides_conflict(LipSide::Upper, LipSide::Neutral));
        assert!(!lip_sides_conflict(LipSide::Neutral, LipSide::Lower));
    }

    fn buckets() -> FacialBuckets {
        let mut map = BTreeMap::new();
        map.insert("upper_lip".to_string(), vec![31, 44, 45]);
        map.insert("lower_lip".to_string(), vec![17, 18, 19]);
        map.insert("tongue".to_string(), vec![20, 21]);
        map.insert("jaw".to_string(), vec![16]);
        map.insert("upper_eyelid_l".to_string(), vec![39]);
        map.insert("lower_eyelid_r".to_string(), vec![37]);
        FacialBuckets { map }
    }

    #[test]
    fn upper_lip_vertex_forbids_lower_lip_and_tongue() {
        let f = forbidden_slots(LipSide::Upper, &buckets());
        assert!(f.contains(&17) && f.contains(&18) && f.contains(&19));
        assert!(f.contains(&20) && f.contains(&21));
        assert!(!f.contains(&31), "must keep its own upper-lip slots");
    }

    #[test]
    fn masking_removes_the_bleed_and_renormalizes() {
        // A lower-lip vertex that picked up 30% upper-lip mass across the closed mouth.
        let mut infl = vec![(17u32, 0.5f32), (16, 0.2), (31, 0.3)];
        let removed = mask_and_renormalize(&mut infl, &forbidden_slots(LipSide::Lower, &buckets()));
        assert!((removed - 0.3).abs() < 1e-6);
        assert!(!infl.iter().any(|(s, _)| *s == 31), "upper-lip mass gone");
        let total: f32 = infl.iter().map(|(_, w)| *w).sum();
        assert!((total - 1.0).abs() < 1e-6, "renormalized: {infl:?}");
        // Ratio between the surviving slots is preserved.
        let m: BTreeMap<u32, f32> = infl.into_iter().collect();
        assert!((m[&17] / m[&16] - 2.5).abs() < 1e-5);
    }

    #[test]
    fn masking_never_empties_a_vertex() {
        // Every slot forbidden → leave it alone rather than ship an unweighted vertex.
        let mut infl = vec![(20u32, 0.6f32), (21, 0.4)];
        let before = infl.clone();
        let removed = mask_and_renormalize(&mut infl, &forbidden_slots(LipSide::Upper, &buckets()));
        assert_eq!(removed, 0.0);
        assert_eq!(infl, before);
    }

    #[test]
    fn eyelid_slots_gather_both_lids_and_sides() {
        assert_eq!(buckets().eyelid_slots(), vec![37, 39]);
    }
}
