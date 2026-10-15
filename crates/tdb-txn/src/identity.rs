//! File identity: byte length + SHA-256, computed by streaming.

use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Streaming buffer for hashing and copying.
pub(crate) const CHUNK: usize = 1 << 20;

/// What a file **is**: its length and whole-file SHA-256 (lower-case hex).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileIdentity {
    pub len: u64,
    pub sha256: String,
}

impl fmt::Display for FileIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bytes / {}", self.len, self.sha256)
    }
}

impl FileIdentity {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self {
            len: bytes.len() as u64,
            sha256: hex(&Sha256::digest(bytes)),
        }
    }

    /// Hash a file by streaming it.
    pub fn of_file(path: &Path) -> io::Result<Self> {
        let mut f = File::open(path)?;
        let mut h = Sha256::new();
        let mut buf = vec![0u8; CHUNK];
        let mut len = 0u64;
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
            len += n as u64;
        }
        Ok(Self {
            len,
            sha256: hex(&h.finalize()),
        })
    }

    /// `Some(identity)` for a present file, `None` for `NotFound`; any other
    /// error (permission, a directory in the way) is reported.
    pub fn of_path_or_absent(path: &Path) -> io::Result<Option<Self>> {
        match std::fs::metadata(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
            Ok(m) if !m.is_file() => Err(io::Error::other(format!(
                "{} exists but is not a regular file",
                path.display()
            ))),
            Ok(_) => Self::of_file(path).map(Some),
        }
    }
}

pub(crate) fn hex(d: &[u8]) -> String {
    let mut s = String::with_capacity(d.len() * 2);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Copy `from` to a **new** file `to`, preallocating `to` to `expected.len`
/// first (so insufficient space fails here, before any bytes move), hashing
/// while copying, fsyncing the result, and refusing a source that is not
/// `expected`. `to` must not exist.
pub(crate) fn copy_new_verified(from: &Path, to: &Path, expected: &FileIdentity) -> io::Result<()> {
    let mut src = File::open(from)?;
    let mut dst = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)?;
    // Space accounting: reserve the final length up front; the copy then
    // writes over the reserved bytes from offset 0.
    dst.set_len(expected.len)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    let mut len = 0u64;
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        dst.write_all(&buf[..n])?;
        len += n as u64;
    }
    dst.sync_all()?;
    let got = FileIdentity {
        len,
        sha256: hex(&h.finalize()),
    };
    if got != *expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} is {got}, expected {expected} while copying to {}",
                from.display(),
                to.display()
            ),
        ));
    }
    Ok(())
}

/// Open a file and `sync_all` it (used on a destination after a rename).
/// Opened for writing: on Windows `FlushFileBuffers` requires a handle with
/// write access, and a read-only handle fails with access denied.
pub(crate) fn fsync_path(path: &Path) -> io::Result<()> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?
        .sync_all()
}
