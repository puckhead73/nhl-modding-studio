//! Adapter for the user's extract-xiso 2.7.1 engine. No GUI, shell, rewrite or delete mode.
use super::*;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const ENGINE: &[u8] = include_bytes!("../vendor/extract-xiso/exiso.exe");

struct Helper(PathBuf);
impl Drop for Helper {
    fn drop(&mut self) {
        // Only files created by this adapter, never the user's ISO or extraction output.
        let _ = std::fs::remove_file(self.0.join("exiso.exe"));
        let _ = std::fs::remove_dir(&self.0);
    }
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) fn extract(
    image: &Path,
    destination: &Path,
    members: &[crate::disc_iso9660::Member],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<DiscExtraction> {
    // Match this engine's -s implementation: any path containing this exact string is skipped.
    let files: Vec<_> = members
        .iter()
        .filter(|m| !m.directory && !m.path.contains("$SystemUpdate"))
        .collect();
    let total: u64 = files.iter().map(|m| m.size).sum();
    if crate::disk::free_space(destination).is_some_and(|free| free < total) {
        return Err(error(format!(
            "not enough free space to extract {total} disc bytes"
        )));
    }
    let image = std::fs::canonicalize(image).map_err(|e| io(image, e))?;
    let mut source = File::open(&image).map_err(|e| io(&image, e))?;
    let before = source.metadata().map_err(|e| io(&image, e))?;
    if cancel.load(Ordering::Relaxed) {
        return Err(error("disc extraction cancelled"));
    }

    let helper_dir = std::env::temp_dir().join(format!(
        "nhlms-exiso-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir(&helper_dir).map_err(|e| io(&helper_dir, e))?;
    let helper = Helper(helper_dir);
    let executable = helper.0.join("exiso.exe");
    std::fs::write(&executable, ENGINE).map_err(|e| io(&executable, e))?;

    // Claim a new output before launch: exiso itself permits an existing directory.
    std::fs::create_dir(destination).map_err(|e| {
        error(format!(
            "could not create new disc folder {}: {e}; choose a new folder with an existing parent",
            destination.display()
        ))
    })?;
    let root = std::fs::canonicalize(destination).map_err(|e| io(destination, e))?;
    let mut child = Process(
        Command::new(&executable)
            .args(["-q", "-s", "-x", "-d"])
            .arg(&root)
            .arg(&image)
            .current_dir(&helper.0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000) // CREATE_NO_WINDOW: Studio owns all interaction.
            .spawn()
            .map_err(|e| error(format!("could not launch the bundled Xbox extractor: {e}")))?,
    );
    let mut stderr = child.0.stderr.take().unwrap();
    // Drain all output to avoid pipe deadlock, retaining only a bounded diagnostic tail.
    let diagnostics = std::thread::spawn(move || {
        let mut kept = Vec::new();
        let mut buf = [0; 4096];
        while let Ok(n) = stderr.read(&mut buf) {
            if n == 0 {
                break;
            }
            kept.extend_from_slice(&buf[..n]);
            if kept.len() > 65536 {
                kept.drain(..kept.len() - 65536);
            }
        }
        String::from_utf8_lossy(&kept).into_owned()
    });
    progress(0, total);
    let mut last = Instant::now();
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.0.kill();
            let _ = child.0.wait();
            let _ = diagnostics.join();
            return Err(error(format!("disc extraction cancelled; incomplete output remains in {}; choose a new folder to retry", root.display())));
        }
        if let Some(status) = child
            .0
            .try_wait()
            .map_err(|e| error(format!("could not read extractor status: {e}")))?
        {
            break status;
        }
        if last.elapsed() >= Duration::from_secs(1) {
            let written = files
                .iter()
                .map(|m| {
                    std::fs::metadata(root.join(&m.path))
                        .map(|x| x.len().min(m.size))
                        .unwrap_or(0)
                })
                .sum();
            progress(written, total);
            last = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let diagnostic = diagnostics.join().unwrap_or_default();
    if !status.success() {
        return Err(error(format!(
            "Xbox extractor failed ({status}): {}. Incomplete output remains in {}",
            diagnostic.trim(),
            root.display()
        )));
    }

    // Exit status alone is not completion evidence. Compare every expected output against
    // its original disc bytes, including zero-length files, before reporting success.
    let mut expected = vec![0u8; 1024 * 1024];
    let mut actual = vec![0u8; expected.len()];
    for member in &files {
        if cancel.load(Ordering::Relaxed) {
            return Err(error("disc extraction cancelled during verification"));
        }
        let output = root.join(&member.path);
        let resolved = std::fs::canonicalize(&output).map_err(|e| io(&output, e))?;
        if !resolved.starts_with(&root) {
            return Err(error("extracted file resolves outside the output folder"));
        }
        let mut file = File::open(&resolved).map_err(|e| io(&resolved, e))?;
        if file.metadata().map_err(|e| io(&resolved, e))?.len() != member.size {
            return Err(error(format!("extracted size differs for {}", member.path)));
        }
        for &(offset, size) in &member.extents {
            source
                .seek(SeekFrom::Start(offset))
                .map_err(|e| io(&image, e))?;
            let mut left = size;
            while left > 0 {
                if cancel.load(Ordering::Relaxed) {
                    return Err(error("disc extraction cancelled during verification"));
                }
                let n = left.min(expected.len() as u64) as usize;
                source
                    .read_exact(&mut expected[..n])
                    .map_err(|e| io(&image, e))?;
                file.read_exact(&mut actual[..n])
                    .map_err(|e| io(&resolved, e))?;
                if expected[..n] != actual[..n] {
                    return Err(error(format!("extracted bytes differ for {}", member.path)));
                }
                left -= n as u64;
            }
        }
    }
    let after = std::fs::metadata(&image).map_err(|e| io(&image, e))?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(error(
            "source image changed during extraction; output is not verified",
        ));
    }
    progress(total, total);
    if cancel.load(Ordering::Relaxed) {
        return Err(error("disc extraction cancelled before completion"));
    }
    Ok(DiscExtraction {
        root,
        members: files.len(),
        bytes: total,
    })
}

use std::fs::File;
