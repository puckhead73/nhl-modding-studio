//! Phase 5.6 functional gate — the Roster Builder round-trip end to end (CI, synthetic corpus, no
//! anchor overwrite): corpus person → cart → **non-mutating preview** → **commit** → **clean
//! validation** → **save → reload stable**, with the created player present in the reloaded db.
//!
//! Complements the registry-level `studio-api/tests/roster_builder.rs` (preview→commit→**ledger
//! dedup** + surfaced `wBIz`) and the `#[ignore]` real-corpus `import_corpus_e2e.rs` (ratings).

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use studio_domain::{
    default_profile, EditSession, FhmCorpus, ImportIdentity, ImportItem, Platform, Project,
    ProjectRole, RosterModel,
};
use tdb_core::TdbFile;
use tdb_fhm::schema::{SCHEMA_SQL, SCHEMA_VERSION};
use tdb_fhm::slug::person_slug;

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

/// One rated skater (season 2024) with the corpus's real hand/nationality vocab.
fn synthetic_corpus() -> FhmCorpus {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(SCHEMA_SQL).unwrap();
    conn.execute("INSERT INTO meta(key,value) VALUES ('schema_version', ?1)", [SCHEMA_VERSION.to_string()]).unwrap();
    let slug = person_slug("Historyman", "Imported", "1992-06-15");
    conn.execute(
        "INSERT INTO person(person_id, slug, first_name, last_name, dob, height, weight, hand,
                            nationality, retired, first_year, last_year, season_count, is_goalie, position)
         VALUES (1, ?1, 'Imported', 'Historyman', '1992-06-15', 74, 205, 'Right', 'Sweden', 0, 2024, 2024, 1, 0, 'LW')",
        [slug],
    ).unwrap();
    conn.execute(
        "INSERT INTO player_season(person_id, season_year, src_player_id, team_id, height, weight)
         VALUES (1, 2024, 10, -1, 74, 205)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO ratings_season(person_id, season_year, lw, ability, potential, attrs_json)
         VALUES (1, 2024, 82, 82.0, 82.0, '{}')",
        [],
    ).unwrap();
    FhmCorpus::from_conn(conn).unwrap()
}

#[test]
fn import_round_trip_preview_commit_validate_save_reload() {
    let corpus = synthetic_corpus();
    let profile = default_profile();
    let maxima = corpus.attribute_maxima().unwrap();

    let mut sess = open_anchor();
    let team = sess.team_summaries().unwrap().first().unwrap().team_id;
    let before = sess.player_summaries().unwrap().len();

    // A one-item cart: create the corpus person on the target team.
    let items = vec![ImportItem {
        person_id: 1,
        season_year: 2024,
        target_team_id: team,
        identity: ImportIdentity::CreateNew,
        jersey: Some(19),
    }];
    let lowered = sess.lower_change_set(&corpus, &items, &profile, maxima.as_ref()).unwrap();
    assert_eq!(lowered.ops.len(), 1, "the create lowered");
    assert!(!lowered.has_blockers(), "no blockers");

    // Preview: non-mutating, one net create, and the previewed state audits clean.
    let preview = sess.preview_import(&lowered.ops, lowered.skipped.clone()).unwrap();
    assert_eq!(preview.players_after, preview.players_before + 1);
    assert_eq!(sess.undo_depth(), 0, "preview must not mutate");
    assert!(preview.audit.is_clean(), "previewed state is clean: {:?}", preview.audit.findings.len());

    // Commit for real → one undo entry.
    sess.commit_import(&lowered.ops).unwrap();
    assert_eq!(sess.undo_depth(), 1);

    // The whole DB still passes the Layer-4 audit after the import.
    assert!(
        sess.audit_report().unwrap().is_clean(),
        "the committed roster validates clean (a clean Dashboard)"
    );

    // The imported player is on the roster with their corpus identity.
    let created = sess
        .player_summaries()
        .unwrap()
        .into_iter()
        .find(|p| p.first_name == "Imported" && p.last_name == "Historyman")
        .expect("the imported player is present");
    assert_eq!(created.team_id, Some(team));

    // Save → reload: the packaged bytes re-parse into a coherent roster with the player retained.
    let saved = sess.save().unwrap();
    let reloaded = TdbFile::parse(&saved).unwrap();
    let model = RosterModel::build(&reloaded).expect("reloaded db builds a roster model");
    assert_eq!(model.players.len(), before + 1, "the created player persists across save/reload");
    assert!(model.player(created.player_id).is_some(), "the created wBIz survives the round-trip");
}
