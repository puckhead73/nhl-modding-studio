//! Xbox 360 disc images as a Studio input (overnight 2026-09-18, `p1.1-xbox360-iso-member-route`).
//!
//! When Xenia launches a title from its `.iso`, the extracted folder beside it is not what the
//! game reads. This module locates the main database and the localization dictionary **inside**
//! the image — as loose members (NHL 10 ships `db/nhlng.db` and `fe/loc/nhl_eng_us.db` on the
//! disc directly) or inside one of the root `.big` archives (NHL 11–15 and Legacy pack them) —
//! and reads them out without extracting anything else. Every location is measured, never
//! assumed from a neighbouring title: the archive that carries the database is found by binding
//! each root archive and searching its table.
//!
//! Reads only. Writing a member back into an image is a same-length byte-range replacement the
//! copied-target/in-place machinery has to perform and verify; it is designed here as
//! [`IsoWriteBack`] (the exact range and the identities to check) and not executed.

use std::io::Read;
use std::path::{Path, PathBuf};

use std::io::{Seek, SeekFrom};

use tdb_archive_big::eb::{EbArchive, EbHeader, HEADER_LEN};
use tdb_xiso::{Member, MemberReader, Xiso};

use crate::error::DomainError;
use crate::Result;

/// Something Studio addresses by member byte ranges: a disc image, or a single archive file (an
/// extracted tree's `cacheboot.big` / `cache.big` / …) whose only "member" is itself at
/// `[0, len)`. The same locate / read / range-patch machinery then serves both the disc route and
/// the tree's packed route without copying multi-hundred-MB archives.
pub enum Container {
    Image(Xiso),
    ArchiveFile { path: PathBuf, member: Member },
}

impl Container {
    /// Open a disc image; when the file carries no XDVDFS volume, accept it as a plain archive
    /// file if it starts with an `EB` or `BIGF`-family magic. Anything else is refused.
    pub fn open(path: &Path) -> Result<Container> {
        match Xiso::open(path) {
            Ok(x) => Ok(Container::Image(x)),
            Err(tdb_xiso::XisoError::NoVolume { .. }) => {
                let mut f = std::fs::File::open(path).map_err(|source| DomainError::Io {
                    path: path.display().to_string(),
                    source,
                })?;
                let len = f
                    .metadata()
                    .map_err(|source| DomainError::Io {
                        path: path.display().to_string(),
                        source,
                    })?
                    .len();
                let mut magic = [0u8; 4];
                f.read_exact(&mut magic).map_err(|source| DomainError::Io {
                    path: path.display().to_string(),
                    source,
                })?;
                let is_archive =
                    &magic[..2] == b"EB" || tdb_archive_big::KNOWN_MAGICS.iter().any(|m| **m == magic);
                if !is_archive || len > u32::MAX as u64 {
                    return Err(DomainError::Extract(format!(
                        "{} is neither an Xbox 360 disc image nor an EB/BIGF archive file (magic {:?})",
                        path.display(),
                        String::from_utf8_lossy(&magic)
                    )));
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "archive".into());
                Ok(Container::ArchiveFile {
                    path: path.to_path_buf(),
                    member: Member {
                        path: name,
                        sector: 0,
                        size: len as u32,
                        attributes: 0x80,
                    },
                })
            }
            Err(e) => Err(xiso_err(e)),
        }
    }

    pub fn layout(&self) -> &'static str {
        match self {
            Container::Image(x) => x.layout().as_str(),
            Container::ArchiveFile { .. } => "archive-file",
        }
    }

    pub fn members(&self) -> Vec<Member> {
        match self {
            Container::Image(x) => x.members().to_vec(),
            Container::ArchiveFile { member, .. } => vec![member.clone()],
        }
    }

    pub fn member(&self, path: &str) -> Option<Member> {
        match self {
            Container::Image(x) => x.member(path).cloned(),
            Container::ArchiveFile { .. } => None,
        }
    }

    /// The root archives to search for packed files: every root `.big` of an image, or the file.
    pub fn root_archives(&self) -> Vec<Member> {
        match self {
            Container::Image(x) => x
                .members()
                .iter()
                .filter(|m| !m.is_dir() && !m.path.contains('/') && m.path.to_ascii_lowercase().ends_with(".big"))
                .cloned()
                .collect(),
            Container::ArchiveFile { member, .. } => vec![member.clone()],
        }
    }

    pub fn member_range(&self, m: &Member) -> Result<(u64, u64)> {
        match self {
            Container::Image(x) => x.member_range(m).map_err(xiso_err),
            Container::ArchiveFile { member, .. } => {
                if m != member {
                    return Err(DomainError::Extract(format!(
                        "{} is not a member of the archive file {}",
                        m.path, member.path
                    )));
                }
                Ok((0, member.size as u64))
            }
        }
    }

    pub fn member_reader(&self, m: &Member) -> Result<MemberReader> {
        match self {
            Container::Image(x) => x.member_reader(m).map_err(xiso_err),
            Container::ArchiveFile { path, .. } => {
                let (s, e) = self.member_range(m)?;
                MemberReader::over_file(path, s, e - s).map_err(xiso_err)
            }
        }
    }

    pub fn read_member(&self, m: &Member) -> Result<Vec<u8>> {
        let mut r = self.member_reader(m)?;
        let mut out = Vec::with_capacity(m.size as usize);
        r.read_to_end(&mut out).map_err(|e| DomainError::Extract(format!("{}: {e}", m.path)))?;
        Ok(out)
    }
}

/// `BIGF`-family root archives larger than this are not searched (that reader needs the whole
/// buffer). EB archives are searched at any size: only their header and tables are read.
pub const MAX_SEARCHED_BIGF: u32 = 96 * 1024 * 1024;

pub const DB_MEMBER: &str = "db/nhlng.db";
// NHL 07's real Xbox 360 disc uses this earlier name; preserve its measured
// member path for staging/write-back instead of pretending it is a later game.
const DB_MEMBERS: &[&str] = &[DB_MEMBER, "db/NHL07NG.db"];
pub const DICT_MEMBER: &str = "fe/loc/nhl_eng_us.db";

/// How a root archive parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveKind {
    /// `EB\0\3` (the stager's bound layout; 16- or 20-byte rows).
    Eb,
    /// `BIGF` / `BIG4`-family (whole-buffer reader).
    Bigf,
}

/// Where a wanted file lives inside the image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Located {
    /// A loose member of the disc file system.
    Loose { member: Member },
    /// An entry of a root archive that is itself a disc member.
    InArchive {
        archive: Member,
        kind: ArchiveKind,
        entry_index: usize,
        entry_path: String,
        /// Absolute offset of the stored bytes inside the archive.
        entry_offset: u64,
        /// Stored (possibly compressed) length inside the archive.
        stored_size: u32,
    },
}

impl Located {
    /// Plain-language description for reports.
    pub fn describe(&self) -> String {
        match self {
            Located::Loose { member } => format!(
                "loose disc member {} (sector {}, {} bytes)",
                member.path, member.sector, member.size
            ),
            Located::InArchive {
                archive,
                kind,
                entry_index,
                entry_path,
                entry_offset,
                stored_size,
            } => format!(
                "{entry_path} = entry #{entry_index} of {} ({:?}, sector {}, {} bytes) at archive offset {entry_offset}, {stored_size} stored bytes",
                archive.path, kind, archive.sector, archive.size
            ),
        }
    }
}

/// One root archive that was probed while searching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbedArchive {
    pub path: String,
    pub size: u32,
    /// `"eb"`, `"bigf"`, another 4-byte magic rendered as text, or `"skipped (size)"` /
    /// `"unreadable: …"`.
    pub verdict: String,
}

/// The database and dictionary locations of one image.
#[derive(Debug, Clone)]
pub struct IsoDatabases {
    pub image: PathBuf,
    pub layout: &'static str,
    pub members: usize,
    pub nhlng: Option<Located>,
    pub dictionary: Option<Located>,
    pub probed: Vec<ProbedArchive>,
}

fn xiso_err(e: tdb_xiso::XisoError) -> DomainError {
    DomainError::Extract(e.to_string())
}

/// Open the image and locate `db/nhlng.db` and `fe/loc/nhl_eng_us.db`: loose first, then every
/// root `.big` in path order (EB tables only; `BIGF` up to [`MAX_SEARCHED_BIGF`]). Both locations are independent (a
/// title may ship one loose and the other packed).
pub fn locate_databases(image: &Path) -> Result<IsoDatabases> {
    let x = Container::open(image)?;
    let mut out = IsoDatabases {
        image: image.to_path_buf(),
        layout: x.layout(),
        members: x.members().len(),
        nhlng: DB_MEMBERS.iter().find_map(|name| x.member(name)).map(|member| Located::Loose { member }),
        dictionary: x.member(DICT_MEMBER).map(|member| Located::Loose { member }),
        probed: Vec::new(),
    };
    if out.nhlng.is_some() && out.dictionary.is_some() {
        return Ok(out);
    }
    for archive in x.root_archives() {
        if out.nhlng.is_some() && out.dictionary.is_some() {
            break;
        }
        let mut r = x.member_reader(&archive)?;
        let mut magic = [0u8; 4];
        if let Err(e) = r.read_exact(&mut magic) {
            out.probed.push(ProbedArchive {
                path: archive.path.clone(),
                size: archive.size,
                verdict: format!("unreadable: {e}"),
            });
            continue;
        }
        if &magic[..2] == b"EB" {
            match eb_tables(&mut r, archive.size as u64) {
                Ok(arc) => {
                    for (wants, slot) in [(DB_MEMBERS, &mut out.nhlng), (&[DICT_MEMBER][..], &mut out.dictionary)] {
                        if slot.is_some() {
                            continue;
                        }
                        if let Some((i, e)) = arc
                            .entries
                            .iter()
                            .enumerate()
                            .find(|(_, e)| wants.iter().any(|want| e.path.eq_ignore_ascii_case(want)))
                        {
                            *slot = Some(Located::InArchive {
                                archive: archive.clone(),
                                kind: ArchiveKind::Eb,
                                entry_index: i,
                                entry_path: e.path.clone(),
                                entry_offset: e.offset,
                                stored_size: e.size,
                            });
                        }
                    }
                    out.probed.push(ProbedArchive {
                        path: archive.path.clone(),
                        size: archive.size,
                        verdict: format!("eb ({} entries)", arc.entries.len()),
                    });
                }
                Err(e) => out.probed.push(ProbedArchive {
                    path: archive.path.clone(),
                    size: archive.size,
                    verdict: format!("eb: {e}"),
                }),
            }
        } else if archive.size > MAX_SEARCHED_BIGF {
            out.probed.push(ProbedArchive {
                path: archive.path.clone(),
                size: archive.size,
                verdict: format!("skipped (size; magic {:?})", String::from_utf8_lossy(&magic)),
            });
        } else if tdb_archive_big::KNOWN_MAGICS.iter().any(|m| **m == magic) {
            let bytes = x.read_member(&archive)?;
            match tdb_archive_big::Archive::parse(bytes) {
                Ok(arc) => {
                    for (wants, slot) in [(DB_MEMBERS, &mut out.nhlng), (&[DICT_MEMBER][..], &mut out.dictionary)] {
                        if slot.is_some() {
                            continue;
                        }
                        if let Some((i, e)) = arc
                            .entries
                            .iter()
                            .enumerate()
                            .find(|(_, e)| wants.iter().any(|want| e.name.eq_ignore_ascii_case(want)))
                        {
                            *slot = Some(Located::InArchive {
                                archive: archive.clone(),
                                kind: ArchiveKind::Bigf,
                                entry_index: i,
                                entry_path: e.name.clone(),
                                entry_offset: e.offset as u64,
                                stored_size: e.size,
                            });
                        }
                    }
                    out.probed.push(ProbedArchive {
                        path: archive.path.clone(),
                        size: archive.size,
                        verdict: format!("{} ({} entries)", arc.magic_str(), arc.entries.len()),
                    });
                }
                Err(e) => out.probed.push(ProbedArchive {
                    path: archive.path.clone(),
                    size: archive.size,
                    verdict: format!("bigf: {e}"),
                }),
            }
        } else {
            out.probed.push(ProbedArchive {
                path: archive.path.clone(),
                size: archive.size,
                verdict: format!("unknown magic {:?}", String::from_utf8_lossy(&magic)),
            });
        }
    }
    Ok(out)
}

/// Parse an EB archive's header and tables from a member window without reading any payload.
/// The table claims are validated against the member length before anything is sliced.
fn eb_tables(r: &mut MemberReader, member_len: u64) -> Result<EbArchive> {
    let err = |e: String| DomainError::Extract(e);
    r.seek(SeekFrom::Start(0)).map_err(|e| err(e.to_string()))?;
    let mut head = vec![0u8; HEADER_LEN];
    r.read_exact(&mut head).map_err(|e| err(e.to_string()))?;
    let header = EbHeader::parse(&head).map_err(|e| err(e.to_string()))?;
    let tables = header
        .table_layout(member_len)
        .map_err(|e| err(e.to_string()))?;
    let mut prefix = vec![0u8; tables.table_end as usize];
    r.seek(SeekFrom::Start(0)).map_err(|e| err(e.to_string()))?;
    r.read_exact(&mut prefix).map_err(|e| err(e.to_string()))?;
    EbArchive::parse(&prefix).map_err(|e| err(e.to_string()))
}

/// Read one located file out of the image, decoded (a `chunkzip` archive payload is inflated;
/// `chunklzx` decoded since 21 Sep 2026). Loose members come back verbatim.
pub fn read_located(image: &Path, located: &Located) -> Result<Vec<u8>> {
    let x = Container::open(image)?;
    match located {
        Located::Loose { member } => x.read_member(member),
        Located::InArchive {
            archive,
            kind,
            entry_index,
            entry_path,
            entry_offset,
            stored_size,
        } => {
            let mut r = x.member_reader(archive)?;
            match kind {
                ArchiveKind::Eb => {
                    r.seek(SeekFrom::Start(*entry_offset)).map_err(|e| {
                        DomainError::Extract(format!("{}: {e}", archive.path))
                    })?;
                    let mut raw = vec![0u8; *stored_size as usize];
                    r.read_exact(&mut raw).map_err(|e| {
                        DomainError::Extract(format!("{}#{entry_index} {entry_path}: {e}", archive.path))
                    })?;
                    match tdb_archive_big::eb::decode_payload(raw)
                        .map_err(|e| DomainError::Extract(format!("{entry_path}: {e}")))?
                    {
                        tdb_archive_big::eb::Payload::Data(d) => Ok(d),
                        tdb_archive_big::eb::Payload::UnsupportedLzx => Err(DomainError::Extract(
                            format!("{entry_path} in {} is chunklzx-compressed; that codec is not implemented", archive.path),
                        )),
                    }
                }
                ArchiveKind::Bigf => {
                    let bytes = x.read_member(archive)?;
                    let arc = tdb_archive_big::Archive::parse(bytes)
                        .map_err(|e| DomainError::Extract(format!("{}: {e}", archive.path)))?;
                    arc.read_entry_at(*entry_index)
                        .map_err(|e| DomainError::Extract(format!("{entry_path}: {e}")))
                }
            }
        }
    }
}

/// The exact byte range a same-length write-back would touch, with the identities to verify
/// before and after. Produced for the review/receipt; nothing here writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsoWriteBack {
    pub image: PathBuf,
    /// The disc member being replaced (the archive, or the loose file).
    pub member_path: String,
    /// Absolute byte range `[start, end)` in the image.
    pub start: u64,
    pub end: u64,
    pub len: u64,
}

/// Plan the write-back of a replacement for the disc member that carries `located`. The
/// replacement must be exactly the member's length: an image member cannot grow or move.
pub fn plan_write_back(image: &Path, located: &Located, replacement_len: u64) -> Result<IsoWriteBack> {
    let x = Container::open(image)?;
    let member = match located {
        Located::Loose { member } => member,
        Located::InArchive { archive, .. } => archive,
    };
    let (start, end) = x.member_range(member)?;
    let len = end - start;
    if replacement_len != len {
        return Err(DomainError::Validation(format!(
            "the replacement for disc member {} is {replacement_len} bytes but the member occupies exactly {len} bytes of the image; an image member cannot change length",
            member.path
        )));
    }
    Ok(IsoWriteBack {
        image: image.to_path_buf(),
        member_path: member.path.clone(),
        start,
        end,
        len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tdb_xiso::synth::{image, SynthFile};
    use tdb_xiso::Layout;

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nhlms-xisosrc-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("image.iso")
    }

    #[test]
    fn nhl07_database_name_is_found_loose_and_in_an_archive() {
        for packed in [false, true] {
            let db = b"early-game database";
            let dict = b"dictionary";
            let mut builder = tdb_archive_big::ArchiveBuilder::new();
            builder.add_entry("db/NHL07NG.db", db.to_vec());
            builder.add_entry(DICT_MEMBER, dict.to_vec());
            let archive = builder.build().unwrap();
            let files = if packed {
                vec![SynthFile { path: "boot.big", bytes: &archive }]
            } else {
                vec![SynthFile { path: "db/NHL07NG.db", bytes: db },
                     SynthFile { path: DICT_MEMBER, bytes: dict }]
            };
            let path = tmp(if packed { "nhl07-packed" } else { "nhl07-loose" });
            std::fs::write(&path, image(Layout::Xgd2, &files, None)).unwrap();
            let found = locate_databases(&path).unwrap();
            let located = found.nhlng.unwrap();
            assert_eq!(read_located(&path, &located).unwrap(), db);
            assert_eq!(read_located(&path, &found.dictionary.unwrap()).unwrap(), dict);
            assert!(located.describe().contains("NHL07NG.db"));
            std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
    }

    #[test]
    fn loose_members_are_found_and_read_and_write_back_is_length_bound() {
        let db = [0xABu8; 3000];
        let dict = b"dictionary bytes".to_vec();
        let files = [
            SynthFile {
                path: "db/nhlng.db",
                bytes: &db,
            },
            SynthFile {
                path: "fe/loc/nhl_eng_us.db",
                bytes: &dict,
            },
        ];
        // The synth builder places files at the root; name them with slashes is invalid, so use
        // a subdirectory-free layout and the loose names as given by a flat root here.
        let p = tmp("loose");
        std::fs::write(&p, image(Layout::Xgd2, &files, None)).unwrap();
        // The flat synth image stores "db/nhlng.db" as a root-level name; the locator's exact
        // path match still finds it (the game's disc layout is nested, real images are covered by
        // the ignored test).
        let found = locate_databases(&p).unwrap();
        assert_eq!(found.layout, "xgd2");
        let n = found.nhlng.clone().expect("db located");
        assert!(matches!(n, Located::Loose { .. }), "{n:?}");
        assert_eq!(read_located(&p, &n).unwrap(), db.to_vec());
        let d = found.dictionary.clone().expect("dictionary located");
        assert_eq!(read_located(&p, &d).unwrap(), dict);
        assert!(found.probed.is_empty(), "no archive search when both are loose");

        let plan = plan_write_back(&p, &n, 3000).unwrap();
        assert_eq!(plan.len, 3000);
        assert_eq!(plan.end - plan.start, 3000);
        let err = plan_write_back(&p, &n, 3001).unwrap_err().to_string();
        assert!(err.contains("cannot change length"), "{err}");
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn a_bigf_root_archive_carrying_the_database_is_searched() {
        let mut b = tdb_archive_big::ArchiveBuilder::new();
        b.add_entry("fe/loc/nhl_eng_us.db", b"DICT".to_vec());
        b.add_entry("db/nhlng.db", vec![7u8; 500]);
        let big = b.build().unwrap();
        let files = [
            SynthFile {
                path: "boot.big",
                bytes: &big,
            },
            SynthFile {
                path: "default.xex",
                bytes: b"XEX2",
            },
        ];
        let p = tmp("bigf");
        std::fs::write(&p, image(Layout::Xgd3, &files, None)).unwrap();
        let found = locate_databases(&p).unwrap();
        let n = found.nhlng.clone().expect("db located in the archive");
        match &n {
            Located::InArchive { archive, kind, entry_path, .. } => {
                assert_eq!(archive.path, "boot.big");
                assert_eq!(*kind, ArchiveKind::Bigf);
                assert_eq!(entry_path, "db/nhlng.db");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(read_located(&p, &n).unwrap(), vec![7u8; 500]);
        assert_eq!(read_located(&p, found.dictionary.as_ref().unwrap()).unwrap(), b"DICT");
        assert_eq!(found.probed.len(), 1);
        assert!(found.probed[0].verdict.starts_with("BIG"), "{:?}", found.probed);
        // Write-back is bound to the archive member's length, not the entry's.
        let plan = plan_write_back(&p, &n, big.len() as u64).unwrap();
        assert_eq!(plan.member_path, "boot.big");
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    /// Real image, opt-in: `NHL_STUDIO_XISO=<iso>`; prints and asserts the locations. The
    /// database must parse as a big-endian TDB and the dictionary must verify.
    #[test]
    #[ignore]
    fn real_image_database_and_dictionary_are_located_and_parse() {
        let iso = std::env::var("NHL_STUDIO_XISO").expect("NHL_STUDIO_XISO must name the image");
        let found = locate_databases(Path::new(&iso)).unwrap();
        eprintln!("{iso}: layout {} members {}", found.layout, found.members);
        for p in &found.probed {
            eprintln!("  probed {} ({} B): {}", p.path, p.size, p.verdict);
        }
        let n = found.nhlng.as_ref().expect("database located");
        eprintln!("  nhlng: {}", n.describe());
        let db = read_located(Path::new(&iso), n).unwrap();
        let parsed = tdb_core::TdbFile::parse(&db).unwrap();
        eprintln!(
            "  nhlng: {} bytes, {} tables, sha256 {}",
            db.len(),
            parsed.tables.len(),
            tdb_txn::FileIdentity::of_bytes(&db).sha256
        );
        assert!(parsed.header_crc_ok());
        let d = found.dictionary.as_ref().expect("dictionary located");
        eprintln!("  dictionary: {}", d.describe());
        let dict = read_located(Path::new(&iso), d).unwrap();
        let checks = tdb_loc::verify(&dict).unwrap();
        assert!(checks.iter().all(|c| c.matches()));
        eprintln!(
            "  dictionary: {} bytes, {} rows, sha256 {}",
            dict.len(),
            tdb_loc::LocDictionary::parse(&dict).unwrap().len(),
            tdb_txn::FileIdentity::of_bytes(&dict).sha256
        );
    }
}

#[cfg(test)]
mod container_tests {
    use super::*;

    /// A plain archive file (an extracted tree's `cacheboot.big`) is a container whose only member
    /// is itself: the database is located inside it, read, and its range is bound to the file.
    #[test]
    fn an_archive_file_is_a_container_of_one_member() {
        let dir = std::env::temp_dir().join(format!("nhlms-container-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut b = tdb_archive_big::ArchiveBuilder::new();
        b.add_entry("fe/loc/nhl_eng_us.db", b"DICT".to_vec());
        b.add_entry("db/nhlng.db", vec![3u8; 700]);
        let big = b.build().unwrap();
        let p = dir.join("cacheboot.big");
        std::fs::write(&p, &big).unwrap();
        let c = Container::open(&p).unwrap();
        assert_eq!(c.layout(), "archive-file");
        assert_eq!(c.members().len(), 1);
        let found = locate_databases(&p).unwrap();
        let n = found.nhlng.clone().unwrap();
        match &n {
            Located::InArchive { archive, entry_path, .. } => {
                assert_eq!(archive.path, "cacheboot.big");
                assert_eq!(entry_path, "db/nhlng.db");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(read_located(&p, &n).unwrap(), vec![3u8; 700]);
        assert_eq!(read_located(&p, found.dictionary.as_ref().unwrap()).unwrap(), b"DICT");
        let plan = plan_write_back(&p, &n, big.len() as u64).unwrap();
        assert_eq!((plan.start, plan.end), (0, big.len() as u64));
        // Neither an image nor an archive: refused with the magic named.
        let junk = dir.join("junk.bin");
        std::fs::write(&junk, vec![0u8; 70_000]).unwrap();
        let err = match Container::open(&junk) {
            Ok(_) => panic!("junk must be refused"),
            Err(e) => e.to_string(),
        };
        assert!(err.contains("neither"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
