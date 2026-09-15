//! Shared **test mechanics** for target-specific evidence cases (package C0-REG1b, factored out of
//! the Legacy module of C0-REG1a-R1/R2 so a second target does not copy a thousand lines).
//!
//! Everything here is parameterised by a [`Target`]: which adapter, which main anchor (by path and
//! SHA-256), which ROSTER fixture (by path, SHA-256 and decoded-payload SHA-256). Every expectation
//! — record counts, code sets, vanilla strings, framing — is supplied by the per-target module that
//! calls in, never by this file, so nothing here can carry a number from one target to another.
//!
//! The helpers read committed files, parse them with the production `tdb_core` / `tdb_savedata`
//! seams and compare bytes with bytes. The codec-order helpers (`decode_stored` / `encode_stored`)
//! demonstrate the `SENTINEL_ENCODING` processing order over an adapter's declared blocks; they are
//! **test helpers**, and this crate ships no production adapter executor.

use crate::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tdb_core::{FieldDescriptor, Table, TdbFile};
use tdb_qa::schema::{snapshot, FieldSnapshot, SchemaSnapshot};

/// One target's committed inputs, by identity.
pub(crate) struct Target {
    pub adapter_file: &'static str,
    pub target: &'static str,
    /// The per-target case module, as cited in adapter provenance (`<file>#<case>`).
    pub cases_file: &'static str,
    pub platform: &'static str,
    pub game: &'static str,
    pub main_anchor: &'static str,
    pub main_anchor_sha256: &'static str,
    pub roster_fixture: &'static str,
    pub roster_fixture_sha256: &'static str,
    /// The inner TDB `extract_db` yields from the fixture.
    pub roster_payload_sha256: &'static str,
}

/// The save container framing a fixture is expected to show, as decoded by `tdb_savedata`. The
/// variant is the one the discriminator resolves (by body marker, not by the version word alone);
/// the size words exist only on the variants whose header carries them (`RawV2` has the
/// uncompressed size and no compressed size; `RawV1` has neither).
pub(crate) struct RosterFraming {
    pub outer_len: usize,
    pub magic: &'static str,
    pub version: tdb_savedata::ContainerVersion,
    pub version_word: u32,
    pub uncompressed_size: Option<u32>,
    pub compressed_size: Option<u32>,
    pub inner_len: usize,
    pub table_count: usize,
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Read a committed file and pin its identity: a case that measured a different file would say so.
pub(crate) fn read_pinned(rel: &str, expected_sha256: &str) -> Vec<u8> {
    let bytes = std::fs::read(repo_root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
    assert_eq!(
        sha256_hex(&bytes),
        expected_sha256,
        "{rel}: not the committed bytes"
    );
    bytes
}

pub(crate) fn table<'a>(db: &'a TdbFile, id: &str) -> &'a Table {
    db.table_canonical(id)
        .unwrap_or_else(|| panic!("table {id} is in the database"))
}

pub(crate) fn field(db: &TdbFile, t: &Table, id: &str) -> FieldDescriptor {
    t.fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == id)
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "field {}.{id} is in the database",
                t.id.canonical(db.endian)
            )
        })
}

pub(crate) fn read(db: &TdbFile, t: &Table, record: u32, f: &FieldDescriptor) -> u64 {
    db.read_u64(t, record, f).unwrap_or_else(|| {
        panic!(
            "record {record} of {} is readable",
            t.id.canonical(db.endian)
        )
    })
}

pub(crate) fn schema_for_validation() -> ProjectSchema {
    let text = std::fs::read_to_string(repo_root().join("defs/canonical/project-schema.toml"))
        .expect("the canonical schema is committed");
    parse_project_schema(&text).expect("it parses")
}

pub(crate) fn meta_table_name(snap: &SchemaSnapshot, id: &str) -> String {
    snap.tables
        .iter()
        .find(|t| t.canonical_id == id)
        .and_then(|t| t.name.clone())
        .unwrap_or_else(|| panic!("the metadata names table {id}"))
}

pub(crate) fn meta_field<'a>(snap: &'a SchemaSnapshot, table: &str, id: &str) -> &'a FieldSnapshot {
    snap.tables
        .iter()
        .find(|t| t.canonical_id == table)
        .and_then(|t| t.fields.iter().find(|f| f.canonical_id == id))
        .unwrap_or_else(|| panic!("the metadata names field {table}.{id}"))
}

pub(crate) fn mapping<'a>(a: &'a Adapter, canonical: &str) -> &'a Mapping {
    a.mappings
        .iter()
        .find(|m| m.canonical == canonical)
        .unwrap_or_else(|| panic!("{} maps {canonical}", a.target))
}

impl Target {
    pub(crate) fn adapter(&self) -> Adapter {
        let text =
            std::fs::read_to_string(repo_root().join("defs/adapters").join(self.adapter_file))
                .expect("adapter is in the tree");
        let a =
            parse_adapter(&text).unwrap_or_else(|e| panic!("{} parses: {e}", self.adapter_file));
        assert_eq!(a.target, self.target);
        a
    }

    pub(crate) fn main_anchor_bytes(&self) -> Vec<u8> {
        read_pinned(self.main_anchor, self.main_anchor_sha256)
    }

    /// The committed snapshot of the main anchor, which carries EA's metadata names — the
    /// independent label source for the semantic cases.
    pub(crate) fn main_snapshot(&self) -> SchemaSnapshot {
        let anchors = tdb_qa::discover_main_anchors(&repo_root()).expect("anchors/ is readable");
        let anchor = anchors
            .iter()
            .find(|a| a.platform == self.platform && a.game == self.game)
            .unwrap_or_else(|| panic!("{}: main anchor is committed", self.target));
        assert_eq!(anchor.db_rel, self.main_anchor);
        snapshot(&repo_root(), anchor).expect("the anchor parses with its metadata")
    }

    pub(crate) fn cites(&self, provenance: &[String], case: &str) -> bool {
        provenance
            .iter()
            .any(|p| p == &format!("{}#{case}", self.cases_file))
    }

    /// The decoded ROSTER fixture: identity and framing asserted on the way in, through the
    /// production save codec.
    pub(crate) fn decoded_roster_fixture(&self, framing: &RosterFraming) -> Vec<u8> {
        let outer = read_pinned(self.roster_fixture, self.roster_fixture_sha256);
        assert_eq!(outer.len(), framing.outer_len);
        let container =
            tdb_savedata::SaveContainer::parse(&outer).expect("the container header parses");
        let h = &container.header;
        assert_eq!(h.magic, framing.magic);
        assert_eq!(h.save_type, tdb_savedata::SaveType::Roster);
        assert_eq!(h.version, framing.version);
        assert_eq!(h.version_word, framing.version_word);
        assert_eq!(h.uncompressed_size, framing.uncompressed_size);
        assert_eq!(h.compressed_size, framing.compressed_size);
        assert!(
            container.checksums_valid(),
            "both container checksums recompute"
        );
        let inner = container
            .extract_db()
            .expect("the body yields a TDB (inflated for ZlibV4, sliced for the raw variants)");
        assert_eq!(inner.len(), framing.inner_len);
        assert_eq!(sha256_hex(&inner), self.roster_payload_sha256);
        let save = TdbFile::parse(&inner).expect("the inner TDB parses");
        assert!(save.header_crc_ok() && save.size_ok());
        assert_eq!(save.endian, tdb_core::Endian::Big);
        assert_eq!(save.tables.len(), framing.table_count);
        inner
    }

    /// Every disagreement between a `roster-save-max-records` capacity and the decoded save it must
    /// have been read from. A capacity attributed to any other file is reported as unverifiable
    /// here. Ceiling >= population is checked within THIS source — the save's own directory — and
    /// nowhere against a different file's population.
    pub(crate) fn roster_capacity_discrepancies(
        &self,
        adapter: &Adapter,
        save: &TdbFile,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for cap in &adapter.capacities {
            let Some(v) = cap.declared.established() else {
                out.push(format!("capacity {}: not established", cap.table));
                continue;
            };
            if v.source != "roster-save-max-records" {
                out.push(format!(
                    "capacity {}: source {} is not the ROSTER fixture this case decodes",
                    cap.table, v.source
                ));
                continue;
            }
            if !cap
                .declared
                .provenance
                .iter()
                .any(|p| p == self.roster_fixture)
            {
                out.push(format!(
                    "capacity {}: provenance does not name the fixture it was read from",
                    cap.table
                ));
            }
            let Some(t) = save.table_canonical(&cap.table) else {
                out.push(format!(
                    "capacity {}: table not in the ROSTER save",
                    cap.table
                ));
                continue;
            };
            if v.ceiling_records != u32::from(t.max_records) {
                out.push(format!(
                    "capacity {}: ceiling_records {} vs fixture max_records {}",
                    cap.table, v.ceiling_records, t.max_records
                ));
            }
            if v.population_records != u32::from(t.current_records) {
                out.push(format!(
                    "capacity {}: population_records {} vs fixture current_records {}",
                    cap.table, v.population_records, t.current_records
                ));
            }
            if t.max_records < t.current_records {
                out.push(format!(
                    "capacity {}: fixture max_records {} below its own current_records {}",
                    cap.table, t.max_records, t.current_records
                ));
            }
        }
        out
    }

    /// The whole capacity case for a target: the save's directory as read, the fixture observation
    /// that its populations equal the main anchor's for every table both carry (verified, never
    /// presumed), and the adapter declaring exactly that, attributed to this fixture, measured, with
    /// the note saying what a directory capacity is not.
    pub(crate) fn assert_capacities_are_the_fixtures(
        &self,
        save: &TdbFile,
        expected: &[(&str, u16, u16)],
        capacity_case: &str,
    ) {
        let observed: BTreeMap<String, (u16, u16)> = save
            .tables
            .iter()
            .map(|t| {
                (
                    t.id.canonical(save.endian),
                    (t.max_records, t.current_records),
                )
            })
            .collect();
        for (id, max, cur) in expected {
            assert_eq!(
                observed[*id],
                (*max, *cur),
                "{id}: (max_records, current_records)"
            );
        }

        let main = self.main_anchor_bytes();
        let main = TdbFile::parse(&main).unwrap();
        let mut compared = 0;
        for t in &main.tables {
            if let Some(s) = save.table_canonical(&t.id.canonical(main.endian)) {
                assert_eq!(
                    s.current_records,
                    t.current_records,
                    "{}: save vs main population (fixture observation)",
                    t.id.canonical(main.endian)
                );
                compared += 1;
            }
        }
        assert_eq!(
            compared,
            save.tables.len(),
            "every save table exists in the main DB"
        );

        let a = self.adapter();
        assert_eq!(
            self.roster_capacity_discrepancies(&a, save),
            Vec::<String>::new()
        );
        assert_eq!(a.capacities.len(), expected.len());
        for cap in &a.capacities {
            assert_eq!(
                cap.declared.state,
                ConstraintState::Established,
                "{}",
                cap.table
            );
            assert_eq!(
                cap.declared.confidence.as_deref(),
                Some("measured"),
                "{}",
                cap.table
            );
            assert!(
                self.cites(&cap.declared.provenance, capacity_case),
                "{}: cites this case",
                cap.table
            );
            assert!(
                cap.declared
                    .note
                    .contains("not a demonstrated runtime limit"),
                "{}: the note says what a directory capacity is not",
                cap.table
            );
        }
    }
}

// ============================================================== position / companions

/// Bio rows partitioned by which companion tables hold their key, with the skater/goalie label of
/// each companion read from EA's metadata name for that table.
pub(crate) struct CompanionPartition {
    pub skaters: BTreeSet<u32>,
    pub goalies: BTreeSet<u32>,
}

pub(crate) fn companion_partition(db: &TdbFile, snap: &SchemaSnapshot) -> CompanionPartition {
    // Independent labels: the metadata names say which companion tables are for skaters and which
    // for goalies. This is the only place a label enters, and it is EA's, not an enum file's.
    let role = |id: &str| -> &'static str {
        let name = meta_table_name(snap, id);
        match (name.contains("skater"), name.contains("goalie")) {
            (true, false) => "skater",
            (false, true) => "goalie",
            _ => panic!("{id}: metadata name {name:?} does not label the table"),
        }
    };
    let companions = [
        ("dSvy", role("dSvy")),
        ("xmja", role("xmja")),
        ("mHuy", role("mHuy")),
        ("fMVl", role("fMVl")),
    ];
    assert_eq!(
        companions.iter().map(|c| c.1).collect::<Vec<_>>(),
        ["skater", "skater", "goalie", "goalie"]
    );

    let bio = table(db, "ubPc");
    let bio_key = field(db, bio, "wBIz");
    let bio_by_key: BTreeMap<u64, u32> = (0..u32::from(bio.current_records))
        .map(|i| (read(db, bio, i, &bio_key), i))
        .collect();
    assert_eq!(
        bio_by_key.len() as u16,
        bio.current_records,
        "bio keys are distinct"
    );

    let mut by_role: BTreeMap<&str, Vec<BTreeSet<u32>>> = BTreeMap::new();
    for (id, label) in companions {
        let t = table(db, id);
        let k = field(db, t, "wBIz");
        let keys: Vec<u64> = (0..u32::from(t.current_records))
            .map(|i| read(db, t, i, &k))
            .collect();
        let distinct: BTreeSet<u64> = keys.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            keys.len(),
            "{id}: one companion row per key (one-to-one)"
        );
        let rows: BTreeSet<u32> = distinct
            .iter()
            .map(|k| {
                *bio_by_key
                    .get(k)
                    .unwrap_or_else(|| panic!("{id}: key {k} has a bio row"))
            })
            .collect();
        by_role.entry(label).or_default().push(rows);
    }
    // Both skater companions hold exactly the same players; so do both goalie companions.
    let same = |sets: &[BTreeSet<u32>]| {
        assert!(
            sets.windows(2).all(|w| w[0] == w[1]),
            "companions of one role cover one set"
        );
        sets[0].clone()
    };
    let skaters = same(&by_role["skater"]);
    let goalies = same(&by_role["goalie"]);
    assert!(skaters.is_disjoint(&goalies));
    assert_eq!(
        skaters.len() + goalies.len(),
        bio_by_key.len(),
        "every bio row has companions"
    );
    CompanionPartition { skaters, goalies }
}

/// The position codes stored on each half of the partition, and the per-row check that the code
/// alone decides the half with no exception.
pub(crate) fn position_codes_by_partition(
    db: &TdbFile,
    part: &CompanionPartition,
    goalie_code: u64,
) -> (BTreeSet<u64>, BTreeSet<u64>) {
    let bio = table(db, "ubPc");
    let position = field(db, bio, "vjla");
    let codes = |rows: &BTreeSet<u32>| -> BTreeSet<u64> {
        rows.iter().map(|&i| read(db, bio, i, &position)).collect()
    };
    for i in 0..u32::from(bio.current_records) {
        let is_goalie_code = read(db, bio, i, &position) == goalie_code;
        assert_eq!(is_goalie_code, part.goalies.contains(&i), "bio record {i}");
    }
    (codes(&part.skaters), codes(&part.goalies))
}

/// The adapter's discriminator, companion rule and the applicability of every rating in `rated`
/// (the skater-companion properties this adapter maps — a title without a stored overall maps one)
/// declare exactly the measured partition and cite the named case.
pub(crate) fn assert_discriminator_declared(
    t: &Target,
    a: &Adapter,
    skater_codes: &[i64],
    goalie_codes: &[i64],
    rated: &[&str],
    case: &str,
) {
    let disc = a.position_discriminator.as_ref().expect("declared");
    assert_eq!(disc.state, ConstraintState::Established);
    assert_eq!(disc.confidence.as_deref(), Some("measured"));
    let v = disc.established().unwrap();
    assert_eq!((v.table.as_str(), v.field.as_str()), ("ubPc", "vjla"));
    assert_eq!(v.skater_codes, skater_codes);
    assert_eq!(v.goalie_codes, goalie_codes);
    assert!(t.cites(&disc.provenance, case));

    let rule = &a
        .companion_rules
        .iter()
        .find(|r| r.primary_table == "ubPc")
        .expect("declared")
        .declared;
    assert_eq!(rule.state, ConstraintState::Established);
    assert_eq!(rule.confidence.as_deref(), Some("measured"));
    assert!(t.cites(&rule.provenance, case));
    let rv = rule.established().unwrap();
    assert_eq!(rv.shared_key_field, "wBIz");
    let declared: Vec<(&str, &str, &str)> = rv
        .companions
        .iter()
        .map(|c| {
            (
                c.table.as_str(),
                c.cardinality.as_str(),
                c.applies_when.as_str(),
            )
        })
        .collect();
    assert_eq!(
        declared,
        [
            ("dSvy", "one-to-one", "position-skater"),
            ("xmja", "one-to-one", "position-skater"),
            ("mHuy", "one-to-one", "position-goalie"),
            ("fMVl", "one-to-one", "position-goalie"),
        ]
    );

    assert!(!rated.is_empty());
    for &canonical in rated {
        let m = mapping(a, canonical);
        let ap = m
            .applicability
            .established()
            .unwrap_or_else(|| panic!("{canonical}: established"));
        assert_eq!(ap.kind, "target-predicate", "{canonical}");
        assert_eq!(
            (ap.table.as_deref(), ap.field.as_deref()),
            (Some("ubPc"), Some("vjla"))
        );
        assert_eq!(ap.codes, skater_codes, "{canonical}");
        assert_eq!(m.applicability.confidence.as_deref(), Some("measured"));
        assert!(t.cites(&m.applicability.provenance, case), "{canonical}");
    }
}

// ============================================================== main team

/// The roster graph, walked from the metadata-declared foreign key: `eGlu.XSWT -> ZBac` is
/// `fkparenttable = exhibitionplayers` in EA's metadata; `ZBac.ykFq -> ubPc.wBIz` is a join on the
/// field EA names `playerid` (not declared as a key; every value resolves). Returns, per bio record,
/// the team codes of the roster slots that resolve to it.
pub(crate) fn roster_slots_by_bio_record(
    db: &TdbFile,
    snap: &SchemaSnapshot,
) -> BTreeMap<u32, Vec<u64>> {
    assert_eq!(
        meta_field(snap, "eGlu", "XSWT").fk_parent.as_deref(),
        Some(meta_table_name(snap, "ZBac").as_str()),
        "the roster -> player hop is declared in the metadata"
    );
    assert_eq!(
        meta_field(snap, "ZBac", "ykFq").name.as_deref(),
        Some("playerid")
    );
    assert_eq!(
        meta_field(snap, "eGlu", "dXSB").name.as_deref(),
        Some("team")
    );

    let eglu = table(db, "eGlu");
    let (slot_player, slot_team) = (field(db, eglu, "XSWT"), field(db, eglu, "dXSB"));
    let zbac = table(db, "ZBac");
    let (z_index, z_player) = (field(db, zbac, "vfEq"), field(db, zbac, "ykFq"));
    let bio = table(db, "ubPc");
    let bio_key = field(db, bio, "wBIz");

    let player_index: BTreeMap<u64, u64> = (0..u32::from(zbac.current_records))
        .map(|i| (read(db, zbac, i, &z_index), read(db, zbac, i, &z_player)))
        .collect();
    let bio_by_key: BTreeMap<u64, u32> = (0..u32::from(bio.current_records))
        .map(|i| (read(db, bio, i, &bio_key), i))
        .collect();
    let mut slots: BTreeMap<u32, Vec<u64>> = BTreeMap::new();
    for i in 0..u32::from(eglu.current_records) {
        let x = read(db, eglu, i, &slot_player);
        let pid = *player_index
            .get(&x)
            .unwrap_or_else(|| panic!("slot {i}: playerindex {x} resolves"));
        let rec = *bio_by_key
            .get(&pid)
            .unwrap_or_else(|| panic!("slot {i}: playerid {pid} resolves"));
        slots
            .entry(rec)
            .or_default()
            .push(read(db, eglu, i, &slot_team));
    }
    slots
}

/// What the main-team pointer stores, against the roster.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MainTeamCounts {
    pub rostered: usize,
    /// Slots whose team code + 1 equals the bio row's stored pattern.
    pub plus_one: usize,
    /// Slots whose team code equals the stored pattern as-is.
    pub same: usize,
    pub unrostered_zero: usize,
    pub unrostered_other: usize,
    /// Rows storing the all-ones pattern.
    pub all_ones: usize,
    /// Distinct `kOtt.vfEq` keys (observation only: containment is not a foreign key).
    pub team_keys: usize,
}

/// Per rostered player exactly one slot must carry `team == pattern - 1`, none `team == pattern`;
/// each un-rostered player is counted by whether it stores 0. Containment of every slot code in the
/// team table's key set is asserted as an observation.
pub(crate) fn main_team_measurement(
    db: &TdbFile,
    snap: &SchemaSnapshot,
    slots: &BTreeMap<u32, Vec<u64>>,
) -> MainTeamCounts {
    let bio = table(db, "ubPc");
    let dxsb = field(db, bio, "dXSB");
    assert_eq!(
        meta_field(snap, "ubPc", "dXSB").name.as_deref(),
        Some("team")
    );
    assert_eq!(dxsb.bit_width, 8);
    let mut c = MainTeamCounts {
        rostered: 0,
        plus_one: 0,
        same: 0,
        unrostered_zero: 0,
        unrostered_other: 0,
        all_ones: 0,
        team_keys: 0,
    };
    for i in 0..u32::from(bio.current_records) {
        let raw = read(db, bio, i, &dxsb);
        if raw == 0xFF {
            c.all_ones += 1;
        }
        match slots.get(&i) {
            Some(teams) => {
                c.rostered += 1;
                assert_ne!(raw, 0, "bio record {i} is rostered but stores 0");
                let hits = teams.iter().filter(|&&t| t + 1 == raw).count();
                c.plus_one += hits;
                c.same += teams.iter().filter(|&&t| t == raw).count();
                assert_eq!(
                    hits, 1,
                    "bio record {i}: exactly one of its slots {teams:?} is its main team (raw {raw})"
                );
            }
            None => {
                if raw == 0 {
                    c.unrostered_zero += 1
                } else {
                    c.unrostered_other += 1
                }
            }
        }
    }
    let kott = table(db, "kOtt");
    let team_key = field(db, kott, "vfEq");
    let team_ids: BTreeSet<u64> = (0..u32::from(kott.current_records))
        .map(|i| read(db, kott, i, &team_key))
        .collect();
    for teams in slots.values() {
        assert!(teams.iter().all(|t| team_ids.contains(t)));
    }
    c.team_keys = team_ids.len();
    c
}

/// The adapter declares the main-team encoding and absence code as measured, keeps the parent a
/// candidate, keeps the mapping blocked on that, and has no human range (it is a reference).
pub(crate) fn assert_main_team_declared(t: &Target, a: &Adapter, case: &str) {
    let m = mapping(a, "player.team");
    let st = m.storage.established().expect("storage established");
    assert_eq!(
        (st.signedness.as_str(), st.range_low, st.range_high),
        ("offset-binary", -1, 254)
    );
    assert!(t.cites(&m.storage.provenance, case));
    assert_eq!(
        (m.read_transform.as_str(), m.write_transform.as_str()),
        ("index-origin-0", "index-origin-0")
    );
    assert_eq!(m.qualification, "blocked");
    assert!(
        !m.inverse_verified,
        "no edit through this field is exercised"
    );
    let tr = m.transform.established().expect("transform established");
    assert_eq!(
        (tr.kind.as_str(), tr.index_origin),
        ("index-origin", Some(0))
    );
    assert_eq!(m.transform.confidence.as_deref(), Some("measured"));
    assert!(t.cites(&m.transform.provenance, case));
    assert_eq!(
        human_range(m),
        None,
        "a reference has no human range: the parent key set, not a numeric interval, is its domain"
    );
    let s = m.null_sentinel.established().expect("sentinel established");
    assert_eq!(
        (s.encoding.as_str(), s.raw, s.canonical_meaning.as_deref()),
        ("literal-code", Some(0), Some("absent")),
        "raw is the unsigned stored bit pattern"
    );
    assert_eq!(m.null_sentinel.confidence.as_deref(), Some("measured"));
    assert!(t.cites(&m.null_sentinel.provenance, case));
    assert_eq!(m.foreign_key.state, ConstraintState::Unknown);
    let fk = m
        .foreign_key
        .candidate_value()
        .expect("the parent proposal is kept");
    assert_eq!(
        (
            fk.parent_table.as_str(),
            fk.parent_key_field.as_str(),
            fk.index_origin
        ),
        ("kOtt", "vfEq", Some(0))
    );
    assert!(m.note.contains("parent"), "the blocker names what is left");
}

// ============================================================== codec order

/// What the sentinel → decode → index pipeline yields for one stored pattern. A **test helper**:
/// it exists to demonstrate the order `SENTINEL_ENCODING` states, over an adapter's `player.team`
/// declaration, and it is not a production executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decoded {
    /// The stored bits matched the sentinel; nothing further was decoded.
    Absent,
    /// The decoded, index-mapped value: a 0-origin **team code** in the roster table's code space.
    /// Which parent table those codes index is exactly what stays unresolved for this mapping.
    TeamCode(u64),
}

/// Steps (1)–(4) of `SENTINEL_ENCODING`'s processing order over the declared blocks: match the
/// stored bits against the sentinel; on a match return absence; otherwise decode by the storage
/// regime; then apply the index-origin transform. No parent-key mapping is performed, because no
/// `[mapping.foreign_key]` is established.
pub(crate) fn decode_stored(m: &Mapping, pattern: u64) -> Decoded {
    let storage = m.storage.established().expect("storage established");
    let sentinel = m.null_sentinel.established().expect("sentinel established");
    let transform = m.transform.established().expect("transform established");
    assert!(pattern < (1u64 << storage.bit_width));
    // (1)/(2): stored bits first.
    let sentinel_pattern = match sentinel.encoding.as_str() {
        "literal-code" => u64::try_from(sentinel.raw.unwrap()).expect("bit-pattern units"),
        "raw-all-ones" => (1u64 << storage.bit_width) - 1,
        other => panic!("no sentinel pattern for {other}"),
    };
    if pattern == sentinel_pattern {
        assert_eq!(sentinel.canonical_meaning.as_deref(), Some("absent"));
        return Decoded::Absent;
    }
    // (3): storage decode.
    let decoded: i64 = match storage.signedness.as_str() {
        "offset-binary" => pattern as i64 + storage.range_low,
        "unsigned" => pattern as i64,
        other => panic!("not exercised here: {other}"),
    };
    assert!((storage.range_low..=storage.range_high).contains(&decoded));
    // (4): index mapping — the decoded value's first index is `index_origin`.
    assert_eq!(transform.kind, "index-origin");
    let origin = transform.index_origin.unwrap();
    assert!(
        decoded >= origin,
        "a non-absent decoded value is a team code"
    );
    Decoded::TeamCode((decoded - origin) as u64)
}

/// The inverse, run backwards: absence writes the sentinel pattern; a team code is index-mapped,
/// storage-encoded, and must fit the field.
pub(crate) fn encode_stored(m: &Mapping, value: Decoded) -> u64 {
    let storage = m.storage.established().unwrap();
    let sentinel = m.null_sentinel.established().unwrap();
    let transform = m.transform.established().unwrap();
    match value {
        Decoded::Absent => match sentinel.encoding.as_str() {
            "literal-code" => u64::try_from(sentinel.raw.unwrap()).unwrap(),
            "raw-all-ones" => (1u64 << storage.bit_width) - 1,
            other => panic!("{other}"),
        },
        Decoded::TeamCode(code) => {
            let decoded = code as i64 + transform.index_origin.unwrap();
            let pattern = match storage.signedness.as_str() {
                "offset-binary" => decoded - storage.range_low,
                "unsigned" => decoded,
                other => panic!("{other}"),
            };
            let pattern = u64::try_from(pattern).expect("non-negative pattern");
            assert!(pattern < (1u64 << storage.bit_width), "fits the field");
            pattern
        }
    }
}

/// Every `ubPc.dXSB` pattern on the anchor through the pipeline and back: absence ⇔ no roster slot,
/// a code ⇔ one of that player's slot team codes, and the inverse restores the pattern. Returns
/// (absent, coded).
pub(crate) fn codec_round_trip_over_anchor(
    m: &Mapping,
    db: &TdbFile,
    slots: &BTreeMap<u32, Vec<u64>>,
) -> (usize, usize) {
    for pattern in 0..=255u64 {
        assert_eq!(encode_stored(m, decode_stored(m, pattern)), pattern);
    }
    let bio = table(db, "ubPc");
    let dxsb = field(db, bio, "dXSB");
    let (mut absent, mut coded) = (0usize, 0usize);
    for i in 0..u32::from(bio.current_records) {
        let pattern = read(db, bio, i, &dxsb);
        let value = decode_stored(m, pattern);
        match (value, slots.get(&i)) {
            (Decoded::Absent, None) => absent += 1,
            (Decoded::TeamCode(code), Some(teams)) => {
                assert!(
                    teams.contains(&code),
                    "bio record {i}: code {code} in {teams:?}"
                );
                coded += 1;
            }
            (v, s) => panic!("bio record {i}: {v:?} vs roster slots {s:?}"),
        }
        assert_eq!(encode_stored(m, value), pattern, "bio record {i}");
    }
    (absent, coded)
}

// ============================================================== team.name

/// `kOtt.YmkJ` (at `field_bit_offset`, the target's own) read / edit / serialize / reopen on an
/// in-memory copy of `original`: the unedited copy re-serializes byte-identically; record 0 reads
/// `vanilla`; after `set_inline_str` +
/// `recompute_crcs` the reopened copy reads `replacement` on record 0 and `vanilla_record_1` on
/// record 1; the only differing bytes are the field span of record 0 and the one CRC word that
/// hashes this table's body; every other kOtt record's every field and every other table's record
/// block compare equal; a second no-op serialization is stable; a 51-byte value is refused with
/// nothing written and a 50-byte value is accepted.
pub(crate) fn team_name_round_trip(
    original: &[u8],
    field_bit_offset: u32,
    vanilla: &str,
    vanilla_record_1: &str,
    replacement: &str,
) {
    let db = TdbFile::parse(original).unwrap();
    assert_eq!(
        db.serialize(),
        original,
        "an unedited copy re-serializes byte-identically"
    );
    let kott = table(&db, "kOtt");
    let name = field(&db, kott, "YmkJ");
    assert_eq!(
        (name.kind, name.bit_offset, name.bit_width),
        (0, field_bit_offset, 400),
        "the field sits where this target's snapshot says (the offset differs across titles)"
    );
    let record = 0;
    assert_eq!(
        db.read_inline_str(kott, record, &name).unwrap(),
        vanilla.as_bytes(),
        "byte-transparent"
    );

    assert!(replacement.len() < 50);
    let mut edited = original.to_vec();
    assert!(db.set_inline_str(&mut edited, kott, record, &name, replacement));
    db.recompute_crcs(&mut edited);

    let reopened = TdbFile::parse(&edited).unwrap();
    assert!(reopened.header_crc_ok() && reopened.size_ok());
    let kott2 = table(&reopened, "kOtt");
    assert_eq!(
        reopened.read_inline_str(kott2, record, &name).unwrap(),
        replacement.as_bytes()
    );
    assert_eq!(
        reopened.read_inline_str(kott2, 1, &name).unwrap(),
        vanilla_record_1.as_bytes()
    );

    let span_start = kott.record_block + (name.bit_offset / 8) as usize;
    let span = span_start..span_start + (name.bit_width / 8) as usize;
    let mut order: Vec<&Table> = db.tables.iter().collect();
    order.sort_by_key(|t| t.block_offset);
    let pos = order
        .iter()
        .position(|t| t.block_offset == kott.block_offset)
        .unwrap();
    let crc_word = match order.get(pos + 1) {
        Some(next) => next.block_offset..next.block_offset + 4,
        None => original.len() - 4..original.len(),
    };
    let differing: Vec<usize> = (0..original.len())
        .filter(|&i| original[i] != edited[i])
        .collect();
    assert!(!differing.is_empty());
    assert!(
        differing
            .iter()
            .all(|i| span.contains(i) || crc_word.contains(i)),
        "bytes changed outside the field span and its CRC word: {:?}",
        differing
            .iter()
            .filter(|i| !span.contains(i) && !crc_word.contains(i))
            .take(8)
            .collect::<Vec<_>>()
    );
    assert!(
        differing.iter().any(|i| crc_word.contains(i)),
        "the body CRC was recomputed"
    );
    for i in 1..u32::from(kott.current_records) {
        for f in &kott.fields {
            if f.kind == 0 {
                assert_eq!(
                    db.read_inline_str(kott, i, f),
                    reopened.read_inline_str(kott2, i, f)
                );
            } else if f.bit_width <= 64 {
                assert_eq!(db.read_u64(kott, i, f), reopened.read_u64(kott2, i, f));
            }
        }
    }
    for t in &db.tables {
        if t.block_offset == kott.block_offset {
            continue;
        }
        let body = t.record_block
            ..t.record_block + t.current_records as usize * t.record_len_bytes as usize;
        assert_eq!(
            original[body.clone()],
            edited[body],
            "{} body",
            t.id.canonical(db.endian)
        );
    }

    assert_eq!(reopened.serialize(), edited);

    let mut untouched = edited.clone();
    let fifty_one = "x".repeat(51);
    assert!(!reopened.set_inline_str(&mut untouched, kott2, record, &name, &fifty_one));
    assert_eq!(untouched, edited);
    let fifty = "y".repeat(50);
    assert!(reopened.set_inline_str(&mut untouched, kott2, record, &name, &fifty));
    assert_eq!(
        reopened
            .read_inline_str(kott2, record, &name)
            .unwrap()
            .len(),
        replacement.len()
    );
    let full = TdbFile::parse(&untouched).unwrap();
    assert_eq!(
        full.read_inline_str(table(&full, "kOtt"), record, &name)
            .unwrap(),
        fifty.as_bytes()
    );
}

/// The adapter claims exactly the round-trip behaviour and names the case.
pub(crate) fn assert_team_name_declared(t: &Target, a: &Adapter, module: &str, case: &str) {
    let m = mapping(a, "team.name");
    assert_eq!(m.qualification, "automated-tested");
    assert!(m.inverse_verified);
    assert_eq!(
        m.automated_case.as_deref(),
        Some(format!("tdb_canonical::{module}::{case}").as_str())
    );
    assert_eq!(
        m.confidence, "derived",
        "the field's meaning is still derived from its metadata name"
    );
    assert_eq!(m.status.confidence.as_deref(), Some("measured"));
    assert!(t.cites(&m.status.provenance, case));
    assert_eq!(
        m.string_storage.established().unwrap().byte_capacity,
        Some(50)
    );
}

// ============================================================== birth date

/// The stored month / day / year value ranges on the anchor, from the three components the adapter
/// maps — a direct measurement of the index origin (a stored month of 0 with no month of 12 is
/// 0-origin) that does not go through any production probe.
pub(crate) fn birth_date_ranges(db: &TdbFile, snap: &SchemaSnapshot) -> [(u64, u64); 3] {
    let bio = table(db, "ubPc");
    let mut out = [(u64::MAX, 0); 3];
    for (k, (id, name)) in [("qFnd", "year"), ("JKLp", "month"), ("Kswi", "day")]
        .into_iter()
        .enumerate()
    {
        assert_eq!(meta_field(snap, "ubPc", id).name.as_deref(), Some(name));
        let f = field(db, bio, id);
        for i in 0..u32::from(bio.current_records) {
            let v = read(db, bio, i, &f);
            out[k].0 = out[k].0.min(v);
            out[k].1 = out[k].1.max(v);
        }
    }
    out
}

// ============================================================== the ledger

/// The named candidate blocks (unknowns carrying a non-operative proposal) across an adapter's
/// mappings, in declaration order.
pub(crate) fn candidate_blocks(a: &Adapter) -> Vec<String> {
    let mut unknown: Vec<String> = Vec::new();
    for m in &a.mappings {
        for (block, state, proposed) in [
            (
                "transform",
                m.transform.state,
                m.transform.candidate.is_some(),
            ),
            (
                "enumeration",
                m.enumeration.state,
                m.enumeration.candidate.is_some(),
            ),
            (
                "null_sentinel",
                m.null_sentinel.state,
                m.null_sentinel.candidate.is_some(),
            ),
            (
                "string_storage",
                m.string_storage.state,
                m.string_storage.candidate.is_some(),
            ),
            (
                "foreign_key",
                m.foreign_key.state,
                m.foreign_key.candidate.is_some(),
            ),
            (
                "applicability",
                m.applicability.state,
                m.applicability.candidate.is_some(),
            ),
            ("storage", m.storage.state, m.storage.candidate.is_some()),
            ("status", m.status.state, m.status.candidate.is_some()),
        ] {
            if state == ConstraintState::Unknown && proposed {
                unknown.push(format!("{}.{block}", m.canonical));
            }
        }
    }
    assert!(a.capacities.iter().all(|c| c.declared.candidate.is_none()));
    assert!(a
        .companion_rules
        .iter()
        .all(|r| r.declared.candidate.is_none()));
    assert!(a
        .position_discriminator
        .as_ref()
        .unwrap()
        .candidate
        .is_none());
    unknown
}
