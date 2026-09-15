//! Six gen7 targets (package C0-REG2): NHL Legacy PS3, NHL 14 Xbox 360, NHL 14 PS3, NHL 13 Xbox
//! 360, NHL 12 Xbox 360 and NHL 11 Xbox 360 — semantic and capacity declarations checked against
//! **each target's own committed evidence**, through the mechanics of `target_evidence`.
//!
//! One measurement runner exists per fact; every target supplies its identities and its
//! expectations in an [`Expect`] and declares the same named cases through the `cases!` macro, so
//! the adapter provenance strings (`<this file>#<module>::<case>`) are the test names by
//! construction. Nothing is shared *between* targets: a number appears in exactly one `Expect`.
//!
//! What is measured, each on the exact file the adapter attributes it to:
//!
//! * capacities on the committed ROSTER fixture, decoded by the production save codec through the
//!   container variant the discriminator resolves for it (`ZlibV4` for NHL 14 / Legacy, `RawV2`
//!   for NHL 12 / 13, `RawV1` for NHL 11 — whose *write* path is experimental; only reads happen
//!   here) — the fixture's own directory, not the roster-capacity audit's table;
//! * for the two titles whose PS3 and Xbox 360 main DB and metadata are byte-identical (NHL 14,
//!   Legacy), that identity asserted **directly** on the bytes as an observation with nothing built
//!   on it, and the twin fixture's identical directory figures shown to be told apart by provenance;
//! * birth-date index origin, from the stored value ranges;
//! * the position discriminator and companion rule, labels from EA's metadata table names;
//! * the main-team pointer's encoding and absence pattern, from the metadata-declared roster key;
//! * `team.name` read / edit / serialize / reopen on an in-memory copy, at the field offset the
//!   target's own snapshot records (336 / 352 / 360 across these titles);
//! * the sentinel → storage-decode → index-mapping order over every `ubPc.dXSB` pattern, with the
//!   **test helper** of `target_evidence` — no production adapter executor exists or is claimed.
//!
//! Each case ends by asserting that the adapter declares exactly what was measured. What a case
//! could NOT establish stays a candidate: handedness (no independently supported label here) and the
//! main-team parent table (containment plus a name is not a foreign key). NHL 11 and NHL 12 carry no
//! `player.rating_overall` mapping — their skater block has no field named for it — and record that
//! as `unknown-field-identity`, not absence.

use super::*;
use crate::target_evidence::*;
use std::collections::{BTreeMap, BTreeSet};
use tdb_core::TdbFile;
use tdb_savedata::ContainerVersion;

const CASES_FILE: &str = "crates/tdb-canonical/src/target_batch_tests.rs";

/// Another platform's files this target's main DB and metadata are byte-identical to. The
/// observation is asserted on the bytes; the adapter must not cite the twin's paths or rendering.
struct Twin {
    main: &'static str,
    meta: &'static str,
    /// The twin's anchor directory prefix (`anchors/<platform>/`), which the adapter text must not
    /// contain.
    anchor_prefix: &'static str,
    /// The twin's snapshot file name, which the adapter text must not contain.
    snapshot_name: &'static str,
    /// The twin's adapter, whose capacity provenance must be reported as not this fixture's.
    adapter_file: &'static str,
}

/// One target's expectations. Every number was measured on the files `target` names (C0-REG2
/// probe, re-measured by the runners below); none is carried from another target.
struct Expect {
    target: Target,
    /// The per-target module name, the prefix of every case this target cites.
    module: &'static str,
    framing: RosterFraming,
    main_meta: &'static str,
    main_meta_sha256: &'static str,
    twin: Option<Twin>,
    /// (table, max_records, current_records) as stored in the fixture's directory.
    capacities: &'static [(&'static str, u16, u16)],
    /// Bit offsets of the day / month / year components on this anchor.
    birth_offsets: (u32, u32, u32),
    /// (skaters, goalies) by companion ownership.
    partition: (usize, usize),
    main_team: MainTeamCounts,
    /// The largest `ubPc.dXSB` pattern stored on the anchor.
    largest_pattern: u64,
    /// `ubPc.GRkp` bit offset and stored histogram.
    handedness: (u32, &'static [(u64, usize)]),
    /// `kOtt.YmkJ` bit offset, record 0, record 1, the replacement written.
    team_name: (u32, &'static str, &'static str, &'static str),
    /// The skater-companion ratings this adapter maps (the applicability predicate holders).
    rated: &'static [&'static str],
    /// (mappings, omissions, established, unknown, candidate).
    population: (usize, usize, usize, usize, usize),
    candidates: &'static [&'static str],
    mapped_fraction: (u32, u32),
}

impl Expect {
    fn case(&self, name: &str) -> String {
        format!("{}::{name}", self.module)
    }
}

/// Declares one `#[test]` per (name, runner) pair inside a target module; the string handed to the
/// runner is the function's own name, so a citation and a case cannot drift apart.
macro_rules! cases {
    ($e:ident: $( $name:ident => $runner:path ),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                $runner(&$e, stringify!($name))
            }
        )*
    };
}

mod run {
    use super::*;

    /// Every capacity the adapter declares is the observed directory capacity of the committed
    /// ROSTER fixture — `max_records` as stored, `current_records` as stored — read through the
    /// production save codec and TDB parser. ESTABLISHED: those figures, and that in this fixture
    /// every table's population equals the main anchor's (observed for this fixture — the probe
    /// found it true on all six — asserted, never assumed). NOT ESTABLISHED: that the game accepts
    /// growth up to `max_records`.
    pub(super) fn capacities(e: &Expect, case: &str) {
        let inner = e.target.decoded_roster_fixture(&e.framing);
        let save = TdbFile::parse(&inner).unwrap();
        e.target
            .assert_capacities_are_the_fixtures(&save, e.capacities, &e.case(case));
    }

    /// The generic capacity mutations are exercised on the Legacy module; here the target-specific
    /// point: a capacity attributed to any other file is reported, and so is one whose provenance
    /// omits the fixture — the checks that tell "read from this save" from "coincides with a table".
    pub(super) fn capacity_misattribution_is_reported(e: &Expect, _case: &str) {
        let inner = e.target.decoded_roster_fixture(&e.framing);
        let save = TdbFile::parse(&inner).unwrap();
        let base = e.target.adapter();
        assert!(e
            .target
            .roster_capacity_discrepancies(&base, &save)
            .is_empty());

        let mut audit_sourced = base.clone();
        for cap in &mut audit_sourced.capacities {
            cap.declared.value.as_mut().unwrap().source = "roster-capacity-audit".into();
        }
        let found = e
            .target
            .roster_capacity_discrepancies(&audit_sourced, &save);
        assert_eq!(found.len(), e.capacities.len());
        assert!(found
            .iter()
            .all(|f| f.contains("is not the ROSTER fixture this case decodes")));

        let mut unattributed = base.clone();
        for cap in &mut unattributed.capacities {
            cap.declared
                .provenance
                .retain(|p| p != e.target.roster_fixture);
        }
        let found = e.target.roster_capacity_discrepancies(&unattributed, &save);
        assert_eq!(found.len(), e.capacities.len());
        assert!(found
            .iter()
            .all(|f| f.ends_with("provenance does not name the fixture it was read from")));

        let mut off_by_one = base.clone();
        off_by_one.capacities[0]
            .declared
            .value
            .as_mut()
            .unwrap()
            .ceiling_records += 1;
        let found = e.target.roster_capacity_discrepancies(&off_by_one, &save);
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("ceiling_records"));
    }

    /// The main DB and metadata are byte-identical to the twin platform's — asserted on the bytes.
    /// Recorded as an observation about four files; nothing rests on it: the adapter cites only its
    /// own platform's files and states the identity's limits where it states the identity.
    pub(super) fn twin_identity(e: &Expect, _case: &str) {
        let twin = e.twin.as_ref().expect("this target has a twin");
        let mine = e.target.main_anchor_bytes();
        let theirs = std::fs::read(repo_root().join(twin.main)).unwrap();
        assert_eq!(mine, theirs, "main DB");
        let my_meta = read_pinned(e.main_meta, e.main_meta_sha256);
        let their_meta = std::fs::read(repo_root().join(twin.meta)).unwrap();
        assert_eq!(my_meta, their_meta, "metadata");

        let a = e.target.adapter();
        let text = std::fs::read_to_string(
            repo_root()
                .join("defs/adapters")
                .join(e.target.adapter_file),
        )
        .unwrap();
        assert_eq!(
            a.physical_snapshot.as_deref(),
            Some(
                format!(
                    "docs/audits/data/schema/{}-{}.json",
                    e.target.platform, e.target.game
                )
                .as_str()
            )
        );
        assert!(
            !text.contains(twin.anchor_prefix),
            "no twin anchor is cited"
        );
        assert!(
            !text.contains(twin.snapshot_name),
            "the twin rendering is not cited"
        );
        assert!(
            text.contains("carries NO save capacity, deployment behaviour or runtime conclusion"),
            "the identity's limits are stated where the identity is"
        );
    }

    /// The twin platform's ROSTER fixture carries the same directory figures as this one (the
    /// content year, not the platform, sets them), so the numbers alone cannot tell "read from this
    /// fixture" from "carried from the twin". Provenance can, and does: relabelled with the twin
    /// adapter's provenance, every capacity is reported.
    pub(super) fn twin_capacity_provenance(e: &Expect, _case: &str) {
        let twin = e.twin.as_ref().expect("this target has a twin");
        let inner = e.target.decoded_roster_fixture(&e.framing);
        let save = TdbFile::parse(&inner).unwrap();
        let base = e.target.adapter();
        assert!(e
            .target
            .roster_capacity_discrepancies(&base, &save)
            .is_empty());
        let other = {
            let text =
                std::fs::read_to_string(repo_root().join("defs/adapters").join(twin.adapter_file))
                    .unwrap();
            parse_adapter(&text).unwrap()
        };
        let mut differing = 0;
        let mut relabelled = base.clone();
        for cap in &mut relabelled.capacities {
            let from = other
                .capacities
                .iter()
                .find(|c| c.table == cap.table)
                .expect("the twin declares the same tables");
            let fv = from.declared.established().expect("established there too");
            let mine = cap.declared.established().unwrap();
            if (fv.ceiling_records, fv.population_records)
                != (mine.ceiling_records, mine.population_records)
            {
                differing += 1;
            }
            cap.declared.provenance = from.declared.provenance.clone();
        }
        assert_eq!(differing, 0, "the figures coincide across the twin pair");
        let found = e.target.roster_capacity_discrepancies(&relabelled, &save);
        assert_eq!(found.len(), e.capacities.len(), "{found:?}");
        assert!(found
            .iter()
            .all(|f| f.ends_with("provenance does not name the fixture it was read from")));
    }

    /// The birth-date index origin, measured directly on this anchor: stored months span 0..=11 and
    /// days 0..=30, which a 1-origin encoding could not produce. The adapter's `index-origin-0` is
    /// that measurement, corroborated by the named `studio-domain` case that lists this anchor.
    pub(super) fn birth_date(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let [year, month, day] = birth_date_ranges(&db, &e.target.main_snapshot());
        assert_eq!((month, day), ((0, 11), (0, 30)));
        assert!(year.0 >= 1 && year.1 <= 255);
        let a = e.target.adapter();
        let m = mapping(&a, "player.birth_date");
        assert_eq!(m.transform.established().unwrap().index_origin, Some(0));
        assert_eq!(m.transform.confidence.as_deref(), Some("measured"));
        assert!(e.target.cites(&m.transform.provenance, &e.case(case)));
        assert!(e.target.cites(&m.provenance, &e.case(case)));
        assert_eq!(m.qualification, "automated-tested");
        let case_file = std::fs::read_to_string(
            repo_root().join("crates/studio-domain/tests/birthdate_encoding.rs"),
        )
        .unwrap();
        assert!(
            case_file.contains(e.target.main_anchor),
            "the named case lists this anchor by path"
        );
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
            e.birth_offsets
        );
    }

    /// The discriminator, measured with EA's metadata names as the labels: the bio rows that own
    /// rows in the goalie companion tables all store 4 and nothing else; those that own skater
    /// companion rows store exactly {0, 1, 2, 3}; the two sets partition the table. ESTABLISHED:
    /// discriminator, one-to-one companion rule, ratings applicability. NOT ESTABLISHED: what 0..3
    /// mean individually.
    pub(super) fn position(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        assert_eq!(
            meta_field(&snap, "ubPc", "vjla").name.as_deref(),
            Some("position")
        );
        let part = companion_partition(&db, &snap);
        assert_eq!((part.skaters.len(), part.goalies.len()), e.partition);
        let (skater_codes, goalie_codes) = position_codes_by_partition(&db, &part, 4);
        assert_eq!(skater_codes, BTreeSet::from([0, 1, 2, 3]));
        assert_eq!(goalie_codes, BTreeSet::from([4]));

        let a = e.target.adapter();
        assert_discriminator_declared(&e.target, &a, &[0, 1, 2, 3], &[4], e.rated, &e.case(case));
        assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
    }

    /// The main-team pointer `ubPc.dXSB` against the roster reached through the metadata-declared
    /// key: for each rostered player exactly one slot carries `team == pattern - 1`; every
    /// un-rostered player stores 0; the all-ones pattern is stored nowhere. Where `same > 0`, a
    /// player is rostered on two adjacent team codes — the `pattern - 1` slot is still unique.
    /// ESTABLISHED: the encoding and the absence pattern. NOT ESTABLISHED: that the codes index
    /// `kOtt.vfEq`; the mapping stays blocked on the parent.
    pub(super) fn main_team(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        let slots = roster_slots_by_bio_record(&db, &snap);
        let counts = main_team_measurement(&db, &snap, &slots);
        assert_eq!(counts, e.main_team);
        assert_main_team_declared(&e.target, &e.target.adapter(), &e.case(case));
    }

    /// The `SENTINEL_ENCODING` processing order over every stored `ubPc.dXSB` pattern on this
    /// anchor, with the test helper: pattern 0 is absence before any decode; every non-zero
    /// pattern decodes to exactly one of that player's roster slot team codes; the inverse restores
    /// each pattern. Codec behaviour only; no production executor is claimed.
    pub(super) fn codec_order(e: &Expect, _case: &str) {
        let a = e.target.adapter();
        let m = mapping(&a, "player.team");
        assert_eq!(m.foreign_key.state, ConstraintState::Unknown);
        assert_eq!(decode_stored(m, 0), Decoded::Absent);
        assert_eq!(decode_stored(m, 1), Decoded::TeamCode(0));
        assert_eq!(
            decode_stored(m, e.largest_pattern),
            Decoded::TeamCode(e.largest_pattern - 1),
            "the largest pattern stored here"
        );
        assert_eq!(
            decode_stored(m, 255),
            Decoded::TeamCode(254),
            "not a sentinel; unobserved"
        );
        assert_eq!(encode_stored(m, Decoded::Absent), 0);
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let slots = roster_slots_by_bio_record(&db, &e.target.main_snapshot());
        assert_eq!(
            codec_round_trip_over_anchor(m, &db, &slots),
            (e.main_team.unrostered_zero, e.main_team.rostered)
        );
        let bio = table(&db, "ubPc");
        let f = field(&db, bio, "dXSB");
        let largest = (0..u32::from(bio.current_records))
            .map(|i| read(&db, bio, i, &f))
            .max()
            .unwrap();
        assert_eq!(largest, e.largest_pattern);
    }

    /// Handedness on this anchor: the stored histogram is a count and nothing more; no enum file
    /// reports counts for this title, and no named player's hand is supported by evidence
    /// independent of the enum in this repository. The mapping stays blocked.
    pub(super) fn handedness(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        assert_eq!(
            meta_field(&snap, "ubPc", "GRkp").name.as_deref(),
            Some("handedness")
        );
        let bio = table(&db, "ubPc");
        let hand = field(&db, bio, "GRkp");
        assert_eq!(hand.bit_offset, e.handedness.0, "this anchor's offset");
        let mut histogram: BTreeMap<u64, usize> = BTreeMap::new();
        for i in 0..u32::from(bio.current_records) {
            *histogram.entry(read(&db, bio, i, &hand)).or_default() += 1;
        }
        assert_eq!(
            histogram,
            e.handedness.1.iter().copied().collect::<BTreeMap<_, _>>()
        );

        let a = e.target.adapter();
        let m = mapping(&a, "player.shoots");
        assert_eq!(m.qualification, "blocked");
        assert!(transform_unresolved(m));
        assert_eq!(m.enumeration.state, ConstraintState::Unknown);
        let c = m
            .enumeration
            .candidate
            .as_ref()
            .expect("the proposal is kept");
        for (_, n) in e.handedness.1 {
            assert!(c.note.contains(&n.to_string()));
        }
        assert!(c.note.contains("counts and nothing more"));
        assert!(e.target.cites(&c.provenance, &e.case(case)));
    }

    /// `team.name` read / edit / serialize / reopen on an in-memory copy of this anchor, at the
    /// field offset this target's snapshot records (see `target_evidence::team_name_round_trip`).
    /// NOT ESTABLISHED: that the game reads the name from this field, any UI path, any
    /// save-container round trip.
    pub(super) fn team_name(e: &Expect, case: &str) {
        let original = e.target.main_anchor_bytes();
        let (offset, record0, record1, replacement) = e.team_name;
        team_name_round_trip(&original, offset, record0, record1, replacement);
        let a = e.target.adapter();
        assert_team_name_declared(&e.target, &a, "target_batch_tests", &e.case(case));
        assert_eq!(
            mapping(&a, "team.name")
                .storage
                .established()
                .unwrap()
                .bit_offset,
            offset
        );
    }

    /// What this adapter establishes and what it leaves, counted, so the receipt's figures are
    /// checked — and the unmapped remainder stays recorded as unknown.
    pub(super) fn ledger(e: &Expect, _case: &str) {
        let a = e.target.adapter();
        let population = registry_population(std::iter::once(&a));
        assert_eq!(population.len(), 1);
        let p = &population[0];
        assert_eq!(
            (
                p.mappings,
                p.omissions,
                p.established_constraints,
                p.unknown_constraints,
                p.candidate_constraints
            ),
            e.population
        );
        assert_eq!(candidate_blocks(&a), e.candidates);
        assert_eq!(mapped_fraction(&a), Some(e.mapped_fraction));
        // A title without a stored-overall mapping records the gap as not-found, never as absence.
        for o in &a.omissions {
            assert_eq!(o.reason, "unknown-field-identity", "{}", o.canonical);
            assert!(o.detail.contains("DISCOVERY NOTE"));
        }
        assert_eq!(
            a.omissions.is_empty(),
            e.rated.contains(&"player.rating_overall")
        );
        // The adapter cites this module and no other per-target module.
        let text = std::fs::read_to_string(
            repo_root()
                .join("defs/adapters")
                .join(e.target.adapter_file),
        )
        .unwrap();
        assert!(text.contains(&format!("{CASES_FILE}#{}::", e.module)));
        for other in [
            "legacy_xbox360_tests.rs",
            "nhl15_ps3_tests.rs",
            "nhllegacy_ps3::",
            "nhl14_xbox360::",
            "nhl14_ps3::",
            "nhl13_xbox360::",
            "nhl12_xbox360::",
            "nhl11_xbox360::",
        ] {
            if other.trim_end_matches("::") != e.module {
                assert!(!text.contains(other), "{}: cites {other}", e.target.target);
            }
        }
    }
}

const EIGHT_MAPPINGS: (usize, usize, usize, usize, usize) = (8, 0, 44, 30, 6);
const SEVEN_MAPPINGS: (usize, usize, usize, usize, usize) = (7, 1, 40, 26, 5);
const CANDIDATES_WITH_OVERALL: &[&str] = &[
    "player.shoots.transform",
    "player.shoots.enumeration",
    "player.shoots.null_sentinel",
    "player.rating_overall.null_sentinel",
    "player.rating_speed.null_sentinel",
    "player.team.foreign_key",
];
const CANDIDATES_WITHOUT_OVERALL: &[&str] = &[
    "player.shoots.transform",
    "player.shoots.enumeration",
    "player.shoots.null_sentinel",
    "player.rating_speed.null_sentinel",
    "player.team.foreign_key",
];
const BOTH_RATINGS: &[&str] = &["player.rating_overall", "player.rating_speed"];
const SPEED_ONLY: &[&str] = &["player.rating_speed"];

// ============================================================== NHL Legacy, PS3

mod nhllegacy_ps3 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhllegacy-ps3.toml",
            target: "nhllegacy-ps3",
            cases_file: CASES_FILE,
            platform: "ps3",
            game: "nhllegacy",
            main_anchor: "anchors/ps3/nhllegacy/databases/nhlng.db",
            main_anchor_sha256: "105d2186da3d1f0f3d3ea8a350ead42ac9d5d802ca9fd37652c39bb4e1f1c230",
            // The committed vanilla PS3 Legacy ROSTER save (`SYS-DATA`; `PS3RosterFile` at byte 0).
            roster_fixture: "anchors/ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA",
            roster_fixture_sha256:
                "c079ec4b5ae7ce2aa5b678373a5cb250c5762eb333f7dfe8bb05a35080744cb9",
            roster_payload_sha256:
                "469a13460b793841599908e5835b80f567aefce7b3b86e74ab66dca1348c2b25",
        },
        module: "nhllegacy_ps3",
        // Version word 4 with a zlib body at 0x30; inflate size exceeds `db_size` by the 4-byte
        // trailer the extractor trims.
        framing: RosterFraming {
            outer_len: 2_456_120,
            magic: "PS3RosterFile",
            version: ContainerVersion::ZlibV4,
            version_word: 4,
            uncompressed_size: Some(2_456_076),
            compressed_size: Some(722_156),
            inner_len: 2_456_072,
            table_count: 39,
        },
        main_meta: "anchors/ps3/nhllegacy/databases/nhlng-meta.xml",
        main_meta_sha256: "99062a42cd2eb3e665d912cd69f013e6a6cd9391cc7691b93a678d9b258eb6de",
        twin: Some(Twin {
            main: "anchors/xbox360/nhllegacy/databases/nhlng.db",
            meta: "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/xbox360/",
            snapshot_name: "xbox360-nhllegacy.json",
            adapter_file: "nhllegacy-xbox360.toml",
        }),
        capacities: &[
            ("ubPc", 7988, 5632),
            ("ZBac", 9955, 6087),
            ("eGlu", 10879, 5532),
            ("dSvy", 7331, 5066),
            ("xmja", 7331, 5066),
            ("mHuy", 957, 566),
            ("fMVl", 957, 566),
            ("kOtt", 252, 252),
        ],
        birth_offsets: (958, 975, 816),
        partition: (5066, 566),
        main_team: MainTeamCounts {
            rostered: 5110,
            plus_one: 5110,
            same: 0,
            unrostered_zero: 522,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 252,
        },
        largest_pattern: 222,
        handedness: (979, &[(0, 3781), (1, 1851)]),
        team_name: (
            352,
            "Anaheim Ducks®",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim®",
        ),
        rated: BOTH_RATINGS,
        population: EIGHT_MAPPINGS,
        candidates: CANDIDATES_WITH_OVERALL,
        mapped_fraction: (10, 2965),
    };

    cases!(E:
        capacities_are_the_observed_directory_of_the_committed_roster_fixture => run::capacities,
        a_capacity_that_is_not_read_from_this_fixture_is_reported => run::capacity_misattribution_is_reported,
        the_ps3_main_db_and_metadata_are_byte_identical_to_xbox360s_and_nothing_rests_on_it => run::twin_identity,
        a_capacity_carried_from_the_xbox360_adapter_is_reported_against_this_fixture => run::twin_capacity_provenance,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 14, Xbox 360

mod nhl14_xbox360 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl14-xbox360.toml",
            target: "nhl14-xbox360",
            cases_file: CASES_FILE,
            platform: "xbox360",
            game: "nhl14",
            main_anchor: "anchors/xbox360/nhl14/databases/nhlng.db",
            main_anchor_sha256: "84e1bdff74281102c5cf16de4cdfdb8c5b81027e5d219da8c74bb3b28e935867",
            // Already outside its STFS wrapper: `RosterFile` at byte 0.
            roster_fixture:
                "anchors/xbox360/nhl14/savefiles/ROSTER 20260627054746/ROSTER 20260627054746",
            roster_fixture_sha256:
                "20f144d761b0ff6f838010e2e4cd10ff088226a99bcd7c431fe67f9f62dae4ae",
            roster_payload_sha256:
                "700b8e790d29529da90e46b13a85d1880da430b8da9d1bf00092363ccf673541",
        },
        module: "nhl14_xbox360",
        // Version word 2 with a zlib body at 0x30 — resolved to ZlibV4 by the body marker.
        framing: RosterFraming {
            outer_len: 2_215_156,
            magic: "RosterFile",
            version: ContainerVersion::ZlibV4,
            version_word: 2,
            uncompressed_size: Some(2_215_112),
            compressed_size: Some(676_462),
            inner_len: 2_215_108,
            table_count: 38,
        },
        main_meta: "anchors/xbox360/nhl14/databases/nhlng-meta.xml",
        main_meta_sha256: "d1f9318e50008161ae4bc8dbbb95ef6e4ddf01eb27d8f2faa3e39383cf1b1c22",
        twin: Some(Twin {
            main: "anchors/ps3/nhl14/databases/nhlng.db",
            meta: "anchors/ps3/nhl14/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/ps3/",
            snapshot_name: "ps3-nhl14.json",
            adapter_file: "nhl14-ps3.toml",
        }),
        capacities: &[
            ("ubPc", 6873, 5374),
            ("ZBac", 8803, 5827),
            ("eGlu", 10353, 5353),
            ("dSvy", 6350, 4851),
            ("xmja", 6350, 4851),
            ("mHuy", 823, 523),
            ("fMVl", 823, 523),
            ("kOtt", 246, 246),
        ],
        birth_offsets: (943, 961, 800),
        partition: (4851, 523),
        main_team: MainTeamCounts {
            rostered: 4989,
            plus_one: 4989,
            same: 0,
            unrostered_zero: 385,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 246,
        },
        largest_pattern: 216,
        handedness: (965, &[(0, 3624), (1, 1750)]),
        team_name: (
            352,
            "Anaheim Ducks™",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim™",
        ),
        rated: BOTH_RATINGS,
        population: EIGHT_MAPPINGS,
        candidates: CANDIDATES_WITH_OVERALL,
        mapped_fraction: (10, 2969),
    };

    cases!(E:
        capacities_are_the_observed_directory_of_the_committed_roster_fixture => run::capacities,
        a_capacity_that_is_not_read_from_this_fixture_is_reported => run::capacity_misattribution_is_reported,
        the_xbox360_main_db_and_metadata_are_byte_identical_to_ps3s_and_nothing_rests_on_it => run::twin_identity,
        a_capacity_carried_from_the_ps3_adapter_is_reported_against_this_fixture => run::twin_capacity_provenance,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 14, PS3

mod nhl14_ps3 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl14-ps3.toml",
            target: "nhl14-ps3",
            cases_file: CASES_FILE,
            platform: "ps3",
            game: "nhl14",
            main_anchor: "anchors/ps3/nhl14/databases/nhlng.db",
            main_anchor_sha256: "84e1bdff74281102c5cf16de4cdfdb8c5b81027e5d219da8c74bb3b28e935867",
            roster_fixture: "anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA",
            roster_fixture_sha256:
                "e54e9621f070402fb2294f468ba649fecb92b1b671bcf9633c25c5c0d3b48ce2",
            roster_payload_sha256:
                "c84fadef0a098cbc19efa523a42b87c9b76f338934c04a683c0917078ac9ffad",
        },
        module: "nhl14_ps3",
        framing: RosterFraming {
            outer_len: 2_215_156,
            magic: "PS3RosterFile",
            version: ContainerVersion::ZlibV4,
            version_word: 2,
            uncompressed_size: Some(2_215_112),
            compressed_size: Some(677_512),
            inner_len: 2_215_108,
            table_count: 38,
        },
        main_meta: "anchors/ps3/nhl14/databases/nhlng-meta.xml",
        main_meta_sha256: "d1f9318e50008161ae4bc8dbbb95ef6e4ddf01eb27d8f2faa3e39383cf1b1c22",
        twin: Some(Twin {
            main: "anchors/xbox360/nhl14/databases/nhlng.db",
            meta: "anchors/xbox360/nhl14/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/xbox360/",
            snapshot_name: "xbox360-nhl14.json",
            adapter_file: "nhl14-xbox360.toml",
        }),
        capacities: &[
            ("ubPc", 6873, 5374),
            ("ZBac", 8803, 5827),
            ("eGlu", 10353, 5353),
            ("dSvy", 6350, 4851),
            ("xmja", 6350, 4851),
            ("mHuy", 823, 523),
            ("fMVl", 823, 523),
            ("kOtt", 246, 246),
        ],
        birth_offsets: (943, 961, 800),
        partition: (4851, 523),
        main_team: MainTeamCounts {
            rostered: 4989,
            plus_one: 4989,
            same: 0,
            unrostered_zero: 385,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 246,
        },
        largest_pattern: 216,
        handedness: (965, &[(0, 3624), (1, 1750)]),
        team_name: (
            352,
            "Anaheim Ducks™",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim™",
        ),
        rated: BOTH_RATINGS,
        population: EIGHT_MAPPINGS,
        candidates: CANDIDATES_WITH_OVERALL,
        mapped_fraction: (10, 2969),
    };

    cases!(E:
        capacities_are_the_observed_directory_of_the_committed_roster_fixture => run::capacities,
        a_capacity_that_is_not_read_from_this_fixture_is_reported => run::capacity_misattribution_is_reported,
        the_ps3_main_db_and_metadata_are_byte_identical_to_xbox360s_and_nothing_rests_on_it => run::twin_identity,
        a_capacity_carried_from_the_xbox360_adapter_is_reported_against_this_fixture => run::twin_capacity_provenance,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );

    /// The ratings-write-codec case opens THIS anchor by path, so the rating transform is measured
    /// here rather than derived; the case still picks a skater's first rating cell rather than a
    /// field by id, so the mappings stay `implemented`.
    #[test]
    fn the_rating_regime_case_ran_on_this_anchor_so_its_transform_is_measured_here() {
        let case_file = std::fs::read_to_string(
            repo_root().join("crates/studio-domain/tests/ratings_write_codec.rs"),
        )
        .unwrap();
        assert!(case_file.contains(E.target.main_anchor));
        let a = E.target.adapter();
        for canonical in BOTH_RATINGS {
            let m = mapping(&a, canonical);
            assert_eq!(m.transform.confidence.as_deref(), Some("measured"));
            assert!(m.transform.provenance.iter().any(|p| {
                p == "crates/studio-domain/tests/ratings_write_codec.rs#ratings_write_is_inverse_of_read_ps3_6bit"
            }));
            assert_eq!(m.transform.established().unwrap().offset, Some(36));
            assert_eq!(m.qualification, "implemented");
        }
    }
}

// ============================================================== NHL 13, Xbox 360

mod nhl13_xbox360 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl13-xbox360.toml",
            target: "nhl13-xbox360",
            cases_file: CASES_FILE,
            platform: "xbox360",
            game: "nhl13",
            main_anchor: "anchors/xbox360/nhl13/databases/nhlng.db",
            main_anchor_sha256: "64460a156dfd459255494ac08d7aa8f4db71204c3ece5035431611a27f9822fb",
            roster_fixture:
                "anchors/xbox360/nhl13/savefiles/ROSTER 20260627051601/ROSTER 20260627051601",
            roster_fixture_sha256:
                "d4bd2f7c3066da07bf1e9d742a1f97bf38ee952ed257ccf900f07970605fe52e",
            roster_payload_sha256:
                "0e16b80e046b8697294993a4c176b8949e62418d3db8e09850408de214b36ad6",
        },
        module: "nhl13_xbox360",
        // Version word 0: a raw big-endian TDB body at 0x2C (RawV2), no compressed-size word; the
        // body is the inner TDB byte for byte.
        framing: RosterFraming {
            outer_len: 2_180_292,
            magic: "RosterFile",
            version: ContainerVersion::RawV2,
            version_word: 0,
            uncompressed_size: Some(2_180_248),
            compressed_size: None,
            inner_len: 2_180_248,
            table_count: 36,
        },
        main_meta: "anchors/xbox360/nhl13/databases/nhlng-meta.xml",
        main_meta_sha256: "c5b9994e6079707bfa71c451e2f462f46c8849f8161f602d1b044b05c75956fc",
        twin: None,
        capacities: &[
            ("ubPc", 6946, 5412),
            ("ZBac", 8907, 5874),
            ("eGlu", 10457, 5446),
            ("dSvy", 6407, 4875),
            ("xmja", 6407, 4875),
            ("mHuy", 839, 537),
            ("fMVl", 839, 537),
            ("kOtt", 248, 248),
        ],
        birth_offsets: (938, 954, 768),
        partition: (4875, 537),
        main_team: MainTeamCounts {
            rostered: 5008,
            plus_one: 5008,
            same: 0,
            unrostered_zero: 404,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 248,
        },
        largest_pattern: 214,
        handedness: (958, &[(0, 3621), (1, 1791)]),
        team_name: (
            336,
            "Anaheim Ducks™",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim™",
        ),
        rated: BOTH_RATINGS,
        population: EIGHT_MAPPINGS,
        candidates: CANDIDATES_WITH_OVERALL,
        mapped_fraction: (10, 2843),
    };

    cases!(E:
        capacities_are_the_observed_directory_of_the_committed_roster_fixture => run::capacities,
        a_capacity_that_is_not_read_from_this_fixture_is_reported => run::capacity_misattribution_is_reported,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );

    /// NHL 13's skater block names its ratings `p_*`, and `p_overall` exists here (`dSvy.bPyZ`)
    /// where NHL 11 / 12 have none and NHL 14+ carry `c_overall` under another id.
    #[test]
    fn the_ratings_map_to_this_titles_p_prefixed_fields() {
        let a = E.target.adapter();
        let snap = E.target.main_snapshot();
        for (canonical, id, name) in [
            ("player.rating_overall", "bPyZ", "p_overall"),
            ("player.rating_speed", "qPWY", "p_speed"),
        ] {
            let m = mapping(&a, canonical);
            assert_eq!(m.field.as_deref(), Some(id));
            assert_eq!(meta_field(&snap, "dSvy", id).name.as_deref(), Some(name));
        }
    }
}

// ============================================================== NHL 12, Xbox 360

mod nhl12_xbox360 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl12-xbox360.toml",
            target: "nhl12-xbox360",
            cases_file: CASES_FILE,
            platform: "xbox360",
            game: "nhl12",
            main_anchor: "anchors/xbox360/nhl12/databases/nhlng.db",
            main_anchor_sha256: "94b31ff282f976b1c46b2ac6d261a7ac1b38a15de79046ba721871353369411d",
            roster_fixture:
                "anchors/xbox360/nhl12/savefiles/ROSTER 20260627052113/ROSTER 20260627052113",
            roster_fixture_sha256:
                "5b361a29516a74c8ed328494783b8fa39fb872c25f050018f78b39c089f9a607",
            roster_payload_sha256:
                "9b8c75bf7f127797d7c76ba1edd284a1a141b69a3938035228b62db42a3ba5e9",
        },
        module: "nhl12_xbox360",
        // Version word 2 with the `DB` marker at 0x2C — resolved to RawV2 by that marker, not the
        // word (the same word means ZlibV4 on the NHL 14 fixtures).
        framing: RosterFraming {
            outer_len: 2_350_856,
            magic: "RosterFile",
            version: ContainerVersion::RawV2,
            version_word: 2,
            uncompressed_size: Some(2_350_812),
            compressed_size: None,
            inner_len: 2_350_812,
            table_count: 36,
        },
        main_meta: "anchors/xbox360/nhl12/databases/nhlng-meta.xml",
        main_meta_sha256: "ddaaff54deeb5b646bfd4b70eed6247028eda41a4ff9fc317ef5971fcb593894",
        twin: None,
        capacities: &[
            ("ubPc", 6898, 5395),
            ("ZBac", 8283, 5779),
            ("eGlu", 7853, 5349),
            ("dSvy", 6324, 4821),
            ("xmja", 6324, 4821),
            ("mHuy", 874, 574),
            ("fMVl", 874, 574),
            ("kOtt", 243, 243),
        ],
        birth_offsets: (1128, 1136, 984),
        partition: (4821, 574),
        // Two players are rostered on two adjacent team codes, so two slots also satisfy
        // `team == pattern`; each of those players still has exactly one `pattern - 1` slot.
        main_team: MainTeamCounts {
            rostered: 4995,
            plus_one: 4995,
            same: 2,
            unrostered_zero: 400,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 243,
        },
        largest_pattern: 211,
        handedness: (1140, &[(0, 3613), (1, 1782)]),
        team_name: (
            336,
            "Anaheim Ducks™",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim™",
        ),
        rated: SPEED_ONLY,
        population: SEVEN_MAPPINGS,
        candidates: CANDIDATES_WITHOUT_OVERALL,
        mapped_fraction: (9, 2593),
    };

    cases!(E:
        capacities_are_the_observed_directory_of_the_committed_roster_fixture => run::capacities,
        a_capacity_that_is_not_read_from_this_fixture_is_reported => run::capacity_misattribution_is_reported,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );

    /// The name search the omission records: no `dSvy` field is named for an overall on this
    /// snapshot; the only `overall` names are the draft-pick tables' `pickoverall`. That is where
    /// the search looked, and it proves nothing about absence.
    #[test]
    fn no_skater_block_field_is_named_for_an_overall_on_this_snapshot() {
        super::overall_name_search(&E, 2);
    }
}

// ============================================================== NHL 11, Xbox 360

mod nhl11_xbox360 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl11-xbox360.toml",
            target: "nhl11-xbox360",
            cases_file: CASES_FILE,
            platform: "xbox360",
            game: "nhl11",
            main_anchor: "anchors/xbox360/nhl11/databases/nhlng.db",
            main_anchor_sha256: "461f8d641c50baea0074735906caaf567057da89578ffd2963b9bb3e80dc3c88",
            roster_fixture:
                "anchors/xbox360/nhl11/savefiles/ROSTER1 20260627054323/ROSTER1 20260627054323",
            roster_fixture_sha256:
                "8508fc99f2acee88e0399c78af3e6183ede2bdd36c2a55176f8a5dc7d23e011e",
            roster_payload_sha256:
                "17ba7cd0522bb7ec2f2e52a9aff82af2a361491adfde52434fb6202e8ec8daa0",
        },
        module: "nhl11_xbox360",
        // Version word 1: a 32-byte header with one checksum and no size words, raw big-endian TDB
        // body at 0x20 (RawV1). Its WRITE path is experimental (`tag_1c` is un-reversed); this
        // module only reads.
        framing: RosterFraming {
            outer_len: 1_935_216,
            magic: "RosterFile",
            version: ContainerVersion::RawV1,
            version_word: 1,
            uncompressed_size: None,
            compressed_size: None,
            inner_len: 1_935_184,
            table_count: 32,
        },
        main_meta: "anchors/xbox360/nhl11/databases/nhlng-meta.xml",
        main_meta_sha256: "a83aae929fcd22dae91f2d3cf9e1afbe6a25ce0abadbc4986e486ea56b2d6f5f",
        twin: None,
        capacities: &[
            ("ubPc", 5910, 5310),
            ("ZBac", 7093, 5593),
            ("eGlu", 7728, 5228),
            ("dSvy", 5260, 4760),
            ("xmja", 5260, 4760),
            ("mHuy", 650, 550),
            ("fMVl", 650, 550),
            ("kOtt", 242, 242),
        ],
        birth_offsets: (1126, 1134, 984),
        partition: (4760, 550),
        // One player is rostered on two adjacent team codes (see NHL 12).
        main_team: MainTeamCounts {
            rostered: 4923,
            plus_one: 4923,
            same: 1,
            unrostered_zero: 387,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 242,
        },
        largest_pattern: 210,
        handedness: (1138, &[(0, 3580), (1, 1730)]),
        team_name: (
            360,
            "Anaheim Ducks™",
            "Atlanta Thrashers®",
            "Mighty Ducks of Anaheim™",
        ),
        rated: SPEED_ONLY,
        population: SEVEN_MAPPINGS,
        candidates: CANDIDATES_WITHOUT_OVERALL,
        mapped_fraction: (9, 2514),
    };

    cases!(E:
        capacities_are_the_observed_directory_of_the_committed_roster_fixture => run::capacities,
        a_capacity_that_is_not_read_from_this_fixture_is_reported => run::capacity_misattribution_is_reported,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );

    #[test]
    fn no_skater_block_field_is_named_for_an_overall_on_this_snapshot() {
        super::overall_name_search(&E, 2);
    }

    /// The RawV1 container: the read path used above is the production one and the write path is
    /// declared experimental by the codec itself. The adapter's capacity notes say so.
    #[test]
    fn the_rawv1_write_path_is_experimental_and_the_adapter_says_so() {
        assert!(ContainerVersion::RawV1.write_is_experimental());
        let text = std::fs::read_to_string(
            repo_root()
                .join("defs/adapters")
                .join(E.target.adapter_file),
        )
        .unwrap();
        assert!(text.contains("WRITE path for this container is experimental"));
        let save_matrix =
            std::fs::read_to_string(repo_root().join("docs/audits/data/save-matrix.csv")).unwrap();
        assert!(save_matrix.contains(
            "xbox360,nhl11,ROSTER1 20260627054323,roster,v1,yes,yes,experimental-roundtrip"
        ));
    }
}

/// A name search over this target's snapshot for `overall`: `expected_hits` fields in all, none of
/// them on the skater block; and the adapter's omission is `unknown-field-identity` citing exactly
/// this snapshot.
fn overall_name_search(e: &Expect, expected_hits: usize) {
    let snap = e.target.main_snapshot();
    let hits: Vec<(String, String)> = snap
        .tables
        .iter()
        .flat_map(|t| {
            t.fields.iter().filter_map(move |f| {
                f.name
                    .as_deref()
                    .filter(|n| n.contains("overall"))
                    .map(|n| (t.canonical_id.clone(), n.to_string()))
            })
        })
        .collect();
    assert_eq!(hits.len(), expected_hits, "{hits:?}");
    assert!(hits.iter().all(|(t, n)| t != "dSvy" && n == "pickoverall"));
    let a = e.target.adapter();
    let omission = a
        .omissions
        .iter()
        .find(|o| o.canonical == "player.rating_overall")
        .expect("recorded");
    assert_eq!(omission.reason, "unknown-field-identity");
    assert_eq!(
        omission.provenance,
        [format!(
            "docs/audits/data/schema/{}-{}.json",
            e.target.platform, e.target.game
        )]
    );
    assert!(omission.detail.contains(&format!(
        "{} tables / {} fields",
        snap.table_count, snap.field_total
    )));
}
