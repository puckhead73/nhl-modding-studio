//! CC5 authored-weights mode — Path C's answer once the source ships a real skeleton.
//!
//! The prior effort's canonical result was that **authored weights win**: NHL24 rigs every
//! vertex, so remapping bone names gets lips, jaw and eyelids right *by construction*, with
//! no geometric region-guessing. A head-only CC5 export cannot play that game — its rig is
//! a three-joint stub — but a full CC5 export absolutely can, and this module is that path.
//!
//! Three sources of truth are combined, in decreasing order of authority:
//!
//! 1. **Authored skin weights** ([`map_authored`]). The CC5 head skin touches ~7 bones;
//!    `CC_Base_JawRoot` carries 10.6% of the reference head. Mapped through
//!    `defs/appearance/cc5-bone-map.toml`, that jaw mass *is* the mouth fix — the lower lip
//!    follows the jaw because Reallusion said so.
//! 2. **Blink morph deltas** ([`eyelid_weights_from_morphs`]). CC5 has no eyelid bones; it
//!    blinks with blend shapes. The vertices `Eye_Blink_L` displaces are exactly the left
//!    upper eyelid, and the normalized displacement is exactly how much each participates —
//!    which is what a lid bone's weight encodes. Authored data, just stored as a morph.
//! 3. **Donor fine detail** ([`blend`]). CC gives 7 bones; the Legacy donor authors 36
//!    slots, including cheek, nose and chin bones that carry the target's subtler facial
//!    animation. Those fill in underneath — but only outside the eye and mouth buckets, so
//!    they can never fight the two channels above.
//!
//! The split matters: (1) and (2) own the mouth and the eyes, (3) is confined to everything
//! else. That is the whole design.

use std::collections::BTreeMap;
use std::path::Path;

use crate::facial::FacialBuckets;
use crate::palette::NamedPalette;

#[derive(Debug, thiserror::Error)]
pub enum Cc5Error {
    #[error("failed to read CC5 bone map: {0}")]
    Io(#[from] std::io::Error),
    #[error("CC5 bone map is not valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unsupported CC5 bone-map schema {found:?} (expected {CC5_BONE_MAP_SCHEMA:?})")]
    Schema { found: String },
    #[error(
        "CC5 bone map sends {cc_bone:?} to {target:?}, which is not a name in the target \
         palette — the map and legacy-bone-names.toml disagree"
    )]
    UnknownTarget { cc_bone: String, target: String },
    #[error(
        "the source head weights {0:?}, which the CC5 bone map does not cover. Add it to \
         defs/appearance/cc5-bone-map.toml rather than letting its mass vanish."
    )]
    UnmappedBone(String),
}

pub const CC5_BONE_MAP_SCHEMA: &str = "cc5-bone-map/1";

#[derive(Debug, serde::Deserialize)]
struct BoneMapFile {
    schema: String,
    #[serde(default, rename = "bone")]
    bones: Vec<BoneEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct BoneEntry {
    source: String,
    target: String,
}

/// CC5 bone name → Legacy palette slot.
#[derive(Debug, Clone)]
pub struct Cc5BoneMap {
    map: BTreeMap<String, u32>,
}

impl Cc5BoneMap {
    pub fn load(path: &Path, palette: &NamedPalette) -> Result<Cc5BoneMap, Cc5Error> {
        let file: BoneMapFile = toml::from_str(&std::fs::read_to_string(path)?)?;
        if file.schema != CC5_BONE_MAP_SCHEMA {
            return Err(Cc5Error::Schema { found: file.schema });
        }
        let inversion = palette.name_to_palette();
        let mut map = BTreeMap::new();
        for b in file.bones {
            let slot = inversion.get(b.target.as_str()).copied().ok_or_else(|| {
                Cc5Error::UnknownTarget {
                    cc_bone: b.source.clone(),
                    target: b.target.clone(),
                }
            })?;
            map.insert(b.source, slot);
        }
        Ok(Cc5BoneMap { map })
    }

    pub fn get(&self, cc_bone: &str) -> Option<u32> {
        self.map.get(cc_bone).copied()
    }
}

/// Remap authored CC5 influences (indices into `bone_names`) onto Legacy palette slots.
///
/// Refuses an unmapped bone rather than dropping its mass: silently losing weight is how a
/// head ends up subtly wrong everywhere instead of loudly wrong once.
pub fn map_authored(
    authored: &[Vec<(u32, f32)>],
    bone_names: &[String],
    map: &Cc5BoneMap,
) -> Result<Vec<Vec<(u32, f32)>>, Cc5Error> {
    let mut out = Vec::with_capacity(authored.len());
    for infl in authored {
        let mut acc: BTreeMap<u32, f32> = BTreeMap::new();
        for &(bone, w) in infl {
            if w <= 0.0 {
                continue;
            }
            let name = bone_names
                .get(bone as usize)
                .map(String::as_str)
                .unwrap_or_default();
            let slot = map
                .get(name)
                .ok_or_else(|| Cc5Error::UnmappedBone(name.to_string()))?;
            *acc.entry(slot).or_insert(0.0) += w;
        }
        out.push(acc.into_iter().collect());
    }
    Ok(out)
}

/// Per-vertex eyelid participation in [0, 1], derived from blink morph deltas.
#[derive(Debug, Clone, Default)]
pub struct EyelidWeights {
    /// Parallel to the head's vertices: (palette slot, participation).
    pub per_vertex: Vec<Vec<(u32, f32)>>,
    /// Which morphs were found and how many vertices each actually moves.
    pub sources: Vec<(String, usize)>,
}

/// The CC5 morphs that stand in for Legacy's eyelid bones, with the facial-region bucket
/// each one drives. `Eye_Blink_*` is the upper lid (a blink is overwhelmingly upper-lid
/// travel); `Eye_LowerLid_Down_*` traces the lower lid.
pub const EYELID_MORPHS: [(&str, &str); 4] = [
    ("Eye_Blink_L", "upper_eyelid_l"),
    ("Eye_Blink_R", "upper_eyelid_r"),
    ("Eye_LowerLid_Down_L", "lower_eyelid_l"),
    ("Eye_LowerLid_Down_R", "lower_eyelid_r"),
];

/// Resolve [`EYELID_MORPHS`] against the shipped buckets into (morph, palette slot) pairs.
/// A bucket with several slots contributes each of them; a missing bucket drops out.
pub fn eyelid_assignment(buckets: &FacialBuckets) -> Vec<(&'static str, u32)> {
    let mut out = Vec::new();
    for (morph, bucket) in EYELID_MORPHS {
        for slot in buckets.get(bucket) {
            out.push((morph, *slot));
        }
    }
    out
}

/// A morph delta below this is export noise, not lid motion.
pub const MORPH_EPSILON_M: f32 = 1e-4;

/// Fraction of a morph's peak displacement below which a vertex is follow-through, not
/// the feature itself.
///
/// A CC5 blink does not only move the eyelid — the cheek and brow drift a fraction of a
/// millimetre with it. Accepting every moved vertex made "eyelid" cover a 138x89 mm patch
/// of face. Gating on a share of the morph's own peak keeps the lid and drops the drift,
/// and scales automatically to however far a given character's blink travels.
pub const MORPH_RELATIVE_FLOOR: f32 = 0.15;

/// Build eyelid weights from CC5 blink/lid morph deltas.
///
/// `morphs` maps a morph name to its per-vertex POSITION deltas; `assignment` says which
/// Legacy palette slot each morph drives. A vertex's weight is its delta magnitude
/// normalized by the largest delta in that morph, so the lid's centre gets ~1.0 and the
/// feathered edge tapers — the same profile a hand-painted lid weight has.
pub fn eyelid_weights_from_morphs(
    vertex_count: usize,
    morphs: &BTreeMap<String, Vec<[f32; 3]>>,
    assignment: &[(&str, u32)],
) -> EyelidWeights {
    let mut per_vertex: Vec<Vec<(u32, f32)>> = vec![Vec::new(); vertex_count];
    let mut sources = Vec::new();
    for (morph_name, slot) in assignment {
        let Some(deltas) = morphs.get(*morph_name) else {
            continue;
        };
        let mags: Vec<f32> = deltas
            .iter()
            .map(|d| (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt())
            .collect();
        let peak = mags.iter().copied().fold(0f32, f32::max);
        if peak <= MORPH_EPSILON_M {
            continue;
        }
        let mut moved = 0usize;
        for (vi, m) in mags.iter().enumerate() {
            if *m <= MORPH_EPSILON_M || vi >= vertex_count {
                continue;
            }
            let w = (m / peak).clamp(0.0, 1.0);
            if w < MORPH_RELATIVE_FLOOR {
                continue;
            }
            moved += 1;
            // Several morphs can touch one vertex (upper and lower lid overlap at the
            // corner); keep the strongest claim per slot.
            let entry = per_vertex[vi].iter_mut().find(|(s, _)| *s == *slot);
            match entry {
                Some((_, existing)) => *existing = existing.max(w),
                None => per_vertex[vi].push((*slot, w)),
            }
        }
        sources.push(((*morph_name).to_string(), moved));
    }
    EyelidWeights {
        per_vertex,
        sources,
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct BlendReport {
    /// Vertices that received donor fine-detail mass.
    pub donor_filled_vertices: usize,
    /// Total mass handed to donor fine-detail slots.
    pub donor_filled_mass: f32,
    /// Vertices carrying morph-derived eyelid weight, and the total.
    pub eyelid_vertices: usize,
    pub eyelid_mass: f32,
    /// Authored mass that stayed exactly as CC5 authored it (jaw + neck + spine).
    pub protected_mass: f32,
}

/// Combine authored CC5 weights, morph eyelids and donor fine detail into final palette
/// influences.
///
/// The rule, in one sentence: **jaw, neck and spine mass is untouchable; the generic Head
/// share is what gets subdivided.**
///
/// * Authored non-Head slots (Jaw, Neck, Spine3) pass through at exactly their authored
///   value. These are the channels that make the mouth and neck correct, so nothing is
///   allowed to dilute them.
/// * The authored `Head` share is the budget everything else competes for. Morph-derived
///   eyelid participation takes its cut first, and the remainder is distributed over the
///   donor's fine facial slots — cheeks, nose, chin, and the donor's own Head mass.
/// * Donor slots inside the **eye and mouth buckets are excluded entirely**, so the donor's
///   lip and lid weighting can never reintroduce the bleed the authored data just fixed.
///
/// `max_influences` is applied last, keeping the heaviest and renormalizing.
pub fn blend(
    authored: &[Vec<(u32, f32)>],
    eyelids: &EyelidWeights,
    donor: &[Vec<(u32, f32)>],
    buckets: &FacialBuckets,
    head_slots: &[u32],
    max_influences: usize,
) -> (Vec<Vec<(u32, f32)>>, BlendReport) {
    // Slots the donor may never contribute: anything that steers an eye or the mouth.
    let mut blocked: Vec<u32> = Vec::new();
    for name in [
        "upper_lip",
        "lower_lip",
        "mouth_corner_l",
        "mouth_corner_r",
        "tongue",
        "jaw",
        "eye_control_l",
        "eye_control_r",
        "upper_eyelid_l",
        "upper_eyelid_r",
        "lower_eyelid_l",
        "lower_eyelid_r",
    ] {
        blocked.extend_from_slice(buckets.get(name));
    }
    blocked.sort_unstable();
    blocked.dedup();

    let mut report = BlendReport::default();
    let mut out = Vec::with_capacity(authored.len());

    for (vi, authored_vertex) in authored.iter().enumerate() {
        let mut final_w: BTreeMap<u32, f32> = BTreeMap::new();

        // Authored mass, split into the protected part and the Head budget.
        let mut head_budget = 0f32;
        for &(slot, w) in authored_vertex {
            if head_slots.contains(&slot) {
                head_budget += w;
            } else {
                *final_w.entry(slot).or_insert(0.0) += w;
                report.protected_mass += w;
            }
        }
        if head_budget <= 0.0 {
            out.push(finish(final_w, max_influences));
            continue;
        }

        // Eyelid participation takes its share of the Head budget first.
        let lid = eyelids.per_vertex.get(vi).map(Vec::as_slice).unwrap_or(&[]);
        let lid_share: f32 = lid.iter().map(|(_, w)| *w).sum::<f32>().clamp(0.0, 1.0);
        if lid_share > 0.0 {
            for &(slot, w) in lid {
                let mass = head_budget * w.clamp(0.0, 1.0);
                *final_w.entry(slot).or_insert(0.0) += mass;
                report.eyelid_mass += mass;
            }
            report.eyelid_vertices += 1;
        }
        let remaining = head_budget * (1.0 - lid_share);
        if remaining <= 0.0 {
            out.push(finish(final_w, max_influences));
            continue;
        }

        // The rest is distributed the way the donor distributes it — minus eye/mouth slots.
        let fine: Vec<(u32, f32)> = donor
            .get(vi)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .copied()
            .filter(|(s, w)| *w > 0.0 && !blocked.contains(s))
            .collect();
        let fine_total: f32 = fine.iter().map(|(_, w)| *w).sum();
        if fine_total > 0.0 {
            let mut added = 0f32;
            for (slot, w) in fine {
                let mass = remaining * (w / fine_total);
                *final_w.entry(slot).or_insert(0.0) += mass;
                if !head_slots.contains(&slot) {
                    added += mass;
                }
            }
            if added > 0.0 {
                report.donor_filled_vertices += 1;
                report.donor_filled_mass += added;
            }
        } else {
            // No usable donor detail here — keep the authored Head mass where it was.
            let anchor = head_slots.first().copied().unwrap_or(8);
            *final_w.entry(anchor).or_insert(0.0) += remaining;
        }

        out.push(finish(final_w, max_influences));
    }
    (out, report)
}

/// Top-`max` influences, renormalized to sum 1.
fn finish(w: BTreeMap<u32, f32>, max: usize) -> Vec<(u32, f32)> {
    let mut list: Vec<(u32, f32)> = w.into_iter().filter(|(_, x)| *x > 0.0).collect();
    list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    list.truncate(max);
    let total: f32 = list.iter().map(|(_, x)| *x).sum();
    if total > 0.0 {
        for (_, x) in &mut list {
            *x /= total;
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buckets() -> FacialBuckets {
        FacialBuckets::from_map(BTreeMap::from([
            ("upper_lip".into(), vec![31, 44, 45]),
            ("lower_lip".into(), vec![17, 18, 19]),
            ("mouth_corner_l".into(), vec![29, 30]),
            ("mouth_corner_r".into(), vec![32, 33]),
            ("tongue".into(), vec![20, 21]),
            ("jaw".into(), vec![16]),
            ("upper_eyelid_l".into(), vec![39]),
            ("upper_eyelid_r".into(), vec![41]),
            ("lower_eyelid_l".into(), vec![36]),
            ("lower_eyelid_r".into(), vec![37]),
            ("eye_control_l".into(), vec![23, 34]),
            ("eye_control_r".into(), vec![24, 35]),
        ]))
    }

    const HEAD: [u32; 3] = [8, 10, 15];

    #[test]
    fn authored_jaw_mass_survives_the_blend_untouched() {
        // 60% Head / 40% Jaw, with a donor that would happily hand out lip weight.
        let authored = vec![vec![(8u32, 0.6f32), (16, 0.4)]];
        let donor = vec![vec![(8u32, 0.5f32), (27, 0.3), (18, 0.2)]]; // 18 = LowerLip
        let (out, _) = blend(
            &authored,
            &EyelidWeights::default(),
            &donor,
            &buckets(),
            &HEAD,
            4,
        );
        let m: BTreeMap<u32, f32> = out[0].iter().copied().collect();
        assert!(
            (m[&16] - 0.4).abs() < 1e-5,
            "jaw mass preserved: {:?}",
            out[0]
        );
        assert!(!m.contains_key(&18), "donor lower-lip mass must be blocked");
        // The Head budget was split between the donor's Head and cheek slots.
        assert!(m.contains_key(&27), "cheek detail filled in: {:?}", out[0]);
        let total: f32 = out[0].iter().map(|(_, w)| *w).sum();
        assert!((total - 1.0).abs() < 1e-5);
    }

    #[test]
    fn donor_can_never_contribute_eye_or_mouth_slots() {
        let authored = vec![vec![(8u32, 1.0f32)]];
        // A donor vertex made entirely of blocked slots.
        let donor = vec![vec![(18u32, 0.4f32), (39, 0.3), (16, 0.3)]];
        let (out, _) = blend(
            &authored,
            &EyelidWeights::default(),
            &donor,
            &buckets(),
            &HEAD,
            4,
        );
        // Nothing usable → the authored Head mass stays put rather than leaking.
        assert_eq!(out[0], vec![(8, 1.0)]);
    }

    #[test]
    fn blink_morph_becomes_eyelid_weight_scaled_by_displacement() {
        // Three vertices: one at the lid centre, one half as far, one unmoved.
        let morphs = BTreeMap::from([(
            "Eye_Blink_L".to_string(),
            vec![[0.0, -0.004, 0.0], [0.0, -0.002, 0.0], [0.0, 0.0, 0.0]],
        )]);
        let e = eyelid_weights_from_morphs(3, &morphs, &[("Eye_Blink_L", 39)]);
        assert_eq!(e.per_vertex[0], vec![(39, 1.0)]);
        assert!((e.per_vertex[1][0].1 - 0.5).abs() < 1e-5);
        assert!(e.per_vertex[2].is_empty(), "unmoved vertex is not eyelid");
        assert_eq!(e.sources, vec![("Eye_Blink_L".to_string(), 2)]);
    }

    #[test]
    fn eyelid_takes_its_share_from_the_head_budget_not_from_the_jaw() {
        let authored = vec![vec![(8u32, 0.5f32), (16, 0.5)]];
        let eyelids = EyelidWeights {
            per_vertex: vec![vec![(39u32, 1.0f32)]],
            sources: vec![],
        };
        let donor = vec![vec![(8u32, 1.0f32)]];
        let (out, rep) = blend(&authored, &eyelids, &donor, &buckets(), &HEAD, 4);
        let m: BTreeMap<u32, f32> = out[0].iter().copied().collect();
        assert!((m[&16] - 0.5).abs() < 1e-5, "jaw untouched: {:?}", out[0]);
        assert!((m[&39] - 0.5).abs() < 1e-5, "lid took the head share");
        assert!((rep.protected_mass - 0.5).abs() < 1e-5);
    }

    #[test]
    fn unmapped_source_bone_is_refused_not_silently_dropped() {
        let map = Cc5BoneMap {
            map: BTreeMap::from([("CC_Base_Head".to_string(), 8u32)]),
        };
        let names = vec!["CC_Base_Head".to_string(), "CC_Base_Mystery".to_string()];
        let authored = vec![vec![(0u32, 0.5f32), (1, 0.5)]];
        let err = map_authored(&authored, &names, &map).unwrap_err();
        assert!(matches!(err, Cc5Error::UnmappedBone(n) if n == "CC_Base_Mystery"));
    }

    #[test]
    fn authored_bones_collapsing_to_one_slot_are_summed() {
        let map = Cc5BoneMap {
            map: BTreeMap::from([
                ("CC_Base_NeckTwist01".to_string(), 6u32),
                ("CC_Base_NeckTwist02".to_string(), 6u32),
            ]),
        };
        let names = vec![
            "CC_Base_NeckTwist01".to_string(),
            "CC_Base_NeckTwist02".to_string(),
        ];
        let out = map_authored(&[vec![(0u32, 0.3f32), (1, 0.2)]], &names, &map).unwrap();
        assert_eq!(out[0], vec![(6, 0.5)]);
    }
}
