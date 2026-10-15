//! Explicit private-fixture resolution for tests (C0-FX1).
//!
//! Several `tdb-rx2` cases can only assert anything against a real EA
//! asset. Those assets are private game data: they are **not** in this
//! repository, they are never committed, and nothing here scans a
//! mounted game tree or any other application's repository looking for
//! them.
//!
//! The contract is therefore explicit on both sides:
//!
//! * every fixture-dependent case is `#[ignore]`d, so the default suite
//!   never reports success for coverage it did not exercise;
//! * when such a case *is* selected, [`require`] either returns the
//!   fixture bytes or panics with a diagnostic naming the environment
//!   variable, the fixture key and the path it resolved to. There is no
//!   early return and no silent skip.
//!
//! Point [`FIXTURE_ENV`] at a directory laid out like this:
//!
//! ```text
//! <root>/
//!   equipment-preview/
//!     feprop_glove_0.rx2
//!     helmet_14_2.rx2
//!   legacy/
//!     centerlogo_0_cm.rx2
//!     playerheadssmall_p1512.xpr2
//!     texlib_67.rx2
//!   mesh/
//!     player_20500.rx2
//! ```
//!
//! ```text
//! NHL_STUDIO_FIXTURES=<root> cargo test -p tdb-rx2 -- --ignored
//! ```
//!
//! See `docs/validation/c0-fx1-explicit-fixture-tests.md`.

use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Environment variable naming the private-fixture root directory.
pub(crate) const FIXTURE_ENV: &str = "NHL_STUDIO_FIXTURES";

/// NHL Legacy `rendering/player/texlib_67.rx2` — hair 256×256 DXT3 plus
/// head 512×512 DXT1 in one texlib.
pub(crate) const LEGACY_TEXLIB_67: &str = "legacy/texlib_67.rx2";

/// NHL Legacy `rendering/icesurface/centerlogo_0_cm.rx2` — 1024×1024
/// DXT3, the third-resolution descriptor reference.
pub(crate) const LEGACY_CENTERLOGO: &str = "legacy/centerlogo_0_cm.rx2";

/// A decompressed `playerheadssmall/p1512.big` entry — 256×128 DXT5
/// XPR2 front-end portrait.
pub(crate) const LEGACY_PORTRAIT_XPR2: &str = "legacy/playerheadssmall_p1512.xpr2";

/// The five-submesh player head template `player_20500.rx2`.
pub(crate) const TEMPLATE_PLAYER_MESH: &str = "mesh/player_20500.rx2";

/// Equipment-preview glove mesh.
pub(crate) const EQUIPMENT_GLOVE: &str = "equipment-preview/feprop_glove_0.rx2";

/// Equipment-preview helmet mesh.
pub(crate) const EQUIPMENT_HELMET: &str = "equipment-preview/helmet_14_2.rx2";

/// Every reason a fixture lookup can fail. Each variant carries enough
/// context to act on without reading this file.
#[derive(Debug)]
pub(crate) enum FixtureError {
    /// `FIXTURE_ENV` is not set in the environment.
    RootUnset { key: String },
    /// `FIXTURE_ENV` is set to an empty value.
    RootEmpty { key: String },
    /// The configured root does not exist.
    RootMissing { root: PathBuf, key: String },
    /// The configured root exists but is not a directory.
    RootNotADirectory { root: PathBuf, key: String },
    /// The fixture key itself is malformed (a defect in the test).
    BadKey { key: String, why: &'static str },
    /// The root resolved, but the fixture is not there.
    Missing {
        root: PathBuf,
        key: String,
        path: PathBuf,
    },
    /// Something exists at the fixture path but it is not a file.
    NotAFile { key: String, path: PathBuf },
    /// The fixture exists but could not be read.
    Unreadable {
        key: String,
        path: PathBuf,
        source: io::Error,
    },
    /// The fixture read back as zero bytes, so it cannot be the asset.
    Empty { key: String, path: PathBuf },
}

/// The configuration paragraph appended to every root-level failure, so
/// the diagnostic is actionable without opening the validation record.
fn configuration_help(key: &str) -> String {
    format!(
        "\n  configure it with:\n    {FIXTURE_ENV}=<root> cargo test -p tdb-rx2 -- --ignored\
         \n  and place the fixture at:\n    <root>/{key}\
         \n  the full layout and the per-test mapping are in\
         \n    docs/validation/c0-fx1-explicit-fixture-tests.md\
         \n  fixtures are private game data: they are never committed, and nothing\
         \n  here searches a game tree or another repository for them."
    )
}

impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootUnset { key } => write!(
                f,
                "fixture `{key}` is required but {FIXTURE_ENV} is not set.{}",
                configuration_help(key)
            ),
            Self::RootEmpty { key } => write!(
                f,
                "fixture `{key}` is required but {FIXTURE_ENV} is set to an empty value.{}",
                configuration_help(key)
            ),
            Self::RootMissing { root, key } => write!(
                f,
                "fixture `{key}` is required but the {FIXTURE_ENV} root does not exist: {}{}",
                root.display(),
                configuration_help(key)
            ),
            Self::RootNotADirectory { root, key } => write!(
                f,
                "fixture `{key}` is required but the {FIXTURE_ENV} root is not a directory: {}{}",
                root.display(),
                configuration_help(key)
            ),
            Self::BadKey { key, why } => write!(
                f,
                "fixture key `{key}` is malformed ({why}) — this is a defect in the test, \
                 not in the fixture configuration."
            ),
            Self::Missing { root, key, path } => write!(
                f,
                "fixture `{key}` is missing.\n  {FIXTURE_ENV} root : {}\n  expected file  : {}{}",
                root.display(),
                path.display(),
                configuration_help(key)
            ),
            Self::NotAFile { key, path } => write!(
                f,
                "fixture `{key}` is not a regular file: {}",
                path.display()
            ),
            Self::Unreadable { key, path, source } => write!(
                f,
                "fixture `{key}` could not be read: {} ({source})",
                path.display()
            ),
            Self::Empty { key, path } => write!(
                f,
                "fixture `{key}` is invalid: {} is 0 bytes",
                path.display()
            ),
        }
    }
}

/// Reject anything that is not a plain relative `a/b/c.ext` key. Keys
/// are crate-internal constants, so a failure here is a coding defect.
fn check_key(key: &str) -> Result<(), FixtureError> {
    let bad = |why| {
        Err(FixtureError::BadKey {
            key: key.to_string(),
            why,
        })
    };
    if key.is_empty() {
        return bad("empty");
    }
    if key.contains('\\') {
        return bad("uses a backslash; fixture keys always use `/`");
    }
    if key.starts_with('/') || Path::new(key).is_absolute() {
        return bad("absolute");
    }
    for segment in key.split('/') {
        match segment {
            "" => return bad("contains an empty path segment"),
            "." | ".." => return bad("contains a relative path segment"),
            _ => {}
        }
    }
    Ok(())
}

/// Resolve `key` against an explicitly supplied root. `root` is the raw
/// environment value; `None` means unset. Nothing is read from the
/// process environment here, so this stays testable and race-free.
pub(crate) fn resolve(root: Option<&OsStr>, key: &str) -> Result<PathBuf, FixtureError> {
    check_key(key)?;
    let Some(root) = root else {
        return Err(FixtureError::RootUnset {
            key: key.to_string(),
        });
    };
    if root.is_empty() {
        return Err(FixtureError::RootEmpty {
            key: key.to_string(),
        });
    }
    let root = PathBuf::from(root);
    if !root.exists() {
        return Err(FixtureError::RootMissing {
            root,
            key: key.to_string(),
        });
    }
    if !root.is_dir() {
        return Err(FixtureError::RootNotADirectory {
            root,
            key: key.to_string(),
        });
    }
    let mut path = root;
    for segment in key.split('/') {
        path.push(segment);
    }
    Ok(path)
}

/// Resolve and read `key` against an explicitly supplied root.
pub(crate) fn load_from(root: Option<&OsStr>, key: &str) -> Result<Vec<u8>, FixtureError> {
    let path = resolve(root, key)?;
    if !path.exists() {
        // `resolve` joined one component per key segment, so popping the
        // same number recovers the root for a whole diagnostic.
        let mut root_path = path.clone();
        for _ in key.split('/') {
            root_path.pop();
        }
        return Err(FixtureError::Missing {
            root: root_path,
            key: key.to_string(),
            path,
        });
    }
    if !path.is_file() {
        return Err(FixtureError::NotAFile {
            key: key.to_string(),
            path,
        });
    }
    let bytes = std::fs::read(&path).map_err(|source| FixtureError::Unreadable {
        key: key.to_string(),
        path: path.clone(),
        source,
    })?;
    if bytes.is_empty() {
        return Err(FixtureError::Empty {
            key: key.to_string(),
            path,
        });
    }
    Ok(bytes)
}

/// Read a required fixture, or fail the selected test with a diagnostic.
///
/// This never returns `None` and never returns early: an absent,
/// unreadable or invalid fixture is a test failure, because the case was
/// explicitly selected on the promise that the fixture is present.
#[track_caller]
pub(crate) fn require(key: &str) -> Vec<u8> {
    match load_from(std::env::var_os(FIXTURE_ENV).as_deref(), key) {
        Ok(bytes) => bytes,
        Err(e) => panic!("{e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A synthetic directory under the OS temp dir, removed on drop. No
    /// dev-dependency is used, so the workspace lockfile is untouched.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            static SEQ: AtomicU64 = AtomicU64::new(0);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before the epoch")
                .as_nanos();
            let name = format!(
                "tdb-rx2-fixtures-{tag}-{}-{}-{nanos}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::Relaxed)
            );
            let path = std::env::temp_dir().join(name);
            std::fs::create_dir_all(&path).expect("create scratch dir");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn as_root(&self) -> OsString {
            self.0.clone().into_os_string()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write(root: &Path, key: &str, bytes: &[u8]) -> PathBuf {
        let mut p = root.to_path_buf();
        for segment in key.split('/') {
            p.push(segment);
        }
        std::fs::create_dir_all(p.parent().expect("key has a parent")).expect("create parents");
        std::fs::write(&p, bytes).expect("write fixture");
        p
    }

    #[test]
    fn unset_root_is_an_error_that_names_the_variable_and_the_key() {
        let e = resolve(None, TEMPLATE_PLAYER_MESH).expect_err("must fail");
        assert!(matches!(e, FixtureError::RootUnset { .. }), "{e:?}");
        let msg = e.to_string();
        assert!(msg.contains(FIXTURE_ENV), "{msg}");
        assert!(msg.contains(TEMPLATE_PLAYER_MESH), "{msg}");
        assert!(
            msg.contains("c0-fx1-explicit-fixture-tests.md"),
            "the diagnostic must point at the record: {msg}"
        );
    }

    #[test]
    fn empty_root_is_rejected_rather_than_treated_as_the_cwd() {
        let e = resolve(Some(OsStr::new("")), TEMPLATE_PLAYER_MESH).expect_err("must fail");
        assert!(matches!(e, FixtureError::RootEmpty { .. }), "{e:?}");
    }

    #[test]
    fn missing_root_is_reported_with_the_path() {
        let scratch = Scratch::new("missing-root");
        let absent = scratch.path().join("nope");
        let e = resolve(Some(absent.as_os_str()), TEMPLATE_PLAYER_MESH).expect_err("must fail");
        assert!(matches!(e, FixtureError::RootMissing { .. }), "{e:?}");
        assert!(e.to_string().contains("nope"), "{e}");
    }

    #[test]
    fn a_file_used_as_the_root_is_rejected() {
        let scratch = Scratch::new("file-root");
        let file = scratch.path().join("root.txt");
        std::fs::write(&file, b"x").expect("write");
        let e = resolve(Some(file.as_os_str()), TEMPLATE_PLAYER_MESH).expect_err("must fail");
        assert!(matches!(e, FixtureError::RootNotADirectory { .. }), "{e:?}");
    }

    #[test]
    fn a_nested_key_resolves_under_the_root() {
        let scratch = Scratch::new("resolve");
        let got = resolve(Some(&scratch.as_root()), TEMPLATE_PLAYER_MESH).expect("resolve");
        assert_eq!(got, scratch.path().join("mesh").join("player_20500.rx2"));
    }

    #[test]
    fn malformed_keys_are_rejected_before_the_root_is_examined() {
        let scratch = Scratch::new("bad-keys");
        for key in ["", "/abs/x.rx2", "mesh//x.rx2", "../x.rx2", "mesh/../x.rx2"] {
            let e = resolve(Some(&scratch.as_root()), key)
                .expect_err(&format!("`{key}` must be rejected"));
            assert!(matches!(e, FixtureError::BadKey { .. }), "`{key}` -> {e:?}");
        }
    }

    #[test]
    fn a_missing_fixture_names_the_root_and_the_expected_file() {
        let scratch = Scratch::new("missing-file");
        let e = load_from(Some(&scratch.as_root()), TEMPLATE_PLAYER_MESH).expect_err("must fail");
        assert!(matches!(e, FixtureError::Missing { .. }), "{e:?}");
        let msg = e.to_string();
        assert!(msg.contains(&scratch.path().display().to_string()), "{msg}");
        assert!(msg.contains("player_20500.rx2"), "{msg}");
    }

    #[test]
    fn a_directory_at_the_fixture_path_is_not_accepted_as_the_fixture() {
        let scratch = Scratch::new("dir-fixture");
        let mut p = scratch.path().to_path_buf();
        for segment in TEMPLATE_PLAYER_MESH.split('/') {
            p.push(segment);
        }
        std::fs::create_dir_all(&p).expect("create dir");
        let e = load_from(Some(&scratch.as_root()), TEMPLATE_PLAYER_MESH).expect_err("must fail");
        assert!(matches!(e, FixtureError::NotAFile { .. }), "{e:?}");
    }

    #[test]
    fn a_zero_byte_fixture_is_invalid_rather_than_an_empty_success() {
        let scratch = Scratch::new("empty-fixture");
        write(scratch.path(), TEMPLATE_PLAYER_MESH, b"");
        let e = load_from(Some(&scratch.as_root()), TEMPLATE_PLAYER_MESH).expect_err("must fail");
        assert!(matches!(e, FixtureError::Empty { .. }), "{e:?}");
        assert!(e.to_string().contains("0 bytes"), "{e}");
    }

    #[test]
    fn a_present_fixture_loads_its_exact_bytes() {
        let scratch = Scratch::new("present");
        write(scratch.path(), LEGACY_TEXLIB_67, b"\x89RW4xb2 synthetic");
        let got = load_from(Some(&scratch.as_root()), LEGACY_TEXLIB_67).expect("load");
        assert_eq!(got, b"\x89RW4xb2 synthetic");
    }

    #[test]
    fn one_configured_root_serves_every_declared_key() {
        let scratch = Scratch::new("layout");
        let keys = [
            LEGACY_TEXLIB_67,
            LEGACY_CENTERLOGO,
            LEGACY_PORTRAIT_XPR2,
            TEMPLATE_PLAYER_MESH,
            EQUIPMENT_GLOVE,
            EQUIPMENT_HELMET,
        ];
        for (i, key) in keys.iter().enumerate() {
            check_key(key).unwrap_or_else(|e| panic!("{e}"));
            write(scratch.path(), key, &[i as u8 + 1]);
        }
        for (i, key) in keys.iter().enumerate() {
            let got = load_from(Some(&scratch.as_root()), key).expect("load");
            assert_eq!(got, vec![i as u8 + 1], "{key}");
        }
    }
}
