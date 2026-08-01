//! Phase 5.2b end-to-end — the corpus→EA bridge against the **real** FHM corpus.
//!
//! `#[ignore]`: needs `corpus/historical.db` (gitignored, not in CI). Run with:
//! `cargo test -p studio-domain --test import_corpus_e2e -- --ignored --nocapture`.
//!
//! Picks a real rated person from the corpus, builds a `PersonSeason` for their latest rated season
//! (bio + physicals + dob + ratings resolved to the target's `dSvy` fields), lowers it into the nhl14
//! anchor, and reads it back — proving the whole pipeline works on live data.

use std::path::{Path, PathBuf};

use studio_domain::{
    import, EditSession, FhmCorpus, Platform, Project, ProjectRole,
};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn open_anchor() -> EditSession {
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

#[test]
#[ignore = "needs corpus/historical.db (gitignored); run with --ignored"]
fn corpus_person_imports_into_anchor() {
    let corpus = FhmCorpus::open(&repo("corpus/historical.db")).expect("open corpus");

    // A real rated skater (first alphabetically with ratings) + their latest rated season.
    let people = corpus.person_summaries("", Some(false), 1).unwrap();
    let person = people.first().expect("a rated person in the corpus");
    let years = corpus.person_played_years(person.person_id).unwrap();
    let season = *years.last().expect("a rated season");
    println!(
        "person #{} {} {} — season {season}",
        person.person_id, person.first_name, person.last_name
    );

    let profile = studio_domain::default_profile();
    let maxima = corpus.attribute_maxima().unwrap();

    let mut sess = open_anchor();
    let team = sess.team_summaries().unwrap().first().unwrap().team_id;

    // Build the PersonSeason (ratings resolved against the target project's dSvy fields).
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let ps = import::person_season_from_corpus(
        &corpus,
        person.person_id,
        season,
        &profile,
        maxima.as_ref(),
        &db,
        sess.project().meta(),
        sess.project().enums(),
    )
    .unwrap()
    .expect("person present");

    assert!(!ps.last_name.is_empty(), "resolved a name");
    assert_eq!(ps.last_name, person.last_name);
    assert!(!ps.ratings.is_empty(), "computed ratings mapped to target dSvy fields");
    println!(
        "  -> {} {}  h={:?} w={:?} dob={:?} pos={} hand={:?} nat={:?} style={:?} ratings={}",
        ps.first_name, ps.last_name, ps.height_inches, ps.weight_lb, ps.birthdate, ps.position,
        ps.handedness, ps.nationality, ps.style, ps.ratings.len()
    );

    // Lower + apply into the anchor.
    let (op, skipped) =
        import::lower_create(&db, sess.project().meta(), sess.project().enums(), &ps, team).unwrap();
    // A resolved skater style must reach the companion as a QgFs write.
    if ps.style.is_some() {
        let studio_domain::ImportOp::Create { writes, .. } = &op else { panic!("create op") };
        assert!(writes.iter().any(|w| w.field == "QgFs"), "style lowered to QgFs");
    }
    println!("  lowered: {} skipped {:?}", ps.ratings.len(), skipped.iter().map(|s| &s.field).collect::<Vec<_>>());
    drop(db);
    sess.apply_import(&[op]).unwrap();

    // The imported person is on the roster with their corpus name.
    let created = sess
        .player_summaries()
        .unwrap()
        .into_iter()
        .find(|p| p.last_name == ps.last_name && p.first_name == ps.first_name)
        .expect("imported person present on the roster");
    assert_eq!(created.team_id, Some(team));
}
