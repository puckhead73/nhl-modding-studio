//! Phase 5.2 two-phase gate — preview is a non-mutating dry-run, and **commit is byte-identical to
//! preview** (the pipeline's headline invariant), for a mixed Create + Match cart.

use std::path::{Path, PathBuf};

use studio_domain::{
    build_player_attributes, entity_fields, import, EditSession, PersonSeason, Platform, Project,
    ProjectRole,
};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
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

fn skater_rating_field(sess: &EditSession) -> String {
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let rec = sess.model().unwrap().players.iter().find_map(|p| p.skater_ai).unwrap();
    let fields = entity_fields(&db, sess.project().meta(), sess.project().enums(), "dSvy", rec);
    build_player_attributes(&fields, "dSvy")
        .flat
        .iter()
        .find_map(|a| a.field.as_ref().filter(|_| a.is_rating).map(|f| f.studio_field.clone()))
        .unwrap()
}

#[test]
fn preview_is_non_mutating_and_commit_is_byte_identical() {
    let mut sess = open();
    let team = sess.team_summaries().unwrap().first().unwrap().team_id;
    let rating = skater_rating_field(&sess);

    // Pick an existing skater to Match-update.
    let existing = {
        let model = sess.model().unwrap();
        model.players.iter().find(|p| p.skater_ai.is_some()).unwrap().player_id
    };

    // Build the cart: one CreateNew + one Match(existing) — lowered against the current db.
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = sess.project().meta();
    let enums = sess.project().enums();

    let new_person = PersonSeason {
        first_name: "Fresh".into(),
        last_name: "Import".into(),
        position: 1,
        height_inches: Some(72),
        weight_lb: Some(200),
        birthdate: Some((1995, 3, 4)),
        jersey: Some(77),
        ratings: vec![(rating.clone(), 80)],
    };
    let upd_person = PersonSeason {
        first_name: "Renamed".into(),
        last_name: "Matched".into(),
        position: 1,
        ratings: vec![(rating.clone(), 71)],
        ..Default::default()
    };

    let (create_op, s1) = import::lower_create(&db, meta, enums, &new_person, team).unwrap();
    let (update_op, s2) = import::lower_update(&db, meta, enums, &upd_person, existing).unwrap();
    drop(db);
    let ops = vec![create_op, update_op];
    let skipped = [s1, s2].concat();

    let before = sess.player_summaries().unwrap().len();

    // Preview: a dry-run that must NOT touch the session.
    let preview = sess.preview_import(&ops, skipped).unwrap();
    assert_eq!(preview.players_before, before);
    assert_eq!(preview.players_after, before + 1, "one net create");
    assert!(preview.skipped.is_empty(), "all fields present on nhl14: {:?}", preview.skipped);
    assert_eq!(sess.undo_depth(), 0, "preview must not mutate the session");
    assert_eq!(sess.player_summaries().unwrap().len(), before, "preview must not add players");

    // Commit: the same ops from the same state → byte-identical to the preview.
    sess.commit_import(&ops).unwrap();
    assert_eq!(sess.undo_depth(), 1, "commit is one undo entry");
    assert_eq!(sess.bytes(), preview.previewed.as_slice(), "commit == preview, byte for byte");

    // The edits are visible: the create landed and the match renamed the existing player.
    let after = sess.player_summaries().unwrap();
    assert!(after.iter().any(|p| p.first_name == "Fresh" && p.last_name == "Import"), "create landed");
    let matched = after.iter().find(|p| p.player_id == existing).unwrap();
    assert_eq!((matched.first_name.as_str(), matched.last_name.as_str()), ("Renamed", "Matched"));
}
