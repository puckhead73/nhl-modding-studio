//! NHL 24 01.60 database overlay write against the real install, into a temporary
//! `game-mods`. The install's own `game-mods` is never touched. Selected runs fail (never
//! skip) when the inputs are missing:
//!
//! ```sh
//! NHL24_INSTALL=F:\Games\NHL24 NHL24_OODLE=<oo2core dll> \
//!   cargo test -p tdb-frostbite --test nhl24_overlay -- --ignored
//! ```
use std::{fs, path::PathBuf};
use tdb_core::TdbFile;
use tdb_frostbite::{nhl24, oodle::Oodle};

fn env(name: &str) -> PathBuf {
    let v = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is not set"));
    let p = PathBuf::from(v);
    assert!(p.exists(), "{name} does not exist: {}", p.display());
    p
}

#[test]
#[ignore = "needs NHL24_INSTALL and NHL24_OODLE"]
fn grown_database_round_trips_through_a_scratch_overlay() {
    let install = env("NHL24_INSTALL");
    let oodle = Oodle::open(&env("NHL24_OODLE")).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let roots = nhl24::Roots {
        mods: scratch.path().join("game-mods"),
        state: scratch.path().join("studio"),
        ..nhl24::Roots::install(&install)
    };
    fs::create_dir_all(&roots.mods).unwrap();

    let stock = nhl24::stock(&roots, Some(&oodle)).unwrap();
    let current = nhl24::read_database(&roots, Some(&oodle)).unwrap();
    assert!(!current.from_overlay);
    let db = TdbFile::parse(&current.db).unwrap();
    assert_eq!(db.serialize(), current.db, "stock database round trip");

    // An unchanged database must reproduce every packed file exactly.
    let same = nhl24::plan(&stock, &current.db, 0x10002).unwrap();
    assert_eq!(same.db_len.0, same.db_len.1);

    // Grow a table and rename one player, as the game-verified growth probe did.
    let bio = db.table("ubPc").unwrap();
    let grown = db.grow_table_max("ubPc", bio.max_records + 1000).unwrap();
    let g = TdbFile::parse(&grown).unwrap();
    let b = g.table("ubPc").unwrap();
    let mut edited = grown.clone();
    assert!(g.set_inline_str(&mut edited, b, 1570, b.field("QbMR").unwrap(), "FBW1GROWTH"));
    g.recompute_crcs(&mut edited);

    let cas = nhl24::choose_cas(&roots).unwrap();
    assert_eq!(cas, 0x10002, "the game-verified index is free");
    let plan = nhl24::plan(&stock, &edited, cas).unwrap();
    assert_eq!(plan.db_len, (current.db.len(), current.db.len() + 180_000));
    // Byte-identical to the files the game loaded in the growth probe (1 Nov 2026).
    let hash = |rel: &str| {
        use sha2::{Digest, Sha256};
        Sha256::digest(&plan.files[rel])
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>()
    };
    assert_eq!(
        hash(nhl24::GLOBALS_TOC),
        "5e4b1514db6fa90bc6f0ffe53260d90bd04c626e69600b0f041d68f23635abf2"
    );
    assert_eq!(
        hash("Patch/Gen4b/superbundlelayout/nhl_installpackage_00/cas_02.cas"),
        "eac93e8583eab844ee6153edddf62c9110dc41fcea89b3ef7a7f3686733c3d4e"
    );
    nhl24::apply(&roots, &plan, || None).unwrap();

    let after = nhl24::read_database(&roots, Some(&oodle)).unwrap();
    assert!(after.from_overlay);
    assert_eq!(after.db, edited);
    assert_eq!(after.meta_xml, current.meta_xml);
    // A second write rebuilds from stock and reuses Studio's CAS.
    assert_eq!(nhl24::choose_cas(&roots).unwrap(), cas);
    let again = nhl24::plan(
        &nhl24::stock(&roots, Some(&oodle)).unwrap(),
        &current.db,
        cas,
    )
    .unwrap();
    nhl24::apply(&roots, &again, || None).unwrap();
    assert_eq!(
        nhl24::read_database(&roots, Some(&oodle)).unwrap().db,
        current.db
    );

    assert_eq!(nhl24::restore(&roots, || None).unwrap(), 2);
    assert_eq!(fs::read_dir(&roots.mods).unwrap().count(), 0);
    assert!(
        !nhl24::read_database(&roots, Some(&oodle))
            .unwrap()
            .from_overlay
    );
}
