//! Big-file extraction — turn an un-extracted install (a folder of EA `.big`
//! archives) into the loose game files discovery expects (`nhlng.db`, its
//! metadata, textures, …).
//!
//! Xbox / Recomp installs ship the databases *packed inside* `.big` archives, so
//! a fresh install has no loose `nhlng.db` for [`crate::discovery`] to find. This
//! module walks the install root, parses every `.big`, and writes each entry out
//! **in place, preserving the archive's internal folder hierarchy** — an entry
//! named `db/nhlng.db` lands at `<archive-dir>/db/nhlng.db`. Re-running discovery
//! then resolves the modding minimum.
//!
//! `chunkzip`-wrapped entries are transparently inflated; `chunklzx` (LZX) entries
//! are counted and skipped (the codec is not yet implemented) rather than failing
//! the whole run. Entry names that would escape the archive's directory (absolute
//! paths, `..`) are refused — a `.big` never dictates writes outside its own tree.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use tdb_archive_big::Archive;

use crate::{DomainError, Result};

/// Bounded depth for the `.big` walk — deep enough for nested EA layouts, capped
/// so a pathological tree can't wander forever.
const MAX_WALK_DEPTH: usize = 8;

/// What an extraction run produced. Surfaced to the UI so the user sees exactly
/// what happened (including partial results — a chunklzx entry or one bad archive
/// doesn't abort the rest).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExtractSummary {
    /// `.big` archives that parsed and were processed.
    pub archives: usize,
    /// Entries written to disk.
    pub files_written: usize,
    /// Total bytes written (decompressed).
    pub bytes_written: u64,
    /// Entries skipped because they're `chunklzx`-wrapped or failed to decode.
    pub skipped_unsupported: usize,
    /// Entries skipped because their name would escape the archive's directory.
    pub skipped_unsafe: usize,
    /// `.big` files that failed to parse (corrupt / not actually a BIG archive).
    pub archives_failed: usize,
}

/// Find every `.big` archive under `root` (case-insensitive extension), to a
/// bounded depth. Used both to *detect* an un-extracted install and to drive
/// [`extract_install`].
pub fn find_big_archives(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                if depth < MAX_WALK_DEPTH {
                    stack.push((entry.path(), depth + 1));
                }
            } else if entry
                .path()
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("big"))
            {
                found.push(entry.path());
            }
        }
    }
    found.sort();
    found
}

/// Extract every `.big` archive under `root` in place, preserving each archive's
/// internal folder hierarchy (an entry `db/nhlng.db` in `<root>/foo/bar.big` is
/// written to `<root>/foo/db/nhlng.db`).
///
/// Best-effort per entry and per archive: an unsupported (`chunklzx`) or
/// corrupt entry is counted and skipped; a `.big` that fails to parse is counted
/// and skipped. Returns `Err` only when the root has no `.big` archives at all,
/// or when every archive present failed to parse (nothing was extracted).
pub fn extract_install(root: &Path) -> Result<ExtractSummary> {
    let archives = find_big_archives(root);
    if archives.is_empty() {
        return Err(DomainError::Extract(format!(
            "no .big archives found under {} — is this the game's install folder?",
            root.display()
        )));
    }

    let mut summary = ExtractSummary::default();
    for archive_path in &archives {
        // The archive's own directory is the extraction base (in place).
        let base = archive_path.parent().unwrap_or(root);
        match extract_one(archive_path, base, &mut summary) {
            Ok(()) => summary.archives += 1,
            Err(_) => summary.archives_failed += 1,
        }
    }

    if summary.archives == 0 {
        return Err(DomainError::Extract(format!(
            "found {} .big archive(s) under {}, but none could be parsed",
            archives.len(),
            root.display()
        )));
    }
    Ok(summary)
}

/// Parse one `.big` and write its entries under `base`. Returns `Err` if the
/// archive itself can't be read/parsed; per-entry problems are folded into
/// `summary` and don't abort the archive.
fn extract_one(archive_path: &Path, base: &Path, summary: &mut ExtractSummary) -> Result<()> {
    let bytes = std::fs::read(archive_path).map_err(|source| DomainError::Io {
        path: archive_path.to_string_lossy().to_string(),
        source,
    })?;
    let archive =
        Archive::parse(bytes).map_err(|e| DomainError::Extract(format!("{e}")))?;

    for idx in 0..archive.entries.len() {
        let name = archive.entries[idx].name.clone();
        let Some(dest) = safe_join(base, &name) else {
            summary.skipped_unsafe += 1;
            continue;
        };
        // chunklzx / decode failures are surfaced as skips, not hard errors —
        // the rest of the archive still extracts.
        let data = match archive.read_entry_at(idx) {
            Ok(d) => d,
            Err(_) => {
                summary.skipped_unsupported += 1;
                continue;
            }
        };
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|source| DomainError::Io {
                path: parent.to_string_lossy().to_string(),
                source,
            })?;
        }
        std::fs::write(&dest, &data).map_err(|source| DomainError::Io {
            path: dest.to_string_lossy().to_string(),
            source,
        })?;
        summary.files_written += 1;
        summary.bytes_written += data.len() as u64;
    }
    Ok(())
}

/// Join an archive entry name onto `base`, refusing anything that would escape
/// `base` (absolute paths, drive prefixes, `..`). EA entry names use forward
/// slashes; we treat both `/` and `\` as separators. Returns `None` for an
/// unsafe or empty name.
fn safe_join(base: &Path, name: &str) -> Option<PathBuf> {
    let mut out = base.to_path_buf();
    let mut pushed = false;
    for raw in name.split(['/', '\\']) {
        if raw.is_empty() || raw == "." {
            continue;
        }
        // Reject traversal and any component that isn't a plain name (an
        // absolute root, a Windows drive/prefix, or `..`).
        let comp = Path::new(raw).components().next();
        match comp {
            Some(Component::Normal(c)) if raw != ".." => {
                out.push(c);
                pushed = true;
            }
            _ => return None,
        }
    }
    pushed.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tdb_archive_big::ArchiveBuilder;

    /// A unique temp dir, cleaned up on drop.
    struct TempTree(PathBuf);
    impl TempTree {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("nhlms-extract-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempTree(dir)
        }
    }
    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Write a `.big` at `path` containing the given (name, body) entries.
    fn write_big(path: &Path, entries: &[(&str, &[u8])]) {
        let mut b = ArchiveBuilder::new();
        for (name, body) in entries {
            b.add_entry(*name, body.to_vec());
        }
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b.build().unwrap()).unwrap();
    }

    #[test]
    fn extracts_entries_preserving_internal_hierarchy() {
        let t = TempTree::new("hier");
        let root = &t.0;
        write_big(
            &root.join("game.big"),
            &[
                ("db/nhlng.db", b"NHLNG-DB-BYTES"),
                ("db/nhlng-meta.xml", b"<meta/>"),
                ("readme.txt", b"hi"),
            ],
        );

        let summary = extract_install(root).unwrap();
        assert_eq!(summary.archives, 1);
        assert_eq!(summary.files_written, 3);
        assert_eq!(fs::read(root.join("db/nhlng.db")).unwrap(), b"NHLNG-DB-BYTES");
        assert_eq!(fs::read(root.join("db/nhlng-meta.xml")).unwrap(), b"<meta/>");
        assert_eq!(fs::read(root.join("readme.txt")).unwrap(), b"hi");
    }

    #[test]
    fn extracts_in_place_relative_to_each_archive() {
        let t = TempTree::new("inplace");
        let root = &t.0;
        // Archive nested in a subfolder — entries land beside it, not at root.
        write_big(&root.join("sub/pack.big"), &[("data/x.bin", b"XX")]);

        extract_install(root).unwrap();
        assert!(root.join("sub/data/x.bin").is_file());
        assert!(!root.join("data/x.bin").exists());
    }

    #[test]
    fn refuses_path_traversal_entry_names() {
        let t = TempTree::new("evil");
        let root = &t.0;
        write_big(
            &root.join("evil.big"),
            &[("../escape.txt", b"nope"), ("ok.txt", b"yes")],
        );

        let summary = extract_install(root).unwrap();
        assert_eq!(summary.skipped_unsafe, 1);
        assert_eq!(summary.files_written, 1);
        assert!(root.join("ok.txt").is_file());
        assert!(!root.parent().unwrap().join("escape.txt").exists());
    }

    #[test]
    fn no_archives_is_an_error() {
        let t = TempTree::new("empty");
        let err = extract_install(&t.0).unwrap_err();
        assert!(matches!(err, DomainError::Extract(_)));
    }

    #[test]
    fn find_big_archives_is_recursive_and_sorted() {
        let t = TempTree::new("find");
        let root = &t.0;
        write_big(&root.join("a.big"), &[("x", b"1")]);
        write_big(&root.join("nested/b.big"), &[("y", b"2")]);
        let found = find_big_archives(root);
        assert_eq!(found.len(), 2);
        assert!(found[0].ends_with("a.big"));
        assert!(found[1].ends_with("b.big"));
    }
}
