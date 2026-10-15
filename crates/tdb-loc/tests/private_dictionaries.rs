//! REG-1 (2026-09-18): explicit private-fixture checks over the **hash-bound install sources** that
//! the committed anchors do not represent. Contract as in C0-FX1
//! (`docs/validation/c0-fx1-explicit-fixture-tests.md`): every case is `#[ignore]`d; a selected case
//! either reads its fixture or fails naming the variable, the key and the resolved path.
//!
//! ```text
//! NHL_STUDIO_FIXTURES=<root> cargo test -p tdb-loc --test private_dictionaries -- --ignored
//! ```
//!
//! Layout under the root (`databases/<title>-<platform>-<package id>/<archive or disc path>/<file>`,
//! the bytes exactly as the install ships them; identities recorded in
//! `docs/validation/league-parity-batch-2026-09-18.md` §1 and `docs/audits/anchors-manifest.md`
//! "Provenance corrections"):
//!
//! ```text
//! <root>/databases/
//!   nhl20-ps4-cusa15439/inxp_00.big/eng_us.db           NHL 20's own dictionary (no anchor exists)
//!   nhl19-ps4-cusa11126/inxp_00.big/eng_us.db           NHL 19 base = the anchor committed as "nhl20"
//!   nhl19-ps4-cusa11126/patch_inxp_56.big/eng_us.db     NHL 19 patch-56 dictionary
//!   nhl21-ps4-cusa18182/inxp_00.big/eng_us.db           NHL 21's own dictionary (no anchor exists)
//!   nhl08-ps3-blus30062/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db   the PS3 disc file (the "ps3" anchor is the Xbox 360 file)
//!   nhl09-ps3-blus30168/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db   same
//! ```

use std::path::PathBuf;

use sha2::{Digest, Sha256};
use tdb_loc::{verify, LocDictionary};

const FIXTURE_ENV: &str = "NHL_STUDIO_FIXTURES";

/// Read a private fixture or fail with a diagnostic; never returns early.
fn require(key: &str, sha256: &str) -> Vec<u8> {
    let root = std::env::var_os(FIXTURE_ENV).unwrap_or_else(|| {
        panic!("{FIXTURE_ENV} is not set; fixture key {key} cannot be resolved (see the module docs)")
    });
    let path = PathBuf::from(root).join(key);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!("{FIXTURE_ENV} key {key}: cannot read {}: {e}", path.display())
    });
    assert!(!bytes.is_empty(), "{FIXTURE_ENV} key {key}: {} is empty", path.display());
    let got = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(got, sha256, "{FIXTURE_ENV} key {key}: {} is not the recorded source", path.display());
    bytes
}

struct Source {
    key: &'static str,
    sha256: &'static str,
    rows: usize,
    big_endian: bool,
    /// `(key, expected text)`; `None` = the key must be absent.
    probes: &'static [(&'static str, Option<&'static str>)],
}

fn check(s: &Source) {
    let bytes = require(s.key, s.sha256);
    for c in verify(&bytes).unwrap() {
        assert!(c.matches(), "{}: CRC scope {} stored {:08x} computed {:08x}", s.key, c.name, c.stored, c.computed);
    }
    let d = LocDictionary::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", s.key));
    assert_eq!(d.stats.endian_big, s.big_endian, "{}", s.key);
    assert_eq!(d.len(), s.rows, "{}", s.key);
    assert_eq!(d.stats.invalid_utf8, 0, "{}", s.key);
    assert_eq!(d.stats.hash_mismatches, 0, "{}", s.key);
    for (k, want) in s.probes {
        assert_eq!(d.text(k), *want, "{}: key {k}", s.key);
    }
}

#[test]
#[ignore = "private fixture databases/nhl20-ps4-cusa15439/inxp_00.big/eng_us.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl20_ps4_install_dictionary_is_not_the_committed_nhl20_anchor() {
    check(&Source {
        key: "databases/nhl20-ps4-cusa15439/inxp_00.big/eng_us.db",
        sha256: "0fc95ec84b71aaf23c2447783b277ba2df22b4ed762f901acf220af83f7739ad",
        rows: 40_535,
        big_endian: false,
        probes: &[
            ("Allsvenskan", Some("HockeyAllsvenskan")),
            ("TXT_ROW_LEAGUE", Some("Rest Of World")),
            ("TXT_USA_LEAGUE", Some("USA")),
            ("TXT_RUSSIA_LEAGUE", Some("RUS")),
            ("NHL", Some("NHL\u{ae}")),
        ],
    });
    // The committed "nhl20" anchor is a different file (37 734 rows): NHL 19 base.
    let anchor = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../anchors/ps4/nhl20/databases/eng_us.db")).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&anchor)), "62260005fcb02a1b5fc6dfeea9b98b25d49e28a1d92a0fbb829ebf8fb167a0f2");
}

#[test]
#[ignore = "private fixture databases/nhl19-ps4-cusa11126/inxp_00.big/eng_us.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl19_ps4_base_dictionary_is_the_anchor_committed_under_nhl20() {
    check(&Source {
        key: "databases/nhl19-ps4-cusa11126/inxp_00.big/eng_us.db",
        sha256: "62260005fcb02a1b5fc6dfeea9b98b25d49e28a1d92a0fbb829ebf8fb167a0f2",
        rows: 37_734,
        big_endian: false,
        probes: &[
            ("Allsvenskan", Some("HockeyAllsvenskan")),
            ("TXT_LEAGUE_NHL_ALUMNI", Some("NHL Alumni Association")),
            ("TXT_USA_LEAGUE", None),
        ],
    });
}

#[test]
#[ignore = "private fixture databases/nhl19-ps4-cusa11126/patch_inxp_56.big/eng_us.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl19_ps4_patch56_dictionary_differs_from_base() {
    check(&Source {
        key: "databases/nhl19-ps4-cusa11126/patch_inxp_56.big/eng_us.db",
        sha256: "fbb8adcf2f2fd0558a7c1564f3a5bdd3d2acfb83f11e91477163f4935a3613fa",
        rows: 38_001,
        big_endian: false,
        probes: &[("Allsvenskan", Some("HockeyAllsvenskan")), ("TXT_ROW_LEAGUE", Some("Rest Of World"))],
    });
}

#[test]
#[ignore = "private fixture databases/nhl21-ps4-cusa18182/inxp_00.big/eng_us.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl21_ps4_own_dictionary_differs_from_the_nhl19_base_proxy_on_league_keys() {
    check(&Source {
        key: "databases/nhl21-ps4-cusa18182/inxp_00.big/eng_us.db",
        sha256: "24693e2f1ae87bde23066d083458bae2efea9a633269518f4fede2354fbac969",
        rows: 55_766,
        big_endian: false,
        probes: &[
            ("Allsvenskan", Some("HockeyAllsvenskan")),
            ("TXT_ROW_LEAGUE", Some("Rest Of World")),
            ("TXT_LEAGUE_EBEL", Some("ICE Hockey League")),
            ("TXT_LEAGUE_NHL_ALL_TIME", Some("NHL Alumni All-time")),
        ],
    });
}

#[test]
#[ignore = "private fixture databases/nhl08-ps3-blus30062/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl08_ps3_disc_dictionary_is_not_the_committed_ps3_anchor() {
    check(&Source {
        key: "databases/nhl08-ps3-blus30062/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db",
        sha256: "b9fb3361dc6db09f33fec4d0adababa39b04664cd2bfb6959580fd1b1b78580b",
        rows: 5_718,
        big_endian: true,
        probes: &[("elitserien", Some("Elitserien")), ("SMLIIGA", Some("SMLiiga")), ("National", Some("National"))],
    });
}

#[test]
#[ignore = "private fixture databases/nhl09-ps3-blus30168/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl09_ps3_disc_dictionary_is_not_the_committed_ps3_anchor() {
    check(&Source {
        key: "databases/nhl09-ps3-blus30168/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db",
        sha256: "537319ced258cdb4892876085b3ed94a9d4d3e0fb2adb3f1cb94ef042e8a121f",
        rows: 9_434,
        big_endian: true,
        probes: &[
            ("TXT_LEAGUE_O2_EXTRALIGA", Some("O2 Extraliga")),
            ("TXT_LEAGUE_SUPER_LEAGUE", Some("Superliga")),
            ("Custom_Teams", Some("Custom Teams")),
        ],
    });
}
