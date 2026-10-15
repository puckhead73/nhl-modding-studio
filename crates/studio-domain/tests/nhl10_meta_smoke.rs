//! Guards the restored `anchors/ps3/nhl10/databases/nhlng-meta.xml`.
//!
//! The PS3 nhl10 anchor shipped its `nhlng.db` but not its meta xml (surfaced by Phase 5.0 Spike S2,
//! which had to skip nhl10). The t3db meta is **platform-invariant** — the PS3 and Xbox360 metas are
//! byte-identical for every title that has both (nhl09/11/14/legacy verified) — so the file was
//! restored from EA's own committed Xbox360 nhl10 meta. This test proves the restored meta actually
//! matches the PS3 nhl10 db: it loads, enumerates a plausible roster, composes birthdates, and probes
//! the birthdate encoding to nhl10's documented **1-indexed** month/day drift — a signal the meta,
//! db, and probe all agree.

use std::path::{Path, PathBuf};

use studio_domain::{
    probe_birthdate_encoding, BirthdateEncoding, EditSession, Platform, Project, ProjectRole,
};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
fn nhl10_ps3_loads_with_restored_meta() {
    let sess = EditSession::begin(
        Project::load(
            repo("anchors/ps3/nhl10/databases/nhlng.db"),
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo("anchors/ps3/nhl10/databases/nhlng-meta.xml"),
            repo("defs"),
        )
        .unwrap(),
    );

    // Plausible roster: nhl10 sits between nhl09 (3968) and nhl11 (5310) in S2's census.
    let players = sess.player_summaries().unwrap();
    assert!(
        (3000..7000).contains(&players.len()),
        "nhl10/ps3 enumerated an implausible roster ({}) — meta likely mismatched",
        players.len()
    );

    // A sample player composes a real birthdate (name + DOB both resolve through the meta).
    let with_dob = players
        .iter()
        .take(200)
        .filter_map(|p| sess.player_view(p.player_id).ok().flatten())
        .filter(|v| v.header.birthdate.is_some())
        .count();
    assert!(
        with_dob > 0,
        "no birthdates composed — bio fields did not resolve against the meta"
    );

    // nhl10's documented drift: month/day are 1-indexed (0-indexed on 11→21). The probe reads the
    // ubPc table via the meta, so this settling correctly is itself meta-agreement evidence.
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let bio = db.table_canonical("ubPc").unwrap();
    assert_eq!(
        probe_birthdate_encoding(&db, bio),
        BirthdateEncoding::OneIndexed,
        "nhl10 birthdate month/day must probe as 1-indexed"
    );
}
