//! Phase 5.0 **Spike S4** — capacity-full policy: is a boundary-insert into a full block safe, and
//! where must the ceiling gate live?
//!
//! `create_player` places a new slot via `eGlu` boundary-insert → `TdbFile::insert_record`. Reading
//! that path (`tdb-core`): when the block is full (`current == max`) the insert **grows the table by
//! one** — bumps `max_records`, shifts every downstream table's directory offset, regrows the file,
//! re-parses — and only hard-errors (`CapacityExhausted`) at `u16::MAX`. So the mechanics are safe and
//! self-growing; the write path does **not** respect the vanilla roster ceiling (txn.rs lists
//! "capacity growth past the ceiling" as still-to-come).
//!
//! This test proves the mechanical half on a real anchor: a GEN7 main db is **packed**
//! (`max == current`), so the very first `create_player` exercises the grow-on-full branch. It must
//! grow `ubPc.max_records` by one, add exactly one bio, and leave a db that re-parses into a coherent
//! roster model. The ceiling *gate* itself is a Phase 5.2 preview concern (see
//! docs/audits/capacity-full-policy.md), not the write path.

use std::path::{Path, PathBuf};

use studio_domain::{EditSession, Platform, Project, ProjectRole};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-s4-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `(max_records, current_records)` of a table in a serialized db.
fn table_cap(bytes: &[u8], table: &str) -> (u16, u16) {
    let db = TdbFile::parse(bytes).unwrap();
    let t = db.table_canonical(table).unwrap();
    (t.max_records, t.current_records)
}

#[test]
fn boundary_insert_into_full_block_grows_and_reparses() {
    let tmp = Tmp::new();
    let dst = tmp.0.join("nhl14.db");
    std::fs::copy(repo("anchors/ps3/nhl14/databases/nhlng.db"), &dst).unwrap();
    let mut sess = EditSession::begin(
        Project::load(
            &dst,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo("anchors/ps3/nhl14/databases/nhlng-meta.xml"),
            repo("defs"),
        )
        .unwrap(),
    );

    let (max0, cur0) = table_cap(sess.bytes(), "ubPc");
    let players0 = sess.model().unwrap().players.len();
    // GEN7 main db ships packed: the bio table is full, so the next insert must grow it.
    assert_eq!(
        max0, cur0,
        "nhl14 GEN7 main db is packed (ubPc max == current)"
    );

    // A real team to place the new player on.
    let team = sess
        .team_summaries()
        .unwrap()
        .first()
        .expect("a team")
        .team_id;

    // Boundary-insert a new skater (position 1). This hits the grow-on-full branch.
    let (_wbiz, _outcome) = sess.create_player(team, 1).unwrap();

    let (max1, cur1) = table_cap(sess.bytes(), "ubPc");
    assert_eq!(cur1, cur0 + 1, "exactly one bio added");
    assert_eq!(max1, max0 + 1, "a full block grew max_records by one");

    // The grown, shifted db must re-parse into a coherent model with the new player present.
    let model = sess.model().unwrap();
    assert_eq!(
        model.players.len(),
        players0 + 1,
        "the new player is in the model"
    );

    // And the whole file stays structurally valid (audit runs over a full re-parse).
    assert!(sess.validate().is_ok(), "post-insert db validates");
}
