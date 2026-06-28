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
