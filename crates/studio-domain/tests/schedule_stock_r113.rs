//! Receipt 48 — the shipped single-phase groups are offered only by exact identity (R123 Q9): every anchor whose
//! database SHA-256 is a recorded R113 source yields its table's groups with Studio's own fingerprint equal to R113's;
//! a changed membership, another title's file and the Austrian two-phase table fail closed.
use std::path::{Path, PathBuf};

use sha2::Digest;
use studio_domain::schedule_stock::{r113_fingerprint, stock_groups, StockPartitionDefs};
use studio_domain::{EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn defs() -> StockPartitionDefs {
    StockPartitionDefs::load(repo("defs/schedules/stock-partitions.json")).unwrap()
}

/// (anchor dir, db file, meta file, platform) for every main-database anchor.
fn anchors() -> Vec<(PathBuf, PathBuf, PathBuf, Platform)> {
    let mut out = Vec::new();
    for (plat, platform) in [("xbox360", Platform::Xbox), ("ps3", Platform::Ps3), ("ps4", Platform::Ps4)] {
        let Ok(rd) = std::fs::read_dir(repo(&format!("anchors/{plat}"))) else { continue };
        for e in rd.flatten() {
            let d = e.path().join("databases");
            for (db, meta) in [("nhlng.db", "nhlng-meta.xml"), ("NHL07NG.db", "NHL07NG-meta.xml"), ("nhl_game.db", "nhl_game-meta.xml")] {
                if d.join(db).is_file() && d.join(meta).is_file() {
                    out.push((d.clone(), d.join(db), d.join(meta), platform));
                }
            }
        }
    }
    out
}

#[test]
fn every_recorded_anchor_source_matches_by_hash_table_and_fingerprint() {
    let defs = defs();
    assert_eq!(defs.contracts.len(), 30);
    assert!(defs.contracts.iter().all(|c| c.table != "austrianschedule"), "two-phase tables are not offered");
    let mut matched = 0;
    let mut unmodelled: Vec<String> = Vec::new();
    for (dir, db, meta, platform) in anchors() {
        let sha = format!("{:x}", sha2::Sha256::digest(std::fs::read(&db).unwrap()));
        let rows: Vec<_> = defs.contracts.iter().filter(|c| c.db_sha256 == sha).collect();
        if rows.is_empty() {
            continue;
        }
        let mut s = EditSession::begin(Project::load(&db, platform, ProjectRole::Nhlng, &meta, repo("defs")).unwrap());
        // The title id selects the league vocabulary on layouts several titles share (as a project open does).
        s.set_game_id(Some(dir.parent().unwrap().file_name().unwrap().to_string_lossy().into_owned()));
        assert_eq!(s.project().loaded_identity().map(|i| i.sha256.as_str()), Some(sha.as_str()));
        let tables = s.schedule_tables().unwrap();
        for c in rows {
            let t = tables.iter().find(|t| t.name == c.table).unwrap_or_else(|| panic!("{} {}", c.source, c.table));
            // A table Studio cannot model (its league is not catalogued on this title) cannot be generated at all;
            // it is listed, not counted.
            let model = match s.schedule_model(&t.studio_table) {
                Ok(m) => m,
                Err(e) => {
                    unmodelled.push(format!("{} {}: {e}", c.source, c.table));
                    continue;
                }
            };
            assert_eq!(r113_fingerprint(&model.slots), c.mapping_fingerprint, "{} {}: Studio's fingerprint = R113's", c.source, c.table);
            let g = stock_groups(&defs, &c.table, Some(&sha), &model.slots).unwrap_or_else(|e| panic!("{} {}: {e}", c.source, c.table));
            let n = g.groups.iter().map(Vec::len).sum::<usize>();
            assert_eq!(n, model.slots.entries.len(), "{} {}: every member in one group", c.source, c.table);
            // The recorded games follow from the groups and quotas (R123 Q4) and equal the table's active rows.
            let within: u64 = g.groups.iter().map(|x| (x.len() * (x.len() - 1) / 2) as u64).sum::<u64>() * g.within as u64;
            let pairs = (n * (n - 1) / 2) as u64;
            let across = (pairs - g.groups.iter().map(|x| (x.len() * (x.len() - 1) / 2) as u64).sum::<u64>()) * g.across as u64;
            assert_eq!(within + across, c.games, "{} {}", c.source, c.table);
            assert_eq!(model.rows.len() as u64, c.games, "{} {}: the table's active rows", c.source, c.table);
            eprintln!("{} {}: {} groups {:?}, within {} across {}, {} games", c.source, c.table, g.groups.len(), g.groups.iter().map(Vec::len).collect::<Vec<_>>(), g.within, g.across, c.games);
            matched += 1;
        }
    }
    eprintln!("matched {matched}; not modelled by Studio: {unmodelled:#?}");
    assert!(unmodelled.is_empty(), "{unmodelled:#?}");
    assert_eq!(matched, 41, "anchor instances matching a recorded source (X360 + byte-identical PS3 copies, PS4 15-21; anchors/ps4/nhl20 = NHL 19 base)");
}

#[test]
fn another_source_a_changed_membership_or_an_unrecorded_table_fail_closed() {
    let defs = defs();
    let d = repo("anchors/xbox360/nhl11/databases");
    let db = d.join("nhlng.db");
    let sha = format!("{:x}", sha2::Sha256::digest(std::fs::read(&db).unwrap()));
    let s = EditSession::begin(Project::load(&db, Platform::Xbox, ProjectRole::Nhlng, d.join("nhlng-meta.xml"), repo("defs")).unwrap());
    let swiss = s.schedule_tables().unwrap().into_iter().find(|t| t.name == "swissschedule").unwrap();
    let model = s.schedule_model(&swiss.studio_table).unwrap();
    assert!(stock_groups(&defs, "swissschedule", Some(&sha), &model.slots).is_ok());
    // Another title's hash for the same table name: refused, never borrowed.
    let other = defs.contracts.iter().find(|c| c.table == "swissschedule" && c.db_sha256 != sha).unwrap();
    let e = stock_groups(&defs, "swissschedule", Some(&other.db_sha256), &model.slots).unwrap_err();
    assert!(e.contains("mapping fingerprint") || e.contains("not one of the recorded sources"), "{e}");
    let e = stock_groups(&defs, "swissschedule", Some(&"0".repeat(64)), &model.slots).unwrap_err();
    assert!(e.contains("not one of the recorded sources"), "{e}");
    assert!(stock_groups(&defs, "swissschedule", None, &model.slots).unwrap_err().contains("not opened from a file"));
    // Membership changed (two clubs swapped): the fingerprint differs.
    let mut slots = model.slots.clone();
    slots.entries.swap(0, 1);
    let (a, b) = (slots.entries[0].slot, slots.entries[1].slot);
    slots.entries[0].slot = b;
    slots.entries[1].slot = a;
    assert!(stock_groups(&defs, "swissschedule", Some(&sha), &slots).unwrap_err().contains("membership or order differs"));
    // Tables without a recorded single-phase partition (NHL, AHL, Austrian two-phase) offer nothing.
    for t in ["nhlschedule", "austrianschedule", "ahlschedule"] {
        assert!(stock_groups(&defs, t, Some(&sha), &model.slots).unwrap_err().contains("no shipped single-phase groups"), "{t}");
    }
}
