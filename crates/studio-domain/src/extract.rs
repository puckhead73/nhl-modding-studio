//! Big-file extraction — turn an un-extracted install (a folder of EA `.big`
//! archives) into the loose game files discovery expects (`nhlng.db`, its
//! metadata, textures, …).
//!
//! Xbox / Recomp / PS4 installs ship the databases *packed inside* `.big`
//! archives, so a fresh install has no loose `nhlng.db` / `nhl_game.db` for
//! [`crate::discovery`] to find. This module walks the install root, parses
//! every `.big`, and writes each entry out **in place, preserving the archive's
//! internal folder hierarchy** — an entry named `db/nhlng.db` lands at
//! `<archive-dir>/db/nhlng.db`. Re-running discovery then resolves the modding
//! minimum.
//!
//! Two container families are handled by the same walk:
//!
//! * **Xbox / PS3 / Recomp** — EA `BIGF`/`BIG4` (`tdb_archive_big::Archive`).
//!   The whole archive is read into memory and sliced; these are small.
//! * **PS4** — EA `EB\0\3` (`tdb_archive_big::eb`). Parsed and extracted by
//!   **streaming** (front tables, then seek + read per entry), because a single
//!   PS4 archive (`game_90.big`) can be 17 GB — far too large to buffer.
//!
//! `chunkzip`-wrapped entries are transparently inflated; `chunklzx` (LZX) entries
//! are counted and skipped (the codec is not yet implemented) rather than failing
//! the whole run. Entry names that would escape the archive's directory (absolute
//! paths, `..`) are refused — a `.big` never dictates writes outside its own tree.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use tdb_archive_big::eb::{self, EbHeader, Payload};
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

/// Parse one `.big` and write its entries under `base`. Dispatches on the 4-byte
/// magic: PS4 `EB\0\3` archives stream ([`extract_eb`]); Xbox/PS3/Recomp
/// `BIGF`/`BIG4` archives are buffered ([`extract_bigf`]). Returns `Err` if the
/// archive itself can't be read/parsed; per-entry problems are folded into
/// `summary` and don't abort the archive.
fn extract_one(archive_path: &Path, base: &Path, summary: &mut ExtractSummary) -> Result<()> {
    let mut file = std::fs::File::open(archive_path).map_err(|source| DomainError::Io {
        path: archive_path.to_string_lossy().to_string(),
        source,
    })?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|source| DomainError::Io {
        path: archive_path.to_string_lossy().to_string(),
        source,
    })?;

    if &magic == eb::MAGIC_EB {
        extract_eb(archive_path, &mut file, base, summary)
    } else {
        extract_bigf(archive_path, base, summary)
    }
}

/// Xbox/PS3/Recomp path: read the whole `BIGF`/`BIG4` into memory and slice it.
/// These archives are small enough to buffer.
fn extract_bigf(archive_path: &Path, base: &Path, summary: &mut ExtractSummary) -> Result<()> {
    let bytes = std::fs::read(archive_path).map_err(|source| DomainError::Io {
        path: archive_path.to_string_lossy().to_string(),
        source,
    })?;
    let archive = Archive::parse(bytes).map_err(|e| DomainError::Extract(format!("{e}")))?;

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
        write_entry(&dest, &data, summary)?;
    }
    Ok(())
}

/// PS4 path: parse the `EB\0\3` front tables, then seek + read each entry's
/// payload on demand. Never buffers the whole archive — `game_90.big` is 17 GB.
fn extract_eb(
    archive_path: &Path,
    file: &mut std::fs::File,
    base: &Path,
    summary: &mut ExtractSummary,
) -> Result<()> {
    let io_err = |source| DomainError::Io {
        path: archive_path.to_string_lossy().to_string(),
        source,
    };
    let parse_err = |e| DomainError::Extract(format!("{e}"));

    // Read the fixed header, then exactly the bytes the table parse needs.
    let mut header_buf = vec![0u8; eb::HEADER_LEN];
    file.seek(SeekFrom::Start(0)).map_err(io_err)?;
    file.read_exact(&mut header_buf).map_err(io_err)?;
    let header = EbHeader::parse(&header_buf).map_err(parse_err)?;

    let mut table_buf = vec![0u8; header.table_end()];
    file.seek(SeekFrom::Start(0)).map_err(io_err)?;
    file.read_exact(&mut table_buf).map_err(io_err)?;
    let archive = eb::EbArchive::parse(&table_buf).map_err(parse_err)?;
    drop(table_buf);

    for entry in &archive.entries {
        let Some(dest) = safe_join(base, &entry.path) else {
            summary.skipped_unsafe += 1;
            continue;
        };
        // Stream just this entry's stored bytes.
        let mut raw = vec![0u8; entry.size as usize];
        if file.seek(SeekFrom::Start(entry.offset)).is_err() || file.read_exact(&mut raw).is_err() {
            // A truncated/unreadable entry is skipped, not fatal for the archive.
            summary.skipped_unsupported += 1;
            continue;
        }
        let data = match eb::decode_payload(raw) {
            Ok(Payload::Data(d)) => d,
            Ok(Payload::UnsupportedLzx) | Err(_) => {
                summary.skipped_unsupported += 1;
                continue;
            }
        };
        write_entry(&dest, &data, summary)?;
    }
    Ok(())
}

/// Create parent dirs and write one extracted entry, tallying it into `summary`.
fn write_entry(dest: &Path, data: &[u8], summary: &mut ExtractSummary) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|source| DomainError::Io {
            path: parent.to_string_lossy().to_string(),
            source,
        })?;
    }
    std::fs::write(dest, data).map_err(|source| DomainError::Io {
        path: dest.to_string_lossy().to_string(),
        source,
    })?;
    summary.files_written += 1;
    summary.bytes_written += data.len() as u64;
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

    /// Write a synthetic PS4 `EB\0\3` `.big` (raw payloads) at `path`. `files`
    /// are `(dir_index, basename, body)`; `dirs` are the directory paths.
    /// Mirrors the real layout (0x30 file table, 20-byte entries with 16-byte
    /// block offsets, `dir_index`-prefixed fixed name slots, 16-byte-aligned
    /// dir table + data) so `extract_eb` exercises the same parse it does on a
    /// real archive.
    fn write_eb_big(path: &Path, dirs: &[&str], files: &[(u16, &str, &[u8])]) {
        const HDR: usize = 0x30;
        let align16 = |n: usize| (n + 15) & !15;
        let name_slot = files.iter().map(|(_, n, _)| n.len() + 3).max().unwrap();
        let dir_slot = dirs.iter().map(|d| d.len() + 1).max().unwrap();
        let name_off = HDR + files.len() * 20;
        let dir_start = align16(name_off + files.len() * name_slot);
        let data_start = align16(dir_start + dirs.len() * dir_slot);

        let mut blocks = Vec::new();
        let mut cur = data_start;
        for (_, _, body) in files {
            blocks.push((cur >> 4, body.len()));
            cur = align16(cur + body.len());
        }
        let mut out = vec![0u8; data_start];
        out[0..4].copy_from_slice(b"EB\x00\x03");
        out[0x04..0x08].copy_from_slice(&(files.len() as u32).to_be_bytes());
        out[0x08..0x0C].copy_from_slice(&0x0011_0400u32.to_be_bytes());
        out[0x0C..0x10].copy_from_slice(&(name_off as u32).to_be_bytes());
        out[0x14] = name_slot as u8;
        out[0x15] = dir_slot as u8;
        out[0x16..0x18].copy_from_slice(&(dirs.len() as u16).to_be_bytes());
        out[0x18..0x20].copy_from_slice(&(cur as u64).to_be_bytes());
        for (i, (block, size)) in blocks.iter().enumerate() {
            let e = HDR + i * 20;
            out[e..e + 4].copy_from_slice(&(*block as u32).to_be_bytes());
            out[e + 8..e + 12].copy_from_slice(&(*size as u32).to_be_bytes());
        }
        for (i, (di, name, _)) in files.iter().enumerate() {
            let n = name_off + i * name_slot;
            out[n..n + 2].copy_from_slice(&di.to_be_bytes());
            out[n + 2..n + 2 + name.len()].copy_from_slice(name.as_bytes());
        }
        for (i, d) in dirs.iter().enumerate() {
            let o = dir_start + i * dir_slot;
            out[o..o + d.len()].copy_from_slice(d.as_bytes());
        }
        for ((_, _, body), (block, _)) in files.iter().zip(blocks.iter()) {
            let o = block << 4;
            out.resize(out.len().max(o + body.len()), 0);
            out[o..o + body.len()].copy_from_slice(body);
        }
        out.resize(cur, 0);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, out).unwrap();
    }

    #[test]
    fn extracts_ps4_eb_archive_by_streaming() {
        let t = TempTree::new("eb");
        let root = &t.0;
        // Two dirs; entries reference them by index. Mirrors a PS4 install:
        // the DB lives deep under nhl/data, textures under nhl/ui.
        write_eb_big(
            &root.join("inxp_00.big"),
            &["nhl/data/common/gamemodes/database/t3db", "nhl/ui/overlays"],
            &[
                (0, "nhl_game.db", b"PS4-MAIN-DB"),
                (0, "initial_experience.db", b"IXP-SNAPSHOT"),
                (1, "scoreclock.gfx", b"CFX%-raw-scaleform"),
            ],
        );

        let summary = extract_install(root).unwrap();
        assert_eq!(summary.archives, 1);
        assert_eq!(summary.files_written, 3);
        assert_eq!(
            fs::read(root.join("nhl/data/common/gamemodes/database/t3db/nhl_game.db")).unwrap(),
            b"PS4-MAIN-DB"
        );
        assert_eq!(
            fs::read(root.join("nhl/data/common/gamemodes/database/t3db/initial_experience.db"))
                .unwrap(),
            b"IXP-SNAPSHOT"
        );
        assert_eq!(
            fs::read(root.join("nhl/ui/overlays/scoreclock.gfx")).unwrap(),
            b"CFX%-raw-scaleform"
        );
    }

    #[test]
    fn eb_and_bigf_archives_extract_in_the_same_walk() {
        let t = TempTree::new("mixed");
        let root = &t.0;
        write_big(&root.join("xbox.big"), &[("db/nhlng.db", b"XBOX")]);
        write_eb_big(&root.join("ps4.big"), &["db"], &[(0, "nhl_game.db", b"PS4")]);

        let summary = extract_install(root).unwrap();
        assert_eq!(summary.archives, 2);
        assert_eq!(fs::read(root.join("db/nhlng.db")).unwrap(), b"XBOX");
        assert_eq!(fs::read(root.join("db/nhl_game.db")).unwrap(), b"PS4");
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

    /// Live check against a real PS4 `EB\0\3` archive on the operator's drive.
    /// Skipped when the drive isn't mounted. Extracts a small real archive and
    /// byte-compares against the reference `Extracted/` tree — exercising the
    /// actual streaming reader, chunkzip inflation, and path reconstruction on
    /// real bytes (not just synthetic fixtures).
    #[test]
    fn ps4_real_eb_archive_extracts_matching_reference() {
        let src = Path::new(r"H:\Emulators\ShadPS4\Games\CUSA18182\databigs\overlays_20.big");
        let reference = Path::new(r"H:\Emulators\ShadPS4\Games\CUSA18182\databigs\Extracted");
        if !src.is_file() || !reference.is_dir() {
            eprintln!("skipping: real PS4 databigs not present");
            return;
        }
        let t = TempTree::new("eb-real");
        fs::copy(src, t.0.join("overlays_20.big")).unwrap();

        let summary = extract_install(&t.0).unwrap();
        assert_eq!(summary.archives, 1);
        assert!(summary.files_written >= 6, "expected the 6 overlay entries");

        // A raw `.gfx` and a chunkzip-wrapped `.gnf` must both round-trip to the
        // reference bytes.
        for rel in [
            "nhl/ui/swfassets/kettle/overlays/energygain/energygainprogressbar.gfx",
            "nhl/ui/swfassets/kettle/overlays/energygain/energygainprogressbar_i1.gnf",
        ] {
            let got = fs::read(t.0.join(rel)).unwrap();
            let want = fs::read(reference.join(rel)).unwrap();
            assert_eq!(got, want, "{rel} did not match the reference extraction");
        }
    }

    /// Live check that a real NHL 20 `inxp_00.big` parses and yields
    /// `nhl_game.db` at the path discovery expects — this is the archive whose
    /// name records are variable-count / drift-prone. Reads only the front
    /// tables + the one DB entry (fast), so it runs the real `EbArchive`
    /// parser and codec on real bytes without a 500 MB full extraction.
    #[test]
    fn ps4_nhl20_inxp_yields_nhl_game_db() {
        let src = Path::new(r"H:\Emulators\ShadPS4\Games\CUSA15439\databigs\inxp_00.big");
        if !src.is_file() {
            eprintln!("skipping: real NHL20 inxp_00.big not present");
            return;
        }
        let mut file = std::fs::File::open(src).unwrap();
        let mut header_buf = vec![0u8; eb::HEADER_LEN];
        file.read_exact(&mut header_buf).unwrap();
        let header = EbHeader::parse(&header_buf).unwrap();
        let mut table_buf = vec![0u8; header.table_end()];
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_exact(&mut table_buf).unwrap();
        let archive = eb::EbArchive::parse(&table_buf).unwrap();

        let entry = archive
            .entries
            .iter()
            .find(|e| e.path == "nhl/data/common/gamemodes/database/t3db/nhl_game.db")
            .expect("nhl_game.db must resolve to the t3db path discovery searches");

        // Decode the payload and confirm it's a real (multi-MB) database.
        let mut raw = vec![0u8; entry.size as usize];
        file.seek(SeekFrom::Start(entry.offset)).unwrap();
        file.read_exact(&mut raw).unwrap();
        let Payload::Data(db) = eb::decode_payload(raw).unwrap() else {
            panic!("nhl_game.db payload should decode, not be chunklzx");
        };
        assert!(db.len() > 1_000_000, "nhl_game.db should be multi-MB, got {}", db.len());
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
