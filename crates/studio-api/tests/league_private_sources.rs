//! REG-1 (2026-09-18): league catalogs over the **hash-bound install sources** that the committed
//! anchors do not represent — the real NHL 20 database (the committed `anchors/ps4/nhl20` files are
//! NHL 19 base members) and the NHL 19 patch-56 database. Contract as in C0-FX1: every case is
//! `#[ignore]`d and fails loudly without its fixture (no early return).
//!
//! ```text
//! NHL_STUDIO_FIXTURES=<root> cargo test -p studio-api --test league_private_sources -- --ignored
//! ```
//!
//! Fixture keys (bytes exactly as extracted from the installed archives; identities in
//! `docs/validation/league-parity-batch-2026-09-18.md` §1.1):
//!
//! ```text
//! <root>/databases/nhl20-ps4-cusa15439/inxp_00.big/{nhl_game.db, nhl_game-meta.xml, eng_us.db}
//! <root>/databases/nhl19-ps4-cusa11126/inxp_00.big/{nhl_game.db, nhl_game-meta.xml, eng_us.db}
//! <root>/databases/nhl19-ps4-cusa11126/patch_inxp_56.big/{nhl_game.db, eng_us.db}
//! ```

use std::path::PathBuf;

use sha2::{Digest, Sha256};
use studio_api::{parse_platform, Registry};

const FIXTURE_ENV: &str = "NHL_STUDIO_FIXTURES";

/// Resolve a private fixture path, verifying its identity; fails loudly, never returns early.
fn require(key: &str, sha256: &str) -> String {
    let root = std::env::var_os(FIXTURE_ENV).unwrap_or_else(|| {
        panic!("{FIXTURE_ENV} is not set; fixture key {key} cannot be resolved (see the module docs)")
    });
    let path = PathBuf::from(root).join(key);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("{FIXTURE_ENV} key {key}: cannot read {}: {e}", path.display()));
    assert!(!bytes.is_empty(), "{FIXTURE_ENV} key {key}: {} is empty", path.display());
    let got = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(got, sha256, "{FIXTURE_ENV} key {key}: {} is not the recorded source", path.display());
    path.to_str().unwrap().to_string()
}

fn defs() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../defs")
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
#[ignore = "private fixture databases/nhl20-ps4-cusa15439/inxp_00.big/*: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl20_ps4_install_database_has_the_nhl21_family_league_table_and_resolves_through_its_own_dictionary() {
    let db = require(
        "databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db",
        "1e2004784bccfdd6bed6e82533a67595798379f2c2d763c0636d9d370977a57b",
    );
    let meta = require(
        "databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game-meta.xml",
        "2fac0e733c9792038a64f966f277b163136517438509283e7f646d2920ad1f93",
    );
    let dict = require(
        "databases/nhl20-ps4-cusa15439/inxp_00.big/eng_us.db",
        "0fc95ec84b71aaf23c2447783b277ba2df22b4ed762f901acf220af83f7739ad",
    );
    let reg = Registry::default();
    let id = reg
        .open_workspace_from_paths(Some("nhl20"), parse_platform("ps4").unwrap(), &db, None, &meta, &defs())
        .unwrap()
        .project_id;
    let d = reg.attach_dictionary(id, &dict).unwrap();
    assert_eq!(d.rows, 40_535);
    let cat = reg.list_leagues(id).unwrap();
    assert_eq!(cat.source, "stockleagues");
    // 31 rows, 0..30 — the NHL 21-family layout, not the 29-row NHL 19 base layout the anchor has.
    assert_eq!(cat.leagues.len(), 31);
    let keys: Vec<&str> = cat.leagues.iter().map(|l| l.name_key.as_deref().unwrap()).collect();
    assert_eq!(keys[27], "TXT_USA_LEAGUE");
    assert_eq!(keys[28], "TXT_RUSSIA_LEAGUE");
    assert_eq!(keys.iter().filter(|k| **k == "CUSTOM_TEAMS").count(), 3);
    let alls = cat.leagues.iter().find(|l| l.name_key.as_deref() == Some("Allsvenskan")).unwrap();
    assert_eq!(alls.code, 8);
    assert_eq!(alls.member_teams, 14);
    assert_eq!(alls.label.as_deref(), Some("HockeyAllsvenskan"));
    assert_eq!(
        cat.leagues.iter().find(|l| l.name_key.as_deref() == Some("TXT_ROW_LEAGUE")).unwrap().label.as_deref(),
        Some("Rest Of World")
    );
    assert_eq!(cat.leagues.iter().find(|l| l.code == 27).unwrap().label.as_deref(), Some("USA"));
}

#[test]
#[ignore = "private fixture databases/nhl19-ps4-cusa11126/*: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl19_ps4_patch56_database_keeps_the_base_league_rows_with_member_drift() {
    let base_db = require(
        "databases/nhl19-ps4-cusa11126/inxp_00.big/nhl_game.db",
        "265e4a68d23ce44cef3240017beda0583c2cb6d246ea6672ca5612472429f408",
    );
    let meta = require(
        "databases/nhl19-ps4-cusa11126/inxp_00.big/nhl_game-meta.xml",
        "c67f4bfd3d152bfb68dd3d3c16823ef4f5b7c833cb46751f0834cb14a0cc90b3",
    );
    let patch_db = require(
        "databases/nhl19-ps4-cusa11126/patch_inxp_56.big/nhl_game.db",
        "61332e666a6b4afd4032212e9b8710658a9eddeea2637d3a4d288b1f30cab58d",
    );
    let patch_dict = require(
        "databases/nhl19-ps4-cusa11126/patch_inxp_56.big/eng_us.db",
        "fbb8adcf2f2fd0558a7c1564f3a5bdd3d2acfb83f11e91477163f4935a3613fa",
    );
    // The base member is byte-identical to the anchor committed under BOTH nhl19 and nhl20.
    let anchor = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../anchors/ps4/nhl20/databases/nhl_game.db")).unwrap();
    assert_eq!(std::fs::read(&base_db).unwrap(), anchor, "nhl20 anchor == NHL 19 base member");
    let reg = Registry::default();
    let idb = reg
        .open_workspace_from_paths(Some("nhl19"), parse_platform("ps4").unwrap(), &base_db, None, &meta, &defs())
        .unwrap()
        .project_id;
    let idp = reg
        .open_workspace_from_paths(Some("nhl19"), parse_platform("ps4").unwrap(), &patch_db, None, &meta, &defs())
        .unwrap()
        .project_id;
    reg.attach_dictionary(idp, &patch_dict).unwrap();
    let base = reg.list_leagues(idb).unwrap();
    let patch = reg.list_leagues(idp).unwrap();
    assert_eq!(base.leagues.len(), 29);
    assert_eq!(patch.leagues.len(), 29);
    for (b, p) in base.leagues.iter().zip(patch.leagues.iter()) {
        assert_eq!(b.code, p.code);
        assert_eq!(b.name_key, p.name_key, "code {}", b.code);
    }
    let members = |c: &studio_api::dto::LeagueCatalogDto, code: i64| c.leagues.iter().find(|l| l.code == code).unwrap().member_teams;
    // Only the Pro-Am league gained teams between base and patch 56 (80 -> 84); every other
    // league keeps its member count.
    assert_eq!((members(&base, 23), members(&patch, 23)), (80, 84), "Pro-Am");
    for code in 0..29 {
        if code != 23 {
            assert_eq!(members(&base, code), members(&patch, code), "code {code}");
        }
    }
    assert_eq!(members(&patch, 8), 14, "Allsvenskan unchanged");
    assert_eq!(
        patch.leagues.iter().find(|l| l.code == 8).unwrap().label.as_deref(),
        Some("HockeyAllsvenskan")
    );
}
