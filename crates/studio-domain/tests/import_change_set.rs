//! Phase 5.2b-next — the **cart orchestration**: a whole `ImportChangeSet` lowers to encoded ops via
//! the corpus adapter, and the per-item trail classifies every outcome (created / matched / blocked).
//! Runs against a **synthetic in-memory corpus** (no gitignored data), so it lives in CI.
//!
//! It proves the seam the Roster Builder drives: mixed Create + Match + two un-lowerable items →
//! exactly the ops that lowered, the right blockers, and a preview that is byte-identical to commit.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use studio_domain::{
    default_profile, EditSession, FhmCorpus, ImportIdentity, ImportItem, ImportOp, ItemOutcome,
    Platform, Project, ProjectRole,
};
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

/// A one-person synthetic corpus: skater #1, born 1995-03-04, 6'0"/200, with a rated 2024 season.
/// Ratings are an empty attrs line — enough to exercise the bio lowering path without depending on
/// the formula's attribute vocabulary (that path is covered by `import_corpus_e2e`).
fn synthetic_corpus() -> FhmCorpus {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(SCHEMA_SQL).unwrap();
    conn.execute(
        "INSERT INTO meta(key, value) VALUES ('schema_version', ?1)",
        [SCHEMA_VERSION.to_string()],
    )
    .unwrap();
    let slug = person_slug("Import", "Fresh", "1995-03-04");
    // `hand`/`nationality` use the corpus's real FHM vocab ("Left"/"Canada"), so the adapter resolves
    // them against the EA enums exactly as it would on live data.
    conn.execute(
        "INSERT INTO person(person_id, slug, first_name, last_name, dob, height, weight, hand,
                            nationality, retired, first_year, last_year, season_count, is_goalie, position)
         VALUES (1, ?1, 'Fresh', 'Import', '1995-03-04', 72, 200, 'Left', 'Canada', 0, 2024, 2024, 1, 0, 'C')",
        [slug],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO player_season(person_id, season_year, src_player_id, team_id, height, weight)
         VALUES (1, 2024, 10, -1, 72, 200)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ratings_season(person_id, season_year, c, ability, potential, attrs_json)
         VALUES (1, 2024, 80, 80.0, 80.0, '{}')",
        [],
    )
    .unwrap();
    FhmCorpus::from_conn(conn).unwrap()
}

#[test]
fn cart_lowers_mixed_create_match_and_blocks_unlowerable_items() {
    let corpus = synthetic_corpus();
    let profile = default_profile();
    let maxima = corpus.attribute_maxima().unwrap();

    let mut sess = open_anchor();
    let team = sess.team_summaries().unwrap().first().unwrap().team_id;
    // An existing EA skater to Match-update.
    let existing = {
        let model = sess.model().unwrap();
        model.players.iter().find(|p| p.skater_ai.is_some()).unwrap().player_id
    };

    // A mixed cart: create, match, a missing person, and a match at a bogus id.
    let items = vec![
        ImportItem { person_id: 1, season_year: 2024, target_team_id: team, identity: ImportIdentity::CreateNew, jersey: Some(42) },
        ImportItem { person_id: 1, season_year: 2024, target_team_id: team, identity: ImportIdentity::Match(existing), jersey: None },
        ImportItem { person_id: 999, season_year: 2024, target_team_id: team, identity: ImportIdentity::CreateNew, jersey: None },
        ImportItem { person_id: 1, season_year: 2024, target_team_id: team, identity: ImportIdentity::Match(9_999_999), jersey: None },
    ];

    let lowered = sess.lower_change_set(&corpus, &items, &profile, maxima.as_ref()).unwrap();

    // Only the two resolvable items produced ops; the other two are blockers.
    assert_eq!(lowered.ops.len(), 2, "create + match lowered; missing/bogus did not");
    assert_eq!(lowered.items.len(), 4, "one trail entry per cart item, in order");
    assert_eq!(lowered.items[0].outcome, ItemOutcome::Created);
    assert_eq!(lowered.items[1].outcome, ItemOutcome::Matched(existing));
    assert_eq!(lowered.items[2].outcome, ItemOutcome::PersonMissing);
    assert_eq!(lowered.items[3].outcome, ItemOutcome::NoSuchPlayer(9_999_999));
    assert!(lowered.has_blockers());
    assert_eq!(lowered.blocked().count(), 2);
    // Human-readable surface: the resolved person name rode through.
    assert_eq!(lowered.items[0].full_name, "Fresh Import");

    // Preview the lowered ops (dry-run), then commit the *same* ops — byte-identical.
    let before = sess.player_summaries().unwrap().len();
    let preview = sess.preview_import(&lowered.ops, lowered.skipped.clone()).unwrap();
    assert_eq!(preview.players_before, before);
    assert_eq!(preview.players_after, before + 1, "exactly the one create adds a player");
    assert_eq!(sess.undo_depth(), 0, "preview must not mutate the session");

    sess.commit_import(&lowered.ops).unwrap();
    assert_eq!(sess.undo_depth(), 1, "the whole cart is one undo entry");
    assert_eq!(sess.bytes(), preview.previewed.as_slice(), "commit == preview, byte for byte");

    // The create landed with its overridden jersey; the match renamed the existing player.
    let after = sess.player_summaries().unwrap();
    assert!(after.iter().any(|p| p.first_name == "Fresh" && p.last_name == "Import"), "create landed");
    let matched = after.iter().find(|p| p.player_id == existing).unwrap();
    assert_eq!((matched.first_name.as_str(), matched.last_name.as_str()), ("Fresh", "Import"));
}

#[test]
fn adapter_resolves_enum_identity_into_the_create_op() {
    let corpus = synthetic_corpus();
    let profile = default_profile();
    let maxima = corpus.attribute_maxima().unwrap();

    let mut sess = open_anchor();
    let team = sess.team_summaries().unwrap().first().unwrap().team_id;

    let items = vec![ImportItem {
        person_id: 1,
        season_year: 2024,
        target_team_id: team,
        identity: ImportIdentity::CreateNew,
        jersey: None,
    }];
    let lowered = sess.lower_change_set(&corpus, &items, &profile, maxima.as_ref()).unwrap();

    let ImportOp::Create { writes, .. } = &lowered.ops[0] else {
        panic!("expected a Create op");
    };
    let field = |id: &str| writes.iter().find(|w| w.field == id);
    // Handedness "Left" → GRkp raw 0; Nationality "Canada" → Lelh raw 0 (both from the verified enums).
    assert!(field("GRkp").is_some(), "handedness resolved to a bio write");
    assert!(field("Lelh").is_some(), "nationality resolved to a bio write");
    // No skips for the identity fields on nhl14 (they exist and the codes are in range).
    assert!(
        !lowered.items[0].skipped.iter().any(|s| s.field == "GRkp" || s.field == "Lelh"),
        "identity fields present on nhl14: {:?}",
        lowered.items[0].skipped
    );
}
