//! The six early-generation database units of package C0-REG5 — NHL 07, 08 and 09 on Xbox 360,
//! NHL 08 and 09 on PS3, and the NHL 07 PS3 **research copy** under its registry identity
//! `nhl07-ps3` — measured against **each unit's own committed evidence** through the mechanics of
//! `target_evidence`, with what these titles genuinely do differently kept explicit:
//!
//! * gen6 NHL 07 (`NHL07NG.db`): no equipment companions (two-companion rule), a thin
//!   `exhibitionteams` index whose names live on `stockteamtable` (the `team.name` mapping sits on
//!   that table, reached through the `(table, teamid)` join the case measures), 1280-bit name
//!   fields, a 7-bit main-team pointer, a 6-bit year, and the gen6 rating regime (offset 50) that the
//!   ratings audit names by title and the probe resolves on the bytes;
//! * NHL 08 / 09 (`nhlng.db`, gen7 v1 era): 320-bit names, an unsigned as-is main-team pointer
//!   (pattern 0 is team code 0 as much as un-rostered — no absence code, exactly as measured on
//!   NHL 10), 1-origin birth month/day (which the studio-domain probe's prose wrongly reserves for
//!   NHL 10 — recorded as a finding), a rating regime that **no audit row names** (the probe's
//!   result is a candidate, never the transform), and on NHL 09 four bio rows that own no companion
//!   row (skater companions `zero-or-one`);
//! * saves: the NHL 09 ROSTER fixtures decode through the committed RawV3 codec and carry the
//!   ceiling (main DB packed, save with slack — the gen7 shape); the NHL 07 and NHL 08 ROSTER
//!   fixtures decode to a small TDB holding **none** of the roster-core tables, so their save
//!   capacity is blocked (no codec research); the research copy commits no save at all;
//! * the PS3 and Xbox 360 main DB and metadata are byte-identical per title (and the NHL 08 ROSTER
//!   fixtures of the two platforms are one file), asserted on the bytes as observations that nothing
//!   rests on;
//! * `nhl07-ps3` is registered as **representation only**: every measurement a candidate under an
//!   unknown block, every mapping blocked, nothing operative — it is a research copy outside the
//!   shipping targets and its bytes are the Xbox 360 anchor's.

use super::*;
use crate::target_evidence::*;
use std::collections::{BTreeMap, BTreeSet};
use tdb_core::TdbFile;
use tdb_savedata::ContainerVersion;

const CASES_FILE: &str = "crates/tdb-canonical/src/target_early_tests.rs";

/// The other folder whose main DB and metadata are byte-identical to this unit's.
struct Twin {
    main: &'static str,
    meta: &'static str,
    anchor_prefix: &'static str,
    snapshot_name: &'static str,
    /// The twin's adapter, for the capacity-provenance relabel (only where both decode a ROSTER).
    adapter_file: Option<&'static str>,
}

/// Whether the unit is a shipping target whose folder label is uncontradicted, or the research copy.
enum Status {
    Target,
    /// Representation only: every block a candidate, nothing operative.
    ResearchCopy,
}

enum RosterSave {
    /// Decoded through the committed codec; the fixture's directory is the capacity.
    Decoded(RosterFraming),
    /// Decoded through the committed codec to a TDB that holds none of the roster-core tables; the
    /// rest of the container is not decoded. Capacity blocked.
    Blocked {
        path: &'static str,
        sha256: &'static str,
        len: usize,
        inner_len: usize,
        inner_sha256: &'static str,
        table_count: usize,
        /// The other platform's fixture this one is byte-identical to, when it is.
        same_as: Option<&'static str>,
    },
    /// No save of any kind committed under the folder.
    Missing,
}

enum Rating {
    /// The gen6 regime: audit row by title, probe on the bytes, named case on the (identical) bytes.
    Gen6,
    /// No audit row names the title; the probe's result is a candidate.
    Unresolved {
        probe: i64,
        /// Six-bit goalie fields storing more than 49 (what makes the probe say 36), by id.
        over_49: &'static [&'static str],
    },
}

struct Expect {
    target: Target,
    module: &'static str,
    main_meta: &'static str,
    main_meta_text_sha256: &'static str,
    status: Status,
    twin: Twin,
    save: RosterSave,
    /// (table, max_records, current_records) from the decoded ROSTER fixture, where one decodes.
    capacities: &'static [(&'static str, u16, u16)],
    /// The tables whose capacity the adapter records as unknown, where no fixture decodes.
    unknown_capacities: &'static [&'static str],
    /// Bit offsets of the day / month / year components; the year field's width and stored range.
    birth_offsets: (u32, u32, u32),
    year: (u32, u64, u64),
    /// The companion tables this title has, in declaration order, with cardinality and role.
    companions: &'static [(&'static str, &'static str, &'static str)],
    /// (skaters, goalies) by companion ownership; the bio records owning no companion at all.
    partition: (usize, usize),
    orphans: &'static [u32],
    /// (bit offset, bit width) of `ubPc.dXSB`.
    dxsb: (u32, u32),
    main_team: MainTeamCounts,
    /// Rostered players whose stored pattern is 0 — all on team code 0.
    rostered_on_team_zero: usize,
    largest_pattern: u64,
    handedness: (u32, &'static [(u64, usize)]),
    /// (table, field, offset, width, record 0, record 1, replacement).
    team_name: (
        &'static str,
        &'static str,
        u32,
        u32,
        &'static str,
        &'static str,
        &'static str,
    ),
    /// `dSvy.qPWY` offset and stored range; the six-bit maxima on `dSvy` and `mHuy`.
    speed: (u32, u64, u64),
    six_bit_max: (u64, u64),
    rating: Rating,
    population: (usize, usize, usize, usize, usize),
    candidates: &'static [&'static str],
    mapped_fraction: (u32, u32),
}

impl Expect {
    fn case(&self, name: &str) -> String {
        format!("{}::{name}", self.module)
    }
    fn research(&self) -> bool {
        matches!(self.status, Status::ResearchCopy)
    }
    /// The adapter as declared, or — research copy — the promoted copy the established-only
    /// mechanics can check the measurements against.
    fn operative_view(&self) -> Adapter {
        let a = self.target.adapter();
        if self.research() {
            promoted(&a)
        } else {
            a
        }
    }
    fn companion_ids(&self) -> Vec<&'static str> {
        self.companions.iter().map(|c| c.0).collect()
    }
}

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

const ROSTER_CORE: [&str; 8] = [
    "ubPc", "ZBac", "eGlu", "dSvy", "xmja", "mHuy", "fMVl", "kOtt",
];

mod run {
    use super::*;

    /// NHL 09: the fixture's own directory, through the gen7 mechanics (RawV3 container).
    pub(super) fn capacities(e: &Expect, case: &str) {
        let RosterSave::Decoded(framing) = &e.save else {
            panic!("this unit's ROSTER fixture decodes")
        };
        let inner = e.target.decoded_roster_fixture(framing);
        let save = TdbFile::parse(&inner).unwrap();
        e.target
            .assert_capacities_are_the_fixtures(&save, e.capacities, &e.case(case));
        // The main DB is packed: the ceiling is visible only in the save.
        let main_bytes = e.target.main_anchor_bytes();
        let main = TdbFile::parse(&main_bytes).unwrap();
        for (id, max, cur) in e.capacities {
            let t = table(&main, id);
            assert_eq!(t.max_records, t.current_records, "{id}: packed main DB");
            assert_eq!(t.current_records, *cur, "{id}");
            assert!(*max >= *cur);
        }
    }

    pub(super) fn capacity_misattribution_is_reported(e: &Expect, _case: &str) {
        let RosterSave::Decoded(framing) = &e.save else {
            panic!()
        };
        let inner = e.target.decoded_roster_fixture(framing);
        let save = TdbFile::parse(&inner).unwrap();
        let base = e.target.adapter();
        assert!(e
            .target
            .roster_capacity_discrepancies(&base, &save)
            .is_empty());
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

    /// The twin platform's ROSTER fixture carries the same directory figures (a different file:
    /// other magic, other bytes); provenance tells them apart.
    pub(super) fn twin_capacity_provenance(e: &Expect, _case: &str) {
        let RosterSave::Decoded(framing) = &e.save else {
            panic!()
        };
        let other_file = e.twin.adapter_file.expect("the twin decodes a ROSTER too");
        let inner = e.target.decoded_roster_fixture(framing);
        let save = TdbFile::parse(&inner).unwrap();
        let base = e.target.adapter();
        let other = parse_adapter(
            &std::fs::read_to_string(repo_root().join("defs/adapters").join(other_file)).unwrap(),
        )
        .unwrap();
        let mut relabelled = base.clone();
        for cap in &mut relabelled.capacities {
            let from = other
                .capacities
                .iter()
                .find(|c| c.table == cap.table)
                .unwrap();
            let (fv, mine) = (
                from.declared.established().unwrap(),
                cap.declared.established().unwrap(),
            );
            assert_eq!(
                (fv.ceiling_records, fv.population_records),
                (mine.ceiling_records, mine.population_records),
                "the figures coincide across the twin pair"
            );
            cap.declared.provenance = from.declared.provenance.clone();
        }
        let found = e.target.roster_capacity_discrepancies(&relabelled, &save);
        assert_eq!(found.len(), e.capacities.len(), "{found:?}");
        assert!(found
            .iter()
            .all(|f| f.ends_with("provenance does not name the fixture it was read from")));
        // The two fixtures really are different files.
        let other_fixture = other
            .capacities
            .iter()
            .flat_map(|c| c.declared.provenance.iter())
            .find(|p| p.starts_with("anchors/"))
            .unwrap();
        assert_ne!(other_fixture, e.target.roster_fixture());
        assert_ne!(
            std::fs::read(repo_root().join(other_fixture)).unwrap(),
            std::fs::read(repo_root().join(e.target.roster_fixture())).unwrap()
        );
    }

    /// NHL 07 (Xbox 360) and NHL 08: the ROSTER fixture is recognised and decoded by the committed
    /// codec, and what it yields holds none of the roster-core tables — nor any table of the main
    /// DB — so the save capacity is blocked, not measured. Every capacity block is unknown and
    /// says so; the packed main DB's population is recorded as a population, not a ceiling.
    pub(super) fn save_blocked(e: &Expect, case: &str) {
        let RosterSave::Blocked {
            path,
            sha256,
            len,
            inner_len,
            inner_sha256,
            table_count,
            same_as,
        } = &e.save
        else {
            panic!("this unit's ROSTER fixture is the blocked kind")
        };
        let outer = read_pinned(path, sha256);
        assert_eq!(outer.len(), *len);
        let c = tdb_savedata::SaveContainer::parse(&outer).expect("the container header parses");
        assert_eq!(c.header.version, ContainerVersion::RawV3);
        assert_eq!(c.header.version_word, 3);
        assert_eq!(c.header.save_type, tdb_savedata::SaveType::Roster);
        assert!(c.checksums_valid());
        let inner = c.extract_db().expect("the committed codec yields a TDB");
        assert_eq!(inner.len(), *inner_len);
        assert_eq!(sha256_hex(&inner), *inner_sha256);
        let save = TdbFile::parse(&inner).unwrap();
        assert!(save.header_crc_ok() && save.size_ok());
        assert_eq!(save.tables.len(), *table_count);
        let main_bytes = e.target.main_anchor_bytes();
        let main = TdbFile::parse(&main_bytes).unwrap();
        for id in ROSTER_CORE {
            assert!(
                save.table_canonical(id).is_none(),
                "{id} is not in the decoded save"
            );
        }
        assert!(
            save.tables
                .iter()
                .all(|t| main.table_canonical(&t.id.canonical(save.endian)).is_none()),
            "no decoded table is a table of the main DB"
        );
        assert!(
            *len - 0x1C > *inner_len,
            "most of the container is not what the codec decoded"
        );
        if let Some(other) = same_as {
            assert_eq!(
                std::fs::read(repo_root().join(other)).unwrap(),
                outer,
                "the two platforms' ROSTER fixtures are one file"
            );
        }
        assert!(e.target.roster.is_none());
        let a = e.target.adapter();
        assert_eq!(a.capacities.len(), e.unknown_capacities.len());
        for (cap, id) in a.capacities.iter().zip(e.unknown_capacities) {
            assert_eq!(cap.table, *id);
            assert_eq!(cap.declared.state, ConstraintState::Unknown);
            assert!(
                cap.declared.candidate.is_none(),
                "no candidate figure either"
            );
            assert!(cap.declared.note.contains("SAVE CAPACITY BLOCKED"));
            assert!(cap.declared.note.contains(&e.case(case)));
            assert!(cap.declared.note.contains("NOT read off as a substitute"));
            assert_eq!(
                same_as.is_some(),
                cap.declared.note.contains("BYTE-IDENTICAL")
            );
            let t = table(&main, id);
            assert_eq!(t.max_records, t.current_records, "{id}: packed");
            assert!(cap
                .declared
                .note
                .contains(&format!("= {}", t.current_records)));
        }
    }

    /// The research copy: no `savefiles/` directory at all.
    pub(super) fn save_missing(e: &Expect, case: &str) {
        assert!(matches!(e.save, RosterSave::Missing));
        assert!(!repo_root()
            .join("anchors")
            .join(e.target.platform)
            .join(e.target.game)
            .join("savefiles")
            .exists());
        assert!(e.target.roster.is_none());
        let a = e.target.adapter();
        assert_eq!(a.capacities.len(), e.unknown_capacities.len());
        for cap in &a.capacities {
            assert_eq!(cap.declared.state, ConstraintState::Unknown);
            assert!(cap.declared.candidate.is_none());
            assert!(cap.declared.note.contains("SAVE CAPACITY MISSING-FIXTURE"));
            assert!(cap.declared.note.contains(&e.case(case)));
        }
    }

    /// Main DB and metadata byte-identical to the twin folder's; the adapter cites only its own
    /// folder and states the identity's limits.
    pub(super) fn twin_identity(e: &Expect, _case: &str) {
        let mine = e.target.main_anchor_bytes();
        let theirs = std::fs::read(repo_root().join(e.twin.main)).unwrap();
        assert_eq!(mine, theirs, "main DB: byte-identical");
        let my_meta = read_pinned_text(e.main_meta, e.main_meta_text_sha256);
        let their_meta = std::fs::read(repo_root().join(e.twin.meta)).unwrap();
        assert_eq!(text_identity_sha256(&their_meta), e.main_meta_text_sha256);
        assert_eq!(
            my_meta, their_meta,
            "metadata: the same bytes as checked out"
        );
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
        // No twin file is cited as provenance for anything (a note may name one as an
        // observation — the byte-identical NHL 08 fixture, the gen6 case's research-copy path).
        let cited =
            |p: &String| p.starts_with(e.twin.anchor_prefix) || p.contains(e.twin.snapshot_name);
        for m in &a.mappings {
            assert!(
                !m.provenance.iter().any(cited),
                "{}: twin cited",
                m.canonical
            );
            for c in [
                &m.storage.provenance,
                &m.status.provenance,
                &m.transform.provenance,
            ] {
                assert!(!c.iter().any(cited), "{}: twin cited", m.canonical);
            }
        }
        assert!(!a
            .capacities
            .iter()
            .any(|c| c.declared.provenance.iter().any(cited)));
        assert!(
            !text.contains(e.twin.snapshot_name),
            "the twin rendering is not cited"
        );
        assert!(
            text.contains("carries NO save capacity, deployment behaviour or runtime conclusion")
        );
    }

    /// The research copy, through the canonical APIs: a valid document from which nothing is
    /// established — every mapping blocked with unresolved transforms, no human range, no editable
    /// scalar, no capacity, no companion rule, no discriminator; promoted (test-only), the same
    /// measurements pass the established-only mechanics.
    pub(super) fn research_copy(e: &Expect, case: &str) {
        assert!(e.research());
        twin_identity(e, case);
        let a = e.target.adapter();
        let text = std::fs::read_to_string(
            repo_root()
                .join("defs/adapters")
                .join(e.target.adapter_file),
        )
        .unwrap();
        for s in [
            "RESEARCH COPY",
            "REPRESENTATION ONLY",
            "not a completion gate",
            "no committed record",
            &e.case(case),
        ] {
            assert!(text.contains(s), "the adapter states it: {s:?}");
        }
        assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
        for m in &a.mappings {
            assert_eq!(m.qualification, "blocked", "{}", m.canonical);
            assert!(transform_unresolved(m), "{}", m.canonical);
            assert!(!m.inverse_verified);
            assert!(m.automated_case.is_none());
            assert_eq!(human_range(m), None, "{}", m.canonical);
            assert_eq!(scalar_editable(m), None, "{}", m.canonical);
            assert!(m.note.contains("BLOCKED (C0-REG5, research copy)"));
            assert!(m.storage.established().is_none() && m.status.established().is_none());
            assert!(m.transform.established().is_none());
            for c in &m.components {
                assert!(c.storage.established().is_none());
            }
        }
        assert!(a
            .capacities
            .iter()
            .all(|c| c.declared.established().is_none() && c.declared.candidate.is_none()));
        assert!(a.companion_rules[0].declared.established().is_none());
        assert!(a
            .position_discriminator
            .as_ref()
            .unwrap()
            .established()
            .is_none());
        let p = &registry_population(std::iter::once(&a))[0];
        assert_eq!(p.established_constraints, 0);
        assert_eq!(all_candidate_blocks(&a).len(), p.candidate_constraints);
        assert!(std::panic::catch_unwind(|| decode_stored(mapping(&a, "player.team"), 1)).is_err());
        let lifted = promoted(&a);
        assert_eq!(
            human_range(mapping(&lifted, "player.rating_speed")),
            Some([50, 113])
        );
        assert_eq!(scalar_editable(mapping(&lifted, "team.name")), Some(true));
    }

    /// Birth month/day stored 1..=12 / 1..=31: 1-origin, identity transform, measured here (no
    /// studio-domain probe lists these anchors).
    pub(super) fn birth_date(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        let [year, month, day] = birth_date_ranges(&db, &snap);
        assert_eq!((month, day), ((1, 12), (1, 31)));
        assert_eq!(year, (e.year.1, e.year.2));
        assert_eq!(field(&db, table(&db, "ubPc"), "qFnd").bit_width, e.year.0);
        let probe = std::fs::read_to_string(
            repo_root().join("crates/studio-domain/tests/birthdate_encoding.rs"),
        )
        .unwrap();
        assert!(
            !probe.contains(e.target.main_anchor),
            "no studio-domain probe lists this anchor"
        );
        assert!(
            probe.contains("only 1-indexed title"),
            "the probe's prose still calls NHL 10 the only 1-indexed title — the finding stands"
        );
        let a = e.target.adapter();
        let m = mapping(&a, "player.birth_date");
        assert!(e.target.cites(&m.provenance, &e.case(case)));
        let tr = if e.research() {
            assert_eq!(m.qualification, "blocked");
            let c = candidate_of(&m.transform, "birth_date.transform");
            assert!(e.target.cites(&c.provenance, &e.case(case)));
            c.value.as_ref().unwrap()
        } else {
            assert_eq!(m.qualification, "automated-tested");
            assert_eq!(
                m.automated_case.as_deref(),
                Some(format!("tdb_canonical::target_early_tests::{}", e.case(case)).as_str())
            );
            assert_eq!(
                (m.read_transform.as_str(), m.write_transform.as_str()),
                ("identity", "identity")
            );
            assert!(e.target.cites(&m.transform.provenance, &e.case(case)));
            m.transform.established().unwrap()
        };
        assert_eq!((tr.kind.as_str(), tr.index_origin), ("identity", None));
        let offset = |role: &str| {
            let c = &m
                .components
                .iter()
                .find(|c| c.role == role)
                .unwrap()
                .storage;
            let st = if e.research() {
                candidate_value(c, role)
            } else {
                c.established().unwrap()
            };
            assert_eq!(st.endian, "big");
            st.bit_offset
        };
        assert_eq!(
            (offset("day"), offset("month"), offset("year")),
            e.birth_offsets
        );
    }

    /// Companions and the discriminator over this title's own companion set: two tables on gen6,
    /// four on NHL 08/09; on NHL 09 the four companion-less code-0 rows are returned, not refused.
    pub(super) fn position(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        assert_eq!(
            meta_field(&snap, "ubPc", "vjla").name.as_deref(),
            Some("position")
        );
        assert_eq!(
            meta_field(&snap, "ubPc", "wBIz").name.as_deref(),
            Some("game_id")
        );
        for absent in ["xmja", "fMVl"] {
            assert_eq!(
                db.table_canonical(absent).is_some(),
                e.companion_ids().contains(&absent),
                "{absent}: present exactly when declared"
            );
        }
        let (part, uncovered) = companion_partition_with(&db, &snap, &e.companion_ids());
        assert_eq!((part.skaters.len(), part.goalies.len()), e.partition);
        assert_eq!(
            uncovered,
            e.orphans.iter().copied().collect::<BTreeSet<_>>()
        );
        let (skater_codes, goalie_codes) = position_codes_by_partition(&db, &part, 4);
        assert_eq!(skater_codes, BTreeSet::from([0, 1, 2, 3]));
        assert_eq!(goalie_codes, BTreeSet::from([4]));
        let bio = table(&db, "ubPc");
        let pos = field(&db, bio, "vjla");
        for &r in e.orphans {
            assert_eq!(
                read(&db, bio, r, &pos),
                0,
                "record {r}: a skater code with no skater row"
            );
        }
        let a = e.operative_view();
        assert_discriminator_declared(
            &e.target,
            &a,
            &[0, 1, 2, 3],
            &[4],
            &["player.rating_speed"],
            e.companions,
            &e.case(case),
        );
        if !e.orphans.is_empty() {
            let rule = a.companion_rules[0].declared.established().unwrap();
            assert!(rule
                .companions
                .iter()
                .filter(|c| c.applies_when == "position-skater")
                .all(|c| c.cardinality == "zero-or-one"));
            for r in e.orphans {
                assert!(a.companion_rules[0].declared.note.contains(&r.to_string()));
            }
        }
        assert_eq!(
            validate_adapter(&e.target.adapter(), &schema_for_validation()),
            vec![]
        );
    }

    /// The as-is main-team regime at this title's width.
    pub(super) fn main_team(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        let slots = roster_slots_by_bio_record(&db, &snap);
        let counts =
            main_team_measurement(&db, &snap, &slots, MainTeamEncoding::SlotTeamAsIs, e.dxsb.1);
        assert_eq!(counts, e.main_team);
        assert_eq!((counts.same, counts.plus_one), (counts.rostered, 0));
        let bio = table(&db, "ubPc");
        let dxsb = field(&db, bio, "dXSB");
        assert_eq!((dxsb.bit_offset, dxsb.bit_width), e.dxsb);
        let largest = (0..u32::from(bio.current_records))
            .map(|i| read(&db, bio, i, &dxsb))
            .max()
            .unwrap();
        assert_eq!(largest, e.largest_pattern);
        assert!(largest < (1u64 << e.dxsb.1));
        let a = e.operative_view();
        assert_main_team_declared(
            &e.target,
            &a,
            &e.case(case),
            MainTeamEncoding::SlotTeamAsIs,
            e.dxsb.1,
        );
        if e.research() {
            let declared = e.target.adapter();
            let m = mapping(&declared, "player.team");
            assert_eq!(m.qualification, "blocked");
            candidate_of(&m.storage, "team.storage");
            candidate_of(&m.transform, "team.transform");
        }
    }

    /// Pattern 0 is team code 0 (a real club) as much as it is un-rostered.
    pub(super) fn ambiguity(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        let kott = table(&db, "kOtt");
        assert_eq!(
            read(&db, kott, 0, &field(&db, kott, "vfEq")),
            0,
            "team code 0 is a real team"
        );
        let (nt, nf, _, _, rec0, _, _) = e.team_name;
        let named = table(&db, nt);
        assert_eq!(
            db.read_inline_str(named, 0, &field(&db, named, nf))
                .unwrap(),
            rec0.as_bytes()
        );
        let slots = roster_slots_by_bio_record(&db, &snap);
        let bio = table(&db, "ubPc");
        let dxsb = field(&db, bio, "dXSB");
        let on_team_zero: Vec<u32> = slots
            .iter()
            .filter(|(_, teams)| teams.contains(&0))
            .map(|(rec, _)| *rec)
            .collect();
        assert_eq!(on_team_zero.len(), e.rostered_on_team_zero);
        assert!(on_team_zero.iter().all(|&r| read(&db, bio, r, &dxsb) == 0));
        let unrostered: Vec<u32> = (0..u32::from(bio.current_records))
            .filter(|r| !slots.contains_key(r))
            .collect();
        assert_eq!(unrostered.len(), e.main_team.unrostered_zero);
        assert!(unrostered.iter().all(|&r| read(&db, bio, r, &dxsb) == 0));
        let a = e.target.adapter();
        let m = mapping(&a, "player.team");
        assert_eq!(m.null_sentinel.state, ConstraintState::Unknown);
        let c = m.null_sentinel.candidate.as_ref().unwrap();
        assert_eq!(c.value.as_ref().unwrap().encoding, "none");
        assert_eq!(c.confidence.as_deref(), Some("asserted"));
        assert!(e.target.cites(&c.provenance, &e.case(case)));
        assert!(e.target.cites(&m.provenance, &e.case(case)));
        for n in [
            e.rostered_on_team_zero.to_string(),
            e.main_team.unrostered_zero.to_string(),
            "team code 0".into(),
        ] {
            assert!(
                m.null_sentinel.note.contains(&n) || c.note.contains(&n),
                "{n}"
            );
        }
    }

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
        assert_eq!(hand.bit_offset, e.handedness.0);
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
        let c = m.enumeration.candidate.as_ref().unwrap();
        for (_, n) in e.handedness.1 {
            assert!(c.note.contains(&n.to_string()));
        }
        assert!(c.note.contains("counts and nothing more"));
        assert!(e.target.cites(&c.provenance, &e.case(case)));
    }

    /// The six-bit regime on this anchor: measured maxima on both AI tables, the audit's rows as
    /// cited, the probe's rule as cited — and the adapter's declaration: the gen6 offset 50
    /// established (derived) on NHL 07, a blocked mapping with the probe's result as a candidate
    /// on NHL 08 / 09, which no audit row names.
    pub(super) fn ratings(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        let mut maxima = Vec::new();
        let mut over_49: Vec<String> = Vec::new();
        for tb in ["dSvy", "mHuy"] {
            let t = table(&db, tb);
            let mut m6 = 0;
            for f in t.fields.iter().filter(|f| f.bit_width == 6) {
                let mx = (0..u32::from(t.current_records))
                    .map(|r| read(&db, t, r, f))
                    .max()
                    .unwrap();
                m6 = m6.max(mx);
                if mx > 49 {
                    over_49.push(f.id.canonical(db.endian));
                }
            }
            maxima.push(m6);
        }
        assert_eq!((maxima[0], maxima[1]), e.six_bit_max);
        let skater = table(&db, "dSvy");
        let speed = field(&db, skater, "qPWY");
        assert_eq!(
            meta_field(&snap, "dSvy", "qPWY").name.as_deref(),
            Some("p_speed")
        );
        assert_eq!(speed.bit_offset, e.speed.0);
        let (lo, hi) = (0..u32::from(skater.current_records))
            .map(|r| read(&db, skater, r, &speed))
            .fold((u64::MAX, 0), |(lo, hi), v| (lo.min(v), hi.max(v)));
        assert_eq!((lo, hi), (e.speed.1, e.speed.2));

        let audit = std::fs::read_to_string(repo_root().join("docs/audits/ratings-write-codec.md"))
            .unwrap();
        assert!(audit.contains("**NHL07 gen6** 6-bit | 0..63 | **50** | 50..99"));
        assert!(audit.contains("NHL 10 → Legacy / 11–21 (PS3/X360) 6-bit | 0..63 | **36**"));
        let probe =
            std::fs::read_to_string(repo_root().join("crates/studio-domain/src/view.rs")).unwrap();
        assert!(probe.contains("fn sixbit_rating_offset") && probe.contains("> 49"));
        let named = std::fs::read_to_string(
            repo_root().join("crates/studio-domain/tests/ratings_write_codec.rs"),
        )
        .unwrap();

        let a = e.target.adapter();
        let m = mapping(&a, "player.rating_speed");
        assert!(e.target.cites(&m.provenance, &e.case(case)));
        match &e.rating {
            Rating::Gen6 => {
                assert_eq!(e.six_bit_max, (49, 49));
                assert!(over_49.is_empty());
                assert!(named.contains("fn ratings_write_is_inverse_of_read_gen6("));
                assert!(named.contains("anchors/ps3/nhl07/databases/NHL07NG.db"));
                let (tr, conf, prov) = if e.research() {
                    assert_eq!(m.qualification, "blocked");
                    let c = candidate_of(&m.transform, "rating.transform");
                    (
                        c.value.as_ref().unwrap(),
                        c.confidence.as_deref(),
                        &c.provenance,
                    )
                } else {
                    assert_eq!(m.qualification, "implemented");
                    assert!(m.inverse_verified);
                    assert_eq!(
                        (m.read_transform.as_str(), m.write_transform.as_str()),
                        ("offset+50", "offset-50")
                    );
                    assert_eq!(human_range(m), Some([50, 113]));
                    (
                        m.transform.established().unwrap(),
                        m.transform.confidence.as_deref(),
                        &m.transform.provenance,
                    )
                };
                assert_eq!((tr.kind.as_str(), tr.offset), ("linear-offset", Some(50)));
                assert_eq!(conf, Some("derived"));
                assert!(e.target.cites(prov, &e.case(case)));
            }
            Rating::Unresolved {
                probe: expected,
                over_49: fields,
            } => {
                assert_eq!(
                    over_49,
                    fields.iter().map(|s| s.to_string()).collect::<Vec<_>>()
                );
                let probe_result = if over_49.is_empty() { 50 } else { 36 };
                assert_eq!(probe_result, *expected);
                for title in ["NHL 08", "NHL08", "NHL 09", "NHL09"] {
                    assert!(
                        !audit.contains(title),
                        "the audit names no regime for {title}"
                    );
                }
                assert_eq!(m.qualification, "blocked");
                assert!(transform_unresolved(m));
                assert_eq!(m.transform.state, ConstraintState::Unknown);
                let c = m.transform.candidate.as_ref().unwrap();
                assert_eq!(c.confidence.as_deref(), Some("derived"));
                let v = c.value.as_ref().unwrap();
                assert_eq!(
                    (v.kind.as_str(), v.offset),
                    ("linear-offset", Some(*expected))
                );
                assert!(e.target.cites(&c.provenance, &e.case(case)));
                for f in *fields {
                    assert!(m.note.contains(f) && c.note.contains(f), "{f} is named");
                }
                assert!(m.note.contains("names NHL 0") && m.note.contains("under neither"));
                assert_eq!(human_range(m), None);
            }
        }
    }

    /// `team.name` round trip on the field this title keeps it in — `kOtt.fullname`, or on gen6
    /// the stock team table reached through the thin index (whose join is measured here).
    pub(super) fn team_name(e: &Expect, case: &str) {
        let (t, f, offset, width, rec0, rec1, replacement) = e.team_name;
        let original = e.target.main_anchor_bytes();
        let snap = e.target.main_snapshot();
        assert_eq!(meta_field(&snap, t, f).name.as_deref(), Some("fullname"));
        inline_string_round_trip(&original, t, f, offset, width, rec0, rec1, replacement);
        if t != "kOtt" {
            let db = TdbFile::parse(&original).unwrap();
            let kott = table(&db, "kOtt");
            assert!(
                kott.fields.iter().all(|x| x.kind != 0),
                "the thin index carries no string"
            );
            assert_eq!(
                meta_field(&snap, "kOtt", "RREB").name.as_deref(),
                Some("table")
            );
            assert_eq!(
                meta_field(&snap, "kOtt", "mCXg").name.as_deref(),
                Some("teamid")
            );
            assert_eq!(meta_table_name(&snap, t), "stockteamtable");
            let (rreb, teamid) = (field(&db, kott, "RREB"), field(&db, kott, "mCXg"));
            let stock = table(&db, t);
            let key = field(&db, stock, "vfEq");
            let by_key: BTreeMap<u64, u32> = (0..u32::from(stock.current_records))
                .map(|i| (read(&db, stock, i, &key), i))
                .collect();
            assert_eq!(by_key.len(), usize::from(stock.current_records));
            for i in 0..u32::from(kott.current_records) {
                assert_eq!(read(&db, kott, i, &rreb), 0, "kOtt record {i}: stock table");
                assert_eq!(
                    by_key.get(&read(&db, kott, i, &teamid)),
                    Some(&i),
                    "kOtt record {i} -> stock row"
                );
            }
            assert_eq!(table(&db, "fLEY").current_records, 0, "no custom teams");
        }
        let a = e.operative_view();
        if e.research() {
            let declared = e.target.adapter();
            let m = mapping(&declared, "team.name");
            assert_eq!(m.qualification, "blocked");
            assert!(m.automated_case.is_none());
            assert_eq!(
                candidate_value(&m.string_storage, "team.name.string_storage").byte_capacity,
                Some(width / 8)
            );
            candidate_of(&m.storage, "team.name.storage");
        } else {
            assert_team_name_declared(
                &e.target,
                &a,
                "target_early_tests",
                &e.case(case),
                width / 8,
            );
        }
        let m = mapping(&a, "team.name");
        assert_eq!((m.table.as_deref(), m.field.as_deref()), (Some(t), Some(f)));
        let st = m.storage.established().unwrap();
        assert_eq!(
            (st.bit_offset, st.bit_width, st.endian.as_str()),
            (offset, width, "big")
        );
        if t != "kOtt" {
            let ap = m.applicability.established().unwrap();
            assert_eq!(
                (
                    ap.kind.as_str(),
                    ap.table.as_deref(),
                    ap.field.as_deref(),
                    ap.codes.as_slice()
                ),
                ("target-predicate", Some("kOtt"), Some("RREB"), &[0][..])
            );
            assert!(m.note.contains("REPRESENTATION LIMIT"));
        }
    }

    /// Zero hits for `overall` anywhere in the snapshot; the omission says so and claims nothing.
    pub(super) fn overall_search(e: &Expect, _case: &str) {
        let snap = e.target.main_snapshot();
        let hits = snap
            .tables
            .iter()
            .flat_map(|t| t.fields.iter())
            .filter(|f| f.name.as_deref().is_some_and(|n| n.contains("overall")))
            .count();
        assert_eq!(hits, 0);
        let a = e.target.adapter();
        let o = a
            .omissions
            .iter()
            .find(|o| o.canonical == "player.rating_overall")
            .unwrap();
        assert_eq!(o.reason, "unknown-field-identity");
        assert!(o.detail.contains("DISCOVERY NOTE"));
        assert!(o.detail.contains(&format!(
            "{} tables / {} fields",
            snap.table_count, snap.field_total
        )));
        assert_eq!(
            o.provenance,
            [format!(
                "docs/audits/data/schema/{}-{}.json",
                e.target.platform, e.target.game
            )]
        );
    }

    pub(super) fn ledger(e: &Expect, _case: &str) {
        let a = e.target.adapter();
        assert_eq!(a.endian, "big");
        assert_eq!(
            a.generation,
            if e.target.game == "nhl07" {
                "gen6"
            } else {
                "gen7"
            }
        );
        let p = &registry_population(std::iter::once(&a))[0];
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
        if e.research() {
            assert_eq!(p.established_constraints, 0);
            assert_eq!(all_candidate_blocks(&a), e.candidates);
            assert!(a
                .coverage_declared
                .as_ref()
                .unwrap()
                .note
                .contains("REGISTERED AS REPRESENTATION ONLY"));
        } else {
            assert_eq!(candidate_blocks(&a), e.candidates);
        }
        assert_eq!(mapped_fraction(&a), Some(e.mapped_fraction));
        assert_eq!(a.omissions.len(), 1);
        assert!(a
            .omissions
            .iter()
            .all(|o| o.reason == "unknown-field-identity"));
        let text = std::fs::read_to_string(
            repo_root()
                .join("defs/adapters")
                .join(e.target.adapter_file),
        )
        .unwrap();
        assert!(text.contains(&format!("{CASES_FILE}#{}::", e.module)));
        for other in [
            "target_batch_tests.rs",
            "target_ps4_tests.rs",
            "legacy_xbox360_tests.rs",
            "nhl15_ps3_tests.rs",
            "nhl07_xbox360::",
            "nhl08_xbox360::",
            "nhl09_xbox360::",
            "nhl07_ps3::",
            "nhl08_ps3::",
            "nhl09_ps3::",
        ] {
            if other.trim_end_matches("::") != e.module {
                assert!(!text.contains(other), "{}: cites {other}", e.target.target);
            }
        }
        assert!(!text.contains("offset+36") && !text.contains("index-origin-0\"\nwrite_transform = \"index-origin-0\"\ninverse_verified = true"));
        // Nothing carried from a later title: no NHL 10+ anchor, no PS4 path.
        for carried in [
            "anchors/ps4/",
            "anchors/ps3/nhl1",
            "anchors/xbox360/nhl1",
            "anchors/ps3/nhllegacy",
            "anchors/xbox360/nhllegacy",
        ] {
            assert!(
                !text.contains(carried),
                "{}: mentions {carried}",
                e.target.target
            );
        }
    }
}

const NHL07_COMPANIONS: &[(&str, &str, &str)] = &[
    ("dSvy", "one-to-one", "position-skater"),
    ("mHuy", "one-to-one", "position-goalie"),
];
const NHL09_COMPANIONS: &[(&str, &str, &str)] = &[
    ("dSvy", "zero-or-one", "position-skater"),
    ("xmja", "zero-or-one", "position-skater"),
    ("mHuy", "one-to-one", "position-goalie"),
    ("fMVl", "one-to-one", "position-goalie"),
];
/// Seven mappings, one omission: NHL 07 (established gen6 rating transform).
const NHL07_POPULATION: (usize, usize, usize, usize, usize) = (7, 1, 31, 33, 6);
/// NHL 08 / 09: the rating transform is a candidate, and (08) capacities are unknown.
const NHL08_POPULATION: (usize, usize, usize, usize, usize) = (7, 1, 30, 36, 7);
const NHL09_POPULATION: (usize, usize, usize, usize, usize) = (7, 1, 38, 28, 7);
const NHL07_CANDIDATES: &[&str] = &[
    "player.shoots.transform",
    "player.shoots.enumeration",
    "player.shoots.null_sentinel",
    "player.rating_speed.null_sentinel",
    "player.team.null_sentinel",
    "player.team.foreign_key",
];
const UNRESOLVED_RATING_CANDIDATES: &[&str] = &[
    "player.shoots.transform",
    "player.shoots.enumeration",
    "player.shoots.null_sentinel",
    "player.rating_speed.transform",
    "player.rating_speed.null_sentinel",
    "player.team.null_sentinel",
    "player.team.foreign_key",
];
/// The research copy: NHL 07's 34 established blocks become candidates alongside its 6.
const NHL07_RESEARCH_POPULATION: (usize, usize, usize, usize, usize) = (7, 1, 0, 64, 37);
const NHL07_RESEARCH_CANDIDATES: &[&str] = &[
    "player.first_name.storage",
    "player.first_name.status",
    "player.first_name.transform",
    "player.first_name.string_storage",
    "player.first_name.applicability",
    "player.last_name.storage",
    "player.last_name.status",
    "player.last_name.transform",
    "player.last_name.string_storage",
    "player.last_name.applicability",
    "player.birth_date.status",
    "player.birth_date.transform",
    "player.birth_date.applicability",
    "player.shoots.storage",
    "player.shoots.status",
    "player.shoots.transform",
    "player.shoots.enumeration",
    "player.shoots.null_sentinel",
    "player.shoots.applicability",
    "player.rating_speed.storage",
    "player.rating_speed.status",
    "player.rating_speed.transform",
    "player.rating_speed.null_sentinel",
    "player.rating_speed.applicability",
    "player.team.storage",
    "player.team.status",
    "player.team.transform",
    "player.team.null_sentinel",
    "player.team.foreign_key",
    "player.team.applicability",
    "team.name.storage",
    "team.name.status",
    "team.name.transform",
    "team.name.string_storage",
    "team.name.applicability",
    "companion_rule.ubPc",
    "position_discriminator",
];

const NHL09_CAPS: &[(&str, u16, u16)] = &[
    ("ubPc", 4568, 3968),
    ("ZBac", 5740, 4240),
    ("eGlu", 6400, 3900),
    ("kOtt", 188, 188),
    ("dSvy", 4049, 3549),
    ("xmja", 4049, 3549),
    ("mHuy", 515, 415),
    ("fMVl", 515, 415),
];
const EIGHT_TABLES: &[&str] = &[
    "ubPc", "ZBac", "eGlu", "kOtt", "dSvy", "xmja", "mHuy", "fMVl",
];
const NHL07_TABLES: &[&str] = &["ubPc", "ZBac", "eGlu", "kOtt", "dSvy", "mHuy"];

const fn nhl09_framing(magic: &'static str) -> RosterFraming {
    RosterFraming {
        outer_len: 1_573_216,
        magic,
        version: ContainerVersion::RawV3,
        version_word: 3,
        uncompressed_size: None,
        compressed_size: None,
        inner_len: 1_573_188,
        table_count: 19,
    }
}

// ============================================================== NHL 07, Xbox 360 (gen6)

mod nhl07_xbox360 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl07-xbox360.toml",
            target: "nhl07-xbox360",
            cases_file: CASES_FILE,
            platform: "xbox360",
            game: "nhl07",
            main_anchor: "anchors/xbox360/nhl07/databases/NHL07NG.db",
            main_anchor_sha256: "67a00beaa05690d30527c9059146c7e610f33f892b0ddadc381e1cfefc8dbad3",
            roster: None,
        },
        module: "nhl07_xbox360",
        main_meta: "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml",
        main_meta_text_sha256: "f6e6f2a16454fb4978ee237a74c18b4ccfb7a745eecfc6419f85e4020d406d9c",
        status: Status::Target,
        twin: Twin {
            main: "anchors/ps3/nhl07/databases/NHL07NG.db",
            meta: "anchors/ps3/nhl07/databases/NHL07NG-meta.xml",
            anchor_prefix: "anchors/ps3/",
            snapshot_name: "ps3-nhl07.json",
            adapter_file: None,
        },
        save: RosterSave::Blocked {
            path: "anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1",
            sha256: "40a06d5344d4ca7460cffeb8ec0cb1626bdefb1a28ba337f17c6f9ddacd2223d",
            len: 1_598_888,
            inner_len: 10_476,
            inner_sha256: "ebe5eb0afaa7039c81ef22deed0bcb36e568f0399b7b32ead75b83dd778bed89",
            table_count: 31,
            same_as: None,
        },
        capacities: &[],
        unknown_capacities: NHL07_TABLES,
        birth_offsets: (3996, 4005, 3966),
        year: (6, 0, 29),
        companions: NHL07_COMPANIONS,
        partition: (2015, 235),
        orphans: &[],
        dxsb: (3856, 7),
        main_team: MainTeamCounts {
            rostered: 2176,
            plus_one: 0,
            same: 2176,
            unrostered_zero: 74,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 93,
        },
        rostered_on_team_zero: 26,
        largest_pattern: 92,
        handedness: (4009, &[(0, 1549), (1, 701)]),
        team_name: (
            "HqzU",
            "YmkJ",
            2048,
            2048,
            "Anaheim Ducks™",
            "Atlanta Thrashers®",
            "Mighty Ducks of Anaheim™",
        ),
        speed: (55, 2, 49),
        six_bit_max: (49, 49),
        rating: Rating::Gen6,
        population: NHL07_POPULATION,
        candidates: NHL07_CANDIDATES,
        mapped_fraction: (9, 1813),
    };

    cases!(E:
        the_roster_fixture_decodes_to_a_tdb_without_the_roster_core_so_the_save_capacity_is_blocked => run::save_blocked,
        the_xbox360_main_db_and_metadata_are_byte_identical_to_the_ps3_research_copys_and_nothing_rests_on_it => run::twin_identity,
        birth_date_month_and_day_are_stored_one_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_as_is_and_zero_is_also_team_zero => run::main_team,
        stored_zero_is_team_code_zero_as_much_as_it_is_unrostered_so_no_absence_code_is_established => run::ambiguity,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        the_gen6_six_bit_rating_regime_resolves_offset_50_on_this_anchor => run::ratings,
        team_name_lives_on_the_stock_team_table_reached_through_the_thin_team_index_and_round_trips => run::team_name,
        no_skater_block_field_is_named_for_an_overall_on_this_snapshot => run::overall_search,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 07, PS3 research copy

mod nhl07_ps3 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl07-ps3.toml",
            target: "nhl07-ps3",
            cases_file: CASES_FILE,
            platform: "ps3",
            game: "nhl07",
            main_anchor: "anchors/ps3/nhl07/databases/NHL07NG.db",
            main_anchor_sha256: "67a00beaa05690d30527c9059146c7e610f33f892b0ddadc381e1cfefc8dbad3",
            roster: None,
        },
        module: "nhl07_ps3",
        main_meta: "anchors/ps3/nhl07/databases/NHL07NG-meta.xml",
        main_meta_text_sha256: "f6e6f2a16454fb4978ee237a74c18b4ccfb7a745eecfc6419f85e4020d406d9c",
        status: Status::ResearchCopy,
        twin: Twin {
            main: "anchors/xbox360/nhl07/databases/NHL07NG.db",
            meta: "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml",
            anchor_prefix: "anchors/xbox360/",
            snapshot_name: "xbox360-nhl07.json",
            adapter_file: None,
        },
        save: RosterSave::Missing,
        capacities: &[],
        unknown_capacities: NHL07_TABLES,
        birth_offsets: (3996, 4005, 3966),
        year: (6, 0, 29),
        companions: NHL07_COMPANIONS,
        partition: (2015, 235),
        orphans: &[],
        dxsb: (3856, 7),
        main_team: MainTeamCounts {
            rostered: 2176,
            plus_one: 0,
            same: 2176,
            unrostered_zero: 74,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 93,
        },
        rostered_on_team_zero: 26,
        largest_pattern: 92,
        handedness: (4009, &[(0, 1549), (1, 701)]),
        team_name: (
            "HqzU",
            "YmkJ",
            2048,
            2048,
            "Anaheim Ducks™",
            "Atlanta Thrashers®",
            "Mighty Ducks of Anaheim™",
        ),
        speed: (55, 2, 49),
        six_bit_max: (49, 49),
        rating: Rating::Gen6,
        population: NHL07_RESEARCH_POPULATION,
        candidates: NHL07_RESEARCH_CANDIDATES,
        mapped_fraction: (9, 1813),
    };

    cases!(E:
        no_save_fixture_is_committed_so_the_save_capacity_is_missing_fixture => run::save_missing,
        the_research_copy_is_byte_identical_to_the_xbox360_anchor_and_establishes_no_ps3_release => run::research_copy,
        birth_date_month_and_day_are_stored_one_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_as_is_and_zero_is_also_team_zero => run::main_team,
        stored_zero_is_team_code_zero_as_much_as_it_is_unrostered_so_no_absence_code_is_established => run::ambiguity,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        the_gen6_six_bit_rating_regime_resolves_offset_50_on_this_anchor => run::ratings,
        team_name_lives_on_the_stock_team_table_reached_through_the_thin_team_index_and_round_trips => run::team_name,
        no_skater_block_field_is_named_for_an_overall_on_this_snapshot => run::overall_search,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 08, Xbox 360

mod nhl08_xbox360 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl08-xbox360.toml",
            target: "nhl08-xbox360",
            cases_file: CASES_FILE,
            platform: "xbox360",
            game: "nhl08",
            main_anchor: "anchors/xbox360/nhl08/databases/nhlng.db",
            main_anchor_sha256: "587204c3b18e9957c6a1530782340139ac743b9d31b84706b350fea7ddc858b9",
            roster: None,
        },
        module: "nhl08_xbox360",
        main_meta: "anchors/xbox360/nhl08/databases/nhlng-meta.xml",
        main_meta_text_sha256: "62eed788df11ad36486c7eaf38bee4c6001245d16c39937a4ff1fed0fe204374",
        status: Status::Target,
        twin: Twin {
            main: "anchors/ps3/nhl08/databases/nhlng.db",
            meta: "anchors/ps3/nhl08/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/ps3/",
            snapshot_name: "ps3-nhl08.json",
            adapter_file: None,
        },
        save: RosterSave::Blocked {
            path: "anchors/xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658",
            sha256: "b7a378359db02bc4eb5feec12d2c75e2fa6892386cf288dcf5a77980fe035755",
            len: 1_238_480,
            inner_len: 14_780,
            inner_sha256: "2b4cf53dcbd5c240813e69b94ef88a524ca5863c22fd9024e399c7d1af839e74",
            table_count: 30,
            same_as: Some("anchors/ps3/nhl08/savefiles/BLUS30062-002-000 - ROSTER/SYS-DATA"),
        },
        capacities: &[],
        unknown_capacities: EIGHT_TABLES,
        birth_offsets: (1396, 1404, 1373),
        year: (7, 0, 30),
        companions: GEN7_COMPANIONS,
        partition: (2647, 315),
        orphans: &[],
        dxsb: (0, 8),
        main_team: MainTeamCounts {
            rostered: 2276,
            plus_one: 0,
            same: 2276,
            unrostered_zero: 686,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 138,
        },
        rostered_on_team_zero: 23,
        largest_pattern: 106,
        handedness: (1408, &[(0, 2099), (1, 863)]),
        team_name: (
            "kOtt",
            "YmkJ",
            664,
            400,
            "Anaheim Ducks™",
            "Atlanta Thrashers®",
            "Mighty Ducks of Anaheim™",
        ),
        speed: (67, 2, 48),
        six_bit_max: (49, 50),
        rating: Rating::Unresolved {
            probe: 36,
            over_49: &["pJbR", "qeTq"],
        },
        population: NHL08_POPULATION,
        candidates: UNRESOLVED_RATING_CANDIDATES,
        mapped_fraction: (9, 1578),
    };

    cases!(E:
        the_roster_fixture_decodes_to_a_tdb_without_the_roster_core_so_the_save_capacity_is_blocked => run::save_blocked,
        the_xbox360_main_db_and_metadata_are_byte_identical_to_ps3s_and_nothing_rests_on_it => run::twin_identity,
        birth_date_month_and_day_are_stored_one_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_as_is_and_zero_is_also_team_zero => run::main_team,
        stored_zero_is_team_code_zero_as_much_as_it_is_unrostered_so_no_absence_code_is_established => run::ambiguity,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        the_six_bit_rating_regime_is_unresolved_on_this_anchor_because_no_audit_row_names_this_title => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        no_skater_block_field_is_named_for_an_overall_on_this_snapshot => run::overall_search,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 08, PS3

mod nhl08_ps3 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl08-ps3.toml",
            target: "nhl08-ps3",
            cases_file: CASES_FILE,
            platform: "ps3",
            game: "nhl08",
            main_anchor: "anchors/ps3/nhl08/databases/nhlng.db",
            main_anchor_sha256: "587204c3b18e9957c6a1530782340139ac743b9d31b84706b350fea7ddc858b9",
            roster: None,
        },
        module: "nhl08_ps3",
        main_meta: "anchors/ps3/nhl08/databases/nhlng-meta.xml",
        main_meta_text_sha256: "62eed788df11ad36486c7eaf38bee4c6001245d16c39937a4ff1fed0fe204374",
        status: Status::Target,
        twin: Twin {
            main: "anchors/xbox360/nhl08/databases/nhlng.db",
            meta: "anchors/xbox360/nhl08/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/xbox360/",
            snapshot_name: "xbox360-nhl08.json",
            adapter_file: None,
        },
        save: RosterSave::Blocked {
            path: "anchors/ps3/nhl08/savefiles/BLUS30062-002-000 - ROSTER/SYS-DATA",
            sha256: "b7a378359db02bc4eb5feec12d2c75e2fa6892386cf288dcf5a77980fe035755",
            len: 1_238_480,
            inner_len: 14_780,
            inner_sha256: "2b4cf53dcbd5c240813e69b94ef88a524ca5863c22fd9024e399c7d1af839e74",
            table_count: 30,
            same_as: Some(
                "anchors/xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658",
            ),
        },
        capacities: &[],
        unknown_capacities: EIGHT_TABLES,
        birth_offsets: (1396, 1404, 1373),
        year: (7, 0, 30),
        companions: GEN7_COMPANIONS,
        partition: (2647, 315),
        orphans: &[],
        dxsb: (0, 8),
        main_team: MainTeamCounts {
            rostered: 2276,
            plus_one: 0,
            same: 2276,
            unrostered_zero: 686,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 138,
        },
        rostered_on_team_zero: 23,
        largest_pattern: 106,
        handedness: (1408, &[(0, 2099), (1, 863)]),
        team_name: (
            "kOtt",
            "YmkJ",
            664,
            400,
            "Anaheim Ducks™",
            "Atlanta Thrashers®",
            "Mighty Ducks of Anaheim™",
        ),
        speed: (67, 2, 48),
        six_bit_max: (49, 50),
        rating: Rating::Unresolved {
            probe: 36,
            over_49: &["pJbR", "qeTq"],
        },
        population: NHL08_POPULATION,
        candidates: UNRESOLVED_RATING_CANDIDATES,
        mapped_fraction: (9, 1578),
    };

    cases!(E:
        the_roster_fixture_decodes_to_a_tdb_without_the_roster_core_so_the_save_capacity_is_blocked => run::save_blocked,
        the_ps3_main_db_and_metadata_are_byte_identical_to_xbox360s_and_nothing_rests_on_it => run::twin_identity,
        birth_date_month_and_day_are_stored_one_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_as_is_and_zero_is_also_team_zero => run::main_team,
        stored_zero_is_team_code_zero_as_much_as_it_is_unrostered_so_no_absence_code_is_established => run::ambiguity,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        the_six_bit_rating_regime_is_unresolved_on_this_anchor_because_no_audit_row_names_this_title => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        no_skater_block_field_is_named_for_an_overall_on_this_snapshot => run::overall_search,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 09, Xbox 360

mod nhl09_xbox360 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl09-xbox360.toml",
            target: "nhl09-xbox360",
            cases_file: CASES_FILE,
            platform: "xbox360",
            game: "nhl09",
            main_anchor: "anchors/xbox360/nhl09/databases/nhlng.db",
            main_anchor_sha256: "0f7c826b2b905c5cbbdd7706ce89efd8723ff86a7d734dc52878d9ecd18cd6cb",
            roster: Some(RosterFixture {
                path:
                    "anchors/xbox360/nhl09/savefiles/ROSTER1 20260717230804/ROSTER1 20260717230804",
                sha256: "9133606dd85dff524b3267a95b142a66f0c6ee98fa393dc2a5319bde6ece5c60",
                payload_sha256: "ad8b54d54efe974d04a16e77aced8994c5bc1fc9d326d470768346b247f4774d",
            }),
        },
        module: "nhl09_xbox360",
        main_meta: "anchors/xbox360/nhl09/databases/nhlng-meta.xml",
        main_meta_text_sha256: "2964a0de5bdc3f9fcbdce7b148a07ab42304a6a727abacae813c21370b7df6a0",
        status: Status::Target,
        twin: Twin {
            main: "anchors/ps3/nhl09/databases/nhlng.db",
            meta: "anchors/ps3/nhl09/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/ps3/",
            snapshot_name: "ps3-nhl09.json",
            adapter_file: Some("nhl09-ps3.toml"),
        },
        save: RosterSave::Decoded(nhl09_framing("Rosters")),
        capacities: NHL09_CAPS,
        unknown_capacities: &[],
        birth_offsets: (1399, 1407, 1304),
        year: (8, 2, 91),
        companions: NHL09_COMPANIONS,
        partition: (3549, 415),
        orphans: &[3746, 3750, 3758, 3762],
        dxsb: (0, 8),
        main_team: MainTeamCounts {
            rostered: 3621,
            plus_one: 0,
            same: 3621,
            unrostered_zero: 347,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 188,
        },
        rostered_on_team_zero: 23,
        largest_pattern: 157,
        handedness: (1411, &[(0, 2874), (1, 1094)]),
        team_name: (
            "kOtt",
            "YmkJ",
            656,
            400,
            "Anaheim Ducks™",
            "Atlanta Thrashers®",
            "Mighty Ducks of Anaheim™",
        ),
        speed: (71, 2, 47),
        six_bit_max: (49, 49),
        rating: Rating::Unresolved {
            probe: 50,
            over_49: &[],
        },
        population: NHL09_POPULATION,
        candidates: UNRESOLVED_RATING_CANDIDATES,
        mapped_fraction: (9, 1547),
    };

    cases!(E:
        capacities_are_the_observed_directory_of_the_committed_roster_fixture => run::capacities,
        a_capacity_that_is_not_read_from_this_fixture_is_reported => run::capacity_misattribution_is_reported,
        the_xbox360_main_db_and_metadata_are_byte_identical_to_ps3s_and_nothing_rests_on_it => run::twin_identity,
        a_capacity_carried_from_the_ps3_adapter_is_reported_against_this_fixture => run::twin_capacity_provenance,
        birth_date_month_and_day_are_stored_one_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_except_four_skater_stubs_without_companions => run::position,
        main_team_pointer_is_the_roster_slot_team_as_is_and_zero_is_also_team_zero => run::main_team,
        stored_zero_is_team_code_zero_as_much_as_it_is_unrostered_so_no_absence_code_is_established => run::ambiguity,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        the_six_bit_rating_regime_is_unresolved_on_this_anchor_because_no_audit_row_names_this_title => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        no_skater_block_field_is_named_for_an_overall_on_this_snapshot => run::overall_search,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 09, PS3

mod nhl09_ps3 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl09-ps3.toml",
            target: "nhl09-ps3",
            cases_file: CASES_FILE,
            platform: "ps3",
            game: "nhl09",
            main_anchor: "anchors/ps3/nhl09/databases/nhlng.db",
            main_anchor_sha256: "0f7c826b2b905c5cbbdd7706ce89efd8723ff86a7d734dc52878d9ecd18cd6cb",
            roster: Some(RosterFixture {
                path: "anchors/ps3/nhl09/savefiles/BLUS3016802002026 - ROSTER/SYS-DATA",
                sha256: "ee886e3447238e965cd711faa0c476613a00c8c1dadbda0a1b8440a29c1e7693",
                payload_sha256: "862c0eba66e893ccb5532b97cfe74a9e6422016fc935a42e84e75218a8e03048",
            }),
        },
        module: "nhl09_ps3",
        main_meta: "anchors/ps3/nhl09/databases/nhlng-meta.xml",
        main_meta_text_sha256: "2964a0de5bdc3f9fcbdce7b148a07ab42304a6a727abacae813c21370b7df6a0",
        status: Status::Target,
        twin: Twin {
            main: "anchors/xbox360/nhl09/databases/nhlng.db",
            meta: "anchors/xbox360/nhl09/databases/nhlng-meta.xml",
            anchor_prefix: "anchors/xbox360/",
            snapshot_name: "xbox360-nhl09.json",
            adapter_file: Some("nhl09-xbox360.toml"),
        },
        save: RosterSave::Decoded(nhl09_framing("RosterFile")),
        capacities: NHL09_CAPS,
        unknown_capacities: &[],
        birth_offsets: (1399, 1407, 1304),
        year: (8, 2, 91),
        companions: NHL09_COMPANIONS,
        partition: (3549, 415),
        orphans: &[3746, 3750, 3758, 3762],
        dxsb: (0, 8),
        main_team: MainTeamCounts {
            rostered: 3621,
            plus_one: 0,
            same: 3621,
            unrostered_zero: 347,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 188,
        },
        rostered_on_team_zero: 23,
        largest_pattern: 157,
        handedness: (1411, &[(0, 2874), (1, 1094)]),
        team_name: (
            "kOtt",
            "YmkJ",
            656,
            400,
            "Anaheim Ducks™",
            "Atlanta Thrashers®",
            "Mighty Ducks of Anaheim™",
        ),
        speed: (71, 2, 47),
        six_bit_max: (49, 49),
        rating: Rating::Unresolved {
            probe: 50,
            over_49: &[],
        },
        population: NHL09_POPULATION,
        candidates: UNRESOLVED_RATING_CANDIDATES,
        mapped_fraction: (9, 1547),
    };

    cases!(E:
        capacities_are_the_observed_directory_of_the_committed_roster_fixture => run::capacities,
        a_capacity_that_is_not_read_from_this_fixture_is_reported => run::capacity_misattribution_is_reported,
        the_ps3_main_db_and_metadata_are_byte_identical_to_xbox360s_and_nothing_rests_on_it => run::twin_identity,
        a_capacity_carried_from_the_xbox360_adapter_is_reported_against_this_fixture => run::twin_capacity_provenance,
        birth_date_month_and_day_are_stored_one_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_except_four_skater_stubs_without_companions => run::position,
        main_team_pointer_is_the_roster_slot_team_as_is_and_zero_is_also_team_zero => run::main_team,
        stored_zero_is_team_code_zero_as_much_as_it_is_unrostered_so_no_absence_code_is_established => run::ambiguity,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        the_six_bit_rating_regime_is_unresolved_on_this_anchor_because_no_audit_row_names_this_title => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        no_skater_block_field_is_named_for_an_overall_on_this_snapshot => run::overall_search,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}
