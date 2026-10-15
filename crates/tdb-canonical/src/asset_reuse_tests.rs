//! `asset-output-profile/5` (package C0-AS5): shared asset definitions by reference. Every case
//! runs the **effective** profile through the existing readiness / resolution APIs
//! (`operation_constraint_readiness`, `resolve_read` / `resolve_write`, `CreationSupport`,
//! `shared_steps` / `ownership_sets`, `asset_registry_population`); none stops at "the TOML parses".

use super::*;
use crate::asset_resolution::*;
use crate::asset_reuse::*;
use crate::target_evidence::repo_root;
use std::collections::BTreeMap;

fn profile(file: &str) -> AssetProfile {
    let text = std::fs::read_to_string(repo_root().join("defs/adapters/asset-profiles").join(file))
        .unwrap();
    parse_asset_profile(&text).unwrap_or_else(|e| panic!("{file}: {e}"))
}

fn parse_ok(text: &str) -> AssetProfile {
    parse_asset_profile(text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

fn declared<'a>(p: &'a AssetProfile, id: &str) -> &'a TargetProfile {
    p.targets.iter().find(|t| t.target == id).unwrap()
}

fn rules(findings: &[Finding]) -> Vec<Rule> {
    findings.iter().map(|f| f.rule.clone()).collect()
}

/// A `/5` document: a source target with an attributable corpus, an owned and a shared family,
/// an operative read-write chain, an `install` operation — plus whatever `extra` appends
/// (typically a reusing target).
fn synthetic(source_extra: &str, extra: &str) -> String {
    format!(
        r#"
schema = "asset-output-profile/5"
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
target = "fixture-source"

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
template = "fe/jerseys/{{variant}}.big"
tokens = ["variant"]
case = "lower"
bucket_resolution = "template-only"

[[target_profile.path_family]]
id = "default"
template = "fe/jerseys/custom.big"
case = "lower"
bucket_resolution = "template-only"

[target_profile.identity]
state = "unknown"
[target_profile.identity.candidate]
confidence = "asserted"
provenance = ["synthetic:fixture"]
note = "PROPOSED, NOT OPERATIVE"
[target_profile.identity.candidate.value]
key = "variant"
source = "xjgw"
ambiguity = "reject"

[[target_profile.resolution_chain]]
id = "thumb"
purpose = "read-write"
[target_profile.resolution_chain.declared]
state = "established"
confidence = "measured"
scope = "inspected-corpus-invariant"
force = "observed-source-format"
corpus = "copy"
provenance = ["synthetic:fixture"]
[[target_profile.resolution_chain.declared.value.step]]
family = "own"
ownership = "owned"
[[target_profile.resolution_chain.declared.value.step]]
family = "default"
ownership = "shared"

[[target_profile.operation]]
operation = "install"
requires = ["install", "resolution_chain:thumb"]

[[target_profile.operation]]
operation = "identify"
requires = ["identity"]
{source_extra}
{extra}
"#
    )
}

const SOURCE_LOOSE_INSTALL: &str = r#"
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

const REUSER: &str = r#"
[[target_profile]]
target = "fixture-reuser"
reuses = "fixture-source"
note = "synthetic: reuses the source's shared definitions"
"#;

/// A reusing target with its own destination evidence: an inspected corpus of its own and a
/// loose-file install established on it.
const REUSER_WITH_INSTALL: &str = r#"
[[target_profile]]
target = "fixture-reuser"
reuses = "fixture-source"
note = "synthetic: reuses the source's shared definitions, installs loosely"

[[target_profile.corpus]]
id = "destination-copy"
kind = "observed-installed-copy"
attribution = "attributable-to-target"
build_identification = "complete"
identity_source = "synthetic:header"
inspected_files = 3
groups = ["current"]
provenance = ["synthetic:fixture"]

[target_profile.install]
state = "established"
confidence = "measured"
scope = "inspected-corpus-invariant"
force = "observed-source-format"
corpus = "destination-copy"
provenance = ["synthetic:fixture"]
[target_profile.install.value]
kind = "loose-file"
size_policy = "allow-growth"
repack = "not-required"
backup = "original-bytes-retained"
updates_references = false
"#;

const SOURCE_CONTAINER: &str = r#"
[target_profile.container]
state = "established"
confidence = "measured"
scope = "inspected-corpus-invariant"
force = "observed-source-format"
corpus = "copy"
provenance = ["synthetic:fixture"]
[target_profile.container.value]
outermost_layer_examined = true
[[target_profile.container.value.layer]]
format = "bigf"
index = "name-table"
"#;

const DESTINATION_CORPUS: &str = r#"
[[target_profile.corpus]]
id = "destination-copy"
kind = "observed-installed-copy"
attribution = "attributable-to-target"
build_identification = "complete"
identity_source = "synthetic:header"
inspected_files = 3
groups = ["current"]
provenance = ["synthetic:fixture"]
"#;

const ARCHIVE_INSTALL: &str = r#"
[target_profile.install]
state = "established"
confidence = "measured"
scope = "inspected-corpus-invariant"
force = "observed-source-format"
corpus = "destination-copy"
provenance = ["synthetic:fixture"]
[target_profile.install.value]
kind = "archive-replace-in-place"
size_policy = "fit-in-slot"
repack = "index-update"
backup = "whole-container-retained"
updates_references = false
"#;

/// A reusing target with its own attributable corpus and an archive install established on it:
/// the install depends on a container the reuser INHERITS.
fn reuser_with_archive_install() -> String {
    format!(
        r#"
[[target_profile]]
target = "fixture-reuser"
reuses = "fixture-source"
note = "synthetic: reuses the source's shared definitions, installs into the inherited archive"
{DESTINATION_CORPUS}
{ARCHIVE_INSTALL}
"#
    )
}

// ============================================ validation after composition (AS5-CLOSE)

/// DEFECT (reproduced before the fix as `asset-constraint-dependency-unmet` on the reuser): the
/// raw reusing declaration was validated before the shared definitions were resolved, so an
/// archive install that depends on the INHERITED container was reported as lacking one. Sequenced
/// correctly — declaration-level checks on the local declaration, cross-field checks on the
/// effective profile — the combination is clean, and the effective profile is what the readiness
/// and creation APIs see.
#[test]
fn an_inherited_established_container_satisfies_a_local_archive_installation() {
    let p = parse_ok(&synthetic(SOURCE_CONTAINER, &reuser_with_archive_install()));
    let findings = validate_asset_profile(&p);
    assert_eq!(findings, vec![], "{findings:?}");
    let e = resolve_target(&p, "fixture-reuser").unwrap().profile;
    assert!(e.container.is_operative(), "inherited, in its source state");
    assert!(e.install.is_operative(), "the reuser's own");
    assert_eq!(e.install.corpus.as_deref(), Some("destination-copy"));
    assert_eq!(
        e.container.corpus.as_deref(),
        Some("copy"),
        "source provenance stays attached"
    );
    let install_op = e
        .operations
        .iter()
        .find(|o| o.operation == "install")
        .unwrap();
    assert_eq!(
        operation_constraint_readiness(&e, None, install_op),
        OperationReadiness::ConstraintsEstablished
    );
    // Describing an archive installation is not the capability to create an archive member:
    // `CreationSupport` is unchanged by this package.
    assert_eq!(
        CreationSupport::of(&e),
        CreationSupport {
            install: false,
            container: false,
            references: true
        }
    );
    let tokens = BTreeMap::from([("variant", "1_019_3")]);
    assert_eq!(
        resolve_write(&e, &e.resolution_chains[0], &tokens, |_| false),
        Err(WriteBlocker::CreationUnsupported {
            path: "fe/jerseys/1_019_3.big".into(),
            missing: vec!["install", "container"]
        })
    );
    assert!(matches!(
        resolve_write(&e, &e.resolution_chains[0], &tokens, |_| true),
        Ok(WriteResolution::Existing { .. })
    ));
}

/// With no container inherited (the source leaves it unknown, or only proposes one), the same
/// local archive install is still a dependency finding — on the effective profile, where the
/// dependency is decided.
#[test]
fn a_missing_inherited_container_still_fails_a_local_archive_installation() {
    let p = parse_ok(&synthetic("", &reuser_with_archive_install()));
    let findings = validate_asset_profile(&p);
    let dependency: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.rule == Rule::AssetConstraintDependencyUnmet)
        .collect();
    assert_eq!(dependency.len(), 1, "{findings:?}");
    assert!(dependency[0].subject.contains("fixture-reuser"));
    assert!(dependency[0]
        .detail
        .contains("[container] is not established"));
    let candidate = r#"
[target_profile.container]
state = "unknown"
[target_profile.container.candidate]
confidence = "asserted"
provenance = ["synthetic:fixture"]
note = "PROPOSED, NOT OPERATIVE"
[target_profile.container.candidate.value]
outermost_layer_examined = false
[[target_profile.container.candidate.value.layer]]
format = "bigf"
index = "name-table"
"#;
    let p = parse_ok(&synthetic(candidate, &reuser_with_archive_install()));
    assert!(rules(&validate_asset_profile(&p)).contains(&Rule::AssetConstraintDependencyUnmet));
}

/// Destination evidence stays the reuser's own: an established local install that cites the
/// SOURCE's corpus is refused — an inherited corpus speaks for the shared definitions, not for the
/// destination — and one that cites nothing is incomplete. A non-reusing target keeps exactly
/// its behaviour.
#[test]
fn a_source_corpus_cannot_substitute_for_destination_evidence() {
    let citing_source = format!(
        r#"
[[target_profile]]
target = "fixture-reuser"
reuses = "fixture-source"
{}
"#,
        ARCHIVE_INSTALL.replace("corpus = \"destination-copy\"", "corpus = \"copy\"")
    );
    let p = parse_ok(&synthetic(SOURCE_CONTAINER, &citing_source));
    let findings = validate_asset_profile(&p);
    assert!(
        rules(&findings).contains(&Rule::AssetCorpusUndeclared),
        "the source corpus is not destination evidence: {findings:?}"
    );
    assert!(
        !rules(&findings).contains(&Rule::AssetConstraintDependencyUnmet),
        "the inherited container is still seen: {findings:?}"
    );
    let uncited = citing_source.replace("corpus = \"copy\"\n", "");
    let p = parse_ok(&synthetic(SOURCE_CONTAINER, &uncited));
    assert_ne!(validate_asset_profile(&p), vec![]);
    // Non-reusing: an archive install with its own container and corpus is clean; without the
    // container it is the same dependency finding it always was.
    let own = synthetic(
        &format!(
            "{SOURCE_CONTAINER}\n{}",
            ARCHIVE_INSTALL.replace("destination-copy", "copy")
        ),
        "",
    );
    assert_eq!(validate_asset_profile(&parse_ok(&own)), vec![]);
    let own_without_container = own.replace(SOURCE_CONTAINER, "");
    assert!(
        rules(&validate_asset_profile(&parse_ok(&own_without_container)))
            .contains(&Rule::AssetConstraintDependencyUnmet)
    );
}

// ================================================================== the boundary

/// The inheritance boundary is fixed by the crate, not by the document: every `ASSET_BLOCK` is
/// exactly one of shared or destination, and the array tables are split the same way.
#[test]
fn every_block_is_either_shared_or_destination_and_never_both() {
    for block in ASSET_BLOCK {
        let shared = SHARED_BLOCKS.contains(block);
        let destination = DESTINATION_BLOCKS.contains(block);
        assert!(shared != destination, "{block}: exactly one side");
    }
    assert_eq!(
        SHARED_BLOCKS.len() + DESTINATION_BLOCKS.len(),
        ASSET_BLOCK.len()
    );
    assert_eq!(
        DESTINATION_BLOCKS,
        &["discovery", "install", "runtime_lookup"]
    );
    assert!(PLACEMENT_BLOCK
        .iter()
        .filter(|b| **b != "path_rule")
        .all(|b| DESTINATION_BLOCKS.contains(b)));
    assert_eq!(
        SHARED_TABLES,
        &["path_family", "variant", "resolution_chain", "operation"]
    );
    assert_eq!(LOCAL_EVIDENCE_TABLES, &["corpus"]);
}

/// A valid reference: the effective profile carries the source's shared definitions, in their
/// states, and nothing is written twice in the document.
#[test]
fn a_valid_reference_inherits_the_shared_definitions_without_duplicating_them() {
    let p = parse_ok(&synthetic("", REUSER));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let local = declared(&p, "fixture-reuser");
    assert_eq!(local.reuses.as_deref(), Some("fixture-source"));
    assert!(local.path_families.is_empty() && local.resolution_chains.is_empty());
    assert!(local.operations.is_empty() && local.corpora.is_empty());
    assert_eq!(shared_declarations(local), Vec::<&str>::new());

    let r = resolve_target(&p, "fixture-reuser").unwrap();
    assert_eq!(r.reused_from.as_deref(), Some("fixture-source"));
    assert_eq!(
        r.inherited,
        [
            "identity",
            "corpus",
            "path_family",
            "resolution_chain",
            "operation"
        ]
    );
    let e = &r.profile;
    assert_eq!(e.target, "fixture-reuser");
    assert_eq!(e.note, local.note);
    let source = declared(&p, "fixture-source");
    assert_eq!(e.path_families, source.path_families);
    assert_eq!(e.resolution_chains, source.resolution_chains);
    assert_eq!(e.operations, source.operations);
    assert_eq!(e.corpora, source.corpora);
    assert_eq!(e.identity, source.identity);
    // The source itself resolves to itself, inheriting nothing.
    let s = resolve_target(&p, "fixture-source").unwrap();
    assert_eq!(s.reused_from, None);
    assert!(s.inherited.is_empty());
    assert_eq!(&s.profile, source);
    // The document says the share once: the text of the reusing target is the reference.
    let text = synthetic("", REUSER);
    assert_eq!(text.matches("[[target_profile.path_family]]").count(), 2);
    assert_eq!(
        text.matches("[[target_profile.resolution_chain]]").count(),
        1
    );
}

// ============================================================ reference errors

#[test]
fn a_missing_reference_is_a_finding_and_a_resolver_error() {
    let text = synthetic("", &REUSER.replace("fixture-source\"", "fixture-nowhere\""));
    let p = parse_ok(&text);
    let findings = validate_asset_profile(&p);
    assert!(
        rules(&findings).contains(&Rule::AssetReuseUnresolved),
        "{findings:?}"
    );
    assert_eq!(
        resolve_target(&p, "fixture-reuser"),
        Err(ReuseError::UnresolvedReference {
            target: "fixture-reuser".into(),
            reuses: "fixture-nowhere".into()
        })
    );
    assert_eq!(
        resolve_target(&p, "fixture-unknown"),
        Err(ReuseError::UnknownTarget {
            target: "fixture-unknown".into()
        })
    );
    assert_eq!(
        reuse_source(&p, declared(&p, "fixture-source")),
        Err(ReuseError::NotAReuse {
            target: "fixture-source".into()
        })
    );
}

#[test]
fn a_self_reference_is_refused() {
    let text = synthetic(
        "",
        &REUSER.replace("reuses = \"fixture-source\"", "reuses = \"fixture-reuser\""),
    );
    let p = parse_ok(&text);
    assert!(rules(&validate_asset_profile(&p)).contains(&Rule::AssetReuseSelfReference));
    assert_eq!(
        resolve_target(&p, "fixture-reuser"),
        Err(ReuseError::SelfReference {
            target: "fixture-reuser".into()
        })
    );
}

/// A → B → A, and A → B → C alike: a reused profile may not itself reuse. Both ends of a cycle are
/// reported, and neither resolves.
#[test]
fn chains_and_cycles_are_refused() {
    let cycle = r#"
[[target_profile]]
target = "fixture-a"
reuses = "fixture-b"

[[target_profile]]
target = "fixture-b"
reuses = "fixture-a"
"#;
    let p = parse_ok(&synthetic("", cycle));
    let findings = validate_asset_profile(&p);
    assert_eq!(
        rules(&findings)
            .iter()
            .filter(|r| **r == Rule::AssetReuseChain)
            .count(),
        2,
        "{findings:?}"
    );
    for (t, other) in [("fixture-a", "fixture-b"), ("fixture-b", "fixture-a")] {
        assert_eq!(
            resolve_target(&p, t),
            Err(ReuseError::SourceReuses {
                target: t.into(),
                reuses: other.into(),
                source_reuses: t.into()
            })
        );
    }
    let chain = r#"
[[target_profile]]
target = "fixture-middle"
reuses = "fixture-source"

[[target_profile]]
target = "fixture-end"
reuses = "fixture-middle"
"#;
    let p = parse_ok(&synthetic("", chain));
    let findings = validate_asset_profile(&p);
    assert!(
        rules(&findings).contains(&Rule::AssetReuseChain),
        "{findings:?}"
    );
    assert!(resolve_target(&p, "fixture-middle").is_ok());
    assert!(matches!(
        resolve_target(&p, "fixture-end"),
        Err(ReuseError::SourceReuses { .. })
    ));
}

// ====================================================== conflicting declarations

/// A reusing target that also declares a shared block or table is refused as a conflict — the
/// validator names every conflicting item, the resolver merges nothing.
#[test]
fn a_local_shared_declaration_on_a_reusing_target_is_a_conflict_not_a_merge() {
    let conflicting = format!(
        "{REUSER}\n{}\n{}",
        r#"
[[target_profile.path_family]]
id = "own"
template = "elsewhere/{variant}.big"
tokens = ["variant"]
case = "lower"
bucket_resolution = "template-only"
"#,
        r#"
[target_profile.container]
state = "not-applicable"
reason = "synthetic: the reuser says loose"
"#
    );
    let p = parse_ok(&synthetic("", &conflicting));
    let findings = validate_asset_profile(&p);
    let conflict: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.rule == Rule::AssetReuseConflict)
        .collect();
    assert_eq!(conflict.len(), 1, "{findings:?}");
    assert!(conflict[0].detail.contains("container") && conflict[0].detail.contains("path_family"));
    assert_eq!(
        resolve_target(&p, "fixture-reuser"),
        Err(ReuseError::ConflictingLocalDeclarations {
            target: "fixture-reuser".into(),
            declared: vec!["container", "path_family"]
        })
    );
    // Even an *unknown* shared block with a candidate is a declaration.
    let with_candidate = format!(
        "{REUSER}\n{}",
        r#"
[target_profile.identity]
state = "unknown"
[target_profile.identity.candidate]
confidence = "asserted"
provenance = ["synthetic:fixture"]
note = "PROPOSED, NOT OPERATIVE"
[target_profile.identity.candidate.value]
key = "numeric-id"
source = "elsewhere"
ambiguity = "reject"
"#
    );
    let p = parse_ok(&synthetic("", &with_candidate));
    assert!(rules(&validate_asset_profile(&p)).contains(&Rule::AssetReuseConflict));
    // A local corpus whose id shadows a source corpus is a conflict too; a distinct one is not.
    let shadow = REUSER_WITH_INSTALL.replace("destination-copy", "copy");
    let p = parse_ok(&synthetic("", &shadow));
    assert!(rules(&validate_asset_profile(&p)).contains(&Rule::AssetReuseConflict));
    assert_eq!(
        resolve_target(&p, "fixture-reuser"),
        Err(ReuseError::CorpusIdCollision {
            target: "fixture-reuser".into(),
            id: "copy".into()
        })
    );
    let p = parse_ok(&synthetic("", REUSER_WITH_INSTALL));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let e = resolve_target(&p, "fixture-reuser").unwrap().profile;
    assert_eq!(
        e.corpora.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["copy", "destination-copy"]
    );
}

// ================================================== candidates stay non-operative

/// The source's unknown-with-candidate identity is inherited as exactly that: readiness of an
/// operation requiring it is blocked on the reuser precisely as on the source, and the resolver
/// makes nothing operative.
#[test]
fn shared_candidates_remain_non_operative_through_reuse() {
    let p = parse_ok(&synthetic("", REUSER));
    let source = declared(&p, "fixture-source");
    let e = resolve_target(&p, "fixture-reuser").unwrap().profile;
    assert_eq!(e.identity.state, ConstraintState::Unknown);
    assert!(e.identity.candidate.is_some() && !e.identity.is_operative());
    let identify = e
        .operations
        .iter()
        .find(|o| o.operation == "identify")
        .unwrap();
    assert_eq!(
        operation_constraint_readiness(&e, None, identify),
        OperationReadiness::BlockedByUnknownConstraints(vec!["identity".into()])
    );
    assert_eq!(
        operation_constraint_readiness(&e, None, identify),
        operation_constraint_readiness(source, None, identify)
    );
    let states: Vec<_> = block_states(&e, None)
        .into_iter()
        .filter(|s| SHARED_BLOCKS.contains(&s.block))
        .collect();
    let source_states: Vec<_> = block_states(source, None)
        .into_iter()
        .filter(|s| SHARED_BLOCKS.contains(&s.block))
        .collect();
    assert_eq!(
        states, source_states,
        "shared block states travel unchanged"
    );
}

// ============================================= destination requirements independent

/// The source establishes a loose-file install; the reuser inherits nothing of it. Its `install`
/// operation is blocked on `install`, creation through the inherited chain is refused for the
/// missing install, and only its OWN destination evidence lifts that — while the shared-file
/// write exclusion is untouched either way.
#[test]
fn destination_blocks_are_never_inherited_and_the_write_exclusion_survives_reuse() {
    let p = parse_ok(&synthetic(SOURCE_LOOSE_INSTALL, REUSER));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let source = declared(&p, "fixture-source");
    let e = resolve_target(&p, "fixture-reuser").unwrap().profile;
    assert!(source.install.is_operative());
    assert_eq!(e.install.state, ConstraintState::Unknown, "not inherited");
    assert_eq!(e.discovery.state, ConstraintState::Unknown);
    assert_eq!(e.runtime_lookup.state, ConstraintState::Unknown);
    // The container decision IS shared: absent on the source, absent on the reuser.
    assert_eq!(e.container.state, ConstraintState::NotApplicable);
    assert_eq!(
        CreationSupport::of(&e),
        CreationSupport {
            install: false,
            container: true,
            references: false
        }
    );
    assert_eq!(
        CreationSupport::of(source),
        CreationSupport {
            install: true,
            container: true,
            references: true
        }
    );
    let install_op = e
        .operations
        .iter()
        .find(|o| o.operation == "install")
        .unwrap();
    assert_eq!(
        operation_constraint_readiness(source, None, install_op),
        OperationReadiness::ConstraintsEstablished
    );
    assert_eq!(
        operation_constraint_readiness(&e, None, install_op),
        OperationReadiness::BlockedByUnknownConstraints(vec!["install".into()])
    );

    let chain = &e.resolution_chains[0];
    assert!(
        chain.declared.is_operative(),
        "the chain is inherited operative"
    );
    assert_eq!(shared_steps(&e), shared_steps(source));
    assert_eq!(ownership_sets(&e), ownership_sets(source));
    let tokens = BTreeMap::from([("variant", "1_019_3")]);
    let own = "fe/jerseys/1_019_3.big";
    let shared = "fe/jerseys/custom.big";
    // Read falls back to the shared default on the reuser as on the source.
    assert_eq!(
        resolve_read(&e, chain, &tokens, |path| path == shared).unwrap(),
        ReadResolution::Found {
            step: 1,
            family: "default".into(),
            path: shared.into(),
            shared: true
        }
    );
    // Write: the source may create; the reuser may not — its install is its own and unknown.
    assert!(matches!(
        resolve_write(source, chain, &tokens, |_| false),
        Ok(WriteResolution::Create { .. })
    ));
    assert_eq!(
        resolve_write(&e, chain, &tokens, |_| false),
        Err(WriteBlocker::CreationUnsupported {
            path: own.into(),
            missing: vec!["install", "references"]
        })
    );
    // An existing owned file is still a replace destination on the reuser.
    assert!(matches!(
        resolve_write(&e, chain, &tokens, |path| path == own),
        Ok(WriteResolution::Existing { step: 0, .. })
    ));
    // The shared-file exclusion: a token that lands the owned template on the shared default is
    // refused on the reuser exactly as on the source, whether or not the file exists.
    let onto_shared = BTreeMap::from([("variant", "custom")]);
    for t in [source, &e] {
        for exists in [true, false] {
            assert_eq!(
                resolve_write(t, chain, &onto_shared, |_| exists),
                Err(WriteBlocker::AliasesSharedDestination {
                    path: "fe/jerseys/custom.big".into(),
                    shared_family: "default".into(),
                    chain: "thumb".into(),
                }),
                "{}: exists = {exists}",
                t.target
            );
        }
    }
    // Only the reuser's OWN destination evidence lifts its creation blocker.
    let p = parse_ok(&synthetic(SOURCE_LOOSE_INSTALL, REUSER_WITH_INSTALL));
    assert_eq!(validate_asset_profile(&p), vec![]);
    let e = resolve_target(&p, "fixture-reuser").unwrap().profile;
    assert!(e.install.is_operative());
    assert_eq!(e.install.corpus.as_deref(), Some("destination-copy"));
    assert!(matches!(
        resolve_write(&e, &e.resolution_chains[0], &tokens, |_| false),
        Ok(WriteResolution::Create { .. })
    ));
    assert_eq!(
        operation_constraint_readiness(&e, None, install_op),
        OperationReadiness::ConstraintsEstablished
    );
    // ... and the exclusion still holds with creation enabled.
    assert!(matches!(
        resolve_write(&e, &e.resolution_chains[0], &onto_shared, |_| false),
        Err(WriteBlocker::AliasesSharedDestination { .. })
    ));
}

// ======================================================== schema compatibility

/// `reuses` is a `/5` construct: a `/3` or `/4` document that uses it is refused as such, and a
/// `/4` document without it is validated exactly as before.
#[test]
fn older_schemas_do_not_silently_accept_reuse() {
    for older in [ASSET_PROFILE_SCHEMA_V3, ASSET_PROFILE_SCHEMA_V4] {
        let text = synthetic("", REUSER).replace(ASSET_PROFILE_SCHEMA, older);
        let p = parse_ok(&text);
        let findings = validate_asset_profile(&p);
        let schema: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.rule == Rule::UnsupportedSchema && f.detail.contains("`reuses`"))
            .collect();
        assert_eq!(schema.len(), 1, "{older}: {findings:?}");
        assert!(schema[0].detail.contains(ASSET_PROFILE_SCHEMA));
    }
    // The same document under `/4` without the reusing target is clean — `/4` meaning unchanged.
    let text = synthetic("", "").replace(ASSET_PROFILE_SCHEMA, ASSET_PROFILE_SCHEMA_V4);
    assert_eq!(validate_asset_profile(&parse_ok(&text)), vec![]);
    assert_eq!(
        ASSET_PROFILE_SCHEMAS_NATIVE,
        [
            ASSET_PROFILE_SCHEMA,
            ASSET_PROFILE_SCHEMA_V4,
            ASSET_PROFILE_SCHEMA_V3
        ]
    );
    assert_eq!(ASSET_PROFILE_SCHEMA, "asset-output-profile/5");
}

// =============================================== the committed Recompiled registration

/// Q47 on the three committed families: `legacyrecomp-recomp` reuses `nhllegacy-xbox360` in each,
/// declares only unknown destination blocks, inherits every shared block in its source state (so
/// nothing becomes operative that is not operative on Legacy Xbox 360), and is reported apart as
/// registered by reference.
#[test]
fn legacy_recompiled_reuses_the_legacy_xbox360_definitions_in_every_committed_family() {
    let mut rows = Vec::new();
    for file in [
        "assets-portraits.toml",
        "assets-logos.toml",
        "assets-jerseys.toml",
    ] {
        let p = profile(file);
        assert_eq!(p.schema, ASSET_PROFILE_SCHEMA, "{file}");
        let findings = validate_asset_profile(&p);
        assert_eq!(findings, vec![], "{file}: {findings:?}");
        let local = declared(&p, "legacyrecomp-recomp");
        assert_eq!(local.reuses.as_deref(), Some("nhllegacy-xbox360"), "{file}");
        assert!(
            local.note.contains("Q47") && local.note.contains("user"),
            "{file}"
        );
        assert!(local.note.contains("not a measurement"), "{file}");
        assert_eq!(shared_declarations(local), Vec::<&str>::new(), "{file}");
        assert!(
            local.corpora.is_empty(),
            "{file}: no Recompiled corpus is invented"
        );
        for (name, block) in [
            ("discovery", local.discovery.state),
            ("install", local.install.state),
            ("runtime_lookup", local.runtime_lookup.state),
        ] {
            assert_eq!(block, ConstraintState::Unknown, "{file}: {name}");
        }
        assert!(local.install.note.contains("Not inherited"), "{file}");

        let r = resolve_target(&p, "legacyrecomp-recomp").unwrap();
        assert_eq!(r.reused_from.as_deref(), Some("nhllegacy-xbox360"));
        let source = declared(&p, "nhllegacy-xbox360");
        let e = &r.profile;
        assert_eq!(e.path_families, source.path_families, "{file}");
        assert_eq!(e.resolution_chains, source.resolution_chains, "{file}");
        assert_eq!(e.variants, source.variants, "{file}");
        assert_eq!(e.operations, source.operations, "{file}");
        assert_eq!(e.corpora, source.corpora, "{file}");
        // Shared block states are the source's; nothing inherited is operative that is not
        // operative there — and on Legacy Xbox 360 nothing is.
        let shared = |t: &TargetProfile| -> Vec<BlockState> {
            block_states(t, None)
                .into_iter()
                .filter(|s| SHARED_BLOCKS.contains(&s.block))
                .collect()
        };
        assert_eq!(shared(e), shared(source), "{file}");
        assert!(
            shared(e).iter().all(|s| !s.operative),
            "{file}: nothing operative"
        );
        // Every operation is blocked on the reuser as on the source — and the destination
        // requirements are among what blocks the installing ones.
        for op in &e.operations {
            let mine = operation_constraint_readiness(e, None, op);
            assert!(
                matches!(mine, OperationReadiness::BlockedByUnknownConstraints(_)),
                "{file}/{}: {mine:?}",
                op.operation
            );
            if op
                .requires
                .iter()
                .any(|r| DESTINATION_BLOCKS.contains(&r.as_str()))
            {
                let OperationReadiness::BlockedByUnknownConstraints(missing) = mine else {
                    unreachable!()
                };
                assert!(
                    missing
                        .iter()
                        .any(|m| DESTINATION_BLOCKS.contains(&m.as_str())),
                    "{file}/{}: {missing:?}",
                    op.operation
                );
            }
        }
        assert!(
            !CreationSupport::of(e).install,
            "{file}: no install capability from reuse"
        );
        for chain in &e.resolution_chains {
            if matches!(chain.purpose.as_str(), "write" | "read-write") {
                assert!(
                    resolve_write(e, chain, &BTreeMap::new(), |_| false).is_err(),
                    "{file}/{}: nothing writable through reuse",
                    chain.id
                );
            }
        }
        rows.extend(asset_registry_population(std::iter::once(&p)));
    }
    let recomp: Vec<&AssetPopulation> = rows
        .iter()
        .filter(|r| r.target == "legacyrecomp-recomp")
        .collect();
    assert_eq!(recomp.len(), 3);
    for row in &recomp {
        assert_eq!(row.reused_from.as_deref(), Some("nhllegacy-xbox360"));
        assert_eq!(
            row.operative_constraints, 0,
            "{}: nothing operative",
            row.family
        );
        let source = rows
            .iter()
            .find(|r| r.family == row.family && r.target == "nhllegacy-xbox360")
            .unwrap();
        assert_eq!(source.reused_from, None);
        assert_eq!(row.declared_path_families, source.declared_path_families);
        assert_eq!(
            row.declared_resolution_chains,
            source.declared_resolution_chains
        );
        assert_eq!(row.declared_corpora, source.declared_corpora);
    }
    assert!(rows
        .iter()
        .filter(|r| r.target != "legacyrecomp-recomp")
        .all(|r| r.reused_from.is_none()));
}
