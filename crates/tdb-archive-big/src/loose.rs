//! Loose-file lookup beside an archive member (package C0-AS7, review finding P2).
//!
//! The `as7_eb_members` evidence tool compares an archive member with whatever sits at the same
//! relative path under a loose root. That comparison is only evidence if **absence** means the
//! file is not there: a permission error, a path that is a directory, or an I/O fault must never
//! be recorded as "no loose file". This module draws that line once:
//!
//! * [`validate_loose_root`] — the root must exist, be a directory, and be listable, before any
//!   member is looked up;
//! * [`read_loose_with`] / [`read_loose`] — `NotFound` is the **only** error that reads as absent;
//!   every other error is returned with the path so the caller can fail the run before writing a
//!   record.
//!
//! The reader is injectable so the error paths are tested without a private installation.

use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Why a loose-file lookup could not produce evidence. Both variants carry the path.
#[derive(Debug, Error)]
pub enum LooseError {
    #[error("loose root {path}: {reason}")]
    InvalidRoot { path: PathBuf, reason: String },
    #[error("loose file {path}: read failed ({kind:?}): {source}")]
    Read {
        path: PathBuf,
        kind: io::ErrorKind,
        #[source]
        source: io::Error,
    },
}

/// The root must exist, be a directory, and be readable (listable). Checked once before a scan,
/// so a mistyped or unmounted root fails the run instead of turning every member into "absent".
pub fn validate_loose_root(root: &Path) -> Result<(), LooseError> {
    let invalid = |reason: String| LooseError::InvalidRoot {
        path: root.to_path_buf(),
        reason,
    };
    let meta = std::fs::metadata(root).map_err(|e| invalid(format!("cannot stat: {e}")))?;
    if !meta.is_dir() {
        return Err(invalid("not a directory".into()));
    }
    std::fs::read_dir(root).map_err(|e| invalid(format!("cannot list: {e}")))?;
    Ok(())
}

/// Read one loose file through `read`. `Ok(None)` **only** for [`io::ErrorKind::NotFound`];
/// any other failure is an error carrying the path.
pub fn read_loose_with<F>(path: &Path, read: F) -> Result<Option<Vec<u8>>, LooseError>
where
    F: FnOnce(&Path) -> io::Result<Vec<u8>>,
{
    match read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(LooseError::Read {
            path: path.to_path_buf(),
            kind: e.kind(),
            source: e,
        }),
    }
}

/// [`read_loose_with`] over `std::fs::read`.
pub fn read_loose(path: &Path) -> Result<Option<Vec<u8>>, LooseError> {
    read_loose_with(path, |p| std::fs::read(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, unique scratch directory under the system temp dir, removed on drop.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "tdb-archive-big-loose-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_readable_file_is_present_with_its_bytes() {
        let s = Scratch::new("readable");
        let p = s.0.join("rendering").join("x.rpsgl");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, b"chunkzip....").unwrap();
        assert_eq!(read_loose(&p).unwrap(), Some(b"chunkzip....".to_vec()));
    }

    #[test]
    fn a_missing_file_is_absent_and_nothing_else_is() {
        let s = Scratch::new("missing");
        let p = s.0.join("fe").join("nope.big");
        assert_eq!(read_loose(&p).unwrap(), None);
        // The injected path: only NotFound reads as absent.
        let r = read_loose_with(Path::new("a/b"), |_| {
            Err(io::Error::new(io::ErrorKind::NotFound, "gone"))
        });
        assert_eq!(r.unwrap(), None);
    }

    #[test]
    fn a_non_not_found_read_failure_is_an_error_carrying_the_path() {
        for kind in [
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::Other,
            io::ErrorKind::InvalidData,
        ] {
            let r = read_loose_with(Path::new("rendering/logo/texlib_10_0_0.rpsgl"), |_| {
                Err(io::Error::new(kind, "injected"))
            });
            match r {
                Err(LooseError::Read {
                    path, kind: got, ..
                }) => {
                    assert_eq!(path, PathBuf::from("rendering/logo/texlib_10_0_0.rpsgl"));
                    assert_eq!(got, kind);
                }
                other => panic!("{kind:?}: expected a read error, got {other:?}"),
            }
        }
        // A real one, not injected: a path whose parent is a FILE cannot be a file, and on
        // every platform that is not NotFound-shaped in the sense that matters here — it must
        // not become absence unless the OS reports NotFound.
        let s = Scratch::new("parent-is-file");
        let file = s.0.join("f");
        std::fs::write(&file, b"x").unwrap();
        let below = file.join("child");
        match read_loose(&below) {
            Ok(None) => {
                // Some platforms report NotFound here; that is the documented meaning of absent.
                assert_eq!(
                    std::fs::read(&below).unwrap_err().kind(),
                    io::ErrorKind::NotFound
                );
            }
            Ok(Some(_)) => panic!("a path below a file cannot be present"),
            Err(LooseError::Read { path, kind, .. }) => {
                assert_eq!(path, below);
                assert_ne!(kind, io::ErrorKind::NotFound);
            }
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn the_loose_root_must_exist_and_be_a_readable_directory() {
        let s = Scratch::new("root");
        assert!(validate_loose_root(&s.0).is_ok());
        let missing = s.0.join("absent");
        match validate_loose_root(&missing) {
            Err(LooseError::InvalidRoot { path, reason }) => {
                assert_eq!(path, missing);
                assert!(reason.starts_with("cannot stat"), "{reason}");
            }
            other => panic!("{other:?}"),
        }
        let file = s.0.join("file.big");
        std::fs::write(&file, b"EB").unwrap();
        match validate_loose_root(&file) {
            Err(LooseError::InvalidRoot { path, reason }) => {
                assert_eq!(path, file);
                assert_eq!(reason, "not a directory");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn errors_name_the_path_in_their_message() {
        let e = read_loose_with(Path::new("x/y"), |_| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
        })
        .unwrap_err();
        let msg = e.to_string();
        assert!(
            msg.contains("x/y") && msg.contains("PermissionDenied"),
            "{msg}"
        );
    }
}
