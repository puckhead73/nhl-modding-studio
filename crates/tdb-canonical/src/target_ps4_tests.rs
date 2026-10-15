//! Seven PS4 (gen8, little-endian) targets from package C0-REG4 — NHL 15 through NHL 21 — semantic
//! and capacity declarations checked against **each target's own committed evidence**, through the
//! mechanics of `target_evidence`, with the mechanics that PS4 genuinely needs kept apart from the
//! gen7 ones.
//!
//! One measurement runner exists per fact; every target supplies its identities and its
//! expectations in an [`Expect`] and declares the same named cases through the `cases!` macro, so
//! the adapter provenance strings (`<this file>#<module>::<case>`) are the test names by
//! construction. Nothing is shared *between* targets, and nothing is carried from a gen7 title: a
//! number appears in exactly one `Expect`.
//!
//! What is measured, each on the exact file the adapter attributes it to:
//!
//! * capacities on the committed **main anchor** (`nhl_game.db`), whose directory is not packed on
//!   this generation — `source = "main-db-max-records"`; the ROSTER save's own directory is a
//!   separate measurement, made where the committed production PS4 codec decodes the committed
//!   fixture (NHL 16, 17, 18, 19), recorded as blocked where it does not (NHL 15: the codec does not
//!   recognise the `DATA` stream) and as missing where no ROSTER save is committed (NHL 20, 21). A
//!   main-DB figure never stands in for a save figure or the reverse;
//! * birth-date index origin, from the stored value ranges — 0-origin on all seven, each read off
//!   its own anchor;
//! * the position discriminator and companion rule, labels from EA's metadata table names;
//! * the main-team pointer's encoding, from the metadata-declared roster key: slot team + 1, at the
//!   width the target's own snapshot records (8, 9 or 10 bits), with pattern 0 stored only on
//!   un-rostered players — and the un-rostered players that store a NON-ZERO pattern counted by
//!   pattern rather than silently passed;
//! * the 7-bit ratings' stored ranges, which rule out the gen7 6-bit offset regimes on the anchor;
//! * `team.name` read / edit / serialize / reopen on an in-memory copy of the little-endian anchor,
//!   on `fullname` (NHL 15) or `teamname` (NHL 17–21) at the target's own offset and width, and the
//!   NHL 16 team table's lack of either field recorded as an unresolved omission;
//! * the sentinel → storage-decode → index-mapping order over every stored `ubPc.dXSB` pattern,
//!   with the **test helper** of `target_evidence` — no production adapter executor exists or is
//!   claimed.
//!
//! **Attribution (C0-REG4-R1).** The files committed under `anchors/ps4/nhl19/` and
//! `anchors/ps4/nhl20/` are byte-identical (main DB, metadata, the four shared auxiliary databases),
//! and no committed capture provenance says which title either copy is. For those two targets every
//! measurement above is still made — it is a fact about the bytes — but the adapter carries it as a
//! **candidate under an unknown block**, every mapping is blocked with unresolved transforms, and
//! the runners assert both halves: the measurement agrees with the candidate, and no canonical API
//! (`human_range`, `scalar_editable`, `registry_population`, the capacity checks, the codec-order
//! helper) treats any of it as established. Registered representation, not target coverage.
//!
//! Each case ends by asserting that the adapter declares exactly what was measured. What a case
//! could NOT establish stays a candidate: handedness (no independently supported label here) and the
//! main-team parent table (containment plus a name is not a foreign key — and on NHL 18–21 the
//! containment itself fails for 44 un-rostered rows, which the adapters record against the
//! candidate).

use super::*;
use crate::target_evidence::*;
use std::collections::{BTreeMap, BTreeSet};
use tdb_core::TdbFile;

const CASES_FILE: &str = "crates/tdb-canonical/src/target_ps4_tests.rs";

/// Another title's folder on this platform whose main DB and metadata are byte-identical to this
/// target's — and, with it, whether that leaves the target's attribution established or not.
struct Twin {
    main: &'static str,
    meta: &'static str,
    /// The other folder's auxiliary databases that are byte-identical too (relative to `anchors/ps4/<game>/databases/`).
    shared_aux: &'static [&'static str],
    anchor_prefix: &'static str,
    snapshot_name: &'static str,
}

/// Whether committed evidence attributes the anchor's bytes to this target.
enum Attribution {
    /// The folder label is uncontradicted (as for every other anchor in the repository).
    Established,
    /// Two folders, two titles, one set of bytes, no capture provenance: every fact is a fixture
    /// fact carried as a candidate; nothing is operative for the target.
    Unresolved(Twin),
}

/// What this title commits as a ROSTER save and what the committed production codec makes of it.
enum RosterSave {
    /// Decoded by `ps4_extract_roster`. The directory comparison with the main anchor is an
    /// observation about two files: `max_records` equal on every roster-core table, populations
    /// equal (a vanilla or unedited roster) or not (a community roster).
    Decoded {
        framing: Ps4RosterFraming,
        populations_equal_main: bool,
    },
    /// Committed, but the committed codec does not decode it: the save capacity is blocked.
    Unsupported {
        path: &'static str,
        sha256: &'static str,
        len: usize,
        error_contains: &'static str,
    },
    /// No ROSTER save is committed. The Threes-mode save the codec does decode is recorded as an
    /// observation only: whether its `max_records` equal the main anchor's on the roster core. The
    /// disagreement's cause (mode projection, update, edit, capture) is not attributed.
    Missing {
        threes: &'static str,
        threes_sha256: &'static str,
        threes_max_equal_main: bool,
    },
}

/// `team.name` on this target: the mapped field, or the omission.
enum TeamName {
    /// (field id, bit offset, bit width, record 0, record 1, replacement written).
    Mapped(
        &'static str,
        u32,
        u32,
        &'static str,
        &'static str,
        &'static str,
    ),
    /// No `fullname` / `teamname` field; the `nickname` field's record 0 / 1 text, recorded only.
    Unresolved(&'static str, &'static str),
}

/// One target's expectations. Every number was measured on the files `target` names (C0-REG4
/// probe, re-measured by the runners below); none is carried from another target.
struct Expect {
    target: Target,
    module: &'static str,
    main_meta: &'static str,
    /// Text identity of the metadata XML (`target_evidence::text_identity_sha256`), not a byte hash.
    main_meta_text_sha256: &'static str,
    attribution: Attribution,
    save: RosterSave,
    /// (table, max_records, current_records) as stored in the MAIN anchor's directory.
    capacities: &'static [(&'static str, u16, u16)],
    /// Bit offsets of the day / month / year components on this anchor.
    birth_offsets: (u32, u32, u32),
    /// The stored range of the raw 8-bit year count on this anchor.
    year_range: (u64, u64),
    /// (skaters, goalies) by companion ownership.
    partition: (usize, usize),
    /// (bit offset, bit width) of `ubPc.dXSB` on this anchor.
    dxsb: (u32, u32),
    main_team: MainTeamCounts,
    /// The non-zero patterns stored on un-rostered rows, by pattern, and how many rows each.
    unrostered_patterns: &'static [(u64, usize)],
    /// The largest `kOtt.vfEq` key on this anchor (containment is checked against the key SET).
    largest_team_key: u64,
    /// The largest `ubPc.dXSB` pattern stored on the anchor, rostered or not.
    largest_pattern: u64,
    /// `ubPc.GRkp` bit offset and stored histogram.
    handedness: (u32, &'static [(u64, usize)]),
    team_name: TeamName,
    /// Stored (min, max) of `dSvy.eWUF` (`c_overall`) and `dSvy.AdEb` (`c_speed`).
    ratings: ((u64, u64), (u64, u64)),
    /// (mappings, omissions, established, unknown, candidate).
    population: (usize, usize, usize, usize, usize),
    candidates: &'static [&'static str],
    mapped_fraction: (u32, u32),
}

impl Expect {
    fn case(&self, name: &str) -> String {
        format!("{}::{name}", self.module)
    }

    fn unresolved(&self) -> bool {
        matches!(self.attribution, Attribution::Unresolved(_))
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

    /// Every capacity the adapter declares is the observed directory capacity of the committed
    /// MAIN anchor — `max_records` as stored, `current_records` as stored — read through the TDB
    /// parser. ESTABLISHED (attributed targets): those figures, and that on this generation the
    /// main DB is not packed on the roster core. Unresolved targets: the same figures are the
    /// candidates, and `main_db_capacity_discrepancies` reports every block as not established.
    /// NOT ESTABLISHED: that the game accepts growth up to `max_records`, and anything about the
    /// ROSTER save's directory (a separate case).
    pub(super) fn capacities(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        assert_eq!(db.endian, tdb_core::Endian::Little);
        assert!(db.header_crc_ok() && db.size_ok());
        let slack = e
            .capacities
            .iter()
            .filter(|(id, max, cur)| *id != "kOtt" && max > cur)
            .count();
        assert_eq!(
            slack, 7,
            "the seven roster-bearing tables carry slack in the main DB"
        );
        if !e.unresolved() {
            e.target
                .assert_capacities_are_the_main_anchors(&db, e.capacities, &e.case(case));
            return;
        }
        for (id, max, cur) in e.capacities {
            let t = table(&db, id);
            assert_eq!((t.max_records, t.current_records), (*max, *cur), "{id}");
        }
        let a = e.target.adapter();
        let found = e.target.main_db_capacity_discrepancies(&a, &db);
        assert_eq!(found.len(), e.capacities.len());
        assert!(
            found.iter().all(|f| f.ends_with("not established")),
            "nothing is operative: {found:?}"
        );
        assert_eq!(a.capacities.len(), e.capacities.len());
        for cap in &a.capacities {
            let c = candidate_of(&cap.declared, &format!("capacity {}", cap.table));
            let v = c.value.as_ref().unwrap();
            let (_, max, cur) = e
                .capacities
                .iter()
                .find(|(id, _, _)| *id == cap.table)
                .unwrap();
            assert_eq!(
                (v.ceiling_records, v.population_records, v.source.as_str()),
                (u32::from(*max), u32::from(*cur), "main-db-max-records"),
                "{}",
                cap.table
            );
            assert_eq!(c.confidence.as_deref(), Some("measured"));
            assert!(e.target.cites(&c.provenance, &e.case(case)));
            assert!(c.provenance.iter().any(|p| p == e.target.main_anchor));
            assert!(c.note.contains("not a demonstrated runtime limit"));
            assert!(c.note.contains("NOT a substitute"));
        }
        // Promoted, the candidates ARE the anchor's directory — the numbers are right, and only
        // the attribution keeps them from being operative.
        assert!(e
            .target
            .main_db_capacity_discrepancies(&promoted(&a), &db)
            .is_empty());
    }

    /// A capacity attributed to any other file — the ROSTER save included — is reported, and so is
    /// one whose provenance omits the anchor, or whose figure is off by one. On an unresolved
    /// target the base adapter itself is reported (nothing established) and the mutations are run
    /// on the promoted copy.
    pub(super) fn capacity_misattribution_is_reported(e: &Expect, _case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let declared = e.target.adapter();
        let base = if e.unresolved() {
            let found = e.target.main_db_capacity_discrepancies(&declared, &db);
            assert_eq!(found.len(), e.capacities.len());
            assert!(found.iter().all(|f| f.ends_with("not established")));
            promoted(&declared)
        } else {
            declared
        };
        assert!(e
            .target
            .main_db_capacity_discrepancies(&base, &db)
            .is_empty());

        let mut save_sourced = base.clone();
        for cap in &mut save_sourced.capacities {
            cap.declared.value.as_mut().unwrap().source = "roster-save-max-records".into();
        }
        let found = e.target.main_db_capacity_discrepancies(&save_sourced, &db);
        assert_eq!(found.len(), e.capacities.len());
        assert!(found
            .iter()
            .all(|f| f.contains("is not the main anchor this case parses")));

        let mut unattributed = base.clone();
        for cap in &mut unattributed.capacities {
            cap.declared
                .provenance
                .retain(|p| p != e.target.main_anchor);
        }
        let found = e.target.main_db_capacity_discrepancies(&unattributed, &db);
        assert_eq!(found.len(), e.capacities.len());
        assert!(found
            .iter()
            .all(|f| f.ends_with("provenance does not name the anchor it was read from")));

        let mut off_by_one = base.clone();
        off_by_one.capacities[0]
            .declared
            .value
            .as_mut()
            .unwrap()
            .ceiling_records += 1;
        let found = e.target.main_db_capacity_discrepancies(&off_by_one, &db);
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("ceiling_records"));
    }

    /// The ROSTER save, as the committed codec sees it. `Decoded`: identity, framing and the
    /// directory comparison with the main anchor, and every capacity note stating that outcome.
    /// `Unsupported`: the committed codec refuses the fixture, and every capacity note says the
    /// save capacity is BLOCKED. `Missing`: no ROSTER save directory exists, the Threes save is an
    /// observation only, and every capacity note says the save capacity is MISSING-FIXTURE.
    pub(super) fn roster_save(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let a = e.target.adapter();
        let unresolved = e.unresolved();
        let notes: Vec<(&str, &[String])> = a
            .capacities
            .iter()
            .map(|c| capacity_note_and_provenance(c, unresolved))
            .collect();
        assert_eq!(notes.len(), 8);
        for (cap, (_, prov)) in a.capacities.iter().zip(&notes) {
            assert!(
                e.target.cites(prov, &e.case(case)),
                "{}: cites the save case",
                cap.table
            );
        }
        match &e.save {
            RosterSave::Decoded {
                framing,
                populations_equal_main,
            } => {
                let inner = e.target.decoded_ps4_roster_fixture(framing);
                let save = TdbFile::parse(&inner).unwrap();
                let rows = directory_comparison(&save, &db, &ROSTER_CORE);
                assert!(
                    rows.iter().all(|(_, smax, _, mmax, _)| smax == mmax),
                    "max_records equal on the roster core: {rows:?}"
                );
                assert_eq!(
                    rows.iter().all(|(_, _, scur, _, mcur)| scur == mcur),
                    *populations_equal_main,
                    "{rows:?}"
                );
                for t in &save.tables {
                    assert!(
                        db.table_canonical(&t.id.canonical(save.endian)).is_some(),
                        "every save table exists in the main DB"
                    );
                }
                let path = e.target.roster_fixture();
                for (cap, (note, prov)) in a.capacities.iter().zip(&notes) {
                    assert!(prov.iter().any(|p| p == path));
                    assert!(note.contains("SAVE CROSS-CHECK"), "{}", cap.table);
                    assert!(note.contains("EQUALS the main anchor's"), "{}", cap.table);
                    assert_eq!(
                        note.contains("populations DIFFER"),
                        !populations_equal_main,
                        "{}",
                        cap.table
                    );
                    assert!(
                        note.contains("not the source of this figure"),
                        "{}",
                        cap.table
                    );
                }
            }
            RosterSave::Unsupported {
                path,
                sha256,
                len,
                error_contains,
            } => {
                let outer = read_pinned(path, sha256);
                assert_eq!(outer.len(), *len);
                assert!(
                    !tdb_savedata::is_ps4_save(&outer),
                    "the committed codec does not recognise this DATA stream"
                );
                let err = tdb_savedata::ps4_extract_roster(&outer)
                    .expect_err("extraction fails")
                    .to_string();
                assert!(err.contains(error_contains), "{err}");
                assert!(e.target.roster.is_none());
                for (cap, (note, prov)) in a.capacities.iter().zip(&notes) {
                    assert!(prov.iter().any(|p| p == path));
                    assert!(note.contains("SAVE CAPACITY BLOCKED"), "{}", cap.table);
                    assert!(note.contains(error_contains), "{}", cap.table);
                }
            }
            RosterSave::Missing {
                threes,
                threes_sha256,
                threes_max_equal_main,
            } => {
                let saves = repo_root()
                    .join("anchors")
                    .join(e.target.platform)
                    .join(e.target.game)
                    .join("savefiles");
                let dirs: BTreeSet<String> = std::fs::read_dir(&saves)
                    .unwrap()
                    .flatten()
                    .map(|d| d.file_name().to_string_lossy().into_owned())
                    .collect();
                assert!(
                    dirs.iter()
                        .all(|d| !d.to_ascii_lowercase().contains("roster")),
                    "no ROSTER save directory is committed: {dirs:?}"
                );
                assert_eq!(
                    dirs,
                    ["BeAGM0", "BeAPro0", "Profile0", "Season0", "ThreeOnThree0"]
                        .into_iter()
                        .map(String::from)
                        .collect()
                );
                assert!(e.target.roster.is_none());
                let outer = read_pinned(threes, threes_sha256);
                let inner = tdb_savedata::ps4_extract_roster(&outer).unwrap();
                let save = TdbFile::parse(&inner).unwrap();
                assert_eq!(save.endian, tdb_core::Endian::Little);
                assert_eq!(
                    save.tables.len(),
                    11,
                    "a Threes-mode projection, not a roster"
                );
                let rows = directory_comparison(&save, &db, &ROSTER_CORE);
                assert_eq!(
                    rows.iter().all(|(_, smax, _, mmax, _)| smax == mmax),
                    *threes_max_equal_main,
                    "{rows:?}"
                );
                assert!(
                    !rows.iter().all(|(_, _, scur, _, mcur)| scur == mcur),
                    "the Threes save's populations are not the main anchor's"
                );
                for (cap, (note, _)) in a.capacities.iter().zip(&notes) {
                    assert!(
                        note.contains("SAVE CAPACITY MISSING-FIXTURE"),
                        "{}",
                        cap.table
                    );
                    assert!(note.contains("ThreeOnThree0"), "{}", cap.table);
                    assert_eq!(
                        note.contains("DISAGREES with this anchor"),
                        !threes_max_equal_main,
                        "{}",
                        cap.table
                    );
                    assert!(note.contains("used for nothing"), "{}", cap.table);
                    assert!(
                        !note.contains("generation"),
                        "{}: the disagreement's cause is not attributed",
                        cap.table
                    );
                }
            }
        }
    }

    /// The main DB, the metadata and the shared auxiliary databases are byte-identical to another
    /// title's folder on this platform — asserted on the bytes (the XMLs additionally on their
    /// text identity). Committed provenance was inspected and establishes neither label (C0-REG4-R1);
    /// the adapter says so, cites only its own folder, and — the regressions — no canonical API
    /// yields anything established from it: every mapping is blocked with unresolved transforms,
    /// `human_range` / `scalar_editable` are `None`, `registry_population` counts nothing
    /// established, the capacity checks report every block, and the codec-order helper refuses the
    /// declared blocks. Promoted, the same file passes the established-only mechanics — which is
    /// what shows the attribution, and only the attribution, is what is missing.
    pub(super) fn attribution(e: &Expect, case: &str) {
        let Attribution::Unresolved(twin) = &e.attribution else {
            panic!("this target's attribution is unresolved")
        };
        let mine = e.target.main_anchor_bytes();
        let theirs = std::fs::read(repo_root().join(twin.main)).unwrap();
        assert_eq!(mine, theirs, "main DB: byte-identical");
        let my_meta = read_pinned_text(e.main_meta, e.main_meta_text_sha256);
        let their_meta = std::fs::read(repo_root().join(twin.meta)).unwrap();
        assert_eq!(
            text_identity_sha256(&their_meta),
            e.main_meta_text_sha256,
            "metadata: the same text (LF-normalised digest)"
        );
        assert_eq!(
            my_meta, their_meta,
            "metadata: the same bytes as checked out"
        );
        let mine_dir = repo_root()
            .join("anchors/ps4")
            .join(e.target.game)
            .join("databases");
        let theirs_dir = repo_root().join(twin.main).parent().unwrap().to_path_buf();
        for aux in twin.shared_aux {
            assert_eq!(
                std::fs::read(mine_dir.join(aux)).unwrap(),
                std::fs::read(theirs_dir.join(aux)).unwrap(),
                "{aux}: byte-identical"
            );
        }

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
        for s in [
            "ATTRIBUTION UNRESOLVED",
            "REGISTERED REPRESENTATION, NOT AN OPERATIVE",
            "e30b596",
            "not capture",
            "NOTHING in\n# this repository establishes",
            "carries NO save capacity, deployment behaviour or runtime conclusion",
            &e.case(case),
        ] {
            assert!(text.contains(s), "the adapter states it: {s:?}");
        }
        assert!(
            !text.contains("database generation"),
            "no cause is attributed to the Threes disagreement"
        );

        // ---- regressions through the canonical APIs ------------------------------------
        assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
        assert!(!a.mappings.is_empty());
        for m in &a.mappings {
            assert_eq!(m.qualification, "blocked", "{}", m.canonical);
            assert!(transform_unresolved(m), "{}", m.canonical);
            assert!(!m.inverse_verified, "{}", m.canonical);
            assert!(m.automated_case.is_none(), "{}", m.canonical);
            assert_eq!(human_range(m), None, "{}", m.canonical);
            assert_eq!(scalar_editable(m), None, "{}", m.canonical);
            assert!(
                m.note.contains("BLOCKED (C0-REG4-R1, attribution)"),
                "{}",
                m.canonical
            );
            for (block, state) in [
                ("storage", m.storage.state),
                ("status", m.status.state),
                ("transform", m.transform.state),
                ("enumeration", m.enumeration.state),
                ("null_sentinel", m.null_sentinel.state),
                ("string_storage", m.string_storage.state),
                ("foreign_key", m.foreign_key.state),
                ("applicability", m.applicability.state),
            ] {
                assert_ne!(
                    state,
                    ConstraintState::Established,
                    "{}.{block}",
                    m.canonical
                );
            }
            for c in &m.components {
                assert!(c.storage.established().is_none());
            }
        }
        assert!(a
            .capacities
            .iter()
            .all(|c| c.declared.established().is_none()));
        assert!(a
            .companion_rules
            .iter()
            .all(|r| r.declared.established().is_none()));
        assert!(a
            .position_discriminator
            .as_ref()
            .unwrap()
            .established()
            .is_none());
        let p = &registry_population(std::iter::once(&a))[0];
        assert_eq!(p.established_constraints, 0, "nothing is established");
        assert!(
            p.candidate_constraints > 0,
            "everything measured is a candidate"
        );
        assert_eq!(all_candidate_blocks(&a).len(), p.candidate_constraints);
        let main = TdbFile::parse(&mine).unwrap();
        assert_eq!(
            e.target.main_db_capacity_discrepancies(&a, &main).len(),
            a.capacities.len()
        );
        let team = mapping(&a, "player.team");
        assert!(
            std::panic::catch_unwind(|| decode_stored(team, 0)).is_err(),
            "the codec-order helper refuses blocks that are not established"
        );

        // Promoted (test-only), the same measurements pass the established-only mechanics.
        let lifted = promoted(&a);
        assert!(e
            .target
            .main_db_capacity_discrepancies(&lifted, &main)
            .is_empty());
        assert_eq!(
            human_range(mapping(&lifted, "player.rating_overall")),
            Some([0, 99])
        );
        assert_eq!(
            decode_stored(mapping(&lifted, "player.team"), 0),
            Decoded::Absent
        );
    }

    /// The birth-date index origin, measured directly on this anchor from the stored ranges:
    /// months 0..=11 and days 0..=30, which a 1-origin encoding could not produce. The adapter's
    /// `index-origin-0` transform is that measurement, corroborated by the named `studio-domain`
    /// case that lists this anchor by path — as an established block on an attributed target, as a
    /// candidate on an unresolved one (where the studio-domain case reading the path is a label,
    /// not attribution, and qualifies nothing).
    pub(super) fn birth_date(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let [year, month, day] = birth_date_ranges(&db, &e.target.main_snapshot());
        assert_eq!((month, day), ((0, 11), (0, 30)));
        assert_eq!(year, e.year_range);
        let a = e.target.adapter();
        let m = mapping(&a, "player.birth_date");
        let case_file = std::fs::read_to_string(
            repo_root().join("crates/studio-domain/tests/birthdate_encoding.rs"),
        )
        .unwrap();
        assert!(
            case_file.contains(e.target.main_anchor),
            "the named case lists this anchor by path"
        );
        let (tr, prov, conf) = if e.unresolved() {
            assert_eq!(m.qualification, "blocked");
            assert!(m.automated_case.is_none());
            assert_eq!(m.read_transform, "unresolved");
            let c = candidate_of(&m.transform, "birth_date.transform");
            (
                c.value.as_ref().unwrap(),
                &c.provenance,
                c.confidence.as_deref(),
            )
        } else {
            assert_eq!(m.qualification, "automated-tested");
            assert_eq!(m.read_transform, "index-origin-0");
            (
                m.transform.established().unwrap(),
                &m.transform.provenance,
                m.transform.confidence.as_deref(),
            )
        };
        assert_eq!(
            (tr.kind.as_str(), tr.index_origin),
            ("index-origin", Some(0))
        );
        assert_eq!(m.write_transform, m.read_transform);
        assert_eq!(conf, Some("measured"));
        assert!(e.target.cites(prov, &e.case(case)));
        assert!(e.target.cites(&m.provenance, &e.case(case)));
        let offset = |role: &str| {
            let c = &m
                .components
                .iter()
                .find(|c| c.role == role)
                .unwrap()
                .storage;
            let st = if e.unresolved() {
                candidate_value(c, &format!("birth_date.{role}.storage"))
            } else {
                c.established().unwrap()
            };
            assert_eq!(st.endian, "little");
            st.bit_offset
        };
        assert_eq!(
            (offset("day"), offset("month"), offset("year")),
            e.birth_offsets
        );
    }

    /// The discriminator, measured with EA's metadata names as the labels: the bio rows that own
    /// rows in the goalie companion tables all store 4 and nothing else; those that own skater
    /// companion rows store exactly {0, 1, 2, 3}; the two sets partition the table.
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
            Some("game_id"),
            "the shared key is named game_id on this platform"
        );
        let part = companion_partition(&db, &snap);
        assert_eq!((part.skaters.len(), part.goalies.len()), e.partition);
        let (skater_codes, goalie_codes) = position_codes_by_partition(&db, &part, 4);
        assert_eq!(skater_codes, BTreeSet::from([0, 1, 2, 3]));
        assert_eq!(goalie_codes, BTreeSet::from([4]));

        let a = e.target.adapter();
        let rated = ["player.rating_overall", "player.rating_speed"];
        if e.unresolved() {
            let disc = a.position_discriminator.as_ref().unwrap();
            let c = candidate_of(disc, "position_discriminator");
            let v = c.value.as_ref().unwrap();
            assert_eq!(
                (v.skater_codes.as_slice(), v.goalie_codes.as_slice()),
                (&[0, 1, 2, 3][..], &[4][..])
            );
            assert_eq!(c.confidence.as_deref(), Some("measured"));
            assert!(e.target.cites(&c.provenance, &e.case(case)));
            let rule = &a.companion_rules[0].declared;
            let c = candidate_of(rule, "companion_rule");
            assert_eq!(c.value.as_ref().unwrap().companions.len(), 4);
            assert!(e.target.cites(&c.provenance, &e.case(case)));
            for canonical in rated {
                let m = mapping(&a, canonical);
                let ap = candidate_value(&m.applicability, canonical);
                assert_eq!(
                    (ap.kind.as_str(), ap.codes.as_slice()),
                    ("target-predicate", &[0, 1, 2, 3][..])
                );
            }
            // Promoted, the shape is exactly what the established-only check demands.
            assert_discriminator_declared(
                &e.target,
                &promoted(&a),
                &[0, 1, 2, 3],
                &[4],
                &rated,
                GEN7_COMPANIONS,
                &e.case(case),
            );
        } else {
            assert_discriminator_declared(
                &e.target,
                &a,
                &[0, 1, 2, 3],
                &[4],
                &rated,
                GEN7_COMPANIONS,
                &e.case(case),
            );
        }
        assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
    }

    /// The main-team pointer `ubPc.dXSB` against the roster reached through the metadata-declared
    /// key, at this anchor's width: for each rostered player exactly one slot carries
    /// `team == pattern - 1`; no rostered player stores 0; the all-ones pattern is stored nowhere;
    /// un-rostered players store 0 — except those that store the non-zero patterns listed in
    /// `unrostered_patterns`, which the adapter must record by pattern and count. ESTABLISHED (or
    /// carried as candidates): the encoding and the absence pattern. NOT ESTABLISHED: the parent;
    /// where the un-rostered patterns exceed every `kOtt.vfEq` key, the adapter must say
    /// containment fails for them.
    pub(super) fn main_team(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        let slots = roster_slots_by_bio_record(&db, &snap);
        let counts = main_team_measurement(
            &db,
            &snap,
            &slots,
            MainTeamEncoding::SlotTeamPlusOne,
            e.dxsb.1,
        );
        assert_eq!(counts, e.main_team);
        assert_eq!(counts.plus_one, counts.rostered);

        let bio = table(&db, "ubPc");
        let dxsb = field(&db, bio, "dXSB");
        assert_eq!((dxsb.bit_offset, dxsb.bit_width), e.dxsb);
        let mut unrostered: BTreeMap<u64, usize> = BTreeMap::new();
        for i in 0..u32::from(bio.current_records) {
            let raw = read(&db, bio, i, &dxsb);
            if !slots.contains_key(&i) && raw != 0 {
                *unrostered.entry(raw).or_default() += 1;
            }
        }
        assert_eq!(
            unrostered,
            e.unrostered_patterns
                .iter()
                .copied()
                .collect::<BTreeMap<_, _>>()
        );
        assert_eq!(unrostered.values().sum::<usize>(), counts.unrostered_other);
        let kott = table(&db, "kOtt");
        let key = field(&db, kott, "vfEq");
        let largest_key = (0..u32::from(kott.current_records))
            .map(|i| read(&db, kott, i, &key))
            .max()
            .unwrap();
        assert_eq!(largest_key, e.largest_team_key);

        let a = e.target.adapter();
        let m = mapping(&a, "player.team");
        let sentinel_note: &str = if e.unresolved() {
            assert_eq!(m.qualification, "blocked");
            assert!(transform_unresolved(m));
            let st = candidate_value(&m.storage, "team.storage");
            assert_eq!(
                (
                    st.signedness.as_str(),
                    st.bit_width,
                    st.range_low,
                    st.range_high
                ),
                ("offset-binary", e.dxsb.1, -1, (1i64 << e.dxsb.1) - 2)
            );
            let tr = candidate_value(&m.transform, "team.transform");
            assert_eq!(
                (tr.kind.as_str(), tr.index_origin),
                ("index-origin", Some(0))
            );
            let s = candidate_of(&m.null_sentinel, "team.null_sentinel");
            let sv = s.value.as_ref().unwrap();
            assert_eq!(
                (
                    sv.encoding.as_str(),
                    sv.raw,
                    sv.canonical_meaning.as_deref()
                ),
                ("literal-code", Some(0), Some("absent"))
            );
            assert_eq!(s.confidence.as_deref(), Some("measured"));
            assert!(e.target.cites(&s.provenance, &e.case(case)));
            assert_eq!(m.foreign_key.state, ConstraintState::Unknown);
            assert_main_team_declared(
                &e.target,
                &promoted(&a),
                &e.case(case),
                MainTeamEncoding::SlotTeamPlusOne,
                e.dxsb.1,
            );
            &s.note
        } else {
            assert_main_team_declared(
                &e.target,
                &a,
                &e.case(case),
                MainTeamEncoding::SlotTeamPlusOne,
                e.dxsb.1,
            );
            &m.null_sentinel.note
        };
        for (pattern, n) in e.unrostered_patterns {
            for text in [m.note.as_str(), sentinel_note] {
                assert!(
                    text.contains(&format!("pattern {pattern}")) && text.contains(&n.to_string()),
                    "the un-rostered pattern {pattern} x{n} is recorded, not smoothed over"
                );
            }
        }
        let fk = m.foreign_key.candidate.as_ref().unwrap();
        let exceeds = e
            .unrostered_patterns
            .iter()
            .any(|(p, _)| p - 1 > e.largest_team_key);
        assert_eq!(
            fk.note.contains("containment fails"),
            exceeds,
            "evidence against the parent candidate is recorded exactly when it exists"
        );
        assert!(fk
            .note
            .contains(&format!("largest key {}", e.largest_team_key)));
    }

    /// The `SENTINEL_ENCODING` processing order over every stored `ubPc.dXSB` pattern on this
    /// anchor, with the test helper: pattern 0 is absence before any decode; every pattern on a
    /// rostered row decodes to one of its slot team codes; the un-rostered rows that store a
    /// non-zero pattern decode to a code that resolves to no slot and are returned by pattern; the
    /// inverse restores each of the 2^w patterns. Codec behaviour only; no executor is claimed. On
    /// an unresolved target the declared blocks are refused by the helper and the order is shown
    /// on the promoted copy.
    pub(super) fn codec_order(e: &Expect, _case: &str) {
        let declared = e.target.adapter();
        let a = if e.unresolved() {
            let m = mapping(&declared, "player.team");
            assert!(std::panic::catch_unwind(|| decode_stored(m, 0)).is_err());
            promoted(&declared)
        } else {
            declared
        };
        let m = mapping(&a, "player.team");
        assert_eq!(m.foreign_key.state, ConstraintState::Unknown);
        let width = e.dxsb.1;
        let all_ones = (1u64 << width) - 1;
        assert_eq!(decode_stored(m, 0), Decoded::Absent);
        assert_eq!(decode_stored(m, 1), Decoded::TeamCode(0));
        assert_eq!(
            decode_stored(m, e.largest_pattern),
            Decoded::TeamCode(e.largest_pattern - 1),
            "the largest pattern stored here"
        );
        assert_eq!(
            decode_stored(m, all_ones),
            Decoded::TeamCode(all_ones - 1),
            "not a sentinel; unobserved"
        );
        assert_eq!(encode_stored(m, Decoded::Absent), 0);
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let slots = roster_slots_by_bio_record(&db, &e.target.main_snapshot());
        let (absent, coded, unrostered) =
            codec_round_trip_over_anchor_with_unrostered_pointers(m, &db, &slots);
        assert_eq!(
            (absent, coded),
            (e.main_team.unrostered_zero, e.main_team.rostered)
        );
        assert_eq!(
            unrostered,
            e.unrostered_patterns
                .iter()
                .copied()
                .collect::<BTreeMap<_, _>>()
        );
        let bio = table(&db, "ubPc");
        let f = field(&db, bio, "dXSB");
        let largest = (0..u32::from(bio.current_records))
            .map(|i| read(&db, bio, i, &f))
            .max()
            .unwrap();
        assert_eq!(largest, e.largest_pattern);
    }

    /// Handedness on this anchor: the stored histogram is a count and nothing more; the mapping
    /// stays blocked.
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
        let st = if e.unresolved() {
            candidate_value(&m.storage, "shoots.storage")
        } else {
            m.storage.established().unwrap()
        };
        assert_eq!((st.bit_offset, st.bit_width), (e.handedness.0, 1));
    }

    /// The two mapped ratings are 7-bit fields with declared range 0..99 whose stored values on
    /// this anchor reach 99 and start above 63: no 6-bit regime (offset 36 or 50) can describe
    /// them, and a maximum of 99 leaves no room for a positive offset. The adapter declares the
    /// identity transform, derived from those measured premises plus the ratings audit's PS4 row
    /// and the engine's metadata-driven classification — both asserted to say what they are cited
    /// for — with the human range derived as 0..99 and `inverse_verified` true only where the
    /// named case ran on this anchor (NHL 15). Unresolved targets carry the transform as a
    /// candidate and derive no human range at all.
    pub(super) fn ratings(e: &Expect, case: &str) {
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let snap = e.target.main_snapshot();
        let skater = table(&db, "dSvy");
        let mut measured = Vec::new();
        for (id, name) in [("eWUF", "c_overall"), ("AdEb", "c_speed")] {
            let mf = meta_field(&snap, "dSvy", id);
            assert_eq!(mf.name.as_deref(), Some(name));
            assert_eq!((mf.range_low, mf.range_high), (Some(0), Some(99)));
            let f = field(&db, skater, id);
            assert_eq!(f.bit_width, 7);
            let (lo, hi) = (0..u32::from(skater.current_records))
                .map(|r| read(&db, skater, r, &f))
                .fold((u64::MAX, 0), |(lo, hi), v| (lo.min(v), hi.max(v)));
            assert!(hi > 63, "a 6-bit code could not store {hi}");
            assert!(hi <= 99);
            measured.push((lo, hi));
        }
        assert_eq!((measured[0], measured[1]), e.ratings);

        let audit = std::fs::read_to_string(repo_root().join("docs/audits/ratings-write-codec.md"))
            .unwrap();
        assert!(audit.contains("**PS4** 7-bit (value *is* the rating) | 0..99 | **0** | 0..99"));
        let classifier =
            std::fs::read_to_string(repo_root().join("crates/studio-domain/src/attributes.rs"))
                .unwrap();
        assert!(classifier.contains("(0, 99) => Some(0),"));
        let named = std::fs::read_to_string(
            repo_root().join("crates/studio-domain/tests/ratings_write_codec.rs"),
        )
        .unwrap();
        assert!(named.contains("fn ratings_write_is_inverse_of_read_ps4_direct("));
        assert!(named.contains("anchors/ps4/nhl15/databases/nhl_game.db"));
        let ran_here = e.target.game == "nhl15";

        let a = e.target.adapter();
        for canonical in ["player.rating_overall", "player.rating_speed"] {
            let m = mapping(&a, canonical);
            assert!(e.target.cites(&m.provenance, &e.case(case)));
            assert_eq!(m.null_sentinel.state, ConstraintState::Unknown);
            if e.unresolved() {
                assert_eq!(m.qualification, "blocked");
                assert!(transform_unresolved(m));
                assert!(!m.inverse_verified);
                let c = candidate_of(&m.transform, canonical);
                let tr = c.value.as_ref().unwrap();
                assert_eq!(
                    (tr.kind.as_str(), tr.offset, tr.index_origin),
                    ("identity", None, None)
                );
                assert_eq!(c.confidence.as_deref(), Some("derived"));
                assert!(e.target.cites(&c.provenance, &e.case(case)));
                assert_eq!(
                    human_range(m),
                    None,
                    "{canonical}: no human range without attribution"
                );
                let st = candidate_value(&m.storage, canonical);
                assert_eq!((st.bit_width, st.range_low, st.range_high), (7, 0, 99));
                continue;
            }
            assert_eq!(
                (m.read_transform.as_str(), m.write_transform.as_str()),
                ("identity", "identity")
            );
            assert_eq!(m.qualification, "implemented");
            assert_eq!(m.inverse_verified, ran_here, "{canonical}");
            let tr = m.transform.established().unwrap();
            assert_eq!(
                (tr.kind.as_str(), tr.offset, tr.index_origin),
                ("identity", None, None)
            );
            assert_eq!(m.transform.confidence.as_deref(), Some("derived"));
            assert!(e.target.cites(&m.transform.provenance, &e.case(case)));
            assert_eq!(human_range(m), Some([0, 99]), "{canonical}");
            let st = m.storage.established().unwrap();
            assert_eq!((st.bit_width, st.range_low, st.range_high), (7, 0, 99));
            assert_eq!(st.endian, "little");
            assert!(m.note.contains("NO offset") || m.note.contains("no offset"));
        }
    }

    /// `team.name` read / edit / serialize / reopen on an in-memory copy of this little-endian
    /// anchor, on the field and at the offset / width this target's snapshot records. NOT
    /// ESTABLISHED: that the game reads the name from this field, any UI path, any save-container
    /// round trip — and on the `teamname` titles, that the engine reads it at all (it does not, at
    /// HEAD: a recorded finding). On an unresolved target the round trip still runs (it is a fact
    /// about the bytes) and the mapping carries it as a blocked candidate.
    pub(super) fn team_name(e: &Expect, case: &str) {
        let TeamName::Mapped(field_id, offset, width, record0, record1, replacement) = e.team_name
        else {
            panic!("this target maps team.name")
        };
        let original = e.target.main_anchor_bytes();
        let snap = e.target.main_snapshot();
        let expected_name = if field_id == "YmkJ" {
            "fullname"
        } else {
            "teamname"
        };
        assert_eq!(
            meta_field(&snap, "kOtt", field_id).name.as_deref(),
            Some(expected_name)
        );
        kott_string_round_trip(
            &original,
            field_id,
            offset,
            width,
            record0,
            record1,
            replacement,
        );
        let a = e.target.adapter();
        let m = mapping(&a, "team.name");
        assert_eq!(m.field.as_deref(), Some(field_id));
        let st = if e.unresolved() {
            assert_eq!(m.qualification, "blocked");
            assert!(m.automated_case.is_none());
            assert!(!m.inverse_verified);
            assert_eq!(
                candidate_value(&m.string_storage, "team.name.string_storage").byte_capacity,
                Some(width / 8)
            );
            assert!(e.target.cites(
                &candidate_of(&m.status, "team.name.status").provenance,
                &e.case(case)
            ));
            candidate_value(&m.storage, "team.name.storage")
        } else {
            assert_team_name_declared(&e.target, &a, "target_ps4_tests", &e.case(case), width / 8);
            m.storage.established().unwrap()
        };
        assert_eq!(
            (st.bit_offset, st.bit_width, st.endian.as_str()),
            (offset, width, "little")
        );
        if field_id != "YmkJ" {
            assert!(
                snap.tables
                    .iter()
                    .find(|t| t.canonical_id == "kOtt")
                    .unwrap()
                    .fields
                    .iter()
                    .all(|f| f.name.as_deref() != Some("fullname")),
                "no fullname field on this team table"
            );
            assert!(
                m.note.contains("FINDING") && m.note.contains("does NOT read this field"),
                "the engine gap is recorded on the mapping"
            );
        }
    }

    /// NHL 16's team table has no field named `fullname` or `teamname`; `nickname` carries the
    /// text the neighbouring titles store under those names. The adapter records `team.name` as an
    /// unresolved omission that names the string fields and the observation, and maps nothing.
    pub(super) fn team_name_unresolved(e: &Expect, case: &str) {
        let TeamName::Unresolved(record0, record1) = e.team_name else {
            panic!("this target leaves team.name unresolved")
        };
        let snap = e.target.main_snapshot();
        let kott = snap
            .tables
            .iter()
            .find(|t| t.canonical_id == "kOtt")
            .unwrap();
        let names: Vec<&str> = kott
            .fields
            .iter()
            .filter(|f| f.kind_code == 0)
            .filter_map(|f| f.name.as_deref())
            .collect();
        assert_eq!(names, ["shortname", "artabbr", "nickname", "abbrname"]);
        let main = e.target.main_anchor_bytes();
        let db = TdbFile::parse(&main).unwrap();
        let t = table(&db, "kOtt");
        let nick = field(&db, t, "cXHn");
        assert_eq!(db.read_inline_str(t, 0, &nick).unwrap(), record0.as_bytes());
        assert_eq!(db.read_inline_str(t, 1, &nick).unwrap(), record1.as_bytes());

        let a = e.target.adapter();
        assert!(a.mappings.iter().all(|m| m.canonical != "team.name"));
        let o = a
            .omissions
            .iter()
            .find(|o| o.canonical == "team.name")
            .expect("recorded");
        assert_eq!(o.reason, "unresolved-semantics");
        assert!(o.detail.contains("DISCOVERY NOTE"));
        for name in names {
            assert!(o.detail.contains(&format!("`{name}`")), "{name}");
        }
        assert!(o.detail.contains(record0) && o.detail.contains(record1));
        assert!(e.target.cites(&o.provenance, &e.case(case)));
        assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
    }

    /// What this adapter establishes and what it leaves, counted; the unmapped remainder stays
    /// recorded as unknown; the adapter cites this module and no other per-target module. On an
    /// unresolved target the count of established blocks is zero by construction and every
    /// measurement is in the candidate tally.
    pub(super) fn ledger(e: &Expect, _case: &str) {
        let a = e.target.adapter();
        assert_eq!(
            (a.generation.as_str(), a.endian.as_str()),
            ("gen8", "little")
        );
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
        if e.unresolved() {
            assert_eq!(p.established_constraints, 0);
            assert_eq!(all_candidate_blocks(&a).len(), e.candidates.len());
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
        match e.team_name {
            TeamName::Mapped(..) => assert!(a.omissions.is_empty()),
            TeamName::Unresolved(..) => {
                assert_eq!(a.omissions.len(), 1);
                assert_eq!(a.omissions[0].canonical, "team.name");
                assert_eq!(a.omissions[0].reason, "unresolved-semantics");
            }
        }
        assert!(a.omissions.iter().all(|o| o.reason != "target-absent"));
        let text = std::fs::read_to_string(
            repo_root()
                .join("defs/adapters")
                .join(e.target.adapter_file),
        )
        .unwrap();
        assert!(text.contains(&format!("{CASES_FILE}#{}::", e.module)));
        assert!(
            !text.contains("target_batch_tests.rs")
                && !text.contains("legacy_xbox360_tests.rs")
                && !text.contains("nhl15_ps3_tests.rs"),
            "no gen7 module is cited"
        );
        for other in [
            "nhl15_ps4::",
            "nhl16_ps4::",
            "nhl17_ps4::",
            "nhl18_ps4::",
            "nhl19_ps4::",
            "nhl20_ps4::",
            "nhl21_ps4::",
        ] {
            if other.trim_end_matches("::") != e.module {
                assert!(!text.contains(other), "{}: cites {other}", e.target.target);
            }
        }
        assert!(
            !text.contains("anchors/ps3/") && !text.contains("anchors/xbox360/"),
            "no gen7 anchor is cited"
        );
        assert!(
            !text.contains("offset+36") && !text.contains("offset 36"),
            "no gen7 rating regime is carried"
        );
    }
}

const EIGHT_MAPPINGS: (usize, usize, usize, usize, usize) = (8, 0, 44, 30, 6);
// Eight mappings less the five established and three unknown blocks of a `team.name` mapping.
const SEVEN_MAPPINGS_ONE_OMISSION: (usize, usize, usize, usize, usize) = (7, 1, 39, 27, 6);
/// An attribution-unresolved adapter: the 44 blocks an attributed eight-mapping adapter
/// establishes are candidates here, alongside the 6 candidates it would carry anyway; 8 × 8 mapping
/// blocks + 8 capacities + 1 companion rule + 1 discriminator = 74 unknown.
const EIGHT_MAPPINGS_UNATTRIBUTED: (usize, usize, usize, usize, usize) = (8, 0, 0, 74, 50);
const CANDIDATES: &[&str] = &[
    "player.shoots.transform",
    "player.shoots.enumeration",
    "player.shoots.null_sentinel",
    "player.rating_overall.null_sentinel",
    "player.rating_speed.null_sentinel",
    "player.team.foreign_key",
];
/// Every block of an unresolved eight-mapping adapter that carries a candidate, in declaration
/// order (see `all_candidate_blocks`).
const CANDIDATES_UNATTRIBUTED: &[&str] = &[
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
    "player.rating_overall.storage",
    "player.rating_overall.status",
    "player.rating_overall.transform",
    "player.rating_overall.null_sentinel",
    "player.rating_overall.applicability",
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
    "capacity.ubPc",
    "capacity.ZBac",
    "capacity.eGlu",
    "capacity.kOtt",
    "capacity.dSvy",
    "capacity.xmja",
    "capacity.mHuy",
    "capacity.fMVl",
    "companion_rule.ubPc",
    "position_discriminator",
];
/// The auxiliary databases both the `nhl19` and `nhl20` folders commit as the same bytes.
const NHL19_NHL20_SHARED_AUX: &[&str] = &[
    "eng_us-meta.xml",
    "entry_draft.db",
    "expansion_draft.db",
    "initial_experience.db",
    "threes_campaign.db",
];

// ============================================================== NHL 15, PS4

mod nhl15_ps4 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl15-ps4.toml",
            target: "nhl15-ps4",
            cases_file: CASES_FILE,
            platform: "ps4",
            game: "nhl15",
            main_anchor: "anchors/ps4/nhl15/databases/nhl_game.db",
            main_anchor_sha256: "ae9d1d56bf896b83b3376cc179e5bf3a6986cb594880f43de8a81e5b7e5078d8",
            // The committed ROSTER `DATA` is not decoded by the committed codec (below).
            roster: None,
        },
        module: "nhl15_ps4",
        main_meta: "anchors/ps4/nhl15/databases/nhl_game-meta.xml",
        main_meta_text_sha256: "2baa3399343a8d17da42731162963d6b272b651a5d1a4e1639d06b48d694a1c4",
        attribution: Attribution::Established,
        save: RosterSave::Unsupported {
            path: "anchors/ps4/nhl15/savefiles/84f8102f - ROSTER/DATA",
            sha256: "17bf9bfaafd8b6444ac56108a37f9e39923caf020e3514eae78ec9b6db833938",
            len: 739_913,
            error_contains: "no LZ4 roster record in DATA",
        },
        capacities: &[
            ("ubPc", 7034, 5555),
            ("ZBac", 8948, 5984),
            ("eGlu", 10619, 5523),
            ("kOtt", 246, 246),
            ("dSvy", 6479, 4996),
            ("xmja", 6479, 4996),
            ("mHuy", 855, 559),
            ("fMVl", 855, 559),
        ],
        birth_offsets: (1159, 1177, 816),
        year_range: (61, 98),
        partition: (4996, 559),
        dxsb: (48, 8),
        main_team: MainTeamCounts {
            rostered: 5107,
            plus_one: 5107,
            same: 0,
            unrostered_zero: 447,
            unrostered_other: 1,
            all_ones: 0,
            team_keys: 246,
        },
        unrostered_patterns: &[(135, 1)],
        largest_team_key: 245,
        largest_pattern: 216,
        handedness: (1181, &[(0, 3797), (1, 1758)]),
        team_name: TeamName::Mapped(
            "YmkJ",
            352,
            400,
            "Anaheim Ducks®",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim®",
        ),
        ratings: ((36, 99), (52, 95)),
        population: EIGHT_MAPPINGS,
        candidates: CANDIDATES,
        mapped_fraction: (10, 3093),
    };

    cases!(E:
        capacities_are_the_directory_of_the_committed_main_anchor => run::capacities,
        a_capacity_that_is_not_read_from_this_anchor_is_reported => run::capacity_misattribution_is_reported,
        the_roster_fixture_is_not_decoded_by_the_committed_codec_so_the_save_capacity_is_blocked => run::roster_save,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_is_stored_only_on_unrostered_players => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        seven_bit_ratings_store_the_human_value_directly_on_this_anchor => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 16, PS4

mod nhl16_ps4 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl16-ps4.toml",
            target: "nhl16-ps4",
            cases_file: CASES_FILE,
            platform: "ps4",
            game: "nhl16",
            main_anchor: "anchors/ps4/nhl16/databases/nhl_game.db",
            main_anchor_sha256: "7d95f1dbe1862fdbc3c380afb1481a8dcff3a729a8f1d49736c60c0c4b95af7c",
            // The one pristine PS4 ROSTER in the anchor set: a bare LZ4 frame (no codec byte).
            roster: Some(RosterFixture {
                path: "anchors/ps4/nhl16/savefiles/84f8102f - ROSTER/DATA",
                sha256: "7a01bf60992f22861670f71a77c5569ec6e55e9c79d8781e552e43fb838c6c5a",
                payload_sha256: "62d28ddfa5366e6eebf442e3741785e72e12e57865f8088ae1d251ee0a5df1d8",
            }),
        },
        module: "nhl16_ps4",
        main_meta: "anchors/ps4/nhl16/databases/nhl_game-meta.xml",
        main_meta_text_sha256: "2bc6f12e4ba33e3fd505067b237dce5ab245f8624d06397aebf0e126cf59053b",
        attribution: Attribution::Established,
        save: RosterSave::Decoded {
            framing: Ps4RosterFraming {
                outer_len: 1_221_291,
                inner_len: 3_393_888,
                table_count: 36,
                size_ok: true,
            },
            populations_equal_main: true,
        },
        capacities: &[
            ("ubPc", 7988, 5632),
            ("ZBac", 9955, 6087),
            ("eGlu", 10879, 5532),
            ("kOtt", 252, 252),
            ("dSvy", 7331, 5066),
            ("xmja", 7331, 5066),
            ("mHuy", 957, 566),
            ("fMVl", 957, 566),
        ],
        birth_offsets: (1342, 1377, 824),
        year_range: (61, 99),
        partition: (5066, 566),
        dxsb: (48, 8),
        main_team: MainTeamCounts {
            rostered: 5110,
            plus_one: 5110,
            same: 0,
            unrostered_zero: 522,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 252,
        },
        unrostered_patterns: &[],
        largest_team_key: 251,
        largest_pattern: 222,
        handedness: (1381, &[(0, 3781), (1, 1851)]),
        team_name: TeamName::Unresolved("Anaheim Ducks®", "Winnipeg Jets™"),
        ratings: ((36, 99), (52, 99)),
        population: SEVEN_MAPPINGS_ONE_OMISSION,
        candidates: CANDIDATES,
        mapped_fraction: (9, 3519),
    };

    cases!(E:
        capacities_are_the_directory_of_the_committed_main_anchor => run::capacities,
        a_capacity_that_is_not_read_from_this_anchor_is_reported => run::capacity_misattribution_is_reported,
        the_roster_fixture_decodes_through_the_committed_ps4_codec_and_its_directory_is_compared_with_the_main_anchor => run::roster_save,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_is_stored_only_on_unrostered_players => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        seven_bit_ratings_store_the_human_value_directly_on_this_anchor => run::ratings,
        no_field_on_the_team_table_is_named_for_the_full_team_name_so_team_name_is_unresolved => run::team_name_unresolved,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 17, PS4

mod nhl17_ps4 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl17-ps4.toml",
            target: "nhl17-ps4",
            cases_file: CASES_FILE,
            platform: "ps4",
            game: "nhl17",
            main_anchor: "anchors/ps4/nhl17/databases/nhl_game.db",
            main_anchor_sha256: "de48b02b054e989a2797a07640da25b2a295416ad1c98d3b58c704d20a4d85ff",
            // `RosterCustom0`: a re-serialized (`size_ok = false`) roster whose populations still
            // equal the main anchor's.
            roster: Some(RosterFixture {
                path: "anchors/ps4/nhl17/savefiles/RosterCustom0/DATA",
                sha256: "70143a1d6b6a021accdb1563d4e37badb3dd5f5c1ec51e2ee8ad8ef2c3fa663e",
                payload_sha256: "a4038e0fd827c5b6ceb0eac1459f0b9f704261dbcb958b373cdec7d6137a26ab",
            }),
        },
        module: "nhl17_ps4",
        main_meta: "anchors/ps4/nhl17/databases/nhl_game-meta.xml",
        main_meta_text_sha256: "be72eda5a314c1834bcd9b72db9368901e83b17eae8d54f4ce717de1047ab682",
        attribution: Attribution::Established,
        save: RosterSave::Decoded {
            framing: Ps4RosterFraming {
                outer_len: 1_448_394,
                inner_len: 5_603_080,
                table_count: 48,
                size_ok: false,
            },
            populations_equal_main: true,
        },
        capacities: &[
            ("ubPc", 9428, 6128),
            ("ZBac", 10935, 6701),
            ("eGlu", 11714, 6430),
            ("kOtt", 256, 256),
            ("dSvy", 8236, 5527),
            ("xmja", 8236, 5527),
            ("mHuy", 1192, 601),
            ("fMVl", 1192, 601),
        ],
        birth_offsets: (1431, 1468, 864),
        year_range: (39, 100),
        partition: (5527, 601),
        dxsb: (80, 8),
        main_team: MainTeamCounts {
            rostered: 5882,
            plus_one: 5882,
            same: 4,
            unrostered_zero: 246,
            unrostered_other: 0,
            all_ones: 0,
            team_keys: 256,
        },
        unrostered_patterns: &[],
        largest_team_key: 399,
        largest_pattern: 253,
        handedness: (1472, &[(0, 4031), (1, 2097)]),
        team_name: TeamName::Mapped(
            "AUsv",
            1512,
            320,
            "Anaheim Ducks®",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim®",
        ),
        ratings: ((36, 99), (52, 99)),
        population: EIGHT_MAPPINGS,
        candidates: CANDIDATES,
        mapped_fraction: (10, 3180),
    };

    cases!(E:
        capacities_are_the_directory_of_the_committed_main_anchor => run::capacities,
        a_capacity_that_is_not_read_from_this_anchor_is_reported => run::capacity_misattribution_is_reported,
        the_roster_fixture_decodes_through_the_committed_ps4_codec_and_its_directory_is_compared_with_the_main_anchor => run::roster_save,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_is_stored_only_on_unrostered_players => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        seven_bit_ratings_store_the_human_value_directly_on_this_anchor => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 18, PS4

mod nhl18_ps4 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl18-ps4.toml",
            target: "nhl18-ps4",
            cases_file: CASES_FILE,
            platform: "ps4",
            game: "nhl18",
            main_anchor: "anchors/ps4/nhl18/databases/nhl_game.db",
            main_anchor_sha256: "65eaa6e82c6dbad77b301f30284e9c2256096d78cf356d0dab7fdad722bb2704",
            roster: Some(RosterFixture {
                path: "anchors/ps4/nhl18/savefiles/RosterCustom0/DATA",
                sha256: "32b5d8189d493ead57d75e7a8cdd13745e507a6212ba3f2aa366e8c958fb3422",
                payload_sha256: "25763d39dda89e2822aab5614917e9cae6e2f5592896f7646831e243e1c5cfc8",
            }),
        },
        module: "nhl18_ps4",
        main_meta: "anchors/ps4/nhl18/databases/nhl_game-meta.xml",
        main_meta_text_sha256: "b63668793c9c640e24c84094499ea0d0d3ca96618aae76a1814db2a65723cee6",
        attribution: Attribution::Established,
        save: RosterSave::Decoded {
            framing: Ps4RosterFraming {
                outer_len: 1_514_666,
                inner_len: 5_999_830,
                table_count: 52,
                size_ok: false,
            },
            populations_equal_main: true,
        },
        capacities: &[
            ("ubPc", 10371, 7236),
            ("ZBac", 11825, 7735),
            ("eGlu", 13502, 7373),
            ("kOtt", 301, 296),
            ("dSvy", 9143, 6522),
            ("xmja", 9143, 6522),
            ("mHuy", 1228, 714),
            ("fMVl", 1228, 714),
        ],
        birth_offsets: (1460, 1497, 856),
        year_range: (35, 101),
        partition: (6522, 714),
        dxsb: (1090, 9),
        main_team: MainTeamCounts {
            rostered: 6888,
            plus_one: 6888,
            same: 0,
            unrostered_zero: 304,
            unrostered_other: 44,
            all_ones: 0,
            team_keys: 296,
        },
        unrostered_patterns: &[(501, 22), (502, 22)],
        largest_team_key: 399,
        largest_pattern: 502,
        handedness: (1501, &[(0, 4798), (1, 2438)]),
        team_name: TeamName::Mapped(
            "AUsv",
            3312,
            320,
            "Anaheim Ducks®",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim®",
        ),
        ratings: ((36, 99), (52, 96)),
        population: EIGHT_MAPPINGS,
        candidates: CANDIDATES,
        mapped_fraction: (10, 3248),
    };

    cases!(E:
        capacities_are_the_directory_of_the_committed_main_anchor => run::capacities,
        a_capacity_that_is_not_read_from_this_anchor_is_reported => run::capacity_misattribution_is_reported,
        the_roster_fixture_decodes_through_the_committed_ps4_codec_and_its_directory_is_compared_with_the_main_anchor => run::roster_save,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_is_stored_only_on_unrostered_players => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        seven_bit_ratings_store_the_human_value_directly_on_this_anchor => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 19, PS4

mod nhl19_ps4 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl19-ps4.toml",
            target: "nhl19-ps4",
            cases_file: CASES_FILE,
            platform: "ps4",
            game: "nhl19",
            main_anchor: "anchors/ps4/nhl19/databases/nhl_game.db",
            main_anchor_sha256: "265e4a68d23ce44cef3240017beda0583c2cb6d246ea6672ca5612472429f408",
            // A community roster: the ceiling survives editing, the populations do not match.
            roster: Some(RosterFixture {
                path: "anchors/ps4/nhl19/savefiles/RosterCustom0/DATA",
                sha256: "1f2f83ee4bfe4b6a52ac5756d263571c7009d694f867651449f5657907171bb5",
                payload_sha256: "36305e73a6ecc0fc01f5b75ba5aea93dc93123bc22c14a871005c593f1412156",
            }),
        },
        module: "nhl19_ps4",
        main_meta: "anchors/ps4/nhl19/databases/nhl_game-meta.xml",
        main_meta_text_sha256: "969a0f20435331f76c9841a2d259b4c8eb15b825e1b648fa0bfc6635d9870893",
        // C0-REG4-R1: the bytes committed under nhl19 and nhl20 are the same; no committed
        // provenance says which title they are, so nothing here is operative for this target.
        attribution: Attribution::Unresolved(Twin {
            main: "anchors/ps4/nhl20/databases/nhl_game.db",
            meta: "anchors/ps4/nhl20/databases/nhl_game-meta.xml",
            shared_aux: NHL19_NHL20_SHARED_AUX,
            anchor_prefix: "anchors/ps4/nhl20/",
            snapshot_name: "ps4-nhl20.json",
        }),
        save: RosterSave::Decoded {
            framing: Ps4RosterFraming {
                outer_len: 1_734_662,
                inner_len: 7_557_054,
                table_count: 54,
                size_ok: false,
            },
            populations_equal_main: false,
        },
        capacities: &[
            ("ubPc", 12714, 7136),
            ("ZBac", 14913, 8581),
            ("eGlu", 15567, 8328),
            ("kOtt", 519, 419),
            ("dSvy", 11078, 6389),
            ("xmja", 11078, 6389),
            ("mHuy", 1636, 747),
            ("fMVl", 1636, 747),
        ],
        birth_offsets: (1421, 1458, 808),
        year_range: (2, 103),
        partition: (6389, 747),
        dxsb: (1042, 10),
        main_team: MainTeamCounts {
            rostered: 6898,
            plus_one: 6898,
            same: 17,
            unrostered_zero: 194,
            unrostered_other: 44,
            all_ones: 0,
            team_keys: 419,
        },
        unrostered_patterns: &[(501, 22), (502, 22)],
        largest_team_key: 679,
        largest_pattern: 680,
        handedness: (1462, &[(0, 4772), (1, 2364)]),
        team_name: TeamName::Mapped(
            "AUsv",
            2768,
            256,
            "Anaheim Ducks®",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim®",
        ),
        ratings: ((36, 99), (52, 99)),
        population: EIGHT_MAPPINGS_UNATTRIBUTED,
        candidates: CANDIDATES_UNATTRIBUTED,
        mapped_fraction: (10, 3156),
    };

    cases!(E:
        capacities_are_the_directory_of_the_committed_main_anchor => run::capacities,
        a_capacity_that_is_not_read_from_this_anchor_is_reported => run::capacity_misattribution_is_reported,
        the_roster_fixture_decodes_through_the_committed_ps4_codec_and_its_directory_is_compared_with_the_main_anchor => run::roster_save,
        the_committed_nhl19_and_nhl20_anchors_are_byte_identical_so_neither_attribution_is_established => run::attribution,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_is_stored_only_on_unrostered_players => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        seven_bit_ratings_store_the_human_value_directly_on_this_anchor => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 20, PS4

mod nhl20_ps4 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl20-ps4.toml",
            target: "nhl20-ps4",
            cases_file: CASES_FILE,
            platform: "ps4",
            game: "nhl20",
            main_anchor: "anchors/ps4/nhl20/databases/nhl_game.db",
            main_anchor_sha256: "265e4a68d23ce44cef3240017beda0583c2cb6d246ea6672ca5612472429f408",
            // No ROSTER save is committed for this title.
            roster: None,
        },
        module: "nhl20_ps4",
        main_meta: "anchors/ps4/nhl20/databases/nhl_game-meta.xml",
        main_meta_text_sha256: "969a0f20435331f76c9841a2d259b4c8eb15b825e1b648fa0bfc6635d9870893",
        // C0-REG4-R1: the bytes committed under nhl19 and nhl20 are the same; no committed
        // provenance says which title they are, so nothing here is operative for this target.
        attribution: Attribution::Unresolved(Twin {
            main: "anchors/ps4/nhl19/databases/nhl_game.db",
            meta: "anchors/ps4/nhl19/databases/nhl_game-meta.xml",
            shared_aux: NHL19_NHL20_SHARED_AUX,
            anchor_prefix: "anchors/ps4/nhl19/",
            snapshot_name: "ps4-nhl19.json",
        }),
        // The Threes save disagrees with this anchor on every roster-core directory figure.
        save: RosterSave::Missing {
            threes: "anchors/ps4/nhl20/savefiles/ThreeOnThree0/DATA",
            threes_sha256: "615a107d2f2d0439c08a196feebb745593b499982c0c7400c374e3e505f84fd8",
            threes_max_equal_main: false,
        },
        capacities: &[
            ("ubPc", 12714, 7136),
            ("ZBac", 14913, 8581),
            ("eGlu", 15567, 8328),
            ("kOtt", 519, 419),
            ("dSvy", 11078, 6389),
            ("xmja", 11078, 6389),
            ("mHuy", 1636, 747),
            ("fMVl", 1636, 747),
        ],
        birth_offsets: (1421, 1458, 808),
        year_range: (2, 103),
        partition: (6389, 747),
        dxsb: (1042, 10),
        main_team: MainTeamCounts {
            rostered: 6898,
            plus_one: 6898,
            same: 17,
            unrostered_zero: 194,
            unrostered_other: 44,
            all_ones: 0,
            team_keys: 419,
        },
        unrostered_patterns: &[(501, 22), (502, 22)],
        largest_team_key: 679,
        largest_pattern: 680,
        handedness: (1462, &[(0, 4772), (1, 2364)]),
        team_name: TeamName::Mapped(
            "AUsv",
            2768,
            256,
            "Anaheim Ducks®",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim®",
        ),
        ratings: ((36, 99), (52, 99)),
        population: EIGHT_MAPPINGS_UNATTRIBUTED,
        candidates: CANDIDATES_UNATTRIBUTED,
        mapped_fraction: (10, 3156),
    };

    cases!(E:
        capacities_are_the_directory_of_the_committed_main_anchor => run::capacities,
        a_capacity_that_is_not_read_from_this_anchor_is_reported => run::capacity_misattribution_is_reported,
        no_roster_fixture_is_committed_so_the_save_capacity_is_missing_fixture => run::roster_save,
        the_committed_nhl20_and_nhl19_anchors_are_byte_identical_so_neither_attribution_is_established => run::attribution,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_is_stored_only_on_unrostered_players => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        seven_bit_ratings_store_the_human_value_directly_on_this_anchor => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}

// ============================================================== NHL 21, PS4

mod nhl21_ps4 {
    use super::*;

    const E: Expect = Expect {
        target: Target {
            adapter_file: "nhl21-ps4.toml",
            target: "nhl21-ps4",
            cases_file: CASES_FILE,
            platform: "ps4",
            game: "nhl21",
            main_anchor: "anchors/ps4/nhl21/databases/nhl_game.db",
            main_anchor_sha256: "90ee1ec49d36dbb0f6d9cefe75e259d9661e3aa05338d03347b4842e33c8e9f9",
            // No ROSTER save is committed for this title.
            roster: None,
        },
        module: "nhl21_ps4",
        main_meta: "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
        main_meta_text_sha256: "c8b85dfcd8c572d19c87d347e80b457b8beee7e228117eca09e58615e7c7384b",
        attribution: Attribution::Established,
        // The Threes save carries this anchor's `max_records` on the roster core, and other
        // populations.
        save: RosterSave::Missing {
            threes: "anchors/ps4/nhl21/savefiles/ThreeOnThree0/DATA",
            threes_sha256: "70d379f015ba1ff7391575cbf513fc4511fcd153a70478a562b5a031c7a40c8c",
            threes_max_equal_main: true,
        },
        capacities: &[
            ("ubPc", 12439, 7247),
            ("ZBac", 15615, 9281),
            ("eGlu", 16576, 9090),
            ("kOtt", 533, 433),
            ("dSvy", 10841, 6498),
            ("xmja", 10841, 6498),
            ("mHuy", 1598, 749),
            ("fMVl", 1598, 749),
        ],
        birth_offsets: (1348, 1384, 848),
        year_range: (0, 104),
        partition: (6498, 749),
        dxsb: (1080, 10),
        main_team: MainTeamCounts {
            rostered: 7055,
            plus_one: 7055,
            same: 45,
            unrostered_zero: 148,
            unrostered_other: 44,
            all_ones: 0,
            team_keys: 433,
        },
        unrostered_patterns: &[(501, 22), (502, 22)],
        largest_team_key: 696,
        largest_pattern: 697,
        handedness: (1388, &[(0, 4812), (1, 2435)]),
        team_name: TeamName::Mapped(
            "AUsv",
            2672,
            256,
            "Anaheim Ducks®",
            "Winnipeg Jets™",
            "Mighty Ducks of Anaheim®",
        ),
        ratings: ((36, 99), (52, 98)),
        population: EIGHT_MAPPINGS,
        candidates: CANDIDATES,
        mapped_fraction: (10, 3265),
    };

    cases!(E:
        capacities_are_the_directory_of_the_committed_main_anchor => run::capacities,
        a_capacity_that_is_not_read_from_this_anchor_is_reported => run::capacity_misattribution_is_reported,
        no_roster_fixture_is_committed_so_the_save_capacity_is_missing_fixture => run::roster_save,
        birth_date_month_and_day_are_stored_zero_origin_on_this_anchor => run::birth_date,
        position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do => run::position,
        main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_is_stored_only_on_unrostered_players => run::main_team,
        stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes => run::codec_order,
        handedness_histogram_is_recorded_but_no_label_is_established => run::handedness,
        seven_bit_ratings_store_the_human_value_directly_on_this_anchor => run::ratings,
        team_name_edit_round_trips_on_a_disposable_copy => run::team_name,
        the_adapters_population_reflects_exactly_these_measurements => run::ledger,
    );
}
