//! Read-only XDVDFS / ISO9660 disc materialization before archive extraction.
//! Extract privately, then publish a complete disc to a new or empty destination.
//! Failed/cancelled attempts leave no partial game folder blocking the next attempt.
use std::collections::BTreeSet;
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{DomainError, Result};

#[cfg(windows)]
#[path = "disc_extract_exiso.rs"]
mod exiso;

#[derive(Debug, serde::Serialize)]
pub struct DiscExtraction {
    pub root: PathBuf,
    pub members: usize,
    pub bytes: u64,
}

fn error(message: impl Into<String>) -> DomainError { DomainError::Extract(message.into()) }
fn io(path: &Path, source: std::io::Error) -> DomainError {
    DomainError::Io { path: path.display().to_string(), source }
}

// Validate Windows names even on other hosts, so the resulting tree is portable.
fn relative(name: &str) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for part in name.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.len() == 4 && matches!(stem.as_bytes()[3], b'1'..=b'9'));
        if part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' '])
            || part.chars().any(|c| c.is_control() || "\\:<>\"|?*".contains(c)) || device {
            return Err(error(format!("unsafe disc member name: {name:?}")));
        }
        out.push(part);
    }
    Ok(out)
}

/// Copy all supported disc files to a fresh destination. `progress` receives copied/total
/// bytes. No archive contents are inflated here; run extract_service afterward.
/// Success means every disc member was fully copied, not that all asset codecs work.
pub fn extract(
    image: &Path, destination: &Path, cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<DiscExtraction> {
    if cancel.load(Ordering::Relaxed) { return Err(error("ISO extraction cancelled. You can retry using the same destination.")); }
    // Identify the source before diagnosing a destination collision. A bad ISO should
    // never be reported as a folder-selection problem.
    std::fs::File::open(image).map_err(|e| error(format!("The selected ISO cannot be opened: {e}. Select an accessible ISO file.")))?;
    match tdb_xiso::Xiso::open(image) {
        Ok(_) => {},
        Err(tdb_xiso::XisoError::NoVolume { .. }) => {
            crate::disc_iso9660::members(image).map_err(|_| error("The selected file is not a readable game ISO. It may be damaged or unsupported. Select a valid game ISO."))?;
        },
        Err(e) => return Err(error(format!("The selected game ISO could not be read: {e}"))),
    }
    let parent = destination.parent().filter(|p| p.is_dir()).ok_or_else(|| error("The extraction destination's parent folder is unavailable. Choose an existing drive and parent folder."))?;
    let check_destination = || -> Result<bool> {
        match std::fs::symlink_metadata(destination) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(io(destination, e)),
            Ok(meta) => {
                #[cfg(windows)]
                let linked = { use std::os::windows::fs::MetadataExt; meta.file_attributes() & 0x400 != 0 };
                #[cfg(not(windows))]
                let linked = meta.file_type().is_symlink();
                if linked || !meta.is_dir() { return Err(error("The extraction destination must be a regular folder, not a file or folder link.")); }
                if std::fs::read_dir(destination).map_err(|e| io(destination, e))?.next().transpose().map_err(|e| io(destination, e))?.is_some() {
                    return Err(error("The extraction destination contains files. Choose an empty or new folder. To use an already extracted game, select Dumped ISO folder instead. Existing files have not been changed."));
                }
                Ok(true)
            }
        }
    };
    check_destination()?;
    let stage = tempfile::Builder::new().prefix(".nhlms-disc-").tempdir_in(parent)
        .map_err(|e| error(format!("Cannot write to the extraction location: {e}. Choose a writable folder.")))?;
    let staged = stage.path().join("game");
    let result = extract_new(image, &staged, cancel, progress);
    let mut receipt = match result {
        Ok(receipt) if !cancel.load(Ordering::Relaxed) => receipt,
        result => {
            let cancelled = cancel.load(Ordering::Relaxed);
            // Remove only this attempt's private tree, never the selected game folder.
            stage.close().map_err(|e| error(format!("Extraction stopped, but its temporary files could not be removed: {e}. The selected destination is unchanged.")))?;
            return if cancelled { Err(error("ISO extraction cancelled. You can retry using the same destination.")) }
                else { result };
        }
    };
    // Recheck after copying: never replace files added while extraction was running.
    if check_destination()? { std::fs::remove_dir(destination).map_err(|e| io(destination, e))?; }
    std::fs::rename(&staged, destination).map_err(|e| error(format!("Could not finish extraction at {}: {e}. Choose a writable, empty destination and retry.", destination.display())))?;
    receipt.root = destination.to_path_buf();
    Ok(receipt)
}

fn extract_new(
    image: &Path, destination: &Path, cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<DiscExtraction> {
    if cancel.load(Ordering::Relaxed) { return Err(error("disc extraction cancelled")); }
    let original = std::fs::metadata(image).map_err(|e| io(image, e))?;
    let xbox_disc = tdb_xiso::Xiso::open(image);
    #[cfg(windows)]
    let use_exiso = xbox_disc.is_ok();
    let members = match xbox_disc {
        Ok(disc) => disc.members().iter().map(|m| {
            let range = disc.member_range(m).map_err(|e|error(e.to_string()))?;
            Ok(crate::disc_iso9660::Member { path:m.path.clone(), extents:vec![(range.0,m.size as u64)],
                size:m.size as u64, directory:m.is_dir() })
        }).collect::<Result<Vec<_>>>()?,
        Err(tdb_xiso::XisoError::NoVolume { .. }) => crate::disc_iso9660::members(image)?,
        Err(e) => return Err(error(e.to_string())),
    };
    let mut names = BTreeSet::new();
    let mut files = Vec::new();
    let mut total = 0u64;
    // Validate the whole manifest before creating anything, including collisions
    // that would only become visible on a case-insensitive destination volume.
    for member in &members {
        let path = relative(&member.path)?;
        let folded = member.path.to_ascii_lowercase();
        if !names.insert(folded) { return Err(error(format!("duplicate disc path: {}", member.path))); }
        if !member.directory {
            total = total.checked_add(member.size as u64).ok_or_else(|| error("disc size overflow"))?;
            files.push((member, path));
        }
    }
    for (member, _) in &files {
        let prefix = format!("{}/", member.path.to_ascii_lowercase());
        if names.range(prefix.clone()..).next().is_some_and(|p| p.starts_with(&prefix)) {
            return Err(error(format!("disc file is also a directory: {}", member.path)));
        }
    }
    let mut input = std::fs::File::open(image).map_err(|e|io(image,e))?;
    // NHL PS3 boot containers must be readable before copying gigabytes of an
    // encrypted image. Do not label the filesystem copy an importable game.
    if let Some((boot, _)) = files.iter().find(|(m, _)| m.path.eq_ignore_ascii_case("PS3_GAME/USRDIR/boot.big")) {
        let mut magic = Vec::with_capacity(4);
        for &(offset, size) in &boot.extents {
            let count = size.min((4 - magic.len()) as u64) as usize;
            input.seek(SeekFrom::Start(offset)).map_err(|e|io(image,e))?;
            let mut part = vec![0; count];
            input.read_exact(&mut part).map_err(|e|io(image,e))?;
            magic.extend(part);
            if magic.len() == 4 { break; }
        }
        if !matches!(magic.as_slice(), b"EB\x00\x03" | b"BIGF" | b"BIG4") {
            return Err(error("PS3 boot.big is not a readable game archive. The image may be encrypted or damaged. Use a decrypted PS3 ISO or an existing extracted installation; no output folder was created."));
        }
    }
    // Windows Xbox extraction uses the exact headless engine supplied with XBOX360 ISO
    // Extract. Keep our complete manifest validation in front of the third-party writer.
    #[cfg(windows)]
    if use_exiso {
        return exiso::extract(image, destination, &members, cancel, progress);
    }
    if crate::disk::free_space(destination).is_some_and(|free| free < total) {
        return Err(error(format!("not enough free space to extract {total} disc bytes")));
    }
    // create_dir (not create_dir_all) claims one fresh root exclusively. Its parent
    // is selected/created by the caller; an existing directory or junction is refused.
    std::fs::create_dir(destination).map_err(|e| error(format!(
        "could not create new disc folder {}: {e}; select a new folder with an existing parent",
        destination.display())))?;
    let root = std::fs::canonicalize(destination).map_err(|e| io(destination, e))?;
    let mut copied = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    progress(0, total);
    for (member, path) in &files {
        let target = root.join(path);
        let parent = target.parent().unwrap();
        std::fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
        if !std::fs::canonicalize(parent).map_err(|e| io(parent,e))?.starts_with(&root) {
            return Err(error("disc destination resolves outside the new output folder"));
        }
        let mut output = std::fs::OpenOptions::new().write(true).create_new(true).open(&target).map_err(|e| io(&target,e))?;
        for &(offset,size) in &member.extents {
          input.seek(SeekFrom::Start(offset)).map_err(|e|io(image,e))?;
          let mut remaining = size;
          while remaining > 0 {
            if cancel.load(Ordering::Relaxed) { return Err(error(format!(
                "disc extraction cancelled; incomplete files remain in {}; choose a new folder to retry", root.display()))); }
            let count = remaining.min(buffer.len() as u64) as usize;
            input.read_exact(&mut buffer[..count]).map_err(|e| io(image,e))?;
            output.write_all(&buffer[..count]).map_err(|e| io(&target,e))?;
            remaining -= count as u64;
            copied += count as u64;
            progress(copied,total);
          }
        }
        output.sync_all().map_err(|e| io(&target,e))?;
    }
    let after = std::fs::metadata(image).map_err(|e| io(image,e))?;
    if original.len() != after.len() || original.modified().ok() != after.modified().ok() {
        return Err(error("source image changed during extraction; output is not a verified complete copy"));
    }
    if cancel.load(Ordering::Relaxed) { return Err(error("disc extraction cancelled before completion")); }
    Ok(DiscExtraction { root, members: files.len(), bytes: copied })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn supplied_exiso_extracts_xgd2_and_skips_system_update() {
        use tdb_xiso::{Layout, synth::{image, SynthFile}};
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let temp = std::env::temp_dir().join(format!("nhlms-exiso-xgd2-{}-{unique}", std::process::id()));
        std::fs::create_dir(&temp).unwrap();
        let partition = image(Layout::Xsf, &[
            SynthFile { path: "default.xex", bytes: b"game executable" },
            SynthFile { path: "data/cache.big", bytes: b"packed game bytes" },
            SynthFile { path: "$SystemUpdate/update.bin", bytes: b"dashboard update" },
        ], None);
        let iso = temp.join("game source.iso");
        let mut file = std::fs::File::create(&iso).unwrap();
        file.seek(SeekFrom::Start(Layout::Xgd2.base())).unwrap();
        file.write_all(&partition).unwrap();
        file.sync_all().unwrap();
        drop(file);
        let len = std::fs::metadata(&iso).unwrap().len();
        let out = temp.join("extracted game");
        let result = extract(&iso, &out, &AtomicBool::new(false), &mut |_, _| {}).unwrap();
        assert_eq!(result.members, 2);
        assert_eq!(result.bytes, 32);
        assert_eq!(std::fs::read(out.join("default.xex")).unwrap(), b"game executable");
        assert_eq!(std::fs::read(out.join("data/cache.big")).unwrap(), b"packed game bytes");
        assert!(!out.join("$SystemUpdate").exists());
        assert_eq!(std::fs::metadata(&iso).unwrap().len(), len);
        let mut file = std::fs::File::open(&iso).unwrap();
        file.seek(SeekFrom::Start(Layout::Xgd2.base())).unwrap();
        let mut after = Vec::new(); file.read_to_end(&mut after).unwrap();
        assert_eq!(after, partition);
        std::fs::remove_dir_all(&temp).unwrap();
    }
    #[test]
    fn streams_files_preserves_source_and_refuses_reuse_and_cancelled_output() {
        use tdb_xiso::synth::{image, SynthFile};
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let temp = std::env::temp_dir().join(format!("nhlms-disc-{}-{unique}",std::process::id()));
        std::fs::create_dir(&temp).unwrap();
        let large = vec![73u8; 2 * 1024 * 1024 + 19];
        let original = image(tdb_xiso::Layout::Xsf, &[
            SynthFile { path:"db/data.db",bytes:&large },
            SynthFile { path:"default.xex",bytes:b"executable" },
            SynthFile { path:"empty",bytes:b"" },
        ],None);
        let iso = temp.join("game.iso");
        std::fs::write(&iso,&original).unwrap();
        let cancel = AtomicBool::new(false);
        let output = temp.join("arbitrary folder with spaces");
        let mut calls = Vec::new();
        let receipt = extract(&iso,&output,&cancel,&mut |n,total| calls.push((n,total))).unwrap();
        assert_eq!(receipt.members,3);
        assert_eq!(receipt.bytes,large.len() as u64 + 10);
        assert_eq!(std::fs::read(output.join("db/data.db")).unwrap(),large);
        assert_eq!(std::fs::metadata(output.join("empty")).unwrap().len(),0);
        assert_eq!(calls.last(),Some(&(receipt.bytes,receipt.bytes)));
        assert!(calls.windows(2).all(|w| w[0].0 <= w[1].0));
        std::fs::write(output.join("db/data.db"),b"user edit").unwrap();
        assert!(extract(&iso,&output,&cancel,&mut |_,_|{}).is_err());
        assert_eq!(std::fs::read(output.join("db/data.db")).unwrap(),b"user edit");
        let partial = temp.join("cancelled");
        let result = extract(&iso,&partial,&cancel,&mut |_,_| { cancel.store(true,Ordering::Relaxed); });
        assert!(result.unwrap_err().to_string().contains("cancelled"));
        assert!(!partial.exists(), "cancelled output is never published");
        cancel.store(false, Ordering::Relaxed);
        extract(&iso, &partial, &cancel, &mut |_, _| {}).unwrap();
        assert_eq!(std::fs::read(partial.join("db/data.db")).unwrap(), large);
        let empty = temp.join("existing-empty");
        std::fs::create_dir(&empty).unwrap();
        assert!(extract(&iso, &empty, &cancel, &mut |_, _| { cancel.store(true, Ordering::Relaxed); }).is_err());
        assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
        cancel.store(false, Ordering::Relaxed);
        extract(&iso, &empty, &cancel, &mut |_, _| {}).unwrap();
        assert_eq!(std::fs::read(empty.join("db/data.db")).unwrap(), large);
        // Files introduced by another process during copying must survive publication.
        let changed = temp.join("changed-destination");
        std::fs::create_dir(&changed).unwrap();
        let err = extract(&iso, &changed, &cancel, &mut |_, _| {
            std::fs::write(changed.join("keep.txt"), b"user file").unwrap();
        }).unwrap_err();
        assert!(err.to_string().contains("contains files"));
        assert_eq!(std::fs::read(changed.join("keep.txt")).unwrap(), b"user file");
        assert_eq!(std::fs::read_dir(&changed).unwrap().count(), 1);
        cancel.store(true, Ordering::Relaxed);
        assert!(extract(&iso,&temp.join("never-created"),&cancel,&mut |_,_|{}).is_err());
        assert!(!temp.join("never-created").exists());
        assert_eq!(std::fs::read(&iso).unwrap(),original);
        let bad = temp.join("bad.iso");
        let unreadable = image(tdb_xiso::Layout::Xsf, &[
            SynthFile { path:"PS3_GAME/USRDIR/boot.big", bytes:b"encrypted payload" },
        ], None);
        std::fs::write(&bad, &unreadable).unwrap();
        let refused = temp.join("encrypted-output");
        let err = extract(&bad, &refused, &AtomicBool::new(false), &mut |_,_|{}).unwrap_err();
        assert!(err.to_string().contains("encrypted or damaged"));
        assert!(!refused.exists());
        assert_eq!(std::fs::read(&bad).unwrap(), unreadable);
        for (index, magic) in [b"EB\x00\x03", b"BIGF", b"BIG4"].iter().enumerate() {
            std::fs::write(&bad, image(tdb_xiso::Layout::Xsf, &[
                SynthFile { path:"PS3_GAME/USRDIR/boot.big", bytes:*magic },
            ], None)).unwrap();
            extract(&bad, &temp.join(format!("readable-{index}")), &AtomicBool::new(false), &mut |_,_|{}).unwrap();
        }
        for files in [
            vec![SynthFile { path:"NUL.bin",bytes:b"x" }],
            vec![SynthFile { path:"DB/a",bytes:b"a" },SynthFile { path:"db/a",bytes:b"b" }],
        ] {
            std::fs::write(&bad,image(tdb_xiso::Layout::Xsf,&files,None)).unwrap();
            assert!(extract(&bad,&temp.join("unsafe-output"),&AtomicBool::new(false),&mut |_,_|{}).is_err());
            assert!(!temp.join("unsafe-output").exists(), "validate names before creating output");
        }
        // Only this test's newly created, uniquely named directory is removed.
        std::fs::remove_dir_all(&temp).unwrap();
    }
    #[test]
    fn rejects_nonportable_and_escaping_names() {
        for name in ["../escape", "/absolute", "C:/drive", "a\\b", "a//b", "a/..", "NUL.bin", "a/LPT1", "a/file.", "a/file ", "a:stream"] {
            assert!(relative(name).is_err(), "{name}");
        }
        assert_eq!(relative("PS3_GAME/USRDIR/db/nhlng.db").unwrap(), PathBuf::from("PS3_GAME/USRDIR/db/nhlng.db"));
    }
}
