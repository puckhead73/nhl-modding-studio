//! Phase 5.0 **Spike S1** — ratings write codec: is write the exact inverse of read?
//!
//! The Roster Builder imports **computed** ratings on the human 36..99 (or PS4 0..99) scale and must
//! write them back so the game reads the same number. The read path decodes a stored rating as
//! `human = meta.decode_raw(raw) + edit_offset`, where `edit_offset` is **resolved from the target
//! field's own metadata + a db probe** (`FieldView.edit_offset`): `+36` on NHL 10→Legacy 6-bit
//! storage, **`+50` on NHL07 gen6**, and **`0` on PS4** (7-bit 0..99 stores the human value directly).
//!
//! This spike proves two things against real anchors, one per offset regime:
//!
//! 1. **Round-trip identity.** Writing `human - edit_offset` back through the ordinary
//!    `EditSession::set_field` seam reproduces the **byte-identical** stored raw — so the import can
//!    reuse `FieldView.edit_offset` and get symmetry for free.
//! 2. **The hazard is real.** Using a *fixed* offset (e.g. trusting `tdb_formula::EaRating.raw_offset`,
//!    which is `36` with no knowledge of the target) instead of the target-derived one **silently
//!    corrupts** the rating — wrong-by-36 on PS4, wrong-by-14 on gen6. This is why the import must
//!    derive the offset from the target field, never from the formula engine.
//!
//! Fast enough to run in the normal suite (a handful of players, three anchors).

use std::path::{Path, PathBuf};

use studio_domain::{
    build_player_attributes, entity_fields, EditSession, FieldValue, Platform, Project,
    ProjectRole, RosterModel,
};
use tdb_core::TdbFile;
use tdb_relations::PlayerKind;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-s1-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    fn copy_in(&self, src: &Path, name: &str) -> String {
        let dst = self.0.join(name);
        std::fs::copy(src, &dst).unwrap();
        dst.to_string_lossy().into_owned()
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Read the raw stored bits of one field on one record from a serialized db.
fn read_raw(bytes: &[u8], table: &str, rec: u32, field: &str) -> u64 {
    let db = TdbFile::parse(bytes).unwrap();
    let t = db.table_canonical(table).unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == field)
        .unwrap();
    db.read_u64(t, rec, f).unwrap()
}

/// The first skater with a `dSvy` AI companion record, for a stable rating target.
fn first_skater_ai(model: &RosterModel) -> Option<(u64, u32)> {
    model
        .players
        .iter()
        .find(|p| p.kind == PlayerKind::Skater && p.skater_ai.is_some())
        .map(|p| (p.player_id, p.skater_ai.unwrap()))
}

/// Open a fresh EditSession over a temp copy of an anchor's main db.
fn open(tmp: &Tmp, db_rel: &str, meta_rel: &str, platform: Platform, tag: &str) -> EditSession {
    let path = tmp.copy_in(&repo(db_rel), &format!("{tag}.db"));
    EditSession::begin(
        Project::load(
            &path,
            platform,
            ProjectRole::Nhlng,
            repo(meta_rel),
            repo("defs"),
        )
        .unwrap(),
    )
}

/// Pick a skater's first true rating cell; returns (companion_record, field_id, decoded, edit_offset).
fn pick_rating(sess: &EditSession) -> Option<(u32, String, i64, i64)> {
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = sess.model().ok()?;
    let (player_id, rec) = first_skater_ai(&model)?;
    let _ = player_id;
    let ratings = entity_fields(
        &db,
        sess.project().meta(),
        sess.project().enums(),
        "dSvy",
        rec,
    );
    let attrs = build_player_attributes(&ratings, "dSvy", false);
    let rv = attrs
        .flat
        .iter()
        .find(|a| a.is_rating && a.field.is_some())?;
    let fv = rv.field.clone()?;
    Some((
        rec,
        fv.studio_field,
        fv.decoded?,
        fv.edit_offset.unwrap_or(0),
    ))
}

/// Assert the round-trip identity + the fixed-offset hazard for one anchor / offset regime.
fn assert_regime(db_rel: &str, meta_rel: &str, platform: Platform, tag: &str, expect_offset: i64) {
    let tmp = Tmp::new(tag);
    let mut sess = open(&tmp, db_rel, meta_rel, platform, tag);
    let (rec, field, decoded, off) = pick_rating(&sess).expect("a skater rating cell");
    assert_eq!(
        off, expect_offset,
        "{tag}: resolved edit_offset for a 6-bit/PS4 rating"
    );

    // Human value the importer would produce (formula output scale).
    let human = decoded + off;
    let raw0 = read_raw(sess.bytes(), "dSvy", rec, &field);

    // (1) Correct import write: decoded_in = human - target_offset → byte-identical round-trip.
    sess.set_field("dSvy", rec, &field, FieldValue::Int(human - off))
        .unwrap();
    let raw1 = read_raw(sess.bytes(), "dSvy", rec, &field);
    assert_eq!(
        raw0, raw1,
        "{tag}: write(read(x)) must be byte-identical using the target offset"
    );

    // (2) Hazard: a *fixed* offset (36) that disagrees with the target silently corrupts. Only
    // demonstrable where the wrong decoded value is still in range and differs from the real one —
    // i.e. exactly the PS4 (off 0) and gen6 (off 50) regimes, which is the point.
    let fixed = 36;
    if fixed != off {
        let wrong_decoded = human - fixed;
        // In-range for both the (0,63) 6-bit and (0,99) PS4 fields when human is a typical rating.
        if (0..=63).contains(&wrong_decoded) && wrong_decoded != decoded {
            sess.set_field("dSvy", rec, &field, FieldValue::Int(wrong_decoded))
                .unwrap();
            let raw2 = read_raw(sess.bytes(), "dSvy", rec, &field);
            assert_ne!(
                raw0, raw2,
                "{tag}: trusting a fixed offset of 36 must visibly corrupt the rating (wrong-by-{})",
                (off - fixed).abs()
            );
        }
    }
}

#[test]
fn ratings_write_is_inverse_of_read_ps3_6bit() {
    // NHL 10→Legacy 6-bit storage → +36 scale.
    assert_regime(
        "anchors/ps3/nhl14/databases/nhlng.db",
        "anchors/ps3/nhl14/databases/nhlng-meta.xml",
        Platform::Ps3,
        "nhl14",
        36,
    );
}

#[test]
fn ratings_write_is_inverse_of_read_ps4_direct() {
    // PS4 7-bit 0..99 storage → the stored value IS the rating (offset 0). Trusting 36 corrupts.
    assert_regime(
        "anchors/ps4/nhl15/databases/nhl_game.db",
        "anchors/ps4/nhl15/databases/nhl_game-meta.xml",
        Platform::Ps4,
        "nhl15ps4",
        0,
    );
}

#[test]
fn ratings_write_is_inverse_of_read_gen6() {
    // NHL07 gen6 6-bit storage → +50 scale (not 36). Trusting 36 corrupts wrong-by-14.
    assert_regime(
        "anchors/ps3/nhl07/databases/NHL07NG.db",
        "anchors/ps3/nhl07/databases/NHL07NG-meta.xml",
        Platform::Ps3,
        "nhl07",
        50,
    );
}
