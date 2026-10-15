//! About / licences (receipt 46; dispatch 2026-09-28 §1): what the packaged archive says about Studio's own
//! licence, the third-party notices and the candidate's release status — read from the files shipped beside
//! the executable, offline. Nothing is fetched and nothing is invented: a file the archive should hold and does
//! not is reported by its path.
use std::path::{Path, PathBuf};

use crate::dto::{AboutDto, AboutLicenceFileDto};

pub const NOTICES: &str = "THIRD-PARTY-NOTICES.txt";
pub const STUDIO_LICENCE: &str = "LICENSE";
pub const BUILD_RECORD: &str = "BUILD-RECORD.json";
/// Folders whose files the page may open (relative to the archive root).
const LICENCE_DIRS: [&str; 2] = ["licenses", "helpers/nhlms-xma2-helper-notices/licenses"];
const MAX_TEXT: u64 = 4 << 20;

fn rel_files(root: &Path, dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.join(dir)];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            // Never follow a link out of the archive.
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(p);
            } else if let Ok(r) = p.strip_prefix(root) {
                out.push(r.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out.sort();
    out
}

/// Paths the notices text names under `licenses\` / `helpers\` (Windows separators, as written).
fn referenced_paths(notices: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in notices.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == ',' || c == ';') {
        let w = word.trim_end_matches(['.', ':']);
        if w.starts_with("licenses\\") || w.starts_with("helpers\\") {
            let p = w.replace('\\', "/").trim_end_matches('/').to_string();
            // Templates such as licenses\rust\<name>-<version>\ describe a pattern, not a file.
            if !p.contains('<') && !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

fn read_text(p: &Path) -> Result<String, String> {
    let meta = std::fs::metadata(p).map_err(|e| format!("{}: {e}", p.display()))?;
    if meta.len() > MAX_TEXT {
        return Err(format!("{}: {} bytes is larger than this page shows", p.display(), meta.len()));
    }
    let b = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok(String::from_utf8_lossy(&b).into_owned())
}

/// Read the About page from `root` (the folder holding the executable; `None` when it cannot be resolved).
pub fn read(root: Option<&Path>, version: &str) -> AboutDto {
    let mut problems = Vec::new();
    let mut dto = AboutDto {
        version: version.to_string(),
        studio_licence_spdx: "MIT".into(),
        studio_licence_text: None,
        packaged: false,
        root: root.map(|r| r.display().to_string()).unwrap_or_default(),
        candidate: None,
        release_ready: false,
        release_blockers: Vec::new(),
        notices: None,
        licence_files: Vec::new(),
        problems: Vec::new(),
    };
    let Some(root) = root else {
        problems.push("Studio could not resolve the folder it runs from, so the packaged notices cannot be read.".into());
        dto.problems = problems;
        return dto;
    };
    let record = root.join(BUILD_RECORD);
    if !record.is_file() {
        problems.push(format!(
            "This build is not running from an extracted release archive ({} is not beside the executable), so the packaged licence and notices are not available here.",
            record.display()
        ));
        dto.problems = problems;
        return dto;
    }
    dto.packaged = true;
    match read_text(&record).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).map_err(|e| format!("{}: {e}", record.display()))) {
        Ok(v) => {
            dto.candidate = v.get("candidate").and_then(|c| c.as_str()).map(str::to_string);
            dto.release_ready = v.get("release_ready").and_then(|r| r.as_bool()).unwrap_or(false);
            match v.get("release_blockers").and_then(|b| b.as_array()) {
                Some(b) => dto.release_blockers = b.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(),
                None => dto.release_blockers.push("The build record states no release readiness (assembled before the release gate existed).".into()),
            }
        }
        Err(e) => problems.push(format!("The build record cannot be read: {e}")),
    }
    let lic = root.join(STUDIO_LICENCE);
    if lic.is_file() {
        match read_text(&lic) {
            Ok(t) => dto.studio_licence_text = Some(t),
            Err(e) => problems.push(format!("Studio's licence file cannot be read: {e}")),
        }
    } else if dto.release_ready {
        problems.push(format!("Required file missing: {} (Studio's licence).", lic.display()));
    }
    let np = root.join(NOTICES);
    match read_text(&np) {
        Ok(t) => {
            for rel in referenced_paths(&t) {
                if !root.join(&rel).exists() {
                    problems.push(format!("The notices name {rel}, which is missing from {}.", root.display()));
                }
            }
            dto.notices = Some(t);
        }
        Err(_) => problems.push(format!("Required file missing: {} (third-party notices).", np.display())),
    }
    for d in LICENCE_DIRS {
        if !root.join(d).is_dir() {
            problems.push(format!("Required folder missing: {}.", root.join(d).display()));
        }
        dto.licence_files.extend(rel_files(root, d).into_iter().map(|path| AboutLicenceFileDto { path }));
    }
    dto.problems = problems;
    dto
}

/// One licence text by its archive-relative path; only the files [`read`] lists, the notices and Studio's licence.
pub fn read_file(root: &Path, rel: &str) -> Result<String, String> {
    let rel = rel.replace('\\', "/");
    let allowed = rel == NOTICES
        || rel == STUDIO_LICENCE
        || LICENCE_DIRS.iter().any(|d| rel_files(root, d).iter().any(|f| *f == rel));
    if !allowed {
        return Err(format!("{rel} is not one of the archive's licence or notice files"));
    }
    let p: PathBuf = root.join(&rel);
    let (Ok(cr), Ok(cp)) = (root.canonicalize(), p.canonicalize()) else {
        return Err(format!("{} is missing", p.display()));
    };
    if !cp.starts_with(&cr) {
        return Err(format!("{rel} resolves outside the archive"));
    }
    read_text(&cp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn referenced_paths_skip_templates_and_normalise() {
        let t = "Licence text: licenses\\libspeex\\COPYING.\nfiles under licenses\\rust\\<name>-<version>\\.\n(helpers\\nhlms-xma2-helper-notices\\source\\)";
        assert_eq!(referenced_paths(t), vec!["licenses/libspeex/COPYING", "helpers/nhlms-xma2-helper-notices/source"]);
    }
}
