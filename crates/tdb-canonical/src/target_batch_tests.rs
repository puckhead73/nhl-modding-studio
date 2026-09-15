//! Ten gen7 targets — six from package C0-REG2 (NHL Legacy PS3, NHL 14 Xbox 360, NHL 14 PS3, NHL 13
//! Xbox 360, NHL 12 Xbox 360, NHL 11 Xbox 360) and four from C0-REG3 (NHL 10, 11, 12 and 13 on
//! PS3) — semantic and capacity declarations checked against **each target's own committed
//! evidence**, through the mechanics of `target_evidence`.
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
//!   for NHL 12 / 13, `RawV1` for NHL 10 / 11 — whose *write* path is experimental; only reads
//!   happen here) — the fixture's own directory, not the roster-capacity audit's table;
//! * for the titles whose PS3 and Xbox 360 main DB and metadata are byte-identical (NHL 10 through
//!   14, Legacy), that identity asserted **directly** on the bytes as an observation with nothing
//!   built on it, and the twin fixture's identical directory figures shown to be told apart by
//!   provenance;
//! * birth-date index origin, from the stored value ranges — 0-origin on NHL 11 onwards, 1-origin
//!   on NHL 10, each read off its own anchor;
//! * the position discriminator and companion rule, labels from EA's metadata table names;
//! * the main-team pointer's encoding, from the metadata-declared roster key: slot team + 1 with
//!   pattern 0 = un-rostered on NHL 11 onwards; the slot team as-is on NHL 10, where pattern 0 is
//!   also team code 0 and so no absence code is established;
//! * `team.name` read / edit / serialize / reopen on an in-memory copy, at the field offset the
//!   target's own snapshot records (336 / 352 / 360 / 680 across these titles);
//! * the sentinel → storage-decode → index-mapping order over every `ubPc.dXSB` pattern, with the
//!   **test helper** of `target_evidence` — no production adapter executor exists or is claimed.
//!
//! Each case ends by asserting that the adapter declares exactly what was measured. What a case
//! could NOT establish stays a candidate: handedness (no independently supported label here) and the
//! main-team parent table (containment plus a name is not a foreign key). NHL 10, 11 and 12 carry no
//! `player.rating_overall` mapping — their skater block has no field named for it — and record that
//! as `unknown-field-identity`, not absence. On NHL 10 the 6-bit rating offset itself is unresolved:
//! the ratings audit names the title under 36 while the production probe resolves 50 on its anchor,
//! and the adapter records the disagreement as a blocked mapping rather than picking a side.

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
    main_meta_text_sha256: &'static str,
    twin: Option<Twin>,
    /// (table, max_records, current_records) as stored in the fixture's directory.
    capacities: &'static [(&'static str, u16, u16)],
    /// Bit offsets of the day / month / year components on this anchor.
    birth_offsets: (u32, u32, u32),
    /// The stored index origin of month and day (0 on NHL 11 onwards, 1 on NHL 10), measured from
    /// the stored ranges — never from the metadata, which is identical either way.
    birth_origin: u64,
    /// (skaters, goalies) by companion ownership.
    partition: (usize, usize),
    main_team_encoding: MainTeamEncoding,
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
                .retain(|p| p != e.target.roster_fixture());
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
        let my_meta = read_pinned_text(e.main_meta, e.main_meta_text_sha256);
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

    /// The birth-date index origin, measured directly on this anchor from the stored ranges: months
    /// 0..=11 and days 0..=30 (which a 1-origin encoding could not produce) on the 0-origin titles,
    /// months 1..=12 and days 1..=31 (which a 0-origin encoding could not produce) on NHL 10. The
    /// adapter's transform — `index-origin-0` or `identity` — is that measurement, corroborated by
    /// the named `studio-domain` case that lists this anchor.
    pub(super) fn birth_date(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let [year, month, day] = birth_date_ranges(&db, &e.target.main_snapshot());
        let o = e.birth_origin;
        assert_eq!((month, day), ((o, o + 11), (o, o + 30)));
        assert!(year.0 >= 1 && year.1 <= 255);
        let a = e.target.adapter();
        let m = mapping(&a, "player.birth_date");
        let tr = m.transform.established().unwrap();
        match o {
            0 => {
                assert_eq!(
                    (tr.kind.as_str(), tr.index_origin),
                    ("index-origin", Some(0))
                );
                assert_eq!(m.read_transform, "index-origin-0");
            }
            1 => {
                assert_eq!((tr.kind.as_str(), tr.index_origin), ("identity", None));
                assert_eq!(m.read_transform, "identity");
            }
            _ => unreachable!(),
        }
        assert_eq!(m.write_transform, m.read_transform);
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
    /// key, under the encoding measured for this anchor. `SlotTeamPlusOne`: for each rostered
    /// player exactly one slot carries `team == pattern - 1`; every un-rostered player stores 0;
    /// the all-ones pattern is stored nowhere; where `same > 0`, a player is rostered on two
    /// adjacent team codes and the `pattern - 1` slot is still unique. `SlotTeamAsIs`: exactly one
    /// slot carries `team == pattern`, no slot `team == pattern - 1`, and pattern 0 is both team
    /// code 0 and what every un-rostered player stores. ESTABLISHED: the encoding, and (plus-one
    /// only) the absence pattern. NOT ESTABLISHED: that the codes index `kOtt.vfEq`; the mapping
    /// stays blocked on the parent.
    pub(super) fn main_team(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        let slots = roster_slots_by_bio_record(&db, &snap);
        let counts = main_team_measurement(&db, &snap, &slots, e.main_team_encoding, 8);
        assert_eq!(counts, e.main_team);
        match e.main_team_encoding {
            MainTeamEncoding::SlotTeamPlusOne => assert_eq!(counts.plus_one, counts.rostered),
            MainTeamEncoding::SlotTeamAsIs => {
                assert_eq!((counts.same, counts.plus_one), (counts.rostered, 0))
            }
        }
        assert_main_team_declared(
            &e.target,
            &e.target.adapter(),
            &e.case(case),
            e.main_team_encoding,
            8,
        );
    }

    /// The `SENTINEL_ENCODING` processing order over every stored `ubPc.dXSB` pattern on this
    /// anchor, with the test helper: pattern 0 is absence before any decode; every non-zero
    /// pattern decodes to exactly one of that player's roster slot team codes; the inverse restores
    /// each pattern. Codec behaviour only; no production executor is claimed. Only the plus-one
    /// regime has an established sentinel to order, so only those targets declare this case.
    pub(super) fn codec_order(e: &Expect, _case: &str) {
        assert_eq!(e.main_team_encoding, MainTeamEncoding::SlotTeamPlusOne);
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
        assert_team_name_declared(&e.target, &a, "target_batch_tests", &e.case(case), 50);
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
            "nhl13_ps3::",
            "nhl12_ps3::",
            "nhl11_ps3::",
            "nhl10_ps3::",
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
            roster: Some(RosterFixture {
                path: "anchors/ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA",
                sha256: "c079ec4b5ae7ce2aa5b678373a5cb250c5762eb333f7dfe8bb05a35080744cb9",
                payload_sha256: "469a13460b793841599908e5835b80f567aefce7b3b86e74ab66dca1348c2b25",
            }),
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
        main_meta_text_sha256: "9eefdbc22c9f2724a8d778dfd8ef0e408040cd2bfca86c60737a7e3713d16819",
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
        birth_origin: 0,
        partition: (5066, 566),
        main_team_encoding: MainTeamEncoding::SlotTeamPlusOne,
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
            roster: Some(RosterFixture {
                path: "anchors/xbox360/nhl14/savefiles/ROSTER 20260627054746/ROSTER 20260627054746",
                sha256: "20f144d761b0ff6f838010e2e4cd10ff088226a99bcd7c431fe67f9f62dae4ae",
                payload_sha256: "700b8e790d29529da90e46b13a85d1880da430b8da9d1bf00092363ccf673541",
            }),
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
        main_meta_text_sha256: "65cbf0e8f0bf8a4983ccd54a3f1fc3e633e7972bef624cc2aaa6c23cd57cddf4",
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
        birth_origin: 0,
        partition: (4851, 523),
        main_team_encoding: MainTeamEncoding::SlotTeamPlusOne,
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
            roster: Some(RosterFixture {
                path: "anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA",
                sha256: "e54e9621f070402fb2294f468ba649fecb92b1b671bcf9633c25c5c0d3b48ce2",
                payload_sha256: "c84fadef0a098cbc19efa523a42b87c9b76f338934c04a683c0917078ac9ffad",
            }),
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
        main_meta_text_sha256: "65cbf0e8f0bf8a4983ccd54a3f1fc3e633e7972bef624cc2aaa6c23cd57cddf4",
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
        birth_origin: 0,
        partition: (4851, 523),
        main_team_encoding: MainTeamEncoding::SlotTeamPlusOne,
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
            roster: Some(RosterFixture {
                path: "anchors/xbox360/nhl13/savefiles/ROSTER 20260627051601/ROSTER 20260627051601",
                sha256: "d4bd2f7c3066da07bf1e9d742a1f97bf38ee952ed257ccf900f07970605fe52e",
                payload_sha256: "0e16b80e046b8697294993a4c176b8949e62418d3db8e09850408de214b36ad6",
            }),
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
        main_meta_text_sha256: "be0829a6eb91ba63d075dc8e668d641ce0322c7ac7d91443f976358fb706c0c8",
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
        birth_origin: 0,
        partition: (4875, 537),
        main_team_encoding: MainTeamEncoding::SlotTeamPlusOne,
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
            roster: Some(RosterFixture {
                path: "anchors/xbox360/nhl12/savefiles/ROSTER 20260627052113/ROSTER 20260627052113",
                sha256: "5b361a29516a74c8ed328494783b8fa39fb872c25f050018f78b39c089f9a607",
                payload_sha256: "9b8c75bf7f127797d7c76ba1edd284a1a141b69a3938035228b62db42a3ba5e9",
            }),
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
        main_meta_text_sha256: "8288938f361c2a4dac17b4be5c2e316b1de32dfd75eead3d106632001cc36df9",
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
        birth_origin: 0,
        partition: (4821, 574),
        // Two players are rostered on two adjacent team codes, so two slots also satisfy
        // `team == pattern`; each of those players still has exactly one `pattern - 1` slot.
        main_team_encoding: MainTeamEncoding::SlotTeamPlusOne,
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
            roster: Some(RosterFixture {
                path:
                    "anchors/xbox360/nhl11/savefiles/ROSTER1 20260627054323/ROSTER1 20260627054323",
                sha256: "8508fc99f2acee88e0399c78af3e6183ede2bdd36c2a55176f8a5dc7d23e011e",
                payload_sha256: "17ba7cd0522bb7ec2f2e52a9aff82af2a361491adfde52434fb6202e8ec8daa0",
            }),
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
        main_meta_text_sha256: "8e65c37eaf96027f35d51c1f6035067becb47d251bba8a39cd34a29362c71fd7",
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
        birth_origin: 0,
        partition: (4760, 550),
        // One player is rostered on two adjacent team codes (see NHL 12).
        main_team_encoding: MainTeamEncoding::SlotTeamPlusOne,
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

    #[test]
    fn the_rawv1_write_path_is_experimental_and_the_adapter_says_so() {
        super::rawv1_write_is_experimental(
            &E,
            "xbox360,nhl11,ROSTER1 20260627054323,roster,v1,yes,yes,experimental-roundtrip",
        );
    }
}

/// The RawV1 container: the read path used by the runners is the production one and the write path
/// is declared experimental by the codec itself. The adapter's capacity notes say so, and the
/// save-matrix row the adapter cites records the fixture as `experimental-roundtrip`.
fn rawv1_write_is_experimental(e: &Expect, save_matrix_row: &str) {
    assert_eq!(e.framing.version, ContainerVersion::RawV1);
    assert!(ContainerVersion::RawV1.write_is_experimental());
    let text = std::fs::read_to_string(
        repo_root()
            .join("defs/adapters")
            .join(e.target.adapter_file),
    )
    .unwrap();
    assert!(text.contains("WRITE path for this container is experimental"));
    let save_matrix =
        std::fs::read_to_string(repo_root().join("docs/audits/data/save-matrix.csv")).unwrap();
    assert!(save_matrix.contains(save_matrix_row));
}

// ============================================================== NHL 13, PS3 (C0-REG3)

mod nhl13_ps3 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl13-ps3.toml",
            target: "nhl13-ps3",
            cases_file: CASES_FILE,
            platform: "ps3",
            game: "nhl13",
            main_anchor: "anchors/ps3/nhl13/databases/nhlng.db",
            main_anchor_sha256: "64460a156dfd459255494ac08d7aa8f4db71204c3ece5035431611a27f9822fb",
            // The committed vanilla PS3 NHL 13 ROSTER save (`SYS-DATA`; `PS3RosterFile` at byte 0).
            roster: Some(RosterFixture {
                path: "anchors/ps3/nhl13/savefiles/BLUS310030200 - ROSTER/SYS-DATA",
                sha256: "afa16f8f0033c6a78e49c0316c12b1b8f82ebf8a3258ce201d4772f5148347f2",
                payload_sha256: "c2cf144d1d2d3dcc2cc0009c306f2665470a70cf18c1311f6f78da3ee8d80f56",
            }),
        },
        module: "nhl13_ps3",
        // Version word 0 with a raw big-endian TDB body at 0x2C (RawV2): the header carries the
        // uncompressed size and no compressed size.
        framing: RosterFraming {
            outer_len: 2_180_292,
            magic: "PS3RosterFile",
            version: ContainerVersion::RawV2,
            version_word: 0,
            uncompressed_size: Some(2_180_248),
            compressed_size: None,
            inner_len: 2_180_248,
            table_count: 36,
        },
        main_meta: "anchors/ps3/nhl13/databases/nhlng-meta.xml",
        main_meta_text_sha256: "be0829a6eb91ba63d075dc8e668d641ce0322c7ac7d91443f976358fb706c0c8",
        twin: Some(Twin {
            main: "anchors/xbox360/nhl13/databases/nhlng.db",
            meta: "anchors/xbox360/nhl13/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/xbox360/",
            snapshot_name: "xbox360-nhl13.json",
            adapter_file: "nhl13-xbox360.toml",
        }),
        // A different fixture from the Xbox 360 one (other outer and payload hashes) with the same
        // directory figures — the twin case shows provenance, not the numbers, tells them apart.
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
        birth_origin: 0,
        partition: (4875, 537),
        main_team_encoding: MainTeamEncoding::SlotTeamPlusOne,
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

    /// The `p_*` ratings of this title, by id, on this snapshot (NHL 14 onwards name them `c_*`).
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

// ============================================================== NHL 12, PS3 (C0-REG3)

mod nhl12_ps3 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl12-ps3.toml",
            target: "nhl12-ps3",
            cases_file: CASES_FILE,
            platform: "ps3",
            game: "nhl12",
            main_anchor: "anchors/ps3/nhl12/databases/nhlng.db",
            main_anchor_sha256: "94b31ff282f976b1c46b2ac6d261a7ac1b38a15de79046ba721871353369411d",
            roster: Some(RosterFixture {
                path: "anchors/ps3/nhl12/savefiles/BLUS307910200 - ROSTER/SYS-DATA",
                sha256: "28132930801e92aedb82bb539c2314f35186399aeb1d169d2bfca74c8d00e3b1",
                payload_sha256: "7074044057d51157a31ac42a14c5b7424066d4004f08c43c6e790263efd07e20",
            }),
        },
        module: "nhl12_ps3",
        // Version word 2 with the `DB` marker at 0x2C — resolved to RawV2 by that marker, not the
        // word (the same word means ZlibV4 on the NHL 14 fixtures).
        framing: RosterFraming {
            outer_len: 2_350_856,
            magic: "PS3RosterFile",
            version: ContainerVersion::RawV2,
            version_word: 2,
            uncompressed_size: Some(2_350_812),
            compressed_size: None,
            inner_len: 2_350_812,
            table_count: 36,
        },
        main_meta: "anchors/ps3/nhl12/databases/nhlng-meta.xml",
        main_meta_text_sha256: "8288938f361c2a4dac17b4be5c2e316b1de32dfd75eead3d106632001cc36df9",
        twin: Some(Twin {
            main: "anchors/xbox360/nhl12/databases/nhlng.db",
            meta: "anchors/xbox360/nhl12/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/xbox360/",
            snapshot_name: "xbox360-nhl12.json",
            adapter_file: "nhl12-xbox360.toml",
        }),
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
        birth_origin: 0,
        partition: (4821, 574),
        main_team_encoding: MainTeamEncoding::SlotTeamPlusOne,
        // Two players are rostered on two adjacent team codes each; their `pattern - 1` slot is
        // still unique.
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

    #[test]
    fn no_skater_block_field_is_named_for_an_overall_on_this_snapshot() {
        super::overall_name_search(&E, 2);
    }
}

// ============================================================== NHL 11, PS3 (C0-REG3)

mod nhl11_ps3 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl11-ps3.toml",
            target: "nhl11-ps3",
            cases_file: CASES_FILE,
            platform: "ps3",
            game: "nhl11",
            main_anchor: "anchors/ps3/nhl11/databases/nhlng.db",
            main_anchor_sha256: "461f8d641c50baea0074735906caaf567057da89578ffd2963b9bb3e80dc3c88",
            roster: Some(RosterFixture {
                path: "anchors/ps3/nhl11/savefiles/BLUS305680200 - ROSTER/SYS-DATA",
                sha256: "c18a39f683fa2438009c433854937dd423f107f128eb8195db522d167a35be9a",
                payload_sha256: "77d2dcb3cfe4e38b624b603eb88b1bf79bba1fb82b51a633366090deef1aa47e",
            }),
        },
        module: "nhl11_ps3",
        // Version word 1: a 32-byte header with one checksum and no size words, raw big-endian TDB
        // body at 0x20 (RawV1). Its WRITE path is experimental (`tag_1c` is un-reversed); this
        // module only reads.
        framing: RosterFraming {
            outer_len: 1_935_216,
            magic: "PS3RosterFile",
            version: ContainerVersion::RawV1,
            version_word: 1,
            uncompressed_size: None,
            compressed_size: None,
            inner_len: 1_935_184,
            table_count: 32,
        },
        main_meta: "anchors/ps3/nhl11/databases/nhlng-meta.xml",
        main_meta_text_sha256: "8e65c37eaf96027f35d51c1f6035067becb47d251bba8a39cd34a29362c71fd7",
        twin: Some(Twin {
            main: "anchors/xbox360/nhl11/databases/nhlng.db",
            meta: "anchors/xbox360/nhl11/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/xbox360/",
            snapshot_name: "xbox360-nhl11.json",
            adapter_file: "nhl11-xbox360.toml",
        }),
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
        birth_origin: 0,
        partition: (4760, 550),
        main_team_encoding: MainTeamEncoding::SlotTeamPlusOne,
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

    #[test]
    fn no_skater_block_field_is_named_for_an_overall_on_this_snapshot() {
        super::overall_name_search(&E, 2);
    }

    #[test]
    fn the_rawv1_write_path_is_experimental_and_the_adapter_says_so() {
        super::rawv1_write_is_experimental(
            &E,
            "ps3,nhl11,BLUS305680200 - ROSTER,roster,v1,yes,yes,experimental-roundtrip",
        );
    }
}

// ============================================================== NHL 10, PS3 (C0-REG3)

/// The earliest gen7 title in the catalog, and the one whose regimes differ: birth month/day are
/// stored 1-origin, the main-team pointer stores the slot's team code as-is over a `0..255` field,
/// and the 6-bit rating offset is contested between the ratings audit (36, by title name) and the
/// production probe (50, by the values this anchor stores). Nothing below is carried from NHL 11.
mod nhl10_ps3 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl10-ps3.toml",
            target: "nhl10-ps3",
            cases_file: CASES_FILE,
            platform: "ps3",
            game: "nhl10",
            main_anchor: "anchors/ps3/nhl10/databases/nhlng.db",
            main_anchor_sha256: "e87b8c806b1d351ae504b9bf0b54c070e999a880e88773328e72f2c4f988d703",
            roster: Some(RosterFixture {
                path: "anchors/ps3/nhl10/savefiles/BLUS303450200 - ROSTER/SYS-DATA",
                sha256: "ceba4776d80edd22ca31220f4c01074824a75a7f43ec61388f7e12fd3deee155",
                payload_sha256: "d2f68d6b380e92d2cd85d92dd49a2c8678d003d800596667fe1b0ab29d941541",
            }),
        },
        module: "nhl10_ps3",
        // Version word 1 (RawV1, as NHL 11): 32-byte header, one checksum, no size words, raw
        // big-endian body at 0x20; 25 tables in the save. Read only — the write path is
        // experimental.
        framing: RosterFraming {
            outer_len: 1_610_252,
            magic: "PS3RosterFile",
            version: ContainerVersion::RawV1,
            version_word: 1,
            uncompressed_size: None,
            compressed_size: None,
            inner_len: 1_610_220,
            table_count: 25,
        },
        main_meta: "anchors/ps3/nhl10/databases/nhlng-meta.xml",
        main_meta_text_sha256: "fdc418c8596f0a62654d8cc57d364d697c3ddc0a2c43220d5c05ca12c0e2855c",
        // The Xbox 360 NHL 10 adapter (C0-CAN1) carries audit-derived capacities over seven tables,
        // not fixture-measured ones over eight, so the twin *capacity* case has no counterpart to
        // relabel from and is not declared here; the identity observation still is.
        twin: Some(Twin {
            main: "anchors/xbox360/nhl10/databases/nhlng.db",
            meta: "anchors/xbox360/nhl10/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/xbox360/",
            snapshot_name: "xbox360-nhl10.json",
            adapter_file: "nhl10-xbox360.toml",
        }),
        capacities: &[
            ("ubPc", 4533, 3933),
            ("ZBac", 5688, 4188),
            ("eGlu", 6219, 3719),
            ("dSvy", 3999, 3499),
            ("xmja", 3999, 3499),
            ("mHuy", 534, 434),
            ("fMVl", 534, 434),
            ("kOtt", 182, 182),
        ],
        birth_offsets: (1400, 1408, 1304),
        birth_origin: 1,
        partition: (3499, 434),
        main_team_encoding: MainTeamEncoding::SlotTeamAsIs,
        // `same` is the count of rostered players whose pattern equals a slot's team code; no
        // slot carries `pattern - 1`. The 494 un-rostered rows store 0 — and so do the 28 rostered
        // rows on team code 0 (see the ambiguity case).
        main_team: MainTeamCounts {
            rostered: 3439,
            plus_one: 0,
            same: 3439,
            unrostered_zero: 494,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 182,
        },
        largest_pattern: 151,
        handedness: (1412, &[(0, 2771), (1, 1162)]),
        team_name: (
            680,
            "Anaheim Ducks™",
            "Atlanta Thrashers®",
            "Mighty Ducks of Anaheim™",
        ),
        rated: SPEED_ONLY,
        // Seven mappings and one omission as NHL 11 / 12, less the two blocks NHL 10 cannot
        // establish: the team sentinel and the speed transform each move from established to
        // unknown-with-candidate.
        population: (7, 1, 38, 28, 7),
        candidates: &[
            "player.shoots.transform",
            "player.shoots.enumeration",
            "player.shoots.null_sentinel",
            "player.rating_speed.transform",
            "player.rating_speed.null_sentinel",
            "player.team.null_sentinel",
            "player.team.foreign_key",
        ],
        mapped_fraction: (9, 1769),
    };

    cases!(E:
        capacities_are_the_observed_directory_of_the_committed_roster_fixture => run::capacities,
        a_capacity_that_is_not_read_from_this_fixture_is_reported => run::capacity_misattribution_is_reported,
        the_ps3_main_db_and_metadata_are_byte_identical_to_xbox360s_and_nothing_rests_on_it => run::twin_identity,
        birth_date_month_and_day_are_stored_one_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_as_is_and_zero_is_also_team_zero => run::main_team,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );

    /// No field named for an overall anywhere in this snapshot — not even the draft-pick
    /// `pickoverall` of NHL 11 onwards. Zero hits is the strongest a name search can say, and it
    /// still says nothing about absence.
    #[test]
    fn no_skater_block_field_is_named_for_an_overall_on_this_snapshot() {
        super::overall_name_search(&E, 0);
    }

    #[test]
    fn the_rawv1_write_path_is_experimental_and_the_adapter_says_so() {
        super::rawv1_write_is_experimental(
            &E,
            "ps3,nhl10,BLUS303450200 - ROSTER,roster,v1,yes,yes,experimental-roundtrip",
        );
    }

    /// Under the as-is encoding pattern 0 is team code 0, and team code 0 is a real club on this
    /// anchor (`kOtt` record 0, key 0, `Anaheim Ducks™`): its 28 rostered players store 0, exactly
    /// as the 494 un-rostered players do. No stored code separates the two, so the adapter keeps
    /// the sentinel unknown, records both counts, and offers "no absence encoding" only as an
    /// asserted candidate citing this case.
    #[test]
    fn stored_zero_is_team_code_zero_as_much_as_it_is_unrostered_so_no_absence_code_is_established()
    {
        let main = E.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = E.target.main_snapshot();
        let kott = table(&db, "kOtt");
        let key = field(&db, kott, "vfEq");
        let name = field(&db, kott, "YmkJ");
        assert_eq!(read(&db, kott, 0, &key), 0, "team code 0 is a real team");
        assert_eq!(
            db.read_inline_str(kott, 0, &name).unwrap(),
            "Anaheim Ducks™".as_bytes()
        );

        let slots = roster_slots_by_bio_record(&db, &snap);
        let bio = table(&db, "ubPc");
        let dxsb = field(&db, bio, "dXSB");
        let on_team_zero: Vec<u32> = slots
            .iter()
            .filter(|(_, teams)| teams.contains(&0))
            .map(|(rec, _)| *rec)
            .collect();
        assert_eq!(on_team_zero.len(), 28);
        assert!(on_team_zero
            .iter()
            .all(|&rec| read(&db, bio, rec, &dxsb) == 0));
        let unrostered_zero = (0..u32::from(bio.current_records))
            .filter(|rec| !slots.contains_key(rec) && read(&db, bio, *rec, &dxsb) == 0)
            .count();
        assert_eq!(unrostered_zero, 494);
        assert_eq!(
            (0..u32::from(bio.current_records))
                .filter(|rec| !slots.contains_key(rec))
                .count(),
            494,
            "every un-rostered row stores 0"
        );

        let a = E.target.adapter();
        let m = mapping(&a, "player.team");
        assert_eq!(m.null_sentinel.state, ConstraintState::Unknown);
        for n in ["28", "494", "team code 0"] {
            assert!(m.null_sentinel.note.contains(n), "{n}");
        }
        let c = m.null_sentinel.candidate.as_ref().unwrap();
        assert_eq!(c.value.as_ref().unwrap().encoding, "none");
        assert_eq!(c.confidence.as_deref(), Some("asserted"));
        assert!(E.target.cites(
            &c.provenance,
            &E.case("stored_zero_is_team_code_zero_as_much_as_it_is_unrostered_so_no_absence_code_is_established")
        ));
        assert!(E.target.cites(
            &m.provenance,
            &E.case("stored_zero_is_team_code_zero_as_much_as_it_is_unrostered_so_no_absence_code_is_established")
        ));
        assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
    }

    /// The two committed sources for the 6-bit rating offset disagree on this anchor. The ratings
    /// audit's regime table names NHL 10 under offset 36; the production probe it designates as the
    /// offset's source (`sixbit_rating_offset`) returns 36 only when a skater or goalie 6-bit
    /// field stores a value above 49 and otherwise 50 — and on this anchor the largest such value
    /// is 49. Measured here: the maximum over every 6-bit field of `dSvy` and `mHuy`, the stored
    /// range of `p_speed`, and that both sources say what they are cited for. The adapter records a
    /// blocked mapping with the probe's result as a derived candidate; neither offset is operative.
    #[test]
    fn the_six_bit_rating_regime_is_unresolved_on_this_anchor_because_the_audit_and_the_probe_disagree(
    ) {
        let main = E.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = E.target.main_snapshot();
        let mut six_bit_fields = BTreeMap::new();
        let mut largest = 0;
        for tbl in ["dSvy", "mHuy"] {
            let t = table(&db, tbl);
            let mut n = 0;
            for f in t.fields.iter().filter(|f| f.bit_width == 6) {
                n += 1;
                for rec in 0..u32::from(t.current_records) {
                    largest = largest.max(read(&db, t, rec, f));
                }
            }
            six_bit_fields.insert(tbl, n);
        }
        assert_eq!(six_bit_fields, BTreeMap::from([("dSvy", 24), ("mHuy", 21)]));
        assert_eq!(largest, 49, "the probe's threshold is `> 49`");
        let skater = table(&db, "dSvy");
        let speed = field(&db, skater, "qPWY");
        assert_eq!(
            meta_field(&snap, "dSvy", "qPWY").name.as_deref(),
            Some("p_speed")
        );
        let (lo, hi) = (0..u32::from(skater.current_records))
            .map(|rec| read(&db, skater, rec, &speed))
            .fold((u64::MAX, 0), |(lo, hi), v| (lo.min(v), hi.max(v)));
        assert_eq!((lo, hi), (2, 47));

        // The two sources, as cited.
        let audit = std::fs::read_to_string(repo_root().join("docs/audits/ratings-write-codec.md"))
            .unwrap();
        assert!(audit.contains("NHL 10 → Legacy / 11–21 (PS3/X360) 6-bit | 0..63 | **36**"));
        let probe =
            std::fs::read_to_string(repo_root().join("crates/studio-domain/src/view.rs")).unwrap();
        assert!(probe.contains("fn sixbit_rating_offset"));
        assert!(probe.contains("> 49"));
        assert!(probe.contains("return 36;"));
        assert!(probe.contains("50 // no 6-bit rating exceeds 49"));

        let a = E.target.adapter();
        let m = mapping(&a, "player.rating_speed");
        assert_eq!(m.qualification, "blocked");
        assert!(transform_unresolved(m));
        assert_eq!(m.transform.state, ConstraintState::Unknown);
        assert!(m.automated_case.is_none());
        let c = m
            .transform
            .candidate
            .as_ref()
            .expect("the probe's result is a candidate");
        assert_eq!(c.confidence.as_deref(), Some("derived"));
        let v = c.value.as_ref().unwrap();
        assert_eq!((v.kind.as_str(), v.offset), ("linear-offset", Some(50)));
        let case = E.case(
            "the_six_bit_rating_regime_is_unresolved_on_this_anchor_because_the_audit_and_the_probe_disagree",
        );
        assert!(E.target.cites(&c.provenance, &case));
        assert!(E.target.cites(&m.provenance, &case));
        for s in ["36", "50", "49", "nhl10-xbox360"] {
            assert!(m.note.contains(s), "{s}");
        }
        assert!(m.note.contains("NOT amended"));
        assert_eq!(
            human_range(m),
            None,
            "no human range while the transform is unknown"
        );
        assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
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
