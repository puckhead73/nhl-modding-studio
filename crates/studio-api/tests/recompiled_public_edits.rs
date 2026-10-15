//! Receipt 62 — a **Legacy Recompiled** project opened with the catalogue id `legacyrecomp` (app/src/lib/catalog.ts) through the
//! public Registry path edits its title-encoded player cells: weight, a six-bit rating, birth day and contract dollars — the
//! cells receipt 59 found refused. Display, pending review, apply, undo / redo, save to a copy and reopen, and the Roster
//! Builder import seam; a wrong declared title refuses before any mutation.
//!
//! Source: the registered install (`H:\Emulators\games\XBOX\NHL Legacy Decomp Test`, read only). Discovery is observed on the
//! install root and on its `game` folder; the database is obtained the way the app does it (`extract_install`), but on an
//! owned scratch copy of `game\cacheboot.big`, never in place. The extracted database's SHA-256 is checked against the X360
//! Legacy anchor research R128 recorded; nothing about Recompiled roster-save ceilings, save sidecars or archive precedence is
//! inferred here.
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use studio_api::{parse_platform, Registry};
use studio_domain::{FieldValue, Platform};
use tdb_fhm::schema::{SCHEMA_SQL, SCHEMA_VERSION};
use tdb_fhm::slug::person_slug;

const INSTALL: &str = r"H:\Emulators\games\XBOX\NHL Legacy Decomp Test";
/// The X360 Legacy anchor database (R128: Recompiled's `cacheboot.big:db/nhlng.db` is these bytes).
const ANCHOR_SHA256: &str = "105d2186da3d1f0f";

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Owned scratch, removed on drop.
struct Tmp(PathBuf);
impl Tmp {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-recomp-edits-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("game")).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

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
    )
    .unwrap();
    conn.execute("INSERT INTO player_season(person_id, season_year, src_player_id, team_id, height, weight) VALUES (1, 2024, 10, -1, 72, 200)", []).unwrap();
    conn.execute("INSERT INTO ratings_season(person_id, season_year, c, ability, potential, attrs_json) VALUES (1, 2024, 80, 80.0, 80.0, '{}')", []).unwrap();
}

fn field<'a>(v: &'a [studio_api::dto::FieldViewDto], id: &str) -> &'a studio_api::dto::FieldViewDto {
    v.iter().find(|f| f.studio_field == id).unwrap_or_else(|| panic!("no field {id}"))
}

/// The four title-encoded cells of a skater: (table, record, field id) for weight, birth day, contract dollars and one rating.
fn encoded_cells(v: &studio_api::dto::PlayerViewDto) -> Vec<(String, u32, String)> {
    let contract = v.all_fields.iter().find(|f| f.name == "contractdollars").expect("contractdollars");
    let rating = v.ratings.iter().find(|f| f.edit_offset == Some(36)).expect("a six-bit rating shown on the 36 offset");
    vec![
        ("ubPc".into(), v.bio_record, "sNZW".into()),
        ("ubPc".into(), v.bio_record, "Kswi".into()),
        ("ubPc".into(), v.bio_record, contract.studio_field.clone()),
        (v.ratings_table.clone(), v.ratings_record.expect("ratings record"), rating.studio_field.clone()),
    ]
}

fn stored(reg: &Registry, id: u32, player: u64, cell: &(String, u32, String)) -> i64 {
    let v = reg.player_view(id, player).unwrap().unwrap();
    let all: Vec<&studio_api::dto::FieldViewDto> = v.all_fields.iter().chain(v.ratings.iter()).collect();
    all.iter().find(|f| f.studio_table == cell.0 && f.studio_field == cell.2).and_then(|f| f.decoded).unwrap_or_else(|| panic!("{cell:?}"))
}

#[test]
#[ignore = "the registered Legacy Recompiled install under H:/Emulators/games/XBOX (read only): select with --ignored"]
fn a_legacyrecomp_project_edits_its_encoded_player_cells_through_the_public_path() {
    let install = PathBuf::from(INSTALL);
    assert!(install.join("game").join("cacheboot.big").is_file(), "{INSTALL}: missing (fails, never skips)");
    // Discovery on the registered install and on its `game` folder: no loose database; the archives are what it has.
    for root in [install.clone(), install.join("game")] {
        let r = studio_domain::discovery::detect_install("legacyrecomp", Platform::Recomp, &root, None);
        assert!(r.db_paths.nhlng.is_none() && r.extract_needed, "{}: {:?}", root.display(), r.db_paths.nhlng);
    }
    // The app's route, on an owned copy: extract `cacheboot.big` in place in scratch.
    let t = Tmp::new();
    std::fs::copy(install.join("game").join("cacheboot.big"), t.0.join("game").join("cacheboot.big")).unwrap();
    let summary = studio_domain::extract::extract_install(&t.0).unwrap();
    let found = studio_domain::discovery::detect_install("legacyrecomp", Platform::Recomp, &t.0, None);
    let db = PathBuf::from(found.db_paths.nhlng.clone().expect("the extracted main database"));
    use sha2::Digest as _;
    let sha = format!("{:x}", sha2::Sha256::digest(std::fs::read(&db).unwrap()));
    assert!(sha.starts_with(ANCHOR_SHA256), "the registered source's db/nhlng.db is the recorded one: {sha}");
    // Metadata: the extracted one when the archive carries it, otherwise the X360 Legacy anchor's (same bytes, R128).
    let meta = found.db_paths.meta_xml.clone().map(PathBuf::from).unwrap_or_else(|| repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml"));
    eprintln!("extracted {} files from cacheboot.big; db {} (sha {}…); meta {}", summary.files_written, db.display(), &sha[..16], meta.display());

    let reg = Registry::default();
    let recomp = parse_platform("recomp").unwrap();
    let open = |game: &str, path: &Path| reg.open_workspace_from_paths(Some(game), recomp, &s(path), None, &s(&meta), &s(&repo("defs"))).unwrap().project_id;
    let id = open("legacyrecomp", &db);

    // Display: the four cells are editable on their encodings (receipt 59's refusal gone).
    let player = reg.list_players(id).unwrap().into_iter().find(|p| !p.is_goalie).unwrap().player_id;
    let v = reg.player_view(id, player).unwrap().unwrap();
    assert!(field(&v.all_fields, "sNZW").editable && field(&v.all_fields, "sNZW").edit_offset.is_some(), "weight editable on its base");
    assert!(v.header.weight.is_some() && v.header.birthdate.is_some(), "header composes weight and birth date");
    let cells = encoded_cells(&v);
    for c in &cells {
        let all: Vec<&studio_api::dto::FieldViewDto> = v.all_fields.iter().chain(v.ratings.iter()).collect();
        let f = all.iter().find(|f| f.studio_table == c.0 && f.studio_field == c.2).unwrap();
        assert!(f.editable, "{} editable", f.name);
    }
    let before: Vec<i64> = cells.iter().map(|c| stored(&reg, id, player, c)).collect();
    // A new stored value inside each field's declared range.
    let after: Vec<i64> = cells
        .iter()
        .zip(&before)
        .map(|(c, b)| {
            let all: Vec<&studio_api::dto::FieldViewDto> = v.all_fields.iter().chain(v.ratings.iter()).collect();
            let f = all.iter().find(|f| f.studio_table == c.0 && f.studio_field == c.2).unwrap();
            let hi = f.range_high.unwrap_or(i64::MAX);
            if *b < hi { b + 1 } else { b - 1 }
        })
        .collect();

    // Apply, one operation per cell; each is a pending cell.
    for (c, a) in cells.iter().zip(&after) {
        reg.set_field(id, &c.0, c.1, &c.2, FieldValue::Int(*a)).unwrap_or_else(|e| panic!("{c:?}: {e}"));
    }
    let review = reg.pending_changes(id, 200).unwrap();
    assert!(review.dirty && review.primary.total_cells >= 4, "{} pending cells", review.primary.total_cells);
    assert_eq!(cells.iter().map(|c| stored(&reg, id, player, c)).collect::<Vec<_>>(), after);
    // Undo / redo, one step per cell.
    for _ in 0..4 {
        assert!(reg.undo(id).unwrap());
    }
    assert_eq!(cells.iter().map(|c| stored(&reg, id, player, c)).collect::<Vec<_>>(), before, "undo restores");
    for _ in 0..4 {
        assert!(reg.redo(id).unwrap());
    }
    assert_eq!(cells.iter().map(|c| stored(&reg, id, player, c)).collect::<Vec<_>>(), after, "redo re-applies");
    // Save to a copy and reopen as the same title.
    let out = t.0.join("saved-nhlng.db");
    reg.save(id, &s(&out)).unwrap();
    let id2 = open("legacyrecomp", &out);
    assert_eq!(cells.iter().map(|c| stored(&reg, id2, player, c)).collect::<Vec<_>>(), after, "the values survive save / reopen");

    // Import seam: the Roster Builder lowers and commits one created player on this project.
    let corpus = t.0.join("historical.db");
    write_corpus(&corpus);
    reg.open_corpus(&s(&corpus)).unwrap();
    let team = reg.list_teams(id2).unwrap().first().unwrap().team_id;
    let items = vec![studio_api::ImportItemDto { person_id: 1, season_year: 2024, target_team_id: team, match_ea_player_id: None, jersey: Some(99) }];
    let n0 = reg.list_players(id2).unwrap().len();
    let pv = reg.import_preview(id2, &items).unwrap();
    assert_eq!((pv.blocked, pv.players_after), (0, pv.players_before + 1), "the import lowers on legacyrecomp");
    let res = reg.import_commit(id2, &items).unwrap();
    assert_eq!(res.items[0].outcome, "created");
    assert_eq!(reg.list_players(id2).unwrap().len(), n0 + 1);
    assert!(reg.undo(id2).unwrap());
    assert_eq!(reg.list_players(id2).unwrap().len(), n0, "one undo step removes the import");

    // A wrong declared title (NHL 09 on the Recompiled platform) refuses before mutation, cell and import alike.
    let bad = open("nhl09", &db);
    let c = &cells[0];
    let b0 = stored(&reg, bad, player, c);
    let e = match reg.set_field(bad, &c.0, c.1, &c.2, FieldValue::Int(after[0])) {
        Ok(_) => panic!("a wrong declared title must refuse"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("not editable"), "{e}");
    assert_eq!(stored(&reg, bad, player, c), b0);
    assert!(!reg.pending_changes(bad, 10).unwrap().dirty, "nothing pending after the refusal");
    let e = match reg.import_preview(bad, &items) {
        Ok(_) => panic!("the import must refuse on a wrong declared title"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("cannot lower the import"), "{e}");
    // The older spelling stays accepted for compatibility.
    let alias = open("nhllegacy-recompiled", &db);
    reg.set_field(alias, &c.0, c.1, &c.2, FieldValue::Int(after[0])).unwrap();
}
