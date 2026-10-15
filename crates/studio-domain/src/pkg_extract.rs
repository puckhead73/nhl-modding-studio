//! PS4 base-package preparation through the standalone shadPS4Plus extractor.
//! Originals are read-only; every attempt claims a fresh output directory.
use crate::{DomainError, Platform, Result};
use std::{
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

fn error(s: impl Into<String>) -> DomainError {
    DomainError::Extract(s.into())
}
fn io(p: &Path, e: std::io::Error) -> DomainError {
    DomainError::Io {
        path: p.display().to_string(),
        source: e,
    }
}

fn command(extractor: &Path) -> Command {
    let mut cmd = Command::new(extractor);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CLI helper stays hidden; Studio owns its progress UI.
    }
    cmd
}

/// Always reap a cancelled/timed-out child before returning to the caller.
fn wait(
    child: &mut Child,
    cancel: &AtomicBool,
    timeout: Option<Duration>,
    tick: &mut dyn FnMut(),
) -> Result<ExitStatus> {
    let start = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) || timeout.is_some_and(|limit| start.elapsed() > limit) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error(if cancel.load(Ordering::Relaxed) {
                "Package extraction cancelled; partial output was retained. Select a new folder to retry."
            } else {
                "Package type check timed out."
            }));
        }
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error(format!("Could not monitor package extractor: {e}")));
            }
        }
        tick();
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn log_tail(path: &Path) -> String {
    let Ok(mut f) = std::fs::File::open(path) else {
        return String::new();
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = f.seek(SeekFrom::Start(len.saturating_sub(8192)));
    let mut bytes = Vec::new();
    let _ = f.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .next_back()
        .unwrap_or("")
        .to_string()
}

#[derive(Debug)]
pub struct PackageOutput {
    pub game_dir: PathBuf,
    pub files: u64,
    pub bytes: u64,
}

// Inspect the fresh tree before discovery follows any directories. Windows
// junctions are reparse points even when file_type().is_symlink() is false.
fn validate_tree(root: &Path, cancel: &AtomicBool) -> Result<(u64, u64)> {
    let mut files = 0;
    let mut bytes = 0;
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Err(error(
                "Package extraction cancelled; partial output was retained.",
            ));
        }
        for entry in std::fs::read_dir(&dir).map_err(|e| io(&dir, e))? {
            let p = entry.map_err(|e| io(&dir, e))?.path();
            let meta = std::fs::symlink_metadata(&p).map_err(|e| io(&p, e))?;
            #[cfg(windows)]
            let linked = {
                use std::os::windows::fs::MetadataExt;
                meta.file_attributes() & 0x400 != 0
            };
            #[cfg(not(windows))]
            let linked = meta.file_type().is_symlink();
            if linked {
                return Err(error("Package output contains a link or reparse point; it is not accepted as a game copy."));
            }
            if meta.is_dir() {
                dirs.push(p);
            } else {
                files += 1;
                bytes += meta.len();
            }
        }
    }
    Ok((files, bytes))
}

/// EB directory parsing alone does not check payload extents against file size.
/// A helper that stopped after writing its tables must not look like a good copy.
fn check_archive_bounds(path: &Path) -> Result<()> {
    use tdb_archive_big::eb;
    let mut f = std::fs::File::open(path).map_err(|e| io(path, e))?;
    let length = f.metadata().map_err(|e| io(path, e))?.len();
    let mut magic = [0; 4];
    f.read_exact(&mut magic).map_err(|e| io(path, e))?;
    if &magic != eb::MAGIC_EB {
        return Ok(());
    } // Other formats are checked by preflight.
    let mut head = vec![0; eb::HEADER_LEN];
    f.rewind().map_err(|e| io(path, e))?;
    f.read_exact(&mut head).map_err(|e| io(path, e))?;
    let header = eb::EbHeader::parse(&head).map_err(|e| error(e.to_string()))?;
    let end = header.table_end();
    if end as u64 > length || end > 256 * 1024 * 1024 {
        return Err(error(format!(
            "Invalid or truncated package archive table: {}",
            path.display()
        )));
    }
    let mut table = vec![0; end];
    f.rewind().map_err(|e| io(path, e))?;
    f.read_exact(&mut table).map_err(|e| io(path, e))?;
    let archive = eb::EbArchive::parse(&table).map_err(|e| error(e.to_string()))?;
    if archive.entries.iter().any(|e| {
        e.offset
            .checked_add(e.size as u64)
            .is_none_or(|end| end > length)
    }) {
        return Err(error(format!(
            "Truncated package archive payload: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Prepare a base game only. Patches/DLC require a separate, explicit merge policy.
/// Success means validated package output is ready for archive extraction, not gameplay.
pub fn extract(
    image: &Path,
    destination: &Path,
    extractor: &Path,
    game_id: &str,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(&str),
) -> Result<PackageOutput> {
    if cancel.load(Ordering::Relaxed) {
        return Err(error("Package extraction cancelled."));
    }
    if destination.exists() {
        return Err(error(
            "Package extraction requires a new folder; existing files are never replaced.",
        ));
    }
    if !destination.parent().is_some_and(Path::is_dir) {
        return Err(error(
            "Choose a new package output folder with an existing parent.",
        ));
    }
    let image = std::fs::canonicalize(image).map_err(|e| io(image, e))?;
    let mut input = std::fs::File::open(&image).map_err(|e| io(&image, e))?;
    let mut magic = [0; 4];
    input.read_exact(&mut magic).map_err(|e| io(&image, e))?;
    if &magic != b"\x7fCNT" {
        return Err(error("This file is not a PS4 package."));
    }
    let length = input.metadata().map_err(|e| io(&image, e))?.len();
    if crate::disk::free_space(destination).is_some_and(|free| free < length) {
        return Err(error("Not enough free space for this package; asset extraction will require additional space."));
    }
    let extractor=std::fs::canonicalize(extractor).map_err(|_|error("The standalone PS4 package extractor was not found. Select pkg_extractor.exe, or install the release's tools/pkg-extractor folder."))?;
    progress("Checking whether this is a base-game package…");
    let mut probe = command(&extractor)
        .arg(&image)
        .arg("--check-type")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| io(&extractor, e))?;
    let kind = wait(
        &mut probe,
        cancel,
        Some(Duration::from_secs(15)),
        &mut || {},
    )?;
    if kind.code() != Some(101) {
        return Err(error(format!("Only base-game packages are supported here (extractor type result {:?}). Select a base package; updates and DLC are not merged automatically.",kind.code())));
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(error("Package extraction cancelled."));
    }
    // An exclusive create prevents two requests from sharing an output tree.
    std::fs::create_dir(destination).map_err(|e| io(destination, e))?;
    let root = std::fs::canonicalize(destination).map_err(|e| io(destination, e))?;
    let log_path = root.join("nhlms-package-extraction.log");
    let log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&log_path)
        .map_err(|e| io(&log_path, e))?;
    let stderr = log.try_clone().map_err(|e| io(&log_path, e))?;
    let mut child = command(&extractor)
        .arg(&image)
        .arg(&root)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|e| io(&extractor, e))?;
    let mut last = Instant::now() - Duration::from_secs(1);
    let status = wait(&mut child, cancel, None, &mut || {
        if last.elapsed() >= Duration::from_secs(1) {
            progress(&log_tail(&log_path));
            last = Instant::now();
        }
    })?;
    if !status.success() {
        return Err(error(format!(
            "Package extractor exited with {status}. See {}. Partial output was retained.",
            log_path.display()
        )));
    }
    // This CLI can return zero after a failure. Require a matching game identity,
    // an executable and readable nonempty game-container manifests independently.
    validate_tree(&root, cancel)?;
    let report = crate::discovery::detect_install(game_id, Platform::Ps4, &root, None);
    if !report.game_candidates.iter().any(|c| c.matches) {
        return Err(error("Extracted package does not identify as the selected game. Output was retained for inspection."));
    }
    let game = PathBuf::from(
        report
            .game_dir
            .ok_or_else(|| error("Package output has no recognised game folder."))?,
    );
    if !game.join("eboot.bin").is_file() {
        return Err(error(
            "Package output is missing eboot.bin; it is not ready to import.",
        ));
    }
    for archive in crate::extract::find_big_archives(&game) {
        if cancel.load(Ordering::Relaxed) {
            return Err(error(
                "Package extraction cancelled; partial output was retained.",
            ));
        }
        check_archive_bounds(&archive)?;
    }
    let pre = crate::extract_service::preflight(&game, Platform::Ps4);
    if pre.archives.is_empty()
        || pre.members_total == 0
        || pre.archives.iter().any(|a| a.parse_error.is_some())
    {
        return Err(error("Package output has missing or unreadable game archives. Inspect the extraction log; exit status alone is not proof of success."));
    }
    let (files, bytes) = validate_tree(&game, cancel)?;
    Ok(PackageOutput {
        game_dir: game,
        files,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "subprocess fixture for cancellation; not a package test"]
    fn child_wait_fixture() {
        std::thread::sleep(Duration::from_secs(10));
    }

    #[test]
    fn cancellation_and_timeout_reap_the_owned_helper() {
        for cancelled in [true, false] {
            let exe = std::env::current_exe().unwrap();
            let mut child = command(&exe)
                .args([
                    "--ignored",
                    "--exact",
                    "pkg_extract::tests::child_wait_fixture",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let result = wait(
                &mut child,
                &AtomicBool::new(cancelled),
                Some(Duration::from_millis(150)),
                &mut || {},
            );
            assert!(result.is_err());
            assert!(
                child.try_wait().unwrap().is_some(),
                "helper must be reaped before returning"
            );
        }
    }
    #[test]
    fn refusal_does_not_create_or_overwrite_output() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("nhlms-pkg-test-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let image = dir.join("game.pkg");
        let out = dir.join("new");
        let tool = dir.join("missing.exe");
        std::fs::write(&image, b"not a package").unwrap();
        let run = |cancel| {
            extract(
                &image,
                &out,
                &tool,
                "nhl21",
                &AtomicBool::new(cancel),
                &mut |_| {},
            )
        };
        assert!(run(false).unwrap_err().to_string().contains("not a PS4"));
        assert!(!out.exists());
        std::fs::write(&image, b"\x7fCNTfixture").unwrap();
        assert!(run(false).unwrap_err().to_string().contains("not found"));
        assert!(!out.exists());
        assert!(run(true).is_err());
        assert!(!out.exists());
        std::fs::create_dir(&out).unwrap();
        std::fs::write(out.join("keep"), b"original").unwrap();
        assert_eq!(
            validate_tree(&out, &AtomicBool::new(false)).unwrap(),
            (1, 8)
        );
        assert!(validate_tree(&out, &AtomicBool::new(true))
            .unwrap_err()
            .to_string()
            .contains("cancelled"));
        assert!(run(false).unwrap_err().to_string().contains("new folder"));
        assert_eq!(std::fs::read(out.join("keep")).unwrap(), b"original");
        // A complete EB table followed by a missing payload must be refused.
        let archive = dir.join("test.big");
        let mut eb = vec![0; 0x64];
        eb[..4].copy_from_slice(b"EB\0\x03");
        eb[4..8].copy_from_slice(&1u32.to_be_bytes());
        eb[8..12].copy_from_slice(&0x00110400u32.to_be_bytes());
        eb[12..16].copy_from_slice(&0x44u32.to_be_bytes());
        eb[0x14] = 8;
        eb[0x15] = 2;
        eb[0x16..0x18].copy_from_slice(&1u16.to_be_bytes());
        eb[0x18..0x20].copy_from_slice(&0x64u64.to_be_bytes());
        eb[0x30..0x34].copy_from_slice(&6u32.to_be_bytes());
        eb[0x38..0x3c].copy_from_slice(&4u32.to_be_bytes());
        eb[0x46..0x4a].copy_from_slice(b"a.db");
        eb[0x50] = b'd';
        eb[0x60..].copy_from_slice(b"DATA");
        std::fs::write(&archive, &eb).unwrap();
        check_archive_bounds(&archive).unwrap();
        std::fs::write(&archive, &eb[..0x60]).unwrap();
        assert!(check_archive_bounds(&archive)
            .unwrap_err()
            .to_string()
            .contains("Truncated package archive payload"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
