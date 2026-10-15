//! Goalie Overall calibration — a bounded, read-only comparison of the fitted formula's consistency
//! input (after-receipt-27 prompt, task 4; receipt 28). Nothing here changes the formula.
//!
//! The fit consumes goalie `consistency` as stored + 36 on six-bit titles and stored + 0 on PS4
//! (`attributes::formula_input_offset`, calibrated so NHL 14 reproduces the stored Overall). PS4 NHL
//! 15's goalie bias (−3.7) equals the consistency coefficient (0.099379) × 36, which suggests the
//! fit may expect +36 on PS4 as well. This case measures, per goalie, `computed − stored` under both
//! inputs — A: stored + 0, B: stored + 36 — on representative era populations (NHL 14 / 15 PS3,
//! NHL Legacy, PS4 NHL 15 → 21 and, as a separate private-fixture case, the genuine NHL 20) and
//! prints the per-title error profile and the per-player movement. It asserts only what the data
//! must satisfy for the comparison to be meaningful (a real population; B − A moves each player by
//! at most one rounding step around 36 × 0.099379 ≈ 3.58), never which input is right.
//!
//! The variant input is produced by presenting the consistency field to the unchanged formula with
//! the declared range that selects the other offset (0..63 → +36, 0..99 → +0); the stored values,
//! every other rating and the coefficients are untouched.
use std::path::{Path, PathBuf};

use studio_domain::{build_player_attributes, entity_fields, FieldView};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

/// The stored Overall parked at the scale floor = never authored (see `overall_formula.rs`).
const UNSET_OVERALL: i64 = 36;

/// Present the consistency field with the range whose formula offset is `offset`.
fn with_consistency_input(fields: &[FieldView], offset: i64) -> Vec<FieldView> {
    fields
        .iter()
        .cloned()
        .map(|mut f| {
            if ["g_consistency", "c_consistency"].contains(&f.name.as_str()) {
                f.range_low = Some(0);
                f.range_high = Some(if offset == 36 { 63 } else { 99 });
            }
            f
        })
        .collect()
}

#[derive(Default)]
struct Profile {
    n: usize,
    abs: Vec<i64>,
    signed: i64,
    exact: usize,
    within1: usize,
    within3: usize,
}

impl Profile {
    fn add(&mut self, d: i64) {
        self.n += 1;
        self.abs.push(d.abs());
        self.signed += d;
        self.exact += (d == 0) as usize;
        self.within1 += (d.abs() <= 1) as usize;
        self.within3 += (d.abs() <= 3) as usize;
    }
    fn line(&mut self) -> String {
        self.abs.sort_unstable();
        let pct = |k: usize| 100.0 * k as f64 / self.n.max(1) as f64;
        format!(
            "median |err| {:>2}, mean {:+.2}, exact {:>5.1} %, ±1 {:>5.1} %, ±3 {:>5.1} %, max |err| {}",
            self.abs.get(self.n / 2).copied().unwrap_or(0),
            self.signed as f64 / self.n.max(1) as f64,
            pct(self.exact),
            pct(self.within1),
            pct(self.within3),
            self.abs.last().copied().unwrap_or(0)
        )
    }
}

/// Measure one database; returns `(authored goalies, share of players B improves, share B worsens)`.
fn measure(label: &str, db_path: &Path, meta_path: &Path) -> (usize, f64, f64) {
    let bytes = std::fs::read(db_path).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(meta_path).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo("defs/enums")).unwrap();
    let n = db.table_canonical("mHuy").unwrap().current_records as u32;
    let (mut a, mut b) = (Profile::default(), Profile::default());
    let mut by_value: std::collections::BTreeMap<i64, (Profile, Profile)> = Default::default();
    let (mut better, mut worse, mut same) = (0usize, 0usize, 0usize);
    let mut current_offset = None;
    for rec in 0..n {
        let fields = entity_fields(&db, &meta, &enums, "mHuy", rec);
        let Some(c) = fields.iter().find(|f| ["g_consistency", "c_consistency"].contains(&f.name.as_str())) else { continue };
        let value = c.decoded.unwrap();
        current_offset.get_or_insert(match (c.range_low, c.range_high) {
            (Some(0), Some(63)) => 36,
            _ => 0,
        });
        let current = build_player_attributes(&fields, "mHuy", false);
        let (Some(stored), Some(now)) = (current.stored_overall, current.computed_overall) else { continue };
        if stored <= UNSET_OVERALL {
            continue;
        }
        let ca = build_player_attributes(&with_consistency_input(&fields, 0), "mHuy", false).computed_overall.unwrap();
        let cb = build_player_attributes(&with_consistency_input(&fields, 36), "mHuy", false).computed_overall.unwrap();
        // The unchanged formula under the title's own range reproduces the current number.
        assert_eq!(now, if current_offset == Some(36) { cb } else { ca }, "{label} rec {rec}: variant plumbing");
        assert!((3..=4).contains(&(cb - ca)), "{label} rec {rec}: B − A = {} (expected one rounding step around 3.58)", cb - ca);
        let (da, dbb) = (ca - stored, cb - stored);
        a.add(da);
        b.add(dbb);
        let e = by_value.entry(value).or_default();
        e.0.add(da);
        e.1.add(dbb);
        match dbb.abs().cmp(&da.abs()) {
            std::cmp::Ordering::Less => better += 1,
            std::cmp::Ordering::Greater => worse += 1,
            std::cmp::Ordering::Equal => same += 1,
        }
    }
    let total = a.n;
    eprintln!("\n{label}: {total} authored goalies (stored Overall > 36); current input = stored + {}", current_offset.unwrap_or(0));
    eprintln!("  A (+0):  {}", a.line());
    eprintln!("  B (+36): {}", b.line());
    for (v, (pa, pb)) in by_value.iter_mut() {
        let (n, la, lb) = (pa.n, pa.line(), pb.line());
        eprintln!("  consistency {v} (n {n}): A {la} | B {lb}");
    }
    eprintln!("  per player: B closer {better}, B further {worse}, equal {same}");
    (total, better as f64 / total.max(1) as f64, worse as f64 / total.max(1) as f64)
}

fn anchor(platform: &str, game: &str) -> (PathBuf, PathBuf) {
    let dir = repo(&format!("anchors/{platform}/{game}/databases"));
    if platform == "ps4" {
        (dir.join("nhl_game.db"), dir.join("nhl_game-meta.xml"))
    } else {
        (dir.join("nhlng.db"), dir.join("nhlng-meta.xml"))
    }
}

#[test]
fn goalie_overall_under_both_consistency_inputs_on_representative_populations() {
    // The committed ps4/nhl20 directory holds an NHL 19 base copy; it is covered by ps4/nhl19 and
    // the genuine NHL 20 is the separate private-fixture case below.
    for (platform, game) in [("ps3", "nhl14"), ("ps3", "nhl15"), ("ps3", "nhllegacy"), ("ps4", "nhl15"), ("ps4", "nhl16"), ("ps4", "nhl17"), ("ps4", "nhl18"), ("ps4", "nhl19"), ("ps4", "nhl21")] {
        let (db, meta) = anchor(platform, game);
        let (n, ..) = measure(&format!("{platform}/{game}"), &db, &meta);
        assert!(n > 300, "{platform}/{game}: a real goalie population ({n})");
    }
}

#[test]
#[ignore = "private fixture databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn goalie_overall_under_both_consistency_inputs_on_the_real_nhl20() {
    let root = PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    let (db, meta) = (root.join("nhl_game.db"), root.join("nhl_game-meta.xml"));
    assert!(db.is_file() && meta.is_file(), "fixture missing (fails, never skips)");
    let (n, ..) = measure("ps4/nhl20 (real install)", &db, &meta);
    assert!(n > 300, "real NHL 20: a real goalie population ({n})");
}
