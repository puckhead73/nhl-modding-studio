//! Bounded, read-only ISO9660 / Joliet traversal for decrypted disc dumps.
//! Layout reference: ECMA-119 (2019), sections 8.4 and 9.1.
//! https://www.ecma-international.org/wp-content/uploads/ECMA-119_4th_edition_june_2019.pdf
//! Consecutive multi-extent file records are assembled; interleaving is refused.
use crate::{DomainError, Result};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Debug)]
pub(crate) struct Member {
    pub path: String,
    pub extents: Vec<(u64, u64)>,
    pub size: u64,
    pub directory: bool,
}
fn bad(s: impl Into<String>) -> DomainError {
    DomainError::Extract(s.into())
}
fn number32(b: &[u8]) -> Result<u32> {
    let le = u32::from_le_bytes(b[..4].try_into().unwrap());
    let be = u32::from_be_bytes(b[4..8].try_into().unwrap());
    if le != be {
        return Err(bad("ISO9660 integer copies disagree"));
    }
    Ok(le)
}
fn number16(b: &[u8]) -> Result<u16> {
    let le = u16::from_le_bytes(b[..2].try_into().unwrap());
    let be = u16::from_be_bytes(b[2..4].try_into().unwrap());
    if le != be {
        return Err(bad("ISO9660 integer copies disagree"));
    }
    Ok(le)
}
fn record(b: &[u8], image_len: u64) -> Result<(u64, u64, bool)> {
    if b.len() < 34 || b[0] as usize != b.len() || b[32] == 0 || 33 + b[32] as usize > b.len() {
        return Err(bad("malformed ISO9660 directory record"));
    }
    if b[26] != 0 || b[27] != 0 || b[25] & 0x82 == 0x82 {
        return Err(bad(format!("ISO9660 interleaved file or multi-extent directory {:?} is not supported (flags {}, unit {}, gap {})",
            String::from_utf8_lossy(&b[33..33+b[32] as usize]),b[25],b[26],b[27])));
    }
    if number16(&b[28..32])? != 1 {
        return Err(bad("multi-volume ISO9660 is not supported"));
    }
    let offset = (number32(&b[2..10])? as u64 + b[1] as u64) * 2048;
    let size = number32(&b[10..18])? as u64;
    if offset.checked_add(size).is_none_or(|end| end > image_len) {
        return Err(bad("ISO9660 member extends beyond the image"));
    }
    Ok((offset, size, b[25] & 2 != 0))
}
fn read(file: &mut File, offset: u64, len: usize) -> Result<Vec<u8>> {
    let mut b = vec![0; len];
    file.seek(SeekFrom::Start(offset))
        .and_then(|_| file.read_exact(&mut b))
        .map_err(|e| bad(format!("reading ISO9660: {e}")))?;
    Ok(b)
}
pub(crate) fn members(path: &Path) -> Result<Vec<Member>> {
    let mut file = File::open(path).map_err(|e| bad(format!("opening disc: {e}")))?;
    let len = file.metadata().map_err(|e| bad(e.to_string()))?.len();
    let mut root = None;
    let mut joliet_level = 0;
    for sector in 16..80u64 {
        if (sector + 1) * 2048 > len {
            break;
        }
        let b = read(&mut file, sector * 2048, 2048)?;
        if &b[1..6] != b"CD001" || b[6] != 1 {
            break;
        }
        if b[0] == 255 {
            break;
        }
        // Prefer Joliet's full UCS-2 names over primary-volume aliases, which may
        // truncate game asset names and extensions. ECMA-119 Annex C.
        let level = if b[0] == 2 && &b[88..90] == b"%/" {
            match b[90] { b'@' => 1, b'C' => 2, b'E' => 3, _ => 0 }
        } else { 0 };
        if (b[0] == 1 && root.is_none()) || level > joliet_level {
            if number16(&b[128..132])? != 2048 || number16(&b[120..124])? != 1 {
                return Err(bad(
                    "only single-volume ISO9660 with 2048-byte blocks is supported",
                ));
            }
            let size = b[156] as usize;
            if size < 34 || 156 + size > b.len() {
                return Err(bad("invalid ISO9660 root record"));
            }
            root = Some(record(&b[156..156 + size], len)?);
            joliet_level = level;
        }
    }
    let (off,size,dir) = root.ok_or_else(||bad("disc is neither supported XDVDFS nor ISO9660; use a decrypted ISO9660 image or an extracted game folder"))?;
    if !dir {
        return Err(bad("ISO9660 root is not a directory"));
    }
    let mut pending = vec![(String::new(), off, size, 0usize)];
    let mut visited = BTreeSet::new();
    let mut result: Vec<Member> = Vec::new();
    while let Some((parent, offset, size, depth)) = pending.pop() {
        if depth > 64 || size > 16 * 1024 * 1024 || !visited.insert(offset) {
            return Err(bad("ISO9660 directory cycle or traversal limit exceeded"));
        }
        let bytes = read(&mut file, offset, size as usize)?;
        let mut pos = 0usize;
        let mut continuation: Option<(usize, String, u8)> = None;
        while pos < bytes.len() {
            let n = bytes[pos] as usize;
            if n == 0 {
                pos = (pos / 2048 + 1) * 2048;
                continue;
            }
            if pos + n > bytes.len() || pos % 2048 + n > 2048 {
                return Err(bad("ISO9660 record crosses its directory sector"));
            }
            let r = &bytes[pos..pos + n];
            let (offset, size, directory) = record(r, len)?;
            pos += n;
            let name = &r[33..33 + r[32] as usize];
            if name == [0] || name == [1] {
                if continuation.is_some() {
                    return Err(bad("interrupted ISO9660 multi-extent file"));
                }
                continue;
            }
            let raw_name = if joliet_level > 0 {
                if name.len() % 2 != 0 { return Err(bad("odd-length Joliet filename")); }
                let units: Vec<u16> = name.chunks_exact(2)
                    .map(|pair| u16::from_be_bytes([pair[0], pair[1]])).collect();
                // Joliet is UCS-2: surrogate code units are not valid filenames.
                if units.iter().any(|u| (0xD800..=0xDFFF).contains(u)) {
                    return Err(bad("invalid UCS-2 Joliet filename"));
                }
                String::from_utf16(&units).map_err(|_| bad("invalid Joliet filename"))?
            } else {
                std::str::from_utf8(name)
                    .map_err(|_| bad("unsupported ISO9660 filename encoding"))?.to_string()
            };
            let name = raw_name.as_str();
            // Strip only a numeric ISO file-version suffix; extraction validates paths.
            let name = match name.rsplit_once(';') {
                Some((name, version))
                    if !directory && !version.is_empty() && version.bytes().all(|c| c.is_ascii_digit()) =>
                {
                    name
                }
                _ => name,
            };
            // Primary-volume extensionless files carry a separator before ;1.
            // Remove only that one separator; manifest collision checks still apply.
            let name = if !directory && joliet_level == 0 {
                name.strip_suffix('.').unwrap_or(name)
            } else { name };
            if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
                return Err(bad("unsafe ISO9660 filename"));
            }
            let path = if parent.is_empty() {
                name.to_string()
            } else {
                format!("{parent}/{name}")
            };
            let more = r[25] & 0x80 != 0;
            if more && size % 2048 != 0 {
                return Err(bad("non-final ISO9660 extent is not block aligned"));
            }
            if let Some((index, expected, flags)) = continuation.take() {
                if directory || expected != raw_name || flags != r[25] & !0x80 {
                    return Err(bad("mismatched ISO9660 multi-extent file records"));
                }
                let member = &mut result[index];
                member.size = member
                    .size
                    .checked_add(size)
                    .ok_or_else(|| bad("ISO9660 file size overflow"))?;
                member.extents.push((offset, size));
                if more {
                    continuation = Some((index, raw_name, flags));
                }
                continue;
            }
            if result.len() >= 100_000 {
                return Err(bad("ISO9660 member limit exceeded"));
            }
            if directory {
                pending.push((path.clone(), offset, size, depth + 1));
            }
            if more {
                continuation = Some((result.len(), raw_name, r[25] & !0x80));
            }
            result.push(Member {
                path,
                extents: vec![(offset, size)],
                size,
                directory,
            });
        }
        if continuation.is_some() {
            return Err(bad("unterminated ISO9660 multi-extent file"));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "read-only real ISO manifest comparison; NHL_QA_ISO and NHL_QA_DISC_DUMP required"]
    fn joliet_manifest_matches_independent_disc_dump_names_and_lengths() {
        let iso = std::path::PathBuf::from(std::env::var("NHL_QA_ISO").unwrap());
        let dump = std::path::PathBuf::from(std::env::var("NHL_QA_DISC_DUMP").unwrap());
        let members = super::members(&iso).unwrap();
        if let Ok(receipt) = std::env::var("NHL_QA_MANIFEST_OUT") {
            // Optional read-only audit receipt; never overwrite earlier evidence.
            let bytes = serde_json::to_vec_pretty(&members.iter().filter(|m| !m.directory)
                .map(|m| serde_json::json!({"path":m.path,"bytes":m.size}))
                .collect::<Vec<_>>()).unwrap();
            use std::io::Write;
            std::fs::OpenOptions::new().create_new(true).write(true).open(receipt)
                .unwrap().write_all(&bytes).unwrap();
        }
        let mut files = 0;
        for member in members.iter().filter(|m| !m.directory) {
            let path = dump.join(&member.path);
            let meta = std::fs::metadata(&path)
                .unwrap_or_else(|e| panic!("missing independent dump member {}: {e}", member.path));
            assert_eq!(meta.len(), member.size, "{}", member.path);
            files += 1;
        }
        eprintln!("{}: {files} full filenames and lengths match the independent dump", iso.display());
    }

    fn both32(b: &mut [u8], n: u32) {
        b[..4].copy_from_slice(&n.to_le_bytes());
        b[4..8].copy_from_slice(&n.to_be_bytes());
    }
    fn both16(b: &mut [u8], n: u16) {
        b[..2].copy_from_slice(&n.to_le_bytes());
        b[2..4].copy_from_slice(&n.to_be_bytes());
    }
    fn rec(name: &[u8], sector: u32, size: u32, dir: bool) -> Vec<u8> {
        let mut r = vec![0; (33 + name.len() + 1) & !1];
        r[0] = r.len() as u8;
        both32(&mut r[2..10], sector);
        both32(&mut r[10..18], size);
        r[25] = if dir { 2 } else { 0 };
        both16(&mut r[28..32], 1);
        r[32] = name.len() as u8;
        r[33..33 + name.len()].copy_from_slice(name);
        r
    }
    #[test]
    fn copies_nested_versioned_files_and_refuses_malformed_images_before_output() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp =
            std::env::temp_dir().join(format!("nhlms-iso9660-{}-{unique}", std::process::id()));
        std::fs::create_dir(&temp).unwrap();
        let mut image = vec![0; 36 * 2048];
        let p = &mut image[16 * 2048..17 * 2048];
        p[0] = 1;
        p[1..6].copy_from_slice(b"CD001");
        p[6] = 1;
        both16(&mut p[120..124], 1);
        both16(&mut p[128..132], 2048);
        let root = rec(&[0], 20, 2048, true);
        p[156..156 + root.len()].copy_from_slice(&root);
        let child = rec(b"PS3_GAME", 21, 2048, true);
        image[20 * 2048..20 * 2048 + child.len()].copy_from_slice(&child);
        let file = rec(b"TEST.DB;1", 22, 5, false);
        image[21 * 2048..21 * 2048 + file.len()].copy_from_slice(&file);
        image[22 * 2048..22 * 2048 + 5].copy_from_slice(b"hello");
        let iso = temp.join("disc.iso");
        std::fs::write(&iso, &image).unwrap();
        let out = temp.join("new location");
        let r = crate::disc_extract::extract(
            &iso,
            &out,
            &std::sync::atomic::AtomicBool::new(false),
            &mut |_, _| {},
        )
        .unwrap();
        assert_eq!((r.members, r.bytes), (1, 5));
        assert_eq!(
            std::fs::read(out.join("PS3_GAME/TEST.DB")).unwrap(),
            b"hello"
        );
        assert_eq!(std::fs::read(&iso).unwrap(), image);
        // Extensionless primary names normalize without permitting two entries
        // to overwrite each other on the destination filesystem.
        let mut plain = image.clone();
        plain[21 * 2048..22 * 2048].fill(0);
        let no_ext = rec(b"README.;1", 22, 5, false);
        plain[21 * 2048..21 * 2048 + no_ext.len()].copy_from_slice(&no_ext);
        std::fs::write(&iso, &plain).unwrap();
        let plain_out = temp.join("extensionless");
        crate::disc_extract::extract(&iso, &plain_out,
            &std::sync::atomic::AtomicBool::new(false), &mut |_, _| {}).unwrap();
        assert_eq!(std::fs::read(plain_out.join("PS3_GAME/README")).unwrap(), b"hello");
        let collision = rec(b"README;1", 22, 5, false);
        plain[21 * 2048 + no_ext.len()..21 * 2048 + no_ext.len() + collision.len()]
            .copy_from_slice(&collision);
        std::fs::write(&iso, &plain).unwrap();
        let collision_out = temp.join("collision");
        assert!(crate::disc_extract::extract(&iso, &collision_out,
            &std::sync::atomic::AtomicBool::new(false), &mut |_, _| {}).is_err());
        assert!(!collision_out.exists());

        // The supplementary directory preserves the full filename, even when
        // the primary tree has only an unrelated truncated alias.
        let mut joliet = image.clone();
        let descriptor = image[16 * 2048..17 * 2048].to_vec();
        joliet[17 * 2048..18 * 2048].copy_from_slice(&descriptor);
        joliet[17 * 2048] = 2;
        joliet[17 * 2048 + 88..17 * 2048 + 91].copy_from_slice(b"%/E");
        let unicode_root = rec(&[0], 25, 2048, true);
        joliet[17 * 2048 + 156..17 * 2048 + 156 + unicode_root.len()].copy_from_slice(&unicode_root);
        joliet[18 * 2048] = 255;
        joliet[18 * 2048 + 1..18 * 2048 + 6].copy_from_slice(b"CD001");
        joliet[18 * 2048 + 6] = 1;
        let full_name = "Alpha_Wipe_Col_2secs_1024_fullA.vp6";
        let encoded: Vec<u8> = full_name.encode_utf16().flat_map(u16::to_be_bytes).collect();
        let unicode_file = rec(&encoded, 22, 5, false);
        joliet[25 * 2048..25 * 2048 + unicode_file.len()].copy_from_slice(&unicode_file);
        std::fs::write(&iso, &joliet).unwrap();
        let unicode_out = temp.join("joliet");
        let copied = crate::disc_extract::extract(&iso, &unicode_out,
            &std::sync::atomic::AtomicBool::new(false), &mut |_, _| {}).unwrap();
        assert_eq!(copied.members, 1);
        assert_eq!(std::fs::read(unicode_out.join(full_name)).unwrap(), b"hello");
        assert!(!unicode_out.join("PS3_GAME").exists());
        // Bad Unicode is rejected before any output directory is created.
        joliet[25 * 2048 + 33..25 * 2048 + 35].copy_from_slice(&0xD800u16.to_be_bytes());
        std::fs::write(&iso, &joliet).unwrap();
        let invalid_out = temp.join("invalid-joliet");
        assert!(crate::disc_extract::extract(&iso, &invalid_out,
            &std::sync::atomic::AtomicBool::new(false), &mut |_, _| {}).is_err());
        assert!(!invalid_out.exists());
        // A file split over non-adjacent extents must be concatenated, not truncated
        // to the first section or copied through the unrelated intervening sector.
        let mut split = image.clone();
        let mut first = rec(b"TEST.DB;1", 22, 2048, false);
        first[25] = 0x80;
        let last = rec(b"TEST.DB;1", 24, 5, false);
        split[21 * 2048..22 * 2048].fill(0);
        split[21 * 2048..21 * 2048 + first.len()].copy_from_slice(&first);
        split[21 * 2048 + first.len()..21 * 2048 + first.len() + last.len()].copy_from_slice(&last);
        split[22 * 2048..23 * 2048].fill(7);
        split[24 * 2048..24 * 2048 + 5].copy_from_slice(b"hello");
        std::fs::write(&iso, &split).unwrap();
        let split_out = temp.join("split file");
        let r = crate::disc_extract::extract(
            &iso,
            &split_out,
            &std::sync::atomic::AtomicBool::new(false),
            &mut |_, _| {},
        )
        .unwrap();
        let mut expected = vec![7; 2048];
        expected.extend_from_slice(b"hello");
        assert_eq!(r.bytes, 2053);
        assert_eq!(
            std::fs::read(split_out.join("PS3_GAME/TEST.DB")).unwrap(),
            expected
        );
        for kind in 0..4 {
            let mut bad_image = image.clone();
            match kind {
                0 => bad_image[21 * 2048 + 25] |= 0x80, // Multi-extent must never silently truncate.
                1 => both32(&mut bad_image[21 * 2048 + 2..21 * 2048 + 10], 100),
                2 => {
                    let cycle = rec(b"LOOP", 20, 2048, true);
                    bad_image[21 * 2048..21 * 2048 + cycle.len()].copy_from_slice(&cycle);
                }
                _ => bad_image[16 * 2048 + 130] = 0, // Mismatched endian copies.
            }
            std::fs::write(&iso, bad_image).unwrap();
            let dest = temp.join(format!("bad-{kind}"));
            assert!(crate::disc_extract::extract(
                &iso,
                &dest,
                &std::sync::atomic::AtomicBool::new(false),
                &mut |_, _| {}
            )
            .is_err());
            assert!(!dest.exists());
        }
        std::fs::remove_dir_all(temp).unwrap();
    }
}
