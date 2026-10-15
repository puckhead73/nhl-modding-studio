// Offline coordinated-unit experiment requested after the frozen fit retained too
// much of Eichel. Native animation remains unverified; never deploy this implicitly.

fn unit_add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn unit_sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

// Reference-only positional weld, for comparable smooth shading and boundary
// inspection. Never used to reorder or weld native donor vertices.
fn reference_surface(pos: &[[f32; 3]], tris: &[[u32; 3]]) -> DonorMesh {
    let mut map = std::collections::BTreeMap::new();
    let mut points = Vec::new();
    let ids: Vec<u32> = pos
        .iter()
        .map(|p| {
            let key = p.map(f32::to_bits);
            *map.entry(key).or_insert_with(|| {
                let i = points.len() as u32;
                points.push(*p);
                i
            })
        })
        .collect();
    DonorMesh::new(
        points,
        tris.iter().map(|t| t.map(|i| ids[i as usize])).collect(),
    )
}

// A harmonic displacement field preserves each protected unit as a translation,
// blends between units, and leaves shell anchors exactly in place. It smooths
// displacements, not the original surface (which would erase facial features).
fn translate_units(mesh: &DonorMesh, fixed: &[Option<[f32; 3]>]) -> Vec<[f32; 3]> {
    assert_eq!(fixed.len(), mesh.positions.len());
    let mut disp: Vec<[f32; 3]> = fixed.iter().map(|d| d.unwrap_or([0.0; 3])).collect();
    for _ in 0..350 {
        let old = disp.clone();
        for i in 0..disp.len() {
            if let Some(d) = fixed[i] {
                disp[i] = d;
                continue;
            }
            if mesh.ring[i].is_empty() {
                continue;
            }
            let mut sum = [0.0; 3];
            let mut total = 0.0;
            for &j in &mesh.ring[i] {
                let w = 1.0 / dist(mesh.positions[i], mesh.positions[j as usize]).max(0.001);
                for k in 0..3 {
                    sum[k] += old[j as usize][k] * w;
                }
                total += w;
            }
            disp[i] = sum.map(|v| v / total);
        }
    }
    mesh.positions
        .iter()
        .zip(disp)
        .map(|(&p, d)| if d == [0.0; 3] { p } else { unit_add(p, d) })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn run_coordinated(
    c: &McdContainer,
    mesh: &DonorMesh,
    masks: &DonorMasks,
    source: &SourceHead,
    aligned: &[[f32; 3]],
    aligned_eyes: &[[f32; 3]],
    xf: &Similarity,
    donor_bytes: &[u8],
    source_bytes: &[u8],
    out_dir: &str,
) {
    let mut manifest = serde_json::json!({
        "status": "offline position-only coordinated-unit experiment; animation and native shading unverified",
        "donor_sha256": fingerprint(donor_bytes), "source_sha256": fingerprint(source_bytes),
        "alignment": {"scale": xf.scale,"rotation": xf.rot,"translation": xf.translation},
    });
    let eye = |name: &str| {
        let sub = c.submesh_index(name).expect("eye");
        let s = c.position_stream_of(sub).expect("positions");
        let (lo, hi) = bounds(c.streams[s].positions());
        [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5)
    };
    // Use the same geometric statistic for donor and source: per-eye bounds midpoint.
    // Source triangle-soup centroids are biased by vertex duplication.
    let source_eye = |positive: bool| {
        let p: Vec<_> = aligned_eyes
            .iter()
            .copied()
            .filter(|p| (p[0] >= 0.0) == positive)
            .collect();
        assert!(!p.is_empty());
        let (lo, hi) = bounds(&p);
        [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5)
    };
    let dr = eye("geo_reyeShape");
    let dl = eye("geo_leyeShape");
    let sr = source_eye(false);
    let sl = source_eye(true);
    let er = unit_sub(sr, dr);
    let el = unit_sub(sl, dl);
    println!("COORDINATED eyes: right {dr:?} -> {sr:?}, left {dl:?} -> {sl:?}");
    println!(
        "eye shifts mm: right {:.3}, left {:.3}",
        dist(dr, sr) * 1000.0,
        dist(dl, sl) * 1000.0
    );
    assert!(
        dist(dr, sr) < 0.015 && dist(dl, sl) < 0.015,
        "eye landmark mismatch exceeds experimental limit"
    );

    let ref_mesh = reference_surface(aligned, &source.head_triangles);
    let dm = centroid(&mesh.positions, masks.get("oral_rim"));
    // Print every source boundary before selecting: a mouth landmark must be in the
    // mouth window and close to the donor opening. Do not substitute arbitrary skin.
    let mut mouth_candidates = Vec::new();
    for ids in &ref_mesh.boundary_components {
        let centre = centroid(&ref_mesh.positions, ids);
        println!(
            "source boundary count={} centre={centre:?} donor-mouth-distance-mm={:.3}",
            ids.len(),
            dist(centre, dm) * 1000.0
        );
        if ids.len() >= 12
            && (centre[1] - dm[1]).abs() < 0.018
            && centre[0].abs() < 0.012
            && (centre[2] - dm[2]).abs() < 0.025
        {
            mouth_candidates.push((dist(centre, dm), centre, ids.len()));
        }
    }
    mouth_candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Unresolved source seam means keep the complete donor mouth in place this pass.
    let (sm, mouth_method) = mouth_candidates
        .first()
        .map(|m| (m.1, "source mouth-window boundary centroid"))
        .unwrap_or((
            dm,
            "source mouth boundary unresolved: donor cavity retained",
        ));
    let em = unit_sub(sm, dm);
    println!("mouth {dm:?} -> {sm:?}, method={mouth_method}");
    assert!(
        dist(dm, sm) < 0.015,
        "mouth shift beyond bounded experiment"
    );

    let mut fixed = vec![None; mesh.positions.len()];
    let shell: HashSet<u32> = [
        "shell_boundary",
        "shell_transition",
        "unclassified_boundary",
    ]
    .iter()
    .flat_map(|n| masks.get(n).iter().copied())
    .collect();
    for &i in &shell {
        fixed[i as usize] = Some([0.0; 3]);
    }
    // Same translation for the entire cavity/seam, including ambiguous coincident IDs.
    let oral: HashSet<u32> = [
        "oral_rim",
        "oral_cavity",
        "oral_transition",
        "oral_seam_ambiguous",
    ]
    .iter()
    .flat_map(|n| masks.get(n).iter().copied())
    .collect();
    let mut right: HashSet<u32> = masks.get("eyelid_pocket_right").iter().copied().collect();
    let mut left: HashSet<u32> = masks.get("eyelid_pocket_left").iter().copied().collect();
    // The old generously widened spheres overlap across the nasal bridge. Keep the
    // actual sockets intact, but let bridge vertices blend rather than imposing two
    // contradictory translations there. Record this changed transition explicitly.
    let bridge: Vec<u32> = right.intersection(&left).copied().collect();
    println!(
        "overlapping eye transition vertices blended: {}",
        bridge.len()
    );
    for v in &bridge {
        right.remove(v);
        left.remove(v);
    }
    let oral_eye: Vec<u32> = oral
        .iter()
        .copied()
        .filter(|v| right.contains(v) || left.contains(v))
        .collect();
    println!(
        "oral/eye broad-transition overlaps retained with the oral unit: {}",
        oral_eye.len()
    );
    // A broad transition is not the inner pocket. Oral membership takes precedence
    // only beyond 25 mm from both eye centres; a core conflict still refuses the fit.
    for v in oral_eye {
        assert!(
            dist(mesh.positions[v as usize], dr).min(dist(mesh.positions[v as usize], dl)) > 0.025,
            "core eye/cavity mask conflict"
        );
        right.remove(&v);
        left.remove(&v);
    }
    for (ids, d) in [(&oral, em), (&right, er), (&left, el)] {
        assert!(ids.is_disjoint(&shell), "unit overlaps attachment");
        for &i in ids {
            fixed[i as usize] = Some(d);
        }
    }
    // Unsupported lower shell stays anchored; side-facing face points are not frozen
    // solely because their Z normal is negative.
    for (i, p) in mesh.positions.iter().enumerate() {
        if p[1] < dm[1] - 0.085 && fixed[i].is_none() {
            fixed[i] = Some([0.0; 3]);
        }
    }
    let moved = translate_units(mesh, &fixed);
    let unit_mesh = DonorMesh::new(moved.clone(), mesh.triangles.clone());
    let normals = unit_mesh.vertex_normals();
    let hits = tdb_headmesh::transfer::closest_points(&moved, aligned, &source.head_triangles)
        .expect("coverage");
    let mut fit_masks = masks.clone();
    fit_masks.hard_frozen.clear();
    fit_masks.editable.clear();
    for i in 0..moved.len() {
        // Admit side surfaces with a nearby, similarly oriented source triangle.
        let h = hits[i];
        let t = h.tri;
        let a = unit_sub(aligned[t[1] as usize], aligned[t[0] as usize]);
        let b = unit_sub(aligned[t[2] as usize], aligned[t[0] as usize]);
        let n = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        let len = dist(n, [0.0; 3]);
        let dot = (0..3).map(|k| n[k] * normals[i][k]).sum::<f32>() / len.max(1e-12);
        let face = moved[i][1] > dm[1] - 0.080 && moved[i][2] > -0.020;
        if fixed[i].is_none() && face && h.distance_mm < 12.0 && dot > 0.2 {
            fit_masks.editable.insert(i as u32);
        } else {
            fit_masks.hard_frozen.insert(i as u32);
        }
    }
    let params = FitParams {
        falloff_rings: 3,
        iterations: 4,
        smooth_factor: 0.10,
        smooth_iterations: 0,
        ..FitParams::default()
    };
    let (mut fitted, report) = fit_donor(
        &unit_mesh,
        &fit_masks,
        aligned,
        &source.head_triangles,
        params,
    );
    println!("COORDINATED fit report {report:?}");
    assert!(
        report.frozen_bit_identical,
        "unit anchor moved during exterior fit"
    );
    // Inversion/degenerate checks must also compare against pristine, not just moved.
    let quality = |pos: &[[f32; 3]]| {
        let mut flipped = 0;
        let mut collapsed = 0;
        for t in &mesh.triangles {
            let n = |p: &[[f32; 3]]| {
                let a = unit_sub(p[t[1] as usize], p[t[0] as usize]);
                let b = unit_sub(p[t[2] as usize], p[t[0] as usize]);
                [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ]
            };
            let before = n(&mesh.positions);
            let after = n(pos);
            if dist(after, [0.0; 3]) < 1e-12 {
                collapsed += 1;
            }
            if (0..3).map(|k| before[k] * after[k]).sum::<f32>() < 0.0 {
                flipped += 1;
            }
        }
        (flipped, collapsed)
    };
    assert_eq!(
        quality(&moved),
        (0, 0),
        "rigid-unit transition itself failed orientation checks"
    );
    // Local backtracking: reduce the exterior fit around bad triangles, while leaving
    // all unit targets exactly fixed. This cannot silently accept a failing native file.
    let mut backed_off = HashSet::new();
    for _ in 0..32 {
        let mut bad = HashSet::new();
        for t in &mesh.triangles {
            let normal = |p: &[[f32; 3]]| {
                let a = unit_sub(p[t[1] as usize], p[t[0] as usize]);
                let b = unit_sub(p[t[2] as usize], p[t[0] as usize]);
                [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ]
            };
            let a = normal(&mesh.positions);
            let b = normal(&fitted);
            if dist(b, [0.0; 3]) < 1e-12 || (0..3).map(|k| a[k] * b[k]).sum::<f32>() <= 0.0 {
                for &v in t {
                    if fit_masks.editable.contains(&v) {
                        bad.insert(v);
                    }
                }
            }
        }
        if bad.is_empty() {
            break;
        }
        let expanded: HashSet<u32> = bad
            .iter()
            .flat_map(|&v| mesh.ring[v as usize].iter().copied())
            .chain(bad.iter().copied())
            .filter(|v| fit_masks.editable.contains(v))
            .collect();
        for v in expanded {
            backed_off.insert(v);
            for k in 0..3 {
                fitted[v as usize][k] = (fitted[v as usize][k] + moved[v as usize][k]) * 0.5;
            }
        }
    }
    let (flipped, collapsed) = quality(&fitted);
    println!("pristine-to-fit orientation reversals={flipped} collapsed={collapsed}");
    let mut eval_ids: Vec<_> = fit_masks.editable.iter().copied().collect();
    eval_ids.sort_unstable();
    let rms = |p: &[[f32; 3]]| {
        let query: Vec<_> = eval_ids.iter().map(|&i| p[i as usize]).collect();
        let hits = tdb_headmesh::transfer::closest_points(&query, aligned, &source.head_triangles)
            .unwrap();
        (hits
            .iter()
            .map(|h| h.distance_mm * h.distance_mm)
            .sum::<f64>()
            / hits.len() as f64)
            .sqrt()
    };
    manifest["eyes"] = serde_json::json!({"right_before":dr,"right_target":sr,"right_translation":er,"left_before":dl,"left_target":sl,"left_translation":el});
    manifest["mouth"] =
        serde_json::json!({"before":dm,"target":sm,"translation":em,"method":mouth_method});
    manifest["checks"] = serde_json::json!({"rigid_unit_anchors_exact":report.frozen_bit_identical,"orientation_reversals":flipped,"collapsed_triangles":collapsed,"editable":fit_masks.editable.len(),"locally_backed_off_vertices":backed_off.len(),"rms_same_editable_donor_mm":rms(&mesh.positions),"rms_same_editable_fitted_mm":rms(&fitted),"max_projection_displacement_mm_before_backtracking":report.max_displacement_m*1000.0});
    println!("{}", manifest["checks"]);

    // Build all companion translations in one parsed vanilla container, never parsing
    // a previous output as a recognised donor or silently modifying the rig tables.
    let mut out_c = c.clone();
    let head = out_c.submesh_index("geo_headShape").unwrap();
    out_c
        .set_positions(out_c.position_stream_of(head).unwrap(), &fitted)
        .unwrap();
    out_c.refresh_bounds(head);
    for (name, delta) in [("geo_reyeShape", er), ("geo_leyeShape", el)] {
        let i = out_c.submesh_index(name).unwrap();
        let st = out_c.position_stream_of(i).unwrap();
        let p: Vec<_> = c.streams[st]
            .positions()
            .iter()
            .map(|&p| unit_add(p, delta))
            .collect();
        out_c.set_positions(st, &p).unwrap();
        out_c.refresh_bounds(i);
    }
    if let Some(i) = out_c.submesh_index("geo_eyelashesShape") {
        let st = out_c.position_stream_of(i).unwrap();
        let p: Vec<_> = c.streams[st]
            .positions()
            .iter()
            .map(|&p| unit_add(p, if dist(p, dr) < dist(p, dl) { er } else { el }))
            .collect();
        out_c.set_positions(st, &p).unwrap();
        out_c.refresh_bounds(i);
    }
    let packed = out_c.serialize();
    let reparsed = McdContainer::parse(&packed).expect("edited structural reparse");
    assert_eq!(reparsed.streams.len(), c.streams.len());
    let packed_head = reparsed.streams[reparsed.position_stream_of(head).unwrap()].positions();
    for (ids, delta) in [(&right, er), (&left, el), (&oral, em), (&shell, [0.0; 3])] {
        for &v in ids {
            let expected = if delta == [0.0; 3] {
                mesh.positions[v as usize]
            } else {
                unit_add(mesh.positions[v as usize], delta)
            };
            assert_eq!(
                packed_head[v as usize], expected,
                "unit anchor mismatch after packing"
            );
        }
    }
    for (name, target) in [("geo_reyeShape", sr), ("geo_leyeShape", sl)] {
        let i = reparsed.submesh_index(name).unwrap();
        let st = reparsed.position_stream_of(i).unwrap();
        let (lo, hi) = bounds(reparsed.streams[st].positions());
        let centre = [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5);
        let residual = dist(centre, target) * 1000.0;
        assert!(residual < 0.001, "packed eye missed target");
        manifest["eyes"][format!("{name}_packed_residual_mm")] = serde_json::json!(residual);
    }
    let spans = c.writable_spans();
    assert!(packed
        .iter()
        .zip(donor_bytes)
        .enumerate()
        .all(|(i, (a, b))| a == b || spans.iter().any(|&(s, e)| i >= s && i < e)));
    manifest["output_sha256"] = serde_json::json!(fingerprint(&packed));
    let sorted = |set: &HashSet<u32>| {
        let mut v: Vec<_> = set.iter().copied().collect();
        v.sort_unstable();
        v
    };
    let mut bridge = bridge;
    bridge.sort_unstable();
    manifest["units"] = serde_json::json!({"right_eye_ids":sorted(&right),"left_eye_ids":sorted(&left),"mouth_ids":sorted(&oral),"shell_ids":sorted(&shell),"editable_ids":eval_ids,"eye_bridge_transition_ids":bridge});
    // Only geometrically passing candidates receive a native diagnostic artifact.
    if flipped == 0 && collapsed == 0 {
        std::fs::write(
            format!("{out_dir}/mogilny_COORDINATED_POSITION_ONLY.mcd"),
            packed,
        )
        .unwrap();
    }
    std::fs::write(
        format!("{out_dir}/coordinated.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(
        format!("{out_dir}/fit_positions.json"),
        serde_json::to_vec(&fitted).unwrap(),
    )
    .unwrap();
    let cy = (dr[1] + dl[1]) * 0.5;
    let inter = dist(dr, dl);
    let cam = Camera::for_face(&mesh.positions, cy, inter);
    // Include native eyes and lashes in the render; earlier previews showed head shell
    // alone, which obscured the exact eye placement being reviewed.
    let combined = |container: &McdContainer, head_pos: &[[f32; 3]]| {
        let mut p = head_pos.to_vec();
        let mut t = mesh.triangles.clone();
        for name in ["geo_reyeShape", "geo_leyeShape", "geo_eyelashesShape"] {
            if let Some(i) = container.submesh_index(name) {
                let st = container.position_stream_of(i).unwrap();
                let off = p.len() as u32;
                p.extend_from_slice(container.streams[st].positions());
                t.extend(
                    container
                        .triangles_of(i)
                        .unwrap()
                        .iter()
                        .map(|tri| tri.map(|v| v + off)),
                );
            }
        }
        (p, t)
    };
    let (dp, dt) = combined(c, &mesh.positions);
    let (fp, ft) = combined(&out_c, &fitted);
    let mut sp = ref_mesh.positions.clone();
    let mut st = ref_mesh.triangles.clone();
    let offset = sp.len() as u32;
    let re = reference_surface(aligned_eyes, &source.eyes_triangles);
    sp.extend_from_slice(&re.positions);
    st.extend(re.triangles.iter().map(|t| t.map(|v| v + offset)));
    for (name, yaw, cut) in [
        ("front", 0.0, false),
        ("profile", 90.0, false),
        ("threequarter", 35.0, false),
        ("cutaway", 90.0, true),
    ] {
        let mut strip = image::RgbImage::new(2400, 1000);
        for (column, tag, p, t) in [
            (0, "donor", &dp, &dt),
            (1, "fitted", &fp, &ft),
            (2, "source", &sp, &st),
        ] {
            let img = cam.shade(p, t, yaw, cut);
            img.save(format!("{out_dir}/{name}_{tag}.png")).unwrap();
            image::imageops::replace(&mut strip, &img, column * 800, 0);
        }
        strip
            .save(format!("{out_dir}/{name}_comparison.png"))
            .unwrap();
    }
    println!(
        "COORDINATED artifact and three-column donor / fitted / source comparisons: {out_dir}"
    );
}

#[cfg(test)]
mod coordinated_tests {
    use super::*;
    #[test]
    fn rigid_anchors_translate_together_and_shell_stays_exact() {
        let mesh = DonorMesh::new(
            vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            vec![[0, 1, 2], [0, 2, 3]],
        );
        let d = [0.002, 0.003, -0.004];
        let p = translate_units(&mesh, &[Some([0.; 3]), Some(d), Some(d), None]);
        assert_eq!(p[0], mesh.positions[0]);
        assert_eq!(p[1], unit_add(mesh.positions[1], d));
        assert_eq!(p[2], unit_add(mesh.positions[2], d));
        assert!((dist(p[1], p[2]) - 1.0).abs() < 1e-6);
        assert!(p[3].iter().all(|v| v.is_finite()));
    }

    #[test]
    fn coincident_lip_ids_remain_distinct_and_coincident_under_unit_translation() {
        let mesh = DonorMesh::new(
            vec![[0., 0., 0.], [0., 0., 0.], [1., 1., 0.], [1., -1., 0.]],
            vec![[0, 2, 3], [1, 3, 2]],
        );
        let d = [0.003, -0.002, 0.001];
        let p = translate_units(&mesh, &[Some(d); 4]);
        assert_eq!(p.len(), 4);
        assert_eq!(p[0], p[1]);
        assert_eq!(p[0], d);
    }
}
