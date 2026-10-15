//! `headmesh` — CLI surface of the conversion engine.
//!
//! The CLI and the Studio call the same library functions; the CLI additionally exposes
//! spike tooling (`uv-fingerprint`, `census`) and the `--allow-unproven` escape hatch
//! that UI surfaces never get.

/// One submesh as the UV tools consume it: its name, UV1 coordinates and triangle indices.
type UvSubmesh = (String, Vec<[f32; 2]>, Vec<[u32; 3]>);

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tdb_headmesh::census::Census;
use tdb_headmesh::ir::SubmeshRole;
use tdb_headmesh::matrix::{ConversionMatrix, GameKey};
use tdb_headmesh::uvfp::{self, UvFingerprint};

const USAGE: &str = "\
headmesh — game-to-game headmesh conversion engine

USAGE:
  headmesh matrix [--defs <headmesh-matrix.toml>]
      Show the conversion capability matrix (pairs + verification status).

  headmesh uv-fingerprint <mesh files...> [--json <out.json>]
      Fingerprint head UV atlases across container formats (.rx2, .mcd, .glb)
      and report which files share a UV layout. Spike HM-S1 tooling.

  headmesh census <player .rx2 files...>
      Aggregate which palette slots vanilla assets actually weight
      (remap targets must stay inside the used set).

  headmesh convert --source nhl24 --target <game> [--allow-unproven]
      --head-glb <head_*_mesh.glb> --weights <head_*_mesh.weights.json>
      --donor <player_67.rx2> --donor-texlib <texlib_67.rx2>
      --head-id <id> --out <dir>
      [--head-texture <color.dds>] [--hair-texture <color.dds>]
      [--collapse-haircap] [--keep-donor-eyes]
      Donor eyeballs + mouthbag are aligned to the source's measured
      placement by default; --keep-donor-eyes keeps donor positions.
      Path A conversion: extract → remap → pack → texture → QA. Writes
      player_<id>.rx2 + texlib_<id>.rx2 + qa.json + install-plan.json
      into --out (never into a game tree — install apply is separate).

  headmesh convert --source cc5 --target <game> [--allow-unproven]
      --head-glb <character.glb>
      --donor <player_67.rx2> --donor-texlib <texlib_67.rx2>
      --head-id <id> --out <dir>
      [--head-texture <Std_Skin_Head_Diffuse.png>] [--hair-texture <img>]
      [--collapse-haircap] [--keep-donor-eyes]
      [--allow-converted-donor] [--eyelid-reach-mm <mm>]
      [--source-offset x,y,z] [--allow-frame-offset]
      Path C conversion (Reallusion Character Creator 5): extract →
      donor weight transfer → pack → texture → report. No --weights: the
      CC5 GLB names its own roles and carries weights inline. CC5 rigs are
      rigid, so skin weights are taken off the donor's rigged surface by
      closest point rather than retargeted. Textures may be PNG/JPEG/DDS.
      The source's eyes/teeth/tongue are measured (to place and size the
      donor's eyeballs and mouthbag) and then dropped — never packed.
      --donor must be a VANILLA mesh: converting against a donor that is
      already a previous output is refused unless --allow-converted-donor.
      A source shipping a real CC5 skeleton uses its AUTHORED weights
      (jaw/neck/spine, mapped via cc5-bone-map.toml) with eyelids taken
      from the Eye_Blink_* morph deltas; the donor only fills fine facial
      detail outside the eye and mouth buckets. A stub-rig export falls
      back to pure donor transfer. The source must already sit in Legacy
      head space — conversion is refused otherwise, reporting the exact
      --source-offset needed.
      Weights are anatomically gated: upper and lower lip never source
      each other (a closed-mouth bind pose otherwise welds them shut) and
      eyelid bones stay inside the donor's own lid band (or the brow
      blinks). --eyelid-reach-mm overrides that band's measured width.
      Same outputs as Path A, with a transfer-fit report in qa.json.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, rest) = match args.split_first() {
        Some((cmd, rest)) => (cmd.as_str(), rest),
        None => {
            eprint!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let result = match cmd {
        "matrix" => cmd_matrix(rest),
        "uv-fingerprint" => cmd_uv_fingerprint(rest),
        "census" => cmd_census(rest),
        "convert" => cmd_convert(rest),
        "--help" | "-h" | "help" => {
            print!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command {other:?}\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}

/// Default matrix defs path: walk up from cwd looking for defs/appearance/.
fn default_matrix_path() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let cand = dir.join("defs/appearance/headmesh-matrix.toml");
        if cand.is_file() {
            return Some(cand);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn positional(args: &[String]) -> Vec<&String> {
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
            continue;
        }
        if a.starts_with("--") {
            // all our flags take a value except --allow-unproven
            skip = a != "--allow-unproven";
            continue;
        }
        out.push(a);
    }
    out
}

fn cmd_matrix(args: &[String]) -> Result<(), String> {
    let path = match flag_value(args, "--defs") {
        Some(p) => PathBuf::from(p),
        None => default_matrix_path()
            .ok_or("no defs/appearance/headmesh-matrix.toml found; pass --defs")?,
    };
    let matrix = ConversionMatrix::load(&path).map_err(|e| e.to_string())?;
    println!("matrix: {} ({} pairs)", path.display(), matrix.pairs.len());
    for p in &matrix.pairs {
        println!(
            "  {:>16} → {:<20} {:?} via {:?}  [{:?}]{}",
            p.source.0,
            p.target.0,
            p.path,
            p.frame_adapter,
            p.status,
            if p.evidence.is_empty() {
                String::new()
            } else {
                format!("  ({})", p.evidence)
            }
        );
    }
    Ok(())
}

/// Fingerprint every head-role submesh of every input file, then group by hash.
fn cmd_uv_fingerprint(args: &[String]) -> Result<(), String> {
    let files = positional(args);
    if files.is_empty() {
        return Err("no input files (expected .rx2 / .mcd / .glb)".into());
    }
    let mut prints: Vec<UvFingerprint> = Vec::new();
    let mut masks: Vec<Vec<u64>> = Vec::new();
    for file in files {
        let path = Path::new(file);
        let data = std::fs::read(path).map_err(|e| format!("{file}: {e}"))?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let submeshes: Vec<UvSubmesh> = match ext.as_str() {
            "rx2" => {
                let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data)
                    .map_err(|e| format!("{file}: {e}"))?;
                asset
                    .submeshes
                    .iter()
                    .map(|s| {
                        (
                            s.name.clone(),
                            s.vertices.iter().map(|v| v.uv).collect(),
                            s.triangles.clone(),
                        )
                    })
                    .collect()
            }
            "mcd" => {
                let asset =
                    tdb_rx2::mcd::parse_player_mesh(&data).map_err(|e| format!("{file}: {e}"))?;
                asset
                    .submeshes
                    .iter()
                    .map(|s| {
                        (
                            s.name.clone(),
                            s.vertices.iter().map(|v| v.uv).collect(),
                            s.triangles.clone(),
                        )
                    })
                    .collect()
            }
            "glb" => {
                let mesh =
                    tdb_headmesh::glb::parse_glb(&data).map_err(|e| format!("{file}: {e}"))?;
                mesh.primitives
                    .into_iter()
                    .map(|p| (p.name, p.uvs, p.triangles))
                    .collect()
            }
            other => return Err(format!("{file}: unsupported extension {other:?}")),
        };
        // One fingerprint per file: the largest head-role submesh. (NHL24 GLB prims all
        // fall back to the mesh name, so eyes/lashes would otherwise classify as Head.)
        if let Some((name, uvs, triangles)) = submeshes
            .into_iter()
            .filter(|(name, uvs, _)| {
                SubmeshRole::classify(name) == SubmeshRole::Head && !uvs.is_empty()
            })
            .max_by_key(|(_, uvs, _)| uvs.len())
        {
            prints.push(uvfp::fingerprint(file, &name, &uvs, triangles.len()));
            masks.push(uvfp::coverage_mask(&uvs, &triangles));
        }
    }
    if prints.is_empty() {
        return Err("no head-role submeshes with UVs found in the inputs".into());
    }

    for p in &prints {
        println!(
            "{}  [{}]  verts={} tris={} uniq_uv={}  uv=[{:.3},{:.3}]..[{:.3},{:.3}]  hash={}",
            p.source,
            p.submesh,
            p.vertex_count,
            p.triangle_count,
            p.unique_uv_count,
            p.uv_min[0],
            p.uv_min[1],
            p.uv_max[0],
            p.uv_max[1],
            p.uv_multiset_hash
        );
    }
    // Group by identical multiset hash — the candidate "same atlas" classes.
    let mut groups: Vec<(&str, Vec<&UvFingerprint>)> = Vec::new();
    for p in &prints {
        match groups.iter_mut().find(|(h, _)| *h == p.uv_multiset_hash) {
            Some((_, members)) => members.push(p),
            None => groups.push((&p.uv_multiset_hash, vec![p])),
        }
    }
    println!("\n{} distinct UV multiset(s):", groups.len());
    for (hash, members) in &groups {
        println!("  {hash}:");
        for m in members {
            println!("    {} [{}]", m.source, m.submesh);
        }
    }
    // Cross-topology atlas comparison. `agree±2` is the decisive number: same layout
    // (with storage jitter) ⇒ ≈ 1.0; moved/re-cut islands fall out. Raw IoU shown for
    // context.
    if prints.len() > 1 {
        println!("\natlas-layout comparison (agree = min directed coverage within 2 texels):");
        for i in 0..prints.len() {
            for j in (i + 1)..prints.len() {
                let agree = uvfp::coverage_agreement(&masks[i], &masks[j], 2);
                let iou = uvfp::coverage_iou(&masks[i], &masks[j]);
                println!(
                    "  agree={agree:.4} iou={iou:.4}  {}  vs  {}",
                    prints[i].source, prints[j].source
                );
            }
        }
    }

    if let Some(out) = flag_value(args, "--json") {
        let json = serde_json::to_string_pretty(&prints).map_err(|e| e.to_string())?;
        std::fs::write(out, json).map_err(|e| format!("{out}: {e}"))?;
        println!("\nfingerprints written to {out}");
    }
    Ok(())
}

fn cmd_census(args: &[String]) -> Result<(), String> {
    let files = positional(args);
    if files.is_empty() {
        return Err("no input files (expected vanilla player .rx2 files)".into());
    }
    let mut census = Census::default();
    for file in files {
        let data = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
        let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data)
            .map_err(|e| format!("{file}: {e}"))?;
        census.add_asset(&asset);
    }
    println!(
        "census over {} asset(s): {} palette slot(s) carry weight",
        census.assets,
        census.mass.len()
    );
    for (slot, mass) in &census.mass {
        println!("  palette {slot:>3}: mass {mass:>10.2}");
    }
    if !census.skipped_rigid.is_empty() {
        let mut names = census.skipped_rigid.clone();
        names.sort();
        names.dedup();
        println!("  (rigid submeshes skipped: {})", names.join(", "));
    }
    Ok(())
}

fn cmd_convert(args: &[String]) -> Result<(), String> {
    let source = flag_value(args, "--source").ok_or("missing --source <game>")?;
    let target = flag_value(args, "--target").ok_or("missing --target <game>")?;
    let allow_unproven = args.iter().any(|a| a == "--allow-unproven");
    let matrix_path = match flag_value(args, "--defs") {
        Some(p) => PathBuf::from(p),
        None => default_matrix_path()
            .ok_or("no defs/appearance/headmesh-matrix.toml found; pass --defs")?,
    };
    let defs_dir = matrix_path
        .parent()
        .ok_or("cannot resolve defs directory")?
        .to_path_buf();
    let matrix = ConversionMatrix::load(&matrix_path).map_err(|e| e.to_string())?;
    let pair = matrix
        .require_pair(
            &GameKey(source.to_owned()),
            &GameKey(target.to_owned()),
            allow_unproven,
        )
        .map_err(|e| e.to_string())?;
    println!(
        "pair {} → {} validated ({:?}, {:?})",
        pair.source, pair.target, pair.path, pair.status
    );
    match source {
        "nhl24" => {}
        "cc5" => return cmd_convert_cc5(args, pair, &defs_dir),
        _ => return Err(
            "implemented sources: nhl24 (Path A, authored weights), cc5 (Path C, donor weights)"
                .into(),
        ),
    }

    let need = |flag: &str| flag_value(args, flag).ok_or(format!("missing {flag}"));
    let head_glb = std::fs::read(need("--head-glb")?).map_err(|e| e.to_string())?;
    let sidecar = tdb_headmesh::sidecar::Sidecar::load(Path::new(need("--weights")?))
        .map_err(|e| e.to_string())?;
    let donor = std::fs::read(need("--donor")?).map_err(|e| e.to_string())?;
    let donor_texlib = std::fs::read(need("--donor-texlib")?).map_err(|e| e.to_string())?;
    let head_id: u32 = need("--head-id")?
        .parse()
        .map_err(|_| "--head-id must be a number")?;
    let out_dir = PathBuf::from(need("--out")?);
    let head_tex = flag_value(args, "--head-texture")
        .map(std::fs::read)
        .transpose()
        .map_err(|e| e.to_string())?;
    let hair_tex = flag_value(args, "--hair-texture")
        .map(std::fs::read)
        .transpose()
        .map_err(|e| e.to_string())?;

    // Validate the head id against the target's id scheme BEFORE doing any work.
    let plan = tdb_headmesh::install::plan_rw_install(pair.id_scheme, head_id)
        .map_err(|e| e.to_string())?;

    let output = tdb_headmesh::convert::convert_nhl24_to_rw(tdb_headmesh::convert::PathAInputs {
        glb: &head_glb,
        sidecar: &sidecar,
        source_paths: vec![
            flag_value(args, "--head-glb")
                .unwrap_or_default()
                .to_owned(),
            flag_value(args, "--weights").unwrap_or_default().to_owned(),
        ],
        donor_rx2: &donor,
        donor_texlib: &donor_texlib,
        head_texture_dds: head_tex.as_deref(),
        hair_texture_dds: hair_tex.as_deref(),
        collapse_haircap: args.iter().any(|a| a == "--collapse-haircap"),
        align_submeshes: !args.iter().any(|a| a == "--keep-donor-eyes"),
        defs_dir: &defs_dir,
    })
    .map_err(|e| e.to_string())?;

    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let mesh_path = out_dir.join(format!("player_{head_id}.rx2"));
    let texlib_path = out_dir.join(format!("texlib_{head_id}.rx2"));
    std::fs::write(&mesh_path, &output.player_rx2).map_err(|e| e.to_string())?;
    std::fs::write(&texlib_path, &output.texlib_rx2).map_err(|e| e.to_string())?;
    let qa_json = serde_json::json!({
        "scorecard": output.scorecard,
        "remap": {
            "collapsed_mass": output.remap_stats.collapsed_mass,
            "unmapped_mass": output.remap_stats.unmapped_mass,
            "truncated_mass": output.remap_stats.truncated_mass,
            "truncated_vertices": output.remap_stats.truncated_vertices,
            "unweighted_vertices": output.remap_stats.unweighted_vertices,
        },
        "pack": {
            "vertices": output.pack_report.vertices,
            "triangles": output.pack_report.triangles,
            "haircap_collapsed": output.pack_report.haircap_collapsed,
        },
        "alignment": {
            "eye_shifts_mm": output.pack_report.eye_shifts_mm,
            "mouthbag_shift_mm": output.pack_report.mouthbag_shift_mm,
        },
    });
    std::fs::write(
        out_dir.join("qa.json"),
        serde_json::to_string_pretty(&qa_json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        out_dir.join("install-plan.json"),
        serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let sc = &output.scorecard;
    println!(
        "converted: {} verts / {} tris → {}",
        output.pack_report.vertices,
        output.pack_report.triangles,
        mesh_path.display()
    );
    println!(
        "texlib: {} ({} bytes)",
        texlib_path.display(),
        output.texlib_rx2.len()
    );
    println!(
        "QA [{:?}]: facial RMS {:.2} mm / max {:.2} mm | retarget RMS {:.2} / max {:.2} | DOF-loss RMS {:.2} | {} poses",
        sc.verdict, sc.facial_rms_mm, sc.facial_max_mm, sc.retarget_rms_mm, sc.retarget_max_mm,
        sc.dof_loss_rms_mm, sc.battery_poses
    );
    for (name, mm) in &output.pack_report.eye_shifts_mm {
        println!("aligned {name}: moved {mm:.2} mm to the source eye center");
    }
    if let Some(mm) = output.pack_report.mouthbag_shift_mm {
        println!("aligned mouthbag: moved {mm:.2} mm to the source mouth opening");
    }
    println!(
        "remap: collapsed {:.1} / unmapped {:.1} / truncated {:.1} mass, {} truncated verts",
        output.remap_stats.collapsed_mass,
        output.remap_stats.unmapped_mass,
        output.remap_stats.truncated_mass,
        output.remap_stats.truncated_vertices
    );
    println!("install plan written (files stay in --out; apply is a separate step)");
    Ok(())
}

/// Path C — Reallusion CC5 head GLB → RW donor. No `--weights` (the GLB is
/// self-describing) and no retarget scorecard (there is no source rig to score); the
/// report is the donor weight-transfer fit instead.
fn cmd_convert_cc5(
    args: &[String],
    pair: &tdb_headmesh::matrix::Pair,
    defs_dir: &Path,
) -> Result<(), String> {
    let need = |flag: &str| flag_value(args, flag).ok_or(format!("missing {flag}"));
    let head_glb_path = need("--head-glb")?.to_owned();
    let head_glb = std::fs::read(&head_glb_path).map_err(|e| e.to_string())?;
    let donor = std::fs::read(need("--donor")?).map_err(|e| e.to_string())?;
    let donor_texlib = std::fs::read(need("--donor-texlib")?).map_err(|e| e.to_string())?;
    let head_id: u32 = need("--head-id")?
        .parse()
        .map_err(|_| "--head-id must be a number")?;
    let out_dir = PathBuf::from(need("--out")?);
    let head_tex = flag_value(args, "--head-texture")
        .map(std::fs::read)
        .transpose()
        .map_err(|e| e.to_string())?;
    let hair_tex = flag_value(args, "--hair-texture")
        .map(std::fs::read)
        .transpose()
        .map_err(|e| e.to_string())?;

    // Validate the head id against the target's id scheme BEFORE doing any work.
    let plan = tdb_headmesh::install::plan_rw_install(pair.id_scheme, head_id)
        .map_err(|e| e.to_string())?;

    let output = tdb_headmesh::convert::convert_cc5_to_rw(tdb_headmesh::convert::PathCInputs {
        glb: &head_glb,
        source_paths: vec![head_glb_path],
        donor_rx2: &donor,
        donor_texlib: &donor_texlib,
        head_texture: head_tex.as_deref(),
        hair_texture: hair_tex.as_deref(),
        collapse_haircap: args.iter().any(|a| a == "--collapse-haircap"),
        align_submeshes: !args.iter().any(|a| a == "--keep-donor-eyes"),
        source_offset: match flag_value(args, "--source-offset") {
            Some(v) => {
                let parts: Vec<&str> = v.split(',').collect();
                if parts.len() != 3 {
                    return Err("--source-offset wants x,y,z in metres".into());
                }
                let mut o = [0f32; 3];
                for (i, p) in parts.iter().enumerate() {
                    o[i] = p
                        .trim()
                        .parse()
                        .map_err(|_| "--source-offset components must be numbers")?;
                }
                Some(o)
            }
            None => None,
        },
        allow_frame_offset: args.iter().any(|a| a == "--allow-frame-offset"),
        eyelid_reach_mm: match flag_value(args, "--eyelid-reach-mm") {
            Some(v) => Some(
                v.parse()
                    .map_err(|_| "--eyelid-reach-mm must be a number")?,
            ),
            None => None,
        },
        defs_dir,
    })
    .map_err(|e| e.to_string())?;

    // Gates first — nothing reaches disk from a conversion we are about to call invalid.
    if !output.invented_slots.is_empty() {
        return Err(format!(
            "transfer produced palette slots the donor never weights: {:?}",
            output.invented_slots
        ));
    }
    if output.donor_is_already_this_head && !args.iter().any(|a| a == "--allow-converted-donor") {
        return Err(format!(
            "--donor already contains this exact head ({} verts, worst fit {:.2} mm): it is a \
             previous output of this pipeline, not a vanilla donor. Installing a converted head \
             over the game's file and then reconverting measures every placement and scale \
             against the converted asset, so the numbers look perfect and mean nothing. Point \
             --donor at the vanilla mesh (an install leaves it beside the original as \
             `<file>.bak`), or pass --allow-converted-donor if this is deliberate.",
            output.pack_report.vertices, output.transfer_report.max_distance_mm
        ));
    }

    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let mesh_path = out_dir.join(format!("player_{head_id}.rx2"));
    let texlib_path = out_dir.join(format!("texlib_{head_id}.rx2"));
    std::fs::write(&mesh_path, &output.player_rx2).map_err(|e| e.to_string())?;
    std::fs::write(&texlib_path, &output.texlib_rx2).map_err(|e| e.to_string())?;

    let b = &output.bind_report;
    let t = &output.transfer_report;
    let qa_json = serde_json::json!({
        "path": "donor-weights",
        "source": "cc5",
        "bind": {
            "joint_names": b.joint_names,
            "max_identity_deviation": b.max_identity_deviation,
            "positions_were_model_space": b.positions_were_model_space,
            "rig_is_rigid": b.rig_is_rigid,
        },
        "transfer": {
            "vertices": t.vertices,
            "mean_distance_mm": t.mean_distance_mm,
            "p95_distance_mm": t.p95_distance_mm,
            "max_distance_mm": t.max_distance_mm,
            "far_vertices": t.far_vertices,
            "far_vertex_gate_mm": tdb_headmesh::transfer::FAR_VERTEX_MM,
            "palette_slots_used": t.palette_slots_used,
            "top_slots_pct": t.top_slots,
            "invented_slots": output.invented_slots,
        },
        "pack": {
            "vertices": output.pack_report.vertices,
            "triangles": output.pack_report.triangles,
            "haircap_collapsed": output.pack_report.haircap_collapsed,
        },
        "alignment": {
            "eye_shifts_mm": output.pack_report.eye_shifts_mm,
            "mouthbag_shift_mm": output.pack_report.mouthbag_shift_mm,
            "eye_scale": output.pack_report.eye_scale,
            "mouthbag_scale": output.pack_report.mouthbag_scale,
            "haircap_conform": output.pack_report.haircap_conform.as_ref().map(|h| {
                serde_json::json!({
                    "vertices": h.vertices,
                    "mean_mm": h.mean_mm,
                    "p95_mm": h.p95_mm,
                    "max_mm": h.max_mm,
                    "clamped": h.clamped,
                    "clamp_limit_mm": tdb_headmesh::pack_rw::SHELL_CONFORM_LIMIT_MM,
                    // Vanilla shells never penetrate the skull; `inside_fraction` after
                    // the conform is the number to read.
                    "clearance_before": { "median_mm": h.clearance_before.0, "inside_fraction": h.clearance_before.1 },
                    "clearance_after": { "median_mm": h.clearance_after.0, "inside_fraction": h.clearance_after.1 },
                    "shape_change_mean_deg": h.normal_drift_mean_deg,
                    "shape_change_p95_deg": h.normal_drift_p95_deg,
                })
            }),
        },
        "companions_measured_then_dropped": output.companions_dropped,
        "gates": output.gates,
        "authored": output.authored,
    });
    std::fs::write(
        out_dir.join("qa.json"),
        serde_json::to_string_pretty(&qa_json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        out_dir.join("install-plan.json"),
        serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    println!(
        "source rig: {} joint(s) [{}] — bind {} model space (max identity deviation {:.2e}), rig is {}",
        b.joint_names.len(),
        b.joint_names.join(", "),
        if b.positions_were_model_space { "already in" } else { "BAKED into" },
        b.max_identity_deviation,
        if b.rig_is_rigid { "rigid → weights transferred from the donor" } else { "articulated" }
    );
    println!(
        "converted: {} verts / {} tris → {}",
        output.pack_report.vertices,
        output.pack_report.triangles,
        mesh_path.display()
    );
    println!(
        "texlib: {} ({} bytes)",
        texlib_path.display(),
        output.texlib_rx2.len()
    );
    println!(
        "weight transfer: mean {:.2} mm / p95 {:.2} mm / max {:.2} mm from the donor surface; \
         {} vert(s) beyond the {:.0} mm gate; {} palette slot(s) used",
        t.mean_distance_mm,
        t.p95_distance_mm,
        t.max_distance_mm,
        t.far_vertices,
        tdb_headmesh::transfer::FAR_VERTEX_MM,
        t.palette_slots_used.len()
    );
    let slots: Vec<String> = t
        .top_slots
        .iter()
        .map(|(s, pct)| format!("{s}={pct:.1}%"))
        .collect();
    println!("  weight mass: {}", slots.join(" "));
    if let Some(a) = &output.authored {
        println!(
            "AUTHORED WEIGHTS: {} source joints mapped through cc5-bone-map.toml;              {:.1} mass protected (jaw/neck/spine, untouched by the blend)",
            a.source_bones, a.blend.protected_mass
        );
        println!(
            "  eyelids from morphs: {}",
            if a.morphs_found.is_empty() {
                "NONE FOUND — lids will not blink".to_string()
            } else {
                a.morphs_found
                    .iter()
                    .map(|(n, c)| format!("{n} ({c} verts)"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
        println!(
            "  donor fine detail filled {} vert(s) with {:.1} mass (eye + mouth buckets barred)",
            a.blend.donor_filled_vertices, a.blend.donor_filled_mass
        );
    } else {
        println!("stub rig — falling back to donor weight transfer + geometric gates");
    }
    let g = &output.gates;
    println!(
        "anatomical gates: lip crease {} ({} upper / {} lower verts in the mouth zone), \
         {} gate rescue(s)",
        if g.lip_crease_found {
            "measured"
        } else {
            "NOT FOUND — lip rules inert"
        },
        g.upper_lip_vertices,
        g.lower_lip_vertices,
        g.gate_rescues
    );
    println!(
        "  cross-lip/tongue mask: {} vert(s), {:.2} weight mass removed",
        g.lip_vertices_masked, g.lip_mass_masked
    );
    println!(
        "  eyelid containment (donor band reach {:.1} mm): {} vert(s) stripped, {:.2} mass; \
         {} vert(s) keep eyelid weight over {}",
        g.eyelid_band_reach_mm,
        g.eyelid_vertices_masked,
        g.eyelid_mass_masked,
        g.eyelid_vertices_kept,
        g.eyelid_kept_extent_mm
            .map(|e| format!("{:.0}×{:.0}×{:.0} mm", e[0], e[1], e[2]))
            .unwrap_or_else(|| "—".into())
    );
    for (name, mm) in &output.pack_report.eye_shifts_mm {
        println!(
            "aligned {name}: moved {mm:.2} mm to the source eye center, scaled ×{:.4}",
            output.pack_report.eye_scale
        );
    }
    if let Some(mm) = output.pack_report.mouthbag_shift_mm {
        println!(
            "aligned mouthbag: moved {mm:.2} mm to the source mouth opening, scaled ×{:.4}",
            output.pack_report.mouthbag_scale
        );
    }
    if let Some(h) = &output.pack_report.haircap_conform {
        println!(
            "conformed haircap: {} verts moved mean {:.2} mm / p95 {:.2} / max {:.2} ({} clamped at {:.0} mm)",
            h.vertices,
            h.mean_mm,
            h.p95_mm,
            h.max_mm,
            h.clamped,
            tdb_headmesh::pack_rw::SHELL_CONFORM_LIMIT_MM
        );
        println!(
            "  clearance over the skull: median {:.2} mm / {:.1}% buried  →  median {:.2} mm / {:.1}% buried \
             (vanilla player_67 buries 1.5%)",
            h.clearance_before.0,
            100.0 * h.clearance_before.1,
            h.clearance_after.0,
            100.0 * h.clearance_after.1
        );
        println!(
            "  shape change: mean {:.2}° / p95 {:.2}° (NORMAL + TANGENT rewritten to match)",
            h.normal_drift_mean_deg, h.normal_drift_p95_deg
        );
    }
    println!(
        "companions measured then dropped (never packed): {}",
        output
            .companions_dropped
            .iter()
            .map(|c| format!(
                "{} [{}] {}v{}",
                c.name,
                c.role,
                c.vertices,
                c.extents_mm
                    .map(|e| format!(" {:.1}×{:.1}×{:.1} mm", e[0], e[1], e[2]))
                    .unwrap_or_default()
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("install plan written (files stay in --out; apply is a separate step)");
    Ok(())
}
