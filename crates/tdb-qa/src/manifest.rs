//! The canonical anchor manifest: every file under `anchors/` with a stable role, size, and sha256.
//! Deterministic — files are emitted sorted by repo-relative path, so repeated runs are byte-identical
//! and any anchor addition/removal or content change shows up as a manifest diff (the drift guard).

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// One manifest row.
pub struct ManifestRow {
    pub relpath: String,
    pub platform: String,
    pub game: String,
    pub role: String,
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
}

/// Classify a file under `anchors/<platform>/<game>/...` into a stable role. `sub` is the first path
/// component beneath the game dir (`databases` / `savefiles`); `filename` is the leaf.
fn role_for(sub: &str, filename: &str) -> &'static str {
    let lower = filename.to_ascii_lowercase();
    match sub {
        "databases" => {
            if lower.ends_with(".xml") {
                if lower.contains("eng_us") {
                    "localization-metadata"
                } else {
                    "schema-metadata"
                }
            } else if lower.ends_with(".db") {
                if lower == "nhlng.db" || lower == "nhl07ng.db" || lower == "nhl_game.db" {
                    "main-database"
                } else if lower == "nhl_eng_us.db" || lower == "eng_us.db" {
                    "localization-database"
                } else {
                    "aux-database"
                }
            } else {
                "database-other"
            }
        }
        "savefiles" => {
            if lower.ends_with(".sfo") {
                "save-param"
            } else if lower.ends_with(".png") {
                "save-icon"
            } else {
                "save-data"
            }
        }
        _ => "other",
    }
}

/// Recursively collect every file under `root`, returning repo-relative slash paths sorted ascending.
fn walk_sorted(root: &Path, base: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect(root, base, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        if path.is_dir() {
            collect(&path, base, out)?;
        } else {
            out.push(path.strip_prefix(base).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Build every manifest row from the on-disk `anchors/` tree at `anchors_root` (the repo `anchors`
/// directory). `repo_root` is its parent, used to form repo-relative paths.
pub fn build_rows(repo_root: &Path) -> std::io::Result<Vec<ManifestRow>> {
    let anchors_root = repo_root.join("anchors");
    let files = walk_sorted(&anchors_root, repo_root)?;
    let mut rows = Vec::with_capacity(files.len());
    for rel in files {
        let abs = repo_root.join(&rel);
        let data = std::fs::read(&abs)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let sha = hex(&hasher.finalize());
        let comps: Vec<String> =
            rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
        // comps = ["anchors", platform, game, sub, ..., filename]
        let platform = comps.get(1).cloned().unwrap_or_default();
        let game = comps.get(2).cloned().unwrap_or_default();
        let sub = comps.get(3).cloned().unwrap_or_default();
        let filename = comps.last().cloned().unwrap_or_default();
        rows.push(ManifestRow {
            relpath: slash(&rel),
            platform,
            game,
            role: role_for(&sub, &filename).to_string(),
            filename,
            bytes: data.len() as u64,
            sha256: sha,
        });
    }
    Ok(rows)
}

fn csv_cell(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Render the manifest rows to a CSV string (trailing newline, `\n` line endings).
pub fn to_csv(rows: &[ManifestRow]) -> String {
    let mut s = String::from("relpath,platform,game,role,filename,bytes,sha256\n");
    for r in rows {
        s.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_cell(&r.relpath),
            csv_cell(&r.platform),
            csv_cell(&r.game),
            csv_cell(&r.role),
            csv_cell(&r.filename),
            r.bytes,
            r.sha256,
        ));
    }
    s
}
