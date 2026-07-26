//! Phase 5.0 **Spike S3** — bulk-import history cost, and the batching decision.
//!
//! `EditSession::run` is the unit of undo history: each call parses the working buffer
//! (`EditTransaction::begin`), serializes a new one (`commit`), and **retains the pre-op full-DB
//! snapshot** on the undo stack (`session.rs`). So N per-op writes retain **N full-DB copies** — and
//! a mirrored [`Workspace`] runs two sessions, doubling it. A cart of N players × (create + ~15 field
//! writes) is O(N × 16) retained DB snapshots.
//!
//! `set_cells_int` already shows the fix: many cell writes inside **one** `run()` produce **one** undo
//! entry and retain **one** snapshot. This spike measures the two paths on identical work to size the
//! blow-up and confirm the import must batch (one `run`/commit = one history entry).
//!
//! `#[ignore]` (allocates hundreds of MB on the per-op path by design). Run:
//! `cargo test -p studio-domain --test import_history_cost -- --ignored --nocapture`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use studio_domain::{
    build_player_attributes, entity_fields, EditSession, FieldValue, Platform, Project, ProjectRole,
};
use tdb_core::TdbFile;
use tdb_relations::PlayerKind;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let safe: String = tag.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
        let dir = std::env::temp_dir().join(format!("nhlms-s3-{safe}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    fn copy_in(&self, src: &Path, name: &str) -> String {
        let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' { c } else { '-' }).collect();
        let dst = self.0.join(safe);
        std::fs::copy(src, &dst).unwrap();
        dst.to_string_lossy().into_owned()
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn open(tmp: &Tmp, db_rel: &str, meta_rel: &str, platform: Platform, tag: &str) -> EditSession {
    let path = tmp.copy_in(&repo(db_rel), &format!("{tag}.db"));
    EditSession::begin(
        Project::load(&path, platform, ProjectRole::Nhlng, repo(meta_rel), repo("defs")).unwrap(),
    )
}

/// A skater's `dSvy` record + its rating field ids (the cells an import writes per player).
fn rating_cells(sess: &EditSession) -> (u32, Vec<String>) {
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = sess.model().unwrap();
    let p = model
        .players
        .iter()
        .find(|p| p.kind == PlayerKind::Skater && p.skater_ai.is_some())
        .unwrap();
    let rec = p.skater_ai.unwrap();
    let fields = entity_fields(&db, sess.project().meta(), sess.project().enums(), "dSvy", rec);
    let attrs = build_player_attributes(&fields, "dSvy");
    let ids = attrs
        .flat
        .iter()
        .filter(|a| a.is_rating)
        .filter_map(|a| a.field.as_ref().map(|f| f.studio_field.clone()))
        .collect();
    (rec, ids)
}

fn bench(db_rel: &str, meta_rel: &str, platform: Platform, tag: &str) {
    let db_size = std::fs::metadata(repo(db_rel)).unwrap().len() as f64 / 1_048_576.0;

    // --- Path A: one set_field per cell (N undo entries, N retained snapshots) ---
    let tmp_a = Tmp::new(&format!("{tag}-perop"));
    let mut a = open(&tmp_a, db_rel, meta_rel, platform, tag);
    let (rec, ids) = rating_cells(&a);
    let n = ids.len();
    let t0 = Instant::now();
    for (k, id) in ids.iter().enumerate() {
        // Write within range for both (0,63) and (0,99); value irrelevant to the cost.
        a.set_field("dSvy", rec, id, FieldValue::Int((k % 60) as i64)).unwrap();
    }
    let per_op_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let per_op_depth = a.undo_depth();
    let per_op_retained = per_op_depth as f64 * db_size;

    // --- Path B: all cells in one set_cells_int (1 undo entry, 1 retained snapshot) ---
    let tmp_b = Tmp::new(&format!("{tag}-batch"));
    let mut b = open(&tmp_b, db_rel, meta_rel, platform, tag);
    let (recb, idsb) = rating_cells(&b);
    let writes: Vec<(String, i64)> =
        idsb.iter().enumerate().map(|(k, id)| (id.clone(), (k % 60) as i64)).collect();
    let t1 = Instant::now();
    b.set_cells_int("dSvy", recb, &writes).unwrap();
    let batch_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let batch_retained = b.undo_depth() as f64 * db_size;

    println!(
        "{:<12} {:>7.1} {:>5} {:>10.1} {:>7} {:>11.1} {:>10.1} {:>7} {:>11.2}",
        tag,
        db_size,
        n,
        per_op_ms,
        per_op_depth,
        per_op_retained,
        batch_ms,
        b.undo_depth(),
        batch_retained,
    );

    // Extrapolate a 25-player team-season (create + ~15 writes ≈ 16 ops/player) on a MIRRORED
    // workspace (×2 sessions). This is the number that motivates batching.
    let ops = 25 * 16;
    let per_op_team = ops as f64 * db_size * 2.0 / 1024.0; // GB retained, per-op path
    let batch_team = 1.0 * db_size * 2.0; // MB retained, single-commit path
    println!(
        "  └ 25-player team-season, mirrored: per-op ≈ {:.2} GB retained vs batched ≈ {:.1} MB (1 commit)",
        per_op_team, batch_team
    );
}

#[test]
#[ignore = "spike benchmark: the per-op path allocates hundreds of MB by design; run with --ignored"]
fn import_history_cost() {
    println!(
        "\n{:<12} {:>7} {:>5} {:>10} {:>7} {:>11} {:>10} {:>7} {:>11}",
        "anchor", "db_MB", "cells", "perop_ms", "depth", "retain_MB", "batch_ms", "depth", "retain_MB"
    );
    println!("{}", "-".repeat(88));
    bench(
        "anchors/ps3/nhl14/databases/nhlng.db",
        "anchors/ps3/nhl14/databases/nhlng-meta.xml",
        Platform::Ps3,
        "nhl14/ps3",
    );
    bench(
        "anchors/ps4/nhl21/databases/nhl_game.db",
        "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
        Platform::Ps4,
        "nhl21/ps4",
    );
    println!();
}
