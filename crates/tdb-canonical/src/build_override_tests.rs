//! Coverage for the `build-mapping-override/1` contract.
//!
//! Every fixture and every constructed document here is **synthetic**. The profile they resolve
//! against is `synthetic-asset-profile.toml`, whose targets are not catalog units and whose
//! constraints describe no real game, so nothing in this file asserts a real target capability.

use super::*;
use std::path::Path;

/// A fixture exactly as it sits on disk, line endings and all.
///
/// Git stores these files with LF and hands them to a working tree in whatever the checkout's
/// `core.autocrlf` dictates, so the same commit yields LF bytes on one machine and CRLF bytes on
/// another. Use this when the *bytes* are the subject; use [`fixture`] to build textual cases.
fn fixture_raw(relative: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative),
    )
    .unwrap_or_else(|e| panic!("fixture {relative}: {e}"))
}

/// Collapse `\r\n` to `\n` and change nothing else.
///
/// A lone `\r` — one not followed by `\n` — is content, not a line ending, and is preserved; TOML
/// permits it inside a string and a test that smuggles one is testing that. This exists so the
/// multi-line anchors below describe the document rather than the checkout that produced it.
fn lf(text: String) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n")
    } else {
        text
    }
}

/// A fixture normalized to LF, for constructing textual cases.
///
/// Every anchor in this file is written with `\n`. Without this the anchors silently fail to match
/// on a CRLF checkout, and a mutation test that mutates nothing still "passes" whenever the
/// unedited document happens to satisfy the assertion.
fn fixture(relative: &str) -> String {
    lf(fixture_raw(relative))
}

/// Replace `from` with `to`, requiring exactly `expected` occurrences.
///
/// `str::replace` is silent about finding nothing, which turns a negative case into a test of the
/// untouched fixture. Naming the count also catches an anchor that has become ambiguous because the
/// fixture grew a second copy of it.
#[track_caller]
fn replace_exactly(text: &str, from: &str, to: &str, expected: usize) -> String {
    let found = text.matches(from).count();
    assert_eq!(
        found, expected,
        "anchor {from:?} occurs {found} times, expected {expected}"
    );
    text.replace(from, to)
}

/// The common case: an anchor that must appear exactly once.
#[track_caller]
fn replace_one(text: &str, from: &str, to: &str) -> String {
    replace_exactly(text, from, to, 1)
}

/// Replace every occurrence, requiring at least one.
///
/// Some edits are deliberately document-wide — changing a codec the record names in more than one
/// place, say — so pinning them to a single site would be wrong. The guard here is only that the
/// edit found something to do.
#[track_caller]
fn replace_present(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "anchor {from:?} occurs 0 times, expected at least one"
    );
    text.replace(from, to)
}

fn asset_profile() -> AssetProfile {
    parse_asset_profile(&fixture("synthetic-asset-profile.toml")).expect("the profile parses")
}

fn validated(profile: &AssetProfile) -> ValidatedProfile<'_> {
    ValidatedProfile::new(profile).expect("the synthetic profile validates")
}

fn record(name: &str) -> BuildOverride {
    let text = fixture(&format!("build-overrides/{name}.toml"));
    parse_build_override(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn record_from(name: &str, edits: &[(&str, &str)]) -> BuildOverride {
    let mut text = fixture(&format!("build-overrides/{name}.toml"));
    for (from, to) in edits {
        // Document-wide by design, matching this helper's long-standing behaviour: an edit such as
        // ("bc7", "rgba8") must reach every site the record names the codec at. The guard replaces
        // the silence of `str::replace`, not its reach.
        text = replace_present(&text, from, to);
    }
    parse_build_override(&text).unwrap_or_else(|e| panic!("{name} edited: {e}"))
}

/// The context in which a record is exactly current: its own declared source and mapping, and
/// nothing observed of the source. Enough for a target with no variants; anything variant-borne is
/// blocked until `observing` supplies the facts.
fn current(record: &BuildOverride) -> BuildContext {
    BuildContext::new(record.source.clone(), record.mapping.clone())
}

/// The same context, plus facts read off the record's own current source.
fn current_observing(record: &BuildOverride, facts: ObservedFacts) -> BuildContext {
    current_observing_by(record, facts, "header-read")
}

/// The same, by a named method - so a test can change how the facts were obtained and nothing else.
fn current_observing_by(
    record: &BuildOverride,
    facts: ObservedFacts,
    method: &str,
) -> BuildContext {
    current(record).observing(SourceObservation {
        of: record.source.clone(),
        method: method.to_string(),
        facts,
    })
}

/// Resolve a record against its own context and the synthetic profile.
fn resolve(record: &BuildOverride, profile: &AssetProfile) -> OverrideOutcome {
    resolve_in(record, profile, current(record))
}

/// Resolve against a context the caller built - a moved source, a stale observation, no facts.
fn resolve_in(
    record: &BuildOverride,
    profile: &AssetProfile,
    context: BuildContext,
) -> OverrideOutcome {
    let validated_record = ValidatedOverride::new(record)
        .unwrap_or_else(|f| panic!("the record must validate first: {f:?}"));
    resolve_build_override(&validated_record, &context, &validated(profile))
}

/// Re-approve a record: recompute its fingerprint so the approval is genuinely current. Used to
/// build the "unchanged inputs stay approved" baseline for each mutation case.
fn reapproved(record: BuildOverride, profile: &AssetProfile) -> BuildOverride {
    reapproved_observing(record, profile, None)
}

/// Re-approve against the facts a resolution will actually be handed. The observation is inside the
/// binding, so an approval built without it would not match a resolution that has one.
fn reapproved_observing(
    mut record: BuildOverride,
    profile: &AssetProfile,
    observation: Option<&SourceObservation>,
) -> BuildOverride {
    let digest = profile_content_digest(profile);
    record.profile.content_digest = digest.clone();
    let fingerprint = override_fingerprint(&record, &digest, observation);
    if let Some(approval) = record.approval.as_mut() {
        approval.fingerprint_version = APPROVAL_FINGERPRINT_VERSION;
        approval.fingerprint = fingerprint;
    }
    record
}

// ============================================================ the permitted case

#[test]
fn a_permitted_setting_choice_validates_and_resolves_to_an_approved_mapping() {
    // The established 1024x1024 page of 256x256 cells is 16 slots. Slot 7 is a CHOICE WITHIN that
    // proven constraint, which is the only thing an override may ever be.
    let record = record("synthetic-approved-slot");
    assert!(record.synthetic);
    let findings = validate_build_override(&record);
    assert_eq!(findings, vec![], "{findings:?}");
    assert_eq!(
        resolve(&record, &asset_profile()),
        OverrideOutcome::ApprovedMapping
    );
}

#[test]
fn the_committed_fixture_carries_the_digests_the_code_computes() {
    // The fixture is not merely self-consistent: its content digest is the one this build derives
    // from the profile, and its fingerprint is the one this build derives from the record. A
    // change to either construction fails here rather than silently re-approving everything.
    let profile = asset_profile();
    let record = record("synthetic-approved-slot");
    assert_eq!(
        record.profile.content_digest,
        profile_content_digest(&profile)
    );
    let approval = record.approval.as_ref().expect("approved");
    assert_eq!(approval.fingerprint_version, APPROVAL_FINGERPRINT_VERSION);
    assert_eq!(
        approval.fingerprint,
        override_fingerprint(&record, &profile_content_digest(&profile), None)
    );
}

#[test]
fn the_permitted_set_comes_from_the_constraint_and_the_record_cannot_widen_it() {
    let profile = asset_profile();
    let base = record("synthetic-approved-slot");
    // Every one of the 16 slots is selectable — with its own approval, which is the point of the
    // rest of this file. Nothing beyond them is selectable at all.
    for slot in 0..16u32 {
        let candidate = BuildOverride {
            proposal: Proposal {
                proposed: SettingChoice::AtlasSlot(AtlasSlotChoice { slot }),
                ..base.proposal.clone()
            },
            ..base.clone()
        };
        let candidate = reapproved(candidate, &profile);
        assert_eq!(
            resolve(&candidate, &profile),
            OverrideOutcome::ApprovedMapping,
            "slot {slot} is inside the established page"
        );
    }
    let outside = record_from("synthetic-approved-slot", &[("slot = 7", "slot = 16")]);
    match resolve(&outside, &profile) {
        OverrideOutcome::RefusedByConstraint { block, detail } => {
            assert_eq!(block, "atlas");
            assert!(detail.contains("16 slots"), "{detail}");
        }
        other => panic!("{other:?}"),
    }
}

// ================================== a known constraint is not negotiable by approval

#[test]
fn violating_a_known_constraint_is_refused_even_with_a_valid_approval_supplied() {
    // The record carries a complete, correctly bound, freshly recomputed approval. It is refused
    // anyway: the constraint is read first, and approval is never consulted for a value the target
    // rejects.
    let profile = asset_profile();
    let record = reapproved(record("synthetic-refused-codec"), &profile);
    assert!(record.approval.is_some());
    let findings = validate_build_override(&record);
    assert_eq!(
        findings,
        vec![],
        "the DOCUMENT is well formed: {findings:?}"
    );

    match resolve(&record, &profile) {
        OverrideOutcome::RefusedByConstraint { block, detail } => {
            assert_eq!(block, "raster");
            assert!(
                detail.contains("dxt5") && detail.contains("bc7"),
                "{detail}"
            );
        }
        other => panic!("an approval must not rescue a refused codec: {other:?}"),
    }
}

#[test]
fn removing_the_approval_changes_nothing_about_a_refusal() {
    // The proof that the refusal does not depend on the approval's state at all.
    let profile = asset_profile();
    let approved = reapproved(record("synthetic-refused-codec"), &profile);
    let unapproved = BuildOverride {
        approval: None,
        ..approved.clone()
    };
    assert_eq!(resolve(&approved, &profile), resolve(&unapproved, &profile));
}

#[test]
fn a_setting_the_target_does_not_have_is_refused_rather_than_permitted() {
    // `fixturetwo-synthetic` records [mipmaps] as not-applicable with its reason. That is a DECIDED
    // fact — the target has no mip setting to choose — so it refuses rather than blocks.
    let record = record_from(
        "synthetic-blocked-unknown",
        &[
            (
                "setting = \"audio-sample-rate\"",
                "setting = \"mipmap-count\"",
            ),
            ("hz = 44100", "count = 5"),
        ],
    );
    match resolve(&record, &asset_profile()) {
        OverrideOutcome::RefusedByConstraint { block, detail } => {
            assert_eq!(block, "mipmaps");
            assert!(detail.contains("no [mipmaps] constraint"), "{detail}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_pinned_constraint_permits_a_set_of_one_and_refuses_everything_else() {
    // `fixtureone-synthetic` establishes [dimensions] as one exact size for its variant. A profile
    // that pins a value permits a set of one, and a different size is outside it.
    let record = record_from(
        "synthetic-approved-slot",
        &[
            ("setting = \"atlas-slot\"\n[proposal.proposed.operand]\nslot = 7", "setting = \"dimensions\"\n[proposal.proposed.operand]\nwidth = 512\nheight = 512"),
            ("setting = \"atlas-slot\"\n[proposal.prior.value.operand]\nslot = 2", "setting = \"dimensions\"\n[proposal.prior.value.operand]\nwidth = 256\nheight = 256"),
        ],
    );
    match resolve(&record, &asset_profile()) {
        OverrideOutcome::RefusedByConstraint { block, detail } => {
            assert_eq!(block, "dimensions");
            assert!(detail.contains("256x256"), "{detail}");
        }
        other => panic!("{other:?}"),
    }
}

// ================================================ unknown constraints cannot be bypassed

#[test]
fn an_unknown_constraint_blocks_and_names_itself() {
    let record = record("synthetic-blocked-unknown");
    let findings = validate_build_override(&record);
    assert_eq!(findings, vec![], "{findings:?}");
    assert_eq!(
        resolve(&record, &asset_profile()),
        OverrideOutcome::BlockedByUnknownConstraint("audio".to_string())
    );
}

#[test]
fn no_approval_can_reach_past_an_unknown_constraint() {
    // Attaching a correctly fingerprinted approval to the blocked record changes nothing: a value
    // cannot be checked against a requirement nobody has established, so there is nothing to
    // approve.
    let profile = asset_profile();
    let blocked = record("synthetic-blocked-unknown");
    let approved = reapproved(
        BuildOverride {
            approval: Some(Approval {
                approved_by: "synthetic:operator".to_string(),
                approved_at: "2026-09-08".to_string(),
                fingerprint_version: APPROVAL_FINGERPRINT_VERSION,
                fingerprint: "0".repeat(64),
                note: String::new(),
            }),
            ..blocked.clone()
        },
        &profile,
    );
    assert_eq!(
        resolve(&approved, &profile),
        OverrideOutcome::BlockedByUnknownConstraint("audio".to_string())
    );
    assert_eq!(resolve(&blocked, &profile), resolve(&approved, &profile));
}

// ======================================= a proposal is not an approved mapping

#[test]
fn a_permitted_choice_nobody_approved_is_a_proposal_and_not_a_mapping() {
    let proposal = record("synthetic-proposal-only");
    assert!(proposal.approval.is_none());
    let findings = validate_build_override(&proposal);
    assert_eq!(findings, vec![], "{findings:?}");
    let outcome = resolve(&proposal, &asset_profile());
    assert_eq!(outcome, OverrideOutcome::Proposed);
    assert_ne!(outcome, OverrideOutcome::ApprovedMapping);
}

// =================================================== R2: exact approval binding
//
// Each case starts from the approved, valid mapping and changes exactly one thing. The baseline is
// re-asserted in every case, so a mutation that fails to invalidate cannot hide behind a fixture
// that was never approved in the first place.

/// Every mutation below must stop the approval applying; the unmutated record must still apply.
///
/// Two assertions, because they prove different things. The **fingerprint** one is the binding
/// itself: the stored digest no longer describes this record, whatever else is true of it. The
/// **outcome** one is what a Build sees. They are kept separate because some mutations are stopped
/// earlier than the approval step — renaming the target sends the record to a target whose atlas
/// extents are not established, so it is refused by the constraint before any approval is read.
/// That is the ordering working as intended, and it must not be mistaken for the binding working.
fn mutation_rejects(what: &str, mutate: impl FnOnce(&mut BuildOverride)) {
    let profile = asset_profile();
    let digest = profile_content_digest(&profile);
    let base = record("synthetic-approved-slot");
    assert_eq!(
        resolve(&base, &profile),
        OverrideOutcome::ApprovedMapping,
        "{what}: the baseline must be approved before the mutation means anything"
    );
    let before = override_fingerprint(&base, &digest, None);

    let mut mutated = base;
    mutate(&mut mutated);
    let validated_record = ValidatedOverride::new(&mutated)
        .unwrap_or_else(|f| panic!("{what}: the mutation must stay a WELL-FORMED record: {f:?}"));

    assert_ne!(
        override_fingerprint(&mutated, &digest, None),
        before,
        "{what}: the approval's fingerprint must no longer describe this record"
    );

    let outcome =
        resolve_build_override(&validated_record, &current(&mutated), &validated(&profile));
    assert_ne!(
        outcome,
        OverrideOutcome::ApprovedMapping,
        "{what}: the approval must not survive this"
    );
}

#[test]
fn changing_the_proposed_value_to_another_permitted_value_invalidates_the_approval() {
    // THE DEFECT THIS PACKAGE EXISTS TO CLOSE. Slot 5 is as permitted as slot 7, and every
    // revision and digest in the record is untouched. Under the old four-field comparison this
    // reused the approval; the fingerprint covers the proposed value, so it cannot.
    mutation_rejects("proposed value", |r| {
        r.proposal.proposed = SettingChoice::AtlasSlot(AtlasSlotChoice { slot: 5 });
    });
}

#[test]
fn changing_the_setting_itself_invalidates_the_approval() {
    // Both choices are valid against this target: slot 7 is inside the atlas, and dxt5 is the
    // established codec. Approving one is not approving the other.
    mutation_rejects("setting", |r| {
        r.proposal.proposed = SettingChoice::RasterCodec(RasterCodecChoice {
            codec: "dxt5".to_string(),
        });
        r.proposal.prior = PriorValue {
            absent: true,
            reason: "synthetic: this mapping carried no codec setting".to_string(),
            value: None,
        };
    });
}

#[test]
fn renaming_the_source_asset_invalidates_the_approval_even_with_identical_bytes() {
    // Same revision, same digest, same everything the old comparison looked at.
    mutation_rejects("source asset id", |r| {
        r.source.asset_id = "synthetic:arena-texture/rink-corner".to_string();
    });
}

#[test]
fn renaming_the_target_or_the_mapping_invalidates_the_approval_at_an_identical_revision() {
    mutation_rejects("mapping id", |r| {
        r.mapping.mapping_id = "arena-texture/rink-side".to_string();
    });
    // A different target is a different mapping. Here the outcome is a constraint refusal rather
    // than an invalidated approval — `fixturetwo-synthetic` establishes an atlas whose page and
    // cell extents are not both known — so the fingerprint assertion is what proves the binding
    // covers the target, and the outcome assertion proves the approval did not survive either way.
    mutation_rejects("target", |r| {
        r.mapping.target = "fixturetwo-synthetic".to_string();
    });
}

#[test]
fn changing_the_prior_value_the_resolution_or_the_provenance_invalidates_the_approval() {
    mutation_rejects("prior value", |r| {
        r.proposal.prior = PriorValue {
            absent: false,
            reason: String::new(),
            value: Some(SettingChoice::AtlasSlot(AtlasSlotChoice { slot: 9 })),
        };
    });
    mutation_rejects("prior absence", |r| {
        r.proposal.prior = PriorValue {
            absent: true,
            reason: "synthetic: nothing occupied this setting".to_string(),
            value: None,
        };
    });
    mutation_rejects("resolution method", |r| {
        r.proposal.resolution = "automatic-fit".to_string();
    });
    mutation_rejects("provenance", |r| {
        r.proposal
            .provenance
            .push("synthetic:second-citation".to_string());
    });
}

#[test]
fn editing_the_profile_in_place_invalidates_the_approval_at_an_identical_version() {
    // The profile keeps its `profile_version` and changes a constraint. A version number cannot
    // detect this; the content digest does.
    let mut edited = asset_profile();
    assert_eq!(edited.profile_version, 2);
    let target = edited
        .targets
        .iter_mut()
        .find(|t| t.target == "fixtureone-synthetic")
        .expect("declared");
    let atlas = target.atlas.value.as_mut().expect("established");
    atlas.page_width = Some(2048);
    assert_eq!(
        edited.profile_version, 2,
        "the version is deliberately unmoved"
    );

    let record = record("synthetic-approved-slot");
    let validated_record = ValidatedOverride::new(&record).expect("well formed");
    match resolve_build_override(&validated_record, &current(&record), &validated(&edited)) {
        OverrideOutcome::RecordProfileMismatch(fields) => {
            assert_eq!(fields, vec!["profile.content_digest".to_string()]);
        }
        other => panic!("a profile edited in place must not stay approved: {other:?}"),
    }
    // And the digest genuinely moved.
    assert_ne!(
        profile_content_digest(&edited),
        profile_content_digest(&asset_profile())
    );
}

#[test]
fn a_changed_current_context_invalidates_the_approval_without_the_record_being_touched() {
    // Nothing in the record changes. The world moves: the asset is re-exported at a new revision
    // and digest, or the mapping is renamed. The record is stale either way, and it says so.
    let profile = asset_profile();
    let record = record("synthetic-approved-slot");
    assert_eq!(resolve(&record, &profile), OverrideOutcome::ApprovedMapping);

    let validated_record = ValidatedOverride::new(&record).expect("well formed");
    for (what, context, expected) in [
        (
            "a re-exported source",
            BuildContext {
                source: SourceIdentity {
                    revision: 5,
                    hash: "a".repeat(64),
                    ..record.source.clone()
                },
                mapping: record.mapping.clone(),
                observation: None,
            },
            vec!["source.revision".to_string(), "source.hash".to_string()],
        ),
        (
            "a renamed asset with identical bytes",
            BuildContext {
                source: SourceIdentity {
                    asset_id: "synthetic:arena-texture/rink-corner".to_string(),
                    ..record.source.clone()
                },
                mapping: record.mapping.clone(),
                observation: None,
            },
            vec!["source.asset_id".to_string()],
        ),
        (
            "a renamed mapping at the same revision",
            BuildContext {
                source: record.source.clone(),
                mapping: MappingIdentity {
                    mapping_id: "arena-texture/rink-side".to_string(),
                    ..record.mapping.clone()
                },
                observation: None,
            },
            vec!["mapping.mapping_id".to_string()],
        ),
    ] {
        let outcome = resolve_build_override(&validated_record, &context, &validated(&profile));
        assert_eq!(
            outcome,
            OverrideOutcome::StaleContext(expected),
            "{what} must not stay approved"
        );
    }
}

#[test]
fn an_approval_from_another_fingerprint_construction_is_invalidated_not_reinterpreted() {
    let profile = asset_profile();
    let mut record = record("synthetic-approved-slot");
    record
        .approval
        .as_mut()
        .expect("approved")
        .fingerprint_version = APPROVAL_FINGERPRINT_VERSION + 1;
    assert_eq!(
        resolve(&record, &profile),
        OverrideOutcome::ApprovalInvalidated(vec!["approval.fingerprint_version".to_string()])
    );
}

#[test]
fn unchanged_inputs_stay_approved_across_repeated_resolution() {
    // The fingerprint is deterministic: resolving the same record against the same context and
    // profile any number of times gives the same answer, and it is still an approval.
    let profile = asset_profile();
    let record = record("synthetic-approved-slot");
    for _ in 0..3 {
        assert_eq!(resolve(&record, &profile), OverrideOutcome::ApprovedMapping);
    }
    assert_eq!(
        override_fingerprint(&record, &profile_content_digest(&profile), None),
        override_fingerprint(&record, &profile_content_digest(&profile), None)
    );
}

#[test]
fn the_fingerprint_excludes_exactly_the_fields_that_decide_nothing() {
    // Prose and bookkeeping must not invalidate an approval, because they cannot change what a
    // Build produces. Everything that can is covered by the cases above.
    let profile = asset_profile();
    let digest = profile_content_digest(&profile);
    let base = record("synthetic-approved-slot");
    let before = override_fingerprint(&base, &digest, None);

    for (what, mutate) in [
        (
            "record note",
            Box::new(|r: &mut BuildOverride| r.note = "reworded".to_string())
                as Box<dyn FnOnce(&mut BuildOverride)>,
        ),
        (
            "proposal note",
            Box::new(|r: &mut BuildOverride| r.proposal.note = "reworded".to_string()),
        ),
        (
            "record_version",
            Box::new(|r: &mut BuildOverride| r.record_version += 1),
        ),
        (
            "synthetic label",
            Box::new(|r: &mut BuildOverride| r.synthetic = !r.synthetic),
        ),
        (
            "approval note",
            Box::new(|r: &mut BuildOverride| {
                r.approval.as_mut().expect("approved").note = "reworded".to_string()
            }),
        ),
    ] {
        let mut mutated = base.clone();
        mutate(&mut mutated);
        assert_eq!(
            override_fingerprint(&mutated, &digest, None),
            before,
            "{what} must not be part of the fingerprint"
        );
    }

    // The approval cannot be inside its own fingerprint, which is why replacing it wholesale
    // leaves the expected digest where it was.
    let mut reapproved_elsewhere = base.clone();
    reapproved_elsewhere
        .approval
        .as_mut()
        .expect("approved")
        .approved_by = "synthetic:another-operator".to_string();
    assert_eq!(
        override_fingerprint(&reapproved_elsewhere, &digest, None),
        before,
        "the approval is excluded from its own fingerprint"
    );
}

#[test]
fn the_profile_digest_ignores_prose_and_the_non_operative_candidate_channel() {
    // A candidate cannot change what a proposal is checked against — `established()` cannot see one
    // — so an editorial change to a profile must not invalidate an approval.
    let base = asset_profile();
    let before = profile_content_digest(&base);

    let mut renoted = base.clone();
    renoted.note = "reworded".to_string();
    renoted.targets[0].note = "reworded".to_string();
    renoted.targets[0].discovery.note = "reworded".to_string();
    assert_eq!(profile_content_digest(&renoted), before);

    let mut recandidated = base.clone();
    recandidated.targets[1].mesh_layout.candidate = None;
    assert_eq!(
        profile_content_digest(&recandidated),
        before,
        "the candidate channel is non-operative and outside the digest"
    );

    // But an operative change moves it.
    let mut restated = base.clone();
    restated.targets[0].tiling.state = ConstraintState::Unknown;
    assert_ne!(profile_content_digest(&restated), before);
}

// ================================================== targets stay independent

#[test]
fn one_targets_record_never_reaches_another_targets_mapping() {
    let profile = asset_profile();
    let one = record("synthetic-approved-slot");
    let two = record("synthetic-blocked-unknown");
    assert_ne!(one.mapping.target, two.mapping.target);

    assert_eq!(resolve(&one, &profile), OverrideOutcome::ApprovedMapping);
    assert_eq!(
        resolve(&two, &profile),
        OverrideOutcome::BlockedByUnknownConstraint("audio".to_string())
    );

    let absent = BuildOverride {
        mapping: MappingIdentity {
            target: "fixtureabsent-synthetic".to_string(),
            ..one.mapping.clone()
        },
        ..one
    };
    assert_eq!(
        resolve(&absent, &profile),
        OverrideOutcome::NoSuchTarget("fixtureabsent-synthetic".to_string())
    );
}

// ============================ canonical assets and profiles are not touched

#[test]
fn resolving_an_override_changes_neither_the_profile_nor_the_record() {
    let profile_before = asset_profile();
    let records: Vec<BuildOverride> = [
        "synthetic-approved-slot",
        "synthetic-refused-codec",
        "synthetic-blocked-unknown",
        "synthetic-proposal-only",
    ]
    .iter()
    .map(|n| record(n))
    .collect();
    let records_before = records.clone();

    let profile_after = profile_before.clone();
    for r in &records {
        let _ = validate_build_override(r);
        if let Ok(validated_record) = ValidatedOverride::new(r) {
            let _ =
                resolve_build_override(&validated_record, &current(r), &validated(&profile_after));
        }
    }
    assert_eq!(profile_before, profile_after, "the profile is untouched");
    assert_eq!(records_before, records, "the records are untouched");
    assert_eq!(
        profile_before,
        parse_asset_profile(&fixture("synthetic-asset-profile.toml")).expect("parses")
    );
}

#[test]
fn no_override_record_can_name_a_canonical_source_field_to_change() {
    // The record identifies the canonical source; it has no key that could modify it.
    let text = fixture("build-overrides/synthetic-approved-slot.toml");
    for (from, to) in [
        (
            "[source]\nasset_id",
            "[source]\nrewrite_canonical = true\nasset_id",
        ),
        (
            "[proposal]\nresolution",
            "[proposal]\ncanonical_value = 3\nresolution",
        ),
    ] {
        let bad = replace_one(&text, from, to);
        assert!(
            parse_build_override(&bad).is_err(),
            "{to:?} must not be expressible"
        );
    }
}

// ================================ malformed input cannot reach an approved mapping

#[test]
fn a_malformed_record_cannot_be_resolved_at_all() {
    // The check is at the type boundary: there is no way to call the resolver without having
    // validated, so no malformed record can produce ApprovedMapping.
    let malformed = record_from(
        "synthetic-approved-slot",
        &[("record_version = 1", "record_version = 0")],
    );
    let refused = ValidatedOverride::new(&malformed).expect_err("must not validate");
    assert!(refused.iter().any(|f| f.rule == Rule::UnsupportedVersion));
}

#[test]
fn a_malformed_profile_cannot_be_resolved_against() {
    let mut broken = asset_profile();
    broken.family = "assets-not-a-family".to_string();
    let refused = ValidatedProfile::new(&broken).expect_err("must not validate");
    assert!(refused.iter().any(|f| f.rule == Rule::AssetFamilyUnknown));
}

#[test]
fn an_unknown_field_anywhere_is_a_parse_error_not_a_silent_drop() {
    let text = fixture("build-overrides/synthetic-approved-slot.toml");
    for (from, to) in [
        ("record_version = 1", "record_version = 1\nunexpected = 1"),
        ("[source]\nasset_id", "[source]\nprovenence = []\nasset_id"),
        ("[mapping]\ntarget", "[mapping]\nplatform = \"ps4\"\ntarget"),
        ("[profile]\nfamily", "[profile]\nadapter = \"x\"\nfamily"),
        (
            "[proposal]\nresolution",
            "[proposal]\nsetting = \"x\"\nresolution",
        ),
        (
            "[proposal.proposed.operand]\nslot = 7",
            "[proposal.proposed.operand]\nslot = 7\nindex = 1",
        ),
        (
            "[approval]\napproved_by",
            "[approval]\ndelegated = true\napproved_by",
        ),
        // The four copied inputs the old approval carried are gone, and cannot be reintroduced by
        // a stale record: they would silently look like a binding while binding nothing.
        (
            "[approval]\napproved_by",
            "[approval]\nsource_revision = 4\napproved_by",
        ),
    ] {
        let bad = replace_one(&text, from, to);
        assert!(
            parse_build_override(&bad).is_err(),
            "{to:?} must be refused"
        );
    }
}

#[test]
fn a_support_or_acceptance_claim_has_no_spelling_in_an_override_record() {
    let text = fixture("build-overrides/synthetic-approved-slot.toml");
    for claim in [
        "converter = \"studio\"",
        "supported = true",
        "qualification = \"implemented\"",
        "game_verified = true",
    ] {
        let bad = replace_one(
            &text,
            "record_version = 1",
            &format!("record_version = 1\n{claim}"),
        );
        assert!(
            parse_build_override(&bad).is_err(),
            "{claim} must not be expressible in an override record"
        );
    }
}

#[test]
fn an_unknown_setting_or_a_mistyped_operand_is_a_parse_error() {
    let text = fixture("build-overrides/synthetic-approved-slot.toml");
    for (from, to) in [
        (
            "setting = \"atlas-slot\"\n[proposal.proposed.operand]\nslot = 7",
            "setting = \"colour-grade\"\n[proposal.proposed.operand]\nslot = 7",
        ),
        (
            "[proposal.proposed.operand]\nslot = 7",
            "[proposal.proposed.operand]\nslot = \"seven\"",
        ),
    ] {
        let bad = replace_one(&text, from, to);
        assert!(
            parse_build_override(&bad).is_err(),
            "{to:?} must be refused"
        );
    }
    assert_eq!(OVERRIDE_SETTING.len(), 7);
}

#[test]
fn a_wrong_schema_or_a_zero_revision_is_reported_rather_than_assumed() {
    let record = record_from(
        "synthetic-approved-slot",
        &[
            (
                "schema = \"build-mapping-override/1\"",
                "schema = \"build-mapping-override/2\"",
            ),
            ("record_version = 1", "record_version = 0"),
        ],
    );
    let rules: Vec<Rule> = validate_build_override(&record)
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(rules.contains(&Rule::UnsupportedSchema));
    assert!(rules.contains(&Rule::UnsupportedVersion));
}

#[test]
fn a_record_bound_to_nothing_is_incomplete() {
    for (edits, expected) in [
        (
            vec![(
                "[source]\nasset_id = \"synthetic:arena-texture/rink-centre\"\nrevision = 4",
                "[source]\nasset_id = \"synthetic:arena-texture/rink-centre\"\nrevision = 0",
            )],
            Rule::OverrideRecordIncomplete,
        ),
        (
            vec![("provenance = [\"synthetic:fixture\"]", "provenance = []")],
            Rule::OverrideRecordIncomplete,
        ),
        (
            vec![(
                "mapping_id = \"arena-texture/rink-centre\"",
                "mapping_id = \"\"",
            )],
            Rule::OverrideRecordIncomplete,
        ),
        (
            vec![(
                "family = \"assets-arenas\"\nschema",
                "family = \"assets-jerseys\"\nschema",
            )],
            Rule::OverrideRecordIncomplete,
        ),
        (
            vec![("fingerprint_version = 2", "fingerprint_version = 0")],
            Rule::OverrideApprovalUnbound,
        ),
        (
            vec![("approved_at = \"2026-09-08\"", "approved_at = \"soon\"")],
            Rule::OverrideApprovalUnbound,
        ),
    ] {
        let record = record_from("synthetic-approved-slot", &edits);
        let rules: Vec<Rule> = validate_build_override(&record)
            .into_iter()
            .map(|f| f.rule)
            .collect();
        assert!(rules.contains(&expected), "{edits:?} -> {rules:?}");
    }
}

#[test]
fn a_truncated_digest_is_refused_wherever_one_is_required() {
    for (from, to, expected) in [
        (
            "content_digest = \"",
            "content_digest = \"abcd\" #",
            Rule::OverrideRecordIncomplete,
        ),
        (
            "fingerprint = \"",
            "fingerprint = \"abcd\" #",
            Rule::OverrideApprovalUnbound,
        ),
    ] {
        let text = replace_one(
            &fixture("build-overrides/synthetic-approved-slot.toml"),
            from,
            to,
        );
        let record = parse_build_override(&text).expect("still parses");
        let rules: Vec<Rule> = validate_build_override(&record)
            .into_iter()
            .map(|f| f.rule)
            .collect();
        assert!(rules.contains(&expected), "{to} -> {rules:?}");
    }
}

#[test]
fn a_prior_value_is_either_recorded_or_explicitly_absent_and_never_both() {
    let both = record_from(
        "synthetic-approved-slot",
        &[(
            "[proposal.prior]\nabsent = false",
            "[proposal.prior]\nabsent = true\nreason = \"synthetic\"",
        )],
    );
    let rules: Vec<Rule> = validate_build_override(&both)
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(rules.contains(&Rule::OverridePriorConflict), "{rules:?}");

    let neither = record_from(
        "synthetic-approved-slot",
        &[("[proposal.prior]\nabsent = false\n[proposal.prior.value]\nsetting = \"atlas-slot\"\n[proposal.prior.value.operand]\nslot = 2", "[proposal.prior]\nabsent = false")],
    );
    let rules: Vec<Rule> = validate_build_override(&neither)
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(rules.contains(&Rule::OverridePriorConflict), "{rules:?}");

    let unexplained = record_from(
        "synthetic-blocked-unknown",
        &[("reason = \"Synthetic. This mapping was created by this Build and carried no sample-rate setting before.\"", "reason = \"\"")],
    );
    let rules: Vec<Rule> = validate_build_override(&unexplained)
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(rules.contains(&Rule::OverrideRecordIncomplete), "{rules:?}");
}

#[test]
fn a_prior_value_must_describe_the_setting_the_proposal_changes() {
    let record = record_from(
        "synthetic-approved-slot",
        &[("[proposal.prior.value]\nsetting = \"atlas-slot\"\n[proposal.prior.value.operand]\nslot = 2", "[proposal.prior.value]\nsetting = \"raster-codec\"\n[proposal.prior.value.operand]\ncodec = \"dxt5\"")],
    );
    let rules: Vec<Rule> = validate_build_override(&record)
        .into_iter()
        .map(|f| f.rule)
        .collect();
    assert!(rules.contains(&Rule::OverridePriorConflict), "{rules:?}");
}

#[test]
fn every_override_rule_has_a_stable_diagnosable_name() {
    let all = [
        Rule::OverrideRecordIncomplete,
        Rule::OverridePriorConflict,
        Rule::OverrideApprovalUnbound,
    ];
    let names: std::collections::BTreeSet<&str> = all.iter().map(Rule::as_str).collect();
    assert_eq!(names.len(), all.len());
    assert!(names.iter().all(|n| !n.is_empty()));
}

#[test]
fn every_committed_override_fixture_is_labelled_synthetic() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/build-overrides");
    let mut seen = 0;
    for entry in std::fs::read_dir(&directory).expect("the fixture directory exists") {
        let path = entry.expect("readable").path();
        if path.extension().is_none_or(|e| e != "toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable");
        let record = parse_build_override(&text).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert!(record.synthetic, "{path:?} must be labelled synthetic");
        assert!(
            record.mapping.target.contains("fixture"),
            "{path:?} must not name a catalog target"
        );
        seen += 1;
    }
    assert_eq!(seen, 4);
}

// ================================== correlated variants, and the force a constraint actually has

/// A record against `fixturethree-synthetic`, carrying the selector facts a Build would have.
fn variant_record(
    setting: SettingChoice,
    selector: Option<VariantSelector>,
    profile: &AssetProfile,
) -> BuildOverride {
    let record = BuildOverride {
        schema: BUILD_OVERRIDE_SCHEMA.to_string(),
        record_version: 1,
        synthetic: true,
        note: "Synthetic.".to_string(),
        source: SourceIdentity {
            asset_id: "synthetic:arena-texture/banner".to_string(),
            revision: 1,
            hash_algorithm: "sha256".to_string(),
            hash: "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff".to_string(),
        },
        mapping: MappingIdentity {
            target: "fixturethree-synthetic".to_string(),
            family: "assets-arenas".to_string(),
            mapping_id: "arena-texture/banner".to_string(),
            revision: 1,
        },
        profile: ProfileIdentity {
            family: "assets-arenas".to_string(),
            schema: ASSET_PROFILE_SCHEMA.to_string(),
            profile_version: profile.profile_version,
            content_digest: profile_content_digest(profile),
        },
        proposal: Proposal {
            proposed: setting,
            variant_selector: selector,
            prior: PriorValue {
                absent: true,
                reason: "Synthetic. A new mapping.".to_string(),
                value: None,
            },
            resolution: "operator-selected".to_string(),
            provenance: vec!["synthetic:fixture".to_string()],
            note: String::new(),
        },
        approval: Some(Approval {
            approved_by: "synthetic:operator".to_string(),
            approved_at: "2026-09-08".to_string(),
            fingerprint_version: APPROVAL_FINGERPRINT_VERSION,
            fingerprint: "0".repeat(64),
            note: String::new(),
        }),
    };
    reapproved(record, profile)
}

fn banner(width: u32, height: u32) -> VariantSelector {
    VariantSelector {
        path_family: Some("banner".to_string()),
        bucket: None,
        numeric_id: None,
        width: Some(width),
        height: Some(height),
    }
}

/// What a Build READ off the source, as opposed to what the record claims about it.
fn banner_facts(width: u32, height: u32) -> ObservedFacts {
    ObservedFacts {
        path_family: Some("banner".to_string()),
        width: Some(width),
        height: Some(height),
        ..Default::default()
    }
}

fn facts_of(family: &str) -> ObservedFacts {
    ObservedFacts {
        path_family: Some(family.to_string()),
        ..Default::default()
    }
}

/// One case: a record, and the context a resolution will be handed. The record is approved against
/// that exact context, so anything the test then changes is the only thing that moved.
fn variant_case(
    setting: SettingChoice,
    selector: Option<VariantSelector>,
    facts: Option<ObservedFacts>,
    profile: &AssetProfile,
) -> (BuildOverride, BuildContext) {
    let record = variant_record(setting, selector, profile);
    let context = match facts {
        Some(facts) => current_observing(&record, facts),
        None => current(&record),
    };
    let record = reapproved_observing(record, profile, context.observation.as_ref());
    (record, context)
}

#[test]
fn one_target_supports_several_variants_and_a_choice_is_checked_against_the_selected_one() {
    // The `/2` gap: one dimension per target, so a target carrying several forms could carry none.
    // Here the same target resolves against two different variants' constraints depending only on
    // what the CURRENT SOURCE turns out to be.
    let profile = asset_profile();
    let bc7 = || {
        SettingChoice::RasterCodec(RasterCodecChoice {
            codec: "bc7".to_string(),
        })
    };

    // `banner-512x128` establishes codec `bc7`. Proposing it is source-preserving and permitted.
    let (permitted, context) = variant_case(
        bc7(),
        Some(banner(512, 128)),
        Some(banner_facts(512, 128)),
        &profile,
    );
    assert_eq!(
        resolve_in(&permitted, &profile, context),
        OverrideOutcome::ApprovedMapping
    );

    // The OTHER banner form leaves its codec unknown, so the very same proposal - same target, same
    // setting, same value - is blocked there. One target, two answers, decided by the source.
    let (other_form, context) = variant_case(
        bc7(),
        Some(banner(512, 256)),
        Some(banner_facts(512, 256)),
        &profile,
    );
    assert_eq!(
        resolve_in(&other_form, &profile, context),
        OverrideOutcome::BlockedByUnknownConstraint("raster".to_string())
    );

    // And the `hero` variant requires `dxt5`, so `bc7` is REFUSED there rather than blocked.
    let (hero, context) = variant_case(
        bc7(),
        Some(VariantSelector {
            path_family: Some("hero".to_string()),
            ..Default::default()
        }),
        Some(facts_of("hero")),
        &profile,
    );
    match resolve_in(&hero, &profile, context) {
        OverrideOutcome::RefusedByConstraint { block, .. } => assert_eq!(block, "raster"),
        other => panic!("a target-required codec refuses: {other:?}"),
    }
}

#[test]
fn a_source_matching_one_variant_cannot_obtain_another_variants_approved_mapping() {
    // C0-AS3-R1's central case, and the one AS3 got wrong. Under AS3 the record's own selector
    // chose the variant, so a record could reach a permitted mapping by DESCRIBING its source as
    // the form whose constraints it wanted. Selection now runs on facts read off the source.
    let profile = asset_profile();
    let bc7 = || {
        SettingChoice::RasterCodec(RasterCodecChoice {
            codec: "bc7".to_string(),
        })
    };

    // The honest case first, so the mutation below is the only thing that changed: a 512x128
    // source, a record that says so, and an approval bound to both.
    let (honest, honest_context) = variant_case(
        bc7(),
        Some(banner(512, 128)),
        Some(banner_facts(512, 128)),
        &profile,
    );
    assert_eq!(
        resolve_in(&honest, &profile, honest_context),
        OverrideOutcome::ApprovedMapping
    );

    // Now the same source - 512x256, whose codec is unknown - with a record claiming it is the
    // 512x128 form, and an approval that MATCHES that record. The approval is genuine; the claim
    // is not. Refused at step 3, before approval is looked at.
    let source = banner_facts(512, 256);
    let (liar, context) = variant_case(bc7(), Some(banner(512, 128)), Some(source), &profile);
    assert_eq!(
        override_fingerprint(
            &liar,
            &profile_content_digest(&profile),
            context.observation.as_ref()
        ),
        liar.approval.as_ref().expect("approved").fingerprint,
        "the approval must be current, or this proves nothing about selection"
    );
    match resolve_in(&liar, &profile, context) {
        OverrideOutcome::SelectorExpectationMismatch(fields) => {
            assert_eq!(
                fields,
                vec!["proposal.variant_selector.height: record 128, source 256".to_string()]
            );
        }
        other => panic!("a contradicted expectation is refused: {other:?}"),
    }

    // The same attempt through the path family rather than the size, and the same answer.
    let (family_liar, context) = variant_case(
        bc7(),
        Some(VariantSelector {
            path_family: Some("hero".to_string()),
            ..Default::default()
        }),
        Some(banner_facts(512, 256)),
        &profile,
    );
    match resolve_in(&family_liar, &profile, context) {
        OverrideOutcome::SelectorExpectationMismatch(fields) => assert_eq!(
            fields,
            vec![
                "proposal.variant_selector.path_family: record \"hero\", source \"banner\""
                    .to_string()
            ]
        ),
        other => panic!("a contradicted expectation is refused: {other:?}"),
    }

    // And with the claim removed entirely, the record cannot reach the 512x128 form either: the
    // source is what it is, and 512x256's codec is unknown.
    let (silent, context) = variant_case(bc7(), None, Some(banner_facts(512, 256)), &profile);
    assert_eq!(
        resolve_in(&silent, &profile, context),
        OverrideOutcome::BlockedByUnknownConstraint("raster".to_string())
    );
}

#[test]
fn a_missing_or_misattributed_observation_blocks_rather_than_letting_the_record_decide() {
    let profile = asset_profile();
    let bc7 = || {
        SettingChoice::RasterCodec(RasterCodecChoice {
            codec: "bc7".to_string(),
        })
    };

    // No observation at all. The record still says what it always said, and it decides nothing:
    // an unobserved source is not a source that matches everything.
    let (record, context) = variant_case(bc7(), Some(banner(512, 128)), None, &profile);
    match resolve_in(&record, &profile, context) {
        OverrideOutcome::BlockedByUnobservedSource(detail) => {
            assert!(detail.contains("raster"), "{detail}")
        }
        other => panic!("an unobserved source blocks: {other:?}"),
    }

    // Facts read off a DIFFERENT source - a previous export at another revision and digest - are
    // not facts about this one, whatever they say.
    let (record, _) = variant_case(
        bc7(),
        Some(banner(512, 128)),
        Some(banner_facts(512, 128)),
        &profile,
    );
    let stale = current(&record).observing(SourceObservation {
        of: SourceIdentity {
            revision: 4,
            hash: "b".repeat(64),
            ..record.source.clone()
        },
        method: "header-read".to_string(),
        facts: banner_facts(512, 128),
    });
    assert_eq!(
        resolve_in(&record, &profile, stale),
        OverrideOutcome::ObservationNotOfSource(vec![
            "observation.of.revision".to_string(),
            "observation.of.hash".to_string()
        ])
    );

    // A partial read - the family, but not the size that separates the two banner forms - leaves
    // the selection undecidable. Refused, and it names what would have to be decided.
    let (record, context) = variant_case(bc7(), None, Some(facts_of("banner")), &profile);
    assert_eq!(
        resolve_in(&record, &profile, context),
        OverrideOutcome::RefusedByAmbiguousVariant(vec![
            "banner-512x128".to_string(),
            "banner-512x256".to_string()
        ])
    );

    // An expectation the source cannot CONFIRM is refused too, rather than quietly dropped: a
    // record that states a fact is asking to be held to it.
    let (record, context) = variant_case(
        bc7(),
        Some(banner(512, 128)),
        Some(facts_of("banner")),
        &profile,
    );
    match resolve_in(&record, &profile, context) {
        OverrideOutcome::SelectorExpectationMismatch(fields) => assert_eq!(fields.len(), 2),
        other => panic!("an unconfirmable expectation is refused: {other:?}"),
    }
}

#[test]
fn an_ambiguous_source_is_refused_and_a_matching_none_is_reported_as_such() {
    let profile = asset_profile();
    let bc7 = || {
        SettingChoice::RasterCodec(RasterCodecChoice {
            codec: "bc7".to_string(),
        })
    };

    // A source read at a size no form carries matches nothing. An explicit unsupported result, not
    // the nearest form.
    let (nowhere, context) = variant_case(
        bc7(),
        Some(banner(64, 64)),
        Some(banner_facts(64, 64)),
        &profile,
    );
    assert!(matches!(
        resolve_in(&nowhere, &profile, context),
        OverrideOutcome::NoMatchingVariant(_)
    ));

    // Nothing read at all leaves every variant live. Same refusal, three names.
    let (unread, context) = variant_case(bc7(), None, Some(ObservedFacts::default()), &profile);
    match resolve_in(&unread, &profile, context) {
        OverrideOutcome::RefusedByAmbiguousVariant(ids) => assert_eq!(ids.len(), 3),
        other => panic!("an undecidable source must refuse, not guess: {other:?}"),
    }

    // A setting whose block is not variant-borne on this target is decided at target level and
    // needs no observation at all - so the gate is narrow rather than a blanket refusal.
    let (compression, context) = variant_case(
        SettingChoice::CompressionPayload(CompressionPayloadChoice {
            payload: "chunkzip".to_string(),
        }),
        None,
        None,
        &profile,
    );
    assert_eq!(
        resolve_in(&compression, &profile, context),
        OverrideOutcome::ApprovedMapping
    );
}

#[test]
fn an_observed_source_format_authorises_reproducing_it_and_nothing_else() {
    // The C0-POR1-R1 distinction, made operative. Reading `BIGF` on every file in an installed copy
    // establishes what those files ARE. It does not establish that the game rejects anything else -
    // so departing is BLOCKED, not refused, and the outcome says which of the two it is.
    let profile = asset_profile();

    let departing = variant_record(
        SettingChoice::CompressionPayload(CompressionPayloadChoice {
            payload: "refpack".to_string(),
        }),
        None,
        &profile,
    );
    match resolve(&departing, &profile) {
        OverrideOutcome::BlockedByUnprovenConstraint { block, detail } => {
            assert_eq!(block, "compression");
            assert!(
                detail.contains("observed-source-format"),
                "the outcome must say WHY it is blocked rather than refused: {detail}"
            );
        }
        other => {
            panic!("an observed source format blocks a departure, never refuses it: {other:?}")
        }
    }

    // On `fixtureone-synthetic` the same kind of departure IS refused, because that target's
    // constraint is a demonstrated target requirement rather than an observation.
    let required = record_from("synthetic-refused-codec", &[("bc7", "rgba8")]);
    let required = reapproved(required, &profile);
    match resolve(&required, &profile) {
        OverrideOutcome::RefusedByConstraint { block, .. } => assert_eq!(block, "raster"),
        other => panic!("a target requirement refuses: {other:?}"),
    }

    // Neither outcome can be reached past by an approval: both are decided before step 5.
    assert!(departing.approval.is_some());
    assert!(required.approval.is_some());
}

#[test]
fn a_caller_cannot_select_a_convenient_variant_to_bypass_a_constraint() {
    // A record states the observable facts; the PROFILE selects. There is no key in which a record
    // could name a variant, and `deny_unknown_fields` makes adding one a parse error rather than a
    // validation warning - the same mechanism that stops a profile recording converter support.
    let text = replace_one(
        &fixture("build-overrides/synthetic-approved-slot.toml"),
        "[proposal]",
        "[proposal]\nvariant = \"hero-256x256\"",
    );
    assert!(parse_build_override(&text).is_err());

    // Two things are inside the approval binding, and each of them alone is enough.
    let profile = asset_profile();
    let (approved, context) = variant_case(
        SettingChoice::RasterCodec(RasterCodecChoice {
            codec: "bc7".to_string(),
        }),
        Some(banner(512, 128)),
        Some(banner_facts(512, 128)),
        &profile,
    );
    assert_eq!(
        resolve_in(&approved, &profile, context.clone()),
        OverrideOutcome::ApprovedMapping
    );

    // FIRST, the record's stated expectation. Editing it to describe another form does not reach
    // that form's constraints - the source has not moved - and it does not carry the approval.
    let moved = BuildOverride {
        proposal: Proposal {
            variant_selector: Some(banner(512, 256)),
            ..approved.proposal.clone()
        },
        ..approved.clone()
    };
    assert_ne!(
        moved.approval.as_ref().expect("carried over").fingerprint,
        override_fingerprint(
            &moved,
            &profile_content_digest(&profile),
            context.observation.as_ref()
        ),
        "the record's expectation is compared against the source, so it must be inside the binding"
    );
    assert!(matches!(
        resolve_in(&moved, &profile, context.clone()),
        OverrideOutcome::SelectorExpectationMismatch(_)
    ));

    // SECOND, the observed facts. The record is untouched here; the SOURCE became the other form,
    // which is a different variant and therefore a different set of constraints. C0-AS3-R1 added
    // this half: under AS3 the binding covered only what the record claimed.
    let elsewhere = current_observing(&approved, banner_facts(512, 256));
    assert_ne!(
        approved.approval.as_ref().expect("approved").fingerprint,
        override_fingerprint(
            &approved,
            &profile_content_digest(&profile),
            elsewhere.observation.as_ref()
        ),
        "the facts the variant was selected from decide what the proposal was checked against"
    );
    assert_ne!(
        resolve_in(&approved, &profile, elsewhere),
        OverrideOutcome::ApprovedMapping
    );
}

#[test]
fn a_scope_force_or_selector_change_in_the_profile_invalidates_an_approval() {
    // `/3` added three things a profile can change without changing any VALUE: how far a fact's
    // evidence reaches, what it asserts about other values, and which variant a query lands on.
    // Each of them changes what a Build may do, so each must move the profile digest.
    let profile = asset_profile();
    let baseline = profile_content_digest(&profile);

    let mut scope_changed = profile.clone();
    scope_changed
        .targets
        .iter_mut()
        .find(|t| t.target == "fixtureone-synthetic")
        .expect("declared")
        .raster
        .scope = Some("inspected-corpus-invariant".to_string());
    assert_ne!(profile_content_digest(&scope_changed), baseline, "scope");

    let mut force_changed = profile.clone();
    force_changed
        .targets
        .iter_mut()
        .find(|t| t.target == "fixtureone-synthetic")
        .expect("declared")
        .raster
        .force = Some("observed-source-format".to_string());
    assert_ne!(profile_content_digest(&force_changed), baseline, "force");

    let mut corpus_changed = profile.clone();
    corpus_changed
        .targets
        .iter_mut()
        .find(|t| t.target == "fixturethree-synthetic")
        .expect("declared")
        .corpora[0]
        .attribution = "not-attributable".to_string();
    assert_ne!(
        profile_content_digest(&corpus_changed),
        baseline,
        "a corpus's attribution decides whether anything resting on it may be established at all"
    );

    let mut selector_changed = profile.clone();
    selector_changed
        .targets
        .iter_mut()
        .find(|t| t.target == "fixturethree-synthetic")
        .expect("declared")
        .variants[1]
        .selectors[1]
        .height = Some(64);
    assert_ne!(
        profile_content_digest(&selector_changed),
        baseline,
        "a selector edit moves which variant a Build lands on without changing a single value"
    );

    let mut property_changed = profile.clone();
    property_changed
        .targets
        .iter_mut()
        .find(|t| t.target == "fixturethree-synthetic")
        .expect("declared")
        .variants[1]
        .raster
        .properties
        .remove("color_space");
    assert_ne!(
        profile_content_digest(&property_changed),
        baseline,
        "which sub-properties are unknown decides which operations a partial block can support"
    );

    // The exclusions still hold: prose and the non-operative candidate channel decide nothing.
    let mut prose_only = profile.clone();
    {
        let target = prose_only
            .targets
            .iter_mut()
            .find(|t| t.target == "fixturethree-synthetic")
            .expect("declared");
        target.note = "reworded".to_string();
        target.variants[1].raster.note = "reworded".to_string();
        target.variants[1].tiling.candidate = None;
    }
    assert_eq!(
        profile_content_digest(&prose_only),
        baseline,
        "an editorial edit must not invalidate an approval that moved nothing about the target"
    );
}

#[test]
fn a_hand_entered_observation_is_kept_and_is_not_authority() {
    // C0-AS3-R2. R1 moved selection off what a record CLAIMS about its source and onto what was
    // read from it. `operator-entered` walks that back one step further along: a typed-in width and
    // a width read out of a header are both strings in the same struct, and only one is a reading.
    let profile = asset_profile();
    let bc7 = || {
        SettingChoice::RasterCodec(RasterCodecChoice {
            codec: "bc7".to_string(),
        })
    };

    // Start from a genuinely approved mapping: a real reading, a record that agrees with it, an
    // approval bound to both.
    let record = variant_record(bc7(), Some(banner(512, 128)), &profile);
    let read = current_observing_by(&record, banner_facts(512, 128), "header-read");
    let approved = reapproved_observing(record, &profile, read.observation.as_ref());
    assert_eq!(
        resolve_in(&approved, &profile, read),
        OverrideOutcome::ApprovedMapping
    );

    // Change ONE thing: how the facts were obtained. Same source, same digest, same facts, same
    // record, and an approval recomputed so it matches exactly - the method is inside the
    // fingerprint, so a stale approval would prove nothing about authority here.
    let typed = current_observing_by(&approved, banner_facts(512, 128), "operator-entered");
    let reapproved_typed = reapproved_observing(approved, &profile, typed.observation.as_ref());
    assert_eq!(
        reapproved_typed
            .approval
            .as_ref()
            .expect("approved")
            .fingerprint,
        override_fingerprint(
            &reapproved_typed,
            &profile_content_digest(&profile),
            typed.observation.as_ref()
        ),
        "the approval must be current, or this proves nothing"
    );
    match resolve_in(&reapproved_typed, &profile, typed) {
        OverrideOutcome::BlockedByUnverifiedObservation(detail) => {
            assert!(detail.contains("operator-entered"), "{detail}");
        }
        other => panic!("a hand-entered fact cannot select a variant: {other:?}"),
    }

    // Approval cannot elevate it, and that is structural rather than a policy check: the gate is at
    // step 3, so removing the approval entirely gives the same answer.
    let unapproved = BuildOverride {
        approval: None,
        ..reapproved_typed.clone()
    };
    let typed_again = current_observing_by(&unapproved, banner_facts(512, 128), "operator-entered");
    assert!(matches!(
        resolve_in(&unapproved, &profile, typed_again),
        OverrideOutcome::BlockedByUnverifiedObservation(_)
    ));

    // It is a rule about where the fact came from, NOT about who ran the tool. A person who opens a
    // file and has the parser read its header has produced a `header-read`, and it is operative.
    for method in OPERATIVE_OBSERVATION_METHOD {
        let record = variant_record(bc7(), Some(banner(512, 128)), &profile);
        let context = current_observing_by(&record, banner_facts(512, 128), method);
        let record = reapproved_observing(record, &profile, context.observation.as_ref());
        assert_eq!(
            resolve_in(&record, &profile, context),
            OverrideOutcome::ApprovedMapping,
            "{method} is a reading and must stay operative"
        );
    }

    // The vocabulary still ADMITS the hand-entered term: the observation is representable, keeps
    // its facts and keeps the source identity it claims to describe. Discarding it would lose the
    // record of what somebody believed; it simply decides nothing.
    assert!(OBSERVATION_METHOD.contains(&"operator-entered"));
    assert!(!OPERATIVE_OBSERVATION_METHOD.contains(&"operator-entered"));
    assert!(OPERATIVE_OBSERVATION_METHOD
        .iter()
        .all(|m| OBSERVATION_METHOD.contains(m)));
}

// ============================================ C0-CAN-EOL1: line-ending portability of the cases
//
// Git stores these fixtures with LF and expands them to CRLF in any checkout configured
// `core.autocrlf=true` — the Git for Windows default. The anchors above are written with `\n`, so
// without normalization they match nothing on such a checkout and every negative case silently
// degrades into a test of the unedited document. These cases pin the repair itself.

/// The same document in both representations must produce the same mutation and the same record.
#[test]
fn an_anchor_matches_in_both_line_ending_representations() {
    let lf_text = fixture("build-overrides/synthetic-approved-slot.toml");
    assert!(!lf_text.contains('\r'), "the normalized form carries no CR");
    let crlf_text = lf_text.replace('\n', "\r\n");
    assert_ne!(
        crlf_text, lf_text,
        "the two representations differ in bytes"
    );

    // The multi-line anchor that the seven original failures tripped on.
    const ANCHOR: &str = "setting = \"atlas-slot\"\n[proposal.proposed.operand]\nslot = 7";
    assert!(lf_text.contains(ANCHOR));
    assert!(
        !crlf_text.contains(ANCHOR),
        "if this ever holds, CRLF stopped being the hazard this case exists for"
    );
    assert!(
        lf(crlf_text.clone()).contains(ANCHOR),
        "normalizing restores it"
    );

    // Both forms parse, and to the same record.
    let from_lf = parse_build_override(&lf_text).expect("LF parses");
    let from_crlf = parse_build_override(&crlf_text).expect("CRLF parses");
    assert_eq!(
        from_lf, from_crlf,
        "line endings are not part of the record"
    );

    // And the mutation lands identically once each is normalized.
    let mutated_lf = replace_one(
        &lf(lf_text),
        ANCHOR,
        "setting = \"colour-grade\"\n[proposal.proposed.operand]\nslot = 7",
    );
    let mutated_crlf = replace_one(
        &lf(crlf_text),
        ANCHOR,
        "setting = \"colour-grade\"\n[proposal.proposed.operand]\nslot = 7",
    );
    assert_eq!(mutated_lf, mutated_crlf);
    assert!(parse_build_override(&mutated_lf).is_err(), "still refused");
}

/// A missing anchor is loud. This is the property `str::replace` does not have.
#[test]
#[should_panic(expected = "occurs 0 times, expected 1")]
fn a_mutation_anchor_that_matches_nothing_fails_explicitly() {
    let text = fixture("build-overrides/synthetic-approved-slot.toml");
    replace_one(&text, "setting = \"no-such-setting-exists\"", "irrelevant");
}

/// An ambiguous anchor is equally loud, so a fixture that grows a second copy cannot quietly widen
/// a mutation from one site to two.
#[test]
#[should_panic(expected = "expected 1")]
fn an_ambiguous_mutation_anchor_fails_explicitly() {
    let doubled = format!(
        "{}\n{}",
        fixture("build-overrides/synthetic-approved-slot.toml"),
        fixture("build-overrides/synthetic-approved-slot.toml")
    );
    replace_one(&doubled, "record_version = 1", "record_version = 2");
}

/// `replace_exactly` states the count it means, and honours a count other than one.
#[test]
fn a_stated_match_count_is_enforced_exactly() {
    let text = "a = 1\nb = 1\nc = 2\n";
    assert_eq!(
        replace_exactly(text, "= 1", "= 9", 2),
        "a = 9\nb = 9\nc = 2\n"
    );
    assert_eq!(
        replace_exactly(text, "= 2", "= 9", 1),
        "a = 1\nb = 1\nc = 9\n"
    );
}

/// Normalization collapses CRLF and nothing else. A lone CR is content and survives.
#[test]
fn normalization_touches_crlf_only_and_preserves_a_lone_cr() {
    assert_eq!(lf("a\r\nb\n".to_string()), "a\nb\n");
    assert_eq!(
        lf("a\rb".to_string()),
        "a\rb",
        "a lone CR is not a line ending"
    );
    assert_eq!(lf("a\r\rb".to_string()), "a\r\rb");
    assert_eq!(
        lf("a\r\r\nb".to_string()),
        "a\r\nb",
        "only the CRLF pair collapses"
    );
    assert_eq!(lf("plain".to_string()), "plain");

    // A CR inside a TOML string is content the parser keeps, and normalization must not eat it.
    let doc = "record_version = 1\r\n[source]\r\nasset_id = \"a\rb\"\r\n";
    assert!(
        lf(doc.to_string()).contains("\rb"),
        "the escaped CR is untouched"
    );
}

/// The parser is fed raw bytes elsewhere in this crate, so it must accept both representations
/// directly — normalization is a convenience for building cases, never a prerequisite for parsing.
#[test]
fn the_parser_accepts_raw_lf_and_raw_crlf_documents() {
    for name in [
        "synthetic-approved-slot",
        "synthetic-blocked-unknown",
        "synthetic-proposal-only",
        "synthetic-refused-codec",
    ] {
        let lf_text = fixture(&format!("build-overrides/{name}.toml"));
        let crlf_text = lf_text.replace('\n', "\r\n");
        let a = parse_build_override(&lf_text).unwrap_or_else(|e| panic!("{name} LF: {e}"));
        let b = parse_build_override(&crlf_text).unwrap_or_else(|e| panic!("{name} CRLF: {e}"));
        assert_eq!(a, b, "{name} parses identically in both representations");
    }

    // The profile these records resolve against, read raw rather than normalized.
    let raw = fixture_raw("synthetic-asset-profile.toml");
    parse_asset_profile(&raw).expect("the profile parses exactly as it sits on disk");
    let crlf = lf(raw.clone()).replace('\n', "\r\n");
    assert_eq!(
        parse_asset_profile(&crlf).expect("CRLF profile parses"),
        parse_asset_profile(&lf(raw)).expect("LF profile parses"),
    );
}

/// Digests and fingerprints are computed over the parsed record, never over file bytes, so the
/// representation a checkout produced cannot move them. This is the guarantee that made
/// normalizing the fixture readers safe.
#[test]
fn digests_and_fingerprints_are_independent_of_line_endings() {
    let profile_lf = parse_asset_profile(&fixture("synthetic-asset-profile.toml")).expect("parses");
    let profile_crlf =
        parse_asset_profile(&fixture("synthetic-asset-profile.toml").replace('\n', "\r\n"))
            .expect("parses");
    assert_eq!(
        profile_content_digest(&profile_lf),
        profile_content_digest(&profile_crlf),
        "the content digest is over the parsed profile, not its bytes"
    );

    let text = fixture("build-overrides/synthetic-approved-slot.toml");
    let record_lf = parse_build_override(&text).expect("parses");
    let record_crlf = parse_build_override(&text.replace('\n', "\r\n")).expect("parses");
    let digest = profile_content_digest(&profile_lf);
    assert_eq!(
        override_fingerprint(&record_lf, &digest, None),
        override_fingerprint(&record_crlf, &digest, None),
        "the approval fingerprint is over the parsed record, not its bytes"
    );
    // And it is still the fingerprint the committed fixture carries.
    assert_eq!(
        record_lf.approval.as_ref().expect("approved").fingerprint,
        override_fingerprint(&record_lf, &digest, None)
    );
}

/// `record_from` edits are document-wide, and that reach is deliberate: the case at
/// `synthetic-refused-codec` names its codec twice and means both. Pinning it to a single site
/// would quietly change what the surrounding cases assert, so the reach is pinned here instead.
#[test]
fn a_record_from_edit_reaches_every_occurrence_and_still_refuses_a_silent_no_op() {
    let text = fixture("build-overrides/synthetic-refused-codec.toml");
    assert_eq!(
        text.matches("bc7").count(),
        2,
        "the case depends on both sites"
    );
    assert_eq!(
        replace_present(&text, "bc7", "rgba8")
            .matches("rgba8")
            .count(),
        2
    );

    // Zero matches is still loud, which is the whole difference from `str::replace`.
    assert_eq!(replace_present("a = 1\n", "a", "b"), "b = 1\n");
}

/// The zero-match guard on the document-wide helper fires too.
#[test]
#[should_panic(expected = "occurs 0 times, expected at least one")]
fn a_document_wide_edit_that_matches_nothing_fails_explicitly() {
    replace_present("a = 1\n", "no-such-anchor", "irrelevant");
}
