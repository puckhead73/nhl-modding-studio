//! End-to-end guard for CC5 Path C in **authored-weights** mode.
//!
//! The unit tests cover each stage in isolation; this pins the seams between them, using a
//! synthetic GLB shaped like a real CC5 export — an articulated skeleton, a head primitive
//! named by its `Std_*` material, inline skin weights, and a **sparse** morph target.
//!
//! Sparse is the point: every CC5 expression target is stored sparsely, and the reader
//! rejected sparse accessors outright until it was fixed. A dense-only fixture would pass
//! while the real thing failed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tdb_headmesh::cc5::{self, Cc5BoneMap};
use tdb_headmesh::facial::FacialBuckets;
use tdb_headmesh::ir::UvConvention;
use tdb_headmesh::palette::NamedPalette;

fn defs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("defs/appearance")
}

/// Four vertices, two triangles, two joints at **distinct** positions (so the rig reads as
/// articulated), and one sparse morph target moving vertex 0 only.
///
/// Vertex 0 is pure `CC_Base_Head`; vertex 1 is pure `CC_Base_JawRoot`; 2 and 3 are split.
fn cc5_rigged_glb() -> Vec<u8> {
    cc5_glb_with_joints([0.0, 1.80, 0.0], [0.0, 1.74, 0.02])
}

/// Same fixture, with the two joint origins under the caller's control — stacking them
/// produces the stub rig a head-only CC5 export ships.
fn cc5_glb_with_joints(head_at: [f32; 3], jaw_at: [f32; 3]) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let positions = [
        [0.0f32, 1.83, 0.09], // 0 — eyelid-ish, moved by the morph
        [0.0, 1.75, 0.09],    // 1 — jaw
        [0.02, 1.79, 0.09],   // 2
        [-0.02, 1.79, 0.09],  // 3
    ];
    for p in positions {
        for c in p {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    let uv_off = bin.len();
    for uv in [[0.1f32, 0.25], [0.2, 0.3], [0.3, 0.35], [0.4, 0.4]] {
        for c in uv {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    let joints_off = bin.len();
    for j in [[0u8, 1, 0, 0], [1, 0, 0, 0], [0, 1, 0, 0], [0, 1, 0, 0]] {
        bin.extend_from_slice(&j);
    }
    let weights_off = bin.len();
    for w in [
        [1.0f32, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.5, 0.5, 0.0, 0.0],
        [0.5, 0.5, 0.0, 0.0],
    ] {
        for c in w {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    // Inverse bind matrices: identity for both joints (bind pose), column-major.
    let ibm_off = bin.len();
    for _ in 0..2 {
        for k in 0..16 {
            let v: f32 = if k % 5 == 0 { 1.0 } else { 0.0 };
            bin.extend_from_slice(&v.to_le_bytes());
        }
    }
    // Sparse morph: one entry, index 0, delta (0, -0.004, 0).
    let sparse_idx_off = bin.len();
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes()); // pad to keep offsets tidy
    let sparse_val_off = bin.len();
    for c in [0.0f32, -0.004, 0.0] {
        bin.extend_from_slice(&c.to_le_bytes());
    }
    let idx_off = bin.len();
    for i in [0u16, 1, 2, 0, 2, 3] {
        bin.extend_from_slice(&i.to_le_bytes());
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }

    let json = serde_json::json!({
        "asset": {"version": "2.0"},
        "materials": [{"name": "Std_Skin_Head.013"}],
        "meshes": [{
            "name": "CC_Base_Body.004",
            "extras": {"targetNames": ["Eye_Blink_L"]},
            "primitives": [{
                "attributes": {"POSITION": 0, "TEXCOORD_0": 1, "JOINTS_0": 2, "WEIGHTS_0": 3},
                "indices": 5,
                "material": 0,
                "targets": [{"POSITION": 6}]
            }]
        }],
        // Joints at DISTINCT model-space positions → articulated, not a stub.
        "nodes": [
            {"name": "CC_Base_Head", "translation": head_at},
            {"name": "CC_Base_JawRoot", "translation": jaw_at},
            {"name": "Armature", "children": [0, 1]}
        ],
        "skins": [{"joints": [0, 1], "inverseBindMatrices": 4}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3"},
            {"bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC2"},
            {"bufferView": 2, "componentType": 5121, "count": 4, "type": "VEC4"},
            {"bufferView": 3, "componentType": 5126, "count": 4, "type": "VEC4"},
            {"bufferView": 4, "componentType": 5126, "count": 2, "type": "MAT4"},
            {"bufferView": 7, "componentType": 5123, "count": 6, "type": "SCALAR"},
            // Sparse morph target: no bufferView, base is all zeros.
            {
                "componentType": 5126, "count": 4, "type": "VEC3",
                "sparse": {
                    "count": 1,
                    "indices": {"bufferView": 5, "componentType": 5123},
                    "values": {"bufferView": 6}
                }
            }
        ],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": uv_off},
            {"buffer": 0, "byteOffset": uv_off, "byteLength": joints_off - uv_off},
            {"buffer": 0, "byteOffset": joints_off, "byteLength": weights_off - joints_off},
            {"buffer": 0, "byteOffset": weights_off, "byteLength": ibm_off - weights_off},
            {"buffer": 0, "byteOffset": ibm_off, "byteLength": sparse_idx_off - ibm_off},
            {"buffer": 0, "byteOffset": sparse_idx_off, "byteLength": 4},
            {"buffer": 0, "byteOffset": sparse_val_off, "byteLength": 12},
            {"buffer": 0, "byteOffset": idx_off, "byteLength": 12}
        ],
        "buffers": [{"byteLength": bin.len()}]
    });
    let mut json_bytes = serde_json::to_vec(&json).unwrap();
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    let mut glb = Vec::new();
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&((12 + 8 + json_bytes.len() + 8 + bin.len()) as u32).to_le_bytes());
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"JSON");
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"BIN\0");
    glb.extend_from_slice(&bin);
    glb
}

fn buckets() -> FacialBuckets {
    FacialBuckets::load(&defs_dir().join("facial-regions.toml")).unwrap()
}

fn bone_map() -> Cc5BoneMap {
    let p = NamedPalette::load(&defs_dir().join("legacy-bone-names.toml")).unwrap();
    Cc5BoneMap::load(&defs_dir().join("cc5-bone-map.toml"), &p).unwrap()
}

#[test]
fn a_real_skeleton_is_recognised_as_articulated() {
    let (_, report) = tdb_headmesh::extract::extract_cc5_head(&cc5_rigged_glb(), vec![]).unwrap();
    assert!(
        !report.rig_is_rigid,
        "two joints at distinct positions is an articulated rig, not a stub: {report:?}"
    );
    assert_eq!(report.joint_names, vec!["CC_Base_Head", "CC_Base_JawRoot"]);
}

#[test]
fn uvs_arrive_in_stored_convention_so_pack_applies_no_flip() {
    let (ir, _) = tdb_headmesh::extract::extract_cc5_head(&cc5_rigged_glb(), vec![]).unwrap();
    let head = ir.head().unwrap();
    assert_eq!(head.uv_convention, UvConvention::Stored);
    assert_eq!(head.uvs[0], [0.1, 0.25], "V must not be flipped for CC5");
}

#[test]
fn sparse_morph_targets_decode_and_land_on_the_right_vertex() {
    let morphs = tdb_headmesh::glb::read_morph_targets(
        &cc5_rigged_glb(),
        "Std_Skin_Head.013",
        &["Eye_Blink_L"],
    )
    .unwrap();
    let deltas = morphs
        .get("Eye_Blink_L")
        .expect("the sparse blink target must decode");
    assert_eq!(deltas.len(), 4, "sparse target densified to one per vertex");
    assert!(
        (deltas[0][1] + 0.004).abs() < 1e-6,
        "moved vertex: {:?}",
        deltas[0]
    );
    for d in &deltas[1..] {
        assert_eq!(*d, [0.0, 0.0, 0.0], "untouched vertices stay zero");
    }
}

#[test]
fn authored_weights_map_through_the_shipped_defs() {
    let (ir, _) = tdb_headmesh::extract::extract_cc5_head(&cc5_rigged_glb(), vec![]).unwrap();
    let head = ir.head().unwrap();
    let authored = head.influences.as_ref().unwrap().vertices.clone();
    let mapped = cc5::map_authored(&authored, &ir.bone_names, &bone_map()).unwrap();

    let p = NamedPalette::load(&defs_dir().join("legacy-bone-names.toml")).unwrap();
    let inv = p.name_to_palette();
    // Vertex 1 is pure JawRoot → pure Jaw slot.
    assert_eq!(mapped[1], vec![(inv["Jaw"], 1.0)]);
    // Vertex 0 is pure Head.
    assert_eq!(mapped[0], vec![(inv["Head"], 1.0)]);
}

/// The whole point of authored mode, exercised across every stage at once.
#[test]
fn jaw_survives_and_the_donor_cannot_touch_the_mouth() {
    let glb = cc5_rigged_glb();
    let (ir, report) = tdb_headmesh::extract::extract_cc5_head(&glb, vec![]).unwrap();
    assert!(!report.rig_is_rigid);
    let head = ir.head().unwrap();
    let authored = head.influences.as_ref().unwrap().vertices.clone();
    let mapped = cc5::map_authored(&authored, &ir.bone_names, &bone_map()).unwrap();

    let b = buckets();
    let morphs =
        tdb_headmesh::glb::read_morph_targets(&glb, "Std_Skin_Head.013", &["Eye_Blink_L"]).unwrap();
    let eyelids = cc5::eyelid_weights_from_morphs(4, &morphs, &cc5::eyelid_assignment(&b));

    // A donor that would happily hand out lower-lip and eyelid mass everywhere.
    let lower_lip = b.get("lower_lip")[0];
    let cheek = 27u32; // LeftCheek — fine detail, not in any barred bucket
    let donor = vec![vec![(cheek, 0.5f32), (lower_lip, 0.5)]; 4];

    let head_slots: Vec<u32> = b.get("head").to_vec();
    let (out, blend_report) = cc5::blend(&mapped, &eyelids, &donor, &b, &head_slots, 4);

    let p = NamedPalette::load(&defs_dir().join("legacy-bone-names.toml")).unwrap();
    let jaw = p.name_to_palette()["Jaw"];

    // 1. Authored jaw mass is untouched.
    let v1: BTreeMap<u32, f32> = out[1].iter().copied().collect();
    assert!(
        (v1.get(&jaw).copied().unwrap_or(0.0) - 1.0).abs() < 1e-5,
        "vertex 1 is pure jaw and must stay pure jaw: {:?}",
        out[1]
    );

    // 2. The donor's lower-lip mass never appears anywhere.
    for (vi, infl) in out.iter().enumerate() {
        assert!(
            !infl.iter().any(|(s, _)| *s == lower_lip),
            "vertex {vi} carries donor lower-lip mass: {infl:?}"
        );
    }

    // 3. The morph gave vertex 0 real eyelid weight, taken from its Head budget.
    let lid_slots = b.get("upper_eyelid_l");
    let v0: BTreeMap<u32, f32> = out[0].iter().copied().collect();
    assert!(
        lid_slots.iter().any(|s| v0.contains_key(s)),
        "vertex 0 is the blink-moved vertex and should carry eyelid weight: {:?}",
        out[0]
    );
    assert!(blend_report.eyelid_vertices >= 1);

    // 4. Donor fine detail still filled in where it was allowed.
    assert!(
        blend_report.donor_filled_mass > 0.0,
        "cheek detail should have filled the remaining head budget"
    );

    // 5. Every vertex is normalized and inside the influence budget.
    for (vi, infl) in out.iter().enumerate() {
        assert!(
            infl.len() <= 4,
            "vertex {vi} exceeds the RW budget: {infl:?}"
        );
        let total: f32 = infl.iter().map(|(_, w)| *w).sum();
        assert!(
            (total - 1.0).abs() < 1e-5,
            "vertex {vi} sums to {total}: {infl:?}"
        );
    }
}

/// The converter branches on `rig_is_rigid`, so the two modes must be distinguishable from
/// the same fixture — otherwise a head-only export could silently take the authored path
/// and remap weights that carry no articulation at all.
#[test]
fn stacking_the_joints_flips_the_rig_back_to_a_stub() {
    let at = [0.0f32, 1.80, 0.0];
    let stub = cc5_glb_with_joints(at, at);
    let (_, report) = tdb_headmesh::extract::extract_cc5_head(&stub, vec![]).unwrap();
    assert!(
        report.rig_is_rigid,
        "coincident joints carry no articulation and must route to the donor-transfer          fallback: {report:?}"
    );

    // The only difference is where the joints sit.
    let (_, articulated) =
        tdb_headmesh::extract::extract_cc5_head(&cc5_rigged_glb(), vec![]).unwrap();
    assert!(!articulated.rig_is_rigid);
}
