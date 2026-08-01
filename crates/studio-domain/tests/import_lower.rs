//! Phase 5.2b gate — `lower_create` encodes a person-season as the exact inverse of the display path.
//!
//! A synthetic `PersonSeason` (no corpus needed) is lowered to an `ImportOp` and applied to the nhl14
//! anchor; every field is then read back through the ordinary display path and must equal the input —
//! proving height (+54), weight (base), birthdate (probed 0-index), jersey, name, and a rating (S1
//! `edit_offset`) all round-trip. The created player must also land on the target team's roster.

use std::path::{Path, PathBuf};

use studio_domain::{
    build_player_attributes, entity_fields, import, EditSession, Platform, Project, ProjectRole,
};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn open(title: &str) -> EditSession {
    EditSession::begin(
        Project::load(
            repo(&format!("anchors/ps3/{title}/databases/nhlng.db")),
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(&format!("anchors/ps3/{title}/databases/nhlng-meta.xml")),
            repo("defs"),
        )
        .unwrap(),
    )
}

/// A real `dSvy` rating field's studio id (era-reassigned between titles — resolve, don't hardcode).
fn companion_rating_field(sess: &EditSession) -> String {
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = sess.model().unwrap();
    let rec = model.players.iter().find_map(|p| p.skater_ai).expect("a skater companion");
    let fields = entity_fields(&db, sess.project().meta(), sess.project().enums(), "dSvy", rec);
    build_player_attributes(&fields, "dSvy")
        .flat
        .iter()
        .find_map(|a| a.field.as_ref().filter(|_| a.is_rating).map(|f| f.studio_field.clone()))
        .expect("a dSvy rating field")
}

/// Lower a fixed synthetic person-season into `title` and assert every field round-trips through the
/// display path. Runs across titles so the **probed** offsets (esp. the nhl10 1-indexed birthdate)
/// are exercised, not just nhl14's.
fn round_trips_on(title: &str) {
    let mut sess = open(title);
    let team = sess.team_summaries().unwrap().first().expect("a team").team_id;
    let rating_field = companion_rating_field(&sess);

    let person = import::PersonSeason {
        first_name: "Synthetic".into(),
        last_name: "Playerton".into(),
        position: 1, // skater
        height_inches: Some(73), // 6'1"
        weight_lb: Some(195),
        birthdate: Some((1990, 5, 15)),
        jersey: Some(91),
        ratings: vec![(rating_field.clone(), 88)],
        ..Default::default()
    };

    // Lower against the current db, then apply as one cart.
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let (op, skipped) =
        import::lower_create(&db, sess.project().meta(), sess.project().enums(), &person, team)
            .expect("lowering produced an op");
    assert!(skipped.is_empty(), "all fields exist on nhl14, none should skip: {skipped:?}");
    drop(db);

    sess.apply_import(&[op]).unwrap();

    // Locate the created player.
    let summaries = sess.player_summaries().unwrap();
    let created = summaries.iter().find(|p| p.last_name == "Playerton").expect("created player present");
    assert_eq!(created.first_name, "Synthetic");
    assert_eq!(created.team_id, Some(team), "player landed on the target team's roster");

    // Read every field back through the display path — must equal the input.
    let view = sess.player_view(created.player_id).unwrap().unwrap();
    assert_eq!(view.header.full_name, "Synthetic Playerton");
    assert_eq!(view.header.height.as_deref(), Some("6'1\""), "height +54 round-trips");
    assert_eq!(view.header.weight.as_deref(), Some("195 lb"), "weight base round-trips");
    assert_eq!(view.header.birthdate.as_deref(), Some("1990-05-15"), "birthdate 0-index round-trips");
    assert_eq!(view.header.jersey_number, Some(91));

    // The rating decodes back to the human value via its edit_offset (Spike S1).
    let model = sess.model().unwrap();
    let ai = model.player(created.player_id).unwrap().skater_ai.unwrap();
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let fields = entity_fields(&db, sess.project().meta(), sess.project().enums(), "dSvy", ai);
    let rating = fields
        .iter()
        .find(|f| f.studio_field == rating_field)
        .expect("the rating field on the new companion");
    assert_eq!(rating.display, "88", "rating round-trips through edit_offset");
}

#[test]
fn lower_create_round_trips_on_nhl14() {
    round_trips_on("nhl14");
}

#[test]
fn lower_create_round_trips_on_nhl10_one_indexed_birthdate() {
    // nhl10 stores month/day 1-indexed; the probe must flow through lower_create unchanged.
    round_trips_on("nhl10");
}
