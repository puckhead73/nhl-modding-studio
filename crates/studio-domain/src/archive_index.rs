//! Receipt 55 — a read-only **member-path index** of an install's `.big` archives, for existence checks on titles whose
//! assets live inside archives (PS3 NHL 09–14, research 2026-09-26 handoff 17 §9 / R134; Legacy Recompiled, 2026-09-28
//! handoff 04 / R138). Only directories are read — `EB\0\3` front tables (PS3 11–14, the Xbox 360 Legacy / Recompiled
//! variant) and `BIG4` directories cross-checked with their `.bh` sidecar (PS3 09 / 10). No payload is read, nothing is
//! written. An archive that cannot be listed is reported by name, never guessed around.
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::Result;

#[derive(Debug, Clone, Default)]
pub struct ArchiveIndex {
    /// Lower-case member path → the archive (file name) that holds it (the first in name order).
    pub members: BTreeMap<String, String>,
    /// `(archive, member count)` for every archive listed.
    pub archives: Vec<(String, usize)>,
    /// `(archive, reason)` for every archive that could not be listed.
    pub unreadable: Vec<(String, String)>,
}

impl ArchiveIndex {
    /// Every `.big` under `root` (bounded depth, as extraction walks it).
    pub fn build(root: &Path) -> Result<Self> {
        let mut idx = ArchiveIndex::default();
        let archives: Vec<PathBuf> = crate::extract::find_big_archives(root);
        for a in archives {
            let name = a.strip_prefix(root).unwrap_or(&a).to_string_lossy().replace('\\', "/");
            match list(&a) {
                // A loose package (`BIGF`, e.g. an extracted portrait `p<id>.big`) is content, not an install archive.
                Err(e) if e.starts_with("not an EB or BIG4 archive") => {}
                Ok(paths) => {
                    idx.archives.push((name.clone(), paths.len()));
                    for p in paths {
                        idx.members.entry(p.to_ascii_lowercase().replace('\\', "/")).or_insert_with(|| name.clone());
                    }
                }
                Err(e) => idx.unreadable.push((name, e)),
            }
        }
        Ok(idx)
    }

    /// The archive holding `path` (case-insensitive, `/`-separated).
    pub fn find(&self, path: &str) -> Option<&str> {
        self.members.get(&path.to_ascii_lowercase()).map(String::as_str)
    }

    /// A gen7 portrait member `fe/ion/artassets/playerheads/[<bucket>/]p<id>.big` (not the `editplayer` heads).
    pub fn portrait(&self, id: u64) -> Option<(&str, &str)> {
        let leaf = format!("/p{id}.big");
        self.members
            .iter()
            .find(|(p, _)| p.ends_with(&leaf) && p.starts_with("fe/ion/artassets/playerheads/") && p.matches('/').count() <= 5)
            .map(|(p, a)| (p.as_str(), a.as_str()))
    }
}

fn list(archive: &Path) -> std::result::Result<Vec<String>, String> {
    let mut f = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic == tdb_archive_big::eb::MAGIC_EB {
        return crate::ps4_vault::eb_index(archive).map(|e| e.into_iter().map(|x| x.path).collect()).map_err(|e| e.to_string());
    }
    if &magic == b"BIG4" {
        let bh = archive.with_extension("bh");
        let sidecar = std::fs::read(&bh).map_err(|e| format!("{}: {e} (a BIG4 directory is read with its .bh)", bh.display()))?;
        let mut f = std::fs::File::open(archive).map_err(|e| e.to_string())?;
        return tdb_archive_big::big4::BoundBig4::read_directory(&mut f, &sidecar).map(|e| e.into_iter().map(|x| x.path).collect()).map_err(|e| e.to_string());
    }
    Err(format!("not an EB or BIG4 archive (magic {magic:02x?})"))
}
