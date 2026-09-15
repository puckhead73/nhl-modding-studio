//! `asset-output-profile/4` (package C0-AS4): composite identities, resolution chains, the
//! `/3`-in-place compatibility rule, the shared-file write exclusion, and the one measurement the
//! jersey profile's composite candidate rests on.

use super::*;
use crate::asset_resolution::*;
use crate::target_evidence::{field, meta_field, read, repo_root, table};
use std::collections::{BTreeMap, BTreeSet};
use tdb_core::TdbFile;

fn profile(file: &str) -> AssetProfile {
    let text = std::fs::read_to_string(repo_root().join("defs/adapters/asset-profiles").join(file))
        .unwrap();
    parse_asset_profile(&text).unwrap_or_else(|e| panic!("{file}: {e}"))
}

fn target<'a>(p: &'a AssetProfile, id: &str) -> &'a TargetProfile {
    p.targets.iter().find(|t| t.target == id).unwrap()
}

fn rules(findings: &[Finding]) -> Vec<&'static str> {
    findings.iter().map(|f| f.rule.as_str()).collect()
}

/// A minimal `/4` document: one attributable corpus, an owned and a shared family, and a chain
/// whose envelope `head` supplies — so one fixture serves the established and the candidate cases.
fn synthetic(head: &str, extra: &str) -> String {
    synthetic_full(IDENTITY_ESTABLISHED, head, extra)
}

/// The composite identity block, established (the default) or swapped for another shape.
const IDENTITY_ESTABLISHED: &str = r#"[target_profile.identity]
state = "established"
confidence = "measured"
scope = "inspected-corpus-invariant"
force = "observed-source-format"
corpus = "copy"
provenance = ["synthetic:fixture"]
[target_profile.identity.value]
key = "composite"
source = "xjgw"
ambiguity = "reject"
[target_profile.identity.value.composite]
format = "{style}_{art}_{variant}"
[[target_profile.identity.value.composite.part]]
name = "style"
table = "xjgw"
field = "cVLR"
render = "decimal"
[[target_profile.identity.value.composite.part]]
name = "art"
table = "xjgw"
field = "GDQL"
render = "zero-padded:3"
[[target_profile.identity.value.composite.part]]
name = "variant"
table = "xjgw"
field = "FccT"
render = "decimal"
null_code = 31"#;

/// The same kind of composite as a candidate under an unknown identity.
const IDENTITY_CANDIDATE: &str = r#"[target_profile.identity]
state = "unknown"
[target_profile.identity.candidate]
confidence = "asserted"
provenance = ["synthetic:fixture"]
note = "PROPOSED, NOT OPERATIVE"
[target_profile.identity.candidate.value]
key = "composite"
source = "xjgw"
ambiguity = "reject"
[target_profile.identity.candidate.value.composite]
format = "{style}_{art}"
[[target_profile.identity.candidate.value.composite.part]]
name = "style"
table = "xjgw"
field = "cVLR"
render = "decimal"
[[target_profile.identity.candidate.value.composite.part]]
name = "art"
table = "xjgw"
field = "GDQL"
render = "decimal""#;

fn synthetic_full(identity: &str, head: &str, extra: &str) -> String {
    format!(
        r#"
schema = "asset-output-profile/4"
profile_version = 1
family = "assets-jerseys"
synthetic = true
note = "synthetic"

[[canonical_source]]
role = "thumb"
class = "raster"
interchange = "rgba8"
retention = "unmodified"

[[target_profile]]
target = "fixture-synthetic"

[[target_profile.corpus]]
id = "copy"
kind = "observed-installed-copy"
attribution = "attributable-to-target"
build_identification = "complete"
identity_source = "synthetic:header"
inspected_files = 10
groups = ["current"]
provenance = ["synthetic:fixture"]

[[target_profile.path_family]]
id = "own"
template = "fe/jerseys/jersey_{{composite-id}}.big"
tokens = ["composite-id"]
case = "lower"
bucket_resolution = "template-only"

[[target_profile.path_family]]
id = "default"
template = "fe/jerseys/custom.big"
case = "lower"
bucket_resolution = "template-only"

{identity}

[[target_profile.resolution_chain]]
id = "thumb"
purpose = "read-write"
[target_profile.resolution_chain.declared]
{head}
[[target_profile.resolution_chain.declared{value}.step]]
family = "own"
ownership = "owned"
[[target_profile.resolution_chain.declared{value}.step]]
family = "default"
ownership = "shared"

[[target_profile.operation]]
operation = "install"
requires = ["install", "resolution_chain:thumb"]
{extra}
"#,
        identity = identity,
        head = head,
        value = if head.contains("candidate") {
            ".candidate.value"
        } else {
            ".value"
        },
        extra = extra
    )
}

const ESTABLISHED: &str = r#"state = "established"
confidence = "measured"
scope = "inspected-corpus-invariant"
force = "observed-source-format"
corpus = "copy"
provenance = ["synthetic:fixture"]"#;

const CANDIDATE: &str = r#"state = "unknown"
[target_profile.resolution_chain.declared.candidate]
confidence = "asserted"
provenance = ["synthetic:fixture"]
note = "PROPOSED, NOT OPERATIVE""#;

fn parse_ok(text: &str) -> AssetProfile {
    parse_asset_profile(text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

// ============================================================ compatibility

/// The two committed `/3` profiles stay `/3` and validate in place with no finding; the jersey
/// profile moved to `/4`. Nothing in a `/3` document is read differently.
#[test]
fn the_committed_profiles_validate_under_their_own_schemas() {
    for (file, schema) in [
        ("assets-portraits.toml", ASSET_PROFILE_SCHEMA_V3),
        ("assets-logos.toml", ASSET_PROFILE_SCHEMA_V3),
        ("assets-jerseys.toml", ASSET_PROFILE_SCHEMA),
    ] {
        let p = profile(file);
        assert_eq!(p.schema, schema, "{file}");
        let findings = validate_asset_profile(&p);
        assert_eq!(findings, vec![], "{file}: {findings:?}");
    }
    assert_eq!(
        ASSET_PROFILE_SCHEMAS_NATIVE,
        [ASSET_PROFILE_SCHEMA, ASSET_PROFILE_SCHEMA_V3]
    );
}

/// A `/3` document that uses a `/4` construct is refused as such — not read under `/4`'s meaning.
#[test]
fn a_v3_document_may_not_use_v4_constructs() {
    let text =
        synthetic(ESTABLISHED, "").replace("asset-output-profile/4", "asset-output-profile/3");
    let p = parse_ok(&text);
    let findings = validate_asset_profile(&p);
    let schema: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.rule == Rule::UnsupportedSchema)
        .collect();
    assert_eq!(schema.len(), 3, "{findings:?}");
    let details: Vec<&str> = schema.iter().map(|f| f.detail.as_str()).collect();
    assert!(details.iter().any(|d| d.contains("[[resolution_chain]]")));
    assert!(details
        .iter()
        .any(|d| d.contains("[identity.value.composite]")));
    assert!(details.iter().any(|d| d.contains("\"composite-id\"")));
    // The same document under `/4` is clean.
    let p = parse_ok(&synthetic(ESTABLISHED, ""));
    assert_eq!(validate_asset_profile(&p), vec![]);
}

/// The `/1` and `/2` transitions still land on the current schema string.
#[test]
fn the_older_transitions_upgrade_to_the_current_schema() {
    assert_eq!(ASSET_PROFILE_SCHEMA, "asset-output-profile/4");
    assert_eq!(ASSET_PROFILE_SCHEMA_V3, "asset-output-profile/3");
    assert_eq!(ASSET_PROFILE_SCHEMA_PREVIOUS, "asset-output-profile/2");
}

// ====================================================== composite construction

fn part(name: &str, render: &str, null_code: Option<i64>) -> CompositePart {
    CompositePart {
        name: name.into(),
        table: "xjgw".into(),
        field: name.into(),
        render: render.into(),
        null_code,
        note: String::new(),
    }
}

#[test]
fn a_composite_renders_each_part_under_its_declared_rendering() {
    let c = CompositeIdentity {
        format: "{style}_{art}_{variant}".into(),
        parts: vec![
            part("style", "decimal", None),
            part("art", "zero-padded:3", None),
            part("variant", "lower-hex", None),
        ],
        note: String::new(),
    };
    let id = render_composite(&c, |p| match p.name.as_str() {
        "style" => PartValue::Code(1),
        "art" => PartValue::Code(19),
        _ => PartValue::Code(26),
    })
    .unwrap();
    assert_eq!(id, "1_019_1a");
    assert_eq!(format_placeholders(&c.format), ["style", "art", "variant"]);
    assert_eq!(slugify("Jean-Sébastien Giguère"), "jean_s_bastien_gigu_re");
    assert_eq!(
        render_part(
            &part("n", "slug", None),
            &PartValue::Text("O'Reilly".into())
        )
        .unwrap(),
        "o_reilly"
    );
}

/// Every missing or unrenderable part is a typed blocker; nothing is substituted.
#[test]
fn a_missing_or_unrenderable_part_blocks_construction() {
    let c = CompositeIdentity {
        format: "{a}_{b}".into(),
        parts: vec![
            part("a", "decimal", None),
            part("b", "zero-padded:2", Some(31)),
        ],
        note: String::new(),
    };
    let run = |a: PartValue, b: PartValue| {
        render_composite(
            &c,
            move |p| if p.name == "a" { a.clone() } else { b.clone() },
        )
    };
    assert_eq!(
        run(PartValue::Absent, PartValue::Code(1)),
        Err(CompositeBlocker::MissingPart { part: "a".into() })
    );
    assert_eq!(
        run(PartValue::Code(1), PartValue::Code(31)),
        Err(CompositeBlocker::NullPart {
            part: "b".into(),
            code: 31
        })
    );
    assert!(matches!(
        run(PartValue::Code(1), PartValue::Code(100)),
        Err(CompositeBlocker::UnrenderablePart { part, .. }) if part == "b"
    ));
    assert!(matches!(
        run(PartValue::Text("x".into()), PartValue::Code(1)),
        Err(CompositeBlocker::UnrenderablePart { part, .. }) if part == "a"
    ));
    assert!(matches!(
        render_part(&part("s", "slug", None), &PartValue::Text("--".into())),
        Err(CompositeBlocker::UnrenderablePart { .. })
    ));
    assert!(matches!(
        render_part(&part("s", "octal", None), &PartValue::Code(1)),
        Err(CompositeBlocker::InvalidDeclaration { .. })
    ));
    // A 0 with no declared null code is a value.
    assert_eq!(
        run(PartValue::Code(0), PartValue::Code(0)),
        Ok("0_00".into())
    );
    assert_eq!(parse_render("zero-padded:0"), None);
    assert_eq!(parse_render("zero-padded"), None);
    assert_eq!(
        parse_render("zero-padded:4"),
        Some(("zero-padded", Some(4)))
    );
}

#[test]
fn collisions_group_the_rows_that_render_one_id() {
    let rows = [
        (0u32, "a".to_string()),
        (1, "b".into()),
        (2, "a".into()),
        (3, "a".into()),
    ];
    let c = collisions(rows);
    assert_eq!(c, BTreeMap::from([("a".to_string(), vec![0, 2, 3])]));
    assert!(collisions([(1u8, "x".to_string()), (2, "y".into())]).is_empty());
}

// ============================================================= path rendering

fn family(id: &str, template: &str, tokens: &[&str]) -> PathFamily {
    PathFamily {
        id: id.into(),
        template: template.into(),
        tokens: tokens.iter().map(|t| t.to_string()).collect(),
        case: "lower".into(),
        bucket_resolution: "template-only".into(),
        observed_buckets: Vec::new(),
        note: String::new(),
    }
}

#[test]
fn a_rendered_path_is_refused_before_it_could_escape_a_root() {
    let f = family(
        "own",
        "fe/jerseys/jersey_{composite-id}.big",
        &["composite-id"],
    );
    let ok = render_path(&f, &BTreeMap::from([("composite-id", "1_19_3")])).unwrap();
    assert_eq!(ok, "fe/jerseys/jersey_1_19_3.big");
    for bad in ["../x", "a/b", "a\\b", "c:", "", ".", "..", "x\u{0}y"] {
        assert!(
            matches!(
                render_path(&f, &BTreeMap::from([("composite-id", bad)])),
                Err(PathBlocker::UnsafeTokenValue { .. })
            ),
            "{bad:?}"
        );
    }
    assert_eq!(
        render_path(&f, &BTreeMap::new()),
        Err(PathBlocker::MissingToken {
            token: "composite-id".into()
        })
    );
    // The convention of `safe_join` / `safe_join_rel`, restated on the string.
    for bad in ["/abs", "\\abs", "c:/x", "a/../b", "..", "", "a/\u{7}"] {
        assert!(!is_safe_relative_path(bad), "{bad:?}");
    }
    for good in ["a", "a/b.c", "a//b", "./a", "fe\\ion\\x.big"] {
        assert!(is_safe_relative_path(good), "{good:?}");
    }
    let literal = family("default", "fe/jerseys/../custom.big", &[]);
    assert!(matches!(
        render_path(&literal, &BTreeMap::new()),
        Err(PathBlocker::UnsafePath { .. })
    ));
}

// ============================================================ chain resolution

#[test]
fn a_read_follows_the_declared_order_and_a_write_never_reaches_a_shared_step() {
    let p = parse_ok(&synthetic(ESTABLISHED, ""));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let t = target(&p, "fixture-synthetic");
    let chain = &t.resolution_chains[0];
    assert!(chain.declared.is_operative());
    let tokens = BTreeMap::from([("composite-id", "1_019_3")]);
    let own = "fe/jerseys/jersey_1_019_3.big";
    let shared = "fe/jerseys/custom.big";

    // Read: the owned file is missing, the shared default exists → found on the shared step.
    let found = resolve_read(t, chain, &tokens, |path| path == shared).unwrap();
    assert_eq!(
        found,
        ReadResolution::Found {
            step: 1,
            family: "default".into(),
            path: shared.into(),
            shared: true
        }
    );
    assert_eq!(
        resolve_read(t, chain, &tokens, |_| false).unwrap(),
        ReadResolution::NotFound {
            looked_for: vec![own.into(), shared.into()]
        }
    );
    // Write, same state: the read fell back to the shared file; the write does NOT. The owned file
    // is missing and this target establishes no [install], so creation is a blocker naming what
    // is missing — not a write to `custom.big`.
    assert_eq!(
        resolve_write(t, chain, &tokens, |path| path == shared),
        Err(WriteBlocker::CreationUnsupported {
            path: own.into(),
            missing: vec!["install", "container", "references"]
        })
    );
    // Owned file present: the write replaces it, whatever the shared file's state.
    assert_eq!(
        resolve_write(t, chain, &tokens, |path| path == own || path == shared),
        Ok(WriteResolution::Existing {
            step: 0,
            family: "own".into(),
            path: own.into()
        })
    );
    assert_eq!(
        CreationSupport::of(t),
        CreationSupport {
            install: false,
            container: false,
            references: false
        }
    );
    assert_eq!(
        (shared_families(t), owned_families(t)),
        (vec!["default".to_string()], vec!["own".to_string()])
    );
}

fn shared_families(t: &TargetProfile) -> Vec<String> {
    ownership_sets(t).0.into_iter().collect()
}
fn owned_families(t: &TargetProfile) -> Vec<String> {
    ownership_sets(t).1.into_iter().collect()
}

/// Creation of a missing owned file is authorised only when install, container and references are
/// established; the fixture's target-level blocks decide that, not the chain.
#[test]
fn a_missing_owned_file_is_created_only_with_established_support() {
    let extra = r#"
[target_profile.install]
state = "established"
confidence = "measured"
scope = "inspected-corpus-invariant"
force = "observed-source-format"
corpus = "copy"
provenance = ["synthetic:fixture"]
[target_profile.install.value]
kind = "loose-file"
size_policy = "allow-growth"
repack = "not-required"
backup = "original-bytes-retained"
updates_references = false

[target_profile.container]
state = "not-applicable"
reason = "synthetic: a loose file has no container"
"#;
    let p = parse_ok(&synthetic(ESTABLISHED, extra));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let t = target(&p, "fixture-synthetic");
    assert_eq!(
        CreationSupport::of(t),
        CreationSupport {
            install: true,
            container: true,
            references: true
        }
    );
    let tokens = BTreeMap::from([("composite-id", "1_019_3")]);
    assert_eq!(
        resolve_write(t, &t.resolution_chains[0], &tokens, |_| false),
        Ok(WriteResolution::Create {
            step: 0,
            family: "own".into(),
            path: "fe/jerseys/jersey_1_019_3.big".into()
        })
    );
    // With references to rewrite and no identity, creation is refused again.
    let p2 = parse_ok(&synthetic_full(
        IDENTITY_CANDIDATE,
        ESTABLISHED,
        &extra.replace("updates_references = false", "updates_references = true"),
    ));
    assert_eq!(validate_asset_profile(&p2), vec![]);
    let t2 = target(&p2, "fixture-synthetic");
    assert!(t2.identity.established().is_none());
    assert_eq!(
        CreationSupport::of(t2),
        CreationSupport {
            install: true,
            container: true,
            references: false
        }
    );
    assert!(matches!(
        resolve_write(t2, &t2.resolution_chains[0], &tokens, |_| false),
        Err(WriteBlocker::CreationUnsupported { missing, .. }) if missing == ["references"]
    ));
}

/// A candidate chain resolves nothing for a write, enables no operation, and does not make a
/// `runtime_lookup` requirement met. Chain order is not runtime behaviour.
#[test]
fn a_candidate_chain_enables_nothing_and_runtime_lookup_stays_required() {
    let p = parse_ok(&synthetic(CANDIDATE, ""));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let t = target(&p, "fixture-synthetic");
    let chain = &t.resolution_chains[0];
    assert!(!chain.declared.is_operative());
    let tokens = BTreeMap::from([("composite-id", "1_019_3")]);
    assert_eq!(resolve_read(t, chain, &tokens, |_| true), None);
    assert_eq!(
        resolve_write(t, chain, &tokens, |_| true),
        Err(WriteBlocker::ChainNotOperative)
    );
    let install = t
        .operations
        .iter()
        .find(|o| o.operation == "install")
        .unwrap();
    assert_eq!(
        operation_constraint_readiness(t, None, install),
        OperationReadiness::BlockedByUnknownConstraints(vec![
            "install".into(),
            "resolution_chain:thumb".into()
        ])
    );
    // Established chain, operation additionally requiring runtime_lookup: the chain does not
    // stand in for it.
    let p = parse_ok(&synthetic(ESTABLISHED, "").replace(
        r#"requires = ["install", "resolution_chain:thumb"]"#,
        r#"requires = ["runtime_lookup", "resolution_chain:thumb"]"#,
    ));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let t = target(&p, "fixture-synthetic");
    let install = t
        .operations
        .iter()
        .find(|o| o.operation == "install")
        .unwrap();
    assert_eq!(
        operation_constraint_readiness(t, None, install),
        OperationReadiness::BlockedByUnknownConstraints(vec!["runtime_lookup".into()])
    );
    let population = asset_registry_population(std::iter::once(&p));
    assert!(population.is_empty(), "synthetic profiles are excluded");
}

#[test]
fn a_read_only_chain_refuses_writes_and_a_write_chain_needs_an_owned_step() {
    let p = parse_ok(
        &synthetic(ESTABLISHED, "").replace("purpose = \"read-write\"", "purpose = \"read\""),
    );
    let findings = validate_asset_profile(&p);
    assert_eq!(
        rules(&findings),
        ["asset-resolution-chain-invalid"],
        "{findings:?}"
    );
    assert!(findings[0].detail.contains("whose purpose is \"read\""));
    let t = target(&p, "fixture-synthetic");
    assert_eq!(
        resolve_write(
            t,
            &t.resolution_chains[0],
            &BTreeMap::from([("composite-id", "1_001_1")]),
            |_| true
        ),
        Err(WriteBlocker::ChainNotForWriting {
            purpose: "read".into()
        })
    );
    // All-shared write chain.
    let p = parse_ok(
        &synthetic(ESTABLISHED, "").replace("ownership = \"owned\"", "ownership = \"shared\""),
    );
    let findings = validate_asset_profile(&p);
    assert!(
        findings
            .iter()
            .any(|f| f.detail.contains("has no owned step")),
        "{findings:?}"
    );
    let t = target(&p, "fixture-synthetic");
    assert_eq!(
        resolve_write(
            t,
            &t.resolution_chains[0],
            &BTreeMap::from([("composite-id", "1_001_1")]),
            |_| true
        ),
        Err(WriteBlocker::NoOwnedStep)
    );
}

/// A shared family cannot be relabelled owned through a second chain, and two families with one
/// template are refused once chains exist.
#[test]
fn a_shared_file_does_not_become_writable_through_an_alias_or_another_chain() {
    let alias = r#"
[[target_profile.resolution_chain]]
id = "other"
purpose = "write"
[target_profile.resolution_chain.declared]
state = "unknown"
[target_profile.resolution_chain.declared.candidate]
confidence = "asserted"
provenance = ["synthetic:fixture"]
note = "PROPOSED, NOT OPERATIVE"
[[target_profile.resolution_chain.declared.candidate.value.step]]
family = "default"
ownership = "owned"
"#;
    let p = parse_ok(&synthetic(ESTABLISHED, alias));
    let findings = validate_asset_profile(&p);
    assert!(
        findings.iter().any(|f| f.rule == Rule::AssetResolutionChainInvalid
            && f.detail.contains("\"default\" is a shared step in one resolution chain and an owned step in another")),
        "{findings:?}"
    );
    let dup = r#"
[[target_profile.path_family]]
id = "default-alias"
template = "fe/jerseys/custom.big"
case = "lower"
bucket_resolution = "template-only"
"#;
    let p = parse_ok(&synthetic(ESTABLISHED, dup));
    let findings = validate_asset_profile(&p);
    assert!(
        findings
            .iter()
            .any(|f| f.rule == Rule::AssetResolutionChainInvalid
                && f.detail.contains("share the template")),
        "{findings:?}"
    );
}

#[test]
fn chain_declarations_are_validated_for_families_ids_purposes_and_requirements() {
    let cases: [(&str, &str, &str); 6] = [
        (
            "family = \"default\"\nownership = \"shared\"",
            "family = \"nowhere\"\nownership = \"shared\"",
            "does not declare",
        ),
        (
            "purpose = \"read-write\"",
            "purpose = \"append\"",
            "[resolution_chain].purpose",
        ),
        (
            "ownership = \"shared\"",
            "ownership = \"borrowed\"",
            "step.ownership",
        ),
        (
            "id = \"thumb\"\npurpose",
            "id = \"thumb2\"\npurpose",
            "requires resolution chain \"thumb\", which this target does not declare",
        ),
        (
            "\"resolution_chain:thumb\"",
            "\"resolution_chain:other\"",
            "requires resolution chain \"other\"",
        ),
        (
            "family = \"default\"\nownership = \"shared\"",
            "family = \"own\"\nownership = \"owned\"",
            "twice",
        ),
    ];
    for (from, to, expect) in cases {
        let text = synthetic(ESTABLISHED, "").replacen(from, to, 1);
        let p = parse_ok(&text);
        let findings = validate_asset_profile(&p);
        assert!(
            findings.iter().any(|f| f.detail.contains(expect)),
            "{from:?} -> {to:?}: {findings:?}"
        );
    }
    // A duplicate chain id.
    let text = synthetic(ESTABLISHED, &format!("[[target_profile.resolution_chain]]\nid = \"thumb\"\npurpose = \"read\"\n[target_profile.resolution_chain.declared]\n{CANDIDATE}\n[[target_profile.resolution_chain.declared.candidate.value.step]]\nfamily = \"own\"\nownership = \"owned\"\n"));
    let findings = validate_asset_profile(&parse_ok(&text));
    assert!(
        findings.iter().any(|f| f.detail.contains("share an id")),
        "{findings:?}"
    );
    // An empty chain.
    let text = synthetic(ESTABLISHED, "").replace("[[target_profile.resolution_chain.declared.value.step]]\nfamily = \"own\"\nownership = \"owned\"\n[[target_profile.resolution_chain.declared.value.step]]\nfamily = \"default\"\nownership = \"shared\"\n", "[target_profile.resolution_chain.declared.value]\n");
    let findings = validate_asset_profile(&parse_ok(&text));
    assert!(
        findings
            .iter()
            .any(|f| f.detail.contains("declares no [[step]]")),
        "{findings:?}"
    );
}

#[test]
fn composite_declarations_are_validated_for_parts_placeholders_renderings_and_keys() {
    let cases: [(&str, &str, &str); 7] = [
        (
            "format = \"{style}_{art}_{variant}\"",
            "format = \"{style}_{art}_{colour}\"",
            "{colour}, which no part declares",
        ),
        (
            "format = \"{style}_{art}_{variant}\"",
            "format = \"{style}_{art}\"",
            "part \"variant\", which the format never uses",
        ),
        (
            "name = \"variant\"",
            "name = \"style\"",
            "is declared twice",
        ),
        (
            "render = \"zero-padded:3\"",
            "render = \"zero-padded:99\"",
            "is not one of",
        ),
        (
            "key = \"composite\"",
            "key = \"numeric-id\"",
            "needs key \"composite\"",
        ),
        ("name = \"art\"", "name = \"Art\"", "part names are"),
        (
            "format = \"{style}_{art}_{variant}\"",
            "format = \"{style}_{art}_{variant}_{style}\"",
            "twice",
        ),
    ];
    for (from, to, expect) in cases {
        let p = parse_ok(&synthetic(ESTABLISHED, "").replacen(from, to, 1));
        let findings = validate_asset_profile(&p);
        assert!(
            findings
                .iter()
                .any(|f| f.rule == Rule::AssetCompositeInvalid && f.detail.contains(expect)),
            "{from:?} -> {to:?}: {findings:?}"
        );
    }
    // Key composite with no composite table.
    let text = synthetic(ESTABLISHED, "");
    let start = text
        .find("[target_profile.identity.value.composite]")
        .unwrap();
    let end = text.find("[[target_profile.resolution_chain]]").unwrap();
    let p = parse_ok(&format!("{}{}", &text[..start], &text[end..]));
    let findings = validate_asset_profile(&p);
    assert!(
        findings
            .iter()
            .any(|f| f.detail.contains("no [composite] table")),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|f| f
            .detail
            .contains("declares no composite (neither established nor as a candidate)")),
        "{findings:?}"
    );
    // A `composite-id` template on an identity established under another key is a dependency
    // finding on the path rule, as `player-slug` is.
    let extra = r#"
[target_profile.path_rule]
state = "established"
confidence = "measured"
scope = "inspected-corpus-invariant"
force = "observed-source-format"
corpus = "copy"
provenance = ["synthetic:fixture"]
[target_profile.path_rule.value]
families = ["own", "default"]
family_selection = "by-chain"
"#;
    let p = parse_ok(&synthetic(ESTABLISHED, extra));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let p = parse_ok(&synthetic_full(IDENTITY_CANDIDATE, CANDIDATE, extra));
    let findings = validate_asset_profile(&p);
    assert!(
        findings
            .iter()
            .any(|f| f.rule == Rule::AssetConstraintDependencyUnmet
                && f.detail
                    .contains("\"composite-id\", but [identity] is not established")),
        "{findings:?}"
    );
}

// ======================================================= the Legacy measurement

/// The composite the jersey profile proposes, measured on the committed Legacy Xbox 360 anchor
/// with the `/4` construction functions themselves: field identities from EA's metadata, stored
/// ranges, no all-ones pattern, 698 distinct ids over 703 rows with exactly three shared tuples,
/// `teamartid` equal to the owning team's `kOtt.lOnr` on every row, the nine zero art ids all on
/// team key 0, and the three path collisions that follow under both composite templates. What this
/// does NOT measure: that any rendered id names a file. The profile's candidate cites this case.
#[test]
fn legacy_x360_xjgw_composite_tuples_are_measured_not_assumed_unique() {
    let main = "anchors/xbox360/nhllegacy/databases/nhlng.db";
    let bytes = std::fs::read(repo_root().join(main)).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let anchors = tdb_qa::discover_main_anchors(&repo_root()).unwrap();
    let anchor = anchors
        .iter()
        .find(|a| a.platform == "xbox360" && a.game == "nhllegacy")
        .unwrap();
    let snap = tdb_qa::schema::snapshot(&repo_root(), anchor).unwrap();

    let p = profile("assets-jerseys.toml");
    let t = target(&p, "nhllegacy-xbox360");
    let identity = t
        .identity
        .candidate_value()
        .expect("the composite is proposed");
    assert_eq!(
        (identity.key.as_str(), identity.ambiguity.as_str()),
        ("composite", "reject")
    );
    let composite = identity.composite.as_ref().unwrap();
    assert_eq!(composite.format, "{style}_{art}_{variant}");
    assert!(composite.parts.iter().all(|p| p.null_code.is_none()));

    let xjgw = table(&db, "xjgw");
    assert_eq!(xjgw.current_records, 703);
    // Field identities from the metadata, and stored ranges with no all-ones pattern anywhere.
    let expected_names = BTreeMap::from([
        ("cVLR", "style"),
        ("GDQL", "teamartid"),
        ("FccT", "variant"),
    ]);
    let mut fields = BTreeMap::new();
    for part in &composite.parts {
        assert_eq!(part.table, "xjgw");
        let f = field(&db, xjgw, &part.field);
        assert_eq!(
            meta_field(&snap, "xjgw", &part.field).name.as_deref(),
            Some(expected_names[part.field.as_str()])
        );
        let all_ones = (1u64 << f.bit_width) - 1;
        let (lo, hi, ones) = (0..703u32)
            .map(|i| read(&db, xjgw, i, &f))
            .fold((u64::MAX, 0, 0), |(lo, hi, ones), v| {
                (lo.min(v), hi.max(v), ones + usize::from(v == all_ones))
            });
        // A 1-bit flag's all-ones pattern is its value 1 (413 rows store it); the two wider
        // fields store their all-ones pattern on no row.
        if f.bit_width > 1 {
            assert_eq!(ones, 0, "{}", part.name);
        } else {
            assert_eq!(ones, 413, "{}", part.name);
        }
        let expected_range = match part.name.as_str() {
            "style" => (0, 1),
            "art" => (0, 421),
            _ => (0, 20),
        };
        assert_eq!((lo, hi), expected_range, "{}", part.name);
        fields.insert(part.name.clone(), f);
    }

    // Render every row through the `/4` construction function.
    let ids: Vec<(u32, String)> = (0..703u32)
        .map(|row| {
            let id = render_composite(composite, |part| {
                PartValue::Code(read(&db, xjgw, row, &fields[&part.name]))
            })
            .unwrap();
            (row, id)
        })
        .collect();
    let distinct: BTreeSet<&str> = ids.iter().map(|(_, id)| id.as_str()).collect();
    assert_eq!(distinct.len(), 698);
    let team = field(&db, xjgw, "mCXg");
    let teams_of = |rows: &[u32]| {
        rows.iter()
            .map(|r| read(&db, xjgw, *r, &team))
            .collect::<Vec<_>>()
    };
    let shared = collisions(ids.iter().cloned());
    assert_eq!(
        shared.keys().map(String::as_str).collect::<Vec<_>>(),
        ["1_19_3", "1_21_4", "1_255_0"]
    );
    assert_eq!(
        (shared["1_19_3"].clone(), teams_of(&shared["1_19_3"])),
        (vec![166, 695], vec![19, 216])
    );
    assert_eq!(
        (shared["1_21_4"].clone(), teams_of(&shared["1_21_4"])),
        (vec![181, 696], vec![21, 217])
    );
    assert_eq!(
        (shared["1_255_0"].clone(), teams_of(&shared["1_255_0"])),
        (vec![699, 700, 701, 702], vec![220, 220, 221, 221])
    );

    // teamartid is the owning team's art id on every row; the nine zero art ids are team key 0's.
    let kott = table(&db, "kOtt");
    let (key, lonr) = (field(&db, kott, "vfEq"), field(&db, kott, "lOnr"));
    let lonr_by_key: BTreeMap<u64, u64> = (0..u32::from(kott.current_records))
        .map(|i| (read(&db, kott, i, &key), read(&db, kott, i, &lonr)))
        .collect();
    let mut zero_art_teams = BTreeSet::new();
    for row in 0..703u32 {
        let (tm, art) = (
            read(&db, xjgw, row, &team),
            read(&db, xjgw, row, &fields["art"]),
        );
        assert_eq!(lonr_by_key.get(&tm), Some(&art), "row {row}");
        if art == 0 {
            zero_art_teams.insert(tm);
        }
    }
    assert_eq!(zero_art_teams, BTreeSet::from([0]));
    assert_eq!(
        ids.iter()
            .filter(|(_, id)| id.starts_with("1_0_") || id.starts_with("0_0_"))
            .count(),
        9
    );

    // The path collisions that follow, under both composite templates.
    for fam in ["outfit-texlib", "fe-thumbnail"] {
        let family = t.path_families.iter().find(|f| f.id == fam).unwrap();
        assert_eq!(family.tokens, ["composite-id"]);
        let paths = ids.iter().map(|(row, id)| {
            (
                *row,
                render_path(family, &BTreeMap::from([("composite-id", id.as_str())])).unwrap(),
            )
        });
        let path_collisions = collisions(paths);
        assert_eq!(path_collisions.len(), 3, "{fam}");
        assert!(
            path_collisions
                .keys()
                .any(|p| p.ends_with("_1_255_0.rx2") || p.ends_with("_1_255_0.big")),
            "{fam}"
        );
    }

    // The profile records the measurement, cites this case, and establishes nothing by it.
    let candidate = t.identity.candidate.as_ref().unwrap();
    assert!(
        candidate
            .provenance
            .iter()
            .any(|p| p
                .ends_with("#legacy_x360_xjgw_composite_tuples_are_measured_not_assumed_unique"))
    );
    assert_eq!(candidate.scope.as_deref(), Some("specimen-observation"));
    for s in ["698", "1_19_3", "1_21_4", "1_255_0", "lOnr", "NOT unique"] {
        assert!(candidate.note.contains(s), "{s}");
    }
    assert!(t.identity.established().is_none());
    assert!(t.path_rule.established().is_none());
    assert!(t
        .path_families
        .iter()
        .all(|f| f.bucket_resolution == "unknown"));
}

/// The jersey profile's chain: a candidate from working-tree source, labelled as such, enabling
/// nothing; and even if it were established, its shared steps could never be a write destination.
#[test]
fn the_jersey_thumbnail_chain_is_a_labelled_working_tree_candidate_that_enables_nothing() {
    let p = profile("assets-jerseys.toml");
    let t = target(&p, "nhllegacy-xbox360");
    assert_eq!(t.resolution_chains.len(), 1);
    let chain = &t.resolution_chains[0];
    assert_eq!(
        (chain.id.as_str(), chain.purpose.as_str()),
        ("kit-thumbnail", "read-write")
    );
    assert_eq!(chain.declared.state, ConstraintState::Unknown);
    assert!(!chain.declared.is_operative());
    let c = chain.declared.candidate.as_ref().unwrap();
    assert!(c
        .provenance
        .iter()
        .all(|p| p.starts_with("WORKING TREE ONLY")));
    assert!(c.note.contains("NOT carried here"));
    let steps: Vec<(&str, &str)> = c
        .value
        .as_ref()
        .unwrap()
        .steps
        .iter()
        .map(|s| (s.family.as_str(), s.ownership.as_str()))
        .collect();
    assert_eq!(
        steps,
        [
            ("fe-thumbnail", "owned"),
            ("fe-thumbnail-custom", "shared"),
            ("fe-thumbnail-customhome", "shared"),
            ("fe-thumbnail-customaway", "shared")
        ]
    );
    let (shared, owned) = ownership_sets(t);
    assert_eq!(shared.len(), 3);
    assert_eq!(owned, BTreeSet::from(["fe-thumbnail".to_string()]));

    let tokens = BTreeMap::from([("composite-id", "1_19_3")]);
    assert_eq!(resolve_read(t, chain, &tokens, |_| true), None);
    assert_eq!(
        resolve_write(t, chain, &tokens, |_| true),
        Err(WriteBlocker::ChainNotOperative)
    );
    let install = t
        .operations
        .iter()
        .find(|o| o.operation == "install")
        .unwrap();
    assert_eq!(
        operation_constraint_readiness(t, None, install),
        OperationReadiness::BlockedByUnknownConstraints(vec![
            "container".into(),
            "install".into(),
            "resolution_chain:kit-thumbnail".into(),
            "runtime_lookup".into(),
        ])
    );
    let population = asset_registry_population(std::iter::once(&p));
    assert_eq!(
        (
            population[0].declared_resolution_chains,
            population[0].operative_resolution_chains
        ),
        (1, 0)
    );
    assert_eq!(population[0].declared_path_families, 6);
}

// ================================================ C0-AS4-R1: resolved-path exclusion

/// The chain fixture with the owned template swapped so a token value can land on the shared
/// file, plus any extra text.
fn aliasing_fixture(owned_template: &str, extra: &str) -> AssetProfile {
    let text = synthetic(ESTABLISHED, extra).replace(
        "template = \"fe/jerseys/jersey_{composite-id}.big\"",
        &format!("template = \"{owned_template}\""),
    );
    let p = parse_ok(&text);
    assert_eq!(
        validate_asset_profile(&p),
        vec![],
        "static validation passes: the templates differ as text"
    );
    p
}

const SECOND_CHAIN_WITH_SHARED_STEP: &str = r#"
[[target_profile.path_family]]
id = "league-default"
template = "fe/jerseys/league.big"
case = "lower"
bucket_resolution = "template-only"

[[target_profile.resolution_chain]]
id = "league"
purpose = "read"
[target_profile.resolution_chain.declared]
state = "unknown"
[target_profile.resolution_chain.declared.candidate]
confidence = "asserted"
provenance = ["synthetic:fixture"]
note = "PROPOSED, NOT OPERATIVE"
[[target_profile.resolution_chain.declared.candidate.value.step]]
family = "own"
ownership = "owned"
[[target_profile.resolution_chain.declared.candidate.value.step]]
family = "league-default"
ownership = "shared"
"#;

/// DEFECT 1 (reproduced before the fix as `Ok(Existing)` on `fe/jerseys/custom.big`): an owned
/// template that differs from the shared one as text renders onto the same file for the supplied
/// token. The write resolver now compares rendered paths, not templates.
#[test]
fn an_owned_step_that_renders_onto_a_shared_destination_is_refused() {
    let p = aliasing_fixture("fe/jerseys/{composite-id}.big", "");
    let t = target(&p, "fixture-synthetic");
    let chain = &t.resolution_chains[0];
    let onto_shared = BTreeMap::from([("composite-id", "custom")]);
    for exists in [true, false] {
        assert_eq!(
            resolve_write(t, chain, &onto_shared, |_| exists),
            Err(WriteBlocker::AliasesSharedDestination {
                path: "fe/jerseys/custom.big".into(),
                shared_family: "default".into(),
                chain: "thumb".into(),
            }),
            "exists = {exists}: refused before Existing or Create"
        );
    }
    // A read through the same chain still finds the file — reads may fall back.
    assert!(matches!(
        resolve_read(t, chain, &onto_shared, |_| true),
        Some(ReadResolution::Found { step: 0, .. })
    ));
    // Distinct paths continue to resolve.
    let distinct = BTreeMap::from([("composite-id", "1_019_3")]);
    assert_eq!(
        resolve_write(t, chain, &distinct, |path| path == "fe/jerseys/1_019_3.big"),
        Ok(WriteResolution::Existing {
            step: 0,
            family: "own".into(),
            path: "fe/jerseys/1_019_3.big".into()
        })
    );
}

/// A shared declaration in ANOTHER chain — even a candidate, read-only one — excludes that
/// destination from every write on the target.
#[test]
fn a_shared_destination_declared_through_another_chain_is_excluded_too() {
    let p = aliasing_fixture(
        "fe/jerseys/{composite-id}.big",
        SECOND_CHAIN_WITH_SHARED_STEP,
    );
    let t = target(&p, "fixture-synthetic");
    let thumb = t
        .resolution_chains
        .iter()
        .find(|c| c.id == "thumb")
        .unwrap();
    assert_eq!(
        shared_steps(t),
        [
            ("thumb".to_string(), "default".to_string()),
            ("league".to_string(), "league-default".to_string())
        ]
    );
    assert_eq!(
        resolve_write(
            t,
            thumb,
            &BTreeMap::from([("composite-id", "league")]),
            |_| false
        ),
        Err(WriteBlocker::AliasesSharedDestination {
            path: "fe/jerseys/league.big".into(),
            shared_family: "league-default".into(),
            chain: "league".into(),
        })
    );
    // Static validation still refuses relabelling: the same family owned elsewhere is a finding.
    let p2 = parse_ok(&synthetic(
        ESTABLISHED,
        &SECOND_CHAIN_WITH_SHARED_STEP.replace(
            "family = \"league-default\"\nownership = \"shared\"",
            "family = \"default\"\nownership = \"owned\"",
        ),
    ));
    assert!(validate_asset_profile(&p2)
        .iter()
        .any(|f| f.rule == Rule::AssetResolutionChainInvalid));
}

/// Separator and case normalisation: `\\`/`/` and `./` segments are one path; a case-only
/// difference is decided by the families' declared casing, and undecidable — refused — when a
/// family preserves case.
#[test]
fn normalisation_aliases_are_refused_and_undecidable_case_is_not_guessed() {
    assert_eq!(
        normalize_path("fe\\jerseys\\.\\custom.big"),
        "fe/jerseys/custom.big"
    );
    assert_eq!(
        normalize_path("./fe//jerseys/custom.big"),
        "fe/jerseys/custom.big"
    );
    assert_eq!(paths_alias("a/b", "a\\b"), PathAlias::Same);
    // Any case-only difference is undetermined, whatever casing the families declared.
    assert_eq!(paths_alias("a/B", "a/b"), PathAlias::CaseUndetermined);
    assert_eq!(paths_alias("A/B", "a/b"), PathAlias::CaseUndetermined);
    assert_eq!(paths_alias("a/b", "a/c"), PathAlias::Distinct);

    // Both families lower their output, so `Custom` cannot survive rendering, and the backslash
    // template is the same destination as the slash one.
    // (TOML basic strings escape the backslash, hence the doubled ones in the template text.)
    let p = aliasing_fixture("fe\\\\jerseys\\\\{composite-id}.big", "");
    let t = target(&p, "fixture-synthetic");
    assert_eq!(
        resolve_write(
            t,
            &t.resolution_chains[0],
            &BTreeMap::from([("composite-id", "Custom")]),
            |_| true
        ),
        Err(WriteBlocker::AliasesSharedDestination {
            path: "fe\\jerseys\\custom.big".into(),
            shared_family: "default".into(),
            chain: "thumb".into(),
        })
    );

    // A case-preserving owned family against a lowering shared one: a case-only difference is
    // undetermined, and refused as such rather than treated as distinct. (The lower-vs-upper
    // case, where both families declare a casing, is the CLOSE regression below.)
    let text = synthetic(ESTABLISHED, "").replace(
        "template = \"fe/jerseys/jersey_{composite-id}.big\"\ntokens = [\"composite-id\"]\ncase = \"lower\"",
        "template = \"fe/jerseys/{composite-id}.big\"\ntokens = [\"composite-id\"]\ncase = \"preserve\"",
    );
    let p = parse_ok(&text);
    assert_eq!(validate_asset_profile(&p), vec![]);
    let t = target(&p, "fixture-synthetic");
    assert!(matches!(
        resolve_write(
            t,
            &t.resolution_chains[0],
            &BTreeMap::from([("composite-id", "Custom")]),
            |_| true
        ),
        Err(WriteBlocker::SharedDestinationUndetermined { shared_family, .. }) if shared_family == "default"
    ));

    // A shared family that needs a token the caller did not supply cannot be compared: refused.
    let text = synthetic(ESTABLISHED, "").replace(
        "template = \"fe/jerseys/custom.big\"\ncase = \"lower\"",
        "template = \"fe/jerseys/{team-id}/custom.big\"\ntokens = [\"team-id\"]\ncase = \"lower\"",
    );
    let p = parse_ok(&text);
    assert_eq!(validate_asset_profile(&p), vec![]);
    let t = target(&p, "fixture-synthetic");
    assert!(matches!(
        resolve_write(
            t,
            &t.resolution_chains[0],
            &BTreeMap::from([("composite-id", "1_019_3")]),
            |_| true
        ),
        Err(WriteBlocker::SharedDestinationUndetermined { reason, .. }) if reason.contains("\"team-id\"")
    ));
    // With the token supplied the two are distinct and the write resolves.
    assert!(matches!(
        resolve_write(
            t,
            &t.resolution_chains[0],
            &BTreeMap::from([("composite-id", "1_019_3"), ("team-id", "19")]),
            |_| true
        ),
        Ok(WriteResolution::Existing { .. })
    ));
}

// ============================================= C0-AS4-R1: creation prerequisites

const LOOSE_INSTALL: &str = r#"
[target_profile.install]
state = "established"
confidence = "measured"
scope = "inspected-corpus-invariant"
force = "observed-source-format"
corpus = "copy"
provenance = ["synthetic:fixture"]
[target_profile.install.value]
kind = "loose-file"
size_policy = "allow-growth"
repack = "not-required"
backup = "original-bytes-retained"
updates_references = false

[target_profile.container]
state = "not-applicable"
reason = "synthetic: a loose file has no container"
"#;

/// DEFECT 2 (reproduced before the fix as `Ok(Create)`): an install that must update references
/// was treated as creatable because `[identity]` was established. No reference-update capability
/// is represented, so creation stays blocked — with identity, install and container all
/// established.
#[test]
fn creation_requiring_reference_updates_is_blocked_even_with_established_identity() {
    let extra = LOOSE_INSTALL.replace("updates_references = false", "updates_references = true");
    let p = parse_ok(&synthetic(ESTABLISHED, &extra));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let t = target(&p, "fixture-synthetic");
    assert!(t.identity.is_operative() && t.install.is_operative());
    assert_eq!(t.container.state, ConstraintState::NotApplicable);
    assert_eq!(
        CreationSupport::of(t),
        CreationSupport {
            install: true,
            container: true,
            references: false
        }
    );
    let tokens = BTreeMap::from([("composite-id", "1_019_3")]);
    assert_eq!(
        resolve_write(t, &t.resolution_chains[0], &tokens, |_| false),
        Err(WriteBlocker::CreationUnsupported {
            path: "fe/jerseys/jersey_1_019_3.big".into(),
            missing: vec!["references"]
        })
    );
    // The existing owned file is still a replace destination: references are a creation matter.
    assert!(matches!(
        resolve_write(t, &t.resolution_chains[0], &tokens, |_| true),
        Ok(WriteResolution::Existing { .. })
    ));
}

/// Describing a format is not the capability to create in it: an archive install kind and an
/// established container each block creation by name; the loose-file case with every
/// prerequisite established still creates.
#[test]
fn described_archive_or_container_formats_do_not_imply_creation_support() {
    let tokens = BTreeMap::from([("composite-id", "1_019_3")]);
    // Supported: loose file, no container, no references.
    let p = parse_ok(&synthetic(ESTABLISHED, LOOSE_INSTALL));
    let t = target(&p, "fixture-synthetic");
    assert_eq!(
        resolve_write(t, &t.resolution_chains[0], &tokens, |_| false),
        Ok(WriteResolution::Create {
            step: 0,
            family: "own".into(),
            path: "fe/jerseys/jersey_1_019_3.big".into()
        })
    );
    let no_container = "[target_profile.container]\nstate = \"not-applicable\"\nreason = \"synthetic: a loose file has no container\"";
    let wrapped_container = "[target_profile.container]\nstate = \"established\"\nconfidence = \"measured\"\nscope = \"inspected-corpus-invariant\"\nforce = \"observed-source-format\"\ncorpus = \"copy\"\nprovenance = [\"synthetic:fixture\"]\n[target_profile.container.value]\noutermost_layer_examined = true\n[[target_profile.container.value.layer]]\nformat = \"bigf\"\nindex = \"name-table\"";
    // An archive install kind describes storage, not member insertion. (The validator requires an
    // established container for an archive kind, so this fixture describes both — and still
    // cannot create: both prerequisites are reported.)
    let archive = LOOSE_INSTALL
        .replace(
            "kind = \"loose-file\"",
            "kind = \"archive-replace-in-place\"",
        )
        .replace(
            "size_policy = \"allow-growth\"",
            "size_policy = \"fit-in-slot\"",
        )
        .replace(no_container, wrapped_container);
    let p = parse_ok(&synthetic(ESTABLISHED, &archive));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let t = target(&p, "fixture-synthetic");
    assert!(t.install.is_operative() && t.container.is_operative());
    assert!(matches!(
        resolve_write(t, &t.resolution_chains[0], &tokens, |_| false),
        Err(WriteBlocker::CreationUnsupported { missing, .. }) if missing == ["install", "container"]
    ));
    // An established container describes the wrapping an existing file has, not the ability to
    // produce a new wrapped payload.
    let wrapped = LOOSE_INSTALL.replace(no_container, wrapped_container);
    let p = parse_ok(&synthetic(ESTABLISHED, &wrapped));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let t = target(&p, "fixture-synthetic");
    assert!(t.container.is_operative());
    assert_eq!(
        CreationSupport::of(t),
        CreationSupport {
            install: true,
            container: false,
            references: true
        }
    );
    assert!(matches!(
        resolve_write(t, &t.resolution_chains[0], &tokens, |_| false),
        Err(WriteBlocker::CreationUnsupported { missing, .. }) if missing == ["container"]
    ));
}

/// C0-AS4-CLOSE: declared casing does not establish destination case sensitivity. An owned
/// `lower` family and a shared `upper` family render `fe/jerseys/custom.big` and
/// `FE/JERSEYS/CUSTOM.BIG` — spellings a case-insensitive destination treats as one file — and the
/// write is refused as undetermined whether the destination exists or not. A genuinely distinct
/// spelling still resolves.
#[test]
fn a_case_only_difference_between_lower_and_upper_families_is_refused_not_declared_distinct() {
    let text = synthetic(ESTABLISHED, "")
        .replace(
            "template = \"fe/jerseys/jersey_{composite-id}.big\"",
            "template = \"fe/jerseys/{composite-id}.big\"",
        )
        .replace(
            "template = \"fe/jerseys/custom.big\"\ncase = \"lower\"",
            "template = \"FE/JERSEYS/CUSTOM.BIG\"\ncase = \"upper\"",
        );
    let p = parse_ok(&text);
    assert_eq!(validate_asset_profile(&p), vec![], "a validated profile");
    let t = target(&p, "fixture-synthetic");
    let (own, shared) = (
        t.path_families.iter().find(|f| f.id == "own").unwrap(),
        t.path_families.iter().find(|f| f.id == "default").unwrap(),
    );
    assert_eq!(
        (own.case.as_str(), shared.case.as_str()),
        ("lower", "upper")
    );
    let chain = &t.resolution_chains[0];
    let tokens = BTreeMap::from([("composite-id", "Custom")]);
    assert_eq!(render_path(own, &tokens).unwrap(), "fe/jerseys/custom.big");
    assert_eq!(
        render_path(shared, &BTreeMap::new()).unwrap(),
        "FE/JERSEYS/CUSTOM.BIG"
    );
    for exists in [true, false] {
        assert!(
            matches!(
                resolve_write(t, chain, &tokens, |_| exists),
                Err(WriteBlocker::SharedDestinationUndetermined { ref path, ref shared_family, ref reason, .. })
                    if path == "fe/jerseys/custom.big" && shared_family == "default" && reason.contains("only in case")
            ),
            "exists = {exists}"
        );
    }
    // Distinct spelling: resolves, existing or to be created (creation support is absent here,
    // so the missing case is the named creation blocker, not an alias blocker).
    let distinct = BTreeMap::from([("composite-id", "1_019_3")]);
    assert_eq!(
        resolve_write(t, chain, &distinct, |_| true),
        Ok(WriteResolution::Existing {
            step: 0,
            family: "own".into(),
            path: "fe/jerseys/1_019_3.big".into()
        })
    );
    assert!(matches!(
        resolve_write(t, chain, &distinct, |_| false),
        Err(WriteBlocker::CreationUnsupported { .. })
    ));
}
