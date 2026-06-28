//! Drive the registry + DTO seam without Tauri: load a real anchor, validate, save, and check the
//! DTOs and error paths.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, parse_role, Registry};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: PathBuf) -> String {
    p.to_str().unwrap().to_string()
}

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DEFS: &str = "defs";

#[test]
fn load_validate_save_through_registry() {
    let reg = Registry::default();
    let platform = parse_platform("ps3").unwrap();
    let role = parse_role("nhlng").unwrap();

    let summary = reg
        .load_project(&s(repo(DB)), platform, role, &s(repo(META)), &s(repo(DEFS)))
        .unwrap();
    assert_eq!(summary.project_id, 0);
    assert_eq!(summary.source, "raw_db");
    assert_eq!(summary.platform, "ps3");
    assert_eq!(summary.role, "nhlng");
    assert!(!summary.dirty);
    assert!(summary.table_count > 0);
    assert_eq!(summary.tables.len(), summary.table_count);

    let report = reg.validate(0).unwrap();
    assert!(report.clean, "{} error findings", report.error_count);

    // Save with no edits == the source file, byte for byte.
    let out = std::env::temp_dir().join("studio_api_nhlng_out.db");
    let written = reg.save(0, out.to_str().unwrap()).unwrap();
    let original = std::fs::read(repo(DB)).unwrap();
    assert_eq!(written as usize, original.len());
    assert_eq!(std::fs::read(&out).unwrap(), original);
    let _ = std::fs::remove_file(&out);

    // Unknown ids and bad enums surface as errors, not panics.
    assert!(reg.summary(99).is_err());
    assert!(parse_platform("dreamcast").is_err());
    assert!(parse_role("savegame").is_err());
}
