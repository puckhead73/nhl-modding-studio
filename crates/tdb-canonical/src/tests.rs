//! Positive and negative coverage for the canonical/adapter contract.
//!
//! Every enforceable rule has at least one negative case proving an invalid declaration cannot pass
//! as a qualified constraint, and every committed document has a positive case proving it validates
//! clean.

use super::*;
use std::path::{Path, PathBuf};

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

fn fixture(relative: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative),
    )
    .unwrap_or_else(|e| panic!("fixture {relative}: {e}"))
}

const COMMITTED_ADAPTERS: [&str; 27] = [
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
    "nhl15-ps4.toml",
    "nhl16-ps4.toml",
    "nhl17-ps4.toml",
    "nhl18-ps4.toml",
    "nhl19-ps4.toml",
    "nhl20-ps4.toml",
    "nhl21-ps4.toml",
    "nhl07-xbox360.toml",
    "nhl08-xbox360.toml",
    "nhl09-xbox360.toml",
    "nhl08-ps3.toml",
    "nhl09-ps3.toml",
    "nhl07-ps3.toml",
];

/// The adapters whose capacities, discriminator and companion rule are measured on their own anchor
/// by a per-target module — on the gen7 targets the capacities come from the target's own ROSTER
/// fixture (C0-REG1a/b, C0-REG2, C0-REG3); on the PS4 targets from the target's own main anchor,
/// whose directory is not packed on that generation (C0-REG4, `target_ps4_tests`). The other two
/// (`nhl15-xbox360`, `nhl10-xbox360`) still carry seven audit-derived capacities and candidate
/// discriminators. The team *sentinel* is measured on all of these but `nhl10-ps3`, where the
/// as-is encoding leaves no distinguishable absence code (see `target_batch_tests::nhl10_ps3`).
const MEASURED_ON_OWN_ANCHOR: [&str; 22] = [
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
    "nhl15-ps4.toml",
    "nhl16-ps4.toml",
    "nhl17-ps4.toml",
    "nhl18-ps4.toml",
    "nhl21-ps4.toml",
    "nhl07-xbox360.toml",
    "nhl08-xbox360.toml",
    "nhl09-xbox360.toml",
    "nhl08-ps3.toml",
    "nhl09-ps3.toml",
];

/// The adapters whose anchor attribution is unresolved (C0-REG4-R1): the files committed under
/// `anchors/ps4/nhl19/` and `anchors/ps4/nhl20/` are byte-identical and no committed capture
/// provenance says which title either is. Every measurement they carry is a candidate under an
/// unknown block; they are registered representation, never target coverage, and every sweep that
/// asserts an established fact takes the "nothing established" branch for them.
const ATTRIBUTION_UNRESOLVED: [&str; 2] = ["nhl19-ps4.toml", "nhl20-ps4.toml"];

/// The NHL 07 PS3 research copy (C0-REG5): registered under its registry identity as
/// representation only — its bytes are the Xbox 360 anchor's, the scope excludes it from the
/// shipping targets, and no PS3 capture is recorded — so, like the two above, it establishes nothing.
const RESEARCH_COPIES: [&str; 1] = ["nhl07-ps3.toml"];

/// Every adapter that is registered representation only (unattributed or research).
fn non_operative(name: &str) -> bool {
    ATTRIBUTION_UNRESOLVED.contains(&name) || RESEARCH_COPIES.contains(&name)
}

/// The early-generation adapters (C0-REG5): gen6 NHL 07 and the gen7 v1-era NHL 08 / 09. Their
/// ROSTER fixtures decode through the RawV3 codec; NHL 09's carry the ceiling, NHL 07's and
/// NHL 08's yield a TDB without the roster core (capacities unknown, not substituted); the rating
/// regime is the audit's gen6 row on NHL 07 and unnamed — a candidate — on NHL 08 / 09.
const EARLY_ADAPTERS: [&str; 6] = [
    "nhl07-xbox360.toml",
    "nhl08-xbox360.toml",
    "nhl09-xbox360.toml",
    "nhl08-ps3.toml",
    "nhl09-ps3.toml",
    "nhl07-ps3.toml",
];

/// The gen8 (PS4) adapters: little-endian `nhl_game.db`, 7-bit ratings with no offset, main-DB
/// capacities. Where a sweep below asserts a gen7 fact (the 6-bit offset-36 regime, a ROSTER-save
/// capacity source), these take the branch their own evidence supports; the gen7 assertions keep
/// their meaning for the gen7 adapters.
const GEN8_ADAPTERS: [&str; 7] = [
    "nhl15-ps4.toml",
    "nhl16-ps4.toml",
    "nhl17-ps4.toml",
    "nhl18-ps4.toml",
    "nhl19-ps4.toml",
    "nhl20-ps4.toml",
    "nhl21-ps4.toml",
];

// ================================================================== positive

#[test]
fn the_committed_canonical_schema_is_internally_consistent() {
    let schema = schema();
    assert_eq!(validate_project_schema(&schema), vec![]);
    assert!(schema.property_paths().contains("player.rating_overall"));
    assert!(schema.property_paths().contains("team.name"));
}

#[test]
fn the_committed_adapters_satisfy_the_contract() {
    let schema = schema();
    for name in COMMITTED_ADAPTERS {
        let adapter = adapter(name);
        let findings = validate_adapter(&adapter, &schema);
        assert_eq!(findings, vec![], "{name}: {findings:?}");
    }
}

#[test]
fn the_synthetic_fixture_validates_and_is_labelled_synthetic() {
    let synthetic = parse_adapter(&fixture("synthetic-adapter.toml")).expect("it parses");
    assert!(synthetic.synthetic, "the fixture declares itself synthetic");
    let findings = validate_adapter(&synthetic, &schema());
    assert_eq!(findings, vec![], "{findings:?}");
}

#[test]
fn no_committed_adapter_is_a_synthetic_fixture() {
    for name in COMMITTED_ADAPTERS {
        assert!(
            !adapter(name).synthetic,
            "{name}: a synthetic fixture must never live in defs/adapters/"
        );
    }
}

#[test]
fn an_adjacent_title_does_not_inherit_its_neighbours_transform() {
    // The concrete point of having two adapters: NHL 10 and NHL 15 disagree, and each states its
    // own fact rather than inheriting one. Every axis asserted here is MEASURED on both targets; the
    // axes that rest on an expectation live in `candidate_origins_differ_but_neither_is_operative`.
    let find = |name: &str, canonical: &str| {
        adapter(name)
            .mappings
            .into_iter()
            .find(|m| m.canonical == canonical)
            .unwrap_or_else(|| panic!("{name} maps {canonical}"))
    };

    // 1. birth-date index origin — proven by a named case on both targets.
    let ten = find("nhl10-xbox360.toml", "player.birth_date");
    let fifteen = find("nhl15-xbox360.toml", "player.birth_date");
    assert_eq!(ten.transform.established().unwrap().kind, "identity");
    let fifteen_transform = fifteen.transform.established().unwrap();
    assert_eq!(fifteen_transform.kind, "index-origin");
    assert_eq!(fifteen_transform.index_origin, Some(0));

    // 2. the main-team pointer's declared range and signedness — read from the two snapshots.
    let ten_team = find("nhl10-xbox360.toml", "player.team");
    let fifteen_team = find("nhl15-xbox360.toml", "player.team");
    let ten_storage = ten_team.storage.established().unwrap();
    let fifteen_storage = fifteen_team.storage.established().unwrap();
    assert_eq!((ten_storage.range_low, ten_storage.range_high), (0, 255));
    assert_eq!(ten_storage.signedness, "unsigned");
    assert_eq!(
        (fifteen_storage.range_low, fifteen_storage.range_high),
        (-1, 254)
    );
    assert_eq!(fifteen_storage.signedness, "neg1-sentinel");

    // 3. the string budget for the same canonical property.
    let ten_name = find("nhl10-xbox360.toml", "player.first_name");
    let fifteen_name = find("nhl15-xbox360.toml", "player.first_name");
    assert_eq!(
        ten_name.string_storage.established().unwrap().byte_capacity,
        Some(40)
    );
    assert_eq!(
        fifteen_name
            .string_storage
            .established()
            .unwrap()
            .byte_capacity,
        Some(25)
    );

    // 4. the same canonical rating sits on a differently named field.
    assert_eq!(
        find("nhl10-xbox360.toml", "player.rating_speed")
            .field
            .as_deref(),
        Some("qPWY")
    );
    assert_eq!(
        find("nhl15-xbox360.toml", "player.rating_speed")
            .field
            .as_deref(),
        Some("AdEb")
    );
}

/// C0-CAN1-R1. The two main-team index origins still differ, and both are still recorded — but as
/// **candidates**, because a runtime-probed expectation documented in a source comment is not a
/// measurement of either target.
#[test]
fn candidate_origins_differ_but_neither_is_operative() {
    for (name, expected_origin) in [("nhl15-xbox360.toml", 1), ("nhl10-xbox360.toml", 0)] {
        let team = adapter(name)
            .mappings
            .into_iter()
            .find(|m| m.canonical == "player.team")
            .expect("mapped");

        assert_eq!(team.transform.state, ConstraintState::Unknown, "{name}");
        assert!(
            team.transform.established().is_none(),
            "{name}: nothing may read a candidate as an established transform"
        );
        let candidate = team
            .transform
            .candidate_value()
            .expect("the proposal is recorded, not lost");
        assert_eq!(candidate.index_origin, Some(expected_origin), "{name}");

        // The operative transform names claim nothing either.
        assert!(transform_unresolved(&team), "{name}");
        assert_eq!(team.qualification, "blocked", "{name}");
        assert!(team.default.is_none(), "{name}");

        // The inferred foreign key is a candidate too.
        assert_eq!(team.foreign_key.state, ConstraintState::Unknown, "{name}");
        assert_eq!(
            team.foreign_key
                .candidate_value()
                .map(|f| f.parent_table.as_str()),
            Some("kOtt"),
            "{name}"
        );
    }
}

/// C0-CAN1-R1. A metadata name search that finds nothing proves only that the search found nothing.
/// It is a discovery note, not an authorization to declare the property target-absent.
#[test]
fn an_unfound_field_is_an_unresolved_identity_not_a_proven_absence() {
    let ten = adapter("nhl10-xbox360.toml");
    let omission = ten
        .omissions
        .iter()
        .find(|o| o.canonical == "player.rating_overall")
        .expect("NHL 10 records the property as unmapped");
    assert_eq!(
        omission.reason, "unknown-field-identity",
        "a name search cannot establish semantic absence"
    );
    // The search result is retained, as a note about where someone already looked.
    assert!(
        omission.detail.contains("DISCOVERY NOTE"),
        "the search is kept as discovery, not as proof"
    );
    assert!(
        omission.detail.contains("1769"),
        "and it still records what it covered"
    );

    // Nothing in either committed adapter claims proof of absence.
    for name in COMMITTED_ADAPTERS {
        assert!(
            adapter(name)
                .omissions
                .iter()
                .all(|o| o.reason != "target-absent"),
            "{name}: no committed adapter claims target-absence on present evidence"
        );
    }

    // ... and the neighbouring title still maps the same property to a real field.
    let mapped = adapter("nhl15-xbox360.toml")
        .mappings
        .into_iter()
        .find(|m| m.canonical == "player.rating_overall")
        .expect("NHL 15 maps it");
    assert_eq!(mapped.table.as_deref(), Some("dSvy"));
    assert_eq!(mapped.field.as_deref(), Some("eWUF"));
}

/// C0-CAN1-R1. Every "this field has no spare code" argument was an inference from a declared
/// range. None of them is operative any more. C0-REG1a-R1: the one sentinel that IS operative is
/// Legacy's main-team absence code, measured on its anchor by a named case — listed here by name so
/// it is the only one.
#[test]
fn an_unsupported_no_sentinel_claim_is_not_operative() {
    let mut candidates = 0;
    // Per adapter, so a new file cannot pass by shifting the total: NHL 15 records the reasoning on
    // shoots / overall / team, NHL 10 on shoots / speed, Legacy on shoots / overall / speed (its
    // team sentinel was promoted from the fourth candidate by measurement).
    // C0-REG2: the six batch adapters measure their team sentinel on their own anchors too; NHL 11
    // and NHL 12 map no stored overall, so they record one rating candidate fewer.
    // C0-REG3: the PS3 NHL 11–13 adapters likewise. NHL 10 PS3 measures that its as-is main-team
    // encoding leaves NO distinguishable absence code (pattern 0 is team code 0 as much as
    // "un-rostered"), so its team sentinel stays a candidate — a measured ambiguity whose
    // "no absence encoding" reading is asserted, not a range inference.
    // C0-REG4: the seven PS4 adapters measure their team sentinel on their own anchors (pattern 0 on
    // un-rostered rows only, at the target's own width) and record the two rating sentinels as
    // range inferences over the 7-bit 0..99 field, plus shoots.
    let expected_per_adapter = [
        ("nhl15-xbox360.toml", 3, None),
        ("nhl10-xbox360.toml", 2, None),
        ("nhllegacy-xbox360.toml", 3, Some("player.team")),
        ("nhl15-ps3.toml", 3, Some("player.team")),
        ("nhllegacy-ps3.toml", 3, Some("player.team")),
        ("nhl14-xbox360.toml", 3, Some("player.team")),
        ("nhl14-ps3.toml", 3, Some("player.team")),
        ("nhl13-xbox360.toml", 3, Some("player.team")),
        ("nhl12-xbox360.toml", 2, Some("player.team")),
        ("nhl11-xbox360.toml", 2, Some("player.team")),
        ("nhl13-ps3.toml", 3, Some("player.team")),
        ("nhl12-ps3.toml", 2, Some("player.team")),
        ("nhl11-ps3.toml", 2, Some("player.team")),
        ("nhl10-ps3.toml", 3, None),
        ("nhl15-ps4.toml", 3, Some("player.team")),
        ("nhl16-ps4.toml", 3, Some("player.team")),
        ("nhl17-ps4.toml", 3, Some("player.team")),
        ("nhl18-ps4.toml", 3, Some("player.team")),
        // Attribution unresolved (C0-REG4-R1): the team sentinel measured on the bytes is a
        // candidate too — a measured one, kept apart below — so four candidates, none established.
        ("nhl19-ps4.toml", 4, None),
        ("nhl20-ps4.toml", 4, None),
        ("nhl21-ps4.toml", 3, Some("player.team")),
        // C0-REG5: shoots, the speed rating, and the as-is main-team pointer whose pattern 0 is
        // team code 0 (no absence code, as on NHL 10) — three asserted candidates each.
        ("nhl07-xbox360.toml", 3, None),
        ("nhl08-xbox360.toml", 3, None),
        ("nhl09-xbox360.toml", 3, None),
        ("nhl08-ps3.toml", 3, None),
        ("nhl09-ps3.toml", 3, None),
        ("nhl07-ps3.toml", 3, None),
    ];
    assert_eq!(expected_per_adapter.len(), COMMITTED_ADAPTERS.len());
    for (name, expected, measured_on_this_anchor) in expected_per_adapter {
        let mut here = 0;
        for mapping in &adapter(name).mappings {
            if Some(mapping.canonical.as_str()) == measured_on_this_anchor {
                let sentinel = mapping
                    .null_sentinel
                    .established()
                    .expect("measured by the per-target module on its own anchor");
                assert_eq!(sentinel.encoding, "literal-code", "{name}");
                assert_eq!(
                    mapping.null_sentinel.confidence.as_deref(),
                    Some("measured"),
                    "{name}"
                );
                continue;
            }
            assert!(
                mapping.null_sentinel.established().is_none(),
                "{name}/{}: no other sentinel claim in these adapters rests on more than a \
                 declared range, so none may be operative",
                mapping.canonical
            );
            if let Some(candidate) = &mapping.null_sentinel.candidate {
                candidates += 1;
                here += 1;
                if ATTRIBUTION_UNRESOLVED.contains(&name) && mapping.canonical == "player.team" {
                    // Measured on the bytes, unattributed to the title: a measured candidate
                    // that drives nothing (`target_ps4_tests::<module>::attribution`).
                    assert_eq!(candidate.confidence.as_deref(), Some("measured"), "{name}");
                    assert!(mapping
                        .null_sentinel
                        .note
                        .contains("ATTRIBUTION UNRESOLVED"));
                } else {
                    assert_eq!(
                        candidate.confidence.as_deref(),
                        Some("asserted"),
                        "{name}/{}: a range inference is asserted, never measured",
                        mapping.canonical
                    );
                }
                assert!(!candidate.provenance.is_empty());
                assert!(
                    candidate.note.contains("NOT OPERATIVE"),
                    "{name}/{}: a candidate says so in words too",
                    mapping.canonical
                );
            }
        }
        assert_eq!(here, expected, "{name}: sentinel candidates");
    }
    // 37 across the fourteen gen7 adapters (C0-REG3) plus 3 on each of the seven PS4 adapters
    // (C0-REG4) plus the two unattributed team sentinels (C0-REG4-R1) plus 3 on each of the six
    // early-generation adapters (C0-REG5).
    assert_eq!(
        candidates, 78,
        "the reasoning is preserved on every field it was claimed for"
    );
}

/// C0-CAN1-R1. A candidate must not reach any derivation. C0-REG1a-R1: on the one adapter whose
/// origin is measured on its own anchor the derivation does run — the contrast is the point.
#[test]
fn a_candidate_index_origin_does_not_drive_derivation() {
    for name in ["nhl15-xbox360.toml", "nhl10-xbox360.toml"] {
        let team = adapter(name)
            .mappings
            .into_iter()
            .find(|m| m.canonical == "player.team")
            .expect("mapped");
        // Storage IS established and a candidate origin IS recorded; a human range must still not
        // fall out of them, because the transform that would shift the range is not established.
        assert!(team.storage.established().is_some(), "{name}");
        assert!(team.transform.candidate_value().is_some(), "{name}");
        assert_eq!(
            human_range(&team),
            None,
            "{name}: a proposed origin must never produce a human range"
        );
    }
    let legacy_team = adapter("nhllegacy-xbox360.toml")
        .mappings
        .into_iter()
        .find(|m| m.canonical == "player.team")
        .expect("mapped");
    assert!(
        legacy_team.transform.candidate.is_none(),
        "promoted, so no proposal remains"
    );
    // C0-REG1a-R2: the derivation still does not run — for a different reason. `player.team` is a
    // reference (it carries a `[mapping.foreign_key]` proposal), and a reference has no human range:
    // the decoded -1..254 shifted to 1-origin would be a numeric interval [0, 255] that is neither
    // the canonical team domain nor a proven set of valid references.
    assert!(legacy_team.transform.established().is_some());
    assert!(legacy_team.foreign_key.candidate.is_some());
    assert_eq!(
        human_range(&legacy_team),
        None,
        "a reference mapping has no human range"
    );
    // The same numeric blocks WITHOUT the reference proposal are what the helper describes: the
    // numeric encoding's interval, and nothing about teams.
    let mut numeric_only = legacy_team.clone();
    numeric_only.foreign_key = Constraint::default();
    assert_eq!(
        human_range(&numeric_only),
        Some([0, 255]),
        "numeric-encoding information only: decoded -1..254 shifted to 1-origin"
    );

    // Same for a candidate storage block, exercised synthetically.
    let synthetic = parse_adapter(&fixture("synthetic-adapter.toml")).expect("parses");
    let candidate_storage = synthetic
        .mappings
        .iter()
        .find(|m| m.canonical == "player.last_name")
        .expect("the fixture's candidate mapping");
    assert!(candidate_storage.storage.candidate_value().is_some());
    assert!(candidate_storage.transform.candidate_value().is_some());
    assert_eq!(human_range(candidate_storage), None);
    assert_eq!(scalar_editable(candidate_storage), None);
}

/// C0-CAN1-R1. The corrections must not have quietly deleted the facts that *are* supported.
#[test]
fn supported_existing_constraints_still_validate() {
    let fifteen = adapter("nhl15-xbox360.toml");
    let overall = fifteen
        .mappings
        .iter()
        .find(|m| m.canonical == "player.rating_overall")
        .expect("mapped");
    // A measured storage range plus a proven offset regime still derives the human range.
    assert_eq!(human_range(overall), Some([36, 99]));
    assert_eq!(overall.transform.established().unwrap().offset, Some(36));
    assert!(overall.inverse_verified);

    // Capacities are untouched: measured per title from real ROSTER saves, carried across
    // platforms by an equivalence bridge the anchors manifest states as verified for NHL 10-15.
    assert!(
        fifteen
            .capacities
            .iter()
            .filter(|c| c.declared.established().is_some())
            .count()
            >= 7
    );

    // The birth-date case still qualifies on every attributed target; on the two unattributed
    // ones the same measurement is a candidate and qualifies nothing (C0-REG4-R1).
    for name in COMMITTED_ADAPTERS {
        let birth = adapter(name)
            .mappings
            .into_iter()
            .find(|m| m.canonical == "player.birth_date")
            .expect("mapped");
        if non_operative(name) {
            assert_eq!(birth.qualification, "blocked", "{name}");
            assert!(birth.transform.established().is_none(), "{name}");
            assert!(birth.transform.candidate_value().is_some(), "{name}");
            continue;
        }
        assert_eq!(birth.qualification, "automated-tested", "{name}");
        assert!(birth.transform.established().is_some(), "{name}");
    }
}

#[test]
fn the_human_range_is_derived_from_the_transform_and_never_declared() {
    let fifteen = adapter("nhl15-xbox360.toml");
    let overall = fifteen
        .mappings
        .iter()
        .find(|m| m.canonical == "player.rating_overall")
        .expect("mapped");
    // 6-bit storage 0..63 plus the proven +36 offset regime.
    assert_eq!(human_range(overall), Some([36, 99]));

    let ten = adapter("nhl10-xbox360.toml");
    let birth = ten
        .mappings
        .iter()
        .find(|m| m.canonical == "player.birth_date")
        .expect("mapped");
    // A composite mapping has no single storage block, so no single human range.
    assert_eq!(human_range(birth), None);
}

#[test]
fn scalar_editability_follows_updatable_and_not_key() {
    let fifteen = adapter("nhl15-xbox360.toml");
    for mapping in &fifteen.mappings {
        if mapping.status.established().is_some() {
            assert_eq!(
                scalar_editable(mapping),
                Some(true),
                "{} is declared updatable and non-key",
                mapping.canonical
            );
        }
    }
}

#[test]
fn a_composite_property_names_every_field_it_spans() {
    for name in COMMITTED_ADAPTERS {
        let a = adapter(name);
        let birth = a
            .mappings
            .iter()
            .find(|m| m.canonical == "player.birth_date")
            .expect("mapped");
        assert!(birth.field.is_none(), "{name}: composite, not single-field");
        let roles: Vec<&str> = birth.components.iter().map(|c| c.role.as_str()).collect();
        assert_eq!(roles, ["year", "month", "day"], "{name}");
        for component in &birth.components {
            let carried = if non_operative(name) {
                component.storage.candidate_value().is_some()
            } else {
                component.storage.established().is_some()
            };
            assert!(
                carried,
                "{name}: component {} carries its own storage",
                component.role
            );
        }
        assert!(
            field_identity_established(birth),
            "{name}: a complete component set IS an established field identity"
        );
    }
}

#[test]
fn capacity_records_which_file_the_ceiling_came_from() {
    let fifteen = adapter("nhl15-xbox360.toml");
    let bio = fifteen
        .capacities
        .iter()
        .find(|c| c.table == "ubPc")
        .expect("declared");
    let value = bio.declared.established().expect("established");
    // The gen7 main DB is packed, so the ceiling is only visible in the roster save.
    assert_eq!(value.source, "roster-save-max-records");
    assert_eq!(value.ceiling_records, 7034);
    assert_eq!(value.population_records, 5534);
    assert!(value.ceiling_records > value.population_records);

    // A table the capacity audit does not cover stays unknown rather than being read off the
    // packed main DB, which would understate it.
    let teams = fifteen
        .capacities
        .iter()
        .find(|c| c.table == "kOtt")
        .expect("registered");
    assert_eq!(teams.declared.state, ConstraintState::Unknown);
    assert!(teams.declared.value.is_none());
}

#[test]
fn declared_coverage_is_stated_rather_than_implied() {
    let (mapped, total) = mapped_fraction(&adapter("nhl15-xbox360.toml")).expect("declared");
    assert_eq!(total, 2969);
    assert!(mapped < total, "the adapter states it is partial");
    let (mapped, total) = mapped_fraction(&adapter("nhl10-xbox360.toml")).expect("declared");
    assert_eq!(total, 1769);
    assert!(mapped < total);
}

#[test]
fn representation_completeness_is_reported_apart_from_registry_population() {
    // Format completeness: every constraint Q07a.1 names has a representation.
    let (represented, required) = representation_completeness();
    assert_eq!(represented, required);
    assert_eq!(required, Q07A1_REPRESENTATION.len());
    for (constraint, location) in Q07A1_REPRESENTATION {
        assert!(!constraint.is_empty() && !location.is_empty());
    }

    // Population: what the committed adapters actually declare — a much smaller number, and the
    // synthetic fixture contributes nothing to it.
    let synthetic = parse_adapter(&fixture("synthetic-adapter.toml")).expect("parses");
    let adapters: Vec<Adapter> = COMMITTED_ADAPTERS
        .iter()
        .map(|n| adapter(n))
        .chain(std::iter::once(synthetic))
        .collect();
    let population = registry_population(adapters.iter());
    assert_eq!(
        population.len(),
        COMMITTED_ADAPTERS.len(),
        "the synthetic fixture is excluded from coverage"
    );
    assert!(population.iter().all(|p| p.target != "fixture-synthetic"));
    // C0-REG4-R1: the two attribution-unresolved adapters establish nothing — that is their
    // honest state — and are reported apart (`registered_representation_is_reported_apart…`).
    let unattributed: BTreeSet<&str> = ATTRIBUTION_UNRESOLVED
        .iter()
        .chain(RESEARCH_COPIES.iter())
        .map(|n| n.trim_end_matches(".toml"))
        .collect();
    assert!(
        population
            .iter()
            .all(|p| (p.established_constraints > 0) != unattributed.contains(p.target.as_str())),
        "every attributed adapter declares real constraints; the unattributed ones declare none"
    );
    assert!(
        population.iter().all(|p| p.unknown_constraints > 0),
        "and every one still carries unknowns, which is the honest state"
    );
    // C0-CAN1-R1: candidates are counted APART from established blocks. A proposal is not coverage.
    assert!(
        population.iter().all(|p| p.candidate_constraints > 0),
        "every adapter records non-operative proposals"
    );
    for p in &population {
        assert!(
            p.candidate_constraints <= p.unknown_constraints,
            "{}: a candidate only ever sits on an unknown block",
            p.target
        );
    }
}

/// C0-REG4-R1. Registered representation and established target coverage are two different
/// counts. The two adapters whose anchor bytes cannot be attributed to their title (byte-identical
/// `nhl19` / `nhl20` folders, no capture provenance) are in the registry — every measurement is
/// there as a candidate — and contribute **nothing** to coverage: nothing established, every
/// mapping blocked, no human range, no editable scalar. The attributed adapters are the coverage.
#[test]
fn registered_representation_is_reported_apart_from_established_target_coverage() {
    let adapters: Vec<Adapter> = COMMITTED_ADAPTERS.iter().map(|n| adapter(n)).collect();
    let population = registry_population(adapters.iter());
    let (mut registered_only, mut established) = (Vec::new(), Vec::new());
    for p in &population {
        if p.established_constraints == 0 {
            registered_only.push(p.target.as_str());
        } else {
            established.push(p.target.as_str());
        }
    }
    assert_eq!(registered_only, ["nhl19-ps4", "nhl20-ps4", "nhl07-ps3"]);
    assert_eq!(
        established.len(),
        COMMITTED_ADAPTERS.len() - ATTRIBUTION_UNRESOLVED.len() - RESEARCH_COPIES.len()
    );
    for name in ATTRIBUTION_UNRESOLVED
        .iter()
        .chain(RESEARCH_COPIES.iter())
        .copied()
    {
        let a = adapter(name);
        assert_eq!(
            validate_adapter(&a, &schema()),
            vec![],
            "{name}: still a valid document"
        );
        for m in &a.mappings {
            assert_eq!(m.qualification, "blocked", "{name}/{}", m.canonical);
            assert!(transform_unresolved(m), "{name}/{}", m.canonical);
            assert_eq!(human_range(m), None, "{name}/{}", m.canonical);
            assert_eq!(scalar_editable(m), None, "{name}/{}", m.canonical);
            assert!(
                field_identity_established(m),
                "{name}/{}: the field is named — representation, not coverage",
                m.canonical
            );
        }
        let p = population.iter().find(|p| p.target == a.target).unwrap();
        assert!(p.candidate_constraints > 0 && p.candidate_constraints <= p.unknown_constraints);
        // Coverage is stated as a fraction of fields *named*, and the file says what that is.
        assert!(mapped_fraction(&a).is_some());
        assert!(a
            .coverage_declared
            .as_ref()
            .unwrap()
            .note
            .contains("REGISTERED AS REPRESENTATION ONLY"));
    }
}

// =========================================================== version transition

#[test]
fn a_v1_document_is_not_validated_in_place() {
    let v1_text = fixture("v1/nhl15-xbox360.toml");
    // It does not even parse as v2: v1 carries keys v2 does not define, and unknown keys are refused
    // rather than dropped.
    assert!(parse_adapter(&v1_text).is_err());
}

#[test]
fn the_v1_transition_is_lossless_by_report_for_both_committed_examples() {
    let schema = schema();
    for name in ["v1/nhl15-xbox360.toml", "v1/nhl10-xbox360.toml"] {
        let source = parse_adapter_v1(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(source.schema, ADAPTER_SCHEMA_PREVIOUS);
        let upgraded = upgrade_v1(&source);

        assert_eq!(upgraded.adapter.schema, ADAPTER_SCHEMA);
        assert_eq!(upgraded.adapter.target, source.target);
        assert_eq!(upgraded.adapter.mappings.len(), source.mappings.len());
        assert_eq!(upgraded.adapter.omissions.len(), source.omissions.len());

        // The transition invents nothing: every constraint block comes out unknown.
        for mapping in &upgraded.adapter.mappings {
            for state in [
                mapping.storage.state,
                mapping.status.state,
                mapping.transform.state,
                mapping.enumeration.state,
                mapping.null_sentinel.state,
                mapping.string_storage.state,
                mapping.foreign_key.state,
                mapping.applicability.state,
            ] {
                assert_eq!(state, ConstraintState::Unknown, "{name}");
            }
        }

        // Every v1 field with no v2 destination is reported rather than dropped.
        let reported: BTreeSet<&str> = upgraded.notes.iter().map(|n| n.field.as_str()).collect();
        let v1_placeholders = source
            .mappings
            .iter()
            .filter(|m| m.table == v1::V1_UNKNOWN || m.field == v1::V1_UNKNOWN)
            .count();
        assert!(
            v1_placeholders > 0,
            "{name}: the v1 examples used placeholders"
        );
        assert!(
            reported.contains("table") || reported.contains("field"),
            "{name}"
        );
        for note in &upgraded.notes {
            assert!(!note.disposition.is_empty(), "{name}: every note says why");
        }

        // And the upgraded document is a faithful translation, not a passing one: it reports the
        // table references it cannot resolve, because a v1 file declares no tables.
        let findings = validate_adapter(&upgraded.adapter, &schema);
        assert!(
            findings.iter().any(|f| f.rule == Rule::UndeclaredTable),
            "{name}: {findings:?}"
        );
    }
}

#[test]
fn the_v1_placeholder_becomes_an_absent_identity_not_the_string_unknown() {
    let source = parse_adapter_v1(&fixture("v1/nhl15-xbox360.toml")).expect("parses");
    let upgraded = upgrade_v1(&source);
    let overall = upgraded
        .adapter
        .mappings
        .iter()
        .find(|m| m.canonical == "player.rating_overall")
        .expect("present in v1");
    assert_eq!(overall.table, None);
    assert_eq!(overall.field, None);
    assert!(!field_identity_established(overall));
    assert!(upgraded
        .notes
        .iter()
        .any(|n| n.field == "table" && n.value == v1::V1_UNKNOWN));
}

#[test]
fn an_unsupported_adapter_schema_version_is_refused() {
    for id in ["target-adapter/1", "target-adapter/3", "target-adapter"] {
        let text = format!(
            "schema = \"{id}\"\nadapter_version = 2\ncanonical_schema_version = 1\n\
             target = \"nhl15-xbox360\"\n"
        );
        let found: Vec<Rule> = validate_adapter(&parse_adapter(&text).expect("parses"), &schema())
            .into_iter()
            .map(|f| f.rule)
            .collect();
        assert!(found.contains(&Rule::UnsupportedSchema), "{id}: {found:?}");
    }
}

// ================================================================== negative

fn adapter_from(body: &str) -> Adapter {
    let head = "schema = \"target-adapter/2\"\nadapter_version = 2\n\
                canonical_schema_version = 1\ntarget = \"nhl15-xbox360\"\nendian = \"big\"\n\
                [[table]]\nid = \"ubPc\"\nname = \"exhibitionplayerbiotable\"\n\
                provenance = [\"x\"]\n\
                [[table]]\nid = \"kOtt\"\nname = \"exhibitionteams\"\nprovenance = [\"x\"]\n";
    parse_adapter(&format!("{head}{body}")).expect("test adapter parses")
}

fn rules(body: &str) -> Vec<Rule> {
    validate_adapter(&adapter_from(body), &schema())
        .into_iter()
        .map(|f| f.rule)
        .collect()
}

/// A minimal well-formed mapping the negative cases mutate.
fn mapping_head(canonical: &str) -> String {
    format!(
        "[[mapping]]\ncanonical = \"{canonical}\"\ntable = \"ubPc\"\nfield = \"GRkp\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n"
    )
}

// ---- the rules that predate the constraint layer ------------------------------

#[test]
fn a_non_identity_read_with_no_inverse_write_is_refused() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.rating_overall\"\nread_transform = \"offset+36\"\n\
         write_transform = \"unknown\"\nconfidence = \"asserted\"\nqualification = \"implemented\"\n\
         provenance = [\"x\"]\n",
    );
    assert!(found.contains(&Rule::InverseWriteMissing), "{found:?}");
}

#[test]
fn an_unknown_default_blocks_rather_than_substituting() {
    for placeholder in ["unknown", "tbd", ""] {
        let found = rules(&format!(
            "{}default = \"{placeholder}\"\n",
            mapping_head("player.shoots")
        ));
        assert!(
            found.contains(&Rule::UnknownDefault),
            "{placeholder:?}: {found:?}"
        );
    }
}

#[test]
fn an_adapter_can_never_declare_game_verification() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.shoots\"\nread_transform = \"identity\"\n\
         write_transform = \"identity\"\nconfidence = \"measured\"\n\
         qualification = \"game-verified\"\nprovenance = [\"x\"]\n",
    );
    assert!(
        found.contains(&Rule::AdapterClaimsGameVerified),
        "{found:?}"
    );
}

#[test]
fn a_known_fact_without_provenance_is_refused() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.shoots\"\nread_transform = \"identity\"\n\
         write_transform = \"identity\"\nconfidence = \"measured\"\nqualification = \"implemented\"\n",
    );
    assert!(found.contains(&Rule::MissingProvenance), "{found:?}");
}

#[test]
fn an_unknown_canonical_property_is_refused_in_both_mappings_and_omissions() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.no_such_thing\"\nread_transform = \"identity\"\n\
         write_transform = \"identity\"\nconfidence = \"unknown\"\nqualification = \"unknown\"\n\
         [[omission]]\ncanonical = \"player.also_not_real\"\nreason = \"unknown-field-identity\"\n",
    );
    assert_eq!(
        found
            .iter()
            .filter(|r| **r == Rule::UnknownCanonicalProperty)
            .count(),
        2,
        "{found:?}"
    );
}

#[test]
fn a_property_cannot_be_both_mapped_and_omitted() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.shoots\"\nread_transform = \"identity\"\n\
         write_transform = \"identity\"\nconfidence = \"unknown\"\nqualification = \"unknown\"\n\
         [[omission]]\ncanonical = \"player.shoots\"\nreason = \"unknown-field-identity\"\n",
    );
    assert!(found.contains(&Rule::MappedAndOmitted), "{found:?}");
}

#[test]
fn a_duplicate_mapping_is_refused() {
    let one = "[[mapping]]\ncanonical = \"player.shoots\"\nread_transform = \"identity\"\n\
               write_transform = \"identity\"\nconfidence = \"unknown\"\nqualification = \"unknown\"\n";
    let found = rules(&format!("{one}{one}"));
    assert!(found.contains(&Rule::DuplicateMapping), "{found:?}");
}

#[test]
fn automated_tested_requires_both_a_verified_inverse_and_a_named_case() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.rating_speed\"\ntable = \"ubPc\"\nfield = \"GRkp\"\n\
         read_transform = \"offset+36\"\nwrite_transform = \"offset-36\"\nconfidence = \"measured\"\n\
         qualification = \"automated-tested\"\nprovenance = [\"x\"]\n",
    );
    assert!(
        found.contains(&Rule::InverseUnverifiedButQualified),
        "{found:?}"
    );
    assert!(found.contains(&Rule::QualifiedWithoutCase), "{found:?}");
}

#[test]
fn an_unestablished_field_identity_cannot_qualify_even_with_a_proven_transform() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.rating_speed\"\ntable = \"ubPc\"\n\
         read_transform = \"offset+36\"\nwrite_transform = \"offset-36\"\ninverse_verified = true\n\
         confidence = \"measured\"\nqualification = \"automated-tested\"\n\
         automated_case = \"x::y\"\nprovenance = [\"x\"]\n",
    );
    assert!(
        found.contains(&Rule::UnknownIdentityButQualified),
        "{found:?}"
    );
}

#[test]
fn an_adapter_written_against_another_schema_version_is_reported() {
    let text = "schema = \"target-adapter/2\"\nadapter_version = 2\n\
                canonical_schema_version = 99\ntarget = \"nhl15-xbox360\"\n";
    let found: Vec<Rule> = validate_adapter(&parse_adapter(text).expect("parses"), &schema())
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(found.contains(&Rule::UnsupportedVersion), "{found:?}");
}

#[test]
fn a_malformed_target_is_refused() {
    let text = "schema = \"target-adapter/2\"\nadapter_version = 2\n\
                canonical_schema_version = 1\ntarget = \"nhl15\"\n";
    let found: Vec<Rule> = validate_adapter(&parse_adapter(text).expect("parses"), &schema())
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(found.contains(&Rule::MalformedTarget), "{found:?}");
}

// ---- misspellings and placeholders -------------------------------------------

#[test]
fn a_misspelled_constraint_field_is_a_parse_error_not_a_silent_drop() {
    // Each of these is one character away from a real key. None may be ignored.
    let cases = [
        // misspelled mapping key
        "[[mapping]]\ncanonical = \"player.shoots\"\nread_transfrom = \"identity\"\n\
         write_transform = \"identity\"\nconfidence = \"unknown\"\nqualification = \"unknown\"\n",
        // misspelled constraint block key
        "[[mapping]]\ncanonical = \"player.shoots\"\nread_transform = \"identity\"\n\
         write_transform = \"identity\"\nconfidence = \"unknown\"\nqualification = \"unknown\"\n\
         [mapping.storage]\nstate = \"unknown\"\nprovenence = [\"x\"]\n",
        // misspelled payload key
        "[[mapping]]\ncanonical = \"player.shoots\"\nread_transform = \"identity\"\n\
         write_transform = \"identity\"\nconfidence = \"asserted\"\nqualification = \"unknown\"\n\
         provenance = [\"x\"]\n[mapping.storage]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.storage.value]\nkind = \"fixed-bit-uint\"\nbit_ofset = 0\n\
         bit_width = 1\nsignedness = \"unsigned\"\nrange_low = 0\nrange_high = 1\nendian = \"big\"\n",
        // an entirely invented top-level key
        "invented_key = true\n",
    ];
    for body in cases {
        let text = format!(
            "schema = \"target-adapter/2\"\nadapter_version = 2\ncanonical_schema_version = 1\n\
             target = \"nhl15-xbox360\"\n{body}"
        );
        assert!(
            parse_adapter(&text).is_err(),
            "an unknown key must be refused, not dropped: {body}"
        );
    }
}

#[test]
fn a_placeholder_string_is_never_an_identity() {
    for placeholder in ["unknown", "TBD", "n/a", "", "?"] {
        let found = rules(&format!(
            "[[mapping]]\ncanonical = \"player.shoots\"\ntable = \"ubPc\"\nfield = \"{placeholder}\"\n\
             read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
             confidence = \"unknown\"\nqualification = \"unknown\"\n"
        ));
        assert!(
            found.contains(&Rule::PlaceholderAsValue),
            "{placeholder:?}: {found:?}"
        );
    }
}

// ---- the constraint envelope --------------------------------------------------

#[test]
fn an_established_constraint_needs_a_value_a_confidence_and_provenance() {
    let found = rules(&format!(
        "{}[mapping.status]\nstate = \"established\"\n",
        mapping_head("player.shoots")
    ));
    assert!(found.contains(&Rule::EstablishedWithoutValue), "{found:?}");
    assert!(
        found.contains(&Rule::EstablishedWithoutConfidence),
        "{found:?}"
    );
    assert!(found.contains(&Rule::MissingProvenance), "{found:?}");
}

#[test]
fn an_established_constraint_may_not_claim_unknown_confidence() {
    let found = rules(&format!(
        "{}[mapping.status]\nstate = \"established\"\nconfidence = \"unknown\"\n\
         provenance = [\"x\"]\n[mapping.status.value]\nkey = false\nupdatable = true\n\
         structural = false\n",
        mapping_head("player.shoots")
    ));
    assert!(
        found.contains(&Rule::EstablishedWithoutConfidence),
        "{found:?}"
    );
}

#[test]
fn an_unknown_constraint_may_not_smuggle_a_value_or_a_confidence() {
    let found = rules(&format!(
        "{}[mapping.status]\nstate = \"unknown\"\nconfidence = \"measured\"\n\
         [mapping.status.value]\nkey = false\nupdatable = true\nstructural = false\n",
        mapping_head("player.shoots")
    ));
    assert_eq!(
        found
            .iter()
            .filter(|r| **r == Rule::ValueWithoutEstablishedState)
            .count(),
        2,
        "{found:?}"
    );
}

#[test]
fn declaring_a_constraint_absent_requires_saying_how_that_was_determined() {
    let found = rules(&format!(
        "{}[mapping.string_storage]\nstate = \"not-applicable\"\n",
        mapping_head("player.shoots")
    ));
    assert!(
        found.contains(&Rule::NotApplicableWithoutReason),
        "{found:?}"
    );
}

// ---- storage, status, transform ------------------------------------------------

#[test]
fn a_storage_range_that_does_not_fit_its_width_is_refused() {
    let cases = [
        ("bit_width = 2", "range_low = 0", "range_high = 63"),
        ("bit_width = 8", "range_low = 100", "range_high = 3"),
        ("bit_width = 0", "range_low = 0", "range_high = 0"),
    ];
    for (width, low, high) in cases {
        let found = rules(&format!(
            "{}[mapping.storage]\nstate = \"established\"\nconfidence = \"measured\"\n\
             provenance = [\"x\"]\n[mapping.storage.value]\nkind = \"fixed-bit-uint\"\n\
             bit_offset = 0\n{width}\nsignedness = \"unsigned\"\n{low}\n{high}\nendian = \"big\"\n",
            mapping_head("player.shoots")
        ));
        assert!(
            found.contains(&Rule::StorageBoundsInconsistent),
            "{width}/{low}/{high}: {found:?}"
        );
    }
}

#[test]
fn an_unsigned_field_cannot_declare_a_negative_range_or_a_neg1_sentinel_regime() {
    let found = rules(&format!(
        "{}[mapping.storage]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.storage.value]\nkind = \"fixed-bit-uint\"\nbit_offset = 0\n\
         bit_width = 8\nsignedness = \"unsigned\"\nrange_low = -1\nrange_high = 254\nendian = \"big\"\n",
        mapping_head("player.shoots")
    ));
    assert!(
        found.contains(&Rule::StorageBoundsInconsistent),
        "{found:?}"
    );
}

#[test]
fn a_field_endianness_cannot_contradict_the_adapters() {
    let found = rules(&format!(
        "{}[mapping.storage]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.storage.value]\nkind = \"fixed-bit-uint\"\nbit_offset = 0\n\
         bit_width = 1\nsignedness = \"unsigned\"\nrange_low = 0\nrange_high = 1\n\
         endian = \"little\"\n",
        mapping_head("player.shoots")
    ));
    assert!(
        found.contains(&Rule::StorageBoundsInconsistent),
        "{found:?}"
    );
}

#[test]
fn a_closed_vocabulary_term_outside_its_vocabulary_is_refused() {
    let found = rules(&format!(
        "{}[mapping.storage]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.storage.value]\nkind = \"six-bit-ish\"\nbit_offset = 0\n\
         bit_width = 1\nsignedness = \"probably-unsigned\"\nrange_low = 0\nrange_high = 1\n\
         endian = \"big\"\n",
        mapping_head("player.shoots")
    ));
    assert_eq!(
        found
            .iter()
            .filter(|r| **r == Rule::UnknownVocabularyTerm)
            .count(),
        2,
        "{found:?}"
    );
}

#[test]
fn a_structured_transform_must_agree_with_the_transform_names() {
    // The index origin says 0 but the read transform spells 1.
    let found = rules(
        "[[mapping]]\ncanonical = \"player.team\"\ntable = \"ubPc\"\nfield = \"dXSB\"\n\
         read_transform = \"index-origin-1\"\nwrite_transform = \"index-origin-1\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.transform]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.transform.value]\nkind = \"index-origin\"\nindex_origin = 0\n",
    );
    assert!(
        found.contains(&Rule::TransformDeclarationConflict),
        "{found:?}"
    );
}

#[test]
fn a_linear_offsets_write_must_be_its_exact_inverse() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.rating_overall\"\ntable = \"ubPc\"\nfield = \"GRkp\"\n\
         read_transform = \"offset+36\"\nwrite_transform = \"offset-50\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.transform]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.transform.value]\nkind = \"linear-offset\"\noffset = 36\n",
    );
    assert!(
        found.contains(&Rule::TransformDeclarationConflict),
        "{found:?}"
    );
}

#[test]
fn a_parameterised_transform_kind_requires_its_parameter() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.team\"\ntable = \"ubPc\"\nfield = \"dXSB\"\n\
         read_transform = \"index-origin-1\"\nwrite_transform = \"index-origin-1\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.transform]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.transform.value]\nkind = \"index-origin\"\n",
    );
    assert!(
        found.contains(&Rule::TransformDeclarationConflict),
        "{found:?}"
    );
}

#[test]
fn an_enum_code_transform_without_an_enumeration_is_refused() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.shoots\"\ntable = \"ubPc\"\nfield = \"GRkp\"\n\
         read_transform = \"enum-code\"\nwrite_transform = \"enum-code\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.transform]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.transform.value]\nkind = \"enum-code\"\n",
    );
    assert!(
        found.contains(&Rule::TransformDeclarationConflict),
        "{found:?}"
    );
}

// ---- enumerations ---------------------------------------------------------------

fn enum_body(entries: &str, extra: &str) -> String {
    format!(
        "[[mapping]]\ncanonical = \"player.shoots\"\ntable = \"ubPc\"\nfield = \"GRkp\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         {extra}[mapping.enumeration]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.enumeration.value]\n{entries}"
    )
}

#[test]
fn duplicate_enum_codes_or_values_are_refused() {
    let found = rules(&enum_body(
        "[[mapping.enumeration.value.entry]]\ncode = 0\ncanonical = \"left\"\n\
         [[mapping.enumeration.value.entry]]\ncode = 0\ncanonical = \"right\"\n",
        "",
    ));
    assert!(found.contains(&Rule::EnumCodeConflict), "{found:?}");

    let found = rules(&enum_body(
        "[[mapping.enumeration.value.entry]]\ncode = 0\ncanonical = \"left\"\n\
         [[mapping.enumeration.value.entry]]\ncode = 1\ncanonical = \"left\"\n",
        "",
    ));
    assert!(found.contains(&Rule::EnumCodeConflict), "{found:?}");
}

#[test]
fn an_enum_value_the_canonical_domain_does_not_define_is_refused() {
    let found = rules(&enum_body(
        "[[mapping.enumeration.value.entry]]\ncode = 0\ncanonical = \"ambidextrous\"\n",
        "",
    ));
    assert!(found.contains(&Rule::EnumValueNotInDomain), "{found:?}");
}

#[test]
fn an_enum_code_outside_the_fields_storage_range_is_refused() {
    let storage = "[mapping.storage]\nstate = \"established\"\nconfidence = \"measured\"\n\
                   provenance = [\"x\"]\n[mapping.storage.value]\nkind = \"fixed-bit-uint\"\n\
                   bit_offset = 0\nbit_width = 1\nsignedness = \"unsigned\"\nrange_low = 0\n\
                   range_high = 1\nendian = \"big\"\n";
    let found = rules(&enum_body(
        "[[mapping.enumeration.value.entry]]\ncode = 7\ncanonical = \"left\"\n",
        storage,
    ));
    assert!(found.contains(&Rule::EnumCodeOutOfRange), "{found:?}");
}

#[test]
fn an_enumeration_on_a_non_enum_property_is_refused() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.rating_overall\"\ntable = \"ubPc\"\nfield = \"GRkp\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.enumeration]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.enumeration.value]\n\
         [[mapping.enumeration.value.entry]]\ncode = 0\ncanonical = \"left\"\n",
    );
    assert!(found.contains(&Rule::ConstraintKindMismatch), "{found:?}");
}

// ---- sentinels ------------------------------------------------------------------

#[test]
fn a_sentinel_must_match_its_encoding_and_its_storage_regime() {
    // `none` carrying a code.
    let found = rules(&format!(
        "{}[mapping.null_sentinel]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.null_sentinel.value]\nencoding = \"none\"\nraw = -1\n",
        mapping_head("player.shoots")
    ));
    assert!(found.contains(&Rule::SentinelConflict), "{found:?}");

    // `raw-all-ones` without a code.
    let found = rules(&format!(
        "{}[mapping.null_sentinel]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.null_sentinel.value]\nencoding = \"raw-all-ones\"\n",
        mapping_head("player.shoots")
    ));
    assert!(found.contains(&Rule::SentinelConflict), "{found:?}");

    // A neg1-sentinel storage regime with a literal sentinel that is not -1.
    let found = rules(
        "[[mapping]]\ncanonical = \"player.team\"\ntable = \"ubPc\"\nfield = \"dXSB\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.storage]\nstate = \"established\"\nconfidence = \"measured\"\nprovenance = [\"x\"]\n\
         [mapping.storage.value]\nkind = \"fixed-bit-uint\"\nbit_offset = 0\nbit_width = 8\n\
         signedness = \"neg1-sentinel\"\nrange_low = -1\nrange_high = 254\nendian = \"big\"\n\
         [mapping.null_sentinel]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.null_sentinel.value]\nencoding = \"literal-code\"\nraw = 0\n",
    );
    assert!(found.contains(&Rule::SentinelConflict), "{found:?}");

    // C0-REG1a-R2: units. `raw-all-ones` is written -1 by convention and nothing else; a
    // `literal-code` is the unsigned stored bit pattern, so it is never negative and must fit the
    // field. The Legacy shape — offset-binary -1..254 with the all-zero pattern as absence — is the
    // clean case.
    let team_with = |storage_signedness: &str, encoding: &str, raw: i64| {
        rules(&format!(
            "[[mapping]]\ncanonical = \"player.team\"\ntable = \"ubPc\"\nfield = \"dXSB\"\n\
             read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
             confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
             [mapping.storage]\nstate = \"established\"\nconfidence = \"measured\"\nprovenance = [\"x\"]\n\
             [mapping.storage.value]\nkind = \"fixed-bit-uint\"\nbit_offset = 0\nbit_width = 8\n\
             signedness = \"{storage_signedness}\"\nrange_low = -1\nrange_high = 254\nendian = \"big\"\n\
             [mapping.null_sentinel]\nstate = \"established\"\nconfidence = \"measured\"\n\
             provenance = [\"x\"]\n[mapping.null_sentinel.value]\nencoding = \"{encoding}\"\nraw = {raw}\n"
        ))
    };
    for (signedness, encoding, raw) in [
        ("neg1-sentinel", "raw-all-ones", 255), // the pattern's numeric value is not the notation
        ("offset-binary", "literal-code", -1),  // a decoded value is not a stored pattern
        ("offset-binary", "literal-code", 256), // does not fit 8 bits
    ] {
        let found = team_with(signedness, encoding, raw);
        assert!(
            found.contains(&Rule::SentinelConflict),
            "{signedness}/{encoding}/{raw}: {found:?}"
        );
    }
    for (signedness, encoding, raw) in [
        ("neg1-sentinel", "raw-all-ones", -1),
        ("offset-binary", "literal-code", 0),
        ("offset-binary", "literal-code", 255),
    ] {
        let found = team_with(signedness, encoding, raw);
        assert!(
            !found.contains(&Rule::SentinelConflict),
            "{signedness}/{encoding}/{raw}: {found:?}"
        );
    }
}

#[test]
fn a_sentinel_may_not_collide_with_an_enum_code() {
    let found = rules(&enum_body(
        "[[mapping.enumeration.value.entry]]\ncode = 0\ncanonical = \"left\"\n\
         [[mapping.enumeration.value.entry]]\ncode = 1\ncanonical = \"right\"\n",
        "[mapping.null_sentinel]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.null_sentinel.value]\nencoding = \"literal-code\"\nraw = 1\n",
    ));
    assert!(found.contains(&Rule::SentinelConflict), "{found:?}");
}

// ---- string storage --------------------------------------------------------------

#[test]
fn a_string_byte_budget_must_agree_with_the_storage_width() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.first_name\"\ntable = \"ubPc\"\nfield = \"HdeP\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"measured\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.storage]\nstate = \"established\"\nconfidence = \"measured\"\nprovenance = [\"x\"]\n\
         [mapping.storage.value]\nkind = \"inline-string\"\nbit_offset = 0\nbit_width = 200\n\
         signedness = \"not-applicable\"\nrange_low = 0\nrange_high = 0\nendian = \"big\"\n\
         [mapping.string_storage]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.string_storage.value]\nstorage = \"inline-fixed\"\n\
         byte_capacity = 40\nencoding = \"byte-transparent\"\npadding = \"nul-fill\"\n\
         over_capacity = \"reject\"\n",
    );
    assert!(found.contains(&Rule::StringCapacityConflict), "{found:?}");
}

#[test]
fn an_inline_string_must_state_a_budget_and_a_pooled_one_must_not() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.first_name\"\ntable = \"ubPc\"\nfield = \"HdeP\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"measured\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.string_storage]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.string_storage.value]\nstorage = \"inline-fixed\"\n\
         encoding = \"byte-transparent\"\npadding = \"nul-fill\"\nover_capacity = \"reject\"\n",
    );
    assert!(found.contains(&Rule::StringCapacityConflict), "{found:?}");

    let found = rules(
        "[[mapping]]\ncanonical = \"player.first_name\"\ntable = \"ubPc\"\nfield = \"HdeP\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"measured\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.string_storage]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.string_storage.value]\nstorage = \"varchar-pool\"\n\
         byte_capacity = 40\nencoding = \"utf8\"\npadding = \"none\"\nover_capacity = \"truncate\"\n",
    );
    assert!(found.contains(&Rule::StringCapacityConflict), "{found:?}");
}

#[test]
fn string_storage_on_a_non_string_property_is_refused() {
    let found = rules(&format!(
        "{}[mapping.string_storage]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.string_storage.value]\nstorage = \"varchar-pool\"\n\
         encoding = \"utf8\"\npadding = \"none\"\nover_capacity = \"truncate\"\n",
        mapping_head("player.shoots")
    ));
    assert!(found.contains(&Rule::ConstraintKindMismatch), "{found:?}");
}

// ---- references, applicability, composites ---------------------------------------

#[test]
fn every_table_reference_must_resolve_to_a_declaration() {
    // A mapping table, a foreign-key parent and a companion table that were never declared.
    let found = rules(
        "[[mapping]]\ncanonical = \"player.shoots\"\ntable = \"nOpE\"\nfield = \"GRkp\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"unknown\"\nqualification = \"unknown\"\n",
    );
    assert!(found.contains(&Rule::UndeclaredTable), "{found:?}");

    let found = rules(
        "[[mapping]]\ncanonical = \"player.team\"\ntable = \"ubPc\"\nfield = \"dXSB\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.foreign_key]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.foreign_key.value]\nparent_table = \"nOpE\"\n\
         parent_key_field = \"vfEq\"\ndeclaration = \"implicit-by-name\"\n",
    );
    assert!(found.contains(&Rule::UndeclaredTable), "{found:?}");
}

#[test]
fn a_table_cannot_be_declared_twice() {
    let text = "schema = \"target-adapter/2\"\nadapter_version = 2\ncanonical_schema_version = 1\n\
                target = \"nhl15-xbox360\"\n[[table]]\nid = \"ubPc\"\nname = \"a\"\n\
                provenance = [\"x\"]\n[[table]]\nid = \"ubPc\"\nname = \"b\"\nprovenance = [\"x\"]\n";
    let found: Vec<Rule> = validate_adapter(&parse_adapter(text).expect("parses"), &schema())
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(
        found.contains(&Rule::DuplicateTableDeclaration),
        "{found:?}"
    );
}

#[test]
fn a_table_declaration_must_cite_evidence() {
    let text = "schema = \"target-adapter/2\"\nadapter_version = 2\ncanonical_schema_version = 1\n\
                target = \"nhl15-xbox360\"\n[[table]]\nid = \"ubPc\"\nname = \"a\"\n";
    let found: Vec<Rule> = validate_adapter(&parse_adapter(text).expect("parses"), &schema())
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(found.contains(&Rule::MissingProvenance), "{found:?}");
}

#[test]
fn an_incomplete_applicability_predicate_is_refused() {
    // target-predicate with no field, and no codes.
    let found = rules(&format!(
        "{}[mapping.applicability]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.applicability.value]\nkind = \"target-predicate\"\n\
         table = \"ubPc\"\n",
        mapping_head("player.shoots")
    ));
    assert_eq!(
        found
            .iter()
            .filter(|r| **r == Rule::ApplicabilityIncomplete)
            .count(),
        2,
        "{found:?}"
    );

    // all-records carrying operands it cannot have.
    let found = rules(&format!(
        "{}[mapping.applicability]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.applicability.value]\nkind = \"all-records\"\ncodes = [1]\n",
        mapping_head("player.shoots")
    ));
    assert!(found.contains(&Rule::ApplicabilityIncomplete), "{found:?}");
}

#[test]
fn an_applicability_predicate_on_an_undefined_canonical_property_is_refused() {
    let found = rules(&format!(
        "{}[mapping.applicability]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.applicability.value]\nkind = \"canonical-predicate\"\n\
         property = \"player.no_such_thing\"\ncodes = [1]\n",
        mapping_head("player.shoots")
    ));
    assert!(found.contains(&Rule::UnknownCanonicalProperty), "{found:?}");
}

#[test]
fn a_composite_mapping_cannot_also_be_a_single_field_mapping() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.birth_date\"\ntable = \"ubPc\"\nfield = \"JKLp\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"unknown\"\nqualification = \"unknown\"\n\
         [[mapping.component]]\nrole = \"month\"\nfield = \"JKLp\"\n",
    );
    assert!(
        found.contains(&Rule::CompositeIdentityConflict),
        "{found:?}"
    );
}

#[test]
fn a_composite_mapping_may_not_duplicate_a_role_or_carry_mapping_level_storage() {
    let found = rules(
        "[[mapping]]\ncanonical = \"player.birth_date\"\ntable = \"ubPc\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"unknown\"\nqualification = \"unknown\"\n\
         [[mapping.component]]\nrole = \"month\"\nfield = \"JKLp\"\n\
         [[mapping.component]]\nrole = \"month\"\nfield = \"Kswi\"\n",
    );
    assert!(
        found.contains(&Rule::CompositeIdentityConflict),
        "{found:?}"
    );

    let found = rules(
        "[[mapping]]\ncanonical = \"player.birth_date\"\ntable = \"ubPc\"\n\
         read_transform = \"identity\"\nwrite_transform = \"identity\"\n\
         confidence = \"asserted\"\nqualification = \"implemented\"\nprovenance = [\"x\"]\n\
         [mapping.storage]\nstate = \"established\"\nconfidence = \"measured\"\nprovenance = [\"x\"]\n\
         [mapping.storage.value]\nkind = \"fixed-bit-uint\"\nbit_offset = 0\nbit_width = 4\n\
         signedness = \"unsigned\"\nrange_low = 0\nrange_high = 15\nendian = \"big\"\n\
         [[mapping.component]]\nrole = \"month\"\nfield = \"JKLp\"\n",
    );
    assert!(
        found.contains(&Rule::CompositeIdentityConflict),
        "{found:?}"
    );
}

// ---- capacity, companions, omissions ---------------------------------------------

#[test]
fn a_capacity_ceiling_below_its_population_is_refused() {
    let found = rules(
        "[[capacity]]\ntable = \"ubPc\"\n[capacity.declared]\nstate = \"established\"\n\
         confidence = \"derived\"\nprovenance = [\"x\"]\n[capacity.declared.value]\n\
         ceiling_records = 100\npopulation_records = 500\nsource = \"main-db-max-records\"\n",
    );
    assert!(found.contains(&Rule::CapacityBelowPopulation), "{found:?}");
}

#[test]
fn a_capacity_source_outside_the_vocabulary_is_refused() {
    let found = rules(
        "[[capacity]]\ntable = \"ubPc\"\n[capacity.declared]\nstate = \"established\"\n\
         confidence = \"derived\"\nprovenance = [\"x\"]\n[capacity.declared.value]\n\
         ceiling_records = 500\npopulation_records = 100\nsource = \"i-guessed-it\"\n",
    );
    assert!(found.contains(&Rule::UnknownVocabularyTerm), "{found:?}");
}

#[test]
fn a_table_cannot_have_two_capacity_declarations() {
    let one = "[[capacity]]\ntable = \"ubPc\"\n[capacity.declared]\nstate = \"unknown\"\n";
    let found = rules(&format!("{one}{one}"));
    assert!(
        found.contains(&Rule::DuplicateCapacityDeclaration),
        "{found:?}"
    );
}

#[test]
fn a_position_conditioned_companion_needs_a_position_discriminator() {
    let found = rules(
        "[[companion_rule]]\nprimary_table = \"ubPc\"\n[companion_rule.declared]\n\
         state = \"established\"\nconfidence = \"measured\"\nprovenance = [\"x\"]\n\
         [companion_rule.declared.value]\nshared_key_field = \"wBIz\"\n\
         [[companion_rule.declared.value.companion]]\ntable = \"kOtt\"\n\
         cardinality = \"one-to-one\"\napplies_when = \"position-skater\"\n",
    );
    assert!(
        found.contains(&Rule::PositionDiscriminatorMissing),
        "{found:?}"
    );
}

#[test]
fn overlapping_or_empty_position_codes_are_refused() {
    let found = rules(
        "[position_discriminator]\nstate = \"established\"\nconfidence = \"derived\"\n\
         provenance = [\"x\"]\n[position_discriminator.value]\ntable = \"ubPc\"\nfield = \"vjla\"\n\
         skater_codes = [0, 1, 4]\ngoalie_codes = [4]\n",
    );
    assert!(found.contains(&Rule::PositionCodeConflict), "{found:?}");

    let found = rules(
        "[position_discriminator]\nstate = \"established\"\nconfidence = \"derived\"\n\
         provenance = [\"x\"]\n[position_discriminator.value]\ntable = \"ubPc\"\nfield = \"vjla\"\n\
         skater_codes = []\ngoalie_codes = [4]\n",
    );
    assert!(found.contains(&Rule::PositionCodeConflict), "{found:?}");
}

#[test]
fn a_companion_table_cannot_be_listed_twice_in_one_rule() {
    let found = rules(
        "[[companion_rule]]\nprimary_table = \"ubPc\"\n[companion_rule.declared]\n\
         state = \"established\"\nconfidence = \"measured\"\nprovenance = [\"x\"]\n\
         [companion_rule.declared.value]\nshared_key_field = \"wBIz\"\n\
         [[companion_rule.declared.value.companion]]\ntable = \"kOtt\"\n\
         cardinality = \"one-to-one\"\napplies_when = \"always\"\n\
         [[companion_rule.declared.value.companion]]\ntable = \"kOtt\"\n\
         cardinality = \"one-to-one\"\napplies_when = \"always\"\n",
    );
    assert!(found.contains(&Rule::DuplicateCompanionTable), "{found:?}");
}

#[test]
fn a_primary_table_cannot_have_two_companion_rules() {
    let one = "[[companion_rule]]\nprimary_table = \"ubPc\"\n[companion_rule.declared]\n\
               state = \"unknown\"\n";
    let found = rules(&format!("{one}{one}"));
    assert!(found.contains(&Rule::DuplicateCompanionRule), "{found:?}");
}

#[test]
fn claiming_a_property_is_target_absent_requires_proof() {
    let found = rules(
        "[[omission]]\ncanonical = \"player.rating_overall\"\nreason = \"target-absent\"\n\
         detail = \"trust me\"\n",
    );
    assert!(found.contains(&Rule::OmissionWithoutProof), "{found:?}");

    // An admission of ignorance needs no proof — it claims nothing.
    let found = rules(
        "[[omission]]\ncanonical = \"player.rating_overall\"\n\
         reason = \"unknown-field-identity\"\n",
    );
    assert!(!found.contains(&Rule::OmissionWithoutProof), "{found:?}");
}

#[test]
fn an_omission_reason_outside_the_vocabulary_is_refused() {
    let found =
        rules("[[omission]]\ncanonical = \"player.rating_overall\"\nreason = \"we-skipped-it\"\n");
    assert!(found.contains(&Rule::UnknownVocabularyTerm), "{found:?}");
}

#[test]
fn a_default_cannot_be_offered_for_a_field_that_cannot_be_written() {
    let found = rules(&format!(
        "{}default = \"0\"\n[mapping.status]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.status.value]\nkey = true\nupdatable = true\n\
         structural = false\n",
        mapping_head("player.shoots")
    ));
    assert!(
        found.contains(&Rule::DefaultOnNonEditableField),
        "{found:?}"
    );
}

// ---- C0-CAN1-R1: evidence strength ------------------------------------------------

#[test]
fn an_asserted_constraint_cannot_be_operative() {
    let found = rules(&format!(
        "{}[mapping.status]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[mapping.status.value]\nkey = false\nupdatable = true\n\
         structural = false\n",
        mapping_head("player.shoots")
    ));
    assert!(found.contains(&Rule::AssertedAsOperative), "{found:?}");
    assert!(
        OPERATIVE_CONFIDENCE.contains(&"measured") && OPERATIVE_CONFIDENCE.contains(&"derived"),
        "measured and derived stay operative"
    );
    assert!(
        !OPERATIVE_CONFIDENCE.contains(&"asserted"),
        "asserted does not"
    );

    // The same claim in the candidate channel is accepted, and is not operative.
    let ok = rules(&format!(
        "{}[mapping.status]\nstate = \"unknown\"\n[mapping.status.candidate]\n\
         confidence = \"asserted\"\nprovenance = [\"x\"]\n[mapping.status.candidate.value]\n\
         key = false\nupdatable = true\nstructural = false\n",
        mapping_head("player.shoots")
    ));
    assert_eq!(ok, vec![], "{ok:?}");
}

#[test]
fn a_candidate_on_a_decided_constraint_is_refused() {
    for (state, extra) in [
        (
            "established",
            "confidence = \"measured\"\nprovenance = [\"x\"]\n[mapping.status.value]\n\
             key = false\nupdatable = true\nstructural = false\n",
        ),
        ("not-applicable", "reason = \"x\"\n"),
    ] {
        let found = rules(&format!(
            "{}[mapping.status]\nstate = \"{state}\"\n{extra}\
             [mapping.status.candidate]\nconfidence = \"asserted\"\nprovenance = [\"x\"]\n\
             [mapping.status.candidate.value]\nkey = false\nupdatable = true\nstructural = false\n",
            mapping_head("player.shoots")
        ));
        assert!(
            found.contains(&Rule::CandidateOnDecidedConstraint),
            "{state}: {found:?}"
        );
    }
}

#[test]
fn a_candidate_without_a_value_confidence_or_citation_is_refused() {
    let found = rules(&format!(
        "{}[mapping.status]\nstate = \"unknown\"\n[mapping.status.candidate]\n",
        mapping_head("player.shoots")
    ));
    assert_eq!(
        found
            .iter()
            .filter(|r| **r == Rule::CandidateIncomplete)
            .count(),
        3,
        "no value, no provenance, no confidence: {found:?}"
    );

    // `unknown` is not a candidate confidence — silence is spelled by omitting the block.
    let found = rules(&format!(
        "{}[mapping.status]\nstate = \"unknown\"\n[mapping.status.candidate]\n\
         confidence = \"unknown\"\nprovenance = [\"x\"]\n[mapping.status.candidate.value]\n\
         key = false\nupdatable = true\nstructural = false\n",
        mapping_head("player.shoots")
    ));
    assert!(found.contains(&Rule::UnknownVocabularyTerm), "{found:?}");
}

#[test]
fn an_unresolved_transform_cannot_be_qualified_verified_or_defaulted() {
    // Unresolved on one side only.
    let found = rules(
        "[[mapping]]\ncanonical = \"player.team\"\ntable = \"ubPc\"\nfield = \"dXSB\"\n\
         read_transform = \"unresolved\"\nwrite_transform = \"identity\"\n\
         confidence = \"measured\"\nqualification = \"blocked\"\nprovenance = [\"x\"]\n",
    );
    assert!(
        found.contains(&Rule::UnresolvedTransformConflict),
        "{found:?}"
    );

    // Unresolved, yet claiming a proven inverse, a named case and a default.
    let found = rules(
        "[[mapping]]\ncanonical = \"player.team\"\ntable = \"ubPc\"\nfield = \"dXSB\"\n\
         read_transform = \"unresolved\"\nwrite_transform = \"unresolved\"\n\
         inverse_verified = true\ndefault = \"0\"\nconfidence = \"measured\"\n\
         qualification = \"automated-tested\"\nautomated_case = \"x::y\"\nprovenance = [\"x\"]\n",
    );
    assert_eq!(
        found
            .iter()
            .filter(|r| **r == Rule::UnresolvedTransformConflict)
            .count(),
        3,
        "verified inverse, automated-tested and a default all refused: {found:?}"
    );

    // Unresolved names beside an established structured transform.
    let found = rules(
        "[[mapping]]\ncanonical = \"player.team\"\ntable = \"ubPc\"\nfield = \"dXSB\"\n\
         read_transform = \"unresolved\"\nwrite_transform = \"unresolved\"\n\
         confidence = \"measured\"\nqualification = \"blocked\"\nprovenance = [\"x\"]\n\
         [mapping.transform]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.transform.value]\nkind = \"identity\"\n",
    );
    assert!(
        found.contains(&Rule::UnresolvedTransformConflict),
        "{found:?}"
    );
}

#[test]
fn migration_does_not_promote_candidate_claims() {
    for name in ["v1/nhl15-xbox360.toml", "v1/nhl10-xbox360.toml"] {
        let source = parse_adapter_v1(&fixture(name)).expect("parses");
        let upgraded = upgrade_v1(&source);
        for mapping in &upgraded.adapter.mappings {
            for (block, present) in [
                ("storage", mapping.storage.candidate.is_some()),
                ("status", mapping.status.candidate.is_some()),
                ("transform", mapping.transform.candidate.is_some()),
                ("enumeration", mapping.enumeration.candidate.is_some()),
                ("null_sentinel", mapping.null_sentinel.candidate.is_some()),
                ("string_storage", mapping.string_storage.candidate.is_some()),
                ("foreign_key", mapping.foreign_key.candidate.is_some()),
                ("applicability", mapping.applicability.candidate.is_some()),
            ] {
                assert!(
                    !present,
                    "{name}/{}: the transition must not manufacture a [{block}.candidate] out of a \
                     v1 field — a candidate is an authored proposal, not a migration artifact",
                    mapping.canonical
                );
            }
        }
        // Nor may it invent an unresolved-transform spelling: v1 transform names survive verbatim.
        for (before, after) in source.mappings.iter().zip(&upgraded.adapter.mappings) {
            assert_eq!(before.read_transform, after.read_transform, "{name}");
            assert_eq!(before.write_transform, after.write_transform, "{name}");
        }
    }
}

// ---- C0-CAN1-R2: target-specific derivation boundaries ----------------------------

/// A stored-code vocabulary confirmed on one title is a candidate on every other title, however
/// well the field ids and widths match. Matching layout is a resemblance, not an equivalence bridge.
#[test]
fn a_cross_title_only_enum_is_a_candidate_not_a_derivation() {
    for name in COMMITTED_ADAPTERS {
        let a = adapter(name);
        for mapping in &a.mappings {
            assert!(
                mapping.enumeration.established().is_none(),
                "{name}/{}: the only enum vocabularies available are anchored on NHL Legacy and \
                 claim no scope beyond it",
                mapping.canonical
            );
        }
        let shoots = a
            .mappings
            .iter()
            .find(|m| m.canonical == "player.shoots")
            .expect("mapped");
        // The proposal survives with its citation.
        let candidate = shoots
            .enumeration
            .candidate_value()
            .expect("the Legacy-anchored mapping is recorded");
        assert_eq!(candidate.entries.len(), 2, "{name}");
        assert_eq!(candidate.entries[0].canonical, "left", "{name}");

        // And the operative surface claims nothing: an enum whose codes are unproven cannot be
        // decoded, so the transform names say so and the mapping is blocked.
        assert!(transform_unresolved(shoots), "{name}");
        assert_eq!(shoots.qualification, "blocked", "{name}");
        assert!(shoots.transform.established().is_none(), "{name}");

        // The independently supported physical facts are untouched — established, or (attribution
        // unresolved) carried as the candidate of an unknown block.
        let unattributed = non_operative(name);
        let storage = if unattributed {
            shoots.storage.candidate_value()
        } else {
            shoots.storage.established()
        }
        .expect("{name}: the bit layout is measured on this target and stays");
        assert_eq!(storage.bit_width, 1);
        assert_eq!((storage.range_low, storage.range_high), (0, 1));
        assert_eq!(
            shoots.status.established().is_some(),
            !unattributed,
            "{name}"
        );
    }
}

/// An unproven discriminator must not drive position-specific applicability or companion
/// requirements — including on the target whose companion audit does cover it. C0-REG1a-R1: the
/// Legacy adapter's discriminator is measured on its own anchor (legacy_xbox360_tests), so there
/// the dependent constraints ARE operative — and nowhere else.
#[test]
fn an_unproven_discriminator_does_not_drive_dependent_constraints() {
    // Measured on their own anchors (legacy_xbox360_tests, nhl15_ps3_tests — C0-REG1b; the ten
    // target_batch_tests modules — C0-REG2 / C0-REG3): there the dependent constraints ARE operative.
    for name in MEASURED_ON_OWN_ANCHOR {
        let measured = adapter(name);
        assert!(measured
            .position_discriminator
            .as_ref()
            .unwrap()
            .established()
            .is_some());
        assert!(measured
            .companion_rules
            .iter()
            .all(|r| r.declared.established().is_some()));
        let predicated: Vec<&str> = measured
            .mappings
            .iter()
            .filter(|m| {
                m.applicability
                    .established()
                    .is_some_and(|a| a.kind == "target-predicate")
            })
            .map(|m| m.canonical.as_str())
            .collect();
        // The skater-companion ratings this adapter maps, and only those: NHL 11 / 12 map no
        // stored overall, so there the list is the speed rating alone.
        // C0-REG5: gen6 NHL 07 keeps its team names on the stock team table reached through
        // the thin `kOtt` index, so its `team.name` mapping is predicated on `kOtt.table` too.
        let expected: Vec<&str> = ["player.rating_overall", "player.rating_speed", "team.name"]
            .into_iter()
            .filter(|c| {
                measured.mappings.iter().any(|m| m.canonical == *c)
                    && (*c != "team.name" || name == "nhl07-xbox360.toml")
            })
            .collect();
        assert!(expected.contains(&"player.rating_speed"), "{name}");
        assert_eq!(
            predicated, expected,
            "{name}: the skater-companion ratings, and only those, are position-conditioned"
        );
    }

    for name in ["nhl15-xbox360.toml", "nhl10-xbox360.toml"] {
        let a = adapter(name);

        // The discriminator itself: proposal kept, nothing operative.
        let discriminator = a
            .position_discriminator
            .as_ref()
            .expect("registered rather than absent");
        assert!(discriminator.established().is_none(), "{name}");
        let proposal = discriminator
            .candidate_value()
            .expect("the Legacy-anchored code assignment is recorded");
        assert_eq!(proposal.skater_codes, vec![0, 1, 2, 3], "{name}");
        assert_eq!(proposal.goalie_codes, vec![4], "{name}");

        // No companion rule is operative on either target.
        for rule in &a.companion_rules {
            assert!(
                rule.declared.established().is_none(),
                "{name}: a position-conditioned companion rule cannot be evaluated without an \
                 established discriminator"
            );
            let companions = rule
                .declared
                .candidate_value()
                .expect("the rule is recorded, not dropped");
            assert_eq!(companions.shared_key_field, "wBIz", "{name}");
            assert_eq!(companions.companions.len(), 4, "{name}");
        }

        // No position-conditioned applicability is operative either.
        for mapping in &a.mappings {
            if let Some(applicability) = mapping.applicability.established() {
                assert_eq!(
                    applicability.kind, "all-records",
                    "{name}/{}: only an unconditional applicability can be operative while the \
                     position codes are unproven",
                    mapping.canonical
                );
            }
        }
    }
}

/// The two targets reach the same non-operative state for different reasons, and each says which.
/// NHL 15's companion rule is blocked on a dependency; NHL 10's is unproven outright.
#[test]
fn the_two_companion_rules_record_different_reasons_for_being_candidates() {
    let fifteen = adapter("nhl15-xbox360.toml");
    let ten = adapter("nhl10-xbox360.toml");

    // NHL 15 is inside the audit's stated range, so the rule itself is derived — it is simply not
    // yet evaluable.
    let fifteen_rule = &fifteen.companion_rules[0].declared;
    assert_eq!(
        fifteen_rule
            .candidate
            .as_ref()
            .unwrap()
            .confidence
            .as_deref(),
        Some("derived")
    );
    assert!(fifteen_rule
        .candidate
        .as_ref()
        .unwrap()
        .note
        .contains("blocked on a dependency"));

    // NHL 10 is outside it. A shared key declaration is not evidence of cardinality, mandatory
    // presence or a skater/goalie condition, so the rule is merely asserted here.
    let ten_rule = &ten.companion_rules[0].declared;
    assert_eq!(
        ten_rule.candidate.as_ref().unwrap().confidence.as_deref(),
        Some("asserted")
    );
    let note = &ten_rule.candidate.as_ref().unwrap().note;
    assert!(note.contains("excludes this title"), "{note}");
}

/// Derivations that rest on premises applicable to this target stay operative. This is the other
/// half of the rule: it removes analogies, not conclusions.
#[test]
fn a_target_supported_derivation_stays_usable() {
    for name in COMMITTED_ADAPTERS {
        let a = adapter(name);

        // The rating offset regime: the audit scopes its 6-bit/offset-36 row to NHL 10 -> Legacy /
        // 11-21 by name, and the field's 6-bit range is measured here.
        let rating = a
            .mappings
            .iter()
            .find(|m| m.canonical == "player.rating_speed")
            .expect("mapped");
        if non_operative(name) {
            // C0-REG4-R1 / C0-REG5: the bytes are unattributed (or a research copy), so the derivation — sound on the bytes — is
            // a candidate and nothing is usable: no established transform, no human range, no
            // established capacity or status, and the mapping is blocked.
            assert_eq!(rating.qualification, "blocked", "{name}");
            assert!(transform_unresolved(rating), "{name}");
            assert!(rating.transform.established().is_none(), "{name}");
            let candidate = rating.transform.candidate_value().expect("the measurement");
            let expected_kind = if RESEARCH_COPIES.contains(&name) {
                "linear-offset"
            } else {
                "identity"
            };
            assert_eq!(candidate.kind, expected_kind, "{name}");
            assert_eq!(human_range(rating), None, "{name}");
            assert!(a
                .capacities
                .iter()
                .all(|c| c.declared.established().is_none()));
            assert!(
                a.capacities
                    .iter()
                    .all(|c| c.declared.candidate_value().is_some()
                        != RESEARCH_COPIES.contains(&name))
            );
            assert!(a.mappings.iter().all(|m| m.status.established().is_none()));
            continue;
        }
        if name == "nhl10-ps3.toml" || name == "nhl10-xbox360.toml" {
            // C0-REG3: on the NHL 10 PS3 anchor the audit's by-name rule and the production probe
            // the audit designates as the offset's source (`sixbit_rating_offset`) disagree — no
            // skater/goalie 6-bit value exceeds 49 there, so the probe resolves 50. A derivation
            // whose own premises conflict on the target is not operative; the adapter records the
            // disagreement as a blocked mapping with the probe's result as a candidate (see
            // `target_batch_tests::nhl10_ps3`). C0-AS4 measured the same on the byte-identical
            // Xbox 360 file and quarantined `nhl10-xbox360` the same way
            // (`nhl10_ratings_conflict_tests`).
            assert_eq!(rating.qualification, "blocked", "{name}");
            assert!(rating.transform.established().is_none(), "{name}");
            let candidate = rating
                .transform
                .candidate
                .as_ref()
                .expect("the probe's result");
            assert_eq!(candidate.value.as_ref().unwrap().offset, Some(50), "{name}");
            assert_eq!(human_range(rating), None, "{name}");
        } else if name == "nhl07-xbox360.toml" {
            // C0-REG5: the gen6 regime — the audit names NHL07 gen6 under 50 by title and the
            // probe resolves 50 on the bytes (no 6-bit value above 49).
            let transform = rating.transform.established().expect("gen6 regime");
            assert_eq!(transform.offset, Some(50), "{name}");
            assert_eq!(human_range(rating), Some([50, 113]), "{name}");
        } else if EARLY_ADAPTERS.contains(&name) {
            // C0-REG5: NHL 08 / 09 are named by no audit row; the probe's result is a candidate.
            assert_eq!(rating.qualification, "blocked", "{name}");
            assert!(rating.transform.established().is_none(), "{name}");
            let candidate = rating
                .transform
                .candidate_value()
                .expect("the probe's result");
            let expected = if name.starts_with("nhl08") { 36 } else { 50 };
            assert_eq!(candidate.offset, Some(expected), "{name}");
            assert_eq!(human_range(rating), None, "{name}");
        } else if GEN8_ADAPTERS.contains(&name) {
            // C0-REG4: the PS4 ratings are 7-bit 0..99 fields whose stored value IS the rating —
            // the audit's PS4 row and the engine's (0, 99) → offset 0 classification, over the
            // measured 7-bit range and the stored values (which reach 99) on each anchor. The
            // gen7 offset-36 rule was not carried here; `target_ps4_tests::*::seven_bit_ratings…`
            // measures each anchor.
            let transform = rating
                .transform
                .established()
                .expect("a platform-scoped audit rule over a measured premise stays operative");
            assert_eq!(
                (transform.kind.as_str(), transform.offset),
                ("identity", None),
                "{name}"
            );
            assert_eq!(human_range(rating), Some([0, 99]), "{name}");
        } else {
            let transform = rating
                .transform
                .established()
                .expect("a title-scoped audit rule over a measured premise stays operative");
            assert_eq!(transform.offset, Some(36), "{name}");
            assert_eq!(human_range(rating), Some([36, 99]), "{name}");
        }

        // Capacity: measured per title, bridged across platforms by a stated, verified identity —
        // except on Legacy, where C0-REG1a-R1 read the target's own ROSTER fixture (8 tables, the
        // team table included) and no bridge is needed. C0-REG4: the PS4 adapters read the eight
        // tables off their own main anchor (`main-db-max-records`), never off a save.
        let capacities = a
            .capacities
            .iter()
            .filter(|c| c.declared.established().is_some())
            .count();
        let expected = if name.starts_with("nhl07") || name.starts_with("nhl08") {
            // C0-REG5: the ROSTER fixture decodes to a TDB without the roster core (NHL 08, NHL 07
            // Xbox 360) or none is committed; every capacity is unknown, none substituted.
            0
        } else if MEASURED_ON_OWN_ANCHOR.contains(&name) {
            8
        } else {
            7
        };
        assert_eq!(capacities, expected, "{name}");
        let expected_source = if GEN8_ADAPTERS.contains(&name) {
            "main-db-max-records"
        } else {
            "roster-save-max-records"
        };
        for c in a
            .capacities
            .iter()
            .filter(|c| c.declared.established().is_some())
        {
            assert_eq!(
                c.declared.established().unwrap().source,
                expected_source,
                "{name}/{}: the ceiling is read from the file its generation exposes it in",
                c.table
            );
        }

        // Field status: measured flags plus an audit whose matrix covers this target.
        assert!(
            a.mappings
                .iter()
                .filter(|m| m.status.established().is_some())
                .count()
                >= 5,
            "{name}"
        );
    }
}

/// Item 3: a confidence label and a citation string are not an argument. Every derived block in
/// these adapters records its premises, their applicability to this target, and its limits.
#[test]
fn every_derived_block_records_its_premises() {
    let mut checked = 0;
    for name in COMMITTED_ADAPTERS {
        let a = adapter(name);
        for mapping in &a.mappings {
            for (label, state, derivation) in [
                (
                    "storage",
                    mapping.storage.confidence.as_deref(),
                    &mapping.storage.derivation,
                ),
                (
                    "status",
                    mapping.status.confidence.as_deref(),
                    &mapping.status.derivation,
                ),
                (
                    "transform",
                    mapping.transform.confidence.as_deref(),
                    &mapping.transform.derivation,
                ),
                (
                    "applicability",
                    mapping.applicability.confidence.as_deref(),
                    &mapping.applicability.derivation,
                ),
                (
                    "string_storage",
                    mapping.string_storage.confidence.as_deref(),
                    &mapping.string_storage.derivation,
                ),
            ] {
                if state == Some("derived") {
                    checked += 1;
                    assert!(
                        derivation.contains("PREMISES") && derivation.contains("CONCLUSION"),
                        "{name}/{}: [{label}] must state its premises and what follows",
                        mapping.canonical
                    );
                    assert!(
                        derivation.contains("NOT ESTABLISHED"),
                        "{name}/{}: [{label}] must state what the argument does NOT establish",
                        mapping.canonical
                    );
                    assert!(
                        derivation.contains("this target") || derivation.contains("THIS target"),
                        "{name}/{}: [{label}] must say why its premises apply to this target",
                        mapping.canonical
                    );
                }
            }
        }
        for capacity in &a.capacities {
            if capacity.declared.confidence.as_deref() == Some("derived") {
                checked += 1;
                let derivation = &capacity.declared.derivation;
                assert!(derivation.contains("PREMISES"), "{name}/{}", capacity.table);
                assert!(
                    derivation.contains("equivalence bridge"),
                    "{name}/{}: a cross-platform carry must name its bridge",
                    capacity.table
                );
                assert!(
                    derivation.contains("NOT ESTABLISHED"),
                    "{name}/{}",
                    capacity.table
                );
            }
        }
    }
    assert!(checked >= 30, "{checked} derived blocks audited");
}

#[test]
fn a_derived_constraint_must_record_its_premises() {
    let found = rules(&format!(
        "{}[mapping.status]\nstate = \"established\"\nconfidence = \"derived\"\n\
         provenance = [\"x\"]\n[mapping.status.value]\nkey = false\nupdatable = true\n\
         structural = false\n",
        mapping_head("player.shoots")
    ));
    assert!(found.contains(&Rule::DerivationUnexplained), "{found:?}");

    // A measured block needs no argument — it is not a conclusion.
    let ok = rules(&format!(
        "{}[mapping.status]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[mapping.status.value]\nkey = false\nupdatable = true\n\
         structural = false\n",
        mapping_head("player.shoots")
    ));
    assert_eq!(ok, vec![], "{ok:?}");
}

// ---- rule coverage ----------------------------------------------------------------

#[test]
fn every_rule_has_a_stable_diagnosable_name() {
    let all = [
        Rule::UnsupportedSchema,
        Rule::UnsupportedVersion,
        Rule::UnknownCanonicalProperty,
        Rule::DuplicateMapping,
        Rule::MappedAndOmitted,
        Rule::InverseWriteMissing,
        Rule::InverseUnverifiedButQualified,
        Rule::QualifiedWithoutCase,
        Rule::UnknownDefault,
        Rule::MissingProvenance,
        Rule::UnknownConfidence,
        Rule::UnknownQualification,
        Rule::AdapterClaimsGameVerified,
        Rule::MalformedTarget,
        Rule::UnknownIdentityButQualified,
        Rule::PlaceholderAsValue,
        Rule::EstablishedWithoutValue,
        Rule::EstablishedWithoutConfidence,
        Rule::ValueWithoutEstablishedState,
        Rule::NotApplicableWithoutReason,
        Rule::UnknownVocabularyTerm,
        Rule::UndeclaredTable,
        Rule::DuplicateTableDeclaration,
        Rule::StorageBoundsInconsistent,
        Rule::EnumCodeConflict,
        Rule::EnumCodeOutOfRange,
        Rule::EnumValueNotInDomain,
        Rule::ConstraintKindMismatch,
        Rule::TransformDeclarationConflict,
        Rule::SentinelConflict,
        Rule::StringCapacityConflict,
        Rule::DuplicateCapacityDeclaration,
        Rule::CapacityBelowPopulation,
        Rule::DuplicateCompanionRule,
        Rule::DuplicateCompanionTable,
        Rule::PositionDiscriminatorMissing,
        Rule::PositionCodeConflict,
        Rule::DefaultOnNonEditableField,
        Rule::ApplicabilityIncomplete,
        Rule::OmissionWithoutProof,
        Rule::CompositeIdentityConflict,
        Rule::AssertedAsOperative,
        Rule::CandidateOnDecidedConstraint,
        Rule::CandidateIncomplete,
        Rule::UnresolvedTransformConflict,
        Rule::DerivationUnexplained,
    ];
    let names: BTreeSet<&str> = all.iter().map(Rule::as_str).collect();
    assert_eq!(names.len(), all.len(), "rule names are unique");
    assert!(names.iter().all(|n| !n.is_empty()));
}
