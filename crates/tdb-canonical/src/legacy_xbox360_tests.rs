//! NHL Legacy, Xbox 360: semantic and capacity declarations checked against **this target's own
//! committed evidence** (package C0-REG1a-R1).
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
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tdb_core::{FieldDescriptor, Table, TdbFile};
use tdb_qa::schema::{snapshot, SchemaSnapshot};

const ADAPTER: &str = "nhllegacy-xbox360.toml";
const TARGET: &str = "nhllegacy-xbox360";
const CASES: &str = "crates/tdb-canonical/src/legacy_xbox360_tests.rs";

/// The main-database anchor the adapter's snapshot was rendered from.
const MAIN_ANCHOR: &str = "anchors/xbox360/nhllegacy/databases/nhlng.db";
const MAIN_ANCHOR_SHA256: &str = "105d2186da3d1f0f3d3ea8a350ead42ac9d5d802ca9fd37652c39bb4e1f1c230";

/// The committed vanilla Xbox 360 Legacy ROSTER save (already outside its STFS wrapper: it starts
/// with the `RosterFile` container magic).
const ROSTER_FIXTURE: &str =
    "anchors/xbox360/nhllegacy/savefiles/ROSTER 20260627060052/ROSTER 20260627060052";
const ROSTER_FIXTURE_SHA256: &str =
    "65a04268a2d1321ee495bd0c2d1509748c2d12ec3495a038d7a567f616ad7d21";
/// The inner TDB the production codec extracts from it (zlib body inflated, trimmed to `db_size`).
const ROSTER_PAYLOAD_SHA256: &str =
    "aeadd37cb4a74c39ec3972a161f83e5abf37cf3cf5c40c6eadad0160877796db";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn adapter() -> Adapter {
    let text = std::fs::read_to_string(repo_root().join("defs/adapters").join(ADAPTER))
        .expect("adapter is in the tree");
    let a = parse_adapter(&text).unwrap_or_else(|e| panic!("{ADAPTER} parses: {e}"));
    assert_eq!(a.target, TARGET);
    a
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Read a committed file and pin its identity: a case that measured a different file would say so.
fn read_pinned(rel: &str, expected_sha256: &str) -> Vec<u8> {
    let bytes = std::fs::read(repo_root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
    assert_eq!(
        sha256_hex(&bytes),
        expected_sha256,
        "{rel}: not the committed bytes"
    );
    bytes
}

fn table<'a>(db: &'a TdbFile, id: &str) -> &'a Table {
    db.table_canonical(id)
        .unwrap_or_else(|| panic!("table {id} is in the database"))
}

fn field(db: &TdbFile, t: &Table, id: &str) -> FieldDescriptor {
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

fn read(db: &TdbFile, t: &Table, record: u32, f: &FieldDescriptor) -> u64 {
    db.read_u64(t, record, f).unwrap_or_else(|| {
        panic!(
            "record {record} of {} is readable",
            t.id.canonical(db.endian)
        )
    })
}

/// The committed snapshot of the main anchor, which carries EA's metadata names — the independent
/// label source for the semantic cases below.
fn main_snapshot() -> SchemaSnapshot {
    let anchors = tdb_qa::discover_main_anchors(&repo_root()).expect("anchors/ is readable");
    let anchor = anchors
        .iter()
        .find(|a| a.platform == "xbox360" && a.game == "nhllegacy")
        .expect("the Legacy Xbox 360 main anchor is committed");
    assert_eq!(anchor.db_rel, MAIN_ANCHOR);
    snapshot(&repo_root(), anchor).expect("the anchor parses with its metadata")
}

fn meta_table_name(snap: &SchemaSnapshot, id: &str) -> String {
    snap.tables
        .iter()
        .find(|t| t.canonical_id == id)
        .and_then(|t| t.name.clone())
        .unwrap_or_else(|| panic!("the metadata names table {id}"))
}

fn meta_field<'a>(
    snap: &'a SchemaSnapshot,
    table: &str,
    id: &str,
) -> &'a tdb_qa::schema::FieldSnapshot {
    snap.tables
        .iter()
        .find(|t| t.canonical_id == table)
        .and_then(|t| t.fields.iter().find(|f| f.canonical_id == id))
        .unwrap_or_else(|| panic!("the metadata names field {table}.{id}"))
}

fn mapping<'a>(a: &'a Adapter, canonical: &str) -> &'a Mapping {
    a.mappings
        .iter()
        .find(|m| m.canonical == canonical)
        .unwrap_or_else(|| panic!("{TARGET} maps {canonical}"))
}

fn cites(provenance: &[String], case: &str) -> bool {
    provenance.iter().any(|p| p == &format!("{CASES}#{case}"))
}

// ============================================================== 1. capacity

/// The decoded ROSTER fixture: framing and identity asserted on the way in.
fn decoded_roster_fixture() -> Vec<u8> {
    let outer = read_pinned(ROSTER_FIXTURE, ROSTER_FIXTURE_SHA256);
    assert_eq!(outer.len(), 2_456_120);
    let container =
        tdb_savedata::SaveContainer::parse(&outer).expect("the container header parses");
    let h = &container.header;
    assert_eq!(h.magic, "RosterFile");
    assert_eq!(h.save_type, tdb_savedata::SaveType::Roster);
    assert_eq!(h.version, tdb_savedata::ContainerVersion::ZlibV4);
    assert_eq!(h.version_word, 4);
    assert_eq!(h.uncompressed_size, Some(2_456_076));
    assert_eq!(h.compressed_size, Some(722_116));
    assert!(
        container.checksums_valid(),
        "both container checksums recompute"
    );
    let inner = container
        .extract_db()
        .expect("the zlib body inflates to a TDB");
    // The declared inflate size exceeds `db_size` by the 4-byte trailer the extractor trims.
    assert_eq!(inner.len(), 2_456_072);
    assert_eq!(sha256_hex(&inner), ROSTER_PAYLOAD_SHA256);
    inner
}

/// Every disagreement between a `roster-save-max-records` capacity and the decoded save it must
/// have been read from. A capacity attributed to any other file is reported as unverifiable here.
fn roster_capacity_discrepancies(adapter: &Adapter, save: &TdbFile) -> Vec<String> {
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
        if !cap.declared.provenance.iter().any(|p| p == ROSTER_FIXTURE) {
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
        // Ceiling >= population is checked within THIS source — the save's own directory — and
        // nowhere against a different file's population (C0-REG1a-R2).
        if t.max_records < t.current_records {
            out.push(format!(
                "capacity {}: fixture max_records {} below its own current_records {}",
                cap.table, t.max_records, t.current_records
            ));
        }
    }
    out
}

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
    let inner = decoded_roster_fixture();
    let save = TdbFile::parse(&inner).expect("the inner TDB parses");
    assert!(save.header_crc_ok() && save.size_ok());
    assert_eq!(save.endian, tdb_core::Endian::Big);
    assert_eq!(save.tables.len(), 39);

    // The observations, by table, as read.
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
    for (id, max, cur) in [
        ("ubPc", 7988, 5632),
        ("ZBac", 9955, 6087),
        ("eGlu", 10879, 5532),
        ("dSvy", 7331, 5066),
        ("xmja", 7331, 5066),
        ("mHuy", 957, 566),
        ("fMVl", 957, 566),
        ("kOtt", 252, 252),
    ] {
        assert_eq!(
            observed[id],
            (max, cur),
            "{id}: (max_records, current_records)"
        );
    }

    // Fixture observation: the save's populations equal the main anchor's for every table both
    // carry. Verified, not presumed — a save is a different file.
    let main = read_pinned(MAIN_ANCHOR, MAIN_ANCHOR_SHA256);
    let main = TdbFile::parse(&main).unwrap();
    let mut compared = 0;
    for t in &main.tables {
        if let Some(s) = save.table_canonical(&t.id.canonical(main.endian)) {
            assert_eq!(
                s.current_records,
                t.current_records,
                "{}: save vs main population",
                t.id.canonical(main.endian)
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 39, "every save table exists in the main DB");

    // The adapter declares exactly these, attributes each to this fixture, and calls them measured.
    let a = adapter();
    assert_eq!(
        roster_capacity_discrepancies(&a, &save),
        Vec::<String>::new()
    );
    assert_eq!(a.capacities.len(), 8);
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
            cites(
                &cap.declared.provenance,
                "capacities_are_the_observed_directory_of_the_committed_roster_fixture"
            ),
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

/// The fixture check is load-bearing: a ceiling one off in either direction, a population that is
/// not the save's, a missing attribution, a capacity attributed to another file, or a table the
/// save does not carry are each reported by table. (The PS3 audit's figures happen to coincide with
/// this fixture's, so a value copied from there would not be caught here — it would be caught by
/// the provenance and source checks, which demand the fixture be named.)
#[test]
fn a_capacity_that_is_not_the_fixtures_is_reported() {
    let inner = decoded_roster_fixture();
    let save = TdbFile::parse(&inner).unwrap();
    let base = adapter();
    assert!(roster_capacity_discrepancies(&base, &save).is_empty());
    let mutate = |table: &str, f: &dyn Fn(&mut CapacityDecl)| {
        let mut a = base.clone();
        f(a.capacities.iter_mut().find(|c| c.table == table).unwrap());
        roster_capacity_discrepancies(&a, &save)
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
        c.declared.provenance.retain(|p| p != ROSTER_FIXTURE)
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

/// Bio rows partitioned by which companion tables hold their key, with the skater/goalie label of
/// each companion read from EA's metadata name for that table.
struct CompanionPartition {
    skaters: BTreeSet<u32>,
    goalies: BTreeSet<u32>,
}

fn companion_partition(db: &TdbFile, snap: &SchemaSnapshot) -> CompanionPartition {
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
    let main = read_pinned(MAIN_ANCHOR, MAIN_ANCHOR_SHA256);
    let db = TdbFile::parse(&main).unwrap();
    let snap = main_snapshot();
    assert_eq!(
        meta_field(&snap, "ubPc", "vjla").name.as_deref(),
        Some("position")
    );
    let part = companion_partition(&db, &snap);
    assert_eq!((part.skaters.len(), part.goalies.len()), (5066, 566));

    let bio = table(&db, "ubPc");
    let position = field(&db, bio, "vjla");
    let codes = |rows: &BTreeSet<u32>| -> BTreeSet<u64> {
        rows.iter().map(|&i| read(&db, bio, i, &position)).collect()
    };
    let skater_codes = codes(&part.skaters);
    let goalie_codes = codes(&part.goalies);
    assert_eq!(skater_codes, BTreeSet::from([0, 1, 2, 3]));
    assert_eq!(goalie_codes, BTreeSet::from([4]));
    // Per-row: the code decides the partition, with no exception in 5632 rows.
    for i in 0..u32::from(bio.current_records) {
        let is_goalie_code = read(&db, bio, i, &position) == 4;
        assert_eq!(is_goalie_code, part.goalies.contains(&i), "bio record {i}");
    }

    // The adapter declares exactly this and cites this case.
    let a = adapter();
    let disc = a.position_discriminator.as_ref().expect("declared");
    assert_eq!(disc.state, ConstraintState::Established);
    assert_eq!(disc.confidence.as_deref(), Some("measured"));
    let v = disc.established().unwrap();
    assert_eq!((v.table.as_str(), v.field.as_str()), ("ubPc", "vjla"));
    assert_eq!(v.skater_codes, vec![0, 1, 2, 3]);
    assert_eq!(v.goalie_codes, vec![4]);
    let case = "position_codes_partition_the_bio_table_exactly_as_the_metadata_named_companions_do";
    assert!(cites(&disc.provenance, case));

    let rule = &a
        .companion_rules
        .iter()
        .find(|r| r.primary_table == "ubPc")
        .expect("declared")
        .declared;
    assert_eq!(rule.state, ConstraintState::Established);
    assert_eq!(rule.confidence.as_deref(), Some("measured"));
    assert!(cites(&rule.provenance, case));
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

    // The ratings on `dSvy` apply to exactly the skater-code rows, and say so operatively now.
    for canonical in ["player.rating_overall", "player.rating_speed"] {
        let m = mapping(&a, canonical);
        let ap = m
            .applicability
            .established()
            .unwrap_or_else(|| panic!("{canonical}: established"));
        assert_eq!(ap.kind, "target-predicate", "{canonical}");
        assert_eq!(
            (ap.table.as_deref(), ap.field.as_deref()),
            (Some("ubPc"), Some("vjla"))
        );
        assert_eq!(ap.codes, vec![0, 1, 2, 3], "{canonical}");
        assert_eq!(m.applicability.confidence.as_deref(), Some("measured"));
        assert!(cites(&m.applicability.provenance, case), "{canonical}");
    }
    assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
}

fn schema_for_validation() -> ProjectSchema {
    let text = std::fs::read_to_string(repo_root().join("defs/canonical/project-schema.toml"))
        .expect("the canonical schema is committed");
    parse_project_schema(&text).expect("it parses")
}

// ============================================================== 3. main team

/// The roster graph, walked from the metadata-declared foreign key: `eGlu.XSWT -> ZBac` is
/// `fkparenttable = exhibitionplayers` in EA's metadata; `ZBac.ykFq -> ubPc.wBIz` is a join on the
/// field EA names `playerid` (not declared as a key; every value resolves). Returns, per bio record,
/// the team codes of the roster slots that resolve to it.
fn roster_slots_by_bio_record(db: &TdbFile, snap: &SchemaSnapshot) -> BTreeMap<u32, Vec<u64>> {
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

/// The main-team pointer `ubPc.dXSB`, measured against the roster: for each of the 5110 rostered
/// players exactly one of its roster slots carries `team == dXSB - 1` and none carries
/// `team == dXSB`; all 522 un-rostered players store 0 and no rostered player does; the all-ones
/// code is stored nowhere.
///
/// ESTABLISHED: the encoding (stored index origin 1 over the roster table's team codes) and the
/// absence code (literal 0 = no main team). NOT ESTABLISHED: that the team codes are `kOtt.vfEq` —
/// every `dXSB - 1` and every slot team code is contained in that key set (observed here), but
/// containment plus a field name is not a proven foreign key, so the parent stays a candidate and
/// the mapping stays blocked on it.
#[test]
fn main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered() {
    let main = read_pinned(MAIN_ANCHOR, MAIN_ANCHOR_SHA256);
    let db = TdbFile::parse(&main).unwrap();
    let snap = main_snapshot();
    let slots = roster_slots_by_bio_record(&db, &snap);
    let bio = table(&db, "ubPc");
    let dxsb = field(&db, bio, "dXSB");
    assert_eq!(
        meta_field(&snap, "ubPc", "dXSB").name.as_deref(),
        Some("team")
    );
    assert_eq!(dxsb.bit_width, 8);

    let (mut rostered, mut plus_one, mut same, mut unrostered_zero, mut unrostered_other) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut all_ones = 0usize;
    for i in 0..u32::from(bio.current_records) {
        let raw = read(&db, bio, i, &dxsb);
        if raw == 0xFF {
            all_ones += 1;
        }
        match slots.get(&i) {
            Some(teams) => {
                rostered += 1;
                assert_ne!(raw, 0, "bio record {i} is rostered but stores 0");
                plus_one += teams.iter().filter(|&&t| t + 1 == raw).count();
                same += teams.iter().filter(|&&t| t == raw).count();
                assert_eq!(
                    teams.iter().filter(|&&t| t + 1 == raw).count(),
                    1,
                    "bio record {i}: exactly one of its slots {teams:?} is its main team (raw {raw})"
                );
            }
            None => {
                if raw == 0 {
                    unrostered_zero += 1
                } else {
                    unrostered_other += 1
                }
            }
        }
    }
    assert_eq!((rostered, plus_one, same), (5110, 5110, 0));
    assert_eq!((unrostered_zero, unrostered_other), (522, 0));
    assert_eq!(
        all_ones, 0,
        "the -1 pattern the taxonomy proposes for absence is never stored"
    );

    // Observation only: the codes are contained in the team table's key set. Recorded, not promoted.
    let kott = table(&db, "kOtt");
    let team_key = field(&db, kott, "vfEq");
    let team_ids: BTreeSet<u64> = (0..u32::from(kott.current_records))
        .map(|i| read(&db, kott, i, &team_key))
        .collect();
    assert_eq!(team_ids.len(), 252);
    for teams in slots.values() {
        assert!(teams.iter().all(|t| team_ids.contains(t)));
    }

    // The adapter declares the encoding and the absence code as measured, keeps the parent a
    // candidate, and keeps the mapping blocked on that.
    let a = adapter();
    let m = mapping(&a, "player.team");
    let case = "main_team_pointer_is_the_roster_slot_team_plus_one_and_zero_when_unrostered";
    // The storage decode carries the +1 (offset-binary over range_low = -1: pattern k decodes to
    // k - 1), the transform then says the decoded index starts at 0, and the absence code is the
    // stored bit pattern 0, matched before that decode runs (`SENTINEL_ENCODING`). One shift, once.
    let st = m.storage.established().expect("storage established");
    assert_eq!(
        (st.signedness.as_str(), st.range_low, st.range_high),
        ("offset-binary", -1, 254)
    );
    assert!(cites(&m.storage.provenance, case));
    assert_eq!(
        (m.read_transform.as_str(), m.write_transform.as_str()),
        ("index-origin-0", "index-origin-0")
    );
    assert_eq!(m.qualification, "blocked");
    assert!(
        !m.inverse_verified,
        "no edit through this field is exercised here"
    );
    let t = m.transform.established().expect("transform established");
    assert_eq!((t.kind.as_str(), t.index_origin), ("index-origin", Some(0)));
    assert_eq!(m.transform.confidence.as_deref(), Some("measured"));
    assert!(cites(&m.transform.provenance, case));
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
    assert!(cites(&m.null_sentinel.provenance, case));
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

// ============================================================== 4. handedness

/// Handedness on this anchor: the stored histogram is {0: 3781, 1: 1851}, the same counts
/// `defs/enums/handedness.toml` reports for the uncommitted `default.db` it was confirmed on.
/// Matching counts prove matching counts — not identical records, not file identity, not content
/// equivalence — and say nothing about which code is `left`: that needs a named player with an
/// independently supported hand, and this repository holds none. The mapping stays blocked; nothing
/// is promoted.
#[test]
fn handedness_counts_match_the_enum_files_reported_counts_but_no_label_is_established() {
    let main = read_pinned(MAIN_ANCHOR, MAIN_ANCHOR_SHA256);
    let db = TdbFile::parse(&main).unwrap();
    let snap = main_snapshot();
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

    let a = adapter();
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
}

// ============================================================== 4b. codec order

/// What the sentinel → decode → index pipeline yields for one stored pattern. A **test helper**:
/// it exists to demonstrate the order `SENTINEL_ENCODING` states, over this adapter's `player.team`
/// declaration, and it is not a production executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decoded {
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
fn decode_stored(m: &Mapping, pattern: u64) -> Decoded {
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
fn encode_stored(m: &Mapping, value: Decoded) -> u64 {
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
    let a = adapter();
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
    for pattern in 0..=255u64 {
        assert_eq!(encode_stored(m, decode_stored(m, pattern)), pattern);
    }

    // Then every row of the anchor, against the roster measurement.
    let main = read_pinned(MAIN_ANCHOR, MAIN_ANCHOR_SHA256);
    let db = TdbFile::parse(&main).unwrap();
    let slots = roster_slots_by_bio_record(&db, &main_snapshot());
    let bio = table(&db, "ubPc");
    let dxsb = field(&db, bio, "dXSB");
    let (mut absent, mut coded) = (0usize, 0usize);
    for i in 0..u32::from(bio.current_records) {
        let pattern = read(&db, bio, i, &dxsb);
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
    assert_eq!((absent, coded), (522, 5110));
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
    let original = read_pinned(MAIN_ANCHOR, MAIN_ANCHOR_SHA256);
    let db = TdbFile::parse(&original).unwrap();
    assert_eq!(
        db.serialize(),
        original,
        "an unedited copy re-serializes byte-identically"
    );
    let kott = table(&db, "kOtt");
    let name = field(&db, kott, "YmkJ");
    assert_eq!((name.kind, name.bit_offset, name.bit_width), (0, 352, 400));
    let record = 0;
    let vanilla = db.read_inline_str(kott, record, &name).unwrap();
    assert_eq!(
        vanilla,
        "Anaheim Ducks®".as_bytes(),
        "byte-transparent: the ® is two UTF-8 bytes"
    );

    // Edit on a copy. `®` keeps the multi-byte case in the round trip.
    let replacement = "Mighty Ducks of Anaheim®";
    assert!(replacement.len() < 50);
    let mut edited = original.clone();
    assert!(db.set_inline_str(&mut edited, kott, record, &name, replacement));
    db.recompute_crcs(&mut edited);

    // Reopen and read back.
    let reopened = TdbFile::parse(&edited).unwrap();
    assert!(reopened.header_crc_ok() && reopened.size_ok());
    let kott2 = table(&reopened, "kOtt");
    assert_eq!(
        reopened.read_inline_str(kott2, record, &name).unwrap(),
        replacement.as_bytes()
    );
    assert_eq!(
        reopened.read_inline_str(kott2, 1, &name).unwrap(),
        "Winnipeg Jets™".as_bytes()
    );

    // Everything else is untouched: the only differing bytes are inside the field's 50-byte span of
    // record 0 and the single 4-byte CRC word that hashes this table's body (the successor table's
    // `prior_crc`, or the EOF CRC when this is the last table).
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
    // Unrelated data, read the long way: every other kOtt record and every other table are identical.
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

    // A second, no-op serialization of the reopened copy is stable.
    assert_eq!(reopened.serialize(), edited);

    // Over budget: refused at entry, nothing written.
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

    // The adapter claims exactly this behaviour, and names this case.
    let a = adapter();
    let m = mapping(&a, "team.name");
    assert_eq!(m.qualification, "automated-tested");
    assert!(m.inverse_verified);
    assert_eq!(
        m.automated_case.as_deref(),
        Some(
            "tdb_canonical::legacy_xbox360_tests::team_name_edit_round_trips_on_a_disposable_copy"
        )
    );
    assert_eq!(
        m.confidence, "derived",
        "the field's meaning is still derived from its metadata name"
    );
    assert_eq!(m.status.confidence.as_deref(), Some("measured"));
    assert!(cites(
        &m.status.provenance,
        "team_name_edit_round_trips_on_a_disposable_copy"
    ));
    assert_eq!(
        m.string_storage.established().unwrap().byte_capacity,
        Some(50)
    );
}

// ============================================================== 6. the ledger

/// What this module promoted and what it left, counted, so the receipt's figures are checked.
#[test]
fn the_adapters_population_reflects_exactly_these_measurements() {
    let a = adapter();
    let population = registry_population(std::iter::once(&a));
    assert_eq!(population.len(), 1);
    let p = &population[0];
    assert_eq!(p.mappings, 8);
    assert_eq!(p.omissions, 0);
    assert_eq!(
        (p.established_constraints, p.unknown_constraints, p.candidate_constraints),
        (44, 30, 6),
        "established / unknown / candidate blocks after C0-REG1a-R1 (C0-REG1a: 37 / 37 / 12);          the unknown count includes every block a mapping does not declare at all"
    );
    // The six candidates — unknowns that carry a recorded, non-operative proposal — by name: shoots
    // (transform, enumeration, null_sentinel), the two ratings' null_sentinel, and the main-team
    // parent. Everything promoted by this package has left this list.
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
    assert_eq!(
        unknown,
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
