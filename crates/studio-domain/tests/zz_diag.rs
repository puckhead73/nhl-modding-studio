//! Scratch diagnostic (temporary) — same measurement as overall_formula.rs, all anchors.
use std::path::{Path, PathBuf};

use studio_domain::{build_player_attributes, entity_fields};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn go(label: &str, dbp: &str, metap: &str, table: &str) {
    let Ok(bytes) = std::fs::read(repo(dbp)) else {
        println!("{label} {table}: MISSING");
        return;
    };
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(metap)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo("defs/enums")).unwrap();
    let Some(t) = db.table_canonical(table) else {
        println!("{label} {table}: no table");
        return;
    };
    let mut d: Vec<i64> = Vec::new();
    let mut unset = 0;
    for rec in 0..t.current_records as u32 {
        let fields = entity_fields(&db, &meta, &enums, table, rec);
        let a = build_player_attributes(&fields, table, false);
        let (Some(c), Some(s)) = (a.computed_overall, a.stored_overall) else {
            continue;
        };
        if s <= 36 {
            unset += 1;
            continue;
        }
        d.push(c - s);
    }
    if d.is_empty() {
        println!("{label} {table}: no rows");
        return;
    }
    let mut abs: Vec<i64> = d.iter().map(|x| x.abs()).collect();
    abs.sort_unstable();
    let n = d.len() as f64;
    let bias = d.iter().sum::<i64>() as f64 / n;
    let within5 = abs.iter().filter(|e| **e <= 5).count() as f64 / n;
    println!(
        "{label} {table}: n={} unset={unset} bias={bias:+.2} med={} p90={} within5={:.0}%",
        d.len(),
        abs[abs.len() / 2],
        abs[(n * 0.9) as usize],
        within5 * 100.0
    );
}

#[test]
fn diag() {
    let ps3 = [
        "nhl10",
        "nhl11",
        "nhl12",
        "nhl13",
        "nhl14",
        "nhl15",
        "nhllegacy",
    ];
    for g in ps3 {
        for t in ["mHuy", "dSvy"] {
            go(
                &format!("PS3-{g}"),
                &format!("anchors/ps3/{g}/databases/nhlng.db"),
                &format!("anchors/ps3/{g}/databases/nhlng-meta.xml"),
                t,
            );
        }
    }
    for g in [
        "nhl15", "nhl16", "nhl17", "nhl18", "nhl19", "nhl20", "nhl21",
    ] {
        for t in ["mHuy", "dSvy"] {
            go(
                &format!("PS4-{g}"),
                &format!("anchors/ps4/{g}/databases/nhl_game.db"),
                &format!("anchors/ps4/{g}/databases/nhl_game-meta.xml"),
                t,
            );
        }
    }
}
