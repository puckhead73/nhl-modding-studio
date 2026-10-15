//! Disc-level reference files, independent of archive extraction progress.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Component, Path},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: String,
    pub game_id: String,
    pub platform: String,
    pub source: String,
    pub region: String,
    pub image_sha256: String,
    pub files: Vec<Entry>,
    pub anchors: Vec<Anchor>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    /// `container` or `loose`: describes the original disc, not extraction progress.
    pub kind: String,
    pub optional: bool,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Anchor {
    pub path: String,
    pub sha256: String,
    pub disc_matches: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Finding {
    pub path: String,
    pub issue: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    /// `vanilla`, `different`, `additional`, or `unverified`.
    pub status: String,
    pub reference: String,
    pub region: String,
    pub matched: usize,
    pub loose: usize,
    pub containers: usize,
    pub findings: Vec<Finding>,
}
pub fn hash(mut reader: impl Read) -> std::io::Result<String> {
    let mut h = Sha256::new();
    let mut buf = vec![0; 1024 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
pub fn unverified(reason: impl Into<String>) -> Report {
    Report {
        status: "unverified".into(),
        reference: reason.into(),
        region: "Unknown".into(),
        matched: 0,
        loose: 0,
        containers: 0,
        findings: vec![],
    }
}
fn safe(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
fn link(md: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if md.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    md.file_type().is_symlink()
}
/// Read-only traversal. Never follow symlinks/junctions into other installations.
fn walk(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, std::path::PathBuf>,
    findings: &mut Vec<Finding>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            findings.push(Finding {
                path: dir.strip_prefix(root).unwrap_or(dir).display().to_string(),
                issue: format!("Cannot read folder: {e}"),
            });
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                findings.push(Finding {
                    path: dir.display().to_string(),
                    issue: e.to_string(),
                });
                continue;
            }
        };
        let path = entry.path();
        let name = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        // Dashboard updates are deliberately omitted by the disc extractor.
        if name.contains("$SystemUpdate") {
            continue;
        }
        match std::fs::symlink_metadata(&path) {
            Ok(md) if link(&md) => findings.push(Finding {
                path: name,
                issue: "Link/reparse point not followed; contents unverified".into(),
            }),
            Ok(md) if md.is_dir() => walk(root, &path, out, findings),
            Ok(_) => {
                if out.insert(name.to_lowercase(), path).is_some() {
                    findings.push(Finding {
                        path: name,
                        issue: "Duplicate case-insensitive path".into(),
                    });
                }
            }
            Err(e) => findings.push(Finding {
                path: name,
                issue: format!("Cannot read file: {e}"),
            }),
        }
    }
}
pub fn check(root: &Path, manifest: &Manifest, game: &str, platform: &str) -> Report {
    let mut names = BTreeSet::new();
    if manifest.schema != "nhlms-game-files/1"
        || manifest.game_id != game
        || manifest.platform != platform
        || manifest.files.is_empty()
        || manifest.files.iter().any(|f| {
            !safe(&f.path)
                || !names.insert(f.path.to_lowercase())
                || f.sha256.len() != 64
                || !f.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        })
    {
        return unverified("The bundled file manifest is invalid or belongs to another target.");
    }
    if !root.is_dir() {
        return unverified("Game folder is unavailable.");
    }
    let mut report = Report {
        status: "vanilla".into(),
        reference: manifest.source.clone(),
        region: manifest.region.clone(),
        matched: 0,
        loose: manifest
            .files
            .iter()
            .filter(|e| !e.optional && e.kind == "loose")
            .count(),
        containers: manifest
            .files
            .iter()
            .filter(|e| !e.optional && e.kind == "container")
            .count(),
        findings: vec![],
    };
    let mut actual = BTreeMap::new();
    walk(root, root, &mut actual, &mut report.findings);
    for f in manifest.files.iter().filter(|e| !e.optional) {
        let issue = match actual.remove(&f.path.to_lowercase()) {
            None => Some("Missing original disc file".into()),
            Some(path) => (|| -> std::io::Result<Option<String>> {
                let mut file = std::fs::File::open(path)?;
                let before = file.metadata()?;
                if before.len() != f.size {
                    return Ok(Some("Size differs from reference".into()));
                }
                let digest = hash(&mut file)?;
                let after = file.metadata()?;
                if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
                    return Ok(Some("File changed during verification".into()));
                }
                Ok((digest != f.sha256).then(|| "SHA-256 differs from reference".into()))
            })()
            .unwrap_or_else(|e| Some(format!("Cannot verify file: {e}"))),
        };
        if let Some(issue) = issue {
            report.findings.push(Finding {
                path: f.path.clone(),
                issue,
            });
        } else {
            report.matched += 1;
        }
    }
    if !report.findings.is_empty() {
        report.status = "different".into();
    }
    for path in actual.values() {
        // Extraction receipts are Studio bookkeeping, not game modifications.
        let name = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if name.ends_with(crate::extract_service::MARKER_SUFFIX) {
            continue;
        }
        report.findings.push(Finding {
            path: name,
            issue: "Additional file (may be an extracted asset or a mod)".into(),
        });
        if report.status == "vanilla" {
            report.status = "additional".into();
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_check_distinguishes_original_loose_files_changes_and_extras() {
        let root = std::env::temp_dir().join(format!(
            "nhlms-manifest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let mut manifest = Manifest {
            schema: "nhlms-game-files/1".into(),
            game_id: "nhl07".into(),
            platform: "xbox".into(),
            source: "Test USA disc".into(),
            region: "USA".into(),
            image_sha256: "test".into(),
            anchors: vec![],
            files: vec![],
        };
        for (path, data, kind, optional) in [
            ("default.xex", b"ABCD".as_slice(), "loose", false),
            ("data.big", b"BIGF".as_slice(), "container", false),
            ("$SystemUpdate/update", b"UPDATE".as_slice(), "loose", true),
        ] {
            manifest.files.push(Entry {
                path: path.into(),
                size: data.len() as u64,
                sha256: hash(data).unwrap(),
                kind: kind.into(),
                optional,
            });
            if !optional {
                std::fs::write(root.join(path), data).unwrap();
            }
        }
        let original = check(&root, &manifest, "nhl07", "xbox");
        assert_eq!(
            (
                original.status.as_str(),
                original.matched,
                original.loose,
                original.containers
            ),
            ("vanilla", 2, 1, 1)
        );
        std::fs::write(root.join("loose-asset.rx2"), b"EXTRACTED").unwrap();
        assert_eq!(
            check(&root, &manifest, "nhl07", "xbox").status,
            "additional"
        );
        std::fs::write(root.join("default.xex"), b"EDIT").unwrap(); // same size: hash must catch it
        std::fs::remove_file(root.join("data.big")).unwrap();
        let changed = check(&root, &manifest, "nhl07", "xbox");
        assert_eq!(changed.status, "different");
        assert!(changed.findings.iter().any(|f| f.issue.contains("SHA-256")));
        assert!(changed.findings.iter().any(|f| f.issue.contains("Missing")));
        assert_eq!(
            std::fs::read(root.join("default.xex")).unwrap(),
            b"EDIT",
            "checker never repairs or overwrites"
        );
        assert_eq!(
            check(&root, &manifest, "nhl08", "xbox").status,
            "unverified"
        );
        manifest.files[0].path = "../outside".into();
        assert_eq!(
            check(&root, &manifest, "nhl07", "xbox").status,
            "unverified"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
