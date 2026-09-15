//! Physical declarations checked against the **anchor bytes**, not against the TOML they were
//! copied into (package C0-REG1a).
//!
//! An adapter's `[mapping.storage]`, `[mapping.status]`, `[[table]]` names, `coverage_declared`
//! totals and capacity populations all restate facts that exist independently in the target's main
//! database. Here each committed adapter's `physical_snapshot` is resolved to the anchor it names,
//! the anchor is parsed with the same `tdb_qa::schema::snapshot` that renders the committed JSON,
//! and every restated fact is compared. A value that only agrees with the file it was typed into
//! proves nothing; a value that agrees with the anchor is a measurement.
//!
//! Every case reads committed anchors only. Nothing here writes, and nothing here promotes a
//! qualification: agreement with the snapshot establishes physical structure, never semantics.
//!
//! Identity is checked before layout (package C0-REG1a-R1): the adapter's `target` must name the
//! very game and platform the snapshot was rendered from and the anchor that was selected for it.
//! Two Legacy anchors are byte-identical across PS3 and Xbox 360, so a layout comparison alone
//! would accept an adapter attributed to the wrong platform; the identity guard is what refuses it.

use super::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tdb_qa::schema::{snapshot, FieldSnapshot, SchemaSnapshot, TableSnapshot};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema() -> ProjectSchema {
    let text = std::fs::read_to_string(repo_root().join("defs/canonical/project-schema.toml"))
        .expect("the canonical schema is committed");
    parse_project_schema(&text).expect("it parses")
}

fn adapter(name: &str) -> Adapter {
    let text = std::fs::read_to_string(repo_root().join("defs/adapters").join(name))
        .expect("adapter is committed");
    parse_adapter(&text).unwrap_or_else(|e| panic!("{name} parses: {e}"))
}

/// The committed adapters, each of which must name a snapshot and agree with its anchor.
const COMMITTED_ADAPTERS: [&str; 14] = [
    "nhl15-xbox360.toml",
    "nhl10-xbox360.toml",
    "nhllegacy-xbox360.toml",
    "nhl15-ps3.toml",
    "nhllegacy-ps3.toml",
    "nhl14-xbox360.toml",
    "nhl14-ps3.toml",
    "nhl13-xbox360.toml",
    "nhl12-xbox360.toml",
    "nhl11-xbox360.toml",
    "nhl13-ps3.toml",
    "nhl12-ps3.toml",
    "nhl11-ps3.toml",
    "nhl10-ps3.toml",
];

/// Resolve `physical_snapshot = "docs/audits/data/schema/<platform>-<game>.json"` to the anchor it
/// was rendered from and parse that anchor. Fails loudly if the adapter names no snapshot or the
/// anchor is absent: an adapter whose physical facts cannot be checked is not verified.
fn anchor_snapshot(adapter: &Adapter) -> SchemaSnapshot {
    let rel = adapter
        .physical_snapshot
        .as_deref()
        .unwrap_or_else(|| panic!("{}: declares no physical_snapshot", adapter.target));
    let file = rel
        .rsplit('/')
        .next()
        .and_then(|f| f.strip_suffix(".json"))
        .unwrap_or_else(|| {
            panic!(
                "{}: snapshot path {rel:?} is not <platform>-<game>.json",
                adapter.target
            )
        });
    let (platform, game) = file.split_once('-').unwrap_or_else(|| {
        panic!(
            "{}: snapshot name {file:?} has no platform-game split",
            adapter.target
        )
    });
    let anchors = tdb_qa::discover_main_anchors(&repo_root()).expect("anchors/ is readable");
    let anchor = anchors
        .iter()
        .find(|a| a.platform == platform && a.game == game)
        .unwrap_or_else(|| {
            panic!(
                "{}: no committed main-DB anchor for {platform}/{game}",
                adapter.target
            )
        });
    let snap = snapshot(&repo_root(), anchor).unwrap_or_else(|e| {
        panic!(
            "{}: anchor {} did not parse: {e}",
            adapter.target, anchor.db_rel
        )
    });
    // The selected anchor is the one the snapshot says it was rendered from — by platform, game and
    // database file — so the identity the adapter is later held to is the anchor's, not a label's.
    assert_eq!(
        (snap.platform.as_str(), snap.game.as_str(), snap.db.as_str()),
        (platform, game, anchor.db_file.as_str()),
        "{}: snapshot identity is not the selected anchor's",
        adapter.target
    );
    assert_eq!(
        anchor.db_rel,
        format!("anchors/{platform}/{game}/databases/{}", anchor.db_file),
        "{}: the selected anchor does not live under its own platform/game",
        adapter.target
    );

    // The cited JSON must be the rendering of this very anchor, so a citation into it is a
    // citation into the bytes. CRLF is normalised only because a Windows checkout expands it.
    let committed = std::fs::read_to_string(repo_root().join(rel))
        .unwrap_or_else(|e| {
            panic!(
                "{}: cited snapshot {rel} is not readable: {e}",
                adapter.target
            )
        })
        .replace("\r\n", "\n");
    assert_eq!(
        committed,
        tdb_qa::schema::to_json(&snap),
        "{}: the cited snapshot {rel} is not the rendering of anchor {}",
        adapter.target,
        anchor.db_rel
    );
    snap
}

fn table<'a>(snap: &'a SchemaSnapshot, id: &str) -> Option<&'a TableSnapshot> {
    snap.tables.iter().find(|t| t.canonical_id == id)
}

fn field<'a>(table: &'a TableSnapshot, id: &str) -> Option<&'a FieldSnapshot> {
    table.fields.iter().find(|f| f.canonical_id == id)
}

/// Every disagreement between what the adapter restates and what the anchor holds, one line each.
/// Empty means every physical declaration is a measurement of this anchor.
fn discrepancies(adapter: &Adapter, snap: &SchemaSnapshot) -> Vec<String> {
    let mut out = Vec::new();
    let mut report = |s: String| out.push(s);

    // Identity before layout: the adapter must be attributed to the game and platform this snapshot
    // (and therefore the anchor it was rendered from) belongs to. A byte-identical database on
    // another platform agrees on every physical fact below and is still the wrong target.
    let snapshot_target = format!("{}-{}", snap.game, snap.platform);
    if adapter.target != snapshot_target {
        report(format!(
            "target: adapter {} vs snapshot {}/{} rendered from {}",
            adapter.target, snap.game, snap.platform, snap.db
        ));
    }
    if let Some(rel) = adapter.physical_snapshot.as_deref() {
        let expected = format!(
            "docs/audits/data/schema/{}-{}.json",
            snap.platform, snap.game
        );
        if rel != expected {
            report(format!(
                "physical_snapshot: adapter cites {rel} but the snapshot checked is {expected}"
            ));
        }
    }

    if adapter.endian != snap.endian {
        report(format!(
            "endian: adapter {} vs anchor {}",
            adapter.endian, snap.endian
        ));
    }
    if adapter.generation != snap.generation {
        report(format!(
            "generation: adapter {} vs anchor {}",
            adapter.generation, snap.generation
        ));
    }
    if let Some(v) = adapter.tdb_version {
        if v != snap.tdb_version {
            report(format!(
                "tdb_version: adapter {v} vs anchor {}",
                snap.tdb_version
            ));
        }
    }

    // Declared totals are the anchor's totals, not a neighbour's.
    match &adapter.coverage_declared {
        None => report("coverage_declared: absent".into()),
        Some(c) => {
            if c.tables_total as usize != snap.table_count {
                report(format!(
                    "coverage_declared.tables_total {} vs anchor {}",
                    c.tables_total, snap.table_count
                ));
            }
            if c.fields_total as usize != snap.field_total {
                report(format!(
                    "coverage_declared.fields_total {} vs anchor {}",
                    c.fields_total, snap.field_total
                ));
            }
        }
    }

    // Every declared table exists under that name.
    for decl in &adapter.tables {
        match table(snap, &decl.id) {
            None => report(format!("[[table]] {}: not in the anchor", decl.id)),
            Some(t) => {
                if t.name.as_deref() != Some(decl.name.as_str()) {
                    report(format!(
                        "[[table]] {}: name {:?} vs anchor {:?}",
                        decl.id, decl.name, t.name
                    ));
                }
            }
        }
    }

    // Every physical field a mapping restates: layout, value class, key/updatable flags.
    let mut mapped: Vec<(String, String)> = Vec::new();
    let check_field = |subject: String,
                       table_id: &str,
                       field_id: &str,
                       storage: Option<&StorageValue>,
                       status: Option<&StatusValue>,
                       transform_kind: Option<&str>,
                       mapped: &mut Vec<(String, String)>,
                       out: &mut Vec<String>| {
        mapped.push((table_id.to_string(), field_id.to_string()));
        let Some(t) = table(snap, table_id) else {
            out.push(format!("{subject}: table {table_id} not in the anchor"));
            return;
        };
        let Some(f) = field(t, field_id) else {
            out.push(format!(
                "{subject}: field {table_id}.{field_id} not in the anchor"
            ));
            return;
        };
        if let Some(s) = storage {
            for (what, declared, actual) in [
                ("bit_offset", s.bit_offset as i64, f.bit_offset as i64),
                ("bit_width", s.bit_width as i64, f.bit_width as i64),
                ("range_low", s.range_low, f.range_low.unwrap_or(0)),
                ("range_high", s.range_high, f.range_high.unwrap_or(0)),
            ] {
                if declared != actual {
                    out.push(format!(
                        "{subject}: storage.{what} {declared} vs anchor {actual}"
                    ));
                }
            }
            if s.endian != snap.endian {
                out.push(format!(
                    "{subject}: storage.endian {} vs anchor {}",
                    s.endian, snap.endian
                ));
            }
            let expected_kind_code = match s.kind.as_str() {
                "inline-string" => Some(0),
                "fixed-bit-uint" => Some(3),
                _ => None,
            };
            if let Some(code) = expected_kind_code {
                if f.kind_code != code {
                    out.push(format!(
                        "{subject}: storage.kind {} implies kind_code {code}, anchor has {}",
                        s.kind, f.kind_code
                    ));
                }
            }
            if s.kind == "inline-string" && f.meta_type.as_deref() != Some("string") {
                out.push(format!(
                    "{subject}: inline-string over a field whose metadata type is {:?}",
                    f.meta_type
                ));
            }
        }
        if let Some(k) = transform_kind {
            if k == "inline-string" && f.kind_code != 0 {
                out.push(format!(
                    "{subject}: transform inline-string but anchor kind_code {}",
                    f.kind_code
                ));
            }
        }
        if let Some(st) = status {
            if Some(st.key) != f.key {
                out.push(format!(
                    "{subject}: status.key {} vs anchor {:?}",
                    st.key, f.key
                ));
            }
            if Some(st.updatable) != f.updatable {
                out.push(format!(
                    "{subject}: status.updatable {} vs anchor {:?}",
                    st.updatable, f.updatable
                ));
            }
        }
    };

    for m in &adapter.mappings {
        let Some(table_id) = m.table.as_deref() else {
            continue;
        };
        let transform_kind = m.transform.established().map(|t| t.kind.as_str());
        if let Some(field_id) = m.field.as_deref() {
            check_field(
                format!("mapping {}", m.canonical),
                table_id,
                field_id,
                m.storage.established(),
                m.status.established(),
                transform_kind,
                &mut mapped,
                &mut out,
            );
        }
        for c in &m.components {
            check_field(
                format!("mapping {} component {}", m.canonical, c.role),
                table_id,
                &c.field,
                c.storage.established(),
                None,
                None,
                &mut mapped,
                &mut out,
            );
        }
    }

    // Mapped physical fields are counted once each, and the declared count is that count.
    let distinct: BTreeSet<&(String, String)> = mapped.iter().collect();
    if distinct.len() != mapped.len() {
        out.push(format!(
            "{} physical field references but only {} distinct",
            mapped.len(),
            distinct.len()
        ));
    }
    if let Some(c) = &adapter.coverage_declared {
        if c.fields_mapped as usize != distinct.len() {
            out.push(format!(
                "coverage_declared.fields_mapped {} vs {} distinct mapped fields",
                c.fields_mapped,
                distinct.len()
            ));
        }
    }

    // A capacity is checked against the file its `source` names, and ONLY that file. A
    // `main-db-max-records` declaration is held to this anchor's own directory (population, ceiling,
    // and ceiling >= population within it). A `roster-save-max-records` figure is a fact about a
    // save, not about the main DB: nothing about it — not its population, not its ceiling — is
    // compared with the main DB here (C0-REG1a-R2 removed the last such proxy). It is verified
    // against the decoded save by the target's own fixture case, when one exists; where none exists
    // (NHL 15, NHL 10) that verification is explicitly incomplete, not approximated.
    for cap in &adapter.capacities {
        let Some(v) = cap.declared.established() else {
            continue;
        };
        let Some(t) = table(snap, &cap.table) else {
            out.push(format!("capacity {}: table not in the anchor", cap.table));
            continue;
        };
        if v.source == "main-db-max-records" {
            if v.population_records != u32::from(t.current_records) {
                out.push(format!(
                    "capacity {}: population_records {} vs anchor current_records {}",
                    cap.table, v.population_records, t.current_records
                ));
            }
            if v.ceiling_records != u32::from(t.max_records) {
                out.push(format!(
                    "capacity {}: ceiling_records {} vs anchor max_records {}",
                    cap.table, v.ceiling_records, t.max_records
                ));
            }
            if t.max_records < t.current_records {
                out.push(format!(
                    "capacity {}: anchor max_records {} below its own current_records {}",
                    cap.table, t.max_records, t.current_records
                ));
            }
        }
    }
    out
}

// ================================================================== positive

#[test]
fn every_committed_adapter_agrees_with_its_anchor_snapshot() {
    for name in COMMITTED_ADAPTERS {
        let adapter = adapter(name);
        let snap = anchor_snapshot(&adapter);
        let found = discrepancies(&adapter, &snap);
        assert!(found.is_empty(), "{name}: {found:#?}");
    }
}

/// The Legacy adapter states Legacy's own facts. Where NHL 15's layout resembles it, the resemblance
/// is measured — and where it differs, the difference is what a copied value would have got wrong.
#[test]
fn the_legacy_adapter_states_its_own_facts_and_not_nhl15s() {
    let legacy = adapter("nhllegacy-xbox360.toml");
    let fifteen = adapter("nhl15-xbox360.toml");
    let find = |a: &Adapter, canonical: &str| {
        a.mappings
            .iter()
            .find(|m| m.canonical == canonical)
            .cloned()
            .unwrap_or_else(|| panic!("{} maps {canonical}", a.target))
    };
    let component_offset = |a: &Adapter, role: &str| {
        find(a, "player.birth_date")
            .components
            .iter()
            .find(|c| c.role == role)
            .and_then(|c| c.storage.established().map(|s| s.bit_offset))
            .unwrap_or_else(|| {
                panic!(
                    "{}: birth_date {role} component has established storage",
                    a.target
                )
            })
    };

    // 1. The physical totals are the target's own.
    assert_eq!(mapped_fraction(&legacy), Some((10, 2965)));
    assert_ne!(
        legacy.coverage_declared.as_ref().unwrap().fields_total,
        fifteen.coverage_declared.as_ref().unwrap().fields_total,
        "Legacy and NHL 15 have different field totals; a copied total would be wrong"
    );

    // 2. Birth-date component layout differs from NHL 15 on this target — a copied offset would fail
    //    `every_committed_adapter_agrees_with_its_anchor_snapshot`.
    assert_eq!(component_offset(&legacy, "day"), 958);
    assert_eq!(component_offset(&legacy, "month"), 975);
    assert_ne!(
        component_offset(&legacy, "day"),
        component_offset(&fifteen, "day")
    );
    assert_ne!(
        component_offset(&legacy, "month"),
        component_offset(&fifteen, "month")
    );

    // 3. The birth-date index origin is MEASURED here by a named case that lists this anchor by path.
    let birth = find(&legacy, "player.birth_date");
    assert_eq!(birth.qualification, "automated-tested");
    assert_eq!(birth.confidence, "measured");
    assert_eq!(birth.transform.established().unwrap().index_origin, Some(0));
    let case = birth.automated_case.as_deref().expect("names its case");
    let (file, function) = (
        "crates/studio-domain/tests/birthdate_encoding.rs",
        case.rsplit("::").next().unwrap(),
    );
    let source = std::fs::read_to_string(repo_root().join(file))
        .expect("the named case's file is committed");
    assert!(
        source.contains(&format!("fn {function}(")),
        "{file} defines {function}"
    );
    assert!(
        source.contains("anchors/xbox360/nhllegacy/databases/nhlng.db"),
        "{file} probes this adapter's anchor by path"
    );

    // 4. `team.name` is mapped here and on no neighbour, over a field the anchor names `fullname`.
    let team_name = find(&legacy, "team.name");
    assert_eq!(team_name.field.as_deref(), Some("YmkJ"));
    assert_eq!(
        team_name.confidence, "derived",
        "semantics are derived from the metadata name"
    );
    assert_eq!(
        team_name
            .string_storage
            .established()
            .unwrap()
            .byte_capacity,
        Some(50)
    );
    assert!(fifteen.mappings.iter().all(|m| m.canonical != "team.name"));

    // 5. What is NOT established stays blocked: no promotion by proximity. `player.shoots` has no
    //    independently labelled evidence on this anchor (legacy_xbox360_tests); `player.team` keeps
    //    its blocked qualification because the parent-table relationship is still a candidate, even
    //    though its encoding is now measured there.
    let shoots = find(&legacy, "player.shoots");
    assert_eq!(shoots.qualification, "blocked");
    assert!(transform_unresolved(&shoots));
    assert!(shoots.transform.established().is_none());
    assert!(
        shoots.transform.candidate.is_some(),
        "player.shoots: the proposal is recorded, not lost"
    );
    let team = find(&legacy, "player.team");
    assert_eq!(team.qualification, "blocked");
    assert!(
        !transform_unresolved(&team),
        "the encoding is measured on this anchor"
    );
    assert!(team.foreign_key.established().is_none());
    assert!(team.foreign_key.candidate.is_some());
    // The discriminator and the companion rule are measured on this anchor by named cases
    // (C0-REG1a-R1); the neighbours' remain candidates — see tests.rs.
    assert!(legacy
        .position_discriminator
        .as_ref()
        .unwrap()
        .established()
        .is_some());
    assert!(legacy
        .companion_rules
        .iter()
        .all(|r| r.declared.established().is_some()));

    // 6. The ratings are derived, not measured, because the named case ran on another anchor.
    for canonical in ["player.rating_overall", "player.rating_speed"] {
        let m = find(&legacy, canonical);
        assert_eq!(m.confidence, "derived", "{canonical}");
        assert_eq!(m.qualification, "implemented", "{canonical}");
        assert_eq!(human_range(&m), Some([36, 99]), "{canonical}");
    }

    // 7. Legacy's capacities are read from Legacy's own ROSTER fixture (legacy_xbox360_tests holds
    //    the byte-level check); the file also validates clean against the contract.
    let ceiling = |t: &str| {
        legacy
            .capacities
            .iter()
            .find(|c| c.table == t)
            .and_then(|c| c.declared.established())
            .map(|v| (v.ceiling_records, v.population_records))
    };
    assert_eq!(ceiling("ubPc"), Some((7988, 5632)));
    assert_eq!(ceiling("eGlu"), Some((10879, 5532)));
    assert_eq!(
        ceiling("kOtt"),
        Some((252, 252)),
        "the team table's directory is packed in the fixture too: an observed capacity, no headroom"
    );
    assert!(legacy
        .capacities
        .iter()
        .all(|c| c.declared.confidence.as_deref() == Some("measured")));
    assert_eq!(validate_adapter(&legacy, &schema()), vec![]);
}

#[test]
fn registry_population_counts_the_legacy_adapter_apart() {
    let adapters: Vec<Adapter> = COMMITTED_ADAPTERS.iter().map(|n| adapter(n)).collect();
    let population = registry_population(adapters.iter());
    assert_eq!(population.len(), COMMITTED_ADAPTERS.len());
    let legacy = population
        .iter()
        .find(|p| p.target == "nhllegacy-xbox360")
        .expect("counted");
    assert_eq!(legacy.mappings, 8);
    assert_eq!(legacy.omissions, 0);
    assert!(legacy.established_constraints > 0);
    assert!(
        legacy.unknown_constraints > 0,
        "the honest state still carries unknowns"
    );
    assert!(legacy.candidate_constraints > 0);
    assert!(legacy.candidate_constraints <= legacy.unknown_constraints);
    // Coverage is stated apart from the block tally: 10 of 2965 physical fields, and nothing more.
    assert_eq!(
        mapped_fraction(&adapter("nhllegacy-xbox360.toml")),
        Some((10, 2965))
    );
}

// ================================================================== negative

/// The cross-check is load-bearing: a declaration that disagrees with the anchor is reported, by
/// subject, and the un-mutated file reports nothing. Each mutation is one plausible copying error.
#[test]
fn a_declaration_that_disagrees_with_the_anchor_is_reported() {
    let base = adapter("nhllegacy-xbox360.toml");
    let snap = anchor_snapshot(&base);
    assert!(discrepancies(&base, &snap).is_empty());

    let mapping_index = |canonical: &str| {
        base.mappings
            .iter()
            .position(|m| m.canonical == canonical)
            .unwrap()
    };

    // 1. A storage offset copied from a neighbour (NHL 15's day offset is 959; Legacy's is 958).
    let mut a = base.clone();
    let i = mapping_index("player.birth_date");
    let day = a.mappings[i]
        .components
        .iter_mut()
        .find(|c| c.role == "day")
        .unwrap();
    day.storage.value.as_mut().unwrap().bit_offset = 959;
    let found = discrepancies(&a, &snap);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("component day") && found[0].contains("bit_offset 959 vs anchor 958"),
        "{found:?}"
    );

    // 2. A field total copied from a neighbour.
    let mut a = base.clone();
    a.coverage_declared.as_mut().unwrap().fields_total = 2969;
    let found = discrepancies(&a, &snap);
    assert!(
        found
            .iter()
            .any(|f| f.contains("fields_total 2969 vs anchor 2965")),
        "{found:?}"
    );

    // 3. A mapped-field count that does not match the distinct fields actually mapped.
    let mut a = base.clone();
    a.coverage_declared.as_mut().unwrap().fields_mapped = 11;
    let found = discrepancies(&a, &snap);
    assert!(
        found
            .iter()
            .any(|f| f.contains("fields_mapped 11 vs 10 distinct")),
        "{found:?}"
    );

    // 4. The same physical field mapped twice would inflate the count without adding coverage.
    let mut a = base.clone();
    let mut dup = a.mappings[mapping_index("player.first_name")].clone();
    dup.canonical = "player.last_name".into();
    a.mappings.push(dup);
    let found = discrepancies(&a, &snap);
    assert!(
        found.iter().any(|f| f.contains("only 10 distinct")),
        "{found:?}"
    );

    // 5. A table declared under a name the anchor does not give it.
    let mut a = base.clone();
    a.tables.iter_mut().find(|t| t.id == "kOtt").unwrap().name = "teams".into();
    let found = discrepancies(&a, &snap);
    assert!(
        found.iter().any(|f| f.starts_with("[[table]] kOtt: name")),
        "{found:?}"
    );

    // 6. A key/updatable flag that contradicts the anchor's metadata.
    let mut a = base.clone();
    let i = mapping_index("player.first_name");
    a.mappings[i].status.value.as_mut().unwrap().key = true;
    let found = discrepancies(&a, &snap);
    assert!(
        found
            .iter()
            .any(|f| f.contains("status.key true vs anchor Some(false)")),
        "{found:?}"
    );

    // 7. A roster-save capacity is NOT held to the main DB in any way. A save whose directory is
    //    smaller than a different file's population is a legitimate declaration (synthetic: a
    //    300-slot directory holding 250 rows against a main DB of 5632) and must not be reported
    //    here for that reason alone — the save-specific check is what holds it to its own fixture
    //    (`legacy_xbox360_tests::a_capacity_that_is_not_the_fixtures_is_reported`).
    let mut a = base.clone();
    let cap = a.capacities.iter_mut().find(|c| c.table == "ubPc").unwrap();
    let v = cap.declared.value.as_mut().unwrap();
    v.ceiling_records = 300;
    v.population_records = 250;
    assert_eq!(v.source, "roster-save-max-records");
    assert_eq!(
        discrepancies(&a, &snap),
        Vec::<String>::new(),
        "a smaller save directory is not a disagreement with the main DB"
    );
    assert_eq!(
        validate_adapter(&a, &schema()),
        vec![],
        "and the contract accepts it"
    );

    // 7b. A capacity attributed to the main DB itself is held to the main DB's directory.
    let mut a = base.clone();
    let cap = a.capacities.iter_mut().find(|c| c.table == "ubPc").unwrap();
    let v = cap.declared.value.as_mut().unwrap();
    v.source = "main-db-max-records".into();
    v.population_records = 5534; // NHL 15's figure
    let found = discrepancies(&a, &snap);
    assert!(
        found
            .iter()
            .any(|f| f.contains("population_records 5534 vs anchor current_records 5632")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("ceiling_records 7988 vs anchor max_records 5632")),
        "{found:?}"
    );

    // 8. An inline-string transform over a field whose value class is not a string.
    let mut a = base.clone();
    let i = mapping_index("player.first_name");
    a.mappings[i].field = Some("GRkp".into());
    let found = discrepancies(&a, &snap);
    assert!(
        found
            .iter()
            .any(|f| f.contains("kind_code 0, anchor has 3")),
        "{found:?}"
    );

    // 9. A field that simply is not in the anchor.
    let mut a = base.clone();
    let i = mapping_index("player.rating_speed");
    a.mappings[i].field = Some("zzzz".into());
    let found = discrepancies(&a, &snap);
    assert!(
        found
            .iter()
            .any(|f| f.contains("field dSvy.zzzz not in the anchor")),
        "{found:?}"
    );
}

/// The identity guard is load-bearing on its own: an adapter whose every physical declaration agrees
/// with the anchor is still refused when it is attributed to a target the anchor does not belong to.
#[test]
fn an_adapter_attributed_to_the_wrong_target_is_reported_even_when_the_layout_agrees() {
    let base = adapter("nhllegacy-xbox360.toml");
    let snap = anchor_snapshot(&base);
    assert!(discrepancies(&base, &snap).is_empty());
    let identity_findings = |found: &[String]| {
        found
            .iter()
            .filter(|f| f.starts_with("target:") || f.starts_with("physical_snapshot:"))
            .count()
    };

    // 1. Same title, other platform. The PS3 and Xbox 360 Legacy `nhlng.db` are byte-identical, so
    //    nothing physical disagrees; the only finding is the attribution.
    let mut a = base.clone();
    a.target = "nhllegacy-ps3".into();
    let found = discrepancies(&a, &snap);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0],
        "target: adapter nhllegacy-ps3 vs snapshot nhllegacy/xbox360 rendered from nhlng.db"
    );

    // 2. The mirror image: the adapter keeps its Xbox 360 target but cites the PS3 rendering. The
    //    PS3 anchor is selected, parses, and agrees with every physical declaration — and the
    //    attribution still fails, because the anchor checked is not the target named.
    let mut a = base.clone();
    a.physical_snapshot = Some("docs/audits/data/schema/ps3-nhllegacy.json".into());
    let ps3 = anchor_snapshot(&a);
    assert_eq!(
        (ps3.platform.as_str(), ps3.game.as_str()),
        ("ps3", "nhllegacy")
    );
    let found = discrepancies(&a, &ps3);
    assert_eq!(identity_findings(&found), 1, "{found:?}");
    assert_eq!(
        found.len(),
        1,
        "no physical disagreement — identity is the whole finding: {found:?}"
    );
    assert_eq!(
        found[0],
        "target: adapter nhllegacy-xbox360 vs snapshot nhllegacy/ps3 rendered from nhlng.db"
    );

    // 3. A different title on the same platform: identity AND layout disagree, and both are said.
    let mut a = base.clone();
    a.target = "nhl15-xbox360".into();
    let found = discrepancies(&a, &snap);
    assert_eq!(identity_findings(&found), 1, "{found:?}");
    let mut a = base.clone();
    a.target = "nhl15-xbox360".into();
    a.physical_snapshot = Some("docs/audits/data/schema/xbox360-nhl15.json".into());
    let fifteen = anchor_snapshot(&a);
    let found = discrepancies(&a, &fifteen);
    assert_eq!(
        identity_findings(&found),
        0,
        "target and snapshot agree with each other; the layout does not: {found:?}"
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("fields_total 2965 vs anchor 2969")),
        "{found:?}"
    );

    // 4. A snapshot path that is not the one checked is reported even when the target matches.
    let mut a = base.clone();
    a.physical_snapshot = Some("docs/audits/data/schema/ps3-nhllegacy.json".into());
    let found = discrepancies(&a, &snap);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0],
        "physical_snapshot: adapter cites docs/audits/data/schema/ps3-nhllegacy.json but the \
         snapshot checked is docs/audits/data/schema/xbox360-nhllegacy.json"
    );
}

/// An adapter that names no snapshot cannot be checked and must not pass as verified.
#[test]
#[should_panic(expected = "declares no physical_snapshot")]
fn an_adapter_without_a_snapshot_cannot_be_checked() {
    let mut a = adapter("nhllegacy-xbox360.toml");
    a.physical_snapshot = None;
    let _ = anchor_snapshot(&a);
}
