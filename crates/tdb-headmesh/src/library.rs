//! Head-library scanning — classify entries under user-registered external roots.
//!
//! Roots are read-only (Phase 5 path-intake discipline: the app indexes and previews in
//! place, never writes into a root). An entry's kind decides which pipelines it can
//! feed: authored-weight components take Path A, everything else takes Path B or is
//! install-only.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// Guard against runaway trees; head libraries are shallow (corpus/<slug>/files).
const MAX_DEPTH: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    /// GLB + `.weights.json` sidecar (NHL24 Frosty component export) — Path A capable.
    Nhl24Component,
    /// `player_<id>.rx2` (+ optional `texlib_<id>.rx2`) — RW source / direct install.
    Rx2Pair,
    /// PS4 `.mcd` — source-only until the PS4 write path is proven.
    Mcd,
    /// Bare geometry (`.obj`, or `.glb` without a weights sidecar) — Path B only.
    LooseMesh,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryEntry {
    /// Stable id: path of the defining file relative to the scanned root.
    pub id: String,
    pub kind: EntryKind,
    /// Human-readable name (de-slugged when the file follows the
    /// `lastname_firstname_dd_mm_yyyy` corpus convention).
    pub display_name: String,
    /// The defining mesh file (absolute).
    pub mesh_path: PathBuf,
    /// Companion files discovered next to it (sidecar, textures, texlib).
    pub companions: Vec<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("library root does not exist or is not a directory: {0}")]
    BadRoot(PathBuf),
    #[error("failed to scan library root: {0}")]
    Io(#[from] std::io::Error),
}

/// Scan one root. Deterministic order (sorted by relative path).
pub fn scan_root(root: &Path) -> Result<Vec<LibraryEntry>, LibraryError> {
    if !root.is_dir() {
        return Err(LibraryError::BadRoot(root.to_path_buf()));
    }
    let mut entries = Vec::new();
    walk(root, root, 0, &mut entries)?;
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

fn walk(
    root: &Path,
    dir: &Path,
    depth: usize,
    out: &mut Vec<LibraryEntry>,
) -> Result<(), LibraryError> {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else {
            files.push(path);
        }
    }

    for path in &files {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        let kind = match ext.as_str() {
            "glb" => {
                if sidecar_for(path).is_some() {
                    Some(EntryKind::Nhl24Component)
                } else {
                    Some(EntryKind::LooseMesh)
                }
            }
            "obj" => Some(EntryKind::LooseMesh),
            "mcd" => Some(EntryKind::Mcd),
            "rx2" => {
                // Mesh side of the pair defines the entry; texlibs ride as companions.
                if stem.starts_with("player_") || stem.starts_with("head_") {
                    Some(EntryKind::Rx2Pair)
                } else {
                    None
                }
            }
            _ => None,
        };
        let Some(kind) = kind else { continue };

        let mut companions: Vec<PathBuf> = Vec::new();
        match kind {
            EntryKind::Nhl24Component => {
                companions.extend(sidecar_for(path));
                companions.extend(textures_near(&files, path));
            }
            EntryKind::Rx2Pair => {
                if let Some(id) = stem.strip_prefix("player_") {
                    let texlib = dir.join(format!("texlib_{id}.rx2"));
                    if files.contains(&texlib) {
                        companions.push(texlib);
                    }
                }
            }
            EntryKind::Mcd | EntryKind::LooseMesh => {
                companions.extend(textures_near(&files, path));
            }
        }

        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push(LibraryEntry {
            id: rel,
            kind,
            display_name: display_name_for(path),
            mesh_path: path.clone(),
            companions,
        });
    }

    for sub in dirs {
        walk(root, &sub, depth + 1, out)?;
    }
    Ok(())
}

/// `<stem minus trailing "_mesh">.weights.json` or `<stem>.weights.json` next to a GLB.
fn sidecar_for(glb: &Path) -> Option<PathBuf> {
    let stem = glb.file_stem()?.to_str()?;
    let dir = glb.parent()?;
    [
        dir.join(format!("{stem}.weights.json")),
        dir.join(format!(
            "{}.weights.json",
            stem.strip_suffix("_mesh").unwrap_or(stem)
        )),
    ]
    .into_iter()
    .find(|cand| cand.is_file())
}

/// Texture files in the same directory sharing the entry's leading name token.
fn textures_near(files: &[PathBuf], mesh: &Path) -> Vec<PathBuf> {
    let Some(prefix) = mesh
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.split('_').next().unwrap_or(s).to_ascii_lowercase())
    else {
        return Vec::new();
    };
    files
        .iter()
        .filter(|f| {
            let ext = f
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            matches!(ext.as_str(), "dds" | "png")
                && f.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.to_ascii_lowercase().starts_with(&prefix))
        })
        .cloned()
        .collect()
}

/// De-slug `lastname_firstname_dd_mm_yyyy` (the corpus / `dev_player_key` convention)
/// into "Firstname Lastname"; otherwise return a cleaned stem. Human-readable surfaces
/// rule: the raw slug stays available as the entry id.
pub fn display_name_for(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    // Strip known component suffixes (head_<slug>_mesh, <slug>_mesh, player_<id> …).
    let core = stem
        .strip_suffix("_mesh")
        .unwrap_or(stem)
        .strip_prefix("head_")
        .unwrap_or_else(|| stem.strip_suffix("_mesh").unwrap_or(stem));
    let parts: Vec<&str> = core.split('_').collect();
    if parts.len() >= 5 {
        let (name_parts, tail) = parts.split_at(parts.len() - 3);
        if tail.iter().all(|t| t.chars().all(|c| c.is_ascii_digit()))
            && tail[2].len() == 4
            && name_parts.len() >= 2
        {
            let mut words: Vec<String> = name_parts[1..]
                .iter()
                .chain(name_parts[..1].iter())
                .map(|w| capitalize(w))
                .collect();
            words.push(format!("({})", tail[2]));
            return words.join(" ");
        }
    }
    if parts.len() >= 2 && parts.iter().all(|p| !p.is_empty()) && parts.len() <= 4 {
        // first_last style folders (NHL24 licensed convention) or plain word stems.
        let all_alpha = parts.iter().all(|p| p.chars().all(char::is_alphabetic));
        if all_alpha {
            return parts
                .iter()
                .map(|w| capitalize(w))
                .collect::<Vec<_>>()
                .join(" ");
        }
    }
    stem.to_owned()
}

fn capitalize(w: &str) -> String {
    let mut chars = w.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_slugs_corpus_names() {
        assert_eq!(
            display_name_for(Path::new("abel_sid_22_02_1918.obj")),
            "Sid Abel (1918)"
        );
        assert_eq!(
            display_name_for(Path::new("head_demitra_pavol_29_11_1974_mesh.glb")),
            "Pavol Demitra (1974)"
        );
        assert_eq!(
            display_name_for(Path::new("jack_hughes.obj")),
            "Jack Hughes"
        );
        assert_eq!(
            display_name_for(Path::new("player_30101.rx2")),
            "player_30101"
        );
    }

    #[test]
    fn scan_classifies_entry_kinds() {
        let dir = std::env::temp_dir().join(format!("headlib_test_{}", std::process::id()));
        let comp = dir.join("corpus/demitra_pavol_29_11_1974/components");
        std::fs::create_dir_all(&comp).unwrap();
        std::fs::write(comp.join("head_demitra_pavol_29_11_1974_mesh.glb"), b"x").unwrap();
        std::fs::write(
            comp.join("head_demitra_pavol_29_11_1974_mesh.weights.json"),
            b"{}",
        )
        .unwrap();
        let rw = dir.join("prebuilt");
        std::fs::create_dir_all(&rw).unwrap();
        std::fs::write(rw.join("player_30101.rx2"), b"x").unwrap();
        std::fs::write(rw.join("texlib_30101.rx2"), b"x").unwrap();
        std::fs::write(rw.join("loose_scan.obj"), b"x").unwrap();
        std::fs::write(rw.join("kucherov_nikita.mcd"), b"x").unwrap();

        let entries = scan_root(&dir).unwrap();
        let kind_of = |suffix: &str| {
            entries
                .iter()
                .find(|e| e.id.ends_with(suffix))
                .map(|e| e.kind)
        };
        assert_eq!(kind_of("_mesh.glb"), Some(EntryKind::Nhl24Component));
        assert_eq!(kind_of("player_30101.rx2"), Some(EntryKind::Rx2Pair));
        assert_eq!(kind_of("loose_scan.obj"), Some(EntryKind::LooseMesh));
        assert_eq!(kind_of("kucherov_nikita.mcd"), Some(EntryKind::Mcd));
        // texlib rides as a companion of the rx2 pair, not its own entry
        assert!(entries.iter().all(|e| !e.id.ends_with("texlib_30101.rx2")));
        let pair = entries
            .iter()
            .find(|e| e.id.ends_with("player_30101.rx2"))
            .unwrap();
        assert_eq!(pair.companions.len(), 1);
        // glb sidecar attached
        let glb = entries
            .iter()
            .find(|e| e.kind == EntryKind::Nhl24Component)
            .unwrap();
        assert!(glb
            .companions
            .iter()
            .any(|c| c.ends_with("head_demitra_pavol_29_11_1974_mesh.weights.json")));

        std::fs::remove_dir_all(&dir).ok();
    }
}
