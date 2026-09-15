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
    /// The committed ROSTER fixture the production save codec decodes for this target, or `None`
    /// when the target has no such fixture — either none is committed, or the committed one is not
    /// decoded by the committed codec (C0-REG4: PS4 NHL 15 / 20 / 21). A `None` here is a
    /// recorded gap the per-target module names by case; it never lets a capacity case run.
    pub roster: Option<RosterFixture>,
}

/// A committed ROSTER fixture, pinned by outer and decoded-payload identity.
pub(crate) struct RosterFixture {
    pub path: &'static str,
    pub sha256: &'static str,
    /// The inner TDB the production codec yields from the fixture.
    pub payload_sha256: &'static str,
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

/// **Text identity** of a committed metadata XML (C0-REG4-R1): SHA-256 over the bytes after every
/// CRLF pair (`0x0D 0x0A`) is replaced by LF (`0x0A`). Nothing else is touched — a lone CR, a byte
/// order mark, trailing whitespace and every other byte count — so two representations agree under
/// this rule exactly when they are the same text modulo CRLF/LF line endings. The metadata XMLs are
/// committed as LF blobs and checked out as CRLF under `core.autocrlf=true`, so a *byte* hash of
/// the checked-out file is a property of the checkout, not of the repository; this digest is the
/// same on both, and equals the raw blob's SHA-256 wherever the blob is LF. It is **not** byte
/// identity and is never reported as such; binary anchors (databases, saves) keep [`read_pinned`].
pub(crate) fn text_identity_sha256(bytes: &[u8]) -> String {
    let mut normalised = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            i += 1;
            continue;
        }
        normalised.push(bytes[i]);
        i += 1;
    }
    sha256_hex(&normalised)
}

/// Read a committed text file and pin its text identity (see [`text_identity_sha256`]). Returns
/// the bytes exactly as read — callers that compare two such files compare what the checkout
/// holds, and say so.
pub(crate) fn read_pinned_text(rel: &str, expected_text_sha256: &str) -> Vec<u8> {
    read_pinned_text_at(&repo_root().join(rel), rel, expected_text_sha256)
}

pub(crate) fn read_pinned_text_at(path: &Path, label: &str, expected_text_sha256: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{label}: {e}"));
    assert_eq!(
        text_identity_sha256(&bytes),
        expected_text_sha256,
        "{label}: not the committed text (LF-normalised digest)"
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

    /// The decodable ROSTER fixture's path; a target without one has no capacity case to cite it.
    pub(crate) fn roster_fixture(&self) -> &'static str {
        self.roster_fixture_identity().path
    }

    fn roster_fixture_identity(&self) -> &RosterFixture {
        self.roster.as_ref().unwrap_or_else(|| {
            panic!(
                "{}: no ROSTER fixture the committed codec decodes is declared for this target",
                self.target
            )
        })
    }

    pub(crate) fn cites(&self, provenance: &[String], case: &str) -> bool {
        provenance
            .iter()
            .any(|p| p == &format!("{}#{case}", self.cases_file))
    }

    /// The decoded ROSTER fixture: identity and framing asserted on the way in, through the
    /// production save codec.
    pub(crate) fn decoded_roster_fixture(&self, framing: &RosterFraming) -> Vec<u8> {
        let fixture = self.roster_fixture_identity();
        let outer = read_pinned(fixture.path, fixture.sha256);
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
        assert_eq!(sha256_hex(&inner), fixture.payload_sha256);
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
                .any(|p| p == self.roster_fixture())
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

// ============================================================== PS4 (gen8) mechanics

/// The framing a PS4 `DATA` ROSTER fixture is expected to show through the committed production
/// codec (`tdb_savedata::ps4_extract_roster`: record stream, LZ4 frame, little-endian TDB v6).
/// `size_ok` is stated per fixture because the committed `RosterCustom0` saves are re-serialized
/// files (`size_ok = false`, as the roster-capacity audit records) while the NHL 16 ROSTER is
/// pristine.
pub(crate) struct Ps4RosterFraming {
    pub outer_len: usize,
    pub inner_len: usize,
    pub table_count: usize,
    pub size_ok: bool,
}

impl Target {
    /// The decoded PS4 ROSTER fixture: identity, codec recognition and framing asserted on the way
    /// in, through the committed production PS4 codec. Separate from [`Target::decoded_roster_fixture`]
    /// because the container is a different format (no `PS3RosterFile` / `RosterFile` header, no
    /// version word, an LZ4 frame instead of zlib or raw) and the inner TDB is little-endian.
    pub(crate) fn decoded_ps4_roster_fixture(&self, framing: &Ps4RosterFraming) -> Vec<u8> {
        let fixture = self.roster_fixture_identity();
        let outer = read_pinned(fixture.path, fixture.sha256);
        assert_eq!(outer.len(), framing.outer_len);
        assert!(
            tdb_savedata::is_ps4_save(&outer),
            "the committed codec recognises the DATA stream"
        );
        let inner =
            tdb_savedata::ps4_extract_roster(&outer).expect("the LZ4 roster record decodes");
        assert_eq!(inner.len(), framing.inner_len);
        assert_eq!(sha256_hex(&inner), fixture.payload_sha256);
        let save = TdbFile::parse(&inner).expect("the inner TDB parses");
        assert!(save.header_crc_ok(), "header CRC recomputes");
        assert_eq!(
            save.size_ok(),
            framing.size_ok,
            "pristine or re-serialized, as recorded"
        );
        assert_eq!(save.endian, tdb_core::Endian::Little);
        assert_eq!(save.tables.len(), framing.table_count);
        inner
    }

    /// Every disagreement between a `main-db-max-records` capacity and the main anchor it must have
    /// been read from. A capacity attributed to any other file — a ROSTER save included — is
    /// reported as not this anchor's. Ceiling >= population is checked within THIS source and
    /// nowhere against another file. The PS4 counterpart of
    /// [`Target::roster_capacity_discrepancies`]: on gen8 the main DB carries the slack, so its own
    /// directory is the file a capacity is read from; a save's directory is a separate measurement
    /// and never substitutes for it or is substituted by it.
    pub(crate) fn main_db_capacity_discrepancies(
        &self,
        adapter: &Adapter,
        main: &TdbFile,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for cap in &adapter.capacities {
            let Some(v) = cap.declared.established() else {
                out.push(format!("capacity {}: not established", cap.table));
                continue;
            };
            if v.source != "main-db-max-records" {
                out.push(format!(
                    "capacity {}: source {} is not the main anchor this case parses",
                    cap.table, v.source
                ));
                continue;
            }
            if !cap
                .declared
                .provenance
                .iter()
                .any(|p| p == self.main_anchor)
            {
                out.push(format!(
                    "capacity {}: provenance does not name the anchor it was read from",
                    cap.table
                ));
            }
            let Some(t) = main.table_canonical(&cap.table) else {
                out.push(format!(
                    "capacity {}: table not in the main anchor",
                    cap.table
                ));
                continue;
            };
            if v.ceiling_records != u32::from(t.max_records) {
                out.push(format!(
                    "capacity {}: ceiling_records {} vs anchor max_records {}",
                    cap.table, v.ceiling_records, t.max_records
                ));
            }
            if v.population_records != u32::from(t.current_records) {
                out.push(format!(
                    "capacity {}: population_records {} vs anchor current_records {}",
                    cap.table, v.population_records, t.current_records
                ));
            }
            if t.max_records < t.current_records {
                out.push(format!(
                    "capacity {}: anchor max_records {} below its own current_records {}",
                    cap.table, t.max_records, t.current_records
                ));
            }
        }
        out
    }

    /// The whole main-DB capacity case for a PS4 target: the anchor's directory as read, and the
    /// adapter declaring exactly that, attributed to this anchor, measured, with the note saying
    /// what a directory capacity is not and that it is not the save's capacity.
    pub(crate) fn assert_capacities_are_the_main_anchors(
        &self,
        main: &TdbFile,
        expected: &[(&str, u16, u16)],
        capacity_case: &str,
    ) {
        for (id, max, cur) in expected {
            let t = table(main, id);
            assert_eq!(
                (t.max_records, t.current_records),
                (*max, *cur),
                "{id}: (max_records, current_records)"
            );
        }
        let a = self.adapter();
        assert_eq!(
            self.main_db_capacity_discrepancies(&a, main),
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
            assert!(
                cap.declared.note.contains("NOT a substitute"),
                "{}: the note says the main-DB figure does not stand in for the save's",
                cap.table
            );
        }
    }
}

/// The eight roster-core tables' directories in a decoded save against the main anchor's, as an
/// observation about two files: (table, save max, save cur, main max, main cur).
pub(crate) fn directory_comparison(
    save: &TdbFile,
    main: &TdbFile,
    tables: &[&str],
) -> Vec<(String, u16, u16, u16, u16)> {
    tables
        .iter()
        .map(|id| {
            let s = table(save, id);
            let m = table(main, id);
            (
                id.to_string(),
                s.max_records,
                s.current_records,
                m.max_records,
                m.current_records,
            )
        })
        .collect()
}

// ============================================================== position / companions

/// Bio rows partitioned by which companion tables hold their key, with the skater/goalie label of
/// each companion read from EA's metadata name for that table.
pub(crate) struct CompanionPartition {
    pub skaters: BTreeSet<u32>,
    pub goalies: BTreeSet<u32>,
}

pub(crate) fn companion_partition(db: &TdbFile, snap: &SchemaSnapshot) -> CompanionPartition {
    let (part, uncovered) = companion_partition_with(db, snap, &["dSvy", "xmja", "mHuy", "fMVl"]);
    assert!(
        uncovered.is_empty(),
        "every bio row has companions (rows without: {uncovered:?})"
    );
    part
}

/// The partition over an explicit companion set (gen6 NHL 07 has no equipment tables), returning
/// the bio rows that own no companion row at all instead of refusing them (C0-REG5: NHL 09 holds
/// four such rows). Every companion is still one row per key, every key still resolves to a bio
/// row, and the companions of one role still cover one set.
pub(crate) fn companion_partition_with(
    db: &TdbFile,
    snap: &SchemaSnapshot,
    companion_ids: &[&str],
) -> (CompanionPartition, BTreeSet<u32>) {
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
    let companions: Vec<(&str, &'static str)> =
        companion_ids.iter().map(|id| (*id, role(id))).collect();
    assert!(
        companions.iter().any(|c| c.1 == "skater") && companions.iter().any(|c| c.1 == "goalie")
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
    let uncovered: BTreeSet<u32> = (0..u32::from(bio.current_records))
        .filter(|i| !skaters.contains(i) && !goalies.contains(i))
        .collect();
    assert_eq!(
        skaters.len() + goalies.len() + uncovered.len(),
        bio_by_key.len()
    );
    (CompanionPartition { skaters, goalies }, uncovered)
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

/// The four companions of every NHL 08 → NHL 21 anchor, one-to-one by position.
pub(crate) const GEN7_COMPANIONS: &[(&str, &str, &str)] = &[
    ("dSvy", "one-to-one", "position-skater"),
    ("xmja", "one-to-one", "position-skater"),
    ("mHuy", "one-to-one", "position-goalie"),
    ("fMVl", "one-to-one", "position-goalie"),
];

/// The adapter's discriminator, companion rule and the applicability of every rating in `rated`
/// (the skater-companion properties this adapter maps — a title without a stored overall maps one)
/// declare exactly the measured partition and cite the named case. `companions` is the measured
/// (table, cardinality, applies_when) set — [`GEN7_COMPANIONS`] on every NHL 08+ anchor; two
/// companions on gen6 NHL 07, and `zero-or-one` skater companions where an anchor holds bio rows
/// without any (C0-REG5).
pub(crate) fn assert_discriminator_declared(
    t: &Target,
    a: &Adapter,
    skater_codes: &[i64],
    goalie_codes: &[i64],
    rated: &[&str],
    companions: &[(&str, &str, &str)],
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
    assert_eq!(declared, companions);

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

/// How a target stores the main-team pointer relative to the roster slot's team code. Measured
/// per anchor by [`main_team_measurement`]; a title's regime is never carried to a neighbour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainTeamEncoding {
    /// The stored pattern is the slot's team code + 1, and pattern 0 is stored on every un-rostered
    /// player and on no rostered one (NHL 11 → Legacy anchors).
    SlotTeamPlusOne,
    /// The stored pattern is the slot's team code as-is. Pattern 0 is then also team code 0, so a
    /// rostered player on that team and an un-rostered player store the same bits (NHL 10 anchors).
    SlotTeamAsIs,
}

/// Per rostered player exactly one slot must carry the encoded pattern (`team + 1` or `team`, by
/// `encoding`); under `SlotTeamPlusOne` no rostered player may store 0, under `SlotTeamAsIs` the
/// count of rostered rows storing 0 is simply what team code 0 holds. Each un-rostered player is
/// counted by whether it stores 0 — an un-rostered row storing a non-zero pattern is counted, not
/// refused (C0-REG4: the PS4 anchors hold some). Containment of every slot code in the team
/// table's key set is asserted as an observation. `bit_width` is the field's width on this anchor
/// (8 on every gen7 title; 8, 9 or 10 across the PS4 titles), so the all-ones pattern is the
/// target's own and never a carried `0xFF`.
pub(crate) fn main_team_measurement(
    db: &TdbFile,
    snap: &SchemaSnapshot,
    slots: &BTreeMap<u32, Vec<u64>>,
    encoding: MainTeamEncoding,
    bit_width: u32,
) -> MainTeamCounts {
    let bio = table(db, "ubPc");
    let dxsb = field(db, bio, "dXSB");
    assert_eq!(
        meta_field(snap, "ubPc", "dXSB").name.as_deref(),
        Some("team")
    );
    assert_eq!(dxsb.bit_width, bit_width, "this anchor's width");
    let all_ones = (1u64 << bit_width) - 1;
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
        if raw == all_ones {
            c.all_ones += 1;
        }
        match slots.get(&i) {
            Some(teams) => {
                c.rostered += 1;
                let plus_one = teams.iter().filter(|&&t| t + 1 == raw).count();
                let same = teams.iter().filter(|&&t| t == raw).count();
                c.plus_one += plus_one;
                c.same += same;
                let hits = match encoding {
                    MainTeamEncoding::SlotTeamPlusOne => {
                        assert_ne!(raw, 0, "bio record {i} is rostered but stores 0");
                        plus_one
                    }
                    MainTeamEncoding::SlotTeamAsIs => same,
                };
                assert_eq!(
                    hits, 1,
                    "bio record {i}: exactly one of its slots {teams:?} is its main team (raw {raw}, {encoding:?})"
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

/// The adapter declares the main-team encoding as measured, keeps the parent a candidate, keeps
/// the mapping blocked on that, and has no human range (it is a reference). Under
/// `SlotTeamPlusOne` the storage is the offset-binary `-1..2^w-2` field (`-1..254` at the gen7
/// width of 8) and the absence code 0 is established; under `SlotTeamAsIs` the storage is the
/// unsigned `0..2^w-1` field and no absence code is established — pattern 0 is team code 0 as
/// much as it is "no team", so the sentinel block must stay unknown with its ambiguity recorded.
/// `bit_width` is the field's width on this anchor, as measured.
pub(crate) fn assert_main_team_declared(
    t: &Target,
    a: &Adapter,
    case: &str,
    encoding: MainTeamEncoding,
    bit_width: u32,
) {
    let m = mapping(a, "player.team");
    let st = m.storage.established().expect("storage established");
    assert_eq!(st.bit_width, bit_width, "this anchor's width");
    assert!(t.cites(&m.storage.provenance, case));
    let top = (1i64 << bit_width) - 1;
    if encoding == MainTeamEncoding::SlotTeamAsIs {
        assert_eq!(
            (st.signedness.as_str(), st.range_low, st.range_high),
            ("unsigned", 0, top)
        );
        assert_eq!(
            (m.read_transform.as_str(), m.write_transform.as_str()),
            ("index-origin-0", "index-origin-0")
        );
        assert_eq!(m.qualification, "blocked");
        assert!(!m.inverse_verified);
        let tr = m.transform.established().expect("transform established");
        assert_eq!(
            (tr.kind.as_str(), tr.index_origin),
            ("index-origin", Some(0))
        );
        assert_eq!(m.transform.confidence.as_deref(), Some("measured"));
        assert!(t.cites(&m.transform.provenance, case));
        assert_eq!(human_range(m), None);
        assert_eq!(m.null_sentinel.state, ConstraintState::Unknown);
        let c = m
            .null_sentinel
            .candidate
            .as_ref()
            .expect("the ambiguity is recorded as a candidate, not silence");
        assert_eq!(c.confidence.as_deref(), Some("asserted"));
        assert!(t.cites(&c.provenance, case));
        assert!(c.note.contains("NOT OPERATIVE"));
        assert!(
            m.null_sentinel.note.contains("team code 0"),
            "the unknown block says why: pattern 0 is also a team"
        );
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
        return;
    }
    assert_eq!(
        (st.signedness.as_str(), st.range_low, st.range_high),
        ("offset-binary", -1, top - 1)
    );
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

/// The same pipeline over an anchor on which some un-rostered bio rows store a non-zero pattern
/// (C0-REG4: the PS4 anchors). Absence ⇔ no roster slot still holds for pattern 0; every rostered
/// row decodes to one of its slot codes; an un-rostered row with a non-zero pattern decodes to a
/// team code that resolves to NO roster slot, and is returned by pattern so the caller can assert
/// exactly which patterns those are rather than have them silently pass. The inverse restores every
/// pattern. Returns (absent, coded and rostered, coded-but-unrostered by stored pattern).
pub(crate) fn codec_round_trip_over_anchor_with_unrostered_pointers(
    m: &Mapping,
    db: &TdbFile,
    slots: &BTreeMap<u32, Vec<u64>>,
) -> (usize, usize, BTreeMap<u64, usize>) {
    let width = m.storage.established().unwrap().bit_width;
    for pattern in 0..(1u64 << width) {
        assert_eq!(encode_stored(m, decode_stored(m, pattern)), pattern);
    }
    let bio = table(db, "ubPc");
    let dxsb = field(db, bio, "dXSB");
    let (mut absent, mut coded) = (0usize, 0usize);
    let mut unrostered: BTreeMap<u64, usize> = BTreeMap::new();
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
            (Decoded::TeamCode(_), None) => *unrostered.entry(pattern).or_default() += 1,
            (Decoded::Absent, Some(teams)) => {
                panic!("bio record {i}: absent but rostered on {teams:?}")
            }
        }
        assert_eq!(encode_stored(m, value), pattern, "bio record {i}");
    }
    (absent, coded, unrostered)
}

// ============================================================== team.name

/// `kOtt.YmkJ` (at `field_bit_offset`, the target's own) read / edit / serialize / reopen on an
/// in-memory copy of `original`: the unedited copy re-serializes byte-identically; record 0 reads
/// `vanilla`; after `set_inline_str` +
/// `recompute_crcs` the reopened copy reads `replacement` on record 0 and `vanilla_record_1` on
/// record 1; the only differing bytes are the field span of record 0 and the one CRC word that
/// hashes this table's body; every other kOtt record's every field and every other table's record
/// block compare equal; a second no-op serialization is stable; a 51-byte value is refused with
/// nothing written and a 50-byte value is accepted. The gen7 / NHL 15 PS4 shape of
/// [`kott_string_round_trip`]: the 400-bit `fullname` field.
pub(crate) fn team_name_round_trip(
    original: &[u8],
    field_bit_offset: u32,
    vanilla: &str,
    vanilla_record_1: &str,
    replacement: &str,
) {
    kott_string_round_trip(
        original,
        "YmkJ",
        field_bit_offset,
        400,
        vanilla,
        vanilla_record_1,
        replacement,
    )
}

/// The same read / edit / serialize / reopen discipline over any inline-string field of `kOtt`
/// (`field_id`, at `field_bit_offset` and `field_bit_width` — all the target's own): the over-budget
/// value refused is `capacity + 1` bytes and the accepted one exactly `capacity` bytes, where
/// capacity is the field's byte width. C0-REG4 exercises the 320- and 256-bit `teamname` fields of
/// the PS4 titles with it; the gen7 modules keep calling [`team_name_round_trip`].
pub(crate) fn kott_string_round_trip(
    original: &[u8],
    field_id: &str,
    field_bit_offset: u32,
    field_bit_width: u32,
    vanilla: &str,
    vanilla_record_1: &str,
    replacement: &str,
) {
    inline_string_round_trip(
        original,
        "kOtt",
        field_id,
        field_bit_offset,
        field_bit_width,
        vanilla,
        vanilla_record_1,
        replacement,
    )
}

/// The same discipline over an inline-string field of any table (`table_id`): C0-REG5 exercises
/// gen6 NHL 07's `stockteamtable.fullname` with it, the table its thin team index points at.
#[allow(clippy::too_many_arguments)]
pub(crate) fn inline_string_round_trip(
    original: &[u8],
    table_id: &str,
    field_id: &str,
    field_bit_offset: u32,
    field_bit_width: u32,
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
    let kott = table(&db, table_id);
    let name = field(&db, kott, field_id);
    assert_eq!(
        (name.kind, name.bit_offset, name.bit_width),
        (0, field_bit_offset, field_bit_width),
        "the field sits where this target's snapshot says (the offset differs across titles)"
    );
    let capacity = (field_bit_width / 8) as usize;
    let record = 0;
    assert_eq!(
        db.read_inline_str(kott, record, &name).unwrap(),
        vanilla.as_bytes(),
        "byte-transparent"
    );

    assert!(replacement.len() < capacity);
    let mut edited = original.to_vec();
    assert!(db.set_inline_str(&mut edited, kott, record, &name, replacement));
    db.recompute_crcs(&mut edited);

    let reopened = TdbFile::parse(&edited).unwrap();
    assert!(reopened.header_crc_ok() && reopened.size_ok());
    let kott2 = table(&reopened, table_id);
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
    let over = "x".repeat(capacity + 1);
    assert!(!reopened.set_inline_str(&mut untouched, kott2, record, &name, &over));
    assert_eq!(untouched, edited);
    let exact = "y".repeat(capacity);
    assert!(reopened.set_inline_str(&mut untouched, kott2, record, &name, &exact));
    assert_eq!(
        reopened
            .read_inline_str(kott2, record, &name)
            .unwrap()
            .len(),
        replacement.len()
    );
    let full = TdbFile::parse(&untouched).unwrap();
    assert_eq!(
        full.read_inline_str(table(&full, table_id), record, &name)
            .unwrap(),
        exact.as_bytes()
    );
}

// ============================================================== candidate-only mechanics
//
// For a target whose bytes cannot be attributed to it (C0-REG4-R1: the byte-identical `nhl19` /
// `nhl20` folders) or that is a research copy outside the shipping targets (C0-REG5: `nhl07-ps3`),
// the adapter carries every measured block as `state = "unknown"` + `[*.candidate]`. These helpers
// assert that shape — value present, confidence as measured, the reason stated on the block, "NOT
// OPERATIVE" on the candidate — and hand the candidate back so the runner can compare it with the
// measurement.

/// The reasons an unknown block may give for carrying a measurement only as a candidate.
pub(crate) const NON_OPERATIVE_REASONS: &[&str] = &["ATTRIBUTION UNRESOLVED", "RESEARCH COPY"];

pub(crate) fn candidate_of<'a, T>(c: &'a Constraint<T>, what: &str) -> &'a Candidate<T> {
    assert_eq!(c.state, ConstraintState::Unknown, "{what}: not established");
    assert!(c.value.is_none() && c.confidence.is_none(), "{what}");
    assert!(
        NON_OPERATIVE_REASONS.iter().any(|r| c.note.contains(r)),
        "{what}: the unknown block says why"
    );
    let cand = c
        .candidate
        .as_ref()
        .unwrap_or_else(|| panic!("{what}: the measurement is preserved as a candidate"));
    assert!(cand.value.is_some(), "{what}: candidate carries the value");
    assert!(cand.note.contains("NOT OPERATIVE"), "{what}");
    assert!(!cand.provenance.is_empty(), "{what}");
    cand
}

pub(crate) fn candidate_value<'a, T>(c: &'a Constraint<T>, what: &str) -> &'a T {
    candidate_of(c, what).value.as_ref().unwrap()
}

/// The capacity block's note and provenance wherever they live: on the established block, or on
/// the candidate of an unresolved one.
pub(crate) fn capacity_note_and_provenance(
    cap: &CapacityDecl,
    unresolved: bool,
) -> (&str, &[String]) {
    if unresolved {
        let c = candidate_of(&cap.declared, &format!("capacity {}", cap.table));
        (c.note.as_str(), &c.provenance)
    } else {
        (cap.declared.note.as_str(), &cap.declared.provenance)
    }
}

/// A copy of the adapter with every unknown-with-candidate block promoted to established (value,
/// confidence and provenance from the candidate). **Test-only**: it says what the measurements would
/// declare *if* the bytes were attributed, so that the measured numbers can be checked through the
/// established-only mechanics; nothing in the tree ever reads such a copy.
pub(crate) fn promoted(a: &Adapter) -> Adapter {
    fn lift<T: Clone>(c: &mut Constraint<T>) {
        if c.state == ConstraintState::Unknown {
            if let Some(cand) = c.candidate.take() {
                // Only a demoted measurement is lifted — the unknown block says why it is one; a
                // candidate the attributed adapter would carry anyway (an asserted range
                // inference, the parent proposal) stays one.
                if !NON_OPERATIVE_REASONS.iter().any(|r| c.note.contains(r)) {
                    c.candidate = Some(cand);
                    return;
                }
                if let Some(v) = cand.value {
                    c.state = ConstraintState::Established;
                    c.value = Some(v);
                    c.confidence = cand.confidence;
                    c.provenance = cand.provenance;
                    c.note = cand.note;
                } else {
                    c.candidate = Some(cand);
                }
            }
        }
    }
    let mut a = a.clone();
    for m in &mut a.mappings {
        lift(&mut m.storage);
        lift(&mut m.status);
        lift(&mut m.transform);
        lift(&mut m.null_sentinel);
        lift(&mut m.string_storage);
        lift(&mut m.applicability);
        for c in &mut m.components {
            lift(&mut c.storage);
        }
    }
    for c in &mut a.capacities {
        lift(&mut c.declared);
    }
    for r in &mut a.companion_rules {
        lift(&mut r.declared);
    }
    if let Some(d) = &mut a.position_discriminator {
        lift(d);
    }
    for m in &mut a.mappings {
        if let Some(t) = m.transform.established() {
            let spelling = match t.kind.as_str() {
                "index-origin" => format!("index-origin-{}", t.index_origin.unwrap()),
                "linear-offset" => format!("offset{:+}", t.offset.unwrap()),
                other => other.to_string(),
            };
            m.read_transform = spelling.clone();
            m.write_transform = spelling;
        }
    }
    a
}

/// The unresolved adapter's blocks with a candidate, in declaration order, mappings first then
/// capacities, companion rule and discriminator — the counterpart of `candidate_blocks`, which
/// refuses non-mapping candidates because on an attributed target they never exist.
pub(crate) fn all_candidate_blocks(a: &Adapter) -> Vec<String> {
    let mut out = Vec::new();
    for m in &a.mappings {
        for (block, present) in [
            ("storage", m.storage.candidate.is_some()),
            ("status", m.status.candidate.is_some()),
            ("transform", m.transform.candidate.is_some()),
            ("enumeration", m.enumeration.candidate.is_some()),
            ("null_sentinel", m.null_sentinel.candidate.is_some()),
            ("string_storage", m.string_storage.candidate.is_some()),
            ("foreign_key", m.foreign_key.candidate.is_some()),
            ("applicability", m.applicability.candidate.is_some()),
        ] {
            if present {
                out.push(format!("{}.{block}", m.canonical));
            }
        }
    }
    for c in &a.capacities {
        if c.declared.candidate.is_some() {
            out.push(format!("capacity.{}", c.table));
        }
    }
    for r in &a.companion_rules {
        if r.declared.candidate.is_some() {
            out.push(format!("companion_rule.{}", r.primary_table));
        }
    }
    if a.position_discriminator
        .as_ref()
        .is_some_and(|d| d.candidate.is_some())
    {
        out.push("position_discriminator".into());
    }
    out
}

/// The adapter claims exactly the round-trip behaviour and names the case; `byte_capacity` is the
/// mapped field's budget on this target (50 for every `fullname` mapping).
pub(crate) fn assert_team_name_declared(
    t: &Target,
    a: &Adapter,
    module: &str,
    case: &str,
    byte_capacity: u32,
) {
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
        Some(byte_capacity)
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

// ============================================================== line-ending portability

/// The text-identity rule, proven on one committed metadata XML: the LF and CRLF representations
/// (and a mixed one) agree with each other and with the pin; a substantive change to the XML does
/// not; a byte hash of the checked-out file is not what is pinned. The disposable copies are
/// written to a scratch directory so that the pinned read runs against real files in both
/// representations, whatever line endings this checkout happens to have.
#[cfg(test)]
mod line_endings {
    use super::*;

    const XML: &str = "anchors/ps4/nhl15/databases/nhl_game-meta.xml";
    /// SHA-256 of the committed LF blob = its text identity.
    const XML_TEXT_SHA256: &str =
        "2baa3399343a8d17da42731162963d6b272b651a5d1a4e1639d06b48d694a1c4";

    fn lf(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                i += 1;
                continue;
            }
            out.push(bytes[i]);
            i += 1;
        }
        out
    }

    fn crlf(lf_bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(lf_bytes.len() * 2);
        for &b in lf_bytes {
            if b == b'\n' {
                out.push(b'\r');
            }
            out.push(b);
        }
        out
    }

    #[test]
    fn lf_and_crlf_representations_have_the_same_text_identity_and_it_is_the_pin() {
        let as_checked_out = std::fs::read(repo_root().join(XML)).unwrap();
        let lf_form = lf(&as_checked_out);
        let crlf_form = crlf(&lf_form);
        assert_ne!(lf_form, crlf_form, "two different byte representations");
        assert_eq!(text_identity_sha256(&lf_form), XML_TEXT_SHA256);
        assert_eq!(text_identity_sha256(&crlf_form), XML_TEXT_SHA256);
        assert_eq!(
            sha256_hex(&lf_form),
            XML_TEXT_SHA256,
            "the LF form is the blob"
        );
        assert_ne!(
            sha256_hex(&crlf_form),
            XML_TEXT_SHA256,
            "a byte hash of the CRLF form is a checkout property, not the pin"
        );
        // A mixed representation (only the first line CRLF) is the same text.
        let mut mixed = lf_form.clone();
        let first_nl = mixed.iter().position(|&b| b == b'\n').unwrap();
        mixed.insert(first_nl, b'\r');
        assert_eq!(text_identity_sha256(&mixed), XML_TEXT_SHA256);
        // Whatever this checkout holds, the pinned read accepts it.
        assert_eq!(lf(&read_pinned_text(XML, XML_TEXT_SHA256)), lf_form);
    }

    #[test]
    fn a_substantive_xml_change_fails_the_pin_in_either_representation() {
        let lf_form = lf(&std::fs::read(repo_root().join(XML)).unwrap());
        let needle = b"firstname";
        let at = lf_form
            .windows(needle.len())
            .position(|w| w == needle)
            .expect("the bio table's firstname descriptor is in the metadata");
        let mut changed = lf_form.clone();
        changed[at..at + needle.len()].copy_from_slice(b"firstnamX");
        assert_ne!(text_identity_sha256(&changed), XML_TEXT_SHA256);
        assert_ne!(text_identity_sha256(&crlf(&changed)), XML_TEXT_SHA256);
        // A lone CR is content, not a line ending, and is not normalised away.
        let mut lone_cr = lf_form.clone();
        lone_cr.insert(at, b'\r');
        assert_ne!(text_identity_sha256(&lone_cr), XML_TEXT_SHA256);
    }

    #[test]
    fn the_pinned_read_accepts_lf_and_crlf_disposable_copies_and_refuses_a_changed_one() {
        let lf_form = lf(&std::fs::read(repo_root().join(XML)).unwrap());
        let dir =
            std::env::temp_dir().join(format!("tdb-canonical-line-endings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let lf_path = dir.join("meta-lf.xml");
        let crlf_path = dir.join("meta-crlf.xml");
        let changed_path = dir.join("meta-changed.xml");
        std::fs::write(&lf_path, &lf_form).unwrap();
        std::fs::write(&crlf_path, crlf(&lf_form)).unwrap();
        let mut changed = crlf(&lf_form);
        let at = changed.windows(9).position(|w| w == b"firstname").unwrap();
        changed[at] = b'F';
        std::fs::write(&changed_path, &changed).unwrap();

        let a = read_pinned_text_at(&lf_path, "lf copy", XML_TEXT_SHA256);
        let b = read_pinned_text_at(&crlf_path, "crlf copy", XML_TEXT_SHA256);
        assert_ne!(a, b, "the bytes read differ; the text identity does not");
        let refused = std::panic::catch_unwind(|| {
            read_pinned_text_at(&changed_path, "changed copy", XML_TEXT_SHA256)
        });
        assert!(refused.is_err(), "a substantive change is refused");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
