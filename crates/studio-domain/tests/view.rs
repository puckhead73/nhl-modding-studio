//! Phase 3.1 — view-model foundation verified against the NHL Legacy anchor: strings read, enum
//! labels resolve, sign-decoding stays in the metadata's legal range.

use std::path::{Path, PathBuf};

use studio_domain::{
    arena_summaries, arena_view, entity_fields, lines_view, player_summaries, player_view,
    team_summaries, team_view, FieldKind,
};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::RosterModel;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DEFS: &str = "defs";

fn fixtures() -> (Vec<u8>, FieldMetaIndex, EnumRegistry) {
    let bytes = std::fs::read(repo(DB)).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(META)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo(DEFS).join("enums")).unwrap();
    (bytes, meta, enums)
}

/// Inline name strings read, the position field resolves to an enum label, and `firstname` is
/// reported editable (its metadata is updatable).
#[test]
fn ubpc_fields_decode_strings_and_enums() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let bio = db.table_canonical("ubPc").unwrap();
    let n = (bio.current_records as u32).min(300);

    let mut found_name = false;
    let mut found_pos_label = false;
    let mut checked_editable = false;

    for i in 0..n {
        let fields = entity_fields(&db, &meta, &enums, "ubPc", i);
        assert!(!fields.is_empty(), "ubPc record {i} has fields");

        if let Some(last) = fields.iter().find(|f| f.name == "lastname") {
            assert_eq!(last.kind, FieldKind::String);
            assert!(last.is_string);
            if !last.display.is_empty() {
                found_name = true;
            }
        }
        if let Some(pos) = fields.iter().find(|f| f.name == "position") {
            assert_eq!(pos.kind, FieldKind::Integer);
            if let Some(label) = &pos.enum_label {
                assert!(!label.is_empty(), "position enum label non-empty");
                found_pos_label = true;
            }
        }
        if let Some(first) = fields.iter().find(|f| f.name == "firstname") {
            assert!(first.editable, "firstname is updatable per metadata");
            checked_editable = true;
        }
    }

    assert!(found_name, "no non-empty lastname found in first {n} bios");
    assert!(found_pos_label, "position never produced an enum label");
    assert!(checked_editable, "firstname field was present and checked");
}

/// Every integer field with a declared range decodes within that range on vanilla anchor data —
/// proves sign/sentinel decoding (`FieldMeta::decode_raw`) against the metadata across a whole table.
#[test]
fn integer_fields_decode_within_declared_range() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();

    // The team table is small and fully populated — a tight, fast invariant check.
    let team = db.table_canonical("kOtt").unwrap();
    let n = team.current_records as u32;
    assert!(n > 0, "kOtt has records");

    for i in 0..n {
        for fv in entity_fields(&db, &meta, &enums, "kOtt", i) {
            if let (Some(d), Some(lo), Some(hi)) = (fv.decoded, fv.range_low, fv.range_high) {
                // Skip degenerate metadata ranges (rangelow==rangehigh==0 marks "unconstrained").
                if lo == 0 && hi == 0 {
                    continue;
                }
                assert!(
                    d >= lo && d <= hi,
                    "kOtt rec {i} field {} ({}) decoded {d} outside [{lo}, {hi}]",
                    fv.studio_field,
                    fv.name
                );
            }
        }
    }
}

/// The player master list covers every modelled player and carries readable names + position labels.
#[test]
fn player_summaries_cover_the_model() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let summaries = player_summaries(&db, &meta, &enums, &model);
    assert_eq!(summaries.len(), model.players.len(), "one row per player");

    let named = summaries.iter().filter(|s| !s.last_name.is_empty()).count();
    assert!(named > summaries.len() / 2, "most players have a last name ({named}/{})", summaries.len());
    assert!(
        summaries.iter().any(|s| !s.position_label.is_empty()),
        "position labels resolve"
    );
}

/// The Player detail assembles curated identity, the kind-correct ratings companion, memberships,
/// and the full bio field list.
#[test]
fn player_view_assembles_detail() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    // A skater that has a roster slot — exercises ratings + memberships.
    let pid = model
        .slots
        .iter()
        .filter_map(|s| s.player_id)
        .find(|&id| model.player(id).is_some_and(|p| p.skater_ai.is_some()))
        .expect("a slotted skater exists");

    let view = player_view(&db, &meta, &enums, &model, pid).expect("player resolves");
    assert_eq!(view.kind, "skater");
    assert!(view.identity.iter().any(|f| f.name == "position"));
    assert!(view.identity.iter().any(|f| f.name == "firstname"));
    assert!(!view.ratings.is_empty(), "skater has rating fields (dSvy)");
    assert!(!view.all_fields.is_empty(), "bio all-fields populated");
    assert!(!view.memberships.is_empty(), "slotted player has a membership");

    assert!(player_view(&db, &meta, &enums, &model, 0xFFFF_FFFF).is_none());
}

/// Team summaries cover the model with readable names; the detail resolves identity, arena, roster.
#[test]
fn team_views_resolve_identity_and_arena() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let teams = team_summaries(&db, &meta, &model);
    assert_eq!(teams.len(), model.teams.len());
    assert!(teams.iter().any(|t| !t.name.is_empty()), "teams have names");
    assert!(teams.iter().any(|t| !t.abbrev.is_empty()), "teams have abbreviations");

    let view = team_view(&db, &meta, &enums, &model, teams[0].team_id).expect("team resolves");
    assert!(view.identity.iter().any(|f| f.name == "fullname"));
    assert_eq!(view.roster_count, model.team_roster(teams[0].team_id).len());
    // At least one team in the league resolves its home arena via kOtt.buDa -> StEO.vfEq.
    assert!(
        teams
            .iter()
            .filter_map(|t| team_view(&db, &meta, &enums, &model, t.team_id))
            .any(|v| v.arena.is_some()),
        "at least one team resolves an arena"
    );

    assert!(team_view(&db, &meta, &enums, &model, 0xFFFF_FFFF).is_none());
}

/// Arena summaries list the rinks; the detail resolves identity (name/city).
#[test]
fn arena_views_resolve() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();

    let arenas = arena_summaries(&db, &meta);
    assert!(!arenas.is_empty(), "anchor has arenas");
    assert!(arenas.iter().any(|a| !a.name.is_empty()), "arenas have names");

    let view = arena_view(&db, &meta, &enums, arenas[0].arena_id).expect("arena resolves");
    assert!(view.identity.iter().any(|f| f.name == "arenaname"));
    assert!(!view.all_fields.is_empty());
}

/// The depth chart lists a team's roster with resolved names and situational slot labels.
#[test]
fn lines_view_builds_depth_chart() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    // A team that actually has a roster.
    let team_id = model
        .teams
        .iter()
        .map(|t| t.team_id)
        .find(|&id| !model.team_roster(id).is_empty())
        .expect("a team with a roster");

    let lines = lines_view(&db, &meta, &enums, &model, team_id).expect("lines resolve");
    assert_eq!(lines.entries.len(), model.team_roster(team_id).len());
    assert!(lines.entries.iter().any(|e| !e.name.is_empty()), "names resolve");
    // Situational flag labels are metadata names (e.g. line/pp/pk slots) when set.
    assert!(
        lines.entries.iter().flat_map(|e| &e.slots).any(|s| !s.is_empty()),
        "at least one roster entry has a labelled situational slot"
    );

    assert!(lines_view(&db, &meta, &enums, &model, 0xFFFF_FFFF).is_none());
}
