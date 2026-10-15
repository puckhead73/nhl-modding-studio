//! Phase 5.2a gate — the batch-import seam (`EditSession::apply_import`).
//!
//! Spike S3 requires a whole cart to land as **one** transaction / **one** undo entry (per-op editing
//! retains a full-DB snapshot per op). These tests prove that on the nhl14 anchor: a multi-item cart
//! adds one undo entry, the same cart applied to two fresh sessions produces **byte-identical** dbs
//! (the preview==commit property at the seam), and a cart with one bad cell rolls back **atomically**
//! (no partial players).

use std::path::{Path, PathBuf};

use studio_domain::{
    build_player_attributes, entity_fields, EditSession, ImportCell, ImportOp, ImportTarget,
    ImportWrite, Platform, Project, ProjectRole,
};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn open() -> EditSession {
    EditSession::begin(
        Project::load(
            repo("anchors/ps3/nhl14/databases/nhlng.db"),
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo("anchors/ps3/nhl14/databases/nhlng-meta.xml"),
            repo("defs"),
        )
        .unwrap(),
    )
}

/// The studio (canonical) field id on `ubPc` whose metadata name is `meta_name`.
fn bio_field_id(sess: &EditSession, meta_name: &str) -> String {
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let bio = db.table_canonical("ubPc").unwrap();
    bio.fields
        .iter()
        .find(|f| {
            sess.project()
                .meta()
                .field(&bio.id.raw(), &f.id.raw())
                .map(|m| m.name == meta_name)
                .unwrap_or(false)
        })
        .map(|f| f.id.canonical(db.endian))
        .unwrap_or_else(|| panic!("no ubPc field named {meta_name}"))
}

fn first_team(sess: &EditSession) -> u64 {
    sess.team_summaries()
        .unwrap()
        .first()
        .expect("a team")
        .team_id
}

/// A real `dSvy` rating field's studio id (era-reassigned between titles — resolve, don't hardcode).
fn companion_rating_field(sess: &EditSession) -> String {
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = sess.model().unwrap();
    let rec = model
        .players
        .iter()
        .find_map(|p| p.skater_ai)
        .expect("a skater with a dSvy companion");
    let fields = entity_fields(
        &db,
        sess.project().meta(),
        sess.project().enums(),
        "dSvy",
        rec,
    );
    build_player_attributes(&fields, "dSvy", false)
        .flat
        .iter()
        .find_map(|a| {
            a.field
                .as_ref()
                .filter(|_| a.is_rating)
                .map(|f| f.studio_field.clone())
        })
        .expect("a dSvy rating field")
}

/// Two named skaters on `team`, each also writing one companion (`dSvy`) rating raw value.
fn cart(first_f: &str, last_f: &str, rating_f: &str, team: u64) -> Vec<ImportOp> {
    let mk = |first: &str, last: &str| ImportOp::Create {
        team,
        position: 1,
        writes: vec![
            ImportWrite {
                target: ImportTarget::Bio,
                field: first_f.to_string(),
                cell: ImportCell::Str(first.into()),
            },
            ImportWrite {
                target: ImportTarget::Bio,
                field: last_f.to_string(),
                cell: ImportCell::Str(last.into()),
            },
            // A dSvy rating cell (raw 20 fits the 6-bit 0..63 storage) — exercises the Companion target.
            ImportWrite {
                target: ImportTarget::Companion,
                field: rating_f.to_string(),
                cell: ImportCell::Int(20),
            },
        ],
    };
    vec![mk("Testy", "Mctestface"), mk("Otheri", "Mctestface")]
}

#[test]
fn batch_import_is_one_undo_entry_and_lands_fields() {
    let mut sess = open();
    let team = first_team(&sess);
    let (first_f, last_f) = (
        bio_field_id(&sess, "firstname"),
        bio_field_id(&sess, "lastname"),
    );
    let rating_f = companion_rating_field(&sess);
    let before = sess.player_summaries().unwrap().len();

    sess.apply_import(&cart(&first_f, &last_f, &rating_f, team))
        .unwrap();

    assert_eq!(
        sess.undo_depth(),
        1,
        "a whole cart is ONE undo entry (Spike S3)"
    );
    let after = sess.player_summaries().unwrap();
    assert_eq!(after.len(), before + 2, "both players created");
    let created: Vec<_> = after
        .iter()
        .filter(|p| p.last_name == "Mctestface")
        .collect();
    assert_eq!(
        created.len(),
        2,
        "the two imported players are present by name"
    );
    assert!(
        created.iter().any(|p| p.first_name == "Testy"),
        "bio string write landed"
    );
}

#[test]
fn identical_cart_lowers_to_byte_identical_dbs() {
    // The preview==commit invariant at the seam: the same ops → the same bytes.
    let mut a = open();
    let mut b = open();
    let team = first_team(&a);
    let (first_f, last_f) = (bio_field_id(&a, "firstname"), bio_field_id(&a, "lastname"));
    let rating_f = companion_rating_field(&a);

    a.apply_import(&cart(&first_f, &last_f, &rating_f, team))
        .unwrap();
    b.apply_import(&cart(&first_f, &last_f, &rating_f, team))
        .unwrap();

    assert_eq!(
        a.bytes(),
        b.bytes(),
        "identical carts must produce byte-identical databases"
    );
}

#[test]
fn a_bad_cell_rolls_back_the_whole_cart() {
    let mut sess = open();
    let team = first_team(&sess);
    let (first_f, last_f) = (
        bio_field_id(&sess, "firstname"),
        bio_field_id(&sess, "lastname"),
    );
    let rating_f = companion_rating_field(&sess);
    let before = sess.player_summaries().unwrap().len();
    let depth = sess.undo_depth();

    // Two valid creates, then an op-level error: update a player that does not exist. (Range is the
    // caller's job — the raw seam masks, not rejects — so we trigger a genuine op error to test that
    // the *whole* cart, including the two good creates, rolls back.)
    let mut ops = cart(&first_f, &last_f, &rating_f, team);
    ops.push(ImportOp::Update {
        player_id: 9_999_999,
        writes: vec![ImportWrite {
            target: ImportTarget::Bio,
            field: first_f.clone(),
            cell: ImportCell::Str("nope".into()),
        }],
    });

    assert!(
        sess.apply_import(&ops).is_err(),
        "an op targeting a missing player must fail the cart"
    );
    assert_eq!(sess.undo_depth(), depth, "a failed cart adds no undo entry");
    assert_eq!(
        sess.player_summaries().unwrap().len(),
        before,
        "a failed cart creates no players (atomic rollback)"
    );
}
