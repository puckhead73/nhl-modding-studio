//! Package C0-AS6: asset-family coverage reported over the seven asset-bearing families (not the
//! fourteen matrix families), the equipment profile's representation, and the anchor measurements
//! the portrait and equipment identity candidates rest on — plus the cross-anchor record of every
//! database field an asset family references, drift-guarded against the committed anchors.
//!
//! Nothing here establishes a block. A measurement of database bytes is the DB half of an identity;
//! the file half — an id placed beside a hashed file in an attributable corpus — is in no committed
//! record, and every case below says so where it matters.

use super::*;
use crate::target_evidence::{meta_field, read, repo_root, sha256_hex, table};
use std::collections::{BTreeMap, BTreeSet};
use tdb_core::{FieldDescriptor, Table, TdbFile};

fn profile(file: &str) -> AssetProfile {
    let text = std::fs::read_to_string(repo_root().join("defs/adapters/asset-profiles").join(file))
        .unwrap();
    parse_asset_profile(&text).unwrap_or_else(|e| panic!("{file}: {e}"))
}

fn target<'a>(p: &'a AssetProfile, id: &str) -> &'a TargetProfile {
    p.targets.iter().find(|t| t.target == id).unwrap()
}

/// Every committed profile, in directory order.
fn committed_profiles() -> Vec<AssetProfile> {
    let dir = repo_root().join("defs/adapters/asset-profiles");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|p| {
            let text = std::fs::read_to_string(p).unwrap();
            parse_asset_profile(&text).unwrap_or_else(|e| panic!("{p:?}: {e}"))
        })
        .collect()
}

/// A committed anchor's bytes and their SHA-256; callers parse the bytes they hold.
fn anchor_bytes(rel: &str) -> (Vec<u8>, String) {
    let bytes = std::fs::read(repo_root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
    let digest = sha256_hex(&bytes);
    (bytes, digest)
}

/// Stored values of one field over every current record: value -> row count.
fn histogram(db: &TdbFile, t: &Table, f: &FieldDescriptor) -> BTreeMap<u64, usize> {
    let mut out = BTreeMap::new();
    for i in 0..u32::from(t.current_records) {
        *out.entry(read(db, t, i, f)).or_default() += 1;
    }
    out
}

fn find_field(db: &TdbFile, t: &Table, id: &str) -> Option<FieldDescriptor> {
    t.fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == id)
        .cloned()
}

/// `[[entries]].raw` of a committed `defs/enums/*.toml`.
fn enum_raws(file: &str) -> BTreeSet<u64> {
    let text = std::fs::read_to_string(repo_root().join("defs/enums").join(file)).unwrap();
    let v: toml::Value = toml::from_str(&text).unwrap();
    v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| u64::try_from(e["raw"].as_integer().unwrap()).unwrap())
        .collect()
}

// ============================================================ the family inventory (task A)

#[test]
fn the_family_vocabulary_partitions_into_seven_asset_bearing_and_seven_workflow_families() {
    // Every tracked family has exactly one category, and the categories partition the axis.
    let categorised: Vec<&str> = FAMILY_CATEGORY.iter().map(|(f, _)| *f).collect();
    assert_eq!(categorised, ASSET_FAMILY);
    let matrix: Vec<&str> = tdb_qa::catalog::FAMILIES.iter().map(|f| f.id).collect();
    assert_eq!(
        categorised, matrix,
        "the categories cover the QA matrix axis exactly"
    );

    let mut by_category: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (family, category) in FAMILY_CATEGORY {
        by_category.entry(category).or_default().push(family);
    }
    assert_eq!(
        by_category,
        BTreeMap::from([
            (
                "asset",
                vec![
                    "assets-portraits",
                    "assets-logos",
                    "assets-jerseys",
                    "assets-equipment",
                    "assets-heads",
                    "assets-arenas",
                    "assets-audio",
                ]
            ),
            ("database", vec!["db-presentation", "db-operations"]),
            (
                "onboarding",
                vec!["onboarding-install", "onboarding-localization"]
            ),
            (
                "save",
                vec!["save-raw-db", "save-inner-container", "save-outer-package"]
            ),
        ])
    );
    assert_eq!(asset_bearing_families(), by_category["asset"]);
    assert_eq!(asset_bearing_families().len(), 7);
    assert_eq!(family_category("assets-heads"), Some("asset"));
    assert_eq!(family_category("save-raw-db"), Some("save"));
    assert_eq!(family_category("assets-fixture"), None);
    // The naming convention is not what decides it - the table is - but the two agree today.
    for (family, category) in FAMILY_CATEGORY {
        assert_eq!(
            family.starts_with("assets-"),
            *category == ASSET_BEARING_CATEGORY,
            "{family}"
        );
    }
}

#[test]
fn a_profile_for_a_database_save_or_onboarding_family_is_refused() {
    // The committed portrait profile with its family swapped for each workflow category. Nothing
    // else changes, so the only new finding is the category refusal.
    let text = std::fs::read_to_string(
        repo_root().join("defs/adapters/asset-profiles/assets-portraits.toml"),
    )
    .unwrap();
    assert!(validate_asset_profile(&parse_asset_profile(&text).unwrap()).is_empty());
    for family in ASSET_FAMILY {
        let swapped = text.replacen(
            "family = \"assets-portraits\"",
            &format!("family = \"{family}\""),
            1,
        );
        let profile = parse_asset_profile(&swapped).unwrap();
        let findings = validate_asset_profile(&profile);
        let refused = findings
            .iter()
            .filter(|f| f.rule == Rule::AssetFamilyNotAssetBearing)
            .count();
        if family_category(family) == Some(ASSET_BEARING_CATEGORY) {
            assert_eq!(
                refused, 0,
                "{family}: an asset-bearing family is not refused"
            );
        } else {
            assert_eq!(refused, 1, "{family}: {findings:?}");
            let finding = findings
                .iter()
                .find(|f| f.rule == Rule::AssetFamilyNotAssetBearing)
                .unwrap();
            assert_eq!(finding.subject, *family);
            assert!(finding.detail.contains(family_category(family).unwrap()));
            assert!(finding.detail.contains("7 asset-bearing"));
            // Refused, and therefore never counted as coverage of anything.
            assert!(asset_family_coverage(std::iter::once(&profile))
                .iter()
                .all(|c| c.status == FamilyCoverageStatus::Absent));
        }
    }
    // A family the matrix does not track is refused by the existing rule, not this one.
    let unknown = text.replacen(
        "family = \"assets-portraits\"",
        "family = \"assets-nothing\"",
        1,
    );
    let findings = validate_asset_profile(&parse_asset_profile(&unknown).unwrap());
    assert!(findings.iter().any(|f| f.rule == Rule::AssetFamilyUnknown));
    assert!(!findings
        .iter()
        .any(|f| f.rule == Rule::AssetFamilyNotAssetBearing));
}

#[test]
fn asset_family_coverage_reports_the_committed_registry_over_seven_families() {
    let profiles = committed_profiles();
    assert_eq!(profiles.len(), 4);
    for p in &profiles {
        assert!(!p.synthetic);
        assert_eq!(family_category(&p.family), Some(ASSET_BEARING_CATEGORY));
        assert_eq!(validate_asset_profile(p), vec![], "{}", p.family);
    }
    let coverage = asset_family_coverage(profiles.iter());
    assert_eq!(
        coverage
            .iter()
            .map(|c| (c.family.as_str(), c.status))
            .collect::<Vec<_>>(),
        [
            ("assets-portraits", FamilyCoverageStatus::Populated),
            ("assets-logos", FamilyCoverageStatus::Represented),
            ("assets-jerseys", FamilyCoverageStatus::Represented),
            ("assets-equipment", FamilyCoverageStatus::Represented),
            ("assets-heads", FamilyCoverageStatus::Absent),
            ("assets-arenas", FamilyCoverageStatus::Absent),
            ("assets-audio", FamilyCoverageStatus::Absent),
        ],
        "one populated, three represented, three absent - of SEVEN, never of fourteen"
    );
    assert!(coverage
        .iter()
        .all(|c| family_category(&c.family) == Some(ASSET_BEARING_CATEGORY)));

    let portraits = &coverage[0];
    assert_eq!(
        portraits.declared_targets,
        ["nhllegacy-ps3", "nhllegacy-xbox360", "nhl21-ps4"]
    );
    assert_eq!(portraits.by_reference_targets, ["legacyrecomp-recomp"]);
    // The population report's own figures: 3 target-level + 4 variant-level operative on PS3
    // (C0-POR1-R1 / C0-AS3), nothing operative anywhere else.
    assert_eq!(portraits.operative_constraints, 7);
    assert!(portraits.candidate_constraints > 0);

    for c in &coverage[1..4] {
        assert_eq!(c.operative_constraints, 0, "{}", c.family);
        assert_eq!(c.specimen_observations, 0, "{}", c.family);
        assert!(c.candidate_constraints > 0, "{}", c.family);
        assert_eq!(
            c.by_reference_targets,
            ["legacyrecomp-recomp"],
            "{}",
            c.family
        );
        assert!(
            c.declared_targets
                .contains(&"nhllegacy-xbox360".to_string()),
            "{}",
            c.family
        );
    }
    assert_eq!(
        coverage[3].declared_targets,
        ["nhllegacy-xbox360", "nhllegacy-ps3"]
    );
    for c in &coverage[4..] {
        assert_eq!(
            (
                c.declared_targets.len(),
                c.by_reference_targets.len(),
                c.operative_constraints,
                c.candidate_constraints
            ),
            (0, 0, 0, 0),
            "{}",
            c.family
        );
    }
    // Status ordering is what the words mean.
    assert!(FamilyCoverageStatus::Absent < FamilyCoverageStatus::Represented);
    assert!(FamilyCoverageStatus::Represented < FamilyCoverageStatus::Populated);
    assert_eq!(FamilyCoverageStatus::Populated.as_str(), "populated");

    // The synthetic fixture (family `assets-arenas`) is excluded, so arenas stay absent with it.
    let synthetic = parse_asset_profile(
        &std::fs::read_to_string(
            repo_root().join("crates/tdb-canonical/tests/fixtures/synthetic-asset-profile.toml"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(synthetic.synthetic);
    let with_fixture = asset_family_coverage(profiles.iter().chain(std::iter::once(&synthetic)));
    assert_eq!(with_fixture, coverage);
}

// ====================================================== the equipment profile (tasks B, C)

#[test]
fn the_equipment_profile_represents_two_legacy_targets_and_establishes_nothing() {
    let p = profile("assets-equipment.toml");
    assert_eq!(p.family, "assets-equipment");
    assert_eq!(p.schema, ASSET_PROFILE_SCHEMA);
    assert!(!p.synthetic);
    assert_eq!(validate_asset_profile(&p), vec![]);
    assert!(p.note.contains("Nothing on any target is established"));
    assert_eq!(
        p.targets
            .iter()
            .map(|t| t.target.as_str())
            .collect::<Vec<_>>(),
        ["nhllegacy-xbox360", "nhllegacy-ps3", "legacyrecomp-recomp"]
    );
    assert_eq!(p.canonical_sources.len(), 6);
    assert!(p.canonical_sources.iter().all(|s| s.class == "raster"));

    let population = asset_registry_population(std::iter::once(&p));
    assert_eq!(population.len(), 3);
    for row in &population {
        assert_eq!(
            (
                row.established_constraints,
                row.operative_constraints,
                row.specimen_observations,
                row.target_required_constraints,
                row.established_variant_constraints,
                row.operative_variant_constraints,
                row.operative_resolution_chains,
            ),
            (0, 0, 0, 0, 0, 0, 0),
            "{}: nothing established",
            row.target
        );
        assert_eq!(
            row.unresolved_path_families, row.declared_path_families,
            "{}: no rendered path is placed beside a file",
            row.target
        );
        assert_eq!(row.declared_operations, 6, "{}", row.target);
    }
    let x360 = &population[0];
    assert_eq!(x360.reused_from, None);
    assert_eq!(
        (
            x360.declared_path_families,
            x360.declared_variants,
            x360.declared_corpora,
            x360.unattributable_corpora,
            x360.candidate_constraints
        ),
        (8, 3, 2, 1, 2),
        "eight spellable grammars from HEAD source (the helmet mapped shape and the blocker/trapper \
         shapes carry two independent components beside the id and stay in prose), three forms, \
         the two Legacy corpora, discovery and compression proposals"
    );
    let ps3 = &population[1];
    assert_eq!(ps3.reused_from, None);
    assert_eq!(
        (
            ps3.declared_path_families,
            ps3.declared_variants,
            ps3.declared_corpora,
            ps3.unattributable_corpora,
            ps3.candidate_constraints
        ),
        (2, 2, 1, 0, 1),
        "the stick slot only: the one PS3 record names no other file"
    );
    let recomp = &population[2];
    assert_eq!(recomp.reused_from.as_deref(), Some("nhllegacy-xbox360"));
    assert_eq!(recomp.declared_path_families, x360.declared_path_families);

    // The spellable slot grammars and the stick thumbnail, by id. No template substitutes one
    // token twice for two different components: `render_path` replaces every occurrence of a
    // token with one value, so `helmet_0_{id}_{map}` or `blocker_{model}_{id}_{map}` cannot be a
    // template and are prose (C0-AS6 review finding P2).
    let t = target(&p, "nhllegacy-xbox360");
    let ids: Vec<&str> = t.path_families.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "stick-texlib",
            "skate-texlib",
            "helmet-texlib",
            "glove-map",
            "goalie-stick-map",
            "goaliepad-map",
            "goalie-helmet-paint",
            "fe-thumbnail-stick",
        ]
    );
    for f in &t.path_families {
        for token in &f.tokens {
            assert_eq!(
                f.template.matches(&format!("{{{token}}}")).count(),
                1,
                "{}: a token appears once, so every cited filename is reachable",
                f.id
            );
        }
    }
    assert!(t.path_families.iter().all(|f| f.tokens.len() <= 2));
    for absent in ["helmet-map", "blocker-map", "trapper-map"] {
        assert!(!ids.contains(&absent), "{absent} is prose, not a template");
    }
    assert!(t
        .path_families
        .iter()
        .all(|f| f.template.starts_with("rendering/") || f.template.starts_with("fe/ion/")));
    assert!(t
        .path_families
        .iter()
        .all(|f| f.tokens.contains(&"numeric-id".to_string())));

    // [identity] is unknown WITHOUT a candidate value on both declared targets: one block cannot
    // carry eight slot sources, and the note says so instead of a single `source` string.
    for name in ["nhllegacy-xbox360", "nhllegacy-ps3"] {
        let t = target(&p, name);
        assert_eq!(t.identity.state, ConstraintState::Unknown, "{name}");
        assert!(t.identity.candidate.is_none(), "{name}");
        assert!(t
            .identity
            .provenance
            .iter()
            .any(|p| p.ends_with("#legacy_equipment_id_fields_are_measured_on_the_anchor")));
        for s in [
            "wXFx", "NmUR", "YUZp", "AOOe", "Kmxz", "OTJR", "mIUA", "JOlt",
        ] {
            assert!(
                t.identity.note.contains(s) || name == "nhllegacy-ps3",
                "{name}: {s}"
            );
        }
        assert!(t.install.candidate.is_none(), "{name}");
        assert!(t.runtime_lookup.candidate.is_none(), "{name}");
        // Every block unknown, at target level and in every variant.
        for entry in block_states(t, None) {
            assert_eq!(
                entry.state,
                ConstraintState::Unknown,
                "{name}: {}",
                entry.block
            );
        }
        for v in &t.variants {
            for entry in block_states(t, Some(v)) {
                assert_eq!(
                    entry.state,
                    ConstraintState::Unknown,
                    "{name}/{}: {}",
                    v.id,
                    entry.block
                );
            }
        }
        // Every operation blocked.
        for op in &t.operations {
            assert!(
                matches!(
                    operation_constraint_readiness(t, None, op),
                    OperationReadiness::BlockedByUnknownConstraints(_)
                ),
                "{name}: {}",
                op.operation
            );
        }
    }
    // The PS3 record's on-ice binding is carried in prose only, on the block that would own it.
    let ps3 = target(&p, "nhllegacy-ps3");
    assert!(ps3.runtime_lookup.note.contains("texlib_N.rpsgl"));
    assert!(ps3.runtime_lookup.note.contains("No value is proposed"));
    assert_eq!(ps3.corpora[0].kind, "observed-installed-copy");
    assert_eq!(ps3.corpora[0].inspected_files, 180);
    assert!(ps3.corpora[0].identity_source.starts_with("NOT READ"));
    let discovery = ps3.discovery.candidate.as_ref().unwrap();
    assert_eq!(discovery.confidence.as_deref(), Some("asserted"));
    assert_eq!(
        discovery.provenance,
        ["defs/enums/stick-model.toml".to_string()]
    );

    // Recompiled resolves by reference (task C) and keeps its destination blocks its own.
    let resolved = crate::asset_reuse::resolve_target(&p, "legacyrecomp-recomp").unwrap();
    assert_eq!(resolved.reused_from.as_deref(), Some("nhllegacy-xbox360"));
    assert_eq!(resolved.profile.path_families.len(), 8);
    assert_eq!(resolved.profile.variants.len(), 3);
    let declared = target(&p, "legacyrecomp-recomp");
    assert!(declared.path_families.is_empty() && declared.corpora.is_empty());
    let destination: [(ConstraintState, bool, &str); 3] = [
        (
            declared.discovery.state,
            declared.discovery.candidate.is_some(),
            &declared.discovery.note,
        ),
        (
            declared.install.state,
            declared.install.candidate.is_some(),
            &declared.install.note,
        ),
        (
            declared.runtime_lookup.state,
            declared.runtime_lookup.candidate.is_some(),
            &declared.runtime_lookup.note,
        ),
    ];
    for (state, has_candidate, note) in destination {
        assert_eq!(state, ConstraintState::Unknown);
        assert!(!has_candidate);
        assert!(note.contains("Not inherited"));
    }
}

// =================================================== the anchor measurements (task B)

/// The database half of the equipment identity: eight fields on the committed Legacy anchor.
/// The PS3 anchor is the same bytes, so one measurement speaks for both platforms' DB side — and
/// for nothing on the file side.
#[test]
fn legacy_equipment_id_fields_are_measured_on_the_anchor() {
    let (bytes, x360_digest) = anchor_bytes("anchors/xbox360/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let (_, ps3_digest) = anchor_bytes("anchors/ps3/nhllegacy/databases/nhlng.db");
    assert_eq!(
        x360_digest, ps3_digest,
        "the two Legacy main anchors are byte-identical (REG2's twin observation, re-asserted here)"
    );
    assert_eq!(
        x360_digest,
        "105d2186da3d1f0f3d3ea8a350ead42ac9d5d802ca9fd37652c39bb4e1f1c230"
    );
    let anchors = tdb_qa::discover_main_anchors(&repo_root()).unwrap();
    let anchor = anchors
        .iter()
        .find(|a| a.platform == "xbox360" && a.game == "nhllegacy")
        .unwrap();
    let snap = tdb_qa::schema::snapshot(&repo_root(), anchor).unwrap();

    /// (table, field, metadata name, bits, HEAD `SLOT_SPECS.max_id` transcribed from
    /// crates/studio-domain/src/equipment_slots.rs at cc913f9, distinct, lo, hi, zero rows)
    type SlotField = (
        &'static str,
        &'static str,
        &'static str,
        u32,
        u32,
        usize,
        u64,
        u64,
        usize,
    );
    let expected: [SlotField; 8] = [
        ("xmja", "wXFx", "stick", 7, 127, 19, 35, 112, 0),
        ("xmja", "NmUR", "gloves", 7, 127, 14, 27, 83, 0),
        ("xmja", "YUZp", "helmetbrand", 8, 255, 10, 1, 25, 0),
        ("xmja", "AOOe", "skatebrand", 7, 127, 8, 39, 48, 0),
        ("fMVl", "Kmxz", "blocker", 7, 127, 29, 11, 82, 0),
        ("fMVl", "OTJR", "trapper", 7, 127, 26, 11, 82, 0),
        ("fMVl", "mIUA", "pads", 7, 127, 29, 11, 82, 0),
        ("fMVl", "JOlt", "maskid", 9, 511, 173, 0, 290, 36),
    ];
    assert_eq!(table(&db, "xmja").current_records, 5066);
    assert_eq!(table(&db, "fMVl").current_records, 566);
    let mut stored: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    for (t, f, name, bits, max_id, distinct, lo, hi, zeros) in expected {
        let tb = table(&db, t);
        let fd = find_field(&db, tb, f).unwrap_or_else(|| panic!("{t}.{f}"));
        assert_eq!(
            meta_field(&snap, t, f).name.as_deref(),
            Some(name),
            "{t}.{f}"
        );
        assert_eq!(fd.bit_width, bits, "{t}.{f}");
        // The HEAD constant equals the anchor's width on Legacy. That is a Legacy fact carried as
        // code: the cross-anchor record below shows 8 bits (stick) and 4-5 bits (the other present
        // slots) on NHL 08, and 10 on NHL 21 PS4.
        assert_eq!((1u32 << bits) - 1, max_id, "{t}.{f}");
        let h = histogram(&db, tb, &fd);
        let all_ones = (1u64 << bits) - 1;
        assert_eq!(h.get(&all_ones), None, "{t}.{f}: no all-ones pattern");
        assert_eq!(h.get(&0).copied().unwrap_or(0), zeros, "{t}.{f}: zero rows");
        assert_eq!(h.len(), distinct, "{t}.{f}: distinct");
        assert_eq!(
            (*h.keys().next().unwrap(), *h.keys().next_back().unwrap()),
            (lo, hi),
            "{t}.{f}: range"
        );
        stored.insert(f, h.keys().copied().collect());
    }
    assert_eq!(
        stored["wXFx"],
        BTreeSet::from([
            35, 82, 84, 85, 86, 87, 95, 98, 99, 102, 103, 104, 105, 107, 108, 109, 110, 111, 112
        ])
    );
    // Consistency between two committed records, not a file observation: every id the vanilla
    // roster stores is one the operator catalogues name (the stick catalogue enumerates the 74
    // ids its record says have a `texlib_N.rpsgl`).
    for (field, file, entries) in [
        ("wXFx", "stick-model.toml", 74),
        ("NmUR", "glove-model.toml", 53),
        ("YUZp", "helmet-model.toml", 20),
        ("AOOe", "skate-model.toml", 31),
    ] {
        let raws = enum_raws(file);
        assert_eq!(raws.len(), entries, "{file}");
        let missing: Vec<&u64> = stored[field].difference(&raws).collect();
        assert!(
            missing.is_empty(),
            "{file}: anchor ids not catalogued: {missing:?}"
        );
    }

    // The profile cites this case on both declared targets, and its figures.
    let p = profile("assets-equipment.toml");
    let note = &target(&p, "nhllegacy-xbox360").identity.note;
    for s in [
        "5 066 rows",
        "566 rows",
        "19 distinct",
        "173 distinct",
        "36 rows store 0",
        "127 / 255 / 511",
        "74 ids",
    ] {
        assert!(note.contains(s), "{s}");
    }
    assert!(target(&p, "nhllegacy-ps3")
        .identity
        .note
        .contains("byte-identical"));
}

/// The database half of the portrait identity on all three declared targets: `ubPc.lOnr` on the
/// Legacy anchor (both platforms, one set of bytes) and on the NHL 21 PS4 anchor, set against the
/// bucket RANGES the C0-POR1 census published. No id is placed beside a file here — the census
/// holds ranges, not ids — which is why every candidate this case backs stays a candidate.
#[test]
fn legacy_portrait_id_field_is_measured_on_the_anchor() {
    let (legacy_bytes, legacy_digest) = anchor_bytes("anchors/ps3/nhllegacy/databases/nhlng.db");
    let legacy = TdbFile::parse(&legacy_bytes).unwrap();
    let (_, x360_digest) = anchor_bytes("anchors/xbox360/nhllegacy/databases/nhlng.db");
    assert_eq!(legacy_digest, x360_digest);
    let (ps4_bytes, ps4_digest) = anchor_bytes("anchors/ps4/nhl21/databases/nhl_game.db");
    let ps4 = TdbFile::parse(&ps4_bytes).unwrap();
    assert_eq!(
        ps4_digest,
        "90ee1ec49d36dbb0f6d9cefe75e259d9661e3aa05338d03347b4842e33c8e9f9"
    );
    let anchors = tdb_qa::discover_main_anchors(&repo_root()).unwrap();
    for (platform, game) in [
        ("ps3", "nhllegacy"),
        ("xbox360", "nhllegacy"),
        ("ps4", "nhl21"),
    ] {
        let a = anchors
            .iter()
            .find(|a| a.platform == platform && a.game == game)
            .unwrap();
        let snap = tdb_qa::schema::snapshot(&repo_root(), a).unwrap();
        let f = meta_field(&snap, "ubPc", "lOnr");
        assert_eq!(
            (f.name.as_deref(), f.bit_width),
            (Some("artid"), 16),
            "{game}-{platform}"
        );
    }

    // The census's observed bucket membership, as RANGES - transcribed from
    // docs/receipts/data/c0-por1-portrait-specimens.json#path_and_id_observations.observed_bucket_membership,
    // the document the portrait profile pins by digest
    // 23964c81140fca35c8862a3449930714dcb1d39cc41eae0d9cb601f3eb443eb2 (uncommitted with the rest
    // of the POR1 receipts, hence transcribed rather than read). The census holds no per-id list,
    // which is exactly why nothing below can place an anchor id beside a file.
    const BUCKETS: [(&str, u64, u64); 4] = [
        ("p0_4000", 1, 3999),
        ("p4001_8000", 4001, 7998),
        ("p8001_12000", 8003, 12401),
        ("silhouettes", 60000, 60003),
    ];
    // The same document's PS4 count: "all 10 421 PS4 files match p<digits>.gnf", no id range.
    const PS4_CENSUS_FILES: usize = 10_421;
    let in_bucket = |ids: &BTreeSet<u64>, i: usize| {
        ids.iter()
            .filter(|v| **v >= BUCKETS[i].1 && **v <= BUCKETS[i].2)
            .count()
    };

    // ---- Legacy (PS3 = Xbox 360 bytes)
    let ubpc = table(&legacy, "ubPc");
    let lonr = find_field(&legacy, ubpc, "lOnr").unwrap();
    assert_eq!((ubpc.current_records, lonr.bit_width), (5632, 16));
    let h = histogram(&legacy, ubpc, &lonr);
    let ids: BTreeSet<u64> = h.keys().copied().collect();
    assert_eq!(ids.len(), 5450);
    assert_eq!((*ids.first().unwrap(), *ids.last().unwrap()), (0, 12814));
    assert_eq!(
        h[&0], 180,
        "180 rows store 0 - a value; no absence code is established"
    );
    assert_eq!(h.get(&0xFFFF), None);
    let shared: BTreeMap<u64, usize> = h
        .iter()
        .filter(|(v, c)| **v != 0 && **c > 1)
        .map(|(v, c)| (*v, *c))
        .collect();
    assert_eq!(
        shared,
        BTreeMap::from([(2, 2), (7, 2), (12, 2)]),
        "the live ambiguity behind `reject`"
    );
    assert_eq!(
        (
            in_bucket(&ids, 0),
            in_bucket(&ids, 1),
            in_bucket(&ids, 2),
            in_bucket(&ids, 3)
        ),
        (1313, 1201, 2585, 0)
    );
    let above: BTreeSet<u64> = ids
        .iter()
        .copied()
        .filter(|v| *v > 12401 && *v < 60000)
        .collect();
    assert_eq!(
        above.len(),
        350,
        "distinct ids above the highest observed Gen7 portrait id"
    );
    assert_eq!(
        (*above.first().unwrap(), *above.last().unwrap()),
        (12403, 12814)
    );
    assert_eq!(
        1 + 1313 + 1201 + 2585 + 350,
        ids.len(),
        "0, the three ranges and the 350 account for every id"
    );
    assert!(ids.iter().all(|v| *v < 60000));

    // ---- NHL 21 PS4
    let ubpc = table(&ps4, "ubPc");
    let lonr = find_field(&ps4, ubpc, "lOnr").unwrap();
    assert_eq!((ubpc.current_records, lonr.bit_width), (7247, 16));
    let h = histogram(&ps4, ubpc, &lonr);
    let ids: BTreeSet<u64> = h.keys().copied().collect();
    assert_eq!(ids.len(), 7244);
    assert_eq!((*ids.first().unwrap(), *ids.last().unwrap()), (1, 21813));
    assert_eq!(h.get(&0), None);
    assert_eq!(h.get(&0xFFFF), None);
    let shared: BTreeMap<u64, usize> = h
        .iter()
        .filter(|(_, c)| **c > 1)
        .map(|(v, c)| (*v, *c))
        .collect();
    assert_eq!(shared, BTreeMap::from([(5, 2), (12, 2), (16, 2)]));
    assert_eq!(PS4_CENSUS_FILES - ids.len(), 3_177);

    // ---- the profile's candidates cite this case and carry these figures
    let p = profile("assets-portraits.toml");
    for (name, figures) in [
        (
            "nhllegacy-ps3",
            vec![
                "5 632",
                "5 450",
                "0..12814",
                "180 rows",
                "1 313 / 1 201 / 2 585",
                "350",
                "12403..12814",
            ],
        ),
        (
            "nhllegacy-xbox360",
            vec!["5 632", "5 450", "0..12814", "180 zeros", "350"],
        ),
        (
            "nhl21-ps4",
            vec!["7 247", "7 244", "1..21813", "10 421", "3 177"],
        ),
    ] {
        let t = target(&p, name);
        assert_eq!(t.identity.state, ConstraintState::Unknown, "{name}");
        let c = t.identity.candidate.as_ref().unwrap();
        assert!(c
            .provenance
            .iter()
            .any(|p| p.ends_with("#legacy_portrait_id_field_is_measured_on_the_anchor")));
        assert_eq!(c.scope.as_deref(), Some("specimen-observation"), "{name}");
        assert_eq!(
            c.corpus.as_deref(),
            Some(t.corpora[0].id.as_str()),
            "{name}"
        );
        for s in figures {
            assert!(c.note.contains(s), "{name}: {s}");
        }
        assert!(c.note.contains("PROPOSED, NOT OPERATIVE"), "{name}");
        let identify = t
            .operations
            .iter()
            .find(|o| o.operation == "identify")
            .unwrap();
        assert_eq!(
            operation_constraint_readiness(t, None, identify),
            OperationReadiness::BlockedByUnknownConstraints(vec!["identity".to_string()]),
            "{name}: a candidate meets no requirement"
        );
    }
}

// ============================ the cross-anchor database-reference record (task B, all targets)

/// Every database field an asset family references, on every committed main anchor.
/// (table, field, family, role)
const ASSET_REFERENCE_FIELDS: [(&str, &str, &str, &str); 19] = [
    ("ubPc", "lOnr", "assets-portraits", "portrait-id"),
    ("ubPc", "pPaD", "assets-heads", "head-id"),
    ("kOtt", "lOnr", "assets-logos", "team-art-id"),
    ("xjgw", "mCXg", "assets-jerseys", "kit-team-key"),
    (
        "xjgw",
        "cVLR",
        "assets-jerseys",
        "kit-style (composite part)",
    ),
    (
        "xjgw",
        "GDQL",
        "assets-jerseys",
        "kit-team-art-id (composite part)",
    ),
    (
        "xjgw",
        "FccT",
        "assets-jerseys",
        "kit-variant (composite part)",
    ),
    ("kOtt", "Rrgs", "assets-jerseys", "team-font"),
    ("kOtt", "Kacu", "assets-jerseys", "team-jersey-style"),
    ("kOtt", "tftz", "assets-jerseys", "team-pants-style"),
    ("kOtt", "nfzw", "assets-jerseys", "team-sock-style"),
    (
        "xmja",
        "wXFx",
        "assets-equipment",
        "stick (shared by the goalie stick)",
    ),
    ("xmja", "NmUR", "assets-equipment", "gloves"),
    ("xmja", "YUZp", "assets-equipment", "helmet"),
    ("xmja", "AOOe", "assets-equipment", "skates"),
    ("fMVl", "Kmxz", "assets-equipment", "blocker"),
    ("fMVl", "OTJR", "assets-equipment", "trapper"),
    ("fMVl", "mIUA", "assets-equipment", "pads"),
    (
        "fMVl",
        "JOlt",
        "assets-equipment",
        "goalie-helmet (mask paint)",
    ),
];

const REFERENCE_RECORD: &str = "docs/receipts/data/c0-as6-asset-db-references.csv";

/// The registry-unit notes that keep a row from being read as a shipping-target fact.
fn unit_note(target: &str) -> &'static str {
    match target {
        "nhl07-ps3" => "research copy, not a shipping target; byte-identical to nhl07-xbox360",
        "nhl19-ps4" | "nhl20-ps4" => {
            "nhl19-ps4 and nhl20-ps4 anchors are byte-identical; attribution unresolved (C0-REG4-R1)"
        }
        _ => "",
    }
}

fn render_reference_record() -> String {
    let anchors = tdb_qa::discover_main_anchors(&repo_root()).unwrap();
    assert_eq!(anchors.len(), 27, "the 27 committed main anchors");
    let mut out = String::from(
        "target,platform,game,anchor_db,anchor_sha256,table,table_name,field,field_name,bit_width,rows,distinct_values,min,max,zero_rows,status,family,role,unit_note\n",
    );
    let mut anchors: Vec<_> = anchors.iter().collect();
    anchors.sort_by_key(|a| (a.platform.clone(), a.game.clone()));
    for a in anchors {
        let target = format!("{}-{}", a.game, a.platform);
        let (bytes, digest) = anchor_bytes(&a.db_rel);
        let db = TdbFile::parse(&bytes).unwrap();
        let snap = tdb_qa::schema::snapshot(&repo_root(), a).unwrap();
        for (t, f, family, role) in ASSET_REFERENCE_FIELDS {
            let prefix = format!(
                "{target},{},{},{},{digest},{t},",
                a.platform, a.game, a.db_rel
            );
            let suffix = format!("{family},\"{role}\",\"{}\"", unit_note(&target));
            let Some(tb) = db.table_canonical(t) else {
                out.push_str(&format!("{prefix},{f},,,,,,,,absent-table,{suffix}\n"));
                continue;
            };
            let table_name = snap
                .tables
                .iter()
                .find(|s| s.canonical_id == t)
                .and_then(|s| s.name.clone())
                .unwrap_or_default();
            let Some(fd) = find_field(&db, tb, f) else {
                out.push_str(&format!(
                    "{prefix}{table_name},{f},,,{},,,,,absent-field,{suffix}\n",
                    tb.current_records
                ));
                continue;
            };
            let field_name = meta_field(&snap, t, f).name.clone().unwrap_or_default();
            let h = histogram(&db, tb, &fd);
            let (lo, hi) = match (h.keys().next(), h.keys().next_back()) {
                (Some(lo), Some(hi)) => (lo.to_string(), hi.to_string()),
                _ => (String::new(), String::new()),
            };
            out.push_str(&format!(
                "{prefix}{table_name},{f},{field_name},{},{},{},{lo},{hi},{},measured,{suffix}\n",
                fd.bit_width,
                tb.current_records,
                h.len(),
                h.get(&0).copied().unwrap_or(0)
            ));
        }
    }
    out
}

/// The committed record equals what the anchors say today. If an anchor, its metadata or the
/// field list changes, this fails and prints the regenerated text: install it deliberately.
#[test]
fn asset_reference_fields_are_measured_on_every_anchor_and_recorded() {
    let rendered = render_reference_record();
    let committed = std::fs::read_to_string(repo_root().join(REFERENCE_RECORD))
        .unwrap_or_else(|e| panic!("{REFERENCE_RECORD}: {e}\n--- regenerate from ---\n{rendered}"))
        .replace("\r\n", "\n");
    assert!(
        committed == rendered,
        "{REFERENCE_RECORD} differs from the anchors' measurement\n--- regenerated ---\n{rendered}"
    );

    // The record's own shape, and the facts the profiles and receipt cite from it.
    let rows: Vec<Vec<&str>> = rendered.lines().skip(1).map(|l| split_csv(l)).collect();
    assert_eq!(rows.len(), 27 * ASSET_REFERENCE_FIELDS.len());
    let cell = |target: &str, table: &str, field: &str, col: usize| -> String {
        rows.iter()
            .find(|r| r[0] == target && r[5] == table && r[7] == field)
            .map(|r| r[col].to_string())
            .unwrap_or_else(|| panic!("{target} {table}.{field}"))
    };
    // status 15, bit_width 9
    assert_eq!(cell("nhllegacy-xbox360", "xmja", "wXFx", 9), "7");
    assert_eq!(cell("nhl08-xbox360", "xmja", "wXFx", 9), "8");
    assert_eq!(cell("nhl08-xbox360", "xmja", "NmUR", 9), "5");
    assert_eq!(cell("nhl08-xbox360", "xmja", "YUZp", 9), "4");
    assert_eq!(cell("nhl21-ps4", "xmja", "wXFx", 9), "10");
    assert_eq!(cell("nhl21-ps4", "fMVl", "JOlt", 9), "10");
    // `xjgw` (the jersey composite) does not exist before NHL 11; `kOtt.lOnr` does not exist on
    // the gen6 title; the goalie mask id does not exist on NHL 08/09; the team style fields are
    // gone on NHL 17+ PS4.
    for target in [
        "nhl07-xbox360",
        "nhl07-ps3",
        "nhl08-xbox360",
        "nhl09-ps3",
        "nhl10-xbox360",
        "nhl10-ps3",
    ] {
        assert_eq!(cell(target, "xjgw", "GDQL", 15), "absent-table", "{target}");
    }
    assert_eq!(cell("nhl11-xbox360", "xjgw", "GDQL", 15), "measured");
    assert_eq!(cell("nhl07-xbox360", "kOtt", "lOnr", 15), "absent-field");
    assert_eq!(cell("nhl07-xbox360", "xmja", "wXFx", 15), "absent-table");
    assert_eq!(cell("nhl08-ps3", "fMVl", "JOlt", 15), "absent-field");
    assert_eq!(cell("nhl10-ps3", "fMVl", "JOlt", 15), "measured");
    assert_eq!(cell("nhl17-ps4", "kOtt", "Rrgs", 15), "absent-field");
    assert_eq!(cell("nhl16-ps4", "kOtt", "Rrgs", 15), "measured");
    // Portrait and head ids exist on every anchor, 14 bits through NHL 14 and 16 from NHL 15.
    for r in rows.iter().filter(|r| r[5] == "ubPc") {
        assert_eq!(r[15], "measured", "{}", r[0]);
        let bits: u32 = r[9].parse().unwrap();
        assert!(matches!(bits, 14..=16), "{} {}: {bits}", r[0], r[7]);
    }
    assert_eq!(cell("nhllegacy-xbox360", "ubPc", "lOnr", 11), "5450");
    assert_eq!(cell("nhl21-ps4", "ubPc", "lOnr", 11), "7244");
    // The unit notes are on exactly the rows they belong to.
    for r in &rows {
        assert_eq!(r[18], unit_note(r[0]), "{}", r[0]);
    }
}

/// Split one CSV line whose quoted cells contain no quotes or newlines (this record's only quoted
/// cells are the free-text `role` and `unit_note`).
fn split_csv(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    loop {
        if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"').expect("closing quote");
            out.push(&stripped[..end]);
            rest = &stripped[end + 1..];
            match rest.strip_prefix(',') {
                Some(r) => rest = r,
                None => break,
            }
        } else {
            match rest.find(',') {
                Some(i) => {
                    out.push(&rest[..i]);
                    rest = &rest[i + 1..];
                }
                None => {
                    out.push(rest);
                    break;
                }
            }
        }
    }
    out
}
