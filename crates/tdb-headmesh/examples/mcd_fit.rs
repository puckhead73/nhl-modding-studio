//! `mcd_fit` — N19-H2b: align the Mogilny reference to the fingerprinted NHL 19 Eichel
//! donor, build the conservative protection masks, run one Path B fit, and emit a
//! **position-only offline diagnostic** container plus comparison renders.
//!
//! ```text
//! cargo run -p tdb-headmesh --example mcd_fit -- <donor.mcd> <source.glb> <out-dir>
//! cargo run -p tdb-headmesh --example mcd_fit -- <donor.mcd> <source.glb> <out-dir> --coordinated
//! ```
//!
//! What this is *not*: it is not a shipping packer. The container it writes carries the
//! donor's original normals, tangents and every opaque block verbatim, so the shading is
//! stale by construction and no lighting or animation claim can rest on it. Preview
//! images regenerate normals purely for review; native shading attributes are H3.
//!
//! Renders use one camera and scale for every image, so donor / fitted / source are
//! directly comparable.

use std::collections::HashSet;

use tdb_headmesh::nhl19_fit::{
    build_masks, fit_donor, frame_similarity, umeyama, DonorMasks, DonorMesh, FitParams,
    Similarity, SourceHead,
};
use tdb_rx2::mcd_write::{fingerprint, McdContainer};

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(donor_path), Some(src_path), Some(out_dir)) = (args.next(), args.next(), args.next())
    else {
        eprintln!("usage: mcd_fit <donor.mcd> <source.glb> <out-dir>");
        std::process::exit(2);
    };
    let coordinated = match args.next().as_deref() {
        None => false,
        Some("--coordinated") => true,
        Some(_) => die("optional fourth argument must be --coordinated"),
    };
    std::fs::create_dir_all(&out_dir).unwrap();

    let donor_bytes =
        std::fs::read(&donor_path).unwrap_or_else(|e| die(&format!("{donor_path}: {e}")));
    let src_bytes = std::fs::read(&src_path).unwrap_or_else(|e| die(&format!("{src_path}: {e}")));

    // Always start from the pristine fingerprinted donor: a previous output is not a
    // vanilla profile and the gate refuses it.
    let c = McdContainer::parse_with_profile(&donor_bytes)
        .unwrap_or_else(|e| die(&format!("donor refused: {e}")));
    let profile = c.profile.expect("gated");
    let donor_hash = fingerprint(&donor_bytes);
    let src_hash = fingerprint(&src_bytes);
    println!(
        "# donor   {donor_path}\n#   profile {} sha256 {donor_hash}",
        profile.name
    );
    println!("# source  {src_path}\n#   sha256 {src_hash}");

    let mesh = DonorMesh::from_container(&c, profile.head_submesh).expect("head submesh");
    println!(
        "# donor head: {} vertices, {} triangles, {} boundary components",
        mesh.positions.len(),
        mesh.triangles.len(),
        mesh.boundary_components.len()
    );

    // ------------------------------------------------------------------ 1. source
    let src = SourceHead::parse(&src_bytes).unwrap_or_else(|e| die(&format!("source: {e}")));
    println!("\n## Source scene (transform applied once per node)");
    for line in &src.provenance {
        println!("  {line}");
    }
    let (s_re, s_le) = src
        .eye_centres()
        .unwrap_or_else(|| die("source eyes have no two sides"));

    // ---------------------------------------------------------------- 2. alignment
    // Landmarks inside the overlapping face surface only. Donor bounds are NOT used:
    // they include torso and scalp the source does not have.
    let donor_eye = |name: &str| -> [f32; 3] {
        let i = c.submesh_index(name).expect("eyeball submesh");
        let (mn, mx) = c.submeshes[i].bounds.expect("eyeball bounds");
        [
            (mn[0] + mx[0]) * 0.5,
            (mn[1] + mx[1]) * 0.5,
            (mn[2] + mx[2]) * 0.5,
        ]
    };
    let d_re = donor_eye("geo_reyeShape");
    let d_le = donor_eye("geo_leyeShape");
    // Nose tip: the most forward vertex near the midline, between eye height and the
    // mouth. Same rule on both surfaces, so the landmark is defined, not eyeballed.
    let nose_tip = |pts: &[[f32; 3]], eye_y: f32, lo_y: f32| -> [f32; 3] {
        pts.iter()
            .filter(|p| p[0].abs() < 0.015 && p[1] < eye_y && p[1] > lo_y)
            .copied()
            .fold([0.0, 0.0, f32::MIN], |a, p| if p[2] > a[2] { p } else { a })
    };
    let d_eye_y = (d_re[1] + d_le[1]) * 0.5;
    let s_eye_y = (s_re[1] + s_le[1]) * 0.5;
    let d_inter = dist(d_re, d_le);
    let s_inter = dist(s_re, s_le);
    let d_nose = nose_tip(&mesh.positions, d_eye_y, d_eye_y - d_inter * 1.1);
    let s_nose = nose_tip(&src.head, s_eye_y, s_eye_y - s_inter * 1.1);

    let from = [s_re, s_le, s_nose];
    let to = [d_re, d_le, d_nose];
    println!("\n## Landmarks (source -> donor)");
    println!("  right_eye_centre\t{s_re:?}\t->\t{d_re:?}");
    println!("  left_eye_centre \t{s_le:?}\t->\t{d_le:?}");
    println!("  nose_tip        \t{s_nose:?}\t->\t{d_nose:?}");
    println!("  interocular distance: source {s_inter:.5} m, donor {d_inter:.5} m");

    // This explicit oriented frame avoids the negative-scale failure observed in our
    // three-point SVD implementation. Three non-collinear point correspondences do
    // determine a proper similarity; rank-two covariance is not itself a blocker.
    let mut xf = frame_similarity(s_re, s_le, s_nose, d_re, d_le, d_nose)
        .unwrap_or_else(|| die("landmark frame is degenerate"));
    report_transform("landmark frame similarity", &xf, &from, &to);

    // Refine with a few ICP rounds over the overlapping face surface only — the donor
    // vertices that are plausibly face, not torso or scalp.
    let face_window: Vec<[f32; 3]> = mesh
        .positions
        .iter()
        .copied()
        .filter(|p| p[1] > d_eye_y - d_inter * 2.2 && p[1] < d_eye_y + d_inter * 1.4 && p[2] > 0.0)
        .collect();
    for round in 0..4 {
        let moved: Vec<[f32; 3]> = src.head.iter().map(|&p| xf.apply(p)).collect();
        let Ok(hits) =
            tdb_headmesh::transfer::closest_points(&face_window, &moved, &src.head_triangles)
        else {
            break;
        };
        let mut a = Vec::new();
        let mut b = Vec::new();
        for (i, h) in hits.iter().enumerate() {
            if h.distance_mm > 12.0 {
                continue; // outside the overlap; not a correspondence
            }
            let t = h.tri;
            let mut foot = [0f32; 3];
            for k in 0..3 {
                foot[k] = h.bary[0] * moved[t[0] as usize][k]
                    + h.bary[1] * moved[t[1] as usize][k]
                    + h.bary[2] * moved[t[2] as usize][k];
            }
            a.push(foot);
            b.push(face_window[i]);
        }
        if a.len() < 32 {
            println!(
                "# ICP round {round}: only {} correspondences, stopping",
                a.len()
            );
            break;
        }
        let Some(step) = umeyama(&a, &b) else { break };
        // Compose: apply `step` after `xf`.
        xf = compose(&step, &xf);
        let rms = rms_after(&src.head, &src.head_triangles, &face_window, &xf);
        println!(
            "# ICP round {round}: {} correspondences, face RMS {:.4} mm",
            a.len(),
            rms
        );
    }
    report_transform("final similarity (landmarks + face ICP)", &xf, &from, &to);

    let aligned: Vec<[f32; 3]> = src.head.iter().map(|&p| xf.apply(p)).collect();
    let aligned_eyes: Vec<[f32; 3]> = src.eyes.iter().map(|&p| xf.apply(p)).collect();
    let (amn, amx) = bounds(&aligned);
    println!("# aligned source bounds min={amn:?} max={amx:?}");

    // Residual where the donor anatomy is *protected*, reported separately from the free
    // face: this is the likeness the frozen eyes cost us, stated in millimetres.
    let a_eyes_mesh = tdb_headmesh::nhl19_fit::SourceHead {
        head: aligned.clone(),
        head_triangles: src.head_triangles.clone(),
        head_uvs: src.head_uvs.clone(),
        eyes: aligned_eyes.clone(),
        eyes_triangles: src.eyes_triangles.clone(),
        provenance: vec![],
    };
    if let Some((a_re, a_le)) = a_eyes_mesh.eye_centres() {
        println!(
            "# protected-region residual after alignment: right eye centre {:.2} mm from the              donor eyeball, left eye {:.2} mm; source interocular {:.2} mm vs donor {:.2} mm              (difference {:+.2} mm)",
            dist(a_re, d_re) * 1000.0,
            dist(a_le, d_le) * 1000.0,
            dist(a_re, a_le) * 1000.0,
            d_inter * 1000.0,
            (dist(a_re, a_le) - d_inter) * 1000.0
        );
        if !coordinated {
            println!("# Conservative mode retains donor eye positions; --coordinated moves eyes and sockets together.");
        }
    }
    println!(
        "# free-face RMS after alignment: {:.3} mm",
        rms_after(&src.head, &src.head_triangles, &face_window, &xf)
    );

    // -------------------------------------------------------------------- 3. masks
    // Coverage is decided by whether a reference surface is actually there, within
    // COVERAGE_MM of the donor vertex — a bounding box would claim the torso is covered.
    const COVERAGE_MM: f64 = 6.0;
    let covered: HashSet<u32> = match tdb_headmesh::transfer::closest_points(
        &mesh.positions,
        &aligned,
        &src.head_triangles,
    ) {
        Ok(hits) => hits
            .iter()
            .enumerate()
            .filter(|(_, h)| h.distance_mm <= COVERAGE_MM)
            .map(|(i, _)| i as u32)
            .collect(),
        Err(e) => die(&format!("coverage correspondence failed: {e}")),
    };
    println!(
        "# source correspondence within {COVERAGE_MM:.0} mm for {} of {} donor vertices",
        covered.len(),
        mesh.positions.len()
    );
    let masks = build_masks(&c, &mesh, profile.head_submesh, &donor_hash, Some(&covered));
    if coordinated {
        run_coordinated(
            &c,
            &mesh,
            &masks,
            &src,
            &aligned,
            &aligned_eyes,
            &xf,
            &donor_bytes,
            &src_bytes,
            &out_dir,
        );
        return;
    }
    println!("\n## Masks");
    for s in &masks.sets {
        println!("  {:<28}{:>6}   {}", s.name, s.ids.len(), s.method);
    }
    println!(
        "  {:<28}{:>6}\n  {:<28}{:>6}",
        "HARD FROZEN (union)",
        masks.hard_frozen.len(),
        "EDITABLE (constructed)",
        masks.editable.len()
    );
    println!(
        "# seam coincidence: {} exact-bit groups, {} groups at {:.0e} m rounding, max intra-group \
         distance {:.3e} m",
        masks.seam_exact_groups,
        masks.seam_rounded_groups,
        tdb_headmesh::nhl19_fit::SEAM_ROUND_M,
        masks.seam_max_pair_distance
    );
    for n in &masks.notes {
        println!("# note: {n}");
    }
    let problems = masks.validate();
    if problems.is_empty() {
        println!("# masks validate: all ids in range, no duplicates, editable ∩ hard-frozen = ∅");
    } else {
        for p in &problems {
            eprintln!("# MASK PROBLEM: {p}");
        }
        die("masks failed validation");
    }
    write_masks_json(
        &format!("{out_dir}/masks.json"),
        &masks,
        &donor_hash,
        &src_hash,
        &xf,
    );

    if masks.editable.is_empty() {
        die("no editable surface remains — refusing to claim a fit");
    }

    // ---------------------------------------------------------------------- 4. fit
    let params = FitParams::default();
    println!("\n## Fit");
    println!(
        "# cap {:.1} mm, max correspondence {:.1} mm, falloff {} rings, {} iterations, smoothing \
         {} x {:.2}, min normal dot {:.2}",
        params.max_displacement_m * 1000.0,
        params.max_correspondence_m * 1000.0,
        params.falloff_rings,
        params.iterations,
        params.smooth_iterations,
        params.smooth_factor,
        params.min_normal_dot
    );
    let (fitted, rep) = fit_donor(&mesh, &masks, &aligned, &src.head_triangles, params);
    println!("editable\t{}", rep.editable);
    println!("moved\t{}", rep.moved);
    println!("no_correspondence\t{}", rep.no_correspondence);
    println!("displacement_capped\t{}", rep.capped);
    println!(
        "max_displacement_mm\t{:.3}",
        rep.max_displacement_m * 1000.0
    );
    println!(
        "mean_displacement_mm\t{:.3}",
        rep.mean_displacement_m * 1000.0
    );
    println!(
        "max_motion_at_protected_boundary_mm\t{:.3}",
        rep.max_boundary_motion_m * 1000.0
    );
    println!(
        "editable_at_full_influence\t{}\neditable_damped_by_falloff\t{}\nmean_influence\t{:.3}",
        rep.editable_at_full_influence, rep.editable_damped, rep.mean_influence
    );
    println!("hard_frozen_bit_identical\t{}", rep.frozen_bit_identical);
    println!("new_inverted_triangles\t{}", rep.new_inverted_triangles);
    println!("new_degenerate_triangles\t{}", rep.new_degenerate_triangles);
    println!("pre_existing_degenerate\t{}", rep.pre_existing_degenerate);
    println!(
        "largest_displacements (vertex, mm)\t{:?}",
        rep.outliers
            .iter()
            .map(|(v, d)| (*v, (d * 1000.0 * 1000.0).round() / 1000.0))
            .collect::<Vec<_>>()
    );
    if !rep.frozen_bit_identical {
        die("a hard-frozen position moved — refusing to emit a candidate");
    }

    // ------------------------------------------------------------------- 5. export
    let out = tdb_headmesh::pack_mcd::pack_positions_into_donor(
        &donor_bytes,
        profile.head_submesh,
        &fitted,
    )
    .unwrap_or_else(|e| die(&format!("pack: {e}")));
    let out_path = format!("{out_dir}/mogilny_on_eichel_POSITION_ONLY_DIAGNOSTIC.mcd");
    std::fs::write(&out_path, &out).unwrap();
    println!("\n# wrote {out_path}");
    println!("#   sha256 {}", fingerprint(&out));
    println!(
        "#   {} bytes (donor {} bytes)",
        out.len(),
        donor_bytes.len()
    );

    // Independent re-read: counts, ownership, protected positions.
    let re = McdContainer::parse(&out).unwrap_or_else(|e| die(&format!("reparse: {e}")));
    let re_sub = re.submesh_index(profile.head_submesh).unwrap();
    let re_stream = re.position_stream_of(re_sub).unwrap();
    let rp = re.streams[re_stream].positions();
    let same_counts = re.submeshes.len() == c.submeshes.len()
        && re
            .submeshes
            .iter()
            .zip(&c.submeshes)
            .all(|(a, b)| a.triangles == b.triangles && a.streams == b.streams);
    let frozen_ok = masks.hard_frozen.iter().all(|&v| {
        (0..3).all(|k| rp[v as usize][k].to_bits() == mesh.positions[v as usize][k].to_bits())
    });
    println!("# reparse: counts_and_ownership_preserved={same_counts} hard_frozen_bit_identical={frozen_ok}");
    let diffs = out.iter().zip(&donor_bytes).filter(|(a, b)| a != b).count();
    let spans = c.writable_spans();
    let outside = out
        .iter()
        .zip(&donor_bytes)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .filter(|(i, _)| !spans.iter().any(|&(s, e)| *i >= s && *i < e))
        .count();
    println!("# byte diff vs donor: {diffs} bytes, {outside} outside the writable allowlist");
    if !same_counts || !frozen_ok || outside != 0 {
        die("post-write verification failed");
    }

    // ------------------------------------------------------------------ 6. renders
    let cam = Camera::for_face(&mesh.positions, d_eye_y, d_inter);
    let shots: [(&str, f32, bool); 4] = [
        ("front", 0.0, false),
        ("profile", 90.0, false),
        ("threequarter", 35.0, false),
        ("cutaway", 90.0, true),
    ];
    for (name, yaw, cut) in shots {
        for (tag, pts, tris) in [
            ("donor", &mesh.positions, &mesh.triangles),
            ("fitted", &fitted, &mesh.triangles),
            ("source", &aligned, &src.head_triangles),
        ] {
            let img = cam.shade(pts, tris, yaw, cut);
            let p = format!("{out_dir}/{name}_{tag}.png");
            img.save(&p).unwrap();
        }
    }
    // Close-ups on the two regions the protection exists for.
    let eye_cam = Camera::close_up(d_le, d_inter * 0.75);
    for (tag, pts) in [("donor", &mesh.positions), ("fitted", &fitted)] {
        eye_cam
            .shade(pts, &mesh.triangles, 0.0, false)
            .save(format!("{out_dir}/eye_closeup_{tag}.png"))
            .unwrap();
    }
    let mouth_centre = centroid(&mesh.positions, masks.get("oral_rim"));
    let mouth_cam = Camera::close_up(mouth_centre, d_inter * 0.9);
    for (tag, pts) in [("donor", &mesh.positions), ("fitted", &fitted)] {
        mouth_cam
            .shade(pts, &mesh.triangles, 25.0, false)
            .save(format!("{out_dir}/mouth_closeup_{tag}.png"))
            .unwrap();
    }
    // Displacement heat map: where the fit actually acted.
    let heat = cam.heat(
        &mesh.positions,
        &mesh.triangles,
        &fitted,
        params.max_displacement_m,
    );
    heat.save(format!("{out_dir}/displacement_front.png"))
        .unwrap();

    println!("# wrote comparison renders (same camera and scale) to {out_dir}");
    println!(
        "\n# NOTE: the emitted container is a POSITION-ONLY diagnostic. Normals, tangents and all \
         opaque blocks are the donor's, so its shading is stale by construction. Preview images \
         regenerate normals for review only. Native shading attributes, texture bake and the \
         runtime A/B test are later packages."
    );
}

fn die(msg: &str) -> ! {
    eprintln!("# ERROR: {msg}");
    std::process::exit(1)
}

// Shares only the offline renderer/alignment helpers; the original conservative path
// remains available without the explicit flag.
include!("support/coordinated_fit.rs");

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn bounds(v: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut mn = [f32::MAX; 3];
    let mut mx = [f32::MIN; 3];
    for p in v {
        for k in 0..3 {
            mn[k] = mn[k].min(p[k]);
            mx[k] = mx[k].max(p[k]);
        }
    }
    (mn, mx)
}

fn centroid(pos: &[[f32; 3]], ids: &[u32]) -> [f32; 3] {
    if ids.is_empty() {
        return [0.0; 3];
    }
    let mut a = [0f64; 3];
    for &v in ids {
        for k in 0..3 {
            a[k] += pos[v as usize][k] as f64;
        }
    }
    [
        (a[0] / ids.len() as f64) as f32,
        (a[1] / ids.len() as f64) as f32,
        (a[2] / ids.len() as f64) as f32,
    ]
}

fn compose(after: &Similarity, before: &Similarity) -> Similarity {
    let mut rot = [[0f32; 3]; 3];
    for (i, row) in rot.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| after.rot[i][k] * before.rot[k][j]).sum();
        }
    }
    let mut t = [0f32; 3];
    for (k, cell) in t.iter_mut().enumerate() {
        *cell = after.scale
            * (after.rot[k][0] * before.translation[0]
                + after.rot[k][1] * before.translation[1]
                + after.rot[k][2] * before.translation[2])
            + after.translation[k];
    }
    Similarity {
        scale: after.scale * before.scale,
        rot,
        translation: t,
    }
}

fn report_transform(label: &str, xf: &Similarity, from: &[[f32; 3]], to: &[[f32; 3]]) {
    println!("\n## {label}");
    println!(
        "  scale\t{:.6}  (dimensionless; both spaces are metres)",
        xf.scale
    );
    for r in 0..3 {
        println!(
            "  rot[{r}]\t[{:+.6}, {:+.6}, {:+.6}]\tt[{r}] {:+.6}",
            xf.rot[r][0], xf.rot[r][1], xf.rot[r][2], xf.translation[r]
        );
    }
    let mut worst = 0f32;
    for (a, b) in from.iter().zip(to) {
        worst = worst.max(dist(xf.apply(*a), *b));
    }
    println!("  landmark residual (max)\t{:.4} mm", worst * 1000.0);
}

fn rms_after(src: &[[f32; 3]], tris: &[[u32; 3]], query: &[[f32; 3]], xf: &Similarity) -> f32 {
    let moved: Vec<[f32; 3]> = src.iter().map(|&p| xf.apply(p)).collect();
    let Ok(hits) = tdb_headmesh::transfer::closest_points(query, &moved, tris) else {
        return f32::NAN;
    };
    let d: Vec<f64> = hits
        .iter()
        .map(|h| h.distance_mm)
        .filter(|d| *d < 12.0)
        .collect();
    if d.is_empty() {
        return f32::NAN;
    }
    ((d.iter().map(|x| x * x).sum::<f64>()) / d.len() as f64).sqrt() as f32
}

fn write_masks_json(path: &str, m: &DonorMasks, donor_hash: &str, src_hash: &str, xf: &Similarity) {
    let mut s = String::from("{\n");
    s.push_str(&format!("  \"donor_sha256\": \"{donor_hash}\",\n"));
    s.push_str(&format!("  \"source_sha256\": \"{src_hash}\",\n"));
    s.push_str(&format!("  \"submesh\": \"{}\",\n", m.submesh));
    s.push_str(&format!("  \"vertex_count\": {},\n", m.vertex_count));
    s.push_str("  \"vertex_id_space\": \"donor stream order, unchanged\",\n");
    s.push_str(&format!(
        "  \"seam_exact_groups\": {},\n",
        m.seam_exact_groups
    ));
    s.push_str(&format!(
        "  \"seam_rounded_groups\": {},\n  \"seam_round_m\": {},\n  \"seam_max_pair_distance_m\": {},\n",
        m.seam_rounded_groups,
        tdb_headmesh::nhl19_fit::SEAM_ROUND_M,
        m.seam_max_pair_distance
    ));
    s.push_str(&format!(
        "  \"alignment\": {{ \"scale\": {}, \"rot\": {:?}, \"translation\": {:?} }},\n",
        xf.scale, xf.rot, xf.translation
    ));
    s.push_str("  \"masks\": {\n");
    for (i, set) in m.sets.iter().enumerate() {
        let ids: Vec<String> = set.ids.iter().map(|v| v.to_string()).collect();
        s.push_str(&format!(
            "    \"{}\": {{ \"method\": \"{}\", \"count\": {}, \"ids\": [{}] }}",
            set.name,
            set.method,
            set.ids.len(),
            ids.join(",")
        ));
        s.push_str(if i + 1 == m.sets.len() { "\n" } else { ",\n" });
    }
    s.push_str("  },\n");
    let mut hf: Vec<u32> = m.hard_frozen.iter().copied().collect();
    hf.sort_unstable();
    let mut ed: Vec<u32> = m.editable.iter().copied().collect();
    ed.sort_unstable();
    s.push_str(&format!(
        "  \"hard_frozen\": [{}],\n  \"editable\": [{}]\n}}\n",
        hf.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(","),
        ed.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    ));
    std::fs::write(path, s).unwrap();
    println!("# wrote {path}");
}

// ---------------------------------------------------------------------- rendering

/// One fixed orthographic camera, shared by every render so the comparisons line up.
struct Camera {
    centre: [f32; 3],
    half: f32,
    w: u32,
    h: u32,
}

impl Camera {
    fn for_face(pos: &[[f32; 3]], eye_y: f32, inter: f32) -> Self {
        let _ = pos;
        Self {
            centre: [0.0, eye_y - inter * 0.35, 0.05],
            half: inter * 2.1,
            w: 800,
            h: 1000,
        }
    }

    fn close_up(centre: [f32; 3], half: f32) -> Self {
        Self {
            centre,
            half,
            w: 700,
            h: 700,
        }
    }

    /// Solid-shaded render with a z-buffer and regenerated normals. `yaw` rotates about
    /// Y; `cut` drops the +x half so interiors are visible.
    fn shade(
        &self,
        pos: &[[f32; 3]],
        tris: &[[u32; 3]],
        yaw_deg: f32,
        cut: bool,
    ) -> image::RgbImage {
        let (w, h) = (self.w, self.h);
        let mut img = image::RgbImage::from_pixel(w, h, image::Rgb([16, 16, 20]));
        let mut zbuf = vec![f32::MIN; (w * h) as usize];
        let yaw = yaw_deg.to_radians();
        let (sy, cy) = (yaw.sin(), yaw.cos());
        let view = |p: [f32; 3]| -> [f32; 3] {
            let d = [
                p[0] - self.centre[0],
                p[1] - self.centre[1],
                p[2] - self.centre[2],
            ];
            [d[0] * cy + d[2] * sy, d[1], -d[0] * sy + d[2] * cy]
        };
        let aspect = h as f32 / w as f32;
        let project = |v: [f32; 3]| -> (f32, f32) {
            (
                (v[0] / self.half * 0.5 + 0.5) * w as f32,
                (0.5 - v[1] / (self.half * aspect) * 0.5) * h as f32,
            )
        };
        // Regenerated normals — preview only; the container keeps the donor's.
        let mesh = DonorMesh::new(pos.to_vec(), tris.to_vec());
        let normals = mesh.vertex_normals();
        let light = {
            let l = [-0.35f32, 0.45, 1.0];
            let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
            [l[0] / n, l[1] / n, l[2] / n]
        };
        for t in tris {
            if cut && t.iter().any(|&v| pos[v as usize][0] > 0.0) {
                continue;
            }
            let vs: Vec<[f32; 3]> = t.iter().map(|&v| view(pos[v as usize])).collect();
            let ps: Vec<(f32, f32)> = vs.iter().map(|&v| project(v)).collect();
            let ns: Vec<[f32; 3]> = t
                .iter()
                .map(|&v| {
                    let n = normals[v as usize];
                    [n[0] * cy + n[2] * sy, n[1], -n[0] * sy + n[2] * cy]
                })
                .collect();
            let minx = ps
                .iter()
                .map(|p| p.0)
                .fold(f32::MAX, f32::min)
                .floor()
                .max(0.0) as u32;
            let maxx = (ps.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as i64)
                .clamp(0, w as i64 - 1) as u32;
            let miny = ps
                .iter()
                .map(|p| p.1)
                .fold(f32::MAX, f32::min)
                .floor()
                .max(0.0) as u32;
            let maxy = (ps.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i64)
                .clamp(0, h as i64 - 1) as u32;
            let den = (ps[1].1 - ps[2].1) * (ps[0].0 - ps[2].0)
                + (ps[2].0 - ps[1].0) * (ps[0].1 - ps[2].1);
            if den.abs() < 1e-9 {
                continue;
            }
            for py in miny..=maxy {
                for px in minx..=maxx {
                    let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                    let l0 = ((ps[1].1 - ps[2].1) * (fx - ps[2].0)
                        + (ps[2].0 - ps[1].0) * (fy - ps[2].1))
                        / den;
                    let l1 = ((ps[2].1 - ps[0].1) * (fx - ps[2].0)
                        + (ps[0].0 - ps[2].0) * (fy - ps[2].1))
                        / den;
                    let l2 = 1.0 - l0 - l1;
                    if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 {
                        continue;
                    }
                    let z = l0 * vs[0][2] + l1 * vs[1][2] + l2 * vs[2][2];
                    let ix = (py * w + px) as usize;
                    if z <= zbuf[ix] {
                        continue;
                    }
                    zbuf[ix] = z;
                    let mut n = [0f32; 3];
                    for k in 0..3 {
                        n[k] = l0 * ns[0][k] + l1 * ns[1][k] + l2 * ns[2][k];
                    }
                    let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
                    let lam = ((n[0] * light[0] + n[1] * light[1] + n[2] * light[2]) / nl).max(0.0);
                    let v = (28.0 + 205.0 * (0.15 + 0.85 * lam)).min(255.0) as u8;
                    img.put_pixel(px, py, image::Rgb([v, v, (v as f32 * 0.97) as u8]));
                }
            }
        }
        img
    }

    /// Displacement magnitude as colour, over the shaded donor.
    fn heat(
        &self,
        pos: &[[f32; 3]],
        tris: &[[u32; 3]],
        fitted: &[[f32; 3]],
        cap: f32,
    ) -> image::RgbImage {
        let mut img = self.shade(pos, tris, 0.0, false);
        let (w, h) = (self.w, self.h);
        let project = |p: [f32; 3]| -> (i32, i32) {
            let d = [p[0] - self.centre[0], p[1] - self.centre[1]];
            let aspect = h as f32 / w as f32;
            (
                ((d[0] / self.half * 0.5 + 0.5) * w as f32) as i32,
                ((0.5 - d[1] / (self.half * aspect) * 0.5) * h as f32) as i32,
            )
        };
        let moved: HashSet<u32> = (0..pos.len() as u32)
            .filter(|&v| dist(pos[v as usize], fitted[v as usize]) > 0.0)
            .collect();
        for v in moved {
            let d = dist(pos[v as usize], fitted[v as usize]) / cap.max(1e-9);
            let t = d.clamp(0.0, 1.0);
            let col = [
                (40.0 + 215.0 * t) as u8,
                (200.0 * (1.0 - t) + 40.0 * t) as u8,
                (60.0 * (1.0 - t)) as u8,
            ];
            let (x, y) = project(pos[v as usize]);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (px, py) = (x + dx, y + dy);
                    if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                        img.put_pixel(px as u32, py as u32, image::Rgb(col));
                    }
                }
            }
        }
        img
    }
}
