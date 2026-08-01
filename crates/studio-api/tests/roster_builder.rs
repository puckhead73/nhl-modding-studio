//! Phase 5.4 — drive the Roster Builder service seam end-to-end without Tauri: open a real anchor +
//! a synthetic corpus through the [`Registry`], then `import_preview` → `import_commit` → re-preview,
//! proving the bridge (corpus + project), the player-count delta, and the **ledger dedup warning**.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use studio_api::{parse_platform, parse_role, Registry};
use tdb_fhm::schema::{SCHEMA_SQL, SCHEMA_VERSION};
use tdb_fhm::slug::person_slug;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: PathBuf) -> String {
    p.to_str().unwrap().to_string()
}

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DEFS: &str = "defs";

/// Write a one-person synthetic corpus to `path` (skater #1, rated 2024 season).
fn write_corpus(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(SCHEMA_SQL).unwrap();
    conn.execute("INSERT INTO meta(key,value) VALUES ('schema_version', ?1)", [SCHEMA_VERSION.to_string()]).unwrap();
    conn.execute("INSERT INTO meta(key,value) VALUES ('person_count', '1')", []).unwrap();
    let slug = person_slug("Import", "Fresh", "1995-03-04");
    conn.execute(
        "INSERT INTO person(person_id, slug, first_name, last_name, dob, height, weight, hand,
                            nationality, retired, first_year, last_year, season_count, is_goalie, position)
         VALUES (1, ?1, 'Fresh', 'Import', '1995-03-04', 72, 200, 'Left', 'Canada', 0, 2024, 2024, 1, 0, 'C')",
        [slug],
    ).unwrap();
    conn.execute(
        "INSERT INTO player_season(person_id, season_year, src_player_id, team_id, height, weight)
         VALUES (1, 2024, 10, -1, 72, 200)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO ratings_season(person_id, season_year, c, ability, potential, attrs_json)
         VALUES (1, 2024, 80, 80.0, 80.0, '{}')",
        [],
    ).unwrap();
}

fn import_item_dto(team: u64) -> studio_api::ImportItemDto {
    studio_api::ImportItemDto {
        person_id: 1,
        season_year: 2024,
        target_team_id: team,
        match_ea_player_id: None, // CreateNew
        jersey: Some(99),
    }
}

#[test]
fn roster_builder_preview_commit_and_ledger_dedup() {
    // A temp dir holds the corpus + the ledger sidecar that opens beside it.
    let dir = std::env::temp_dir().join(format!("rb-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let corpus_path = dir.join("historical.db");
    write_corpus(&corpus_path);

    let reg = Registry::default();
    let platform = parse_platform("ps3").unwrap();
    let role = parse_role("nhlng").unwrap();
    let summary = reg
        .load_project(&s(repo(DB)), platform, role, &s(repo(META)), &s(repo(DEFS)))
        .unwrap();
    let id = summary.project_id;
    reg.open_corpus(corpus_path.to_str().unwrap()).unwrap();

    let team = reg.list_teams(id).unwrap().first().unwrap().team_id;
    let before = reg.list_players(id).unwrap().len();
    let items = vec![import_item_dto(team)];

    // Preview: one create, no blockers, not yet in the ledger.
    let preview = reg.import_preview(id, &items).unwrap();
    assert_eq!(preview.blocked, 0);
    assert_eq!(preview.players_after, preview.players_before + 1);
    assert_eq!(preview.items.len(), 1);
    assert_eq!(preview.items[0].outcome, "created");
    assert!(!preview.items[0].already_imported, "first import is not a dup");
    assert_eq!(reg.list_players(id).unwrap().len(), before, "preview did not mutate");

    // Commit: the create lands, and the result echoes the freshly-created wBIz.
    let result = reg.import_commit(id, &items).unwrap();
    assert_eq!(result.items[0].outcome, "created");
    let new_id = result.items[0].ea_player_id.expect("the created wBIz is surfaced");
    assert!(new_id != 0, "a real player id, not the 0 placeholder");
    let players = reg.list_players(id).unwrap();
    assert_eq!(players.len(), before + 1, "commit added the player");
    assert!(players.iter().any(|p| p.player_id == new_id), "the created id is a real player on the roster");

    // Re-preview the same cart: the ledger now flags this person-season as already imported.
    let again = reg.import_preview(id, &items).unwrap();
    assert!(
        again.items[0].already_imported,
        "the ledger surfaces the dedup warning after commit"
    );

    std::fs::remove_dir_all(&dir).ok();
}
