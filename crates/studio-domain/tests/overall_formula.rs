//! The derived Overall, measured against every anchor that carries ratings.
//!
//! `PlayerAttributes::computed_overall` recomputes EA's Overall from a player's own ratings instead
//! of reading the stored `eWUF` value, so the Attributes tab tracks rating edits live. That's only
//! honest if the regression reproduces EA's own numbers, so these tests measure it on real data and
//! pin both the era where it's exact and the era gate that keeps it off the screen elsewhere.
//!
//! **Measured** — median absolute error of `computed - stored`, floor rows (stored Overall parked at
//! the scale floor of 36, i.e. never authored) excluded:
//!
//! | anchor | goalie | skater | note |
//! |---|---|---|---|
//! | NHL 10 / 11 / 12 | — | — | vocabulary too different; coverage gate suppresses it |
//! | NHL 13 PS3 | 13 (bias −13) | 13 (bias −12) | `p_`/`g_` era — **suppressed by the era gate** |
//! | NHL 14 PS3 | 0 | 1 | the fit's own era |
//! | NHL 15 PS3 | 0 | 0 | |
//! | NHL 15 PS4 | 4 (bias −3.7) | 0 | 0..99 storage, same accuracy on skaters |
//! | NHL Legacy PS3 / NHL 16 PS4 | 1 | 1 | |
//! | NHL 17–21 PS4 | 3–4 | 2–4 | drifts as EA re-tuned; still within a rating tier |
//!
//! So the coefficients are an **NHL 14/15-era fit**: exact on the titles they were regressed from,
//! good through NHL 16, loosening but usable to NHL 21, and flatly mis-calibrated on the pre-NHL 14
//! `p_`/`g_` vocabulary. `era_calibrated_for_overall` gates that last case out; the UI labels the
//! number "calculated" and shows the stored value beside it whenever they disagree.
//!
//! The goalie side additionally validates a re-keying: those coefficients arrived keyed by EA
//! display name (`Glove Low`, `Angles`, `Rebounds`, …) and were mapped onto our era-portable rating
//! suffixes (`gsl`, `positioning`, `reboundcontrol`, …). A single bad pairing would still produce
//! plausible-looking numbers — the NHL 14 exactness below is what proves the mapping.

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

/// 36 is the floor of the display scale (raw 0 on the 6-bit era, a literal 36 on PS4). EA leaves the
/// stored Overall parked there on placeholder rows whose ratings are nevertheless populated, so
/// those rows measure nothing about the formula — the computed value is the *more* correct one
/// there, and that's exactly the case the tab's stored-vs-computed delta is for.
const UNSET_OVERALL: i64 = 36;

/// `computed - stored` for every authored record of a ratings companion table in an anchor.
/// `is_defenseman = false`: the bio join isn't available here and D are a minority of `dSvy`, so the
/// forward branch is what the population measures.
fn deltas(rel_db: &str, rel_meta: &str, table: &str) -> Vec<i64> {
    let bytes = std::fs::read(repo(rel_db)).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(rel_meta)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo("defs/enums")).unwrap();
    let count = db
        .table_canonical(table)
        .expect("companion table")
        .current_records as u32;
    (0..count)
        .filter_map(|rec| {
            let fields = entity_fields(&db, &meta, &enums, table, rec);
            let a = build_player_attributes(&fields, table, false);
            let (computed, stored) = (a.computed_overall?, a.stored_overall?);
            (stored > UNSET_OVERALL).then_some(computed - stored)
        })
        .collect()
}

/// `(median absolute error, share within 5, mean signed bias)`.
fn profile(deltas: &[i64]) -> (i64, f64, f64) {
    let mut abs: Vec<i64> = deltas.iter().map(|d| d.abs()).collect();
    abs.sort_unstable();
    let within = abs.iter().filter(|e| **e <= 5).count() as f64 / abs.len() as f64;
    let bias = deltas.iter().sum::<i64>() as f64 / deltas.len() as f64;
    (abs[abs.len() / 2], within, bias)
}

/// NHL 14 (PS3) is the era the coefficients were fit on: both regressions must reproduce EA's stored
/// Overall almost exactly. This is the guard on the formulas themselves — including the goalie
/// suffix re-keying, which only lands this well if every coefficient sits on the right attribute.
#[test]
fn nhl14_ps3_reproduces_stored_overall() {
    for (table, kind) in [("mHuy", "goalie"), ("dSvy", "skater")] {
        let d = deltas(
            "anchors/ps3/nhl14/databases/nhlng.db",
            "anchors/ps3/nhl14/databases/nhlng-meta.xml",
            table,
        );
        assert!(
            d.len() > 400,
            "{kind}: expected a real population, got {}",
            d.len()
        );
        let (median, within, bias) = profile(&d);
        assert!(median <= 1, "{kind} median error {median} (want <= 1)");
        assert!(
            within >= 0.98,
            "{kind}: only {:.0}% within 5 (want >= 98%)",
            within * 100.0
        );
        assert!(
            bias.abs() <= 1.0,
            "{kind} bias {bias:+.2} (want |bias| <= 1)"
        );
    }
}

/// NHL 15 on **PS4**, where ratings are stored 0..99 with no offset. The regression consumes
/// *display* values, so it has to land as well here as on the 6-bit PS3 file — this is the guard
/// against feeding it raw 6-bit values (or double-applying the +36).
#[test]
fn nhl15_ps4_skater_overall_survives_the_0_99_scale() {
    let d = deltas(
        "anchors/ps4/nhl15/databases/nhl_game.db",
        "anchors/ps4/nhl15/databases/nhl_game-meta.xml",
        "dSvy",
    );
    assert!(
        d.len() > 3000,
        "expected a real skater population, got {}",
        d.len()
    );
    let (median, within, bias) = profile(&d);
    assert!(median <= 1, "PS4 skater median error {median} (want <= 1)");
    assert!(
        within >= 0.98,
        "only {:.0}% within 5 (want >= 98%)",
        within * 100.0
    );
    assert!(bias.abs() <= 1.0, "PS4 skater bias {bias:+.2}");
}

/// The rest of the `c_` era stays inside a rating tier. NHL 21 is the far end of the range and the
/// loosest anchor — if it degrades past this, the single-fit approach has run out and the formula
/// needs a per-era re-fit rather than a wider tolerance.
#[test]
fn later_c_era_titles_stay_within_a_tier() {
    for (title, db, meta) in [
        (
            "NHL 16",
            "anchors/ps4/nhl16/databases/nhl_game.db",
            "anchors/ps4/nhl16/databases/nhl_game-meta.xml",
        ),
        (
            "NHL 18",
            "anchors/ps4/nhl18/databases/nhl_game.db",
            "anchors/ps4/nhl18/databases/nhl_game-meta.xml",
        ),
        (
            "NHL 21",
            "anchors/ps4/nhl21/databases/nhl_game.db",
            "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
        ),
    ] {
        for (table, kind) in [("mHuy", "goalie"), ("dSvy", "skater")] {
            let d = deltas(db, meta, table);
            assert!(
                d.len() > 300,
                "{title} {kind}: thin population ({})",
                d.len()
            );
            let (median, _, bias) = profile(&d);
            assert!(
                median <= 4,
                "{title} {kind} median error {median} (want <= 4)"
            );
            assert!(
                bias.abs() <= 4.0,
                "{title} {kind} bias {bias:+.2} (want |bias| <= 4)"
            );
        }
    }
}

/// The era gate. NHL 13 is the pre-`c_` vocabulary the regression was never fit on — it reads a flat
/// ~13 low there — and NHL 10–12 don't even resolve enough of the formula's inputs. Neither may
/// surface a derived Overall: a confidently wrong rating is worse than none.
///
/// NHL 10/12 also carry **no** `*_overall` field at all (another era gap, alongside the arena FK and
/// draft-field gaps), so the tab has nothing to fall back to there — asserted so a future change
/// that starts inventing a value gets caught.
#[test]
fn pre_nhl14_ratings_vocabulary_reports_no_overall() {
    for (title, dir) in [
        ("NHL 10", "nhl10"),
        ("NHL 12", "nhl12"),
        ("NHL 13", "nhl13"),
    ] {
        let has_overall_field = dir == "nhl13";
        let bytes = std::fs::read(repo(&format!("anchors/ps3/{dir}/databases/nhlng.db"))).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(
            &std::fs::read_to_string(repo(&format!("anchors/ps3/{dir}/databases/nhlng-meta.xml")))
                .unwrap(),
        )
        .unwrap();
        let enums = EnumRegistry::from_dir(repo("defs/enums")).unwrap();
        for table in ["dSvy", "mHuy"] {
            let fields = entity_fields(&db, &meta, &enums, table, 0);
            let a = build_player_attributes(&fields, table, false);
            assert!(a.kind.is_some(), "{title} {table}: expected a ratings view");
            assert_eq!(
                a.computed_overall, None,
                "{title} {table}: derived Overall must be suppressed outside the calibrated era",
            );
            // Where the schema carries the field, the stored value is still read — it's what the
            // tab falls back to reporting once the derived number is gated off.
            assert_eq!(
                a.stored_overall.is_some(),
                has_overall_field,
                "{title} {table}: stored Overall presence should track the schema",
            );
        }
    }
}
