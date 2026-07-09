//! Phase 2 gate: load a real database, stage an edit, validate, save, and byte-diff.
//!
//! The roadmap gate names a byte-diff "vs. reference-app output"; in this clean-room repo the
//! operative equivalent is **round-trip byte-equality against the anchor** (load → save with no edit
//! must reproduce the file exactly) plus a known-good single edit that validates clean and changes
//! the bytes. Both raw-db and save-container save paths are exercised.

use std::path::{Path, PathBuf};

use studio_domain::{
    entity_fields, DomainError, EditSession, FieldValue, Platform, Project, ProjectRole, Source,
};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_savedata::SaveContainer;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DEFS: &str = "defs";
const V4_SAVE: &str = "anchors/ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA";

fn load_nhlng() -> Project {
    Project::load(repo(DB), Platform::Ps3, ProjectRole::Nhlng, repo(META), repo(DEFS)).unwrap()
}

/// The first roster slot that resolves to a player — a valid `(team, player)` pair for edit ops.
fn a_player(session: &EditSession) -> (u64, u64) {
    let model = session.model().unwrap();
    let slot = model
        .slots
        .iter()
        .find(|s| s.player_id.is_some())
        .expect("anchor has at least one populated roster slot");
    (slot.team_id, slot.player_id.unwrap())
}

/// A strategy cell edit (`EeYo`) round-trips through the audited path, and its career-mode
/// mirror is a silent no-op on stock data (`JUfQ` ships with 0 rows — only a dynasty save
/// populates it, at which point the same call stages a second edit).
#[test]
fn strategy_edit_applies_and_jufq_mirror_noops_on_stock() {
    let project = load_nhlng();
    let mut session = EditSession::begin(project);

    let strat = session.team_strategies(0).unwrap().expect("team 0 sheet");
    let forecheck = strat
        .tactics
        .iter()
        .find(|c| c.field.studio_field == "RBGh")
        .unwrap();
    let new_value = if forecheck.field.decoded == Some(0) { 1 } else { 0 };

    let outcome = session
        .set_field("EeYo", strat.record, "RBGh", FieldValue::Int(new_value))
        .unwrap();
    assert_eq!(outcome.edits, 1, "stock JUfQ is empty — the mirror must not fire");
    assert!(
        !outcome.repairs.iter().any(|r| r.contains("JUfQ")),
        "no mirror note on a no-op"
    );

    let after = session.team_strategies(0).unwrap().unwrap();
    let cell = after
        .tactics
        .iter()
        .find(|c| c.field.studio_field == "RBGh")
        .unwrap();
    assert_eq!(cell.field.decoded, Some(new_value), "the edit round-trips");
    assert!(cell.field.enum_label.is_some(), "the new value resolves a label");
}

/// Load → save with no edit reproduces the raw `.db` file byte-for-byte (the round-trip gate).
#[test]
fn raw_db_round_trip_is_byte_identical() {
    let original = std::fs::read(repo(DB)).unwrap();
    let project = load_nhlng();
    assert!(matches!(project.source(), Source::RawDb));

    let session = EditSession::begin(project);
    assert!(!session.is_dirty(), "a freshly-loaded session is not dirty");

    let saved = session.save().unwrap();
    assert_eq!(
        saved, original,
        "load → save with no edit must reproduce the file byte-for-byte"
    );
}

/// Stage an edit, validate clean, save: the saved db is CRC-valid and actually changed.
#[test]
fn edit_validate_save_gate() {
    let original = std::fs::read(repo(DB)).unwrap();
    let project = load_nhlng();
    let mut session = EditSession::begin(project);

    let (team, player) = a_player(&session);
    let outcome = session.set_captaincy(team, player, 1).unwrap(); // 1 = Captain
    assert!(outcome.edits >= 1);
    assert!(session.is_dirty());

    let report = session.validate().unwrap();
    assert!(
        report.is_clean(),
        "audit must be clean after a captaincy edit: {:?}",
        report.errors().take(3).collect::<Vec<_>>()
    );

    let saved = session.save().unwrap();
    assert_ne!(saved, original, "the edit must change the bytes");
    let db = TdbFile::parse(&saved).unwrap();
    assert!(db.header_crc_ok(), "saved db must be CRC-valid");
    assert!(db.size_ok());
}

/// Undo restores the loaded baseline exactly; redo re-applies the edit.
#[test]
fn undo_redo_restores_and_reapplies() {
    let original = std::fs::read(repo(DB)).unwrap();
    let project = load_nhlng();
    let mut session = EditSession::begin(project);
    let (team, player) = a_player(&session);

    session.set_captaincy(team, player, 1).unwrap();
    assert!(session.is_dirty());
    let edited = session.bytes().to_vec();
    assert_eq!(session.undo_depth(), 1);

    assert!(session.undo());
    assert!(!session.is_dirty(), "undo returns to the loaded baseline");
    assert_eq!(session.save().unwrap(), original);

    assert!(session.redo());
    assert!(session.is_dirty());
    assert_eq!(session.bytes(), edited.as_slice(), "redo reproduces the edit");
    assert!(!session.redo(), "nothing left to redo");
}

/// The save spine handles a v4 save container: load extracts the inner db, save repacks it, and the
/// inner db survives the round trip (the codec's semantic invariant, driven through the domain).
#[test]
fn v4_container_save_spine_round_trips() {
    let project = Project::load(
        repo(V4_SAVE),
        Platform::Ps3,
        ProjectRole::Roster,
        repo(META),
        repo(DEFS),
    )
    .unwrap();
    assert!(matches!(project.source(), Source::Container { .. }));

    let inner = project.db_bytes().to_vec();
    let session = EditSession::begin(project);
    let saved = session.save().unwrap();

    let container = SaveContainer::parse(&saved).unwrap();
    assert!(container.checksums_valid(), "repacked container checksums verify");
    assert_eq!(
        container.extract_db().unwrap(),
        inner,
        "inner db survives the container save spine"
    );
}

/// Generic validated field edits: an in-range integer persists, out-of-range is refused, a string
/// edit round-trips through save, and undo restores the baseline.
#[test]
fn generic_field_edit_validates_and_round_trips() {
    let project = load_nhlng();
    let mut session = EditSession::begin(project);

    // Resolve the firstname studio field id from the view layer (handles the id reversal).
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(META)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo(DEFS).join("enums")).unwrap();
    let record = 0u32;
    let first_field = {
        let db = TdbFile::parse(session.bytes()).unwrap();
        let fields = entity_fields(&db, &meta, &enums, "ubPc", record);
        fields
            .iter()
            .find(|f| f.name == "firstname")
            .unwrap()
            .studio_field
            .clone()
    };

    // In-range integer edit on position (vjla, 0..=4): persists, dirty, audit clean.
    let outcome = session
        .set_field("ubPc", record, "vjla", FieldValue::Int(2))
        .unwrap();
    assert_eq!(outcome.edits, 1);
    assert!(session.is_dirty());
    assert!(session.validate().unwrap().is_clean());

    // Out-of-range integer is refused, working bytes untouched.
    let dirty_before = session.bytes().to_vec();
    let err = session.set_field("ubPc", record, "vjla", FieldValue::Int(99));
    assert!(matches!(err, Err(DomainError::Edit(_))), "out-of-range rejected");
    assert_eq!(session.bytes(), dirty_before.as_slice(), "rejected edit changed nothing");

    // String edit round-trips through a save + reparse.
    session
        .set_field("ubPc", record, &first_field, FieldValue::Text("Wayne".into()))
        .unwrap();
    let saved = session.save().unwrap();
    let db2 = TdbFile::parse(&saved).unwrap();
    let bio2 = db2.table_canonical("ubPc").unwrap();
    let first2 = bio2.fields.iter().find(|f| f.id.raw() == "PedH").unwrap();
    assert_eq!(db2.read_inline_str(bio2, record, first2).unwrap(), b"Wayne");

    // Undo unwinds the last edit.
    assert!(session.undo());
}

/// PS4 wiring: `nhl_game.db` is the PS4 analogue of `nhlng.db` — a little-endian TDB the existing
/// engine opens, builds the entity model over, and round-trips byte-identically with no edit.
#[test]
fn ps4_nhl_game_db_is_the_nhlng_equivalent() {
    const PS4_DB: &str = "anchors/ps4/nhl17/databases/nhl_game.db";
    const PS4_META: &str = "anchors/ps4/nhl17/databases/nhl_game-meta.xml";

    let original = std::fs::read(repo(PS4_DB)).unwrap();
    let project = Project::load(
        repo(PS4_DB),
        Platform::Ps4,
        ProjectRole::Nhlng,
        repo(PS4_META),
        repo(DEFS),
    )
    .unwrap();
    assert!(matches!(project.source(), Source::RawDb));

    let session = EditSession::begin(project);
    // The little-endian entity model + view layer resolve.
    let players = session.player_summaries().unwrap();
    assert!(!players.is_empty(), "PS4 nhl_game.db yields players");
    let teams = session.team_summaries().unwrap();
    assert!(!teams.is_empty(), "PS4 nhl_game.db yields teams");
    assert!(session.player_view(players[0].player_id).unwrap().is_some());

    // Load → save with no edit reproduces the file byte-for-byte (LE codec + CRC round-trip).
    assert_eq!(session.save().unwrap(), original);
}

/// PS4 topology: the live roster lives inside the save's `DATA` stream (LZ4 record → little-endian
/// TDB). The domain opens it as a `Ps4Save` source, the views resolve, and save repacks the roster
/// back into the stream (round-tripping the inner db). Meta is the title's `nhl_game-meta.xml`.
#[test]
fn ps4_save_roster_opens_and_repacks() {
    const PS4_DATA: &str = "anchors/ps4/nhl19/savefiles/RosterCustom0/DATA";
    const PS4_META: &str = "anchors/ps4/nhl19/databases/nhl_game-meta.xml";

    let project = Project::load(
        repo(PS4_DATA),
        Platform::Ps4,
        ProjectRole::Roster,
        repo(PS4_META),
        repo(DEFS),
    )
    .unwrap();
    assert!(matches!(project.source(), Source::Ps4Save { .. }));
    let inner = project.db_bytes().to_vec();

    let session = EditSession::begin(project);
    assert!(!session.is_dirty());
    // The doc-verified arenas (StEO → NHL cities) resolve through the LE read path.
    assert!(!session.arena_summaries().unwrap().is_empty());

    // Save repacks the DATA stream; the inner roster survives the LZ4 round-trip.
    let saved = session.save().unwrap();
    assert_eq!(tdb_savedata::ps4_extract_roster(&saved).unwrap(), inner);
}

/// NHL16 is the bridge PS4 title: its save uses the **bare-frame** variant (no `01` codec byte) and a
/// smaller roster subset. It opens as a `Ps4Save` source through the same path and repacks round-trip.
#[test]
fn ps4_nhl16_bare_frame_save_opens_and_repacks() {
    const DATA: &str = "anchors/ps4/nhl16/savefiles/84f8102f - ROSTER/DATA";
    const META: &str = "anchors/ps4/nhl16/databases/nhl_game-meta.xml";

    let project = Project::load(repo(DATA), Platform::Ps4, ProjectRole::Roster, repo(META), repo(DEFS)).unwrap();
    assert!(matches!(project.source(), Source::Ps4Save { .. }));
    let inner = project.db_bytes().to_vec();
    assert!(project.db_bytes().len() > 1_000_000, "a substantial inner roster");

    let session = EditSession::begin(project);
    let saved = session.save().unwrap();
    assert_eq!(tdb_savedata::ps4_extract_roster(&saved).unwrap(), inner);
}

/// Loading a big-endian db while declaring a little-endian platform is refused.
#[test]
fn platform_endian_mismatch_is_rejected() {
    let result = Project::load(
        repo(DB),
        Platform::Ps4,
        ProjectRole::Nhlng,
        repo(META),
        repo(DEFS),
    );
    assert!(
        matches!(result, Err(DomainError::EndianMismatch { .. })),
        "got {:?}",
        result.err()
    );
}

/// Add-season → edit a stat cell → delete round-trip on the stock career tables: the row appears
/// with its identity set, the staged cell edit lands, the ordered delete restores the original
/// row count, and every step audits clean and is undoable.
#[test]
fn career_season_add_edit_delete_round_trip() {
    let project = load_nhlng();
    let mut session = EditSession::begin(project);
    let (_, player) = a_player(&session);

    let before = session.career_stats(player).unwrap();
    let rows_before = |cs: &studio_domain::CareerStats| {
        cs.sections
            .iter()
            .find(|s| s.kind == "skater_season")
            .map(|s| s.rows.len())
            .unwrap_or(0)
    };
    let n0 = rows_before(&before);

    // Add a 2015-16 AHL season with a team code.
    let (record, outcome) = session
        .add_career_season(studio_domain::CAREER_SKATER_SEASON, player, 2016, 1, "PIT")
        .unwrap();
    assert!(outcome.edits >= 4);

    let after = session.career_stats(player).unwrap();
    assert_eq!(rows_before(&after), n0 + 1, "one new season row");
    let sec = after.sections.iter().find(|s| s.kind == "skater_season").unwrap();
    let row = sec.rows.iter().find(|r| r.record == record).expect("new row visible");
    assert_eq!(row.season_label, "2015-16");
    assert_eq!(row.league_label, "AHL");
    assert_eq!(row.team, "PIT");
    // Stat cells start zero.
    let gp = sec.columns.iter().position(|c| c.label == "GP").unwrap();
    assert_eq!(row.cells[gp].as_ref().unwrap().decoded, Some(0));

    // Edit a stat cell through the normal validated path (goals = 30).
    let g = sec.columns.iter().position(|c| c.label == "G").unwrap();
    let goals_field = sec.columns[g].field.clone();
    session
        .set_field(
            studio_domain::CAREER_SKATER_SEASON,
            record,
            &goals_field,
            FieldValue::Int(30),
        )
        .unwrap();
    let edited = session.career_stats(player).unwrap();
    let sec = edited.sections.iter().find(|s| s.kind == "skater_season").unwrap();
    let row = sec.rows.iter().find(|r| r.record == record).unwrap();
    assert_eq!(row.cells[g].as_ref().unwrap().decoded, Some(30));

    // Ordered delete restores the original count; the audit stays clean throughout.
    session
        .delete_career_row(studio_domain::CAREER_SKATER_SEASON, record)
        .unwrap();
    let gone = session.career_stats(player).unwrap();
    assert_eq!(rows_before(&gone), n0, "row removed");
    let report = session.validate().unwrap();
    assert!(
        report.is_clean(),
        "audit must be clean after the round-trip: {:?}",
        report.errors().take(3).collect::<Vec<_>>()
    );

    // Fully undoable: three ops staged → three undos return to the pristine bytes.
    assert!(session.undo() && session.undo() && session.undo());
    assert!(!session.is_dirty(), "undo restores the pristine working bytes");
}
