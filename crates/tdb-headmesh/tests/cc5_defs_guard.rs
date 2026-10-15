//! Drift guard for the CC5 (Path C) definitions — pattern: `matrix_guard.rs`.
//!
//! Path C's correctness rests on two def files agreeing with the shipped palette and with
//! the code that reads them. Every failure mode guarded here is silent at runtime: a
//! renamed bucket makes the eyelid assignment empty (no blink, no error), a mis-targeted
//! bone quietly moves mass to the wrong slot, and a bone missing from the map only
//! surfaces when someone converts a character that happens to weight it.

use std::path::{Path, PathBuf};

use tdb_headmesh::cc5::{self, Cc5BoneMap};
use tdb_headmesh::facial::FacialBuckets;
use tdb_headmesh::palette::NamedPalette;

fn defs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/tdb-headmesh sits two levels under the repo root")
        .join("defs/appearance")
}

fn palette() -> NamedPalette {
    NamedPalette::load(&defs_dir().join("legacy-bone-names.toml")).expect("palette loads")
}

fn bone_map() -> Cc5BoneMap {
    Cc5BoneMap::load(&defs_dir().join("cc5-bone-map.toml"), &palette()).expect("cc5 bone map loads")
}

fn buckets() -> FacialBuckets {
    FacialBuckets::load(&defs_dir().join("facial-regions.toml")).expect("facial regions load")
}

/// Every bone the reference CC5 head skin actually weights (dan-suckow rigged export,
/// 2026-08-13). A CC5 head that weights a bone outside this set is refused at convert
/// time, so the map must at minimum cover all of these.
const REFERENCE_HEAD_BONES: [&str; 7] = [
    "CC_Base_Head",
    "CC_Base_JawRoot",
    "CC_Base_NeckTwist01",
    "CC_Base_NeckTwist02",
    "CC_Base_Spine02",
    "CC_Base_L_Clavicle",
    "CC_Base_R_Clavicle",
];

#[test]
fn cc5_bone_map_loads_and_every_target_resolves() {
    // Loading is itself the assertion: an unresolvable target is a hard error.
    let _ = bone_map();
}

#[test]
fn cc5_bone_map_covers_every_bone_the_reference_head_weights() {
    let map = bone_map();
    for bone in REFERENCE_HEAD_BONES {
        assert!(
            map.get(bone).is_some(),
            "cc5-bone-map.toml does not cover {bone}, which the reference CC5 head weights — \
             a conversion using it would be refused"
        );
    }
}

#[test]
fn jaw_maps_to_jaw_and_upper_jaw_does_not() {
    let map = bone_map();
    let p = palette();
    let inv = p.name_to_palette();
    let jaw = inv["Jaw"];
    let head = inv["Head"];

    // The whole mouth fix is that CC's jaw mass lands on the Legacy jaw slot.
    assert_eq!(
        map.get("CC_Base_JawRoot"),
        Some(jaw),
        "CC_Base_JawRoot must drive the Jaw slot — this is what makes the lower lip follow \
         the jaw instead of welding shut"
    );
    // And the trap: the UPPER jaw is rigid to the skull. Sending it to Jaw would open the
    // entire midface whenever the mouth does.
    assert_eq!(
        map.get("CC_Base_UpperJaw"),
        Some(head),
        "CC_Base_UpperJaw is rigid to the skull and must map to Head, never Jaw"
    );
}

#[test]
fn neck_resolves_to_the_donor_used_slot_not_the_excluded_one() {
    let map = bone_map();
    // legacy-bone-names.toml names both 5 and 6 "Neck"; 5 is census-excluded garbage and
    // every real donor authors 6. The prefer/exclude flags must carry through the map.
    for bone in ["CC_Base_NeckTwist01", "CC_Base_NeckTwist02"] {
        assert_eq!(
            map.get(bone),
            Some(6),
            "{bone} must land on palette 6; slot 5 is the excluded Neck"
        );
    }
}

#[test]
fn no_cc5_bone_targets_an_excluded_palette_slot() {
    let map = bone_map();
    let p = palette();
    for bone in REFERENCE_HEAD_BONES {
        let slot = map.get(bone).expect("covered by the coverage test");
        let entry = p.get(slot).expect("slot exists in the palette");
        assert!(
            !entry.exclude,
            "{bone} maps to palette {slot}, which is census-excluded garbage"
        );
    }
}

#[test]
fn every_eyelid_morph_names_a_bucket_that_exists_and_is_populated() {
    let b = buckets();
    for (morph, bucket) in cc5::EYELID_MORPHS {
        assert!(
            !b.get(bucket).is_empty(),
            "eyelid morph {morph} drives bucket {bucket:?}, which facial-regions.toml does \
             not define — the assignment would be silently empty and nothing would blink"
        );
    }
}

#[test]
fn eyelid_assignment_covers_both_lids_on_both_sides() {
    let assignment = cc5::eyelid_assignment(&buckets());
    assert!(
        assignment.len() >= 4,
        "expected at least one slot per lid per side, got {assignment:?}"
    );
    let slots: Vec<u32> = assignment.iter().map(|(_, s)| *s).collect();
    let b = buckets();
    for bucket in [
        "upper_eyelid_l",
        "upper_eyelid_r",
        "lower_eyelid_l",
        "lower_eyelid_r",
    ] {
        assert!(
            b.get(bucket).iter().any(|s| slots.contains(s)),
            "{bucket} contributes no slot to the eyelid assignment"
        );
    }
}

#[test]
fn buckets_the_blend_bars_from_the_donor_all_exist() {
    // If one of these is renamed, the blend silently stops barring it and the donor's lip
    // or lid weighting leaks back in underneath the authored data.
    let b = buckets();
    for bucket in [
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
        assert!(
            !b.get(bucket).is_empty(),
            "the blend bars donor mass on {bucket:?}, but that bucket is empty or missing"
        );
    }
    // And the Head bucket must exist, since it is the budget the blend subdivides.
    assert!(
        !b.get("head").is_empty(),
        "facial-regions.toml defines no `head` bucket — the blend would have no budget to \
         distribute and every vertex would keep only its authored coarse weights"
    );
}

#[test]
fn head_and_jaw_buckets_are_disjoint() {
    // The blend treats Head slots as the divisible budget and everything else as
    // protected. Overlap would make jaw mass both protected and redistributed.
    let b = buckets();
    for h in b.get("head") {
        assert!(
            !b.get("jaw").contains(h),
            "palette {h} is in both the head and jaw buckets"
        );
    }
}
