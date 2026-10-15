//! Path A orchestration — NHL24 component → RW donor, end to end.
//!
//! extract → remap → pack → texture → QA. Pure in/out: bytes and defs in, bytes and
//! reports out. No filesystem writes, no game paths — the CLI and the studio layer own
//! IO and install planning. Every stage's gate stays hard: strict re-parse after pack,
//! exact texlib shape, QA scorecard always produced.

use std::path::Path;

use crate::extract::{self, ExtractError};
use crate::ir::IrSubmesh;
use crate::morph::MorphError;
use crate::pack_rw::{self, PackRwError, PackRwOptions, PackRwReport};
use crate::palette::{NamedPalette, PaletteError};
use crate::qa::{self, QaError, Scorecard};
use crate::remap::{BoneMap, BoneOverrides, RemapError, RemapStats};
use crate::sidecar::Sidecar;
use crate::texture::{self, TextureError};

/// Target influence budget for RW (4 × u8 weights).
pub const RW_MAX_INFLUENCES: usize = 4;

/// How far the source's eye midpoint may sit from the donor's before the conversion is
/// refused as unaligned. Generous enough to pass a genuinely different face, tight enough
/// to catch a whole-body export that was never moved into head space.
pub const FRAME_OFFSET_LIMIT_MM: f32 = 25.0;

/// Pool every Eyes-role submesh, for the frame check (CC5 splits eyes across prims).
fn pooled_source_eyes_for_frame(ir: &crate::ir::HeadIr) -> Vec<[f32; 3]> {
    ir.submeshes
        .iter()
        .filter(|s| s.role == crate::ir::SubmeshRole::Eyes)
        .flat_map(|s| s.positions.iter().copied())
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error(transparent)]
    Extract(#[from] ExtractError),
    #[error(transparent)]
    Palette(#[from] PaletteError),
    #[error(transparent)]
    Remap(#[from] RemapError),
    #[error(transparent)]
    Pack(#[from] PackRwError),
    #[error(transparent)]
    Texture(#[from] TextureError),
    #[error(transparent)]
    Qa(#[from] QaError),
    #[error(transparent)]
    Morph(#[from] MorphError),
    #[error("donor mesh: {0}")]
    Donor(#[from] tdb_rx2::playermesh::PlayerMeshError),
    #[error("head submesh carries no authored influences")]
    NoAuthoredWeights,
    #[error(transparent)]
    Transfer(#[from] crate::transfer::TransferError),
    #[error("donor {0} has no Head-role submesh to source weights from (handles: {1})")]
    DonorHasNoHead(String, String),
    #[error(transparent)]
    Facial(#[from] crate::facial::FacialError),
    #[error(transparent)]
    Cc5(#[from] crate::cc5::Cc5Error),
    #[error(
        "source head sits {dist_mm:.0} mm from Legacy head space — it has not been aligned.          Either export it from your aligned scene, or pass the measured correction back in:          --source-offset {0},{1},{2}", offset_m[0], offset_m[1], offset_m[2]
    )]
    FrameOffset { dist_mm: f32, offset_m: [f32; 3] },
    #[error("reading morph targets: {0}")]
    MorphTargets(#[from] crate::glb::GlbError),
}

pub struct PathAInputs<'a> {
    pub glb: &'a [u8],
    pub sidecar: &'a Sidecar,
    pub source_paths: Vec<String>,
    pub donor_rx2: &'a [u8],
    pub donor_texlib: &'a [u8],
    /// DDS bytes; `None` keeps the donor raster (which will NOT match the source UVs —
    /// only sensible for pipeline debugging).
    pub head_texture_dds: Option<&'a [u8]>,
    pub hair_texture_dds: Option<&'a [u8]>,
    pub collapse_haircap: bool,
    /// Align donor eyeballs/mouthbag to the source's measured placement (default
    /// behaviour; disable only for A/B debugging against donor positions).
    pub align_submeshes: bool,
    /// `defs/appearance/` directory (palette, overrides, facial regions).
    pub defs_dir: &'a Path,
}

pub struct PathAOutput {
    pub player_rx2: Vec<u8>,
    pub texlib_rx2: Vec<u8>,
    pub scorecard: Scorecard,
    pub remap_stats: RemapStats,
    pub pack_report: PackRwReport,
    /// The converted head submesh (for previews/exports).
    pub head: IrSubmesh,
}

pub fn convert_nhl24_to_rw(inputs: PathAInputs<'_>) -> Result<PathAOutput, ConvertError> {
    // Extract.
    let ir = extract::extract_nhl24_head(inputs.glb, inputs.sidecar, inputs.source_paths)?;
    let head = ir.head().expect("extract guarantees a head").clone();
    let authored = head
        .influences
        .as_ref()
        .ok_or(ConvertError::NoAuthoredWeights)?
        .vertices
        .clone();

    // Remap.
    let palette = NamedPalette::load(&inputs.defs_dir.join("legacy-bone-names.toml"))?;
    let overrides = BoneOverrides::load(&inputs.defs_dir.join("bone-overrides.toml"))?;
    let skeleton = inputs.sidecar.skeleton();
    let bone_map = BoneMap::build(&skeleton, &palette, &overrides)?;
    let (palette_influences, remap_stats) =
        crate::remap::remap_vertices(&authored, &bone_map, RW_MAX_INFLUENCES);

    // Measure the source's eyeball/mouth placement so the donor's rigid companions
    // land where the source authored its head surface around them.
    let donor_asset = tdb_rx2::playermesh::parse_player_mesh_preview(inputs.donor_rx2)?;
    let alignment = if inputs.align_submeshes {
        let eye_centers = ir
            .submeshes
            .iter()
            .find(|s| s.role == crate::ir::SubmeshRole::Eyes)
            .and_then(|eyes| crate::placement::split_eye_centers(&eyes.positions));
        let source_mouth = crate::placement::mouth_centroid(&head.positions, &head.triangles);
        let donor_mouth = donor_asset
            .submeshes
            .iter()
            .find(|s| crate::ir::SubmeshRole::classify(&s.name) == crate::ir::SubmeshRole::Head)
            .and_then(|s| {
                let positions: Vec<[f32; 3]> = s.vertices.iter().map(|v| v.position).collect();
                crate::placement::mouth_centroid(&positions, &s.triangles)
            });
        let mouth_delta = match (source_mouth, donor_mouth) {
            (Some(s), Some(d)) => Some([s[0] - d[0], s[1] - d[1], s[2] - d[2]]),
            _ => None,
        };
        pack_rw::SubmeshAlignment {
            eye_centers,
            mouth_delta,
            // Translation only. Path A is roundtrip_verified against the NHL24 recipe;
            // companion scaling arrived with Path C and changing a verified pipeline's
            // output is a re-verification, not a refactor. Wire it here once an NHL24
            // conversion has been re-gated with it.
            eye_scale: None,
            mouth_scale: None,
        }
    } else {
        pack_rw::SubmeshAlignment::default()
    };

    // Pack (strict re-parse inside).
    let (player_rx2, pack_report) = pack_rw::pack_head_into_donor(
        inputs.donor_rx2,
        &head,
        &palette_influences,
        PackRwOptions {
            collapse_haircap: inputs.collapse_haircap,
            // Off for the same reason `eye_scale`/`mouth_scale` are: Path A is
            // roundtrip_verified against the NHL24 recipe, and re-seating the haircap
            // changes its output. NHL24 heads have the same donor-shell mismatch, so wire
            // this on once an NHL24 conversion has been re-gated with it.
            conform_haircap: false,
            alignment,
        },
    )?;

    // Texture (exact-shape gate inside).
    let head_rgba = inputs
        .head_texture_dds
        .map(texture::decode_source_dds)
        .transpose()?;
    let hair_rgba = inputs
        .hair_texture_dds
        .map(texture::decode_source_dds)
        .transpose()?;
    let texlib_rx2 = texture::build_legacy_texlib_rgba(
        inputs.donor_texlib,
        head_rgba.as_ref().map(|(d, w, h)| (d.as_slice(), *w, *h)),
        hair_rgba.as_ref().map(|(d, w, h)| (d.as_slice(), *w, *h)),
    )?;

    // QA.
    let target_binds: Vec<[f32; 3]> = donor_asset
        .skin
        .matrices
        .iter()
        .map(|m| tdb_rx2::skeleton::palette_bind_pose(m).0)
        .collect();
    let facial = qa::facial_palette_set(&inputs.defs_dir.join("facial-regions.toml"))?;
    let scorecard = qa::run_battery(
        &head.positions,
        &authored,
        &inputs.sidecar.bind_translations(),
        &bone_map,
        &palette_influences,
        &target_binds,
        &facial,
    )?;

    Ok(PathAOutput {
        player_rx2,
        texlib_rx2,
        scorecard,
        remap_stats,
        pack_report,
        head,
    })
}

// ---------------------------------------------------------------------------
// Path C — Reallusion CC5 → RW donor
// ---------------------------------------------------------------------------

/// Inputs for [`convert_cc5_to_rw`].
///
/// Deliberately smaller than [`PathAInputs`]: a CC5 GLB is self-describing, so there is
/// no weights sidecar, and the source rig is not consulted at all — weights come from the
/// donor. `defs_dir` is still taken because the QA containment check reads the target's
/// palette naming.
pub struct PathCInputs<'a> {
    pub glb: &'a [u8],
    pub source_paths: Vec<String>,
    pub donor_rx2: &'a [u8],
    pub donor_texlib: &'a [u8],
    /// Head atlas — DDS, PNG or JPEG. CC5 UVs are nothing like the donor's, so shipping
    /// the source geometry without its own atlas renders a scrambled face; `None` is for
    /// pipeline debugging only.
    pub head_texture: Option<&'a [u8]>,
    pub hair_texture: Option<&'a [u8]>,
    pub collapse_haircap: bool,
    /// Align the donor's rigid companions (eyeballs, mouthbag) to the source's measured
    /// sockets and mouth. CC5 heads keep open eye sockets and an open mouth, so this is
    /// what stops the donor eyes floating behind the new face.
    pub align_submeshes: bool,
    /// Translation applied to every source vertex before anything else, in metres.
    ///
    /// The source is expected to arrive **already aligned to Legacy head space** — that
    /// alignment is a judgement call about how the head should sit in the helmet, and it
    /// belongs to whoever prepared the mesh, not to this converter. A CC5 export taken
    /// straight from the character's own scene is ~18 cm low; rather than guess a
    /// correction, the converter refuses (see `allow_frame_offset`) and reports the
    /// measured delta so it can be passed back in here or baked into the export.
    pub source_offset: Option<[f32; 3]>,
    /// Convert anyway when the source head is far from Legacy head space.
    pub allow_frame_offset: bool,
    /// Override the eyelid-containment reach, in mm. `None` measures it from the donor's
    /// own eyelid band (its median vertex spacing). Raise it if the blink reads too weakly
    /// in game, lower it if the brow still moves.
    pub eyelid_reach_mm: Option<f32>,
    pub defs_dir: &'a Path,
}

/// A source companion submesh that was measured and then dropped — never packed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompanionMeasurement {
    pub name: String,
    pub role: String,
    pub vertices: usize,
    /// Axis-aligned extent in mm, `None` for an empty submesh.
    pub extents_mm: Option<[f32; 3]>,
}

/// What the authored-weights path did, when the source shipped a real skeleton.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuthoredReport {
    /// Joints in the source skeleton.
    pub source_bones: usize,
    /// (morph name, vertices it moves) for each eyelid morph that was found.
    pub morphs_found: Vec<(String, usize)>,
    pub blend: crate::cc5::BlendReport,
}

/// What the anatomical gates actually did — the numbers that say whether the lip and
/// eyelid rules fired sensibly or are miscalibrated.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GateReport {
    /// A mouth loop was found on both meshes, so the lip rules are live.
    pub lip_crease_found: bool,
    /// Source vertices classified upper / lower / outside the mouth zone.
    pub upper_lip_vertices: usize,
    pub lower_lip_vertices: usize,
    /// Vertices where the gate rejected every donor candidate and an ungated nearest
    /// point was used instead. A large number means the gate is wrong, not the mesh.
    pub gate_rescues: usize,
    /// Vertices whose cross-lip / tongue mass was masked off, and the total mass removed.
    pub lip_vertices_masked: usize,
    pub lip_mass_masked: f32,
    /// Vertices that lost eyelid weight for sitting outside the eyelid band.
    pub eyelid_vertices_masked: usize,
    pub eyelid_mass_masked: f32,
    /// Reach of the donor's own eyelid band, and the cutoff derived from it (mm).
    pub eyelid_band_reach_mm: f32,
    /// Vertices that KEPT eyelid weight, and the extent of that surviving band in mm.
    /// The containment rule is only correct if this still covers two lid-sized patches —
    /// a near-zero count would mean the lids were stripped along with the brow.
    pub eyelid_vertices_kept: usize,
    pub eyelid_kept_extent_mm: Option<[f32; 3]>,
}

/// Path C result. Carries the transfer report in place of Path A's retarget scorecard —
/// there is no source rig to score a retarget against.
pub struct PathCOutput {
    pub player_rx2: Vec<u8>,
    pub texlib_rx2: Vec<u8>,
    pub bind_report: crate::extract::Cc5BindReport,
    pub transfer_report: crate::transfer::TransferReport,
    pub pack_report: PackRwReport,
    /// Palette slots the transferred weights use that the donor head did **not** —
    /// always empty by construction; kept as an explicit assertion in the output.
    pub invented_slots: Vec<u32>,
    /// Source companion submeshes (eyes, corneas, teeth, tongue) that were used purely as
    /// measuring sticks for the donor's own companions and then discarded.
    pub companions_dropped: Vec<CompanionMeasurement>,
    /// Anatomical gating outcome — see [`GateReport`].
    pub gates: GateReport,
    /// Present when the source shipped a real skeleton and authored weights were used;
    /// `None` means the stub-rig fallback (pure donor transfer) ran instead.
    pub authored: Option<AuthoredReport>,
    /// The donor's head is (near-)identical to the source head — i.e. the "donor" is an
    /// already-converted asset, most often because a previous output was installed over
    /// the vanilla file. Every measurement this path makes is relative to the donor, so
    /// this quietly produces a no-op re-conversion with meaningless placement and scale
    /// figures. Callers must not treat it as a normal result.
    pub donor_is_already_this_head: bool,
    pub head: IrSubmesh,
}

/// Convert a Reallusion **Character Creator 5** head GLB into an RW donor container.
///
/// Path C = extract → **donor weight transfer** → pack → texture → report. The one
/// structural difference from Path A is the weighting stage: CC5 ships no articulated
/// rig, so [`crate::remap`] has nothing to retarget and [`crate::transfer`] sources the
/// skin weights from the donor's own rigged surface by proximity. Everything downstream
/// (the eye rule, submesh alignment, the strict re-parse, the exact-texlib-shape gate) is
/// the same machinery Path A uses.
pub fn convert_cc5_to_rw(inputs: PathCInputs<'_>) -> Result<PathCOutput, ConvertError> {
    // Extract — roles from CC5 material names, frame proven rather than assumed.
    let (mut ir, bind_report) = crate::extract::extract_cc5_head(inputs.glb, inputs.source_paths)?;
    // Caller-supplied alignment into Legacy head space, applied before anything measures
    // anything. See `PathCInputs::source_offset` for why this is an input and not a guess.
    if let Some(off) = inputs.source_offset {
        for s in &mut ir.submeshes {
            for p in &mut s.positions {
                for k in 0..3 {
                    p[k] += off[k];
                }
            }
        }
    }
    let head = ir
        .head()
        .expect("extract guarantees exactly one head")
        .clone();

    // The donor supplies both the container and, on Path C, the skin weights.
    let donor_asset = tdb_rx2::playermesh::parse_player_mesh_preview(inputs.donor_rx2)?;
    let donor_head = donor_asset
        .submeshes
        .iter()
        .find(|s| crate::ir::SubmeshRole::classify(&s.name) == crate::ir::SubmeshRole::Head)
        .ok_or_else(|| {
            ConvertError::DonorHasNoHead(
                "player rx2".into(),
                donor_asset
                    .submeshes
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })?;
    let donor_positions: Vec<[f32; 3]> = donor_head.vertices.iter().map(|v| v.position).collect();

    // Frame gate. Every measurement downstream — the weight transfer, the eye/mouth
    // placement, the seam at the neck — assumes the source already sits in Legacy head
    // space. A head 18 cm low still converts, silently and wrongly, so check it here and
    // hand back the exact translation needed rather than a vague complaint.
    let frame_offset_mm = {
        let src_mid = crate::placement::split_eye_centers(&pooled_source_eyes_for_frame(&ir)).map(
            |(l, r)| {
                [
                    (l[0] + r[0]) / 2.0,
                    (l[1] + r[1]) / 2.0,
                    (l[2] + r[2]) / 2.0,
                ]
            },
        );
        let donor_eyes: Vec<[f32; 3]> = donor_asset
            .submeshes
            .iter()
            .filter(|s| crate::ir::SubmeshRole::classify(&s.name) == crate::ir::SubmeshRole::Eyes)
            .flat_map(|s| s.vertices.iter().map(|v| v.position))
            .collect();
        let dst_mid = crate::placement::split_eye_centers(&donor_eyes).map(|(l, r)| {
            [
                (l[0] + r[0]) / 2.0,
                (l[1] + r[1]) / 2.0,
                (l[2] + r[2]) / 2.0,
            ]
        });
        match (src_mid, dst_mid) {
            (Some(s), Some(d)) => Some([
                (d[0] - s[0]) * 1000.0,
                (d[1] - s[1]) * 1000.0,
                (d[2] - s[2]) * 1000.0,
            ]),
            _ => None,
        }
    };
    if let Some(off) = frame_offset_mm {
        let dist = (off[0] * off[0] + off[1] * off[1] + off[2] * off[2]).sqrt();
        if dist > FRAME_OFFSET_LIMIT_MM && !inputs.allow_frame_offset {
            return Err(ConvertError::FrameOffset {
                dist_mm: dist,
                offset_m: [off[0] / 1000.0, off[1] / 1000.0, off[2] / 1000.0],
            });
        }
    }
    // RW BLENDINDICES are palette × 3; the transfer works in palette space.
    let donor_influences: Vec<Vec<(u32, f32)>> = donor_head
        .vertices
        .iter()
        .map(|v| {
            (0..4)
                .filter(|k| v.weights[*k] > 0.0)
                .map(|k| (u32::from(v.joints[k] / 3), v.weights[k]))
                .collect()
        })
        .collect();
    let donor_slots: std::collections::BTreeSet<u32> =
        donor_influences.iter().flatten().map(|(s, _)| *s).collect();

    // ---- Anatomical gates on the correspondence ---------------------------------------
    //
    // Pure proximity welds the mouth shut: at a closed bind pose the nearest donor surface
    // to a lower-lip vertex is often the UPPER lip, so the lower lip inherits upper-lip
    // weights and stops following the jaw. Two gates keep the correspondence honest, and a
    // post-mask cleans up what survives them (a donor vertex near the lip seam carries a
    // little of the other side's weight in its own right).
    let source_normals = pack_rw::smooth_normals(&head.positions, &head.triangles);
    let donor_normals = pack_rw::smooth_normals(&donor_positions, &donor_head.triangles);
    let donor_tri_normal = |ti: usize| -> [f32; 3] {
        let t = donor_head.triangles[ti];
        let mut n = [0f32; 3];
        for &vi in &t {
            for k in 0..3 {
                n[k] += donor_normals[vi as usize][k];
            }
        }
        n
    };

    let buckets = crate::facial::FacialBuckets::load(&inputs.defs_dir.join("facial-regions.toml"))?;
    let source_crease = crate::placement::mouth_loop_points(&head.positions, &head.triangles)
        .and_then(|pts| crate::facial::MouthCrease::new(pts, crate::facial::DEFAULT_LIP_ZONE_M));
    let donor_crease = crate::placement::mouth_loop_points(&donor_positions, &donor_head.triangles)
        .and_then(|pts| crate::facial::MouthCrease::new(pts, crate::facial::DEFAULT_LIP_ZONE_M));
    let source_side: Vec<crate::facial::LipSide> = head
        .positions
        .iter()
        .map(|p| match &source_crease {
            Some(c) => c.side_of(*p),
            None => crate::facial::LipSide::Neutral,
        })
        .collect();
    // A donor triangle's side is its centroid's side.
    let donor_tri_side: Vec<crate::facial::LipSide> = donor_head
        .triangles
        .iter()
        .map(|t| {
            let mut c = [0f32; 3];
            for &vi in t {
                for k in 0..3 {
                    c[k] += donor_positions[vi as usize][k] / 3.0;
                }
            }
            match &donor_crease {
                Some(d) => d.side_of(c),
                None => crate::facial::LipSide::Neutral,
            }
        })
        .collect();

    let gate = |qi: usize, ti: usize| -> bool {
        // Normal gate: an opposed-facing donor triangle is the far side of a crease or the
        // inside of the shell, never the surface this vertex belongs to.
        let (a, b) = (source_normals[qi], donor_tri_normal(ti));
        if a[0] * b[0] + a[1] * b[1] + a[2] * b[2] < 0.0 {
            return false;
        }
        // Lip gate: upper never sources lower, and vice versa.
        !crate::facial::lip_sides_conflict(source_side[qi], donor_tri_side[ti])
    };

    let (mut palette_influences, transfer_report, gate_rescues) =
        crate::transfer::transfer_weights_gated(
            &head.positions,
            &crate::transfer::DonorSkin {
                positions: &donor_positions,
                triangles: &donor_head.triangles,
                influences: &donor_influences,
            },
            RW_MAX_INFLUENCES,
            &gate,
        )?;
    // Keep the raw donor transfer: in authored mode it is demoted from "the weights" to
    // "the fine-detail filler", blended under the authored jaw/neck mass below.
    let donor_transfer = palette_influences.clone();

    // Post-mask: strip cross-lip and tongue mass that rode in on an otherwise-valid
    // sample, then renormalize.
    let mut lip_mass_masked = 0f32;
    let mut lip_vertices_masked = 0usize;
    // Only meaningful in the stub-rig fallback: with authored weights the jaw mass IS the
    // lip separation, and re-deriving it from a guessed crease could only make it worse.
    for (vi, infl) in palette_influences
        .iter_mut()
        .enumerate()
        .filter(|_| bind_report.rig_is_rigid)
    {
        let forbidden = crate::facial::forbidden_slots(source_side[vi], &buckets);
        let removed = crate::facial::mask_and_renormalize(infl, &forbidden);
        if removed > 0.0 {
            lip_mass_masked += removed;
            lip_vertices_masked += 1;
        }
    }
    // Eyelid containment. The eyelid bones drive a narrow band of skin; brow vertices that
    // land on the donor's lid inherit it and the blink then moves the brow. The band's
    // reach is measured off the donor rather than guessed: take the donor vertices the
    // eyelid bones actually drive, and allow source vertices only within that cloud's own
    // radius (plus a small margin for the shape difference we are transplanting).
    let eyelid_slots = buckets.eyelid_slots();
    let donor_eyelid: Vec<[f32; 3]> = donor_influences
        .iter()
        .enumerate()
        .filter(|(_, infl)| {
            infl.iter()
                .filter(|(s, _)| eyelid_slots.contains(s))
                .map(|(_, w)| *w)
                .sum::<f32>()
                >= 0.5
        })
        .map(|(vi, _)| donor_positions[vi])
        .collect();
    let mut eyelid_vertices_masked = 0usize;
    let mut eyelid_mass_masked = 0f32;
    let mut eyelid_band_reach_mm = 0f32;
    if !donor_eyelid.is_empty() && bind_report.rig_is_rigid {
        // Band reach = the MEDIAN nearest-neighbour spacing inside the eyelid cloud: how
        // finely the band is sampled, and therefore how far off it a source vertex can sit
        // and still plausibly be the same strip of skin. The max gap was tried first and is
        // the wrong statistic — one outlier lid vertex inflated it to 9.1 mm, which let
        // eyelid weight survive over a 117×48 mm patch (a real lid band is ~15 mm tall), so
        // the brow kept blinking. The median is robust to that.
        let mut nn: Vec<f32> = Vec::with_capacity(donor_eyelid.len());
        for (i, a) in donor_eyelid.iter().enumerate() {
            let nearest = donor_eyelid
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, b)| (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f32>())
                .fold(f32::MAX, f32::min);
            if nearest.is_finite() {
                nn.push(nearest.sqrt() * 1000.0);
            }
        }
        nn.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        eyelid_band_reach_mm = inputs
            .eyelid_reach_mm
            .unwrap_or_else(|| nn.get(nn.len() / 2).copied().unwrap_or(3.0));
        let cutoff_m = (eyelid_band_reach_mm / 1000.0).max(0.003);
        for (vi, infl) in palette_influences.iter_mut().enumerate() {
            if !infl.iter().any(|(s, _)| eyelid_slots.contains(s)) {
                continue;
            }
            let p = head.positions[vi];
            let d2 = donor_eyelid
                .iter()
                .map(|q| (0..3).map(|k| (q[k] - p[k]).powi(2)).sum::<f32>())
                .fold(f32::MAX, f32::min);
            if d2.sqrt() > cutoff_m {
                let removed = crate::facial::mask_and_renormalize(infl, &eyelid_slots);
                if removed > 0.0 {
                    eyelid_vertices_masked += 1;
                    eyelid_mass_masked += removed;
                }
            }
        }
    }

    // ---- Authored mode -----------------------------------------------------------------
    //
    // When the source ships a real skeleton, the donor transfer stops being the answer and
    // becomes the filler. CC5's authored weights own the jaw and neck; its blink morphs own
    // the eyelids; the donor supplies only the fine facial detail CC has no bones for, and
    // is barred from the eye and mouth buckets so it cannot undo either.
    let mut authored_report = None;
    if !bind_report.rig_is_rigid {
        let authored = head
            .influences
            .as_ref()
            .ok_or(ConvertError::NoAuthoredWeights)?
            .vertices
            .clone();
        let palette = NamedPalette::load(&inputs.defs_dir.join("legacy-bone-names.toml"))?;
        let bone_map =
            crate::cc5::Cc5BoneMap::load(&inputs.defs_dir.join("cc5-bone-map.toml"), &palette)?;
        let mapped = crate::cc5::map_authored(&authored, &ir.bone_names, &bone_map)?;

        // Eyelids from the blink/lid morphs — CC5 has no lid bones.
        let head_material = head.name.clone();
        let wanted: Vec<&str> = crate::cc5::EYELID_MORPHS.iter().map(|(m, _)| *m).collect();
        let morphs = crate::glb::read_morph_targets(inputs.glb, &head_material, &wanted)?;
        let assignment = crate::cc5::eyelid_assignment(&buckets);
        let eyelids =
            crate::cc5::eyelid_weights_from_morphs(head.positions.len(), &morphs, &assignment);

        let head_slots: Vec<u32> = buckets.get("head").to_vec();
        let (blended, blend_report) = crate::cc5::blend(
            &mapped,
            &eyelids,
            &donor_transfer,
            &buckets,
            &head_slots,
            RW_MAX_INFLUENCES,
        );
        palette_influences = blended;
        authored_report = Some(AuthoredReport {
            source_bones: ir.bone_names.len(),
            morphs_found: eyelids
                .sources
                .iter()
                .map(|(n, c)| (n.clone(), *c))
                .collect(),
            blend: blend_report,
        });
    }

    let kept_eyelid: Vec<[f32; 3]> = palette_influences
        .iter()
        .enumerate()
        .filter(|(_, infl)| infl.iter().any(|(s, _)| eyelid_slots.contains(s)))
        .map(|(vi, _)| head.positions[vi])
        .collect();

    let gates = GateReport {
        lip_crease_found: source_crease.is_some() && donor_crease.is_some(),
        upper_lip_vertices: source_side
            .iter()
            .filter(|s| **s == crate::facial::LipSide::Upper)
            .count(),
        lower_lip_vertices: source_side
            .iter()
            .filter(|s| **s == crate::facial::LipSide::Lower)
            .count(),
        gate_rescues,
        lip_vertices_masked,
        lip_mass_masked,
        eyelid_vertices_masked,
        eyelid_mass_masked,
        eyelid_band_reach_mm,
        eyelid_vertices_kept: kept_eyelid.len(),
        eyelid_kept_extent_mm: crate::placement::extents(&kept_eyelid)
            .map(|e| [e[0] * 1000.0, e[1] * 1000.0, e[2] * 1000.0]),
    };

    let invented_slots: Vec<u32> = transfer_report
        .palette_slots_used
        .iter()
        .copied()
        .filter(|s| !donor_slots.contains(s))
        .collect();

    // A vanilla donor and a CC5 head are different meshes; a sub-millimetre fit across an
    // identical vertex count means the "donor" is a previous output of this very pipeline
    // (a converted head installed over the vanilla file). Detect it here, because every
    // downstream measurement is donor-relative and would otherwise look perfect while
    // being meaningless.
    let donor_is_already_this_head =
        donor_positions.len() == head.positions.len() && transfer_report.max_distance_mm < 1.0;

    // ---- Companion submeshes: measure, then discard -------------------------------
    //
    // The source's eyeballs, corneas, teeth and tongue never enter the container. RW
    // targets have no teeth or tongue slot, and the donor's own eyeballs and mouthbag are
    // not interchangeable parts — the eyeballs UV into the shared eye raster and animate
    // against the donor eyelid pocket, and the mouthbag is the target's mouth-interior
    // convention. What the source geometry *is* good for is telling us where those donor
    // submeshes belong and how big they should be, which is all we take from it.
    let pooled_source_eyes: Vec<[f32; 3]> = ir
        .submeshes
        .iter()
        .filter(|s| s.role == crate::ir::SubmeshRole::Eyes)
        .flat_map(|s| s.positions.iter().copied())
        .collect();
    // Teeth + tongue are the source's stand-in for the donor's single mouthbag volume.
    let pooled_source_mouth: Vec<[f32; 3]> = ir
        .submeshes
        .iter()
        .filter(|s| {
            matches!(
                s.role,
                crate::ir::SubmeshRole::Teeth
                    | crate::ir::SubmeshRole::Tongue
                    | crate::ir::SubmeshRole::Mouthbag
            )
        })
        .flat_map(|s| s.positions.iter().copied())
        .collect();
    let donor_submesh_positions = |role: crate::ir::SubmeshRole| -> Vec<[f32; 3]> {
        donor_asset
            .submeshes
            .iter()
            .filter(|s| crate::ir::SubmeshRole::classify(&s.name) == role)
            .flat_map(|s| s.vertices.iter().map(|v| v.position))
            .collect()
    };

    let alignment = if inputs.align_submeshes {
        // CC5 splits the eyes across several primitives (eyeball + cornea, L and R), so
        // pool them before the L/R split — one primitive alone is a single eye and would
        // bisect that eye instead of separating the pair.
        let eye_centers = crate::placement::split_eye_centers(&pooled_source_eyes);

        // Eye size: compare one eye against one eye. Pooled X extent would span the whole
        // pair (interpupillary distance included), so both sides are split first and the
        // two per-side ratios averaged into the single uniform factor the packer applies.
        let eye_scale =
            crate::placement::split_eye_clusters(&pooled_source_eyes).and_then(|(sl, sr)| {
                let donor_eyes = donor_submesh_positions(crate::ir::SubmeshRole::Eyes);
                let (dl, dr) = crate::placement::split_eye_clusters(&donor_eyes)?;
                let l = crate::placement::uniform_scale_from_width(&sl, &dl)?;
                let r = crate::placement::uniform_scale_from_width(&sr, &dr)?;
                Some((l + r) / 2.0)
            });

        let source_mouth = crate::placement::mouth_centroid(&head.positions, &head.triangles);
        let donor_mouth = crate::placement::mouth_centroid(&donor_positions, &donor_head.triangles);
        let mouth_delta = match (source_mouth, donor_mouth) {
            (Some(s), Some(d)) => Some([s[0] - d[0], s[1] - d[1], s[2] - d[2]]),
            _ => None,
        };
        let mouth_scale = crate::placement::uniform_scale_from_width(
            &pooled_source_mouth,
            &donor_submesh_positions(crate::ir::SubmeshRole::Mouthbag),
        );

        pack_rw::SubmeshAlignment {
            eye_centers,
            eye_scale,
            mouth_delta,
            mouth_scale,
        }
    } else {
        pack_rw::SubmeshAlignment::default()
    };

    // Everything the companions had to say is now in `alignment`; from here only the head
    // travels. Recorded so the report can state what was measured and dropped.
    let companions_dropped: Vec<CompanionMeasurement> = ir
        .submeshes
        .iter()
        .filter(|s| s.role != crate::ir::SubmeshRole::Head)
        .map(|s| CompanionMeasurement {
            name: s.name.clone(),
            role: format!("{:?}", s.role),
            vertices: s.positions.len(),
            extents_mm: crate::placement::extents(&s.positions)
                .map(|e| [e[0] * 1000.0, e[1] * 1000.0, e[2] * 1000.0]),
        })
        .collect();

    // Pack (strict re-parse inside) — unchanged from Path A.
    let (player_rx2, pack_report) = pack_rw::pack_head_into_donor(
        inputs.donor_rx2,
        &head,
        &palette_influences,
        PackRwOptions {
            collapse_haircap: inputs.collapse_haircap,
            // Same switch as the companions: if the eyes and mouthbag are being moved onto
            // the source's anatomy, the shell that carries the eyelashes has to follow, or
            // it keeps the donor's eye plane while the face moves out from under it.
            conform_haircap: inputs.align_submeshes,
            alignment,
        },
    )?;

    // Texture — CC5 atlases arrive as PNG/JPEG, so sniff the container.
    let head_rgba = inputs
        .head_texture
        .map(texture::decode_source_image)
        .transpose()?;
    let hair_rgba = inputs
        .hair_texture
        .map(texture::decode_source_image)
        .transpose()?;
    let texlib_rx2 = texture::build_legacy_texlib_rgba(
        inputs.donor_texlib,
        head_rgba.as_ref().map(|(d, w, h)| (d.as_slice(), *w, *h)),
        hair_rgba.as_ref().map(|(d, w, h)| (d.as_slice(), *w, *h)),
    )?;

    let _ = inputs.defs_dir;

    Ok(PathCOutput {
        player_rx2,
        texlib_rx2,
        bind_report,
        transfer_report,
        pack_report,
        invented_slots,
        companions_dropped,
        gates,
        authored: authored_report,
        donor_is_already_this_head,
        head,
    })
}
