//! NHL 15, PlayStation 3: semantic and capacity declarations checked against **this target's own
//! committed evidence** (package C0-REG1b), through the mechanics of `target_evidence` with this
//! target's identities and this target's expectations.
//!
//! What is measured here, each on the exact file the adapter attributes it to:
//!
//! * capacities on the committed PS3 NHL 15 ROSTER fixture (`BLUS314360200 - ROSTER/SYS-DATA`),
//!   decoded by the production save codec — the fixture's own directory, not the roster-capacity
//!   audit's table and not anything carried across the main-DB hash identity with Xbox 360;
//! * the PS3 / Xbox 360 main-DB and metadata identity itself, asserted **directly** on the bytes as
//!   an observation, with nothing built on it;
//! * birth-date index origin, from the stored value ranges;
//! * the position discriminator and companion rule, labels from EA's metadata table names;
//! * the main-team pointer's encoding and absence pattern, from the metadata-declared roster key;
//! * `team.name` read / edit / serialize / reopen on an in-memory copy;
//! * the sentinel → storage-decode → index-mapping order over every `ubPc.dXSB` pattern, with the
//!   **test helper** of `target_evidence` — no production adapter executor exists or is claimed.
//!
//! Each case ends by asserting that the adapter declares exactly what was measured. What a case
//! could NOT establish stays a candidate: handedness (no independently supported label here) and the
//! main-team parent table (containment plus a name is not a foreign key).

use super::*;
use crate::target_evidence::*;
use std::collections::{BTreeMap, BTreeSet};
use tdb_core::TdbFile;

const T: Target = Target {
    adapter_file: "nhl15-ps3.toml",
    target: "nhl15-ps3",
    cases_file: "crates/tdb-canonical/src/nhl15_ps3_tests.rs",
    platform: "ps3",
    game: "nhl15",
    main_anchor: "anchors/ps3/nhl15/databases/nhlng.db",
    main_anchor_sha256: "028ae06ad2b25bf59aae37249b96144add014664771403e0313b5e3054b03398",
    // The committed vanilla PS3 NHL 15 ROSTER save, as extracted from its PS3 save directory
    // (`SYS-DATA`; the `PS3RosterFile` container magic is at byte 0).
    roster_fixture: "anchors/ps3/nhl15/savefiles/BLUS314360200 - ROSTER/SYS-DATA",
    roster_fixture_sha256: "fd234dc030586fb1366ebace47cf805cfd1aa38ac49cf1f9d52783b3d3a2f98c",
    roster_payload_sha256: "362c40ded18e17f3e412557d5a141096c819684138440c8facfc443fdf974d1e",
};

/// As decoded by `tdb_savedata`: magic `PS3RosterFile`, **version word 2** with a zlib body at 0x30
/// (the discriminator resolves it to the `ZlibV4` framing by the body marker, not the word — the
/// Xbox 360 Legacy fixture carries word 4 for the same framing), the declared inflate size exceeding
/// `db_size` by the 4-byte trailer the extractor trims.
const FRAMING: RosterFraming = RosterFraming {
    outer_len: 2_259_432,
    magic: "PS3RosterFile",
    version_word: 2,
    uncompressed_size: 2_259_388,
    compressed_size: 704_009,
    inner_len: 2_259_384,
    table_count: 38,
};

const MAIN_META: &str = "anchors/ps3/nhl15/databases/nhlng-meta.xml";
const MAIN_META_SHA256: &str = "bc0588daa82306c28f9129b0a33f6d5f46b80c4824f825be81b8a0a3139cb4de";

// ============================================================== 0. identity

/// The PS3 and Xbox 360 NHL 15 main databases and their metadata are byte-identical — asserted on
/// the committed bytes, not taken from a manifest. Recorded as an observation about four files.
/// Nothing in the adapter rests on it: the capacities below come from the PS3 save, the semantics
/// from the PS3 anchor, and the adapter cites the PS3 snapshot, which is a separate rendering.
#[test]
fn the_ps3_main_db_and_metadata_are_byte_identical_to_xbox360s_and_nothing_rests_on_it() {
    let ps3 = T.main_anchor_bytes();
    let x360 = std::fs::read(repo_root().join("anchors/xbox360/nhl15/databases/nhlng.db")).unwrap();
    assert_eq!(ps3, x360, "main DB");
    let ps3_meta = read_pinned(MAIN_META, MAIN_META_SHA256);
    let x360_meta =
        std::fs::read(repo_root().join("anchors/xbox360/nhl15/databases/nhlng-meta.xml")).unwrap();
    assert_eq!(ps3_meta, x360_meta, "metadata");

    // The adapter cites only PS3 files; no Xbox 360 path or the Xbox 360 adapter appears anywhere.
    let a = T.adapter();
    let text =
        std::fs::read_to_string(repo_root().join("defs/adapters").join(T.adapter_file)).unwrap();
    assert_eq!(
        a.physical_snapshot.as_deref(),
        Some("docs/audits/data/schema/ps3-nhl15.json")
    );
    assert!(
        !text.contains("anchors/xbox360/"),
        "no Xbox 360 anchor is cited"
    );
    assert!(
        !text.contains("xbox360-nhl15.json"),
        "the Xbox 360 rendering is not cited"
    );
    assert!(
        text.contains("carries NO save capacity, deployment behaviour or runtime conclusion"),
        "the identity's limits are stated where the identity is"
    );
}

// ============================================================== 1. capacity

/// Every capacity the adapter declares is the observed directory capacity of the committed PS3
/// NHL 15 ROSTER fixture — `max_records` as stored, `current_records` as stored — read through the
/// production save codec and TDB parser.
///
/// ESTABLISHED: the fixture's per-table `max_records` / `current_records`, and that in this fixture
/// every table's population equals the main anchor's (an observation about these two files,
/// asserted as such, never assumed). NOT ESTABLISHED: that the game accepts growth up to
/// `max_records` — a directory allocation is not a demonstrated runtime limit.
#[test]
fn capacities_are_the_observed_directory_of_the_committed_roster_fixture() {
    let inner = T.decoded_roster_fixture(&FRAMING);
    let save = TdbFile::parse(&inner).unwrap();
    T.assert_capacities_are_the_fixtures(
        &save,
        &[
            ("ubPc", 7034, 5534),
            ("ZBac", 8948, 5942),
            ("eGlu", 10619, 5511),
            ("dSvy", 6479, 4978),
            ("xmja", 6479, 4978),
            ("mHuy", 855, 556),
            ("fMVl", 855, 556),
            ("kOtt", 246, 246),
        ],
        "capacities_are_the_observed_directory_of_the_committed_roster_fixture",
    );
}

/// Target-specific rejection: the seven figures the NHL 15 **Xbox 360** adapter carried from the
/// roster-capacity audit coincide with this fixture's — the audit measured this very save — so the
/// numbers alone cannot tell "measured on the PS3 save" from "inherited"; the provenance check is
/// what does, and it reports every carried block. The team table, which the audit never covered,
/// exists only here. (The generic mutations — one off, wrong source, missing attribution, absent
/// table — are exercised on the Legacy module and are not duplicated here.)
#[test]
fn a_capacity_carried_from_the_xbox360_adapter_is_reported_against_this_fixture() {
    let inner = T.decoded_roster_fixture(&FRAMING);
    let save = TdbFile::parse(&inner).unwrap();
    let base = T.adapter();
    assert!(T.roster_capacity_discrepancies(&base, &save).is_empty());

    let x360 = {
        let text =
            std::fs::read_to_string(repo_root().join("defs/adapters/nhl15-xbox360.toml")).unwrap();
        parse_adapter(&text).unwrap()
    };
    let mut carried = base.clone();
    let mut differing = 0;
    for cap in &mut carried.capacities {
        let Some(from) = x360
            .capacities
            .iter()
            .find(|c| c.table == cap.table)
            .and_then(|c| c.declared.established())
        else {
            continue;
        };
        {
            let mine = cap.declared.value.as_mut().unwrap();
            if (from.ceiling_records, from.population_records)
                != (mine.ceiling_records, mine.population_records)
            {
                differing += 1;
            }
            mine.ceiling_records = from.ceiling_records;
            mine.population_records = from.population_records;
        }
    }
    // The PS3 audit measured the very same save, so the seven roster-bearing figures coincide with
    // what the Xbox 360 adapter carried from that audit: the numbers alone cannot tell the two
    // apart. Provenance can, and does — the Xbox 360 adapter's blocks do not name this fixture.
    assert_eq!(differing, 0);
    let mut relabelled = carried.clone();
    for cap in &mut relabelled.capacities {
        if let Some(src) = x360
            .capacities
            .iter()
            .find(|c| c.table == cap.table && c.declared.established().is_some())
        {
            cap.declared.provenance = src.declared.provenance.clone();
        }
    }
    let found = T.roster_capacity_discrepancies(&relabelled, &save);
    assert_eq!(found.len(), 7, "{found:?}");
    assert!(found
        .iter()
        .all(|f| f.ends_with("provenance does not name the fixture it was read from")));
    // And the team table — which the audit never covered — is this fixture's alone: the Xbox 360
    // adapter has no established figure for it.
    assert!(x360
        .capacities
        .iter()
        .all(|c| c.table != "kOtt" || c.declared.established().is_none()));
    assert_eq!(save.table_canonical("kOtt").unwrap().max_records, 246);
}

// ============================================================== 2. birth date

/// The birth-date index origin, measured directly on this anchor: stored months span 0..=11 and
/// days 0..=30, which a 1-origin encoding could not produce. The adapter's `index-origin-0` is that
/// measurement, corroborated by the named `studio-domain` case that lists this anchor by path.
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
    let case_file = std::fs::read_to_string(
        repo_root().join("crates/studio-domain/tests/birthdate_encoding.rs"),
    )
    .unwrap();
    assert!(
        case_file.contains(T.main_anchor),
        "the named case lists this anchor by path"
    );
    // The component offsets are this anchor's (Legacy's day/month sit at 958/975).
    let offset = |role: &str| {
        m.components
            .iter()
            .find(|c| c.role == role)
            .unwrap()
            .storage
            .established()
            .unwrap()
            .bit_offset
    };
    assert_eq!(
        (offset("day"), offset("month"), offset("year")),
        (959, 977, 816)
    );
}

// ============================================================== 3. position

/// The discriminator, measured with EA's metadata names as the labels: the 556 bio rows that own
/// rows in the goalie companion tables all store 4 and nothing else; the 4978 that own skater
/// companion rows store exactly {0, 1, 2, 3}; the two sets partition the 5534-row table.
///
/// ESTABLISHED: discriminator, one-to-one companion rule, ratings applicability. NOT ESTABLISHED:
/// what 0..3 mean individually.
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
    assert_eq!((part.skaters.len(), part.goalies.len()), (4978, 556));
    let (skater_codes, goalie_codes) = position_codes_by_partition(&db, &part, 4);
    assert_eq!(skater_codes, BTreeSet::from([0, 1, 2, 3]));
    assert_eq!(goalie_codes, BTreeSet::from([4]));

    let a = T.adapter();
    assert_discriminator_declared(
        &T,
        &a,
        &[0, 1, 2, 3],
        &[4],
        "position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do",
    );
    assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
}

// ============================================================== 4. main team

/// The main-team pointer `ubPc.dXSB` against the roster reached through the metadata-declared key:
/// for each of the 5088 rostered players exactly one slot carries `team == pattern - 1` and none
/// `team == pattern`; all 446 un-rostered players store 0; the all-ones pattern is stored nowhere.
///
/// ESTABLISHED: the encoding (offset-binary storage over the 0-origin roster code space) and the
/// absence pattern (stored 0). NOT ESTABLISHED: that the codes index `kOtt.vfEq` — containment in
/// its 246 keys is observed, and is not a foreign key; the mapping stays blocked on the parent.
#[test]
fn main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered() {
    let main = T.main_anchor_bytes();
    let db = TdbFile::parse(&main).unwrap();
    let snap = T.main_snapshot();
    let slots = roster_slots_by_bio_record(&db, &snap);
    let counts = main_team_measurement(&db, &snap, &slots);
    assert_eq!(
        counts,
        MainTeamCounts {
            rostered: 5088,
            plus_one: 5088,
            same: 0,
            unrostered_zero: 446,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 246,
        }
    );
    assert_main_team_declared(
        &T,
        &T.adapter(),
        "main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered",
    );
}

/// The `SENTINEL_ENCODING` processing order over every stored `ubPc.dXSB` pattern on this anchor,
/// with the test helper: pattern 0 is absence before any decode; every non-zero pattern decodes to
/// exactly one of that player's roster slot team codes; the inverse restores each pattern. Codec
/// behaviour only — the parent stays a candidate, and no production executor is claimed.
#[test]
fn stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes() {
    let a = T.adapter();
    let m = mapping(&a, "player.team");
    assert_eq!(m.foreign_key.state, ConstraintState::Unknown);
    assert_eq!(decode_stored(m, 0), Decoded::Absent);
    assert_eq!(decode_stored(m, 1), Decoded::TeamCode(0));
    assert_eq!(
        decode_stored(m, 217),
        Decoded::TeamCode(216),
        "the largest pattern stored here"
    );
    assert_eq!(
        decode_stored(m, 255),
        Decoded::TeamCode(254),
        "not a sentinel; unobserved"
    );
    assert_eq!(encode_stored(m, Decoded::Absent), 0);
    assert_eq!(encode_stored(m, Decoded::TeamCode(216)), 217);
    let main = T.main_anchor_bytes();
    let db = TdbFile::parse(&main).unwrap();
    let slots = roster_slots_by_bio_record(&db, &T.main_snapshot());
    assert_eq!(codec_round_trip_over_anchor(m, &db, &slots), (446, 5088));
}

// ============================================================== 5. handedness

/// Handedness on this anchor: the stored histogram is {0: 3782, 1: 1752}. That is a count and
/// nothing more; no enum file reports counts for this title, and no named player's hand is
/// supported by evidence independent of the enum in this repository. The mapping stays blocked.
#[test]
fn handedness_histogram_is_recorded_but_no_label_is_established() {
    let main = T.main_anchor_bytes();
    let db = TdbFile::parse(&main).unwrap();
    let snap = T.main_snapshot();
    assert_eq!(
        meta_field(&snap, "ubPc", "GRkp").name.as_deref(),
        Some("handedness")
    );
    let bio = table(&db, "ubPc");
    let hand = field(&db, bio, "GRkp");
    assert_eq!(
        hand.bit_offset, 981,
        "this anchor's offset (Legacy's is 979)"
    );
    let mut histogram: BTreeMap<u64, usize> = BTreeMap::new();
    for i in 0..u32::from(bio.current_records) {
        *histogram.entry(read(&db, bio, i, &hand)).or_default() += 1;
    }
    assert_eq!(histogram, BTreeMap::from([(0, 3782), (1, 1752)]));

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
    assert!(c.note.contains("3782") && c.note.contains("1752"));
    assert!(c.note.contains("counts and nothing more"));
    assert!(T.cites(
        &c.provenance,
        "handedness_histogram_is_recorded_but_no_label_is_established"
    ));
}

// ============================================================== 6. team.name

/// `team.name` read / edit / serialize / reopen on an in-memory copy of this anchor (see
/// `target_evidence::team_name_round_trip` for exactly what is exercised). NOT ESTABLISHED: that the
/// game reads the name from this field, any UI path, any save-container round trip.
#[test]
fn team_name_edit_round_trips_on_a_disposable_copy() {
    let original = T.main_anchor_bytes();
    team_name_round_trip(
        &original,
        "Anaheim Ducks®",
        "Winnipeg Jets™",
        "Mighty Ducks of Anaheim®",
    );
    assert_team_name_declared(
        &T,
        &T.adapter(),
        "nhl15_ps3_tests",
        "team_name_edit_round_trips_on_a_disposable_copy",
    );
}

// ============================================================== 7. the ledger

/// What this adapter establishes and what it leaves, counted, so the receipt's figures are checked.
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
        "the same shape as Legacy after R1/R2, measured here from the start"
    );
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
    assert_eq!(mapped_fraction(&a), Some((10, 2969)));
}
