//! The 6-bit rating **scale** probe across the title range (Players 3.2, 25 Sep 2026).
//!
//! PS3 / Xbox 360 titles store ratings in 6 bits (`0..63`); the display scale is `raw + 36`
//! (36..99) on NHL 11 → Legacy and `raw + 50` (50..99) on the gen6-era titles. EA's metadata is
//! identical (`rangehigh="63"`), so the scale is probed from the data: a **rating** value above 49
//! cannot exist on the 50..99 scale. The probe must look only at fields classified as ratings —
//! NHL 08's goalie table carries a 6-bit *non-rating* field above 49, which sent the whole title to
//! the 36 scale while its skater table tops out at 49.
//!
//! Measured here per anchor: the maximum raw value over the rating-classified 6-bit fields of
//! `dSvy` / `mHuy`, the count of rating cells above 49, and the maximum over every other 6-bit
//! field. The rule the code applies (`view::sixbit_rating_offset`): more than 1 % of rating cells
//! above 49 → 36..99, else 50..99. NHL 08 has 33 such cells (goalie `g_positioning` = 50, out of
//! ~64 000) — 0.05 %, one field at one value — and stays on the 50 scale like NHL 07 / 09 / 10;
//! NHL 11 → Legacy have 2–4 %.
use std::path::{Path, PathBuf};

use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn anchors() -> Vec<(String, String, PathBuf, PathBuf)> {
    let root = repo("anchors");
    let pairs = [
        ("NHL07NG.db", "NHL07NG-meta.xml"),
        ("nhlng.db", "nhlng-meta.xml"),
        ("nhl_game.db", "nhl_game-meta.xml"),
    ];
    let mut out = Vec::new();
    let mut platforms: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().collect();
    platforms.sort_by_key(|e| e.file_name());
    for platform in platforms.iter().filter(|e| e.path().is_dir()) {
        let mut games: Vec<_> = std::fs::read_dir(platform.path())
            .unwrap()
            .flatten()
            .collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let dbs = game.path().join("databases");
            if let Some((db, meta)) = pairs
                .iter()
                .find(|(d, m)| dbs.join(d).is_file() && dbs.join(m).is_file())
            {
                out.push((
                    platform.file_name().to_string_lossy().into_owned(),
                    game.file_name().to_string_lossy().into_owned(),
                    dbs.join(db),
                    dbs.join(meta),
                ));
            }
        }
    }
    assert_eq!(out.len(), 27);
    out
}

fn is_rating_name(name: &str) -> bool {
    let Some(suffix) = ["p_", "g_", "c_"].iter().find_map(|p| name.strip_prefix(p)) else {
        return false;
    };
    !["fighting", "style", "paddledown", "potential"].contains(&suffix)
}

/// Per anchor and companion table: (max raw over rating 6-bit fields, max raw over other 6-bit
/// fields, a note naming the fields; rating cells counted, rating cells above 49).
fn measure(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    studio_table: &str,
) -> Option<(u64, u64, String, usize, usize)> {
    let t = db.table_canonical(studio_table)?;
    let (mut rmax, mut omax, mut oname) = (0u64, 0u64, String::new());
    let mut rname = String::new();
    let mut over49 = 0usize;
    let mut cells = 0usize;
    for f in t.fields.iter().filter(|f| f.bit_width == 6) {
        let name = meta
            .field(&t.id.raw(), &f.id.raw())
            .map(|m| m.name.clone())
            .unwrap_or_default();
        let rating = is_rating_name(&name)
            && meta
                .field(&t.id.raw(), &f.id.raw())
                .is_some_and(|m| m.range_low == Some(0) && m.range_high == Some(63));
        let mut mx = 0u64;
        for rec in 0..t.current_records as u32 {
            let v = db.read_u64(t, rec, f).unwrap_or(0);
            if rating {
                cells += 1;
                if v > 49 {
                    over49 += 1;
                }
            }
            mx = mx.max(v);
        }
        if rating {
            if mx > rmax {
                rmax = mx;
                rname = name.clone();
            }
        } else if mx > omax {
            omax = mx;
            oname = name;
        }
    }
    Some((
        rmax,
        omax,
        format!("{oname}; top rating {rname}, {over49} of {cells} rating cells > 49"),
        cells,
        over49,
    ))
}

fn expected_offset(game: &str, platform: &str) -> i64 {
    if platform == "ps4" {
        0
    } else if matches!(game, "nhl07" | "nhl08" | "nhl09" | "nhl10") {
        50
    } else {
        36
    }
}

#[test]
fn six_bit_rating_maxima_per_anchor_and_the_probed_scale() {
    eprintln!("\n| anchor | dSvy ratings max / other max (field) | mHuy ratings max / other max (field) | expected offset |");
    eprintln!("|---|---|---|---|");
    for (platform, game, db_path, meta_path) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let sk = measure(&db, &meta, "dSvy");
        let gl = measure(&db, &meta, "mHuy");
        let fmt = |m: &Option<(u64, u64, String, usize, usize)>| match m {
            Some((r, o, n, ..)) => format!("{r} / {o} ({n})"),
            None => "—".into(),
        };
        let expected = expected_offset(&game, &platform);
        eprintln!(
            "| {platform}/{game} | {} | {} | {expected} |",
            fmt(&sk),
            fmt(&gl)
        );
        if platform != "ps4" {
            // More than 1 % of rating cells above 49 → the 36..99 scale, else 50..99.
            let cells =
                sk.as_ref().map(|m| m.3).unwrap_or(0) + gl.as_ref().map(|m| m.3).unwrap_or(0);
            let over =
                sk.as_ref().map(|m| m.4).unwrap_or(0) + gl.as_ref().map(|m| m.4).unwrap_or(0);
            let probed = if over * 100 > cells { 36 } else { 50 };
            assert_eq!(
                probed, expected,
                "{platform}/{game}: {over} of {cells} rating cells above 49"
            );
            if game == "nhl08" {
                assert_eq!(over, 33, "{platform}/{game}: the g_positioning = 50 rows");
            }
        }
    }
}
