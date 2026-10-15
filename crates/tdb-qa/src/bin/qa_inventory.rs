//! Phase-0 QA inventory generator. Regenerates the four committed artifacts under
//! `docs/audits/data/` from the immutable `anchors/` tree. Deterministic: re-running produces no diff.
//!
//! Run from the repo:  `cargo run -p tdb-qa --bin qa-inventory`
//! The `anchors/` tree is only ever read.

use tdb_qa::{diff, discover_main_anchors, manifest, matrix, repo_root_from_manifest, schema};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = repo_root_from_manifest();
    let data = repo_root.join("docs/audits/data");
    std::fs::create_dir_all(&data)?;

    // 1) Anchor manifest (all files, roles, sha256).
    let rows = manifest::build_rows(&repo_root)?;
    let manifest_csv = manifest::to_csv(&rows);
    std::fs::write(data.join("anchors-manifest.csv"), &manifest_csv)?;
    println!("anchors-manifest.csv: {} files", rows.len());

    // 2) Per-anchor schema snapshots. Rebuild the schema dir from scratch so the committed set is
    //    exactly the generated set (removes the superseded partial/inconsistent snapshots).
    let anchors = discover_main_anchors(&repo_root)?;
    let schema_dir = data.join("schema");
    if schema_dir.exists() {
        std::fs::remove_dir_all(&schema_dir)?;
    }
    std::fs::create_dir_all(&schema_dir)?;
    for a in &anchors {
        let snap = schema::snapshot(&repo_root, a)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        std::fs::write(schema_dir.join(schema::filename(a)), schema::to_json(&snap))?;
    }
    println!("schema/: {} main-DB snapshots", anchors.len());

    // 3) Field-level cross-title / cross-platform schema diff.
    let diff_csv = diff::build_csv(&repo_root, &anchors)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    std::fs::write(data.join("schema-diff.csv"), &diff_csv)?;
    println!("schema-diff.csv: {} rows", line_count(&diff_csv));

    // 4) QA ledger matrix.
    let matrix_csv = matrix::build_csv(&repo_root);
    std::fs::write(data.join("game-qa-matrix.csv"), &matrix_csv)?;
    println!("game-qa-matrix.csv: {} rows", line_count(&matrix_csv));

    Ok(())
}

/// Count data rows (lines minus the header, minus the trailing newline).
fn line_count(csv: &str) -> usize {
    csv.lines().count().saturating_sub(1)
}
