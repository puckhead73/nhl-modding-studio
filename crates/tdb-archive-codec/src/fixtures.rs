//! Explicit private-fixture regressions for the `chunkzip` decoder (S1, Q-S1.2).
//!
//! The synthetic cases in `lib.rs` pin the framing rules; these cases pin the
//! decoder against **stored member bytes cut from real archives**, with each
//! member's identity, stored digest, expected decoded size/digest and the
//! provenance of that expectation recorded in [`FIXTURES`]. The bytes are
//! private game data: never committed, never searched for. The contract is the
//! C0-FX1 one (`docs/validation/c0-fx1-explicit-fixture-tests.md`):
//!
//! * every fixture case is `#[ignore]`d, so the default suite never reports
//!   coverage it did not exercise;
//! * a selected case either reads its fixture or **fails** with a diagnostic
//!   naming the variable, the key and the resolved path — no early return.
//!
//! ```text
//! NHL_STUDIO_FIXTURES=<root> cargo test -p tdb-archive-codec -- --ignored
//! ```
//!
//! Layout under the root (`<archive>/<basename>.chunkzip` holds the member's
//! stored bytes exactly as they sit in the archive):
//!
//! ```text
//! <root>/chunkzip/
//!   nhllegacy-ps3-blus31540/patch.big/{name_1_365_7_cm,blocker_0_86_1_tm,
//!                                      feblocker_1_86_1_tm,fetrapper_0_86_0_dm}.rpsgl.chunkzip
//!   nhl21-ps4-cusa18182/overlays_21.big/{guestinthebootharrived,guestintheboothteaser}.gfx.chunkzip
//! ```

use std::fmt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Environment variable naming the private-fixture root (shared with `tdb-rx2`).
pub(crate) const FIXTURE_ENV: &str = "NHL_STUDIO_FIXTURES";

/// One recorded member. `stored_sha256` identifies the fixture bytes; the
/// `decoded_*` pair is what the decoder must produce; `oracle` says where that
/// expectation comes from, so a mismatch can be attributed.
pub(crate) struct Fixture {
    pub key: &'static str,
    /// Title / platform / copy identity, as recorded by C0-POR1 and C0-AS7.
    pub copy: &'static str,
    pub archive: &'static str,
    pub archive_sha256: &'static str,
    pub member_index: usize,
    pub member_path: &'static str,
    pub stored_size: usize,
    pub stored_sha256: &'static str,
    pub decoded_size: usize,
    pub decoded_sha256: &'static str,
    pub oracle: &'static str,
}

const PS3: &str = "NHL Legacy PS3 BLUS31540 (PARAM.SFO TITLE_ID, APP_VER 01.01; C0-POR1 §1)";
const PS3_PATCH: &str = "USRDIR/patch.big";
const PS3_PATCH_SHA: &str = "d2d26273a742cc21667b56e483b089ae15c23b462c335bea2dcf38405dc9bbe0";
const PS3_ORACLE: &str = "decoder output equals an independent Python zlib walk (raw deflate per chunk, headers at the next offset ≡ 8 mod 16) of the same stored bytes; the loose .rpsgl at the same path differs (C0-AS7 §A.3), so no external file oracle exists";
const PS4: &str = "NHL 21 PS4 CUSA18182 (param.sfo TITLE `NHL 21`, APP_VER 01.00; C0-POR1 §1.2)";
const PS4_OVERLAYS_21: &str = "databigs/overlays_21.big";
const PS4_OVERLAYS_21_SHA: &str =
    "cdcf3889395eface41e5fcc1f1b0a66fb5229d8dd2bbc5989fac5b32ed7ce010";
const PS4_ORACLE: &str = "byte-identical to the externally extracted file at databigs/Extracted/<member path>; final chunk is stored (flag 4)";

/// The regression set: four PS3 multi-chunk members with distinct first-chunk
/// `zsize mod 8` residues (5, 2, 6, 4) and chunk counts (3, 2, 6, 11), and the
/// two PS4 members whose final chunk is stored.
pub(crate) const FIXTURES: &[Fixture] = &[
    Fixture {
        key: "chunkzip/nhllegacy-ps3-blus31540/patch.big/name_1_365_7_cm.rpsgl.chunkzip",
        copy: PS3,
        archive: PS3_PATCH,
        archive_sha256: PS3_PATCH_SHA,
        member_index: 306,
        member_path: "rendering/jersey/name_1_365_7_cm.rpsgl",
        stored_size: 1397,
        stored_sha256: "a11f8200d1a808caf99933d06b2e5c057376199192c9b2cf97c26238afdd432c",
        decoded_size: 350_164,
        decoded_sha256: "579ab9d7e48913c6497167a091e52f226f03263bb0247596e0db869095378645",
        oracle: PS3_ORACLE,
    },
    Fixture {
        key: "chunkzip/nhllegacy-ps3-blus31540/patch.big/blocker_0_86_1_tm.rpsgl.chunkzip",
        copy: PS3,
        archive: PS3_PATCH,
        archive_sha256: PS3_PATCH_SHA,
        member_index: 139,
        member_path: "rendering/blocker/blocker_0_86_1_tm.rpsgl",
        stored_size: 2041,
        stored_sha256: "beea012315af63dc24ee3189e6e7eedb55638eade91ad9e836fd8a30eaba53e4",
        decoded_size: 131_712,
        decoded_sha256: "2273a72d7090d6c0b7098981d6f09ba2ae19a7ae9c9acd2e6aa7576a70f9076a",
        oracle: PS3_ORACLE,
    },
    Fixture {
        key: "chunkzip/nhllegacy-ps3-blus31540/patch.big/feblocker_1_86_1_tm.rpsgl.chunkzip",
        copy: PS3,
        archive: PS3_PATCH,
        archive_sha256: PS3_PATCH_SHA,
        member_index: 197,
        member_path: "rendering/blocker/feblocker_1_86_1_tm.rpsgl",
        stored_size: 7566,
        stored_sha256: "8208c994aee2016c5704b976d2325cc7491be9611467ac05d9da3a2be9b2f977",
        decoded_size: 699_704,
        decoded_sha256: "c87a77a8634199743351d9864e3b7f688a5a2c0e8157a8f2cd95b01454f0ad70",
        oracle: PS3_ORACLE,
    },
    Fixture {
        key: "chunkzip/nhllegacy-ps3-blus31540/patch.big/fetrapper_0_86_0_dm.rpsgl.chunkzip",
        copy: PS3,
        archive: PS3_PATCH,
        archive_sha256: PS3_PATCH_SHA,
        member_index: 373,
        member_path: "rendering/trapper/fetrapper_0_86_0_dm.rpsgl",
        stored_size: 8810,
        stored_sha256: "57ebdf2a641054a808190d4f2cad88dfb518c1580cbed9adee60d78eed504336",
        decoded_size: 1_398_768,
        decoded_sha256: "28d08b12d269e3327f112886981d565a9f9255c107472814dad271591ed83d3d",
        oracle: PS3_ORACLE,
    },
    Fixture {
        key: "chunkzip/nhl21-ps4-cusa18182/overlays_21.big/guestinthebootharrived.gfx.chunkzip",
        copy: PS4,
        archive: PS4_OVERLAYS_21,
        archive_sha256: PS4_OVERLAYS_21_SHA,
        member_index: 5,
        member_path: "nhl/ui/swfassets/kettle/overlays/guestinthebooth/guestinthebootharrived.gfx",
        stored_size: 135_239,
        stored_sha256: "2f310dbadd66a8ae767cb8f693a091c0ae65f17c1197c840d51e791744c7da20",
        decoded_size: 137_911,
        decoded_sha256: "f516e40e4c5fe70cfa89626b0fbd12f7cec245b08057949f3c8b2124cef1ee3c",
        oracle: PS4_ORACLE,
    },
    Fixture {
        key: "chunkzip/nhl21-ps4-cusa18182/overlays_21.big/guestintheboothteaser.gfx.chunkzip",
        copy: PS4,
        archive: PS4_OVERLAYS_21,
        archive_sha256: PS4_OVERLAYS_21_SHA,
        member_index: 246,
        member_path: "nhl/ui/swfassets/kettle/overlays/guestinthebooth/guestintheboothteaser.gfx",
        stored_size: 163_696,
        stored_sha256: "2447f07144b2ddb3640e3e9f094fa64951f707107b05ed40196b5d8df1e738b9",
        decoded_size: 166_192,
        decoded_sha256: "a19e9e61c718fbd98af41e893efc5a31d136b611ef56caef58a6a97187898676",
        oracle: PS4_ORACLE,
    },
];

/// Every way a fixture lookup can fail; each carries what is needed to act.
#[derive(Debug)]
pub(crate) enum FixtureError {
    RootUnset {
        key: String,
    },
    RootEmpty {
        key: String,
    },
    RootMissing {
        root: PathBuf,
        key: String,
    },
    RootNotADirectory {
        root: PathBuf,
        key: String,
    },
    Missing {
        key: String,
        path: PathBuf,
    },
    NotAFile {
        key: String,
        path: PathBuf,
    },
    Unreadable {
        key: String,
        path: PathBuf,
        source: std::io::Error,
    },
    Empty {
        key: String,
        path: PathBuf,
    },
    /// The bytes at the path are not the recorded fixture.
    WrongBytes {
        key: String,
        path: PathBuf,
        want: String,
        got: String,
    },
}

fn help(key: &str) -> String {
    format!(
        "\n  configure it with:\n    {FIXTURE_ENV}=<root> cargo test -p tdb-archive-codec -- --ignored\
         \n  and place the fixture at:\n    <root>/{key}\
         \n  identities and provenance: crates/tdb-archive-codec/src/fixtures.rs\
         \n  fixtures are private game data: never committed, never searched for."
    )
}

impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootUnset { key } => write!(
                f,
                "fixture `{key}` is required but {FIXTURE_ENV} is not set.{}",
                help(key)
            ),
            Self::RootEmpty { key } => write!(
                f,
                "fixture `{key}` is required but {FIXTURE_ENV} is empty.{}",
                help(key)
            ),
            Self::RootMissing { root, key } => write!(
                f,
                "fixture `{key}` is required but the {FIXTURE_ENV} root does not exist: {}{}",
                root.display(),
                help(key)
            ),
            Self::RootNotADirectory { root, key } => write!(
                f,
                "fixture `{key}` is required but the {FIXTURE_ENV} root is not a directory: {}{}",
                root.display(),
                help(key)
            ),
            Self::Missing { key, path } => write!(
                f,
                "fixture `{key}` is missing: {}{}",
                path.display(),
                help(key)
            ),
            Self::NotAFile { key, path } => {
                write!(f, "fixture `{key}` is not a file: {}", path.display())
            }
            Self::Unreadable { key, path, source } => write!(
                f,
                "fixture `{key}` could not be read: {} ({source})",
                path.display()
            ),
            Self::Empty { key, path } => {
                write!(f, "fixture `{key}` is empty: {}", path.display())
            }
            Self::WrongBytes {
                key,
                path,
                want,
                got,
            } => write!(
                f,
                "fixture `{key}` at {} is not the recorded member: SHA-256 {got}, recorded {want}",
                path.display()
            ),
        }
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Resolve `key` under the root given by `root` (the `FIXTURE_ENV` value, or
/// `None` when unset) and load it. Separated from the environment so the
/// resolution rules are testable without touching the process environment.
pub(crate) fn load_with_root(
    root: Option<&std::ffi::OsStr>,
    fx: &Fixture,
) -> Result<Vec<u8>, FixtureError> {
    let key = fx.key.to_owned();
    let root = match root {
        None => return Err(FixtureError::RootUnset { key }),
        Some(r) if r.is_empty() => return Err(FixtureError::RootEmpty { key }),
        Some(r) => PathBuf::from(r),
    };
    match std::fs::metadata(&root) {
        Err(_) => return Err(FixtureError::RootMissing { root, key }),
        Ok(m) if !m.is_dir() => return Err(FixtureError::RootNotADirectory { root, key }),
        Ok(_) => {}
    }
    let path: PathBuf = root.join(Path::new(fx.key));
    let meta = match std::fs::metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(FixtureError::Missing { key, path })
        }
        Err(source) => return Err(FixtureError::Unreadable { key, path, source }),
        Ok(m) => m,
    };
    if !meta.is_file() {
        return Err(FixtureError::NotAFile { key, path });
    }
    let bytes = std::fs::read(&path).map_err(|source| FixtureError::Unreadable {
        key: key.clone(),
        path: path.clone(),
        source,
    })?;
    if bytes.is_empty() {
        return Err(FixtureError::Empty { key, path });
    }
    let got = sha256_hex(&bytes);
    if got != fx.stored_sha256 {
        return Err(FixtureError::WrongBytes {
            key,
            path,
            want: fx.stored_sha256.to_owned(),
            got,
        });
    }
    Ok(bytes)
}

/// Load the fixture or panic with the full diagnostic. Never returns early.
pub(crate) fn require(fx: &Fixture) -> Vec<u8> {
    let root = std::env::var_os(FIXTURE_ENV);
    match load_with_root(root.as_deref(), fx) {
        Ok(bytes) => bytes,
        Err(e) => panic!("{e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompress_chunkzip;

    fn check(fx: &Fixture) {
        let stored = require(fx);
        assert_eq!(stored.len(), fx.stored_size, "{}: stored size", fx.key);
        let decoded = decompress_chunkzip(&stored).unwrap_or_else(|e| {
            panic!(
                "{} ({}; {} {} member {} {}): {e}",
                fx.key, fx.copy, fx.archive, fx.archive_sha256, fx.member_index, fx.member_path
            )
        });
        assert_eq!(decoded.len(), fx.decoded_size, "{}: decoded size", fx.key);
        assert_eq!(
            sha256_hex(&decoded),
            fx.decoded_sha256,
            "{}: decoded digest; expectation from: {}",
            fx.key,
            fx.oracle
        );
    }

    #[test]
    #[ignore = "private fixture chunkzip/nhllegacy-ps3-blus31540/patch.big/name_1_365_7_cm.rpsgl.chunkzip: set NHL_STUDIO_FIXTURES and select with --ignored"]
    fn ps3_patch_name_1_365_7_cm_three_chunks_residue_5() {
        check(&FIXTURES[0]);
    }

    #[test]
    #[ignore = "private fixture chunkzip/nhllegacy-ps3-blus31540/patch.big/blocker_0_86_1_tm.rpsgl.chunkzip: set NHL_STUDIO_FIXTURES and select with --ignored"]
    fn ps3_patch_blocker_0_86_1_tm_two_chunks_residue_2() {
        check(&FIXTURES[1]);
    }

    #[test]
    #[ignore = "private fixture chunkzip/nhllegacy-ps3-blus31540/patch.big/feblocker_1_86_1_tm.rpsgl.chunkzip: set NHL_STUDIO_FIXTURES and select with --ignored"]
    fn ps3_patch_feblocker_1_86_1_tm_six_chunks_residue_6() {
        check(&FIXTURES[2]);
    }

    #[test]
    #[ignore = "private fixture chunkzip/nhllegacy-ps3-blus31540/patch.big/fetrapper_0_86_0_dm.rpsgl.chunkzip: set NHL_STUDIO_FIXTURES and select with --ignored"]
    fn ps3_patch_fetrapper_0_86_0_dm_eleven_chunks_residue_4() {
        check(&FIXTURES[3]);
    }

    #[test]
    #[ignore = "private fixture chunkzip/nhl21-ps4-cusa18182/overlays_21.big/guestinthebootharrived.gfx.chunkzip: set NHL_STUDIO_FIXTURES and select with --ignored"]
    fn ps4_nhl21_overlays_21_guestinthebootharrived_stored_tail() {
        check(&FIXTURES[4]);
    }

    #[test]
    #[ignore = "private fixture chunkzip/nhl21-ps4-cusa18182/overlays_21.big/guestintheboothteaser.gfx.chunkzip: set NHL_STUDIO_FIXTURES and select with --ignored"]
    fn ps4_nhl21_overlays_21_guestintheboothteaser_stored_tail() {
        check(&FIXTURES[5]);
    }

    // ----- self-contained: the resolution contract -----

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "tdb-archive-codec-fixtures-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn manifest_is_well_formed() {
        let mut keys = std::collections::BTreeSet::new();
        for fx in FIXTURES {
            assert!(keys.insert(fx.key), "duplicate key {}", fx.key);
            assert!(fx.key.starts_with("chunkzip/") && fx.key.ends_with(".chunkzip"));
            assert!(!fx.key.contains('\\') && !fx.key.contains(".."));
            for h in [fx.archive_sha256, fx.stored_sha256, fx.decoded_sha256] {
                assert_eq!(h.len(), 64);
                assert!(h
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
            }
            assert!(fx.stored_size > 0 && fx.decoded_size > 0);
            assert!(!fx.oracle.is_empty() && !fx.copy.is_empty() && !fx.archive.is_empty());
        }
        assert_eq!(FIXTURES.len(), 6);
    }

    #[test]
    fn unset_empty_missing_and_non_directory_roots_are_distinct_errors() {
        let fx = &FIXTURES[0];
        assert!(matches!(
            load_with_root(None, fx),
            Err(FixtureError::RootUnset { .. })
        ));
        assert!(matches!(
            load_with_root(Some(std::ffi::OsStr::new("")), fx),
            Err(FixtureError::RootEmpty { .. })
        ));
        let d = temp_dir("root");
        let absent = d.join("absent");
        assert!(matches!(
            load_with_root(Some(absent.as_os_str()), fx),
            Err(FixtureError::RootMissing { .. })
        ));
        let file = d.join("a-file");
        std::fs::write(&file, b"x").unwrap();
        assert!(matches!(
            load_with_root(Some(file.as_os_str()), fx),
            Err(FixtureError::RootNotADirectory { .. })
        ));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_empty_wrong_and_correct_fixture_files_under_a_valid_root() {
        let fx = &FIXTURES[1];
        let d = temp_dir("file");
        let err = load_with_root(Some(d.as_os_str()), fx).unwrap_err();
        assert!(matches!(err, FixtureError::Missing { .. }), "{err}");
        assert!(err.to_string().contains(fx.key) && err.to_string().contains(FIXTURE_ENV));

        let path = d.join(fx.key);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"").unwrap();
        assert!(matches!(
            load_with_root(Some(d.as_os_str()), fx),
            Err(FixtureError::Empty { .. })
        ));

        std::fs::write(&path, b"not the member").unwrap();
        let err = load_with_root(Some(d.as_os_str()), fx).unwrap_err();
        assert!(matches!(err, FixtureError::WrongBytes { .. }), "{err}");
        assert!(err.to_string().contains(fx.stored_sha256));

        // A directory where the file should be.
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(matches!(
            load_with_root(Some(d.as_os_str()), fx),
            Err(FixtureError::NotAFile { .. })
        ));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_synthetic_fixture_with_its_recorded_digest_loads() {
        // Build a fixture whose recorded digest matches: hash the bytes first.
        let bytes = b"synthetic chunkzip stand-in".to_vec();
        let digest = sha256_hex(&bytes);
        let leaked: &'static str = Box::leak(digest.into_boxed_str());
        let fx = Fixture {
            key: "chunkzip/synthetic/x.big/y.chunkzip",
            copy: "synthetic",
            archive: "x.big",
            archive_sha256: "0000000000000000000000000000000000000000000000000000000000000000",
            member_index: 0,
            member_path: "y",
            stored_size: bytes.len(),
            stored_sha256: leaked,
            decoded_size: 0,
            decoded_sha256: "",
            oracle: "none",
        };
        let d = temp_dir("ok");
        let path = d.join(fx.key);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(load_with_root(Some(d.as_os_str()), &fx).unwrap(), bytes);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn sha256_known_answers() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
