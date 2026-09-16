//! Positive and negative coverage for the `asset-output-profile/3` contract.
//!
//! Every asset representation has a positive case proving it parses and validates, and every
//! enforceable asset rule has a negative case proving an invalid declaration cannot pass as a
//! qualified constraint.

use super::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(relative: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative),
    )
    .unwrap_or_else(|e| panic!("fixture {relative}: {e}"))
}

const COMMITTED_PROFILE: &str = "defs/adapters/asset-profiles/assets-portraits.toml";

fn committed_profile() -> AssetProfile {
    let text = std::fs::read_to_string(repo_root().join(COMMITTED_PROFILE))
        .expect("the profile is committed");
    parse_asset_profile(&text).expect("it parses")
}

fn synthetic_profile() -> AssetProfile {
    parse_asset_profile(&fixture("synthetic-asset-profile.toml")).expect("it parses")
}

fn target<'a>(profile: &'a AssetProfile, name: &str) -> &'a TargetProfile {
    profile
        .targets
        .iter()
        .find(|t| t.target == name)
        .unwrap_or_else(|| panic!("{name} is declared"))
}

/// Whether a block is established, read through the same `block_states` view every population and
/// readiness report goes through - so a test cannot accidentally assert over a different notion of
/// "established" than the code uses.
fn established_by_name(t: &TargetProfile, block: &str) -> bool {
    assert!(
        ASSET_BLOCK.contains(&block),
        "{block} is not an ASSET_BLOCK name"
    );
    block_states(t, None)
        .into_iter()
        .find(|s| s.block == block)
        .map(|s| s.state == ConstraintState::Established)
        .unwrap_or(false)
}

/// The same, for one variant's correlated blocks.
fn variant_established(v: &AssetVariant, block: &str) -> bool {
    match block {
        "raster" => v.raster.established().is_some(),
        "dimensions" => v.dimensions.established().is_some(),
        "mipmaps" => v.mipmaps.established().is_some(),
        "tiling" => v.tiling.established().is_some(),
        other => panic!("{other} is not a VARIANT_BLOCK name"),
    }
}

fn variant<'a>(t: &'a TargetProfile, id: &str) -> &'a AssetVariant {
    t.variants
        .iter()
        .find(|v| v.id == id)
        .unwrap_or_else(|| panic!("{id} is declared on {}", t.target))
}

// ================================================================== positive

#[test]
fn the_committed_asset_profile_satisfies_the_contract() {
    let profile = committed_profile();
    // The portrait profile was a `/3` document through C0-AS4 (validated in place under its own
    // rules); C0-AS5 moved it to `/5` for the Recompiled reference. `/3` compatibility is checked
    // on the synthetic fixtures below and in `asset_resolution_tests`.
    assert_eq!(profile.schema, ASSET_PROFILE_SCHEMA);
    assert!(ASSET_PROFILE_SCHEMAS_NATIVE.contains(&profile.schema.as_str()));
    let findings = validate_asset_profile(&profile);
    assert_eq!(findings, vec![], "{findings:?}");
    // C0-POR1 added `nhllegacy-ps3`, whose measured copy is the only genuine shipping tree of the
    // three declared targets and therefore carries the strongest portrait evidence in the
    // repository; C0-AS5 added `legacyrecomp-recomp` by reference (nothing declared, nothing
    // operative).
    assert_eq!(profile.targets.len(), 4);
}

#[test]
fn the_committed_profile_establishes_only_what_its_evidence_scope_supports() {
    // History of this tripwire, because the number it pins has moved three times.
    //
    // C0-AS1:     nothing established anywhere - the only portrait source was an operations CSV.
    // C0-POR1:    measured three private copies and promoted 14 blocks across three targets.
    // C0-POR1-R1: review found most of those over-scoped. Three survived, all on one target.
    // C0-AS3:     the same evidence, re-represented under `/3`. Nothing new was measured. The three
    //             survivors keep their promotion and now say WHAT KIND of promotion it is; one
    //             further fact - a per-variant dimension - becomes expressible for the first time.
    //
    // The rule, now enforced rather than narrated: a block is established only when its `scope` is
    // an inspected-corpus invariant or better AND the `corpus` it cites is attributable to the
    // target. Only `nhllegacy-ps3` clears the second half: its corpus is that title's own installed
    // directory. The Xbox 360 corpus is a derived `_compiled` tree and the PS4 corpus is an
    // extraction, and `asset-corpus-attribution-unsupported` refuses either as a basis.
    //
    // NOTHING here reaches `supported-target-constraint`: no load, acceptance or shipping-manifest
    // test was run on any target, so every established block below carries
    // `force = "observed-source-format"` and none of them authorises refusing a Build anything.
    let profile = committed_profile();
    let population = asset_registry_population(std::iter::once(&profile));
    // Three declared targets and one registered by reference (C0-AS5: `legacyrecomp-recomp`
    // reuses `nhllegacy-xbox360`, so its row carries the Xbox 360 counts and is reported apart).
    assert_eq!(population.len(), 4);
    let by_reference = population
        .iter()
        .find(|r| r.target == "legacyrecomp-recomp")
        .unwrap();
    assert_eq!(
        by_reference.reused_from.as_deref(),
        Some("nhllegacy-xbox360")
    );
    assert_eq!(by_reference.established_constraints, 0);

    let expected: [(&str, &[&str]); 3] = [
        // Invariant across all 18 663 inspected files INCLUDING the backup-labelled group and the
        // 206 current files carrying mip chains. Surviving those is what earns the promotion.
        ("nhllegacy-ps3", &["container", "compression", "atlas"]),
        ("nhllegacy-xbox360", &[]),
        ("nhl21-ps4", &[]),
    ];

    for (name, blocks) in expected {
        let t = target(&profile, name);
        let row = population
            .iter()
            .find(|r| r.target == name)
            .unwrap_or_else(|| panic!("{name} is counted"));
        assert_eq!(
            row.established_constraints,
            blocks.len(),
            "{name}: established target-level block count"
        );
        assert_eq!(row.not_applicable_constraints, 0);
        assert_eq!(
            row.established_constraints + row.unknown_constraints,
            ASSET_BLOCK.len(),
            "{name}: every block is accounted for"
        );
        for block in ASSET_BLOCK {
            assert_eq!(
                established_by_name(t, block),
                blocks.contains(block),
                "{name}/{block}: established state does not match the expected set"
            );
        }
        // Every established block says how far its evidence reaches, what it asserts about other
        // values, and which corpus it rests on. None of them claims a target requirement.
        for block in blocks {
            let states = block_states(t, None);
            assert!(states.iter().any(|s| s.block == *block));
            let (scope, force, corpus) = match *block {
                "container" => (
                    t.container.scope.clone(),
                    t.container.force.clone(),
                    t.container.corpus.clone(),
                ),
                "compression" => (
                    t.compression.scope.clone(),
                    t.compression.force.clone(),
                    t.compression.corpus.clone(),
                ),
                "atlas" => (
                    t.atlas.scope.clone(),
                    t.atlas.force.clone(),
                    t.atlas.corpus.clone(),
                ),
                other => panic!("{other} is not in the expected set"),
            };
            assert_eq!(scope.as_deref(), Some("inspected-corpus-invariant"));
            assert_eq!(
                force.as_deref(),
                Some("observed-source-format"),
                "{name}/{block}: no portrait constraint may claim the target REQUIRES it"
            );
            let corpus = corpus.expect("an established block names its corpus");
            let declared = t
                .corpora
                .iter()
                .find(|c| c.id == corpus)
                .expect("the corpus is declared");
            assert_eq!(declared.attribution, "attributable-to-target");
        }
    }

    // No corpus anywhere claims a complete build identification, and every unattributable one is
    // counted apart rather than blended into a "measured" total.
    for row in &population {
        let t = target(&profile, &row.target);
        for corpus in &t.corpora {
            assert_eq!(
                corpus.build_identification, "incomplete",
                "{}/{}: a title id plus a version field is not a build",
                row.target, corpus.id
            );
            assert!(!corpus.limitations.is_empty());
        }
        if row.target == "nhllegacy-ps3" {
            assert_eq!(row.unattributable_corpora, 0);
        } else {
            assert_eq!(
                row.unattributable_corpora, row.declared_corpora,
                "{}: a derived or extracted tree is attributable to nothing",
                row.target
            );
        }
    }

    // The one fact `/2` could not express at all: a per-variant dimension. It is established only
    // where it is NOT circular - the small-portrait variant is selected by path family, so its size
    // is a finding about the selected subset rather than the definition of it.
    let ps3 = target(&profile, "nhllegacy-ps3");
    assert_eq!(ps3.variants.len(), 3);
    let small = variant(ps3, "playerheadssmall-256x128");
    assert!(variant_established(small, "dimensions"));
    let dimensions = small.dimensions.established().expect("established");
    assert_eq!((dimensions.width, dimensions.height), (256, 128));
    assert_eq!(
        small.dimensions.force.as_deref(),
        Some("observed-source-format")
    );
    for id in ["playerheads-512x512", "playerheads-512x256"] {
        assert!(
            !variant_established(variant(ps3, id), "dimensions"),
            "{id}: this variant is SELECTED by its stored size, so establishing that size would \
             report the definition as the discovery - the circularity R1 caught in [mipmaps]"
        );
    }

    // Blocks R1 specifically demoted, each with its own reason. Asserted by name so a future
    // re-promotion has to argue with the reason rather than silently flip a state.
    assert!(
        ps3.discovery.established().is_none(),
        "placement in one installed copy is not a discovery rule; patch.big was never examined"
    );
    assert!(
        ps3.runtime_lookup.established().is_none(),
        "what the game LOADS is unknown on every target: patch.big could shadow the loose files"
    );
    for variant in &ps3.variants {
        assert!(
            !variant_established(variant, "mipmaps"),
            "{}: the mip finding was circular - the group was defined by its mip count",
            variant.id
        );
        assert!(
            !variant_established(variant, "tiling"),
            "{}: linear layout rests on three specimen decodes, not on a corpus invariant",
            variant.id
        );
    }

    // Partially known values, and the sub-properties that stay unknown rather than being filled.
    // C0-POR1 wrote a guessed colour space into the bundle to make it well-formed; `/3` removes the
    // pressure to do that by letting the bundle be incomplete.
    for variant in &ps3.variants {
        let raster = variant.raster.established().expect("codec is established");
        assert_eq!(raster.codec, "dxt5");
        assert!(
            raster.color_space.is_none(),
            "{}: the DDS header records no colour space; filling one is fabrication",
            variant.id
        );
        assert!(
            raster.alpha.is_none(),
            "{}: alpha is a three-specimen observation, not a target-wide mode",
            variant.id
        );
        assert!(variant.raster.properties.contains_key("color_space"));
        assert!(variant.raster.properties.contains_key("alpha"));
        for evidence in variant.raster.properties.values() {
            assert_eq!(evidence.state, ConstraintState::Unknown);
            assert!(!evidence.reason.trim().is_empty());
        }
    }

    // [atlas] is established on the fact a payload count actually measures, and NOT on the atlas
    // kind C0-POR1 read off that same count.
    let atlas = ps3.atlas.established().expect("established");
    assert_eq!(atlas.sharing, "per-asset-surface");
    assert!(
        atlas.kind.is_none(),
        "counting one payload per archive does not distinguish an atlas from a pool from neither"
    );
    assert!(ps3.atlas.properties.contains_key("kind"));

    // Bundled values whose unknown fields were previously filled with guesses stay unfilled.
    for name in ["nhllegacy-ps3", "nhllegacy-xbox360", "nhl21-ps4"] {
        let t = target(&profile, name);
        assert!(
            t.raster.established().is_none(),
            "{name}: raster is per variant"
        );
        // R1 removed a `reject` ambiguity policy C0-POR1 had filled in with nothing behind it.
        // C0-AS6 put a candidate back - MEASURED on the committed anchor (`ubPc.lOnr`, with the
        // collisions the anchor stores grounding `reject` the way C0-AS4 grounded it for jerseys)
        // - and the block itself stays unknown: no anchor id was placed beside a file.
        let identity = t
            .identity
            .candidate
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: the C0-AS6 identity candidate"));
        assert!(t.identity.established().is_none(), "{name}: identity");
        assert_eq!(identity.confidence.as_deref(), Some("measured"), "{name}");
        assert_eq!(
            identity.scope.as_deref(),
            Some("specimen-observation"),
            "{name}: the database bytes were measured, not a file beside an id"
        );
        assert!(
            identity
                .provenance
                .iter()
                .any(|p| p.ends_with("#legacy_portrait_id_field_is_measured_on_the_anchor")),
            "{name}: the candidate names the case that measured it"
        );
        let value = identity.value.as_ref().expect("a value");
        assert_eq!(
            (value.key.as_str(), value.ambiguity.as_str()),
            ("numeric-id", "reject"),
            "{name}"
        );
        assert!(value.source.starts_with("ubPc.lOnr"), "{name}");
        assert!(t.path_rule.established().is_none(), "{name}: path_rule");
        assert!(t.install.established().is_none(), "{name}: install");
        // Both portrait path families are recorded, and neither resolves.
        assert!(!t.path_families.is_empty());
        let row = population
            .iter()
            .find(|r| r.target == name)
            .expect("counted");
        assert_eq!(
            row.unresolved_path_families, row.declared_path_families,
            "{name}: only bucket MEMBERSHIP was observed; no id-to-directory rule was identified"
        );
    }
    assert_eq!(target(&profile, "nhllegacy-ps3").path_families.len(), 2);
    assert_eq!(target(&profile, "nhllegacy-xbox360").path_families.len(), 2);

    // The derived-tree targets keep their measurements as candidates - demoting for provenance must
    // not throw the measurement away.
    for name in ["nhllegacy-xbox360", "nhl21-ps4"] {
        let t = target(&profile, name);
        assert!(
            t.container.candidate.is_some(),
            "{name}: container candidate"
        );
        assert!(t.atlas.candidate.is_some(), "{name}: atlas candidate");
        for variant in &t.variants {
            assert!(
                variant.mipmaps.candidate.is_some(),
                "{name}/{}: mipmaps candidate",
                variant.id
            );
        }
    }

    // PS4 compression is a different kind of unknown and must stay bare: the extracted files are
    // stored decompressed, so the shipping payload codec is not observable at all. An absence of
    // evidence, not a measurement of `none`.
    let ps4 = target(&profile, "nhl21-ps4");
    assert!(
        ps4.compression.candidate.is_none(),
        "PS4 compression is unobservable in an extracted tree, not measured as none"
    );
}

#[test]
fn no_operation_on_the_committed_profile_is_ready_and_every_gap_is_named() {
    // C0-POR1 briefly had `nhllegacy-ps3/discover` reporting `ConstraintsEstablished`, because it
    // had promoted [discovery] on observed file placement alone. R1 demoted that, and `/3` adds
    // [runtime_lookup] so the question placement was standing in for has a home. Once again NO
    // operation on a real profile has its constraints met - on any variant.
    //
    // What survived C0-POR1 is that the blocking set is a strict SUBSET of the required set rather
    // than all of it, so this asserts the subset relation and that the report never names something
    // that is actually decided.
    let profile = committed_profile();
    for t in &profile.targets {
        let variants: Vec<Option<&AssetVariant>> = if t.variants.is_empty() {
            vec![None]
        } else {
            t.variants.iter().map(Some).collect()
        };
        for chosen in variants {
            for declaration in &t.operations {
                let label = chosen.map(|v| v.id.as_str()).unwrap_or("(no variant)");
                match operation_constraint_readiness(t, chosen, declaration) {
                    OperationReadiness::BlockedByUnknownConstraints(missing) => {
                        let required: BTreeSet<&str> =
                            declaration.requires.iter().map(String::as_str).collect();
                        let named: BTreeSet<&str> = missing.iter().map(String::as_str).collect();
                        assert!(!named.is_empty());
                        assert!(
                            named.is_subset(&required),
                            "{}/{label}/{}: {named:?} is not a subset of {required:?}",
                            t.target,
                            declaration.operation
                        );
                    }
                    OperationReadiness::ConstraintsEstablished => panic!(
                        "{}/{label}/{}: no operation may have its constraints met on a real \
                         profile yet",
                        t.target, declaration.operation
                    ),
                }
            }
        }
    }
}

#[test]
fn the_committed_profile_carries_correlated_variants_and_no_invented_combination() {
    // The C0-POR1 gap `/2` could not close: NHL Legacy carries three portrait forms at once, and a
    // single `DimensionsValue` per target could hold one of them. What matters as much as being
    // able to hold three is that the three stay CORRELATED - there is no per-target list of allowed
    // sizes and no per-target list of allowed codecs, so a size from one form cannot be paired with
    // a codec from another.
    let profile = committed_profile();
    for name in ["nhllegacy-ps3", "nhllegacy-xbox360"] {
        let t = target(&profile, name);
        assert_eq!(t.variants.len(), 3, "{name}");
        for block in VARIANT_BLOCK {
            assert!(
                !established_by_name(t, block),
                "{name}/{block}: a target with variants must not also decide the geometry at target \
                 level, or a caller could read a value no variant carries"
            );
        }
    }
    assert_eq!(target(&profile, "nhl21-ps4").variants.len(), 1);

    let ps3 = target(&profile, "nhllegacy-ps3");

    // Path family alone does not identify a `playerheads` portrait: two forms live in that folder
    // and nothing observed separates them by path. The answer is a REFUSAL naming both, not the
    // first or the closest.
    let by_family = VariantQuery {
        path_family: Some("playerheads".to_string()),
        ..Default::default()
    };
    assert_eq!(
        select_variant(ps3, &by_family),
        VariantSelection::Ambiguous(vec![
            "playerheads-512x256".to_string(),
            "playerheads-512x512".to_string(),
        ])
    );

    // Adding the stored size resolves it. This is the source-preserving case: a Build planning a
    // replacement knows the size of the asset it is replacing.
    let resolved = VariantQuery {
        path_family: Some("playerheads".to_string()),
        stored_dimensions: Some((512, 256)),
        ..Default::default()
    };
    assert_eq!(
        select_variant(ps3, &resolved),
        VariantSelection::Selected("playerheads-512x256".to_string())
    );

    // The small family has one form, so the family alone is enough.
    let small = VariantQuery {
        path_family: Some("playerheadssmall".to_string()),
        ..Default::default()
    };
    assert_eq!(
        select_variant(ps3, &small),
        VariantSelection::Selected("playerheadssmall-256x128".to_string())
    );

    // A family this target does not have matches nothing, and says so explicitly.
    let elsewhere = VariantQuery {
        path_family: Some("playerportraits".to_string()),
        ..Default::default()
    };
    assert_eq!(select_variant(ps3, &elsewhere), VariantSelection::NoMatch);

    // An impossible cross-variant combination - the small family's path with a large form's size -
    // is not "nearly" the small variant. It matches nothing.
    let impossible = VariantQuery {
        path_family: Some("playerheadssmall".to_string()),
        stored_dimensions: Some((512, 512)),
        ..Default::default()
    };
    assert_eq!(
        select_variant(ps3, &impossible),
        VariantSelection::Selected("playerheadssmall-256x128".to_string()),
        "the small variant declares no size clause, so a size cannot mismatch it - and its \
         established dimensions are what a caller is then held to"
    );
    let held_to = variant(ps3, "playerheadssmall-256x128")
        .dimensions
        .established()
        .expect("established");
    assert_eq!((held_to.width, held_to.height), (256, 128));

    // An empty query decides nothing, so every variant stays live and the result is a refusal.
    match select_variant(ps3, &VariantQuery::default()) {
        VariantSelection::Ambiguous(ids) => assert_eq!(ids.len(), 3),
        other => panic!("an undecidable query must refuse, not guess: {other:?}"),
    }
}

#[test]
fn an_unknown_sub_property_blocks_only_the_operations_that_need_it() {
    // The point of the per-sub-property split. `preview` needs the codec and does not care about
    // the colour space; `replace` re-encodes pixels and does. Under `/2` the unmeasurable colour
    // space made the whole `raster` block unknown and blocked both.
    let profile = committed_profile();
    let ps3 = target(&profile, "nhllegacy-ps3");
    let small = variant(ps3, "playerheadssmall-256x128");

    let preview = ps3
        .operations
        .iter()
        .find(|o| o.operation == "preview")
        .expect("declared");
    assert!(preview.requires.contains(&"raster.codec".to_string()));
    match operation_constraint_readiness(ps3, Some(small), preview) {
        OperationReadiness::BlockedByUnknownConstraints(missing) => assert!(
            !missing.iter().any(|m| m.starts_with("raster")),
            "the codec IS established, so no raster requirement may be reported: {missing:?}"
        ),
        OperationReadiness::ConstraintsEstablished => {
            panic!("[tiling] is unknown on every portrait variant, so preview cannot be ready")
        }
    }

    let replace = ps3
        .operations
        .iter()
        .find(|o| o.operation == "replace")
        .expect("declared");
    match operation_constraint_readiness(ps3, Some(small), replace) {
        OperationReadiness::BlockedByUnknownConstraints(missing) => {
            assert!(
                missing.contains(&"raster.color_space".to_string()),
                "replace re-encodes pixels and needs the colour space: {missing:?}"
            );
            assert!(
                missing.contains(&"atlas.kind".to_string()),
                "replace must know whether a surface is shared: {missing:?}"
            );
            assert!(
                !missing.contains(&"atlas".to_string()),
                "the block itself is established; only the sub-property is missing"
            );
        }
        OperationReadiness::ConstraintsEstablished => panic!("replace cannot be ready"),
    }
}

#[test]
fn the_synthetic_asset_fixture_validates_and_is_labelled_synthetic() {
    let profile = synthetic_profile();
    assert!(profile.synthetic);
    let findings = validate_asset_profile(&profile);
    assert_eq!(findings, vec![], "{findings:?}");
    // Four: C0-AS3-R1 added `fixturefour-synthetic` for per-variant containers, a provenance
    // bridge, and an established constraint that is operative for nothing.
    assert_eq!(profile.targets.len(), 4);
    assert!(profile.targets.iter().all(|t| t.target.contains("fixture")));
}

#[test]
fn no_committed_asset_profile_is_a_synthetic_fixture() {
    let directory = repo_root().join("defs/adapters/asset-profiles");
    let mut seen = 0;
    for entry in std::fs::read_dir(&directory).expect("the profile directory exists") {
        let path = entry.expect("readable").path();
        if path.extension().is_none_or(|e| e != "toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable");
        let profile = parse_asset_profile(&text).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert!(
            !profile.synthetic,
            "{path:?} is committed as a registry profile and cannot be a synthetic fixture"
        );
        seen += 1;
    }
    assert_eq!(
        seen, 4,
        "four of the seven asset-bearing families have a profile (portraits populated; logos,          jerseys and equipment represented); heads, arenas and audio do not - see          asset_coverage_tests"
    );
}

/// C0-REG2: the logo and uniform profiles are REPRESENTATION, not coverage. Each parses, validates
/// clean, declares one target (NHL Legacy Xbox 360 - the only target with committed records for
/// either family), and establishes NOTHING: no block at target or variant level, no operative
/// constraint, no specimen observation. What they carry is the evidence at its real scope -
/// corpora, path families, candidates - and a population that says zero in every established
/// column while the candidate column is non-zero. A later revision that promotes a block must
/// change these numbers deliberately.
#[test]
fn the_logo_and_jersey_profiles_represent_their_families_and_establish_nothing() {
    let expected: [(&str, &str, usize, usize, usize, bool); 2] = [
        // family, file, path families, variants, candidates (target-level blocks and variant
        // correlated blocks carrying a proposal; a variant's container proposal is not counted),
        // whether [identity] carries a candidate.
        ("assets-logos", "assets-logos.toml", 2, 2, 4, false),
        // C0-AS4 (`/4`): three shared-fallback families and a measured composite-identity
        // candidate were added; the chain is a candidate too and is counted apart (see
        // asset_resolution_tests).
        ("assets-jerseys", "assets-jerseys.toml", 6, 3, 3, true),
    ];
    for (family, file, families, variants, candidates, identity_candidate) in expected {
        let text =
            std::fs::read_to_string(repo_root().join("defs/adapters/asset-profiles").join(file))
                .unwrap();
        let profile = parse_asset_profile(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(profile.family, family);
        assert!(!profile.synthetic);
        let findings = validate_asset_profile(&profile);
        assert_eq!(findings, vec![], "{file}: {findings:?}");
        assert_eq!(
            profile
                .targets
                .iter()
                .map(|t| t.target.as_str())
                .collect::<Vec<_>>(),
            ["nhllegacy-xbox360", "legacyrecomp-recomp"],
            "{file}: one represented target, and Recompiled by reference to it (C0-AS5)"
        );
        assert!(
            profile
                .note
                .contains("Nothing on any target is established"),
            "{file}: the profile says what it is"
        );

        let population = asset_registry_population(std::iter::once(&profile));
        assert_eq!(population.len(), 2);
        // The reference row repeats the Xbox 360 counts (it inherits those definitions) and
        // establishes nothing of its own either; the declared row is what is checked below.
        assert_eq!(
            population[1].reused_from.as_deref(),
            Some("nhllegacy-xbox360"),
            "{file}"
        );
        assert_eq!(population[1].established_constraints, 0, "{file}");
        let p = &population[0];
        assert_eq!(p.reused_from, None);
        assert_eq!(
            (
                p.established_constraints,
                p.operative_constraints,
                p.specimen_observations,
                p.target_required_constraints,
                p.established_variant_constraints,
                p.operative_variant_constraints,
            ),
            (0, 0, 0, 0, 0, 0),
            "{file}: representation establishes nothing"
        );
        assert_eq!(p.unknown_constraints, ASSET_BLOCK.len(), "{file}");
        assert_eq!(p.candidate_constraints, candidates, "{file}");
        assert_eq!(p.declared_variants, variants, "{file}");
        assert_eq!(
            (p.declared_path_families, p.unresolved_path_families),
            (families, families),
            "{file}: every family is unresolved"
        );
        assert_eq!(
            (p.declared_corpora, p.unattributable_corpora),
            (2, 1),
            "{file}: the shipping archives are attributable, the derived tree is not"
        );

        // No operation's requirements are met, and none is missing a declaration.
        let t = target(&profile, "nhllegacy-xbox360");
        assert_eq!(t.operations.len(), 6, "{file}");
        for declaration in &t.operations {
            assert!(
                matches!(
                    operation_constraint_readiness(t, None, declaration),
                    OperationReadiness::BlockedByUnknownConstraints(_)
                ),
                "{file}: {} is blocked",
                declaration.operation
            );
        }
        // Every placement block is unknown with no operative content, and no candidate on a
        // placement block claims more than a specimen observation.
        for block in PLACEMENT_BLOCK {
            assert!(!established_by_name(t, block), "{file}: {block}");
        }
        assert_eq!(
            t.discovery
                .candidate
                .as_ref()
                .and_then(|c| c.scope.as_deref()),
            Some("specimen-observation"),
            "{file}: a summary line is not a corpus invariant"
        );
        // Identity: the DB half is measured and the file half is prose. On the logo profile the
        // block is unknown with provenance and no proposal; on the jersey profile `/4` lets the
        // measured composite be carried as a candidate, and it is still unknown.
        assert!(!t.identity.provenance.is_empty());
        assert_eq!(t.identity.candidate.is_some(), identity_candidate, "{file}");
        assert!(t.identity.established().is_none());
        assert!(t.install.candidate.is_none() && t.runtime_lookup.candidate.is_none());
    }
}

#[test]
fn the_synthetic_fixture_is_excluded_from_registry_population() {
    let committed = committed_profile();
    let synthetic = synthetic_profile();
    let population = asset_registry_population([&committed, &synthetic]);
    assert_eq!(
        population.len(),
        4,
        "only the committed profile's targets are counted (three declared, one by reference)"
    );
    assert!(population.iter().all(|p| !p.target.contains("fixture")));
}

#[test]
fn the_family_vocabulary_matches_the_qa_matrix() {
    // The profile's `family` is a coverage key. If this crate's copy drifted from the matrix, a
    // profile could register itself against an axis the ledger does not have.
    let matrix: Vec<&str> = tdb_qa::catalog::FAMILIES.iter().map(|f| f.id).collect();
    assert_eq!(ASSET_FAMILY, matrix.as_slice());
}

#[test]
fn asset_representation_completeness_is_reported_apart_from_population() {
    // Format completeness: every asset constraint Q07a.1 names has a representation.
    let (represented, required) = asset_representation_completeness();
    assert_eq!(represented, required);
    assert_eq!(required, Q07A1_ASSET_REPRESENTATION.len());
    let mut requirements: BTreeSet<&str> = BTreeSet::new();
    for (requirement, location) in Q07A1_ASSET_REPRESENTATION {
        assert!(!requirement.is_empty() && !location.is_empty());
        assert!(
            requirements.insert(requirement),
            "{requirement} listed twice"
        );
    }

    // Population: what the committed profile actually declares. Since C0-POR1 that is no longer
    // nothing - but it is still far short of representation, which is the point this test makes.
    // Representing 26 constraints says nothing about populating them, in either direction.
    let profile = committed_profile();
    let population = asset_registry_population(std::iter::once(&profile));
    let established: usize = population.iter().map(|p| p.established_constraints).sum();
    let unknown: usize = population.iter().map(|p| p.unknown_constraints).sum();
    assert_eq!(
        established, 3,
        "after C0-POR1-R1 scoping: 3 on nhllegacy-ps3, 0 on the two derived-tree targets"
    );
    assert!(
        unknown > established,
        "most portrait constraints are still unknown: {unknown} unknown vs {established} established"
    );
    assert!(
        represented > 0,
        "representing 26 constraints says nothing about populating them"
    );
}

#[test]
fn the_synthetic_fixture_exercises_every_representation_the_registry_does_not() {
    // Each Q07a.1 asset requirement is reachable as a parsed value on the synthetic target.
    let profile = synthetic_profile();
    let one = target(&profile, "fixtureone-synthetic");
    let two = target(&profile, "fixturetwo-synthetic");

    let discovery = one.discovery.established().expect("discovery");
    assert_eq!(discovery.scope, "archive-member");
    let identity = one.identity.established().expect("identity");
    assert_eq!(identity.ambiguity, "reject");
    let container = one.container.established().expect("container");
    assert_eq!(container.layers.len(), 2, "archives and their indexes");
    assert_eq!(container.layers[0].index, "offset-table");
    let path = one.path_rule.established().expect("path_rule");
    assert_eq!(path.families, vec!["heads"], "paths");
    assert_eq!(one.path_families[0].tokens, vec!["player-slug"], "slugs");
    assert_eq!(
        two.path_families[0].tokens,
        vec!["team-id", "variant"],
        "IDs"
    );
    let raster = one.raster.established().expect("raster");
    assert_eq!(raster.codec, "dxt5", "raster formats");
    assert_eq!(raster.channels.len(), 4, "channels");
    let dimensions = one.dimensions.established().expect("dimensions");
    assert_eq!((dimensions.width, dimensions.height), (256, 256));
    let mipmaps = one.mipmaps.established().expect("mipmaps");
    assert_eq!(mipmaps.count, Some(5));
    assert_eq!(
        one.compression.established().expect("compression").payload,
        "chunkzip"
    );
    assert_eq!(one.tiling.established().expect("tiling").kind, "gnf-tiled");
    let atlas = one.atlas.established().expect("atlas");
    assert_eq!(atlas.sharing, "shared-surface");
    assert_eq!(atlas.kind.as_deref(), Some("atlas"));
    assert_eq!(
        two.atlas.established().expect("atlas").kind.as_deref(),
        Some("pool")
    );
    let mesh = one.mesh_layout.established().expect("mesh_layout");
    assert_eq!(mesh.vertex_attributes.len(), 6, "mesh vertex layouts");
    assert_eq!(mesh.lod_levels, 3, "LODs");
    assert_eq!(one.uv.established().expect("uv").origin, "top-left");
    let rig = one.rig.established().expect("rig");
    assert_eq!(
        (rig.bone_count, rig.palette_size),
        (122, 64),
        "bones/palettes"
    );
    assert_eq!(rig.max_weights_per_vertex, 4, "weights");
    let audio = one.audio.established().expect("audio");
    assert_eq!((audio.sample_rate_hz, audio.channels), (48000, 2));
    let install = one.install.established().expect("install");
    assert_eq!(install.repack, "index-update", "repack rules");
    assert_eq!(install.kind, "archive-append-relocate", "install rules");
}

#[test]
fn a_decided_absence_satisfies_a_requirement_but_an_unknown_never_does() {
    let profile = synthetic_profile();
    let two = target(&profile, "fixturetwo-synthetic");

    let export = two
        .operations
        .iter()
        .find(|o| o.operation == "export")
        .expect("declared");
    assert!(export.requires.contains(&"mipmaps".to_string()));
    assert_eq!(two.mipmaps.state, ConstraintState::NotApplicable);
    assert_eq!(
        operation_constraint_readiness(two, None, export),
        OperationReadiness::ConstraintsEstablished,
        "\"this target has no mip chain\" is a fact an exporter can act on"
    );

    let rebuild = two
        .operations
        .iter()
        .find(|o| o.operation == "rebuild")
        .expect("declared");
    assert!(two.mesh_layout.candidate.is_some());
    assert_eq!(
        operation_constraint_readiness(two, None, rebuild),
        OperationReadiness::BlockedByUnknownConstraints(vec![
            "mesh_layout".to_string(),
            "uv".to_string()
        ]),
        "a candidate satisfies no requirement"
    );
}

#[test]
fn readiness_never_states_that_studio_can_perform_an_operation() {
    // The type has two variants and neither is about Studio. This test exists so that adding a
    // "supported"/"implemented" variant later has to break it deliberately.
    let profile = synthetic_profile();
    let one = target(&profile, "fixtureone-synthetic");
    let install = one
        .operations
        .iter()
        .find(|o| o.operation == "install")
        .expect("declared");
    let readiness = operation_constraint_readiness(one, None, install);
    assert_eq!(readiness, OperationReadiness::ConstraintsEstablished);
    let rendered = format!("{readiness:?}").to_ascii_lowercase();
    for forbidden in ["support", "implement", "verified", "pass"] {
        assert!(
            !rendered.contains(forbidden),
            "readiness must not read as a capability claim: {rendered}"
        );
    }
}

// =========================================================== version transition

#[test]
fn a_v1_asset_profile_is_not_validated_in_place() {
    let v1_text = fixture("asset-profiles/v1/assets-portraits.toml");
    // It does not even parse as v2: v1 carries keys v2 does not define, and unknown keys are
    // refused rather than dropped.
    assert!(parse_asset_profile(&v1_text).is_err());

    // And a document that merely claims an older schema id is refused with the RIGHT transition
    // named. There are two of them now, and pointing at the wrong one would send an author through
    // a translation that cannot read their document.
    for (schema, transition) in [
        (ASSET_PROFILE_SCHEMA_LEGACY, "upgrade_asset_profile_v1"),
        (ASSET_PROFILE_SCHEMA_PREVIOUS, "upgrade_asset_profile_v2"),
    ] {
        let mistagged = format!(
            "schema = \"{schema}\"\nprofile_version = 1\nfamily = \
             \"assets-portraits\"\n{RASTER_SOURCE}"
        );
        let profile = parse_asset_profile(&mistagged).expect("the envelope parses");
        let findings = validate_asset_profile(&profile);
        assert!(
            findings
                .iter()
                .any(|f| f.rule == Rule::UnsupportedSchema && f.detail.contains(transition)),
            "{schema} must name {transition}: {findings:?}"
        );
    }
}

#[test]
fn the_v1_asset_transition_promotes_no_prose_into_a_fact() {
    let v1 = parse_asset_profile_v1(&fixture("asset-profiles/v1/assets-portraits.toml"))
        .expect("v1 parses");
    let upgraded = upgrade_asset_profile_v1(&v1);

    assert_eq!(upgraded.profile.schema, ASSET_PROFILE_SCHEMA);
    assert_eq!(upgraded.profile.family, v1.family);
    assert_eq!(upgraded.profile.targets.len(), v1.targets.len());
    assert!(!upgraded.profile.synthetic);

    // Not one block is established, and not one candidate is manufactured.
    let population = asset_registry_population(std::iter::once(&upgraded.profile));
    for row in &population {
        assert_eq!(row.established_constraints, 0);
        assert_eq!(row.not_applicable_constraints, 0);
        assert_eq!(row.candidate_constraints, 0, "a translation cites nothing");
        assert_eq!(row.unknown_constraints, ASSET_BLOCK.len());
    }

    // Every v1 field that carried content is reported rather than dropped.
    let reported: BTreeSet<&str> = upgraded.notes.iter().map(|n| n.field.as_str()).collect();
    for field in [
        "container_chain",
        "raster_codec",
        "dimensions",
        "mipmaps",
        "swizzle",
        "atlas",
        "install_rule",
        "confidence",
        "qualification",
        "provenance",
    ] {
        assert!(reported.contains(field), "{field} was dropped silently");
    }

    // The v1 string that LOOKED like a v2 vocabulary term is the one most at risk of being waved
    // through. It is reported, not promoted.
    let swizzle = upgraded
        .notes
        .iter()
        .find(|n| n.field == "swizzle" && n.subject.ends_with("nhllegacy-xbox360"))
        .expect("reported");
    assert_eq!(swizzle.value, "xbox-tiled");
    assert!(swizzle.disposition.contains("[target_profile.tiling]"));

    // The converter-capability label is reported as having no destination at all.
    let qualification = upgraded
        .notes
        .iter()
        .find(|n| n.field == "qualification")
        .expect("reported");
    assert!(qualification.disposition.contains("evidence system"));

    // The canonical source describes the PROJECT, not a target, so it does carry across.
    let source = upgraded
        .profile
        .canonical_sources
        .first()
        .expect("carried across");
    assert_eq!(source.class, "raster");
    assert_eq!(source.interchange, "rgba8");

    // The translated document is a faithful translation, not a passing one: it validates cleanly
    // only because every block is honestly unknown.
    assert_eq!(validate_asset_profile(&upgraded.profile), vec![]);
}

#[test]
fn an_unexpressible_v1_canonical_source_is_dropped_and_the_profile_then_refused() {
    let v1 = parse_asset_profile_v1(
        "schema = \"asset-output-profile/1\"\nprofile_version = 1\nfamily = \"assets-audio\"\n\
         [canonical_source]\nkind = \"waveform\"\nformat = \"aiff\"\n",
    )
    .expect("v1 parses");
    let upgraded = upgrade_asset_profile_v1(&v1);
    assert!(
        upgraded.profile.canonical_sources.is_empty(),
        "a class outside the vocabulary is dropped, never guessed"
    );
    assert!(upgraded.notes.iter().any(|n| n.field == "kind"));
    assert!(upgraded.notes.iter().any(|n| n.field == "format"));
    let rules: Vec<Rule> = validate_asset_profile(&upgraded.profile)
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(rules.contains(&Rule::AssetOperandsIncomplete));
}

// ================================================================== negative

const RASTER_SOURCE: &str = "\n[[canonical_source]]\nrole = \"portrait\"\nclass = \"raster\"\n\
                             interchange = \"rgba8\"\nretention = \"unmodified\"\n";
const MESH_SOURCE: &str = "\n[[canonical_source]]\nrole = \"head\"\nclass = \"mesh\"\n\
                           interchange = \"mesh-canonical\"\nretention = \"unmodified\"\n";
const AUDIO_SOURCE: &str = "\n[[canonical_source]]\nrole = \"clip\"\nclass = \"audio\"\n\
                            interchange = \"pcm-s16\"\nretention = \"unmodified\"\n";

/// The corpus every negative-case body cites. Attributable, so a block that names it is stopped by
/// whatever the case is actually testing rather than by its corpus binding.
const FIXTURE_CORPUS: &str = "\n[[target_profile.corpus]]\nid = \"fixture-corpus\"\nkind = \
                              \"observed-installed-copy\"\nattribution = \
                              \"attributable-to-target\"\nbuild_identification = \"complete\"\n\
                              identity_source = \"synthetic:header\"\ninspected_files = 8\n\
                              provenance = [\"x\"]\n";

/// The three keys `/3` requires on every established block, spelled once.
const SCOPED: &str = "scope = \"inspected-corpus-invariant\"\nforce = \
                      \"observed-source-format\"\ncorpus = \"fixture-corpus\"\n";

/// A whole profile document around one target profile body.
fn doc(sources: &str, body: &str) -> String {
    format!(
        "schema = \"asset-output-profile/3\"\nprofile_version = 1\nfamily = \
         \"assets-portraits\"\n{sources}\n[[target_profile]]\ntarget = \
         \"fixture-synthetic\"\n{FIXTURE_CORPUS}\n{body}\n"
    )
}

fn rules_of(text: &str) -> Vec<Rule> {
    let profile = parse_asset_profile(text).unwrap_or_else(|e| panic!("parses: {e}\n{text}"));
    validate_asset_profile(&profile)
        .into_iter()
        .map(|f| f.rule)
        .collect()
}

/// The rules a raster-only target body produces.
fn raster_rules(body: &str) -> Vec<Rule> {
    rules_of(&doc(RASTER_SOURCE, body))
}

#[test]
fn a_converter_status_key_cannot_even_be_written_in_a_profile() {
    // This is the machine enforcement of "format capability is not converter capability": v1's
    // per-target `qualification` and `confidence` have no v2 spelling, and `deny_unknown_fields`
    // makes writing one a PARSE error rather than a validation warning.
    for key in [
        "qualification = \"implemented\"",
        "confidence = \"measured\"",
        "supported = true",
    ] {
        let text = doc(RASTER_SOURCE, key);
        assert!(
            parse_asset_profile(&text).is_err(),
            "{key} must not be expressible in a target profile"
        );
    }
}

#[test]
fn a_misspelled_asset_constraint_key_is_a_parse_error_not_a_silent_drop() {
    let text = doc(
        RASTER_SOURCE,
        "[target_profile.raster]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenence = [\"x\"]\n[target_profile.raster.value]\ncodec = \"dxt5\"\n\
         color_space = \"srgb\"\nchannels = [\"r\"]\nalpha = \"none\"\n",
    );
    assert!(parse_asset_profile(&text).is_err());
}

#[test]
fn an_unknown_family_or_a_zero_version_is_refused() {
    let text = "schema = \"asset-output-profile/3\"\nprofile_version = 0\nfamily = \
                \"assets-mascots\"\n"
        .to_string()
        + RASTER_SOURCE;
    let rules = rules_of(&text);
    assert!(rules.contains(&Rule::AssetFamilyUnknown));
    assert!(rules.contains(&Rule::UnsupportedVersion));
}

#[test]
fn a_profile_must_declare_at_least_one_canonical_source() {
    let rules = rules_of(&doc("", ""));
    assert!(rules.contains(&Rule::AssetOperandsIncomplete));
}

#[test]
fn a_duplicate_source_role_or_target_profile_is_refused() {
    let duplicated_role = format!(
        "schema = \"asset-output-profile/3\"\nprofile_version = 1\nfamily = \
         \"assets-portraits\"\n{RASTER_SOURCE}{RASTER_SOURCE}"
    );
    assert!(rules_of(&duplicated_role).contains(&Rule::DuplicateSourceRole));

    let duplicated_target =
        doc(RASTER_SOURCE, "") + "\n[[target_profile]]\ntarget = \"fixture-synthetic\"\n";
    assert!(rules_of(&duplicated_target).contains(&Rule::DuplicateTargetProfile));
}

#[test]
fn an_interchange_representation_must_belong_to_its_declared_class() {
    let text = "schema = \"asset-output-profile/3\"\nprofile_version = 1\nfamily = \
                \"assets-portraits\"\n\n[[canonical_source]]\nrole = \"portrait\"\nclass = \
                \"raster\"\ninterchange = \"mesh-canonical\"\nretention = \"unmodified\"\n";
    assert!(rules_of(text).contains(&Rule::ConstraintKindMismatch));
}

#[test]
fn a_malformed_target_is_refused_in_a_profile_too() {
    let text = format!(
        "schema = \"asset-output-profile/3\"\nprofile_version = 1\nfamily = \
         \"assets-portraits\"\n{RASTER_SOURCE}\n[[target_profile]]\ntarget = \"nhl15\"\n"
    );
    assert!(rules_of(&text).contains(&Rule::MalformedTarget));
}

#[test]
fn a_block_belonging_to_an_undeclared_asset_class_is_refused() {
    // A portrait family has no rig question at all, so even declaring the rig ABSENT is a category
    // error rather than a fact about the target.
    let rules = raster_rules(
        "[target_profile.rig]\nstate = \"not-applicable\"\nreason = \"portraits are not skinned\"\n",
    );
    assert!(rules.contains(&Rule::ConstraintKindMismatch));

    // And requiring it from an operation is refused for the same reason.
    let rules = raster_rules(
        "[[target_profile.operation]]\noperation = \"rebuild\"\nrequires = [\"rig\"]\n",
    );
    assert!(rules.contains(&Rule::ConstraintKindMismatch));

    // The same block is fine once the profile declares a mesh source.
    let ok = rules_of(&doc(
        MESH_SOURCE,
        "[target_profile.rig]\nstate = \"not-applicable\"\nreason = \"this LOD is a static prop\"\n",
    ));
    assert!(!ok.contains(&Rule::ConstraintKindMismatch));
}

#[test]
fn the_shared_evidence_envelope_applies_to_asset_blocks() {
    // established with no value, no confidence and no provenance
    let rules = raster_rules("[target_profile.dimensions]\nstate = \"established\"\n");
    assert!(rules.contains(&Rule::EstablishedWithoutValue));
    assert!(rules.contains(&Rule::EstablishedWithoutConfidence));
    assert!(rules.contains(&Rule::MissingProvenance));

    // absence with no account of how it was determined
    let rules = raster_rules("[target_profile.atlas]\nstate = \"not-applicable\"\n");
    assert!(rules.contains(&Rule::NotApplicableWithoutReason));

    // a value smuggled in under unknown
    let rules = raster_rules(
        "[target_profile.tiling]\nstate = \"unknown\"\n[target_profile.tiling.value]\nkind = \
         \"linear\"\n",
    );
    assert!(rules.contains(&Rule::ValueWithoutEstablishedState));
}

#[test]
fn an_asserted_asset_claim_cannot_be_operative_and_a_derived_one_must_argue() {
    let asserted = raster_rules(
        "[target_profile.tiling]\nstate = \"established\"\nconfidence = \"asserted\"\n\
         provenance = [\"x\"]\n[target_profile.tiling.value]\nkind = \"linear\"\n",
    );
    assert!(asserted.contains(&Rule::AssertedAsOperative));

    let underived = raster_rules(
        "[target_profile.tiling]\nstate = \"established\"\nconfidence = \"derived\"\n\
         provenance = [\"x\"]\n[target_profile.tiling.value]\nkind = \"linear\"\n",
    );
    assert!(underived.contains(&Rule::DerivationUnexplained));

    let argued = raster_rules(
        "[target_profile.tiling]\nstate = \"established\"\nconfidence = \"derived\"\n\
         derivation = \"PREMISES: … APPLICABILITY: … NOT ESTABLISHED: …\"\nprovenance = [\"x\"]\n\
         [target_profile.tiling.value]\nkind = \"linear\"\n",
    );
    assert!(!argued.contains(&Rule::DerivationUnexplained));
    assert!(!argued.contains(&Rule::AssertedAsOperative));
}

#[test]
fn an_asset_candidate_obeys_the_same_rules_as_a_database_one() {
    let decided = raster_rules(
        "[target_profile.tiling]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[target_profile.tiling.value]\nkind = \"linear\"\n\
         [target_profile.tiling.candidate]\nconfidence = \"asserted\"\nprovenance = [\"y\"]\n\
         [target_profile.tiling.candidate.value]\nkind = \"gnf-tiled\"\n",
    );
    assert!(decided.contains(&Rule::CandidateOnDecidedConstraint));

    let uncited = raster_rules(
        "[target_profile.tiling]\nstate = \"unknown\"\n[target_profile.tiling.candidate]\n\
         [target_profile.tiling.candidate.value]\nkind = \"linear\"\n",
    );
    assert!(uncited.contains(&Rule::CandidateIncomplete));
}

#[test]
fn a_container_chain_must_be_a_chain_and_only_its_innermost_layer_may_lack_an_index() {
    let empty = raster_rules(&format!(
        "[target_profile.container]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.container.value]\n\
         outermost_layer_examined = true\n"
    ));
    assert!(empty.contains(&Rule::AssetOperandsIncomplete));

    // New in `/3`: a chain whose outer end was never examined cannot be established. C0-POR1
    // measured `GNF ` files pulled out of a `.big`, knew an outer layer existed, and `/2` had
    // nowhere to say so - the chain read as complete.
    let unexamined = raster_rules(&format!(
        "[target_profile.container]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.container.value]\n\
         outermost_layer_examined = false\n[[target_profile.container.value.layer]]\nformat = \
         \"gnf\"\nindex = \"none\"\n"
    ));
    assert!(unexamined.contains(&Rule::AssetOperandsIncomplete));

    let unaddressed = raster_rules(&format!(
        "[target_profile.container]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.container.value]\n\
         outermost_layer_examined = true\n[[target_profile.container.value.layer]]\nformat = \
         \"bigf\"\nindex = \"none\"\n[[target_profile.container.value.layer]]\nformat = \
         \"rx2\"\nindex = \"none\"\n"
    ));
    assert!(unaddressed.contains(&Rule::AssetBoundsInconsistent));
}

#[test]
fn a_path_template_and_its_token_list_must_agree_in_both_directions() {
    let family = |template: &str, tokens: &str| {
        format!(
            "[[target_profile.path_family]]\nid = \"portraits\"\ntemplate = \"{template}\"\n\
             tokens = {tokens}\ncase = \"lower\"\nbucket_resolution = \"template-only\"\n"
        )
    };
    assert!(
        raster_rules(&family("p{numeric-id}.big", "[]")).contains(&Rule::AssetOperandsIncomplete)
    );
    assert!(raster_rules(&family("portrait.big", "[\"numeric-id\"]"))
        .contains(&Rule::AssetOperandsIncomplete));
    assert!(raster_rules(&family("{jersey}.big", "[\"jersey\"]"))
        .contains(&Rule::UnknownVocabularyTerm));
    let ok = raster_rules(&family("p{numeric-id}.big", "[\"numeric-id\"]"));
    assert!(!ok.contains(&Rule::AssetOperandsIncomplete));
    assert!(!ok.contains(&Rule::UnknownVocabularyTerm));
}

#[test]
fn observed_bucket_membership_is_never_an_id_to_directory_rule() {
    // The C0-POR1-R1 finding, enforced. Observing which ids sit in which directory is membership.
    // C0-POR1 turned a mismatch between a folder's name and its contents into the finding "the rule
    // is not the literal folder-name ranges"; no candidate rule was ever formulated or tested, and
    // `/3` gives membership a home that cannot be read as a function.
    let observed = "[[target_profile.path_family]]\nid = \"portraits\"\ntemplate = \
                    \"{variant}/p{numeric-id}.big\"\ntokens = [\"variant\", \"numeric-id\"]\n\
                    case = \"lower\"\nbucket_resolution = \"unknown\"\n\
                    [[target_profile.path_family.observed_bucket]]\ndirectory = \"p8001_12000\"\n\
                    census = \"docs/example-census.json#membership\"\ncensus_digest = \"\
                    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"\nscope = \
                    \"inspected-corpus-invariant\"\ncorpus = \"fixture-corpus\"\n";

    // Recording it is legal and produces nothing.
    assert!(raster_rules(observed).is_empty());

    // Establishing [path_rule] through it is not: membership cannot resolve a path.
    let resolved = format!(
        "{observed}[target_profile.path_rule]\nstate = \"established\"\nconfidence = \
         \"measured\"\n{SCOPED}provenance = [\"x\"]\n[target_profile.path_rule.value]\nfamilies = \
         [\"portraits\"]\nfamily_selection = \"by role\"\n"
    );
    assert!(raster_rules(&resolved).contains(&Rule::AssetPathFamilyUnresolved));

    // A bucket citing a corpus the target does not declare is refused: an observation with no
    // recorded source is not an observation.
    let uncited = observed.replace("fixture-corpus", "some-other-corpus");
    assert!(raster_rules(&uncited).contains(&Rule::AssetCorpusUndeclared));

    // And a family claiming its template resolves paths by itself may not also observe buckets the
    // template cannot produce.
    let contradictory = observed.replace("\"unknown\"", "\"template-only\"");
    assert!(raster_rules(&contradictory).contains(&Rule::AssetPathFamilyUnresolved));

    // C0-AS3-R1: the DETAILED membership lives in the census, and the profile keeps a reference to
    // it. A reference with no usable digest names whatever the path holds today rather than the
    // observation somebody made, which is not a citation.
    let undigested = observed.replace(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "see the receipt",
    );
    assert!(raster_rules(&undigested).contains(&Rule::AssetPathFamilyUnresolved));

    // And membership is never something the TARGET requires. A directory holding an id is
    // something somebody saw; the target requiring it to be there is a different claim entirely,
    // and no reading of a folder listing reaches it.
    let overclaimed = observed.replace(
        "scope = \"inspected-corpus-invariant\"\ncorpus",
        "scope = \"supported-target-constraint\"\ncorpus",
    );
    assert!(raster_rules(&overclaimed).contains(&Rule::AssetPathFamilyUnresolved));
}

#[test]
fn a_target_may_declare_several_path_families_and_path_rule_names_the_ones_it_uses() {
    // `/2` held ONE template per target, which is why the two portrait families could not both be
    // recorded and `[path_rule]` had to carry the gap in a note.
    let two = "[[target_profile.path_family]]\nid = \"playerheads\"\ntemplate = \
               \"playerheads/p{numeric-id}.big\"\ntokens = [\"numeric-id\"]\ncase = \"lower\"\n\
               bucket_resolution = \"template-only\"\n[[target_profile.path_family]]\nid = \
               \"playerheadssmall\"\ntemplate = \"playerheadssmall/p{numeric-id}.big\"\ntokens = \
               [\"numeric-id\"]\ncase = \"lower\"\nbucket_resolution = \"template-only\"\n";
    let rule = format!(
        "{two}[target_profile.path_rule]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.path_rule.value]\nfamilies = \
         [\"playerheads\", \"playerheadssmall\"]\nfamily_selection = \"by portrait size class\"\n"
    );
    assert!(raster_rules(&rule).is_empty(), "{:?}", raster_rules(&rule));

    // Naming a family that is not declared is refused rather than ignored.
    let missing = rule.replace(
        "\"playerheadssmall\"]\nfamily_selection",
        "\"playerheadslarge\"]\nfamily_selection",
    );
    assert!(raster_rules(&missing).contains(&Rule::AssetPathFamilyUnresolved));

    // Two families sharing an id make a citation ambiguous.
    let duplicated = two.replace("playerheadssmall\"\ntemplate", "playerheads\"\ntemplate");
    assert!(raster_rules(&duplicated).contains(&Rule::AssetPathFamilyUnresolved));
}

#[test]
fn a_slug_path_cannot_be_established_while_the_identity_it_names_is_not() {
    let family = "[[target_profile.path_family]]\nid = \"portraits\"\ntemplate = \
                  \"{player-slug}.big\"\ntokens = [\"player-slug\"]\ncase = \"lower\"\n\
                  bucket_resolution = \"template-only\"\n";
    let rule = format!(
        "{family}[target_profile.path_rule]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.path_rule.value]\nfamilies = \
         [\"portraits\"]\nfamily_selection = \"by role\"\n"
    );
    assert!(raster_rules(&rule).contains(&Rule::AssetConstraintDependencyUnmet));

    let wrong_key = format!(
        "{rule}[target_profile.identity]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.identity.value]\nkey = \"numeric-id\"\n\
         source = \"t.f\"\nambiguity = \"reject\"\n"
    );
    assert!(raster_rules(&wrong_key).contains(&Rule::AssetConstraintDependencyUnmet));

    let matched = format!(
        "{rule}[target_profile.identity]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.identity.value]\nkey = \"player-slug\"\n\
         source = \"t.f\"\nambiguity = \"reject\"\n"
    );
    assert!(!raster_rules(&matched).contains(&Rule::AssetConstraintDependencyUnmet));
}

#[test]
fn an_ambiguous_match_may_never_be_resolved_silently() {
    let rules = raster_rules(
        "[target_profile.identity]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[target_profile.identity.value]\nkey = \"numeric-id\"\nsource = \
         \"t.f\"\nambiguity = \"first-match\"\n",
    );
    assert!(rules.contains(&Rule::UnknownVocabularyTerm));
}

fn raster_block(codec: &str, channels: &str, alpha: &str) -> String {
    format!(
        "[target_profile.raster]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.raster.value]\ncodec = \"{codec}\"\n\
         color_space = \"srgb\"\nchannels = {channels}\nalpha = \"{alpha}\"\n"
    )
}

#[test]
fn a_raster_channel_set_must_agree_with_its_alpha_mode_and_its_codec() {
    assert!(
        raster_rules(&raster_block("dxt5", "[\"r\", \"g\", \"b\"]", "straight"))
            .contains(&Rule::AssetBoundsInconsistent)
    );
    assert!(raster_rules(&raster_block(
        "dxt5",
        "[\"r\", \"g\", \"b\", \"a\"]",
        "none"
    ))
    .contains(&Rule::AssetBoundsInconsistent));
    assert!(
        raster_rules(&raster_block("l8", "[\"r\", \"a\"]", "straight"))
            .contains(&Rule::AssetBoundsInconsistent)
    );
    assert!(raster_rules(&raster_block("a8", "[\"r\"]", "none"))
        .contains(&Rule::AssetBoundsInconsistent));
    assert!(
        raster_rules(&raster_block("dxt5", "[]", "none")).contains(&Rule::AssetOperandsIncomplete)
    );
    assert!(raster_rules(&raster_block(
        "dxt5",
        "[\"r\", \"r\", \"g\", \"b\"]",
        "none"
    ))
    .contains(&Rule::AssetBoundsInconsistent));
    // The well-formed case produces neither.
    let ok = raster_rules(&raster_block(
        "dxt5",
        "[\"r\", \"g\", \"b\", \"a\"]",
        "straight",
    ));
    assert!(!ok.contains(&Rule::AssetBoundsInconsistent));
    assert!(!ok.contains(&Rule::AssetOperandsIncomplete));
}

fn dimensions_block(width: u32, height: u32, power_of_two: bool) -> String {
    format!(
        "[target_profile.dimensions]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.dimensions.value]\nwidth = {width}\n\
         height = {height}\nvariant = \"detail\"\npower_of_two = {power_of_two}\n"
    )
}

#[test]
fn dimensions_must_be_real_and_must_fit_the_codec_that_stores_them() {
    assert!(raster_rules(&dimensions_block(0, 128, false)).contains(&Rule::AssetBoundsInconsistent));
    assert!(raster_rules(&dimensions_block(96, 64, true)).contains(&Rule::AssetBoundsInconsistent));

    // A block-compressed codec cannot store a partial block, and both facts are established here.
    let mismatched = format!(
        "{}{}",
        dimensions_block(130, 128, false),
        raster_block("dxt5", "[\"r\", \"g\", \"b\", \"a\"]", "straight")
    );
    assert!(raster_rules(&mismatched).contains(&Rule::AssetBoundsInconsistent));

    let aligned = format!(
        "{}{}",
        dimensions_block(128, 128, true),
        raster_block("dxt5", "[\"r\", \"g\", \"b\", \"a\"]", "straight")
    );
    assert!(!raster_rules(&aligned).contains(&Rule::AssetBoundsInconsistent));
}

fn mipmap_block(policy: &str, count: &str) -> String {
    format!(
        "[target_profile.mipmaps]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.mipmaps.value]\npolicy = \"{policy}\"\n\
         {count}"
    )
}

#[test]
fn a_mip_policy_owns_its_level_count() {
    assert!(raster_rules(&mipmap_block("fixed-count", "")).contains(&Rule::AssetOperandsIncomplete));
    assert!(raster_rules(&mipmap_block("full-chain", "count = 4\n"))
        .contains(&Rule::AssetBoundsInconsistent));
    assert!(raster_rules(&mipmap_block("fixed-count", "count = 0\n"))
        .contains(&Rule::AssetBoundsInconsistent));
    // 64x64 admits 7 levels; 9 is impossible.
    let impossible = format!(
        "{}{}",
        mipmap_block("fixed-count", "count = 9\n"),
        dimensions_block(64, 64, true)
    );
    assert!(raster_rules(&impossible).contains(&Rule::AssetBoundsInconsistent));
    let possible = format!(
        "{}{}",
        mipmap_block("fixed-count", "count = 7\n"),
        dimensions_block(64, 64, true)
    );
    assert!(!raster_rules(&possible).contains(&Rule::AssetBoundsInconsistent));
}

#[test]
fn a_tiling_alignment_must_be_a_power_of_two() {
    let rules = raster_rules(
        "[target_profile.tiling]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[target_profile.tiling.value]\nkind = \"xbox360-tiled\"\n\
         alignment_bytes = 96\n",
    );
    assert!(rules.contains(&Rule::AssetBoundsInconsistent));
}

fn atlas_block(body: &str) -> String {
    format!(
        "[target_profile.atlas]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.atlas.value]\n{body}"
    )
}

#[test]
fn an_atlas_or_pool_must_carry_exactly_the_operands_its_kind_defines() {
    assert!(raster_rules(&atlas_block(
        "sharing = \"per-asset-surface\"\nkind = \"none\"\npage_width = 512\n"
    ))
    .contains(&Rule::AssetOperandsIncomplete));
    assert!(raster_rules(&atlas_block(
        "sharing = \"shared-surface\"\nkind = \"atlas\"\npage_width = 512\npage_height = 512\n"
    ))
    .contains(&Rule::AssetOperandsIncomplete));
    assert!(raster_rules(&atlas_block(
        "sharing = \"shared-surface\"\nkind = \"atlas\"\npage_width = 512\npage_height = \
         512\ncell_x = 384\ncell_y = 0\ncell_width = 256\ncell_height = 64\n"
    ))
    .contains(&Rule::AssetBoundsInconsistent));
    assert!(raster_rules(&atlas_block(
        "sharing = \"shared-surface\"\nkind = \"pool\"\npage_width = 512\npage_height = 512\n"
    ))
    .contains(&Rule::AssetOperandsIncomplete));
    assert!(raster_rules(&atlas_block(
        "sharing = \"shared-surface\"\nkind = \"pool\"\npage_width = 512\npage_height = \
         512\nslot = 3\ncell_x = 0\n"
    ))
    .contains(&Rule::AssetOperandsIncomplete));

    let ok = raster_rules(&atlas_block(
        "sharing = \"per-asset-surface\"\nkind = \"none\"\n",
    ));
    assert!(!ok.contains(&Rule::AssetOperandsIncomplete));
    assert!(!ok.contains(&Rule::AssetBoundsInconsistent));
}

fn mesh_block(stride: u32, attributes: &str, lods: u32) -> String {
    format!(
        "[target_profile.mesh_layout]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.mesh_layout.value]\nvertex_stride_bytes = \
         {stride}\nindex_format = \"u16\"\nwinding = \"ccw\"\ncoordinate_system = \"y-up-lh\"\n\
         unit = \"centimetre\"\nlod_levels = {lods}\n{attributes}"
    )
}

const POSITION_ATTRIBUTE: &str = "\n[[target_profile.mesh_layout.value.vertex_attribute]]\n\
                                  semantic = \"position\"\ncomponent_type = \"f32\"\n\
                                  component_count = 3\nbyte_offset = 0\n";

fn mesh_rules(body: &str) -> Vec<Rule> {
    rules_of(&doc(MESH_SOURCE, body))
}

#[test]
fn a_vertex_attribute_must_fit_the_stride_and_be_declared_once() {
    assert!(
        mesh_rules(&mesh_block(8, POSITION_ATTRIBUTE, 1)).contains(&Rule::AssetBoundsInconsistent)
    );
    assert!(mesh_rules(&mesh_block(
        12,
        &format!("{POSITION_ATTRIBUTE}{POSITION_ATTRIBUTE}"),
        1
    ))
    .contains(&Rule::AssetBoundsInconsistent));
    assert!(mesh_rules(&mesh_block(
        32,
        "\n[[target_profile.mesh_layout.value.vertex_attribute]]\nsemantic = \"position\"\n\
         component_type = \"f32\"\ncomponent_count = 5\nbyte_offset = 0\n",
        1
    ))
    .contains(&Rule::AssetBoundsInconsistent));
    assert!(
        mesh_rules(&mesh_block(12, POSITION_ATTRIBUTE, 0)).contains(&Rule::AssetBoundsInconsistent)
    );
    assert!(mesh_rules(&mesh_block(12, "", 1)).contains(&Rule::AssetOperandsIncomplete));

    let ok = mesh_rules(&mesh_block(12, POSITION_ATTRIBUTE, 1));
    assert!(!ok.contains(&Rule::AssetBoundsInconsistent));
    assert!(!ok.contains(&Rule::AssetOperandsIncomplete));
}

#[test]
fn a_uv_convention_cannot_claim_more_sets_than_the_vertex_carries() {
    let uv = "[target_profile.uv]\nstate = \"established\"\nconfidence = \"measured\"\n\
              provenance = [\"x\"]\n[target_profile.uv.value]\norigin = \"top-left\"\nwrap = \
              \"repeat\"\nchannel_count = 2\n";
    let body = format!("{uv}{}", mesh_block(12, POSITION_ATTRIBUTE, 1));
    assert!(mesh_rules(&body).contains(&Rule::AssetConstraintDependencyUnmet));

    let zero = "[target_profile.uv]\nstate = \"established\"\nconfidence = \"measured\"\n\
                provenance = [\"x\"]\n[target_profile.uv.value]\norigin = \"top-left\"\nwrap = \
                \"repeat\"\nchannel_count = 0\n";
    assert!(mesh_rules(zero).contains(&Rule::AssetBoundsInconsistent));
}

fn rig_block(bones: u32, palette: u32, weights: u32) -> String {
    format!(
        "[target_profile.rig]\nstate = \"established\"\nconfidence = \"measured\"\nprovenance = \
         [\"x\"]\n[target_profile.rig.value]\nbone_count = {bones}\npalette_size = {palette}\n\
         max_weights_per_vertex = {weights}\nweight_quantization = \"f32\"\nbind_pose = \
         \"model-space\"\n"
    )
}

#[test]
fn a_palette_cannot_exceed_its_rig_and_weights_cannot_exceed_the_vertex() {
    assert!(mesh_rules(&rig_block(60, 64, 4)).contains(&Rule::AssetBoundsInconsistent));
    assert!(mesh_rules(&rig_block(122, 64, 9)).contains(&Rule::AssetBoundsInconsistent));
    assert!(mesh_rules(&rig_block(122, 0, 4)).contains(&Rule::AssetBoundsInconsistent));

    // The vertex declares two weight slots; the rig may not allow four.
    let two_slots = "\n[[target_profile.mesh_layout.value.vertex_attribute]]\nsemantic = \
                     \"blend_weight\"\ncomponent_type = \"u8-normalized\"\ncomponent_count = 2\n\
                     byte_offset = 12\n";
    let body = format!(
        "{}{}",
        rig_block(122, 64, 4),
        mesh_block(16, &format!("{POSITION_ATTRIBUTE}{two_slots}"), 1)
    );
    assert!(mesh_rules(&body).contains(&Rule::AssetConstraintDependencyUnmet));
}

fn audio_block(codec: &str, channels: u32, rate: u32, depth: &str) -> String {
    format!(
        "[target_profile.audio]\nstate = \"established\"\nconfidence = \"measured\"\nprovenance = \
         [\"x\"]\n[target_profile.audio.value]\nbank = \"indexed-bank\"\ncodec = \"{codec}\"\n\
         sample_rate_hz = {rate}\nchannels = {channels}\nloop_metadata = \"preserved-opaque\"\n\
         {depth}"
    )
}

fn audio_rules(body: &str) -> Vec<Rule> {
    rules_of(&doc(AUDIO_SOURCE, body))
}

#[test]
fn audio_sample_constraints_must_be_realisable_and_owned_by_the_codec() {
    assert!(
        audio_rules(&audio_block("pcm-s16", 2, 0, "bit_depth = 16\n"))
            .contains(&Rule::AssetBoundsInconsistent)
    );
    assert!(
        audio_rules(&audio_block("pcm-s16", 0, 48000, "bit_depth = 16\n"))
            .contains(&Rule::AssetBoundsInconsistent)
    );
    assert!(
        audio_rules(&audio_block("pcm-s16", 2, 48000, "")).contains(&Rule::AssetOperandsIncomplete)
    );
    assert!(
        audio_rules(&audio_block("xma2", 2, 48000, "bit_depth = 16\n"))
            .contains(&Rule::AssetBoundsInconsistent)
    );
    assert!(
        audio_rules(&audio_block("pcm-s16", 2, 48000, "bit_depth = 12\n"))
            .contains(&Rule::AssetBoundsInconsistent)
    );
    assert!(audio_rules(&audio_block(
        "pcm-s16",
        2,
        48000,
        "bit_depth = 16\nmax_duration_ms = 0\n"
    ))
    .contains(&Rule::AssetBoundsInconsistent));

    let ok = audio_rules(&audio_block("pcm-s16", 2, 48000, "bit_depth = 16\n"));
    assert!(!ok.contains(&Rule::AssetBoundsInconsistent));
    assert!(!ok.contains(&Rule::AssetOperandsIncomplete));
}

fn install_block(kind: &str, size_policy: &str, repack: &str) -> String {
    format!(
        "[target_profile.install]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[target_profile.install.value]\nkind = \"{kind}\"\nsize_policy = \
         \"{size_policy}\"\nrepack = \"{repack}\"\nbackup = \"original-bytes-retained\"\n"
    )
}

const LOOSE_CONTAINER: &str = "[target_profile.container]\nstate = \"established\"\nconfidence = \
                               \"measured\"\nprovenance = [\"x\"]\n\
                               [[target_profile.container.value.layer]]\nformat = \"bigf\"\n\
                               index = \"name-table\"\n";

#[test]
fn an_archive_install_rule_needs_an_established_container() {
    let rules = raster_rules(&install_block(
        "archive-replace-in-place",
        "fit-in-slot",
        "index-update",
    ));
    assert!(rules.contains(&Rule::AssetConstraintDependencyUnmet));

    let with_container = format!(
        "{LOOSE_CONTAINER}{}",
        install_block("archive-replace-in-place", "fit-in-slot", "index-update")
    );
    assert!(!raster_rules(&with_container).contains(&Rule::AssetConstraintDependencyUnmet));
}

#[test]
fn an_install_kind_and_its_size_policy_cannot_contradict_each_other() {
    let grows_in_place = format!(
        "{LOOSE_CONTAINER}{}",
        install_block("archive-replace-in-place", "allow-growth", "index-update")
    );
    assert!(raster_rules(&grows_in_place).contains(&Rule::AssetBoundsInconsistent));

    let relocates_without_growth = format!(
        "{LOOSE_CONTAINER}{}",
        install_block("archive-append-relocate", "fit-in-slot", "index-update")
    );
    assert!(raster_rules(&relocates_without_growth).contains(&Rule::AssetBoundsInconsistent));

    let loose_repacks = install_block("loose-file", "allow-growth", "full-rebuild");
    assert!(raster_rules(&loose_repacks).contains(&Rule::AssetBoundsInconsistent));
}

#[test]
fn an_operation_declaration_is_a_format_requirement_and_never_a_support_claim() {
    let verified = raster_rules(
        "[[target_profile.operation]]\noperation = \"game-acceptance\"\nrequires = \
         [\"container\"]\n",
    );
    assert!(
        verified.contains(&Rule::AdapterClaimsGameVerified),
        "a profile can never reach game acceptance"
    );

    let empty =
        raster_rules("[[target_profile.operation]]\noperation = \"export\"\nrequires = []\n");
    assert!(empty.contains(&Rule::AssetOperandsIncomplete));

    let duplicated = raster_rules(
        "[[target_profile.operation]]\noperation = \"export\"\nrequires = [\"container\"]\n\
         [[target_profile.operation]]\noperation = \"export\"\nrequires = [\"raster\"]\n",
    );
    assert!(duplicated.contains(&Rule::DuplicateOperationDeclaration));

    let unknown_block = raster_rules(
        "[[target_profile.operation]]\noperation = \"export\"\nrequires = [\"palette\"]\n",
    );
    assert!(unknown_block.contains(&Rule::UnknownVocabularyTerm));

    let repeated_block = raster_rules(
        "[[target_profile.operation]]\noperation = \"export\"\nrequires = [\"raster\", \
         \"raster\"]\n",
    );
    assert!(repeated_block.contains(&Rule::AssetBoundsInconsistent));
}

#[test]
fn a_placeholder_is_never_an_asset_identity() {
    let rules = raster_rules(
        "[target_profile.discovery]\nstate = \"established\"\nconfidence = \"measured\"\n\
         provenance = [\"x\"]\n[target_profile.discovery.value]\nscope = \"loose-tree\"\nroot = \
         \"unknown\"\nrecursive = false\n",
    );
    assert!(rules.contains(&Rule::PlaceholderAsValue));
}

#[test]
fn every_asset_rule_has_a_stable_diagnosable_name() {
    let all = [
        Rule::AssetFamilyUnknown,
        Rule::DuplicateTargetProfile,
        Rule::DuplicateSourceRole,
        Rule::AssetBoundsInconsistent,
        Rule::AssetOperandsIncomplete,
        Rule::AssetConstraintDependencyUnmet,
        Rule::DuplicateOperationDeclaration,
        Rule::AssetEvidenceScopeMissing,
        Rule::AssetEvidenceScopeInsufficient,
        Rule::AssetCorpusIncomplete,
        Rule::AssetCorpusUndeclared,
        Rule::AssetCorpusAttributionUnsupported,
        Rule::AssetVariantSelectorIncomplete,
        Rule::AssetVariantSelectorContradiction,
        Rule::AssetVariantBlockConflict,
        Rule::AssetPropertyEvidenceConflict,
        Rule::AssetPathFamilyUnresolved,
        Rule::AssetProvenanceBridgeInvalid,
        Rule::AssetCompositeInvalid,
        Rule::AssetResolutionChainInvalid,
        Rule::AssetReuseUnresolved,
        Rule::AssetReuseSelfReference,
        Rule::AssetReuseChain,
        Rule::AssetReuseConflict,
    ];
    let names: BTreeSet<&str> = all.iter().map(Rule::as_str).collect();
    assert_eq!(names.len(), all.len());
    assert!(names.iter().all(|n| !n.is_empty()));
}

// ============================================== evidence scope, force and corpus attribution

#[test]
fn an_established_block_must_say_how_far_its_evidence_reaches_and_what_it_asserts() {
    // The `/2` gap this closes: a block could be `measured` and read as target-wide, because there
    // was no key in which to say which population the measurement covered.
    let unscoped = raster_rules(
        "[target_profile.tiling]\nstate = \"established\"\nconfidence = \"measured\"\ncorpus = \
         \"fixture-corpus\"\nprovenance = [\"x\"]\n[target_profile.tiling.value]\nkind = \
         \"linear\"\n",
    );
    assert!(unscoped.contains(&Rule::AssetEvidenceScopeMissing));

    // A handful of decoded specimens may be RECORDED - somebody measured those files and that is a
    // true statement about them - and it is never a statement about the target. C0-AS3 refused it
    // outright; C0-AS3-R1 admits it at `specimen-observation` scope, where nothing reads it.
    let specimen_body = "[target_profile.tiling]\nstate = \"established\"\nconfidence = \
                         \"measured\"\nscope = \"specimen-observation\"\nforce = \
                         \"observed-source-format\"\ncorpus = \
                         \"fixture-corpus\"\nprovenance = [\"x\"]\n\
                         [target_profile.tiling.value]\nkind = \"linear\"\n";
    let specimen = raster_rules(specimen_body);
    assert!(
        !specimen.contains(&Rule::AssetEvidenceScopeInsufficient),
        "a scoped specimen measurement is legitimate evidence: {specimen:?}"
    );

    // Decided, and operative for NOTHING. The two are separate questions and `/3` answers them
    // separately: the block is established, and no Build reads it.
    let profile = parse_asset_profile(&doc(RASTER_SOURCE, specimen_body)).expect("parses");
    let target = &profile.targets[0];
    assert_eq!(target.tiling.state, ConstraintState::Established);
    assert!(target.tiling.established().is_some());
    assert!(
        target.tiling.operative().is_none(),
        "a specimen observation speaks for its specimens, not for the target"
    );
    let tiling = block_states(target, None)
        .into_iter()
        .find(|b| b.block == "tiling")
        .expect("declared");
    assert!(!tiling.operative);

    // What it may NOT describe is placement. A specimen's own bytes cannot say where it sat, so
    // the same scope on a placement block is refused rather than merely non-operative.
    let placement = raster_rules(
        "[target_profile.discovery]\nstate = \"established\"\nconfidence = \"measured\"\n\
         scope = \"specimen-observation\"\nforce = \"observed-source-format\"\ncorpus = \
         \"fixture-corpus\"\nprovenance = [\"x\"]\n[target_profile.discovery.value]\nscope = \
         \"loose-tree\"\nroot = \"fe/ion\"\nrecursive = true\n",
    );
    assert!(placement.contains(&Rule::AssetEvidenceScopeInsufficient));

    // Reading the bytes one copy carries establishes what the source IS, never what the target
    // REJECTS - so an inspected-corpus invariant cannot carry `target-required`.
    let overclaimed = raster_rules(
        "[target_profile.tiling]\nstate = \"established\"\nconfidence = \"measured\"\nscope = \
         \"inspected-corpus-invariant\"\nforce = \"target-required\"\ncorpus = \
         \"fixture-corpus\"\nprovenance = [\"x\"]\n[target_profile.tiling.value]\nkind = \
         \"linear\"\n",
    );
    assert!(overclaimed.contains(&Rule::AssetEvidenceScopeInsufficient));

    // The honest form produces neither.
    let ok = raster_rules(&format!(
        "[target_profile.tiling]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.tiling.value]\nkind = \"linear\"\n"
    ));
    assert!(ok.is_empty(), "{ok:?}");

    // And an undecided block may not carry a force: force says what an ESTABLISHED constraint
    // asserts about other values, and an undecided one asserts nothing.
    let forced_unknown =
        raster_rules("[target_profile.tiling]\nstate = \"unknown\"\nforce = \"target-required\"\n");
    assert!(forced_unknown.contains(&Rule::ValueWithoutEstablishedState));
}

#[test]
fn corpus_evidence_cannot_authorize_a_universal_constraint_it_does_not_cover() {
    // The C0-POR1-R1 verdict, enforced. However exhaustively a derived or extracted tree was
    // measured, a fact about it is not a fact about the shipped target - and the profile refuses
    // the claim rather than leaving it to a reviewer to catch a second time.
    let derived = "\n[[target_profile.corpus]]\nid = \"extraction\"\nkind = \"extracted-tree\"\n\
                   attribution = \"not-attributable\"\nbuild_identification = \"incomplete\"\n\
                   identity_source = \"synthetic:manifest\"\ninspected_files = 10000\n\
                   limitations = [\"An extraction.\"]\nprovenance = [\"x\"]\n";
    let resting_on_it = format!(
        "{derived}[target_profile.tiling]\nstate = \"established\"\nconfidence = \"measured\"\n\
         scope = \"inspected-corpus-invariant\"\nforce = \"observed-source-format\"\ncorpus = \
         \"extraction\"\nprovenance = [\"x\"]\n[target_profile.tiling.value]\nkind = \"linear\"\n"
    );
    assert!(raster_rules(&resting_on_it).contains(&Rule::AssetCorpusAttributionUnsupported));

    // Nor can the corpus simply relabel itself: an extraction is the output of an extraction.
    let relabelled = derived.replace("\"not-attributable\"", "\"attributable-to-target\"");
    assert!(raster_rules(&relabelled).contains(&Rule::AssetCorpusAttributionUnsupported));

    // A block that names no corpus at all cannot be established either. Two corpora can hold
    // byte-identical files and support entirely different claims, so the binding is not optional.
    let uncited = raster_rules(
        "[target_profile.tiling]\nstate = \"established\"\nconfidence = \"measured\"\nscope = \
         \"inspected-corpus-invariant\"\nforce = \"observed-source-format\"\nprovenance = \
         [\"x\"]\n[target_profile.tiling.value]\nkind = \"linear\"\n",
    );
    assert!(uncited.contains(&Rule::AssetCorpusUndeclared));

    // And an unidentified build must say what that costs rather than leaving it to the reader.
    let silent = "\n[[target_profile.corpus]]\nid = \"copy\"\nkind = \
                  \"observed-installed-copy\"\nattribution = \"attributable-to-target\"\n\
                  build_identification = \"incomplete\"\nidentity_source = \"synthetic:sfo\"\n\
                  inspected_files = 10\nprovenance = [\"x\"]\n";
    assert!(raster_rules(silent).contains(&Rule::AssetCorpusIncomplete));
    let empty = "\n[[target_profile.corpus]]\nid = \"copy\"\nkind = \"observed-installed-copy\"\n\
                 attribution = \"attributable-to-target\"\nbuild_identification = \"complete\"\n\
                 identity_source = \"synthetic:sfo\"\ninspected_files = 0\nprovenance = [\"x\"]\n";
    assert!(raster_rules(empty).contains(&Rule::AssetCorpusIncomplete));
}

// ================================================================ partial values

#[test]
fn a_raster_block_may_be_established_with_its_unmeasurable_fields_absent() {
    // `/2` bundled four fields into one value, so an unmeasurable colour space blocked the codec
    // too - and C0-POR1 filled it with a guess to make the bundle well-formed. `/3` removes the
    // pressure: the bundle may be incomplete, and the gap keeps its own reason.
    let partial = format!(
        "[target_profile.raster]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.raster.value]\ncodec = \"dxt5\"\nchannels \
         = [\"r\", \"g\", \"b\", \"a\"]\n[target_profile.raster.property.color_space]\nstate = \
         \"unknown\"\nreason = \"The DDS header records none.\"\n"
    );
    assert!(
        raster_rules(&partial).is_empty(),
        "{:?}",
        raster_rules(&partial)
    );

    // An absent alpha MODE is not `alpha = "none"`: the stored alpha channel is still there, and
    // under `/2` that pairing would have been a contradiction.
    let profile = parse_asset_profile(&doc(RASTER_SOURCE, &partial)).expect("parses");
    let raster = profile.targets[0]
        .raster
        .established()
        .expect("established");
    assert_eq!(raster.codec, "dxt5");
    assert!(raster.color_space.is_none());
    assert!(raster.alpha.is_none());
    assert!(raster.channels.contains(&"a".to_string()));

    // Evidence for a property that is actually KNOWN is a contradiction - it would be a reason for
    // an absence that is not absent.
    let both = format!(
        "[target_profile.raster]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.raster.value]\ncodec = \"dxt5\"\nchannels \
         = [\"r\"]\n[target_profile.raster.property.color_space]\nstate = \"established\"\nreason \
         = \"x\"\n"
    );
    assert!(raster_rules(&both).contains(&Rule::AssetPropertyEvidenceConflict));

    // A property the block cannot leave unknown has no evidence table.
    let unknown_property = format!(
        "[target_profile.raster]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.raster.value]\ncodec = \"dxt5\"\nchannels \
         = [\"r\"]\n[target_profile.raster.property.codec]\nstate = \"unknown\"\nreason = \"x\"\n"
    );
    assert!(raster_rules(&unknown_property).contains(&Rule::AssetPropertyEvidenceConflict));

    // Silence is not a reason.
    let unreasoned = format!(
        "[target_profile.raster]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.raster.value]\ncodec = \"dxt5\"\nchannels \
         = [\"r\"]\n[target_profile.raster.property.alpha]\nstate = \"unknown\"\nreason = \"  \"\n"
    );
    assert!(raster_rules(&unreasoned).contains(&Rule::AssetPropertyEvidenceConflict));
}

#[test]
fn an_atlas_may_record_what_a_payload_count_measures_without_naming_a_kind() {
    // C0-POR1 read `kind = "none"` off a count of one texture payload per archive. The count shows
    // no surface is SHARED; it does not distinguish an atlas from a pool from neither.
    let sharing_only = format!(
        "[target_profile.atlas]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.atlas.value]\nsharing = \
         \"per-asset-surface\"\n[target_profile.atlas.property.kind]\nstate = \"unknown\"\nreason \
         = \"A payload count does not answer it.\"\n"
    );
    assert!(
        raster_rules(&sharing_only).is_empty(),
        "{:?}",
        raster_rules(&sharing_only)
    );

    // With the kind unknown, the operands only a known kind gives meaning to must be absent.
    let half_filled = sharing_only.replace(
        "sharing = \"per-asset-surface\"\n",
        "sharing = \"per-asset-surface\"\npage_width = 512\n",
    );
    assert!(raster_rules(&half_filled).contains(&Rule::AssetOperandsIncomplete));

    // And a kind that contradicts the sharing fact is refused rather than preferred.
    let contradictory = format!(
        "[target_profile.atlas]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.atlas.value]\nsharing = \
         \"per-asset-surface\"\nkind = \"pool\"\npage_width = 512\npage_height = 512\nslot = 1\n"
    );
    assert!(raster_rules(&contradictory).contains(&Rule::AssetBoundsInconsistent));
}

// ==================================================================== variants

/// A raster target body carrying `count` variants built from `bodies`.
fn variant_doc(bodies: &str) -> Vec<Rule> {
    raster_rules(&format!(
        "[[target_profile.path_family]]\nid = \"heads\"\ntemplate = \"p{{numeric-id}}.big\"\n\
         tokens = [\"numeric-id\"]\ncase = \"lower\"\nbucket_resolution = \"template-only\"\n\
         {bodies}"
    ))
}

#[test]
fn a_variant_must_be_selectable_and_its_selector_must_be_coherent() {
    // A variant nothing can choose is an unreachable value, not a variant.
    assert!(variant_doc("[[target_profile.variant]]\nid = \"only\"\n")
        .contains(&Rule::AssetVariantSelectorIncomplete));

    // Each clause takes exactly the operands its dimension defines.
    let wrong_operand = "[[target_profile.variant]]\nid = \"only\"\n\
                         [[target_profile.variant.selector]]\ndimension = \"path-family\"\n\
                         id_low = 1\nid_high = 2\n";
    assert!(variant_doc(wrong_operand).contains(&Rule::AssetVariantSelectorIncomplete));

    let inverted = "[[target_profile.variant]]\nid = \"only\"\n\
                    [[target_profile.variant.selector]]\ndimension = \"numeric-id-range\"\n\
                    id_low = 90\nid_high = 10\n";
    assert!(variant_doc(inverted).contains(&Rule::AssetBoundsInconsistent));

    // Two clauses over one dimension are a conjunction nothing can satisfy unambiguously.
    let twice = "[[target_profile.variant]]\nid = \"only\"\n\
                 [[target_profile.variant.selector]]\ndimension = \"path-family\"\nequals = \
                 \"heads\"\n[[target_profile.variant.selector]]\ndimension = \"path-family\"\n\
                 equals = \"heads\"\n";
    assert!(variant_doc(twice).contains(&Rule::AssetVariantSelectorContradiction));

    // A selector naming a path family the target does not declare is refused rather than ignored.
    let unknown_family = "[[target_profile.variant]]\nid = \"only\"\n\
                          [[target_profile.variant.selector]]\ndimension = \"path-family\"\n\
                          equals = \"elsewhere\"\n";
    assert!(variant_doc(unknown_family).contains(&Rule::AssetPathFamilyUnresolved));

    // Two variants nothing can tell apart are a defect in the profile, not a runtime condition.
    let identical = "[[target_profile.variant]]\nid = \"a\"\n\
                     [[target_profile.variant.selector]]\ndimension = \"path-family\"\nequals = \
                     \"heads\"\n[[target_profile.variant]]\nid = \"b\"\n\
                     [[target_profile.variant.selector]]\ndimension = \"path-family\"\nequals = \
                     \"heads\"\n";
    assert!(variant_doc(identical).contains(&Rule::AssetVariantSelectorContradiction));
}

#[test]
fn an_impossible_cross_variant_combination_is_not_spellable() {
    // A selector that selects on a size the variant does not store would hand a caller one form's
    // geometry under another form's key.
    let contradictory = format!(
        "[[target_profile.variant]]\nid = \"only\"\n[[target_profile.variant.selector]]\n\
         dimension = \"stored-dimensions\"\nwidth = 512\nheight = 512\n\
         [target_profile.variant.dimensions]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.variant.dimensions.value]\nwidth = 256\n\
         height = 128\nvariant = \"small\"\npower_of_two = true\n"
    );
    assert!(variant_doc(&contradictory).contains(&Rule::AssetVariantSelectorContradiction));

    // And a codec that cannot store the size beside it is caught INSIDE the variant, because the
    // two are correlated there rather than sitting in independent per-target lists.
    let unstorable = format!(
        "[[target_profile.variant]]\nid = \"only\"\n[[target_profile.variant.selector]]\n\
         dimension = \"path-family\"\nequals = \"heads\"\n\
         [target_profile.variant.dimensions]\nstate = \"established\"\nconfidence = \"measured\"\n\
         {SCOPED}provenance = [\"x\"]\n[target_profile.variant.dimensions.value]\nwidth = 130\n\
         height = 128\nvariant = \"odd\"\npower_of_two = false\n[target_profile.variant.raster]\n\
         state = \"established\"\nconfidence = \"measured\"\n{SCOPED}provenance = [\"x\"]\n\
         [target_profile.variant.raster.value]\ncodec = \"dxt5\"\nchannels = [\"r\", \"g\", \
         \"b\", \"a\"]\n"
    );
    assert!(variant_doc(&unstorable).contains(&Rule::AssetBoundsInconsistent));
}

#[test]
fn a_target_with_variants_may_not_also_decide_the_geometry_at_target_level() {
    let both = format!(
        "[[target_profile.variant]]\nid = \"only\"\n[[target_profile.variant.selector]]\n\
         dimension = \"path-family\"\nequals = \"heads\"\n[target_profile.tiling]\nstate = \
         \"established\"\nconfidence = \"measured\"\n{SCOPED}provenance = [\"x\"]\n\
         [target_profile.tiling.value]\nkind = \"linear\"\n"
    );
    assert!(variant_doc(&both).contains(&Rule::AssetVariantBlockConflict));

    // The blocks that are NOT correlated stay at target level and are unaffected.
    let container = format!(
        "[[target_profile.variant]]\nid = \"only\"\n[[target_profile.variant.selector]]\n\
         dimension = \"path-family\"\nequals = \"heads\"\n[target_profile.compression]\nstate = \
         \"established\"\nconfidence = \"measured\"\n{SCOPED}provenance = [\"x\"]\n\
         [target_profile.compression.value]\npayload = \"refpack\"\ndeterministic = false\n"
    );
    assert!(!variant_doc(&container).contains(&Rule::AssetVariantBlockConflict));
}

#[test]
fn variant_selection_is_deterministic_and_never_falls_back() {
    let profile = synthetic_profile();
    let three = target(&profile, "fixturethree-synthetic");

    // One family, one form: decided.
    assert_eq!(
        select_variant(
            three,
            &VariantQuery {
                path_family: Some("hero".to_string()),
                ..Default::default()
            }
        ),
        VariantSelection::Selected("hero-256x256".to_string())
    );

    // One family, two forms, no size: REFUSED, naming both. Not the first, not the closest, and
    // not the better-populated one.
    assert_eq!(
        select_variant(
            three,
            &VariantQuery {
                path_family: Some("banner".to_string()),
                ..Default::default()
            }
        ),
        VariantSelection::Ambiguous(vec![
            "banner-512x128".to_string(),
            "banner-512x256".to_string()
        ])
    );

    // With the size, decided.
    assert_eq!(
        select_variant(
            three,
            &VariantQuery {
                path_family: Some("banner".to_string()),
                stored_dimensions: Some((512, 256)),
                ..Default::default()
            }
        ),
        VariantSelection::Selected("banner-512x256".to_string())
    );

    // A size that matches no `banner` form is NoMatch - an explicit unsupported result. The `hero`
    // variant is ruled out by its family clause, so nothing is left to be ambiguous about.
    assert_eq!(
        select_variant(
            three,
            &VariantQuery {
                path_family: Some("banner".to_string()),
                stored_dimensions: Some((64, 64)),
                ..Default::default()
            }
        ),
        VariantSelection::NoMatch
    );

    // Declaration order is not part of the decision: reversing the variants changes nothing.
    let mut reversed = three.clone();
    reversed.variants.reverse();
    assert_eq!(
        select_variant(
            &reversed,
            &VariantQuery {
                path_family: Some("banner".to_string()),
                stored_dimensions: Some((512, 128)),
                ..Default::default()
            }
        ),
        VariantSelection::Selected("banner-512x128".to_string())
    );

    // A target with no variants says so, rather than reporting an absence as a no-match.
    assert_eq!(
        select_variant(
            target(&profile, "fixtureone-synthetic"),
            &VariantQuery::default()
        ),
        VariantSelection::NoVariants
    );
}

// ==================================================== observed paths, unknown lookup

#[test]
fn an_unknown_runtime_lookup_blocks_discovery_and_install_on_every_committed_target() {
    // The `/3` block for what the GAME loads. C0-POR1 established `[discovery]` from observed file
    // placement in one installed copy; R1 withdrew that, and the reason was `patch.big` - present
    // at USRDIR root, never opened, and able to shadow every loose portrait.
    let profile = committed_profile();
    for declared in &profile.targets {
        // C0-AS5: the Recompiled target inherits its operations by reference; readiness is
        // evaluated on the effective profile, whose `runtime_lookup` is Recompiled's own (unknown).
        let resolved = resolve_target(&profile, &declared.target).unwrap();
        let t = &resolved.profile;
        assert!(
            t.runtime_lookup.established().is_none(),
            "{}: no trace, disassembly or load test exists for any target",
            t.target
        );
        for operation in ["discover", "install"] {
            let declaration = t
                .operations
                .iter()
                .find(|o| o.operation == operation)
                .unwrap_or_else(|| panic!("{}/{operation} is declared", t.target));
            assert!(
                declaration.requires.contains(&"runtime_lookup".to_string()),
                "{}/{operation} must depend on what the runtime actually reads",
                t.target
            );
            match operation_constraint_readiness(t, None, declaration) {
                OperationReadiness::BlockedByUnknownConstraints(missing) => assert!(
                    missing.contains(&"runtime_lookup".to_string()),
                    "{}/{operation}: {missing:?}",
                    t.target
                ),
                OperationReadiness::ConstraintsEstablished => {
                    panic!("{}/{operation} cannot be ready", t.target)
                }
            }
        }
    }
}

// =============================================================== v2 -> v3 transition

#[test]
fn the_v2_asset_transition_demotes_every_constraint_and_promotes_none() {
    // `/3` requires an established block to name its evidence scope, its force over other values
    // and the corpus it rests on. A `/2` document records none of the three, and none can be
    // inferred: a `/2` "measured" says a measurement happened, not what population it covered or
    // whether that population is attributable to the target - which is exactly what decided which
    // C0-POR1 promotions survived review.
    let v2_text = fixture("asset-profiles/v2/assets-portraits.toml");

    // It does not even parse as `/3`: `/2` carries keys `/3` does not define, and unknown keys are
    // refused rather than dropped. The mistagged-schema refusal is covered separately.
    assert!(parse_asset_profile(&v2_text).is_err());

    let v2 = parse_asset_profile_v2(&v2_text).expect("v2 parses");
    let upgraded = upgrade_asset_profile_v2(&v2);

    assert_eq!(upgraded.profile.schema, ASSET_PROFILE_SCHEMA);
    assert_eq!(upgraded.profile.family, v2.family);
    assert_eq!(upgraded.profile.targets.len(), v2.targets.len());

    // Nothing established, nothing not-applicable, and no variant, corpus or path family invented.
    let population = asset_registry_population(std::iter::once(&upgraded.profile));
    assert!(!population.is_empty());
    for row in &population {
        assert_eq!(row.established_constraints, 0, "{}", row.target);
        assert_eq!(row.not_applicable_constraints, 0, "{}", row.target);
        assert_eq!(row.unknown_constraints, ASSET_BLOCK.len(), "{}", row.target);
        assert_eq!(row.declared_variants, 0, "{}", row.target);
        assert_eq!(row.declared_corpora, 0, "{}", row.target);
        assert_eq!(row.declared_path_families, 0, "{}", row.target);
    }

    // The demoted values are preserved rather than discarded: a conservative transition loses no
    // measurement, it only stops the measurement driving anything.
    let ps3 = upgraded
        .profile
        .targets
        .iter()
        .find(|t| t.target == "nhllegacy-ps3")
        .expect("carried across");
    let compression = ps3
        .compression
        .candidate_value()
        .expect("the /2 value is kept as a proposal");
    assert_eq!(compression.payload, "refpack");
    assert!(ps3.compression.scope.is_none(), "no scope is invented");
    assert!(ps3.compression.corpus.is_none(), "no corpus is invented");

    // Every demotion is reported, and so is each thing the transition cannot produce.
    let fields: BTreeSet<&str> = upgraded.notes.iter().map(|n| n.field.as_str()).collect();
    assert!(fields.contains("[compression].state"));
    assert!(fields.contains("[[corpus]]"));
    assert!(fields.contains("[[path_family]]"));
    assert!(fields.contains("[[variant]]"));

    // And the translated document is a faithful translation, not a passing one: it validates
    // cleanly only because every block is honestly unknown.
    assert_eq!(validate_asset_profile(&upgraded.profile), vec![]);
}

#[test]
fn a_v2_decided_absence_is_demoted_rather_than_carried_across() {
    // `not-applicable` is OPERATIVE: it SATISFIES an operation's requirement. Carrying one across
    // unexamined would let a `/2`-era judgement keep driving `/3` readiness without ever citing the
    // corpus behind it, so the transition demotes it and preserves the reason as a note.
    let v2 = parse_asset_profile_v2(
        "schema = \"asset-output-profile/2\"
profile_version = 1
family =          \"assets-portraits\"
[[canonical_source]]
role = \"portrait\"
class =          \"raster\"
interchange = \"rgba8\"
retention = \"unmodified\"
         [[target_profile]]
target = \"nhl15-xbox360\"
[target_profile.mipmaps]
state =          \"not-applicable\"
reason = \"The loader has no level field.\"
",
    )
    .expect("v2 parses");
    let upgraded = upgrade_asset_profile_v2(&v2);
    let target = &upgraded.profile.targets[0];
    assert_eq!(target.mipmaps.state, ConstraintState::Unknown);
    assert!(
        target.mipmaps.note.contains("no level field"),
        "the reason survives as prose: {:?}",
        target.mipmaps.note
    );
    assert!(
        target.mipmaps.reason.is_empty(),
        "it is no longer a decided absence"
    );
    assert!(upgraded
        .notes
        .iter()
        .any(|n| n.value == "not-applicable" && n.disposition.contains("OPERATIVE")));

    // C0-AS3-R1: demoting is not erasing, and the DOCUMENT says so rather than only the transition
    // report. What `/2` asserted, why the demotion happened, and the original reason verbatim all
    // survive in the upgraded block - so re-deciding it is a matter of citing a corpus, not of
    // rediscovering why somebody wrote it down.
    assert!(
        target.mipmaps.note.contains("NOT-APPLICABLE"),
        "the original assertion survives, not just its reason: {:?}",
        target.mipmaps.note
    );
    assert!(
        target
            .mipmaps
            .note
            .contains("not because the assertion was found to be wrong"),
        "the demotion is about missing scope, and saying so is the difference between demoting \
         and overruling: {:?}",
        target.mipmaps.note
    );

    // An absence with no recorded reason is demoted too, and the empty reason is itself preserved
    // as part of why it could not be carried across.
    let v2 = parse_asset_profile_v2(
        "schema = \"asset-output-profile/2\"
profile_version = 1
family = \"assets-portraits\"
[[canonical_source]]
role = \"portrait\"
class = \"raster\"
interchange = \"rgba8\"
retention = \"unmodified\"
[[target_profile]]
target = \"nhl15-xbox360\"
[target_profile.atlas]
state = \"not-applicable\"
",
    )
    .expect("v2 parses");
    let upgraded = upgrade_asset_profile_v2(&v2);
    let target = &upgraded.profile.targets[0];
    assert_eq!(target.atlas.state, ConstraintState::Unknown);
    assert!(target.atlas.note.contains("NOT-APPLICABLE"));
    assert!(target.atlas.note.contains("recorded no reason"));

    // And a `/2` candidate stays a candidate rather than being promoted by the move.
    let v2 = parse_asset_profile_v2(
        "schema = \"asset-output-profile/2\"
profile_version = 1
family =          \"assets-portraits\"
[[canonical_source]]
role = \"portrait\"
class =          \"raster\"
interchange = \"rgba8\"
retention = \"unmodified\"
         [[target_profile]]
target = \"nhl15-xbox360\"
[target_profile.tiling]
state =          \"unknown\"
[target_profile.tiling.candidate]
confidence = \"measured\"
         provenance = [\"x\"]
[target_profile.tiling.candidate.value]
kind = \"linear\"
",
    )
    .expect("v2 parses");
    let upgraded = upgrade_asset_profile_v2(&v2);
    let target = &upgraded.profile.targets[0];
    assert_eq!(target.tiling.state, ConstraintState::Unknown);
    assert_eq!(
        target.tiling.candidate_value().map(|v| v.kind.as_str()),
        Some("linear")
    );
    assert!(target
        .tiling
        .candidate
        .as_ref()
        .expect("kept")
        .scope
        .is_none());
}

#[test]
fn the_database_half_of_the_registry_is_untouched_by_the_asset_schema_revision() {
    // `/3`'s scope, force and corpus keys live on `AssetConstraint`, NOT on the shared
    // `Constraint<T>` the database adapters use. A `target-adapter/2` document must parse exactly
    // as it did, and must still refuse the asset keys as unknown - otherwise this revision would
    // have quietly widened a schema it was not asked to change.
    let schema_text =
        std::fs::read_to_string(repo_root().join("defs/canonical/project-schema.toml"))
            .expect("the project schema is committed");
    let schema = parse_project_schema(&schema_text).expect("it parses");
    let adapter_text =
        std::fs::read_to_string(repo_root().join("defs/adapters/nhl15-xbox360.toml"))
            .expect("the adapter is committed");
    let adapter = parse_adapter(&adapter_text).expect("it parses unchanged");
    assert_eq!(adapter.schema, ADAPTER_SCHEMA);
    assert_eq!(validate_adapter(&adapter, &schema), vec![]);

    // The anchor is single-line, so it survives either checkout's line endings; the guard is here
    // because `replacen` is silent when it matches nothing, and a mutation that mutated nothing
    // would leave this asserting that the *unedited* committed adapter is refused — which it is
    // not. The committed file is read raw and is never newline-normalized: this case is about what
    // the parser accepts, not about anchor arithmetic.
    assert!(
        adapter_text.contains("state = \"established\""),
        "the committed adapter no longer carries the anchor this case mutates"
    );
    let with_asset_key = adapter_text.replacen(
        "state = \"established\"",
        "state = \"established\"\nscope = \"inspected-corpus-invariant\"",
        1,
    );
    assert_ne!(
        with_asset_key, adapter_text,
        "the asset evidence key was never inserted, so the refusal below proves nothing"
    );
    assert!(
        parse_adapter(&with_asset_key).is_err(),
        "an asset evidence key must remain unspellable in a database adapter"
    );
}

// ================================ C0-AS3-R1: observation, attribution, and per-variant containers

#[test]
fn an_unattributable_corpus_supports_a_scoped_specimen_measurement_and_nothing_wider() {
    // AS3 refused every established block resting on an extracted or derived tree. That is one
    // over-correction past the finding: an extraction is a real body of files, and measuring them
    // establishes something true ABOUT THEM. What it never establishes is a fact about the target.
    let extraction = "\n[[target_profile.corpus]]\nid = \"extraction\"\nkind = \
                      \"extracted-tree\"\nattribution = \"not-attributable\"\n\
                      build_identification = \"complete\"\nidentity_source = \
                      \"synthetic:manifest\"\ninspected_files = 12\nprovenance = [\"x\"]\n";

    let scoped = format!(
        "{extraction}[target_profile.tiling]\nstate = \"established\"\nconfidence = \
         \"measured\"\nscope = \"specimen-observation\"\nforce = \
         \"observed-source-format\"\ncorpus = \"extraction\"\nprovenance = \
         [\"x\"]\n[target_profile.tiling.value]\nkind = \"linear\"\n"
    );
    let rules = raster_rules(&scoped);
    assert!(
        !rules.contains(&Rule::AssetCorpusAttributionUnsupported),
        "a scoped measurement of an extraction's own files is legitimate: {rules:?}"
    );

    // Widen the same measurement to the corpus and it is refused again. The scope is what carries
    // the claim, and only one of the two scopes is a claim about the target.
    let widened = scoped.replace("\"specimen-observation\"", "\"inspected-corpus-invariant\"");
    assert!(raster_rules(&widened).contains(&Rule::AssetCorpusAttributionUnsupported));

    // And a PLACEMENT fact is refused at either scope. Extracting is the step that discarded where
    // the files sat, so no reading of the output recovers it.
    let placement = format!(
        "{extraction}[target_profile.discovery]\nstate = \"established\"\nconfidence = \
         \"measured\"\nscope = \"specimen-observation\"\nforce = \
         \"observed-source-format\"\ncorpus = \"extraction\"\nprovenance = \
         [\"x\"]\n[target_profile.discovery.value]\nscope = \"archive-member\"\nroot = \
         \"fe/ion\"\nrecursive = false\n"
    );
    assert!(raster_rules(&placement).contains(&Rule::AssetEvidenceScopeInsufficient));
}

#[test]
fn a_provenance_bridge_is_checked_as_evidence_and_not_accepted_as_a_label() {
    // The one route from an extraction to a placement fact, and every way it fails. `fixture-corpus`
    // is the attributable corpus every case here bridges to.
    let bridged = |bridge: &str| {
        format!(
            "\n[[target_profile.corpus]]\nid = \"extraction\"\nkind = \
             \"extracted-tree\"\nattribution = \"not-attributable\"\nbuild_identification = \
             \"complete\"\nidentity_source = \"synthetic:manifest\"\ninspected_files = \
             12\nprovenance = [\"x\"]\n[target_profile.corpus.bridge]\n{bridge}\
             [target_profile.discovery]\nstate = \"established\"\nconfidence = \
             \"measured\"\nscope = \"inspected-corpus-invariant\"\nforce = \
             \"observed-source-format\"\ncorpus = \"extraction\"\nprovenance = \
             [\"x\"]\n[target_profile.discovery.value]\nscope = \"archive-member\"\nroot = \
             \"fe/ion\"\nrecursive = false\n"
        )
    };

    // The valid bridge: a known method, an attributable corpus on the other end, and a citation.
    let ok = raster_rules(&bridged(
        "method = \"archive-member-manifest\"\noriginal_corpus = \"fixture-corpus\"\nprovenance = \
         [\"synthetic:manifest\"]\n",
    ));
    assert!(ok.is_empty(), "{ok:?}");

    // An uncited bridge is the assertion that a record existed, which is not the record.
    assert!(raster_rules(&bridged(
        "method = \"archive-member-manifest\"\noriginal_corpus = \"fixture-corpus\"\nprovenance = \
         []\n",
    ))
    .contains(&Rule::AssetProvenanceBridgeInvalid));

    // A bridge to itself carries nothing across.
    assert!(raster_rules(&bridged(
        "method = \"archive-member-manifest\"\noriginal_corpus = \"extraction\"\nprovenance = \
         [\"x\"]\n",
    ))
    .contains(&Rule::AssetProvenanceBridgeInvalid));

    // A bridge to a corpus this target does not declare moves the question rather than answering
    // it, and takes the placement fact down with it.
    let dangling = raster_rules(&bridged(
        "method = \"archive-member-manifest\"\noriginal_corpus = \"somewhere-else\"\nprovenance = \
         [\"x\"]\n",
    ));
    assert!(dangling.contains(&Rule::AssetProvenanceBridgeInvalid));
    assert!(dangling.contains(&Rule::AssetCorpusAttributionUnsupported));

    // A folder layout is not a bridge method, and there is no term for one.
    assert!(raster_rules(&bridged(
        "method = \"the extractor's output layout\"\noriginal_corpus = \
         \"fixture-corpus\"\nprovenance = [\"x\"]\n",
    ))
    .contains(&Rule::UnknownVocabularyTerm));

    // C0-AS3-R2: all of the above is STRUCTURAL. Nothing opened the cited record or tested the tie
    // it asserts, and a valid bridge says nothing whatever about what the runtime loads - which is
    // a separate block, and stays unknown here. A file's placement in an archive and the runtime's
    // lookup precedence are different questions.
    let profile = parse_asset_profile(&doc(
        RASTER_SOURCE,
        &bridged(
            "method = \"archive-member-manifest\"
original_corpus =              \"fixture-corpus\"
provenance = [\"synthetic:manifest\"]
",
        ),
    ))
    .expect("parses");
    let target = &profile.targets[0];
    assert!(
        target.discovery.operative().is_some(),
        "placement is reached"
    );
    assert_eq!(
        target.runtime_lookup.state,
        ConstraintState::Unknown,
        "a bridge carries placement across, and answers nothing about runtime lookup"
    );
}

#[test]
fn two_variants_may_carry_different_containers_and_never_two_answers_to_one_question() {
    // C0-AS3-R1: the container is shared on the portrait targets and is not shared in general. Both
    // must be sayable, and the level a fact is stated at must be unambiguous.
    let profile = synthetic_profile();

    // Shared: `fixturethree-synthetic` states one chain at target level, and no variant restates it.
    // Every variant therefore resolves to the same chain.
    let three = target(&profile, "fixturethree-synthetic");
    assert!(three.container.established().is_some());
    for variant in &three.variants {
        assert_eq!(variant.container.state, ConstraintState::Unknown);
        let resolved = block_states(three, Some(variant))
            .into_iter()
            .find(|b| b.block == "container")
            .expect("declared");
        assert_eq!(resolved.state, ConstraintState::Established);
    }

    // Per variant: `fixturefour-synthetic` leaves the target-level block unknown, and its two forms
    // carry genuinely different chains - a bare texture and an archive member.
    let four = target(&profile, "fixturefour-synthetic");
    assert_eq!(four.container.state, ConstraintState::Unknown);
    let chains: Vec<Vec<String>> = four
        .variants
        .iter()
        .map(|v| {
            v.container
                .established()
                .expect("each form states its own chain")
                .layers
                .iter()
                .map(|l| l.format.clone())
                .collect()
        })
        .collect();
    assert_eq!(chains, vec![vec!["dds"], vec!["bigf", "dds"]]);

    // And the resolution is the variant's, deterministically.
    for (variant, expected) in four.variants.iter().zip(&chains) {
        let resolved = block_states(four, Some(variant))
            .into_iter()
            .find(|b| b.block == "container")
            .expect("declared");
        assert_eq!(resolved.state, ConstraintState::Established);
        assert_eq!(
            variant
                .container
                .established()
                .expect("stated")
                .layers
                .len(),
            expected.len()
        );
    }

    // Deciding it at BOTH levels is refused. Not a precedence puzzle: two answers to one question,
    // and which one a reader gets would depend on where they looked.
    let both = "[[target_profile.variant]]\nid = \"only\"\n\
                [[target_profile.variant.selector]]\ndimension = \"path-family\"\nequals = \
                \"portraits\"\n[target_profile.variant.container]\nstate = \
                \"established\"\nconfidence = \"measured\"\nscope = \
                \"inspected-corpus-invariant\"\nforce = \"observed-source-format\"\ncorpus = \
                \"fixture-corpus\"\nprovenance = \
                [\"x\"]\n[target_profile.variant.container.value]\noutermost_layer_examined = \
                true\n[[target_profile.variant.container.value.layer]]\nformat = \"dds\"\nindex = \
                \"none\"\n[[target_profile.path_family]]\nid = \"portraits\"\ntemplate = \
                \"p{numeric-id}.big\"\ntokens = [\"numeric-id\"]\ncase = \
                \"lower\"\nbucket_resolution = \"template-only\"\n";
    assert!(raster_rules(both).is_empty(), "{:?}", raster_rules(both));

    let conflicting = format!(
        "{both}[target_profile.container]\nstate = \"established\"\nconfidence = \
         \"measured\"\n{SCOPED}provenance = [\"x\"]\n[target_profile.container.value]\n\
         outermost_layer_examined = true\n[[target_profile.container.value.layer]]\nformat = \
         \"bigf\"\nindex = \"name-table\"\n"
    );
    assert!(raster_rules(&conflicting).contains(&Rule::AssetVariantBlockConflict));
}

#[test]
fn an_established_specimen_observation_satisfies_no_operation_and_decides_no_build() {
    // Established and operative are separate questions, and the readiness report answers the second.
    let profile = synthetic_profile();
    let four = target(&profile, "fixturefour-synthetic");
    let loose = four
        .variants
        .iter()
        .find(|v| v.id == "loose-texture")
        .expect("declared");

    assert_eq!(loose.tiling.state, ConstraintState::Established);
    assert!(loose.tiling.established().is_some());
    assert!(loose.tiling.operative().is_none());
    assert!(!loose.tiling.is_operative());

    // `export` requires [tiling]; the only thing decided about it here speaks for four files.
    let export = four
        .operations
        .iter()
        .find(|o| o.operation == "export")
        .expect("declared");
    match operation_constraint_readiness(four, Some(loose), export) {
        OperationReadiness::BlockedByUnknownConstraints(missing) => {
            assert_eq!(missing, vec!["tiling".to_string()])
        }
        other => panic!("a specimen observation defines no conversion: {other:?}"),
    }

    // `preview` names what it actually needs, and every one of those IS operative.
    let preview = four
        .operations
        .iter()
        .find(|o| o.operation == "preview")
        .expect("declared");
    assert_eq!(
        operation_constraint_readiness(four, Some(loose), preview),
        OperationReadiness::ConstraintsEstablished
    );
}

#[test]
fn the_population_report_separates_observed_facts_from_target_requirements() {
    // "7 established constraints" is a true sentence that invites a false reading. The report says
    // how many of them speak for the target, and how many are target REQUIREMENTS - which is the
    // number a reader wanting to know what this repository has proven should be looking at.
    let profile = committed_profile();
    let population = asset_registry_population(std::iter::once(&profile));

    let established: usize = population.iter().map(|p| p.established_constraints).sum();
    let in_variants: usize = population
        .iter()
        .map(|p| p.established_variant_constraints)
        .sum();
    let operative: usize = population.iter().map(|p| p.operative_constraints).sum();
    let operative_in_variants: usize = population
        .iter()
        .map(|p| p.operative_variant_constraints)
        .sum();
    let specimens: usize = population.iter().map(|p| p.specimen_observations).sum();
    let required: usize = population
        .iter()
        .map(|p| p.target_required_constraints)
        .sum();

    assert_eq!(established, 3, "target-level established blocks");
    assert_eq!(in_variants, 4, "established inside a variant");
    assert_eq!(operative, established, "all of them are corpus invariants");
    assert_eq!(operative_in_variants, in_variants);
    assert_eq!(specimens, 0, "no real profile records a specimen promotion");

    // The number that has not moved and must not be read as if it had.
    assert_eq!(
        required, 0,
        "nothing in this repository has been load-tested, acceptance-tested or checked against a \
         shipping manifest, so no target REQUIRES anything"
    );

    // The synthetic fixture is the only place a target requirement exists, and it is excluded from
    // the registry population - so exercising the refusal path cannot inflate a real count.
    let synthetic = synthetic_profile();
    assert!(synthetic.synthetic);
    assert!(asset_registry_population(std::iter::once(&synthetic)).is_empty());
    assert!(target(&synthetic, "fixturethree-synthetic")
        .variants
        .iter()
        .any(|v| v.raster.is_target_required()));
}
