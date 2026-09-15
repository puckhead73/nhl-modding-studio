//! NHL Legacy, Xbox 360: semantic and capacity declarations checked against **this target's own
//! committed evidence** (package C0-REG1a-R1/R2; mechanics shared through `target_evidence` since
//! C0-REG1b, expectations unchanged).
//!
//! `snapshot_tests` proves that an adapter's physical declarations restate its anchor. It cannot
//! say what a code *means*, how many records a save directory allocates, or whether an edit through
//! a declared field behaves. Those are the facts this module measures, each on the exact file the
//! adapter attributes it to:
//!
//! * capacities on the committed Xbox 360 Legacy ROSTER fixture, decoded by the production save
//!   codec — not inferred from the PS3 audit across a main-database hash identity, which
//!   establishes main-DB identity and nothing about save directories;
//! * the position discriminator and the companion rule, with the expected labels taken from EA's
//!   own metadata table names (`exhibitionskaterai`, `exhibitiongoalieai`, …) — never from the enum
//!   file whose codes are under test;
//! * the main-team pointer's encoding, from the metadata-declared roster foreign key, with every
//!   rostered player's slot compared to what its bio row stores;
//! * `team.name` read / edit / serialize / reopen on an in-memory copy of the anchor;
//! * the sentinel → storage-decode → index-mapping processing order of `SENTINEL_ENCODING`,
//!   exercised over every `ubPc.dXSB` pattern on the anchor with a **test helper** — this crate
//!   ships no production executor of that order, and nothing here claims one.
//!
//! Each case ends by asserting that the adapter declares exactly what was measured, so the TOML
//! cannot drift from the measurement. What a case could NOT establish is stated in its doc comment
//! and stays a candidate in the adapter. Nothing here touches a file on disk, runs the game, or
//! promotes a qualification beyond `automated-tested` for the one behaviour actually exercised.

use super::*;
use crate::target_evidence::*;
use std::collections::{BTreeMap, BTreeSet};
use tdb_core::TdbFile;

const T: Target = Target {
    adapter_file: "nhllegacy-xbox360.toml",
    target: "nhllegacy-xbox360",
    cases_file: "crates/tdb-canonical/src/legacy_xbox360_tests.rs",
    platform: "xbox360",
    game: "nhllegacy",
    main_anchor: "anchors/xbox360/nhllegacy/databases/nhlng.db",
    main_anchor_sha256: "105d2186da3d1f0f3d3ea8a350ead42ac9d5d802ca9fd37652c39bb4e1f1c230",
    // The committed vanilla Xbox 360 Legacy ROSTER save (already outside its STFS wrapper: it starts
    // with the `RosterFile` container magic).
    roster_fixture:
        "anchors/xbox360/nhllegacy/savefiles/ROSTER 20260627060052/ROSTER 20260627060052",
    roster_fixture_sha256: "65a04268a2d1321ee495bd0c2d1509748c2d12ec3495a038d7a567f616ad7d21",
    roster_payload_sha256: "aeadd37cb4a74c39ec3972a161f83e5abf37cf3cf5c40c6eadad0160877796db",
};

/// As decoded by `tdb_savedata`: magic `RosterFile`, version word 4 with a zlib body at 0x30, the
/// declared inflate size exceeding `db_size` by the 4-byte trailer the extractor trims.
const FRAMING: RosterFraming = RosterFraming {
    outer_len: 2_456_120,
    magic: "RosterFile",
    version: tdb_savedata::ContainerVersion::ZlibV4,
    version_word: 4,
    uncompressed_size: Some(2_456_076),
    compressed_size: Some(722_116),
    inner_len: 2_456_072,
    table_count: 39,
};

// ============================================================== 1. capacity

/// Every capacity the adapter declares is the observed directory capacity of the committed Xbox 360
/// Legacy ROSTER fixture — `max_records` as stored, `current_records` as stored — read through the
/// production save codec and TDB parser.
///
/// ESTABLISHED: the fixture's per-table `max_records` / `current_records`, and that in this fixture
/// every roster-bearing table's population equals the main anchor's (an observation about these two
/// files, asserted as such, never assumed). NOT ESTABLISHED: that the game accepts growth up to
/// `max_records` — a directory allocation is not a demonstrated runtime limit.
#[test]
fn capacities_are_the_observed_directory_of_the_committed_roster_fixture() {
    let inner = T.decoded_roster_fixture(&FRAMING);
    let save = TdbFile::parse(&inner).unwrap();
    T.assert_capacities_are_the_fixtures(
        &save,
        &[
            ("ubPc", 7988, 5632),
            ("ZBac", 9955, 6087),
            ("eGlu", 10879, 5532),
            ("dSvy", 7331, 5066),
            ("xmja", 7331, 5066),
            ("mHuy", 957, 566),
            ("fMVl", 957, 566),
            ("kOtt", 252, 252),
        ],
        "capacities_are_the_observed_directory_of_the_committed_roster_fixture",
    );
}

/// The fixture check is load-bearing: a ceiling one off in either direction, a population that is
/// not the save's, a missing attribution, a capacity attributed to another file, or a table the
/// save does not carry are each reported by table. (The PS3 audit's figures happen to coincide with
/// this fixture's, so a value copied from there would not be caught here — it would be caught by
/// the provenance and source checks, which demand the fixture be named.)
#[test]
fn a_capacity_that_is_not_the_fixtures_is_reported() {
    let inner = T.decoded_roster_fixture(&FRAMING);
    let save = TdbFile::parse(&inner).unwrap();
    let base = T.adapter();
    assert!(T.roster_capacity_discrepancies(&base, &save).is_empty());
    let mutate = |table: &str, f: &dyn Fn(&mut CapacityDecl)| {
        let mut a = base.clone();
        f(a.capacities.iter_mut().find(|c| c.table == table).unwrap());
        T.roster_capacity_discrepancies(&a, &save)
    };

    let found = mutate("ubPc", &|c| {
        c.declared.value.as_mut().unwrap().ceiling_records = 7987
    });
    assert_eq!(
        found,
        vec!["capacity ubPc: ceiling_records 7987 vs fixture max_records 7988"]
    );

    let found = mutate("ubPc", &|c| {
        c.declared.value.as_mut().unwrap().ceiling_records = 8000
    });
    assert_eq!(
        found,
        vec!["capacity ubPc: ceiling_records 8000 vs fixture max_records 7988"]
    );

    let found = mutate("kOtt", &|c| {
        c.declared.value.as_mut().unwrap().population_records = 251
    });
    assert_eq!(
        found,
        vec!["capacity kOtt: population_records 251 vs fixture current_records 252"]
    );

    let found = mutate("eGlu", &|c| {
        c.declared.provenance.retain(|p| p != T.roster_fixture)
    });
    assert_eq!(
        found,
        vec!["capacity eGlu: provenance does not name the fixture it was read from"]
    );

    let found = mutate("dSvy", &|c| {
        c.declared.value.as_mut().unwrap().source = "main-db-max-records".into()
    });
    assert_eq!(
        found,
        vec![
            "capacity dSvy: source main-db-max-records is not the ROSTER fixture this case decodes"
        ]
    );

    let found = mutate("mHuy", &|c| c.table = "zzzz".into());
    assert_eq!(found, vec!["capacity zzzz: table not in the ROSTER save"]);
}

// ============================================================== 2. position

/// How this anchor tells skaters from goalies, measured: the bio rows that own rows in the
/// metadata-named goalie companion tables all store position code 4 and nothing else; the rows that
/// own skater companion rows store exactly the codes {0, 1, 2, 3}; the two sets partition the table.
///
/// ESTABLISHED: the discriminator (`ubPc.vjla`: skater codes 0..3, goalie code 4), the one-to-one
/// companion rule on `wBIz`, and therefore which bio rows the `dSvy` ratings mappings apply to.
/// NOT ESTABLISHED: what 0, 1, 2 and 3 mean individually (centre / wing / defence is not measured
/// here and is not declared by the adapter).
#[test]
fn position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do() {
    let main = T.main_anchor_bytes();
    let db = TdbFile::parse(&main).unwrap();
    let snap = T.main_snapshot();
    assert_eq!(
        meta_field(&snap, "ubPc", "vjla").name.as_deref(),
        Some("position")
    );
    let part = companion_partition(&db, &snap);
    assert_eq!((part.skaters.len(), part.goalies.len()), (5066, 566));
    let (skater_codes, goalie_codes) = position_codes_by_partition(&db, &part, 4);
    assert_eq!(skater_codes, BTreeSet::from([0, 1, 2, 3]));
    assert_eq!(goalie_codes, BTreeSet::from([4]));

    let a = T.adapter();
    assert_discriminator_declared(
        &T,
        &a,
        &[0, 1, 2, 3],
        &[4],
        &["player.rating_overall", "player.rating_speed"],
        "position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do",
    );
    assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
}

// ============================================================== 3. main team

/// The main-team pointer `ubPc.dXSB`, measured against the roster: for each of the 5110 rostered
/// players exactly one of its roster slots carries `team == dXSB - 1` and none carries
/// `team == dXSB`; all 522 un-rostered players store 0 and no rostered player does; the all-ones
/// code is stored nowhere.
///
/// ESTABLISHED: the encoding (stored pattern = roster team code + 1, i.e. offset-binary storage
/// over the 0-origin code space) and the absence code (stored pattern 0 = no main team).
/// NOT ESTABLISHED: that the team codes are `kOtt.vfEq` — every `dXSB - 1` and every slot team
/// code is contained in that key set (observed here), but containment plus a field name is not a
/// proven foreign key, so the parent stays a candidate and the mapping stays blocked on it.
#[test]
fn main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered() {
    let main = T.main_anchor_bytes();
    let db = TdbFile::parse(&main).unwrap();
    let snap = T.main_snapshot();
    let slots = roster_slots_by_bio_record(&db, &snap);
    let counts = main_team_measurement(&db, &snap, &slots, MainTeamEncoding::SlotTeamPlusOne);
    assert_eq!(
        counts,
        MainTeamCounts {
            rostered: 5110,
            plus_one: 5110,
            same: 0,
            unrostered_zero: 522,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 252,
        }
    );
    assert_main_team_declared(
        &T,
        &T.adapter(),
        "main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered",
        MainTeamEncoding::SlotTeamPlusOne,
    );
}

// ============================================================== 3b. birth date

/// The birth-date index origin, measured directly on this anchor rather than through the
/// production probe the mapping also cites: stored months span 0..=11 and days 0..=30, which a
/// 1-origin encoding could not produce (it would reach 12 and 31 and never store 0). The adapter's
/// `index-origin-0` is that measurement; the year is a raw count and is not index-shifted.
#[test]
fn birth_date_month_and_day_are_stored_zero_origin_on_this_anchor() {
    let main = T.main_anchor_bytes();
    let db = TdbFile::parse(&main).unwrap();
    let [year, month, day] = birth_date_ranges(&db, &T.main_snapshot());
    assert_eq!((month, day), ((0, 11), (0, 30)));
    assert!(year.0 >= 1 && year.1 <= 255);
    let a = T.adapter();
    let m = mapping(&a, "player.birth_date");
    assert_eq!(m.transform.established().unwrap().index_origin, Some(0));
    assert_eq!(m.qualification, "automated-tested");
}

// ============================================================== 4. handedness

/// Handedness on this anchor: the stored histogram is {0: 3781, 1: 1851}, the same counts
/// `defs/enums/handedness.toml` reports for the uncommitted `default.db` it was confirmed on.
/// Matching counts prove matching counts — not identical records, not file identity, not content
/// equivalence — and say nothing about which code is `left`: that needs a named player with an
/// independently supported hand, and this repository holds none. The mapping stays blocked; nothing
/// is promoted.
#[test]
fn handedness_counts_match_the_enum_files_reported_counts_but_no_label_is_established() {
    let main = T.main_anchor_bytes();
    let db = TdbFile::parse(&main).unwrap();
    let snap = T.main_snapshot();
    assert_eq!(
        meta_field(&snap, "ubPc", "GRkp").name.as_deref(),
        Some("handedness")
    );
    let bio = table(&db, "ubPc");
    let hand = field(&db, bio, "GRkp");
    let mut histogram: BTreeMap<u64, usize> = BTreeMap::new();
    for i in 0..u32::from(bio.current_records) {
        *histogram.entry(read(&db, bio, i, &hand)).or_default() += 1;
    }
    assert_eq!(histogram, BTreeMap::from([(0, 3781), (1, 1851)]));

    let enum_file =
        std::fs::read_to_string(repo_root().join("defs/enums/handedness.toml")).unwrap();
    assert!(enum_file.contains("Left = 3781") && enum_file.contains("Right = 1851"));

    let a = T.adapter();
    let m = mapping(&a, "player.shoots");
    assert_eq!(m.qualification, "blocked");
    assert!(transform_unresolved(m));
    assert_eq!(m.enumeration.state, ConstraintState::Unknown);
    let c = m
        .enumeration
        .candidate
        .as_ref()
        .expect("the proposal is kept");
    assert!(
        c.note.contains("3781") && c.note.contains("1851"),
        "the candidate records the observation and what it does not show"
    );
    assert!(
        c.note.contains("matching counts only"),
        "and claims no more than counts"
    );
    assert!(
        T.cites(
            &c.provenance,
            "handedness_counts_match_the_enum_files_reported_counts_but_no_label_is_established"
        ),
        "the candidate cites this case by its current name"
    );
}

// ============================================================== 4b. codec order

/// The processing order of `SENTINEL_ENCODING`, demonstrated on `player.team`'s declaration over
/// every stored `ubPc.dXSB` pattern on the anchor: pattern 0 is absence and is never handed to the
/// offset-binary decode (which would read it as -1); every non-zero pattern decodes, through the
/// storage regime and then the index-origin transform, to exactly the roster slot's team code that
/// the R1 measurement found; and the inverse restores the original pattern for all 5632 rows.
///
/// ESTABLISHED: codec behaviour of the declared blocks, in the stated order, with a test helper.
/// NOT ESTABLISHED: which table those team codes index — the parent stays a candidate and the
/// mapping stays blocked; this case neither reads `kOtt` nor claims a production executor.
#[test]
fn stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes() {
    let a = T.adapter();
    let m = mapping(&a, "player.team");
    assert_eq!(m.foreign_key.state, ConstraintState::Unknown);

    // Representative patterns first, by hand.
    assert_eq!(decode_stored(m, 0), Decoded::Absent);
    assert_eq!(decode_stored(m, 1), Decoded::TeamCode(0));
    assert_eq!(decode_stored(m, 84), Decoded::TeamCode(83));
    assert_eq!(decode_stored(m, 223), Decoded::TeamCode(222));
    assert_eq!(
        decode_stored(m, 255),
        Decoded::TeamCode(254),
        "the all-ones pattern is NOT this field's sentinel; it decodes like any other (unobserved on the anchor)"
    );
    assert_eq!(encode_stored(m, Decoded::Absent), 0);
    assert_eq!(encode_stored(m, Decoded::TeamCode(0)), 1);
    assert_eq!(encode_stored(m, Decoded::TeamCode(83)), 84);

    // Then every row of the anchor, against the roster measurement.
    let main = T.main_anchor_bytes();
    let db = TdbFile::parse(&main).unwrap();
    let slots = roster_slots_by_bio_record(&db, &T.main_snapshot());
    assert_eq!(codec_round_trip_over_anchor(m, &db, &slots), (522, 5110));
}

// ============================================================== 5. team.name

/// `team.name` through the declared field, on an in-memory copy of the anchor: read the vanilla
/// value, overwrite it through the production inline-string writer, recompute the CRC scopes,
/// reopen, and read it back. Every byte outside the edited field span and the one CRC word that
/// covers that table body is unchanged; a second no-op serialization of the reopened copy is
/// byte-identical; an over-budget value is refused before any byte moves.
///
/// ESTABLISHED: this read / edit / serialize / reopen behaviour of `kOtt.YmkJ` on this anchor.
/// NOT ESTABLISHED: that the game reads the name from this field (the semantics stay derived from
/// the metadata name), any UI path, and any save-container round trip.
#[test]
fn team_name_edit_round_trips_on_a_disposable_copy() {
    let original = T.main_anchor_bytes();
    // `®` keeps the multi-byte case in the round trip: the ® is two UTF-8 bytes.
    team_name_round_trip(
        &original,
        352,
        "Anaheim Ducks®",
        "Winnipeg Jets™",
        "Mighty Ducks of Anaheim®",
    );
    assert_team_name_declared(
        &T,
        &T.adapter(),
        "legacy_xbox360_tests",
        "team_name_edit_round_trips_on_a_disposable_copy",
    );
}

// ============================================================== 6. the ledger

/// What this module promoted and what it left, counted, so the receipt's figures are checked.
#[test]
fn the_adapters_population_reflects_exactly_these_measurements() {
    let a = T.adapter();
    let population = registry_population(std::iter::once(&a));
    assert_eq!(population.len(), 1);
    let p = &population[0];
    assert_eq!(p.mappings, 8);
    assert_eq!(p.omissions, 0);
    assert_eq!(
        (
            p.established_constraints,
            p.unknown_constraints,
            p.candidate_constraints
        ),
        (44, 30, 6),
        "established / unknown / candidate blocks after C0-REG1a-R1 (C0-REG1a: 37 / 37 / 12); \
         the unknown count includes every block a mapping does not declare at all"
    );
    // The six candidates — unknowns that carry a recorded, non-operative proposal — by name: shoots
    // (transform, enumeration, null_sentinel), the two ratings' null_sentinel, and the main-team
    // parent. Everything promoted by this package has left this list.
    assert_eq!(
        candidate_blocks(&a),
        [
            "player.shoots.transform",
            "player.shoots.enumeration",
            "player.shoots.null_sentinel",
            "player.rating_overall.null_sentinel",
            "player.rating_speed.null_sentinel",
            "player.team.foreign_key",
        ]
    );
    assert_eq!(
        mapped_fraction(&a),
        Some((10, 2965)),
        "coverage is unchanged: no new field is mapped"
    );
}
