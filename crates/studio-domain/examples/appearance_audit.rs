//! Read-only appearance inventory and optional export smoke test.
//! usage: appearance_audit <platform> <game> <install> <report-dir> [database metadata] [--export]
use std::path::Path;
use studio_domain::{assets::AssetRoots, Platform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        return Err("usage: appearance_audit <xbox|ps4> <game> <install> <report-dir> [database metadata] [--export]".into());
    }
    let platform = match args[1].as_str() {
        "xbox" => Platform::Xbox,
        "ps4" => Platform::Ps4,
        _ => return Err("platform must be xbox or ps4".into()),
    };
    let output = Path::new(&args[4]);
    std::fs::create_dir_all(output)?;
    if args.iter().any(|a| a == "--inflate-fixtures-only") {
        // Mirror onboarding's EB decompression only in this audit's staged copy.
        // Refuse installed games and every path outside our declared fixture root.
        let source = std::fs::canonicalize(&args[3])?;
        let allowed = std::fs::canonicalize(output.join("xbox-fixtures"))?;
        if !source.starts_with(&allowed) {
            return Err("not an owned Xbox fixture directory".into());
        }
        let mut dirs = vec![source];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    dirs.push(entry.path());
                    continue;
                }
                if !entry.file_type()?.is_file() {
                    continue;
                }
                let bytes = std::fs::read(entry.path())?;
                if bytes.starts_with(b"chunkzip") || bytes.starts_with(b"chunklzx") {
                    let decoded = tdb_archive_codec::decompress_if_wrapped(&bytes)?;
                    std::fs::write(entry.path(), decoded)?;
                }
            }
        }
        return Ok(());
    }
    let roots = AssetRoots::resolve(Path::new(&args[3]), platform);
    eprintln!("Resolved roots: {roots:?}, complete={}", roots.complete());
    let report =
        studio_domain::head_census::census_tree(Path::new(&roots.rendering), platform, &args[2], 8);
    std::fs::write(
        output.join(format!("{}-{}.csv", args[1], args[2])),
        studio_domain::head_census::report_to_csv(&report),
    )?;
    eprintln!("Counts: {:?}", report.counts);
    eprintln!("Submeshes: {:?}", report.submesh_vocab);
    let players = if args.len() >= 7 && args[5] != "--export" {
        let bytes = std::fs::read(&args[5])?;
        let db = tdb_core::TdbFile::parse(&bytes)?;
        let meta = tdb_defs::FieldMetaIndex::from_xml(&std::fs::read_to_string(&args[6])?)?;
        let players = studio_domain::scan_player_assets(&db, &meta, &roots)?;
        eprintln!(
            "Players={}, named={}, meshes={}, portraits={}",
            players.len(),
            players.iter().filter(|p| p.slug.is_some()).count(),
            players.iter().filter(|p| p.mesh_path.is_some()).count(),
            players.iter().filter(|p| p.portrait_path.is_some()).count()
        );
        players
    } else {
        Vec::new()
    };
    if args.iter().any(|a| a == "--export") {
        let summary = studio_domain::export_full_catalog(
            &players,
            &roots,
            &output.join(format!("{}-{}-catalog", args[1], args[2])),
            &studio_domain::CatalogOptions {
                game_tag: args[2].clone(),
                textures: true,
            },
        )?;
        eprintln!(
            "Export: heads={}, hair={}, textures={}, failures={:?}",
            summary.heads_written, summary.hair_written, summary.textures_written, summary.failures
        );
        std::fs::write(
            output.join(format!("{}-{}-summary.json", args[1], args[2])),
            serde_json::to_vec_pretty(&summary)?,
        )?;
    }
    Ok(())
}
