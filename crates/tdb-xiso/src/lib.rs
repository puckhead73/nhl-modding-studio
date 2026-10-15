//! XDVDFS — the Xbox / Xbox 360 game-disc file system — read side only.
//!
//! An Xbox 360 game image is a flat file whose game partition starts at a layout-specific
//! **base offset**; everything in the file system is addressed in 2048-byte sectors relative to
//! that base. The volume descriptor sits at sector 32 of the partition and is bracketed by the
//! 20-byte magic `MICROSOFT*XBOX*MEDIA` at both ends of the sector; between them are the root
//! directory table's start sector and byte size (little-endian u32s) and a FILETIME.
//!
//! A directory table is a binary tree of fixed-layout entries laid out contiguously in its
//! sectors: `left`/`right` (u16 LE, offsets in 4-byte units from the table start, `0` = none),
//! `sector` (u32 LE), `size` (u32 LE), `attributes` (u8; `0x10` = directory), `name_len` (u8) and
//! the name (ASCII, `name_len` bytes, entry padded to a 4-byte boundary). An entry never crosses
//! a sector boundary; the tail of a sector that cannot hold the next entry is `0xFF` padding.
//! A directory's own table starts at its `sector` and is `size` bytes long.
//!
//! Layouts and bases (the partition offsets this reader probes, in this order): XSF `0`,
//! XGD3 `0x0200_0000` + `0x8_0000` = `0x0208_0000`, XGD2 `0x0FD9_0000`, XGD1 `0x1830_0000`.
//! Both XGD2 (NHL 10–15, measured 2026-09-18 on the research listings) and XGD3 (NHL Legacy)
//! occur in the campaign's sources.
//!
//! What this crate does **not** do: it never writes, never extracts whole images, and never
//! interprets member contents. Member bytes are exposed through [`Xiso::member_range`] (absolute
//! byte range in the image) and [`Xiso::member_reader`] (a `Read + Seek` window over that range)
//! so an archive reader can bind to a member in place. Same-length member replacement is a
//! byte-range write on the image that a caller performs and verifies itself; nothing here claims
//! anything about relocating or resizing members.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub const SECTOR: u64 = 2048;
pub const MAGIC: &[u8; 20] = b"MICROSOFT*XBOX*MEDIA";
const VOLUME_SECTOR: u64 = 32;
const ATTR_DIRECTORY: u8 = 0x10;

/// The partition layouts probed, with their base offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// A bare game partition (no video partition), base 0.
    Xsf,
    Xgd3,
    Xgd2,
    Xgd1,
}

impl Layout {
    pub fn base(self) -> u64 {
        match self {
            Layout::Xsf => 0,
            Layout::Xgd3 => 0x0208_0000,
            Layout::Xgd2 => 0x0FD9_0000,
            Layout::Xgd1 => 0x1830_0000,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Layout::Xsf => "xsf",
            Layout::Xgd3 => "xgd3",
            Layout::Xgd2 => "xgd2",
            Layout::Xgd1 => "xgd1",
        }
    }

    const ALL: [Layout; 4] = [Layout::Xsf, Layout::Xgd3, Layout::Xgd2, Layout::Xgd1];
}

#[derive(Debug, thiserror::Error)]
pub enum XisoError {
    #[error("io error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{path}: no XDVDFS volume descriptor at any known partition base (xsf, xgd3, xgd2, xgd1)")]
    NoVolume { path: PathBuf },
    #[error("{path}: directory table at sector {sector} ({size} bytes) lies outside the image")]
    TableOutOfImage { path: PathBuf, sector: u32, size: u32 },
    #[error("{path}: directory entry at table offset {offset} of {dir:?} is malformed ({why})")]
    BadEntry {
        path: PathBuf,
        dir: String,
        offset: usize,
        why: &'static str,
    },
    #[error("{path}: member {member:?} (sector {sector}, {size} bytes) lies outside the image")]
    MemberOutOfImage {
        path: PathBuf,
        member: String,
        sector: u32,
        size: u32,
    },
    #[error("{path}: no member {member:?}")]
    NoSuchMember { path: PathBuf, member: String },
}

/// One file or directory of the image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Forward-slash path from the partition root, as stored (case preserved).
    pub path: String,
    /// First sector relative to the partition base.
    pub sector: u32,
    /// Byte size (a directory's table size for directories).
    pub size: u32,
    pub attributes: u8,
}

impl Member {
    pub fn is_dir(&self) -> bool {
        self.attributes & ATTR_DIRECTORY != 0
    }
}

/// An opened image: its layout, root table and the full member listing.
#[derive(Debug)]
pub struct Xiso {
    path: PathBuf,
    layout: Layout,
    image_len: u64,
    root_sector: u32,
    root_size: u32,
    members: Vec<Member>,
}

/// A `Read + Seek` window over one member's byte range of the image.
pub struct MemberReader {
    file: File,
    start: u64,
    len: u64,
    pos: u64,
}

impl MemberReader {
    /// A window over `[start, start + len)` of any file (the image, or a plain archive file whose
    /// only "member" is itself). The file is opened read-only.
    pub fn over_file(path: &Path, start: u64, len: u64) -> Result<Self, XisoError> {
        let file = File::open(path).map_err(|e| io_err(path, e))?;
        let total = file.metadata().map_err(|e| io_err(path, e))?.len();
        if start + len > total {
            return Err(XisoError::MemberOutOfImage {
                path: path.to_path_buf(),
                member: format!("[{start}, {})", start + len),
                sector: 0,
                size: len as u32,
            });
        }
        Ok(MemberReader {
            file,
            start,
            len,
            pos: 0,
        })
    }
}

impl Read for MemberReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len {
            return Ok(0);
        }
        let remaining = (self.len - self.pos) as usize;
        let want = buf.len().min(remaining);
        self.file.seek(SeekFrom::Start(self.start + self.pos))?;
        let n = self.file.read(&mut buf[..want])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for MemberReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let target: i128 = match pos {
            SeekFrom::Start(p) => p as i128,
            SeekFrom::End(d) => self.len as i128 + d as i128,
            SeekFrom::Current(d) => self.pos as i128 + d as i128,
        };
        if target < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before the member start",
            ));
        }
        self.pos = target as u64;
        Ok(self.pos)
    }
}

fn io_err(path: &Path, source: io::Error) -> XisoError {
    XisoError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn read_exact_at(file: &mut File, path: &Path, at: u64, buf: &mut [u8]) -> Result<(), XisoError> {
    file.seek(SeekFrom::Start(at)).map_err(|e| io_err(path, e))?;
    file.read_exact(buf).map_err(|e| io_err(path, e))
}

fn u16le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn u32le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

impl Xiso {
    /// Open an image, probe the known partition bases for the volume descriptor and read every
    /// directory table. Nothing but directory tables is read.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, XisoError> {
        let path = path.as_ref();
        let mut file = File::open(path).map_err(|e| io_err(path, e))?;
        let image_len = file.metadata().map_err(|e| io_err(path, e))?.len();
        let mut found = None;
        let mut sector = vec![0u8; SECTOR as usize];
        for layout in Layout::ALL {
            let at = layout.base() + VOLUME_SECTOR * SECTOR;
            if at + SECTOR > image_len {
                continue;
            }
            read_exact_at(&mut file, path, at, &mut sector)?;
            if &sector[..20] == MAGIC && &sector[SECTOR as usize - 20..] == MAGIC {
                found = Some((layout, u32le(&sector[20..24]), u32le(&sector[24..28])));
                break;
            }
        }
        let Some((layout, root_sector, root_size)) = found else {
            return Err(XisoError::NoVolume {
                path: path.to_path_buf(),
            });
        };
        let mut x = Xiso {
            path: path.to_path_buf(),
            layout,
            image_len,
            root_sector,
            root_size,
            members: Vec::new(),
        };
        let mut members = Vec::new();
        x.walk_table(&mut file, "", root_sector, root_size, &mut members)?;
        members.sort_by(|a, b| a.path.cmp(&b.path));
        x.members = members;
        Ok(x)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn base(&self) -> u64 {
        self.layout.base()
    }

    pub fn image_len(&self) -> u64 {
        self.image_len
    }

    pub fn root(&self) -> (u32, u32) {
        (self.root_sector, self.root_size)
    }

    /// Every member (files and directories), sorted by path.
    pub fn members(&self) -> &[Member] {
        &self.members
    }

    /// Find a member by path, case-insensitively (the game's own lookups are case-insensitive),
    /// accepting either slash.
    pub fn member(&self, path: &str) -> Option<&Member> {
        let want = path.replace('\\', "/").trim_start_matches('/').to_ascii_lowercase();
        self.members
            .iter()
            .find(|m| m.path.to_ascii_lowercase() == want)
    }

    /// The absolute byte range `[start, end)` of a member inside the image file.
    pub fn member_range(&self, m: &Member) -> Result<(u64, u64), XisoError> {
        let start = self.base() + m.sector as u64 * SECTOR;
        let end = start + m.size as u64;
        if end > self.image_len {
            return Err(XisoError::MemberOutOfImage {
                path: self.path.clone(),
                member: m.path.clone(),
                sector: m.sector,
                size: m.size,
            });
        }
        Ok((start, end))
    }

    /// A `Read + Seek` window over a member (its own file handle; the image is opened read-only).
    pub fn member_reader(&self, m: &Member) -> Result<MemberReader, XisoError> {
        let (start, end) = self.member_range(m)?;
        let file = File::open(&self.path).map_err(|e| io_err(&self.path, e))?;
        Ok(MemberReader {
            file,
            start,
            len: end - start,
            pos: 0,
        })
    }

    /// Read a whole member into memory (small members only — callers size-gate this).
    pub fn read_member(&self, m: &Member) -> Result<Vec<u8>, XisoError> {
        let mut r = self.member_reader(m)?;
        let mut out = Vec::with_capacity(m.size as usize);
        r.read_to_end(&mut out).map_err(|e| io_err(&self.path, e))?;
        Ok(out)
    }

    /// Convenience: the member at `path` or a `NoSuchMember` error.
    pub fn require(&self, path: &str) -> Result<&Member, XisoError> {
        self.member(path).ok_or_else(|| XisoError::NoSuchMember {
            path: self.path.clone(),
            member: path.to_string(),
        })
    }

    fn walk_table(
        &self,
        file: &mut File,
        prefix: &str,
        sector: u32,
        size: u32,
        out: &mut Vec<Member>,
    ) -> Result<(), XisoError> {
        if size == 0 {
            return Ok(());
        }
        let start = self.base() + sector as u64 * SECTOR;
        if start + size as u64 > self.image_len {
            return Err(XisoError::TableOutOfImage {
                path: self.path.clone(),
                sector,
                size,
            });
        }
        let mut table = vec![0u8; size as usize];
        read_exact_at(file, &self.path, start, &mut table)?;
        // Tree walk from offset 0 with an explicit stack; a link seen twice is a malformed table.
        let mut seen = BTreeSet::new();
        let mut stack = vec![0usize];
        let bad = |offset: usize, why: &'static str| XisoError::BadEntry {
            path: self.path.clone(),
            dir: if prefix.is_empty() {
                "/".to_string()
            } else {
                prefix.to_string()
            },
            offset,
            why,
        };
        while let Some(off) = stack.pop() {
            if !seen.insert(off) {
                return Err(bad(off, "link cycle"));
            }
            if off + 14 > table.len() {
                return Err(bad(off, "entry header past the table"));
            }
            let e = &table[off..];
            let left = u16le(&e[0..2]);
            let right = u16le(&e[2..4]);
            if left == 0xFFFF && right == 0xFFFF {
                // Sector padding reached through a link: an empty directory's table looks like
                // this at offset 0; anywhere else it is malformed.
                if off == 0 {
                    return Ok(());
                }
                return Err(bad(off, "link into sector padding"));
            }
            let msector = u32le(&e[4..8]);
            let msize = u32le(&e[8..12]);
            let attributes = e[12];
            let name_len = e[13] as usize;
            if off + 14 + name_len > table.len() {
                return Err(bad(off, "name past the table"));
            }
            let name = String::from_utf8_lossy(&e[14..14 + name_len]).into_owned();
            if name.is_empty() || name.contains('/') || name == "." || name == ".." {
                return Err(bad(off, "invalid name"));
            }
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            let m = Member {
                path: path.clone(),
                sector: msector,
                size: msize,
                attributes,
            };
            let is_dir = m.is_dir();
            out.push(m);
            if is_dir {
                self.walk_table(file, &path, msector, msize, out)?;
            }
            for link in [right, left] {
                if link != 0 {
                    let target = link as usize * 4;
                    if target >= table.len() {
                        return Err(bad(off, "link past the table"));
                    }
                    stack.push(target);
                }
            }
        }
        Ok(())
    }
}

/// Build a small image in memory (tests and fixtures): files at any nested path, laid out as
/// proper directory tables — enough to exercise layout probing, the tree walk, sector padding and
/// ranged reads without any real disc data.
pub mod synth {
    use super::*;
    use std::collections::BTreeMap;

    /// A file to place in the synthetic image (forward-slash path from the root).
    pub struct SynthFile<'a> {
        pub path: &'a str,
        pub bytes: &'a [u8],
    }

    #[derive(Default)]
    struct Node<'a> {
        files: BTreeMap<String, &'a [u8]>,
        dirs: BTreeMap<String, Node<'a>>,
    }

    fn entry(left: u16, right: u16, sector: u32, size: u32, attr: u8, name: &str) -> Vec<u8> {
        let mut e = Vec::new();
        e.extend_from_slice(&left.to_le_bytes());
        e.extend_from_slice(&right.to_le_bytes());
        e.extend_from_slice(&sector.to_le_bytes());
        e.extend_from_slice(&size.to_le_bytes());
        e.push(attr);
        e.push(name.len() as u8);
        e.extend_from_slice(name.as_bytes());
        while e.len() % 4 != 0 {
            e.push(0);
        }
        e
    }

    /// Lay out a directory table as a right-linked chain (a degenerate but valid tree), padded
    /// to whole sectors with `0xFF`.
    fn table(entries: &[(u32, u32, u8, String)]) -> Vec<u8> {
        let mut t = Vec::new();
        for (i, (sector, size, attr, name)) in entries.iter().enumerate() {
            let this_len = entry(0, 0, *sector, *size, *attr, name).len();
            let right = if i + 1 < entries.len() {
                ((t.len() + this_len) / 4) as u16
            } else {
                0
            };
            t.extend_from_slice(&entry(0, right, *sector, *size, *attr, name));
        }
        if t.is_empty() {
            t.extend_from_slice(&[0xFF; 4]);
        }
        while t.len() % SECTOR as usize != 0 {
            t.push(0xFF);
        }
        t
    }

    struct Placer {
        base: usize,
        cursor: usize,
    }

    impl Placer {
        /// Copy `bytes` at the cursor (sector-aligned) and return their sector.
        fn place(&mut self, img: &mut Vec<u8>, bytes: &[u8]) -> u32 {
            let s = ((self.cursor - self.base) / SECTOR as usize) as u32;
            if img.len() < self.cursor + bytes.len() {
                img.resize(self.cursor + bytes.len(), 0);
            }
            img[self.cursor..self.cursor + bytes.len()].copy_from_slice(bytes);
            self.cursor += bytes.len();
            while (self.cursor - self.base) % SECTOR as usize != 0 {
                self.cursor += 1;
            }
            if img.len() < self.cursor {
                img.resize(self.cursor, 0);
            }
            s
        }
    }

    /// Place a node's payloads and child tables, then its own table; returns (sector, size).
    fn emit(node: &Node<'_>, img: &mut Vec<u8>, p: &mut Placer) -> (u32, u32) {
        let mut entries: Vec<(u32, u32, u8, String)> = Vec::new();
        for (name, child) in &node.dirs {
            let (s, sz) = emit(child, img, p);
            entries.push((s, sz, 0x90, name.clone()));
        }
        for (name, bytes) in &node.files {
            let s = p.place(img, bytes);
            entries.push((s, bytes.len() as u32, 0x80, name.clone()));
        }
        let t = table(&entries);
        let s = p.place(img, &t);
        (s, t.len() as u32)
    }

    /// Build an image for `layout` holding `files` at their nested paths. When `mirror_under` is
    /// given, every file is placed again under that directory (a second tree level to walk).
    pub fn image(layout: Layout, files: &[SynthFile<'_>], mirror_under: Option<&str>) -> Vec<u8> {
        fn insert<'a>(root: &mut Node<'a>, path: &str, bytes: &'a [u8]) {
            let mut parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            let name = parts.pop().expect("file name");
            let mut node = root;
            for d in parts {
                node = node.dirs.entry(d.to_string()).or_default();
            }
            node.files.insert(name.to_string(), bytes);
        }
        let mut root: Node<'_> = Node::default();
        for f in files {
            insert(&mut root, f.path, f.bytes);
            if let Some(d) = mirror_under {
                insert(&mut root, &format!("{d}/{}", f.path), f.bytes);
            }
        }
        let base = layout.base() as usize;
        let mut img = vec![0u8; base + 33 * SECTOR as usize];
        let mut p = Placer {
            base,
            cursor: base + 33 * SECTOR as usize,
        };
        let (root_sector, root_size) = emit(&root, &mut img, &mut p);
        let vol = base + 32 * SECTOR as usize;
        img[vol..vol + 20].copy_from_slice(MAGIC);
        img[vol + 20..vol + 24].copy_from_slice(&root_sector.to_le_bytes());
        img[vol + 24..vol + 28].copy_from_slice(&root_size.to_le_bytes());
        let end = vol + SECTOR as usize;
        img[end - 20..end].copy_from_slice(MAGIC);
        img
    }
}

#[cfg(test)]
mod tests {
    use super::synth::{image, SynthFile};
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nhlms-xiso-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("image.iso")
    }

    #[test]
    fn probes_every_layout_and_walks_root_and_subdirectory() {
        let files = [
            SynthFile {
                path: "default.xex",
                bytes: b"XEX2....",
            },
            SynthFile {
                path: "cacheboot.big",
                bytes: &[7u8; 5000],
            },
            SynthFile {
                path: "z.txt",
                bytes: b"last",
            },
        ];
        for layout in [Layout::Xsf, Layout::Xgd3, Layout::Xgd2] {
            let p = tmp(layout.as_str());
            std::fs::write(&p, image(layout, &files, Some("gamedata"))).unwrap();
            let x = Xiso::open(&p).unwrap();
            assert_eq!(x.layout(), layout);
            let paths: Vec<&str> = x.members().iter().map(|m| m.path.as_str()).collect();
            assert_eq!(
                paths,
                [
                    "cacheboot.big",
                    "default.xex",
                    "gamedata",
                    "gamedata/cacheboot.big",
                    "gamedata/default.xex",
                    "gamedata/z.txt",
                    "z.txt"
                ]
            );
            assert!(x.member("GAMEDATA").unwrap().is_dir());
            let big = x.member("CacheBoot.BIG").unwrap();
            assert_eq!(big.size, 5000);
            assert_eq!(x.read_member(big).unwrap(), vec![7u8; 5000]);
            let (s, e) = x.member_range(big).unwrap();
            assert_eq!(e - s, 5000);
            assert_eq!(s % SECTOR, x.base() % SECTOR);
            let xex = x.require("default.xex").unwrap();
            let mut r = x.member_reader(xex).unwrap();
            let mut head = [0u8; 4];
            r.read_exact(&mut head).unwrap();
            assert_eq!(&head, b"XEX2");
            r.seek(SeekFrom::End(-4)).unwrap();
            let mut tail = [0u8; 4];
            r.read_exact(&mut tail).unwrap();
            assert_eq!(&tail, b"....");
            assert_eq!(r.read(&mut tail).unwrap(), 0, "reads stop at the member end");
            assert!(x.member("nope").is_none());
            assert!(matches!(
                x.require("nope"),
                Err(XisoError::NoSuchMember { .. })
            ));
            let _ = std::fs::remove_dir_all(p.parent().unwrap());
        }
    }

    #[test]
    fn a_file_without_a_volume_descriptor_is_refused() {
        let p = tmp("novol");
        std::fs::write(&p, vec![0u8; 100 * SECTOR as usize]).unwrap();
        assert!(matches!(Xiso::open(&p), Err(XisoError::NoVolume { .. })));
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn a_member_past_the_image_end_is_refused_not_read() {
        let files = [SynthFile {
            path: "a.bin",
            bytes: b"abc",
        }];
        let p = tmp("oob");
        let mut img = image(Layout::Xsf, &files, None);
        // Corrupt the root entry's size to exceed the image.
        let x = Xiso::open({
            std::fs::write(&p, &img).unwrap();
            &p
        })
        .unwrap();
        let (root_sector, _) = x.root();
        let off = root_sector as usize * SECTOR as usize + 8;
        img[off..off + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(&p, &img).unwrap();
        let x = Xiso::open(&p).unwrap();
        let a = x.member("a.bin").unwrap();
        assert!(matches!(
            x.member_range(a),
            Err(XisoError::MemberOutOfImage { .. })
        ));
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn a_link_cycle_is_refused() {
        let files = [SynthFile {
            path: "a.bin",
            bytes: b"abc",
        }];
        let p = tmp("cycle");
        let mut img = image(Layout::Xsf, &files, None);
        std::fs::write(&p, &img).unwrap();
        let (root_sector, _) = Xiso::open(&p).unwrap().root();
        // Point the root entry's right link at itself (offset 0).
        let off = root_sector as usize * SECTOR as usize;
        // right link = 0 means none; use left link to a bogus entry that links back.
        img[off + 2..off + 4].copy_from_slice(&0u16.to_le_bytes());
        // Craft a second entry at offset 20 (5 units) linking left back to 0? A link of 0 is
        // "none", so build a two-node cycle: entry A (off 0) -> right = 5 (off 20); entry B (off
        // 20) -> right = 0 is none... so use B.left = 5 (itself).
        img[off + 2..off + 4].copy_from_slice(&5u16.to_le_bytes());
        let b = off + 20;
        img[b..b + 2].copy_from_slice(&5u16.to_le_bytes()); // left -> itself
        img[b + 2..b + 4].copy_from_slice(&0u16.to_le_bytes());
        img[b + 4..b + 8].copy_from_slice(&1u32.to_le_bytes());
        img[b + 8..b + 12].copy_from_slice(&1u32.to_le_bytes());
        img[b + 12] = 0x80;
        img[b + 13] = 1;
        img[b + 14] = b'b';
        std::fs::write(&p, &img).unwrap();
        assert!(matches!(Xiso::open(&p), Err(XisoError::BadEntry { why: "link cycle", .. })));
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    /// Real-image check, opt-in: `NHL_STUDIO_XISO=<path to a .iso>`; the image must carry
    /// `default.xex` (XEX2) — and, when `NHL_STUDIO_XISO_LISTING` names a JSON listing with
    /// `{"base":..,"root_sector":..,"entries":[{"path","sector","size","attr"}]}`, every entry
    /// must match exactly. Never scans for images; a missing or unreadable path fails.
    #[test]
    #[ignore]
    fn real_image_lists_default_xex_and_matches_the_listing() {
        let iso = std::env::var("NHL_STUDIO_XISO").expect("NHL_STUDIO_XISO must name the image");
        let x = Xiso::open(&iso).unwrap();
        let xex = x.require("default.xex").unwrap();
        let mut r = x.member_reader(xex).unwrap();
        let mut head = [0u8; 4];
        r.read_exact(&mut head).unwrap();
        assert_eq!(&head, b"XEX2", "{iso}: default.xex does not start with XEX2");
        if let Ok(listing) = std::env::var("NHL_STUDIO_XISO_LISTING") {
            let text = std::fs::read_to_string(&listing).unwrap();
            // Minimal JSON field extraction without a JSON dependency: the listing is generated,
            // flat and regular ("path": "...", "sector": n, "size": n, "attr": n).
            let base: u64 = field(&text, "\"base\": ").parse().unwrap();
            assert_eq!(base, x.base());
            let root: u32 = field(&text, "\"root_sector\": ").parse().unwrap();
            assert_eq!(root, x.root().0);
            let mut n = 0usize;
            for chunk in text.split("\"path\": \"").skip(1) {
                let path = chunk.split('"').next().unwrap();
                let sector: u32 = field(chunk, "\"sector\": ").parse().unwrap();
                let size: u32 = field(chunk, "\"size\": ").parse().unwrap();
                let attr: u8 = field(chunk, "\"attr\": ").parse().unwrap();
                let m = x.member(path).unwrap_or_else(|| panic!("missing member {path}"));
                assert_eq!((m.sector, m.size, m.attributes), (sector, size, attr), "{path}");
                n += 1;
            }
            assert_eq!(n, x.members().len(), "member count differs from the listing");
        }
    }

    #[cfg(test)]
    fn field<'a>(text: &'a str, key: &str) -> &'a str {
        let i = text.find(key).unwrap_or_else(|| panic!("no {key}")) + key.len();
        let rest = &text[i..];
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        &rest[..end]
    }
}
