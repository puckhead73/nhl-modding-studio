//! The external decoder helper runner (receipt-27 review decision: an optional, separately invoked
//! minimal FFmpeg helper for XMA2 previews). Studio never runs a shell and never searches `PATH`:
//! the helper is an explicit file path, invoked with a fixed, structured argument list; its input is
//! written on one thread while stdout and stderr are drained on two others (so neither pipe can
//! deadlock the child); output bytes, stderr bytes and wall time are bounded; a cancellation flag is
//! polled; and the child is killed and reaped on every error, timeout, cancellation or panic (the
//! [`Owned`] guard). On Windows the helper runs without a console window. Nothing here is network-
//! facing, and Studio works fully without a helper: [`locate`] returns [`HelperError::Missing`] and
//! callers refuse with its text.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The environment variable that names an XMA2 helper explicitly (an absolute file path).
pub const XMA2_HELPER_ENV: &str = "NHL_STUDIO_XMA2_HELPER";
/// The file name Studio looks for in a `helpers` folder beside its executable.
pub const XMA2_HELPER_FILE: &str = if cfg!(windows) { "nhlms-xma2-helper.exe" } else { "nhlms-xma2-helper" };

/// Resource limits for one helper run.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_stdout: usize,
    pub max_stderr: usize,
    pub timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_stdout: 256 << 20, max_stderr: 64 << 10, timeout: Duration::from_secs(60) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperError {
    /// No helper is configured or present; the message says how one is found.
    Missing(String),
    Spawn(String),
    Timeout(Duration),
    Cancelled,
    OutputTooLarge(usize),
    Failed { status: String, stderr: String },
    Io(String),
}

impl std::fmt::Display for HelperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HelperError::Missing(m) => write!(f, "{m}"),
            HelperError::Spawn(e) => write!(f, "the decoder helper could not start: {e}"),
            HelperError::Timeout(d) => write!(f, "the decoder helper did not finish within {} s and was stopped", d.as_secs()),
            HelperError::Cancelled => write!(f, "the decode was cancelled and the helper stopped"),
            HelperError::OutputTooLarge(n) => write!(f, "the decoder helper produced more than {n} bytes and was stopped"),
            HelperError::Failed { status, stderr } => write!(f, "the decoder helper failed ({status}): {stderr}"),
            HelperError::Io(e) => write!(f, "decoder helper I/O: {e}"),
        }
    }
}

impl std::error::Error for HelperError {}

/// Find the XMA2 helper: the file named by [`XMA2_HELPER_ENV`] (must exist), else
/// `helpers/<XMA2_HELPER_FILE>` beside `exe_dir`. Never searches `PATH`.
pub fn locate(exe_dir: Option<&Path>) -> Result<PathBuf, HelperError> {
    if let Some(p) = std::env::var_os(XMA2_HELPER_ENV) {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(HelperError::Missing(format!("{XMA2_HELPER_ENV} names {}, which is not a file", p.display())))
        };
    }
    if let Some(d) = exe_dir {
        let p = d.join("helpers").join(XMA2_HELPER_FILE);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(HelperError::Missing(format!(
        "no XMA2 decoder helper is installed (Studio looks for helpers\\{XMA2_HELPER_FILE} beside its executable, or the file named by {XMA2_HELPER_ENV}); EA-XMA entries stay listed and framed without a preview"
    )))
}

/// Kills and reaps the child unless it was already waited for.
struct Owned(Option<Child>);

impl Drop for Owned {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn drain(mut r: impl Read, cap: usize, over: &AtomicBool) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut buf = [0u8; 64 << 10];
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            return Ok(out);
        }
        if out.len() + n > cap {
            over.store(true, Ordering::SeqCst);
            // Keep draining nothing further: the caller kills the child.
            return Ok(out);
        }
        out.extend_from_slice(&buf[..n]);
    }
}

/// Run `helper args…` with `input` on stdin; returns stdout. Refuses by name on every failure.
pub fn run(helper: &Path, args: &[&str], input: &[u8], limits: Limits, cancel: Option<&AtomicBool>) -> Result<Vec<u8>, HelperError> {
    let mut cmd = Command::new(helper);
    cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|e| HelperError::Spawn(format!("{}: {e}", helper.display())))?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut owned = Owned(Some(child));
    let (out_over, err_over) = (AtomicBool::new(false), AtomicBool::new(false));
    std::thread::scope(|s| {
        let writer = s.spawn(move || {
            if let Some(mut w) = stdin {
                // A helper that exits early closes the pipe; that surfaces through its status.
                let _ = w.write_all(input);
            }
        });
        let out_t = s.spawn(|| stdout.map(|r| drain(r, limits.max_stdout, &out_over)).unwrap_or_else(|| Ok(Vec::new())));
        let err_t = s.spawn(|| stderr.map(|r| drain(r, limits.max_stderr, &err_over)).unwrap_or_else(|| Ok(Vec::new())));
        let start = Instant::now();
        let waited: Result<ExitStatus, HelperError> = loop {
            let c = owned.0.as_mut().expect("child owned");
            match c.try_wait() {
                Ok(Some(st)) => break Ok(st),
                Ok(None) => {}
                Err(e) => break Err(HelperError::Io(e.to_string())),
            }
            if out_over.load(Ordering::SeqCst) {
                break Err(HelperError::OutputTooLarge(limits.max_stdout));
            }
            if cancel.is_some_and(|f| f.load(Ordering::SeqCst)) {
                break Err(HelperError::Cancelled);
            }
            if start.elapsed() > limits.timeout {
                break Err(HelperError::Timeout(limits.timeout));
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        match &waited {
            // Reaped by `try_wait`: nothing left to kill.
            Ok(_) => owned.0 = None,
            // Stop the child now so its pipes close and the drain threads finish.
            Err(_) => drop(std::mem::replace(&mut owned, Owned(None))),
        }
        let _ = writer.join();
        let out = out_t.join().map_err(|_| HelperError::Io("stdout reader panicked".into()))?.map_err(|e| HelperError::Io(e.to_string()));
        let err = err_t.join().map_err(|_| HelperError::Io("stderr reader panicked".into()))?.map_err(|e| HelperError::Io(e.to_string()));
        let st = waited?;
        if out_over.load(Ordering::SeqCst) {
            return Err(HelperError::OutputTooLarge(limits.max_stdout));
        }
        let (out, err) = (out?, err?);
        if !st.success() {
            return Err(HelperError::Failed { status: st.to_string(), stderr: String::from_utf8_lossy(&err).trim().chars().take(400).collect() });
        }
        Ok(out)
    })
}

/// The fixed argument list for decoding a RIFF XMA2 file from stdin to 16-bit little-endian PCM on
/// stdout: no network, no file names, no filters.
pub const XMA2_ARGS: &[&str] = &["-nostdin", "-hide_banner", "-loglevel", "error", "-f", "wav", "-c:a", "xma2", "-i", "pipe:0", "-map", "0:a:0", "-f", "s16le", "-acodec", "pcm_s16le", "pipe:1"];

/// Decode one RIFF XMA2 file through the helper; returns interleaved 16-bit samples.
pub fn decode_xma2(helper: &Path, riff: &[u8], limits: Limits, cancel: Option<&AtomicBool>) -> Result<Vec<i16>, HelperError> {
    let pcm = run(helper, XMA2_ARGS, riff, limits, cancel)?;
    if pcm.len() % 2 != 0 {
        return Err(HelperError::Io(format!("odd PCM byte count {}", pcm.len())));
    }
    Ok(pcm.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect())
}
