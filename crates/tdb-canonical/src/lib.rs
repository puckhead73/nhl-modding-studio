//! Canonical Project schema, target-adapter contract and asset output profiles — the machine-
//! readable half of roadmap C0 item 7 (Q07a.1).
//!
//! Three document kinds, all versioned and all validated by this crate:
//!
//! | Kind | File | Schema |
//! |---|---|---|
//! | canonical Project schema | `defs/canonical/project-schema.toml` | `canonical-project/1` |
//! | target adapter | `defs/adapters/<target>.toml` | `target-adapter/2` |
//! | asset output profile | `defs/adapters/asset-profiles/<family>.toml` | `asset-output-profile/1` |
//!
//! The principles the validator enforces, each traceable to an accepted decision:
//!
//! | Principle | Rule here |
//! |---|---|
//! | Canonical data keeps the richest known semantics (Q07a.1) | a canonical property is defined once, target-neutrally; an adapter may map it or declare an omission, never redefine it |
//! | Every read transform needs a verified inverse write | [`Rule::InverseWriteMissing`], [`Rule::InverseUnverifiedButQualified`] |
//! | Unknown target defaults **block** rather than substitute | [`Rule::UnknownDefault`], [`Rule::PlaceholderAsValue`] |
//! | Omissions/loss require explicit approval | an omission carries a closed-vocabulary reason distinguishing *the target has no such field* from *we have not found it* |
//! | Reconciliation corrections affect target Build mappings, not canonical data (Q07a.3a) | nothing in an adapter can write back into the canonical schema; the types make it impossible |
//! | Schema/profile facts cite provenance and qualification | [`Rule::MissingProvenance`], [`Rule::UnknownQualification`] |
//! | Documentation never promotes support | [`Rule::AdapterClaimsGameVerified`] — an adapter file may **never** say `game-verified` |
//!
//! # The constraint layer (`target-adapter/2`)
//!
//! Q07a.1 requires the registry to record, per database entry: table and field identity, semantic
//! canonical property, type, endianness, bit/byte offset and width, signedness, range, enum
//! vocabulary **and stored code**, scale/offset/index origin, null sentinel, string encoding/pool
//! behaviour, update/key/structural status, foreign-key relationships, companion/cardinality rules,
//! capacity and presence/applicability. `target-adapter/2` adds a structured, evidence-qualified
//! block for each — see [`Q07A1_REPRESENTATION`], which is asserted against the types by a test.
//!
//! Every block is a [`Constraint<T>`](model::Constraint) carrying one of three
//! [states](model::ConstraintState): **established** (a real fact, with confidence and provenance),
//! **unknown** (nothing establishes it — export blocks), or **not-applicable** (the target genuinely
//! has no such constraint, with a recorded reason). A missing block is `unknown`, so silence is
//! never read as a fact, and a placeholder string such as `"unknown"` in a value slot is refused
//! outright rather than being mistaken for one.
//!
//! **Representation is not population.** Supporting a constraint in the format says nothing about
//! how many of the 28 catalog units declare it. [`representation_completeness`] and
//! [`registry_population`] report those two things separately and must never be conflated.
//!
//! This crate is **specification and validation only**. It implements no Project UI, no converter
//! and no Build engine, and it reads no game data.

use std::collections::{BTreeMap, BTreeSet};

pub mod asset;
pub mod asset_resolution;
pub mod asset_reuse;
pub mod asset_v1;
pub mod asset_v2;
pub mod build_override;
pub mod model;
pub mod v1;

pub use asset::{
    asset_bearing_families, asset_family_coverage, asset_registry_population,
    asset_representation_completeness, block_states, family_category,
    operation_constraint_readiness, parse_asset_profile, select_variant, selected_variant,
    validate_asset_profile, AssetCandidate, AssetConstraint, AssetPopulation, AssetProfile,
    AssetVariant, AtlasValue, AudioValue, BlockState, CanonicalSource, CompressionValue,
    ContainerLayer, ContainerValue, Corpus, DimensionsValue, DiscoveryValue, FamilyCoverage,
    FamilyCoverageStatus, IdentityValue, InstallValue, MeshLayoutValue, MipmapValue,
    ObservedBucket, OperationDecl, OperationReadiness, PathFamily, PathRuleValue, PropertyEvidence,
    ProvenanceBridge, RasterValue, RigValue, RuntimeLookupValue, SelectorClause, TargetProfile,
    TilingValue, UvValue, VariantQuery, VariantSelection, VertexAttribute, ASSET_BEARING_CATEGORY,
    ASSET_BLOCK, ASSET_FAMILY, ASSET_OPERATION, ATLAS_SHARING, BRIDGE_METHOD, BUCKET_RESOLUTION,
    BUILD_IDENTIFICATION, CONSTRAINT_FORCE, CORPUS_ATTRIBUTION, CORPUS_KIND, EVIDENCE_SCOPE,
    FAMILY_CATEGORY, OPERATIVE_SCOPE, OPTIONAL_BLOCK_PROPERTY, PATCH_PRECEDENCE, PLACEMENT_BLOCK,
    Q07A1_ASSET_REPRESENTATION, REQUIRABLE_BLOCK_PROPERTY, RUNTIME_SOURCE, SCOPE_FORCE,
    SELECTOR_DIMENSION, UNATTRIBUTABLE_CORPUS_KIND, VARIANT_BLOCK, VARIANT_OPTIONAL_BLOCK,
};
pub use asset_reuse::{
    resolve_target, reuse_source, shared_declarations, ResolvedTarget, ReuseError,
    DESTINATION_BLOCKS, LOCAL_EVIDENCE_TABLES, SHARED_BLOCKS, SHARED_TABLES,
};
pub use asset_v1::{
    parse_asset_profile_v1, upgrade_asset_profile_v1, AssetProfileV1, AssetUpgrade,
    CanonicalSourceV1, TargetProfileV1,
};
pub use asset_v2::{
    parse_asset_profile_v2, upgrade_asset_profile_v2, AssetProfileV2, AssetUpgradeV2,
    TargetProfileV2,
};
pub use build_override::{
    context_drift, observation_drift, override_fingerprint, parse_build_override,
    profile_content_digest, resolve_build_override, validate_build_override, Approval,
    AtlasSlotChoice, AudioSampleRateChoice, BuildContext, BuildOverride, CompressionPayloadChoice,
    DimensionsChoice, MappingIdentity, MipmapCountChoice, ObservedFacts, OverrideOutcome,
    PriorValue, ProfileIdentity, Proposal, RasterCodecChoice, SettingChoice, SourceIdentity,
    SourceObservation, TilingChoice, ValidatedOverride, ValidatedProfile, VariantSelector,
    APPROVAL_FINGERPRINT_VERSION, HASH_ALGORITHM, OBSERVATION_METHOD, OPERATIVE_OBSERVATION_METHOD,
    OVERRIDE_SETTING, SETTING_RESOLUTION,
};
pub use model::{
    Adapter, ApplicabilityValue, Candidate, CapacityDecl, CapacityValue, CompanionEntry,
    CompanionRuleDecl, CompanionValue, Constraint, ConstraintState, Coverage, Domain, Entity,
    EnumEntry, EnumerationValue, FieldComponent, ForeignKeyValue, Mapping, NullSentinelValue,
    Omission, PositionDiscriminatorValue, ProjectSchema, Property, StatusValue, StorageValue,
    StringStorageValue, TableDecl, TransformValue,
};
pub use v1::{parse_adapter_v1, upgrade_v1, AdapterV1, Upgrade, UpgradeNote};

// =================================================================== schemas

/// The canonical Project schema id this crate validates.
pub const PROJECT_SCHEMA: &str = "canonical-project/1";
/// The target-adapter schema id this crate validates natively.
pub const ADAPTER_SCHEMA: &str = "target-adapter/2";
/// The previous adapter schema. Accepted only through [`upgrade_v1`], never validated in place.
pub const ADAPTER_SCHEMA_PREVIOUS: &str = "target-adapter/1";
/// The asset output profile schema id this crate validates natively.
///
/// `/5` (C0-AS5) adds one optional key: `reuses = "<target id>"` on a `[[target_profile]]`, which
/// inherits another target's shared definitions in the same document (see [`asset_reuse`]).
pub const ASSET_PROFILE_SCHEMA: &str = "asset-output-profile/5";
/// The `/4` schema: composite identities, resolution chains, the `composite-id` token. Validated in
/// place under its own rules, plus one: it may not use `reuses` while claiming `/4`.
pub const ASSET_PROFILE_SCHEMA_V4: &str = "asset-output-profile/4";
/// The `/3` schema. `/4` and `/5` add only optional structures, so a `/3` document is validated
/// **in place under exactly its own rules**, plus one: it may not use a `/4` or `/5` construct while
/// claiming `/3`. Nothing in a `/3` document is reinterpreted; see [`ASSET_PROFILE_SCHEMAS_NATIVE`].
pub const ASSET_PROFILE_SCHEMA_V3: &str = "asset-output-profile/3";
/// The schemas validated in place. Order is newest first.
pub const ASSET_PROFILE_SCHEMAS_NATIVE: &[&str] = &[
    ASSET_PROFILE_SCHEMA,
    ASSET_PROFILE_SCHEMA_V4,
    ASSET_PROFILE_SCHEMA_V3,
];
/// The schema of a target Build mapping override record (Q07a.3a).
pub const BUILD_OVERRIDE_SCHEMA: &str = "build-mapping-override/1";
/// The previous asset profile schema. Accepted only through
/// [`upgrade_asset_profile_v2`](asset_v2::upgrade_asset_profile_v2), never validated in place.
pub const ASSET_PROFILE_SCHEMA_PREVIOUS: &str = "asset-output-profile/2";
/// The original asset profile schema. Accepted only through
/// [`upgrade_asset_profile_v1`](asset_v1::upgrade_asset_profile_v1), never validated in place.
pub const ASSET_PROFILE_SCHEMA_LEGACY: &str = "asset-output-profile/1";

// ============================================================= vocabularies

/// Closed vocabulary: how well a fact is known.
pub const CONFIDENCE: &[&str] = &["measured", "derived", "asserted", "unknown"];

/// The confidences an **operative** constraint may carry.
///
/// `asserted` is deliberately absent. An asserted claim — a documented expectation, an inference
/// from a field name, a fact confirmed on a neighbouring title — is a *candidate*, not a constraint
/// something may act on. It keeps its citation in `[<block>.candidate]` and drives nothing.
/// See [`Rule::AssertedAsOperative`].
pub const OPERATIVE_CONFIDENCE: &[&str] = &["measured", "derived"];

/// The confidences a **candidate** may carry. `unknown` is absent: a candidate with no confidence
/// at all is not a proposal, it is silence, and silence is spelled by omitting the block.
pub const CANDIDATE_CONFIDENCE: &[&str] = &["measured", "derived", "asserted"];

/// The `read_transform` / `write_transform` spelling for a mapping whose transform is **not
/// established**.
///
/// This is distinct from `"unknown"` on the write side, which means *this read has no declared
/// inverse* and is refused. `"unresolved"` claims nothing in either direction: the field identity
/// may be measured while the encoding it carries is still undecided.
pub const UNRESOLVED_TRANSFORM: &str = "unresolved";

/// Closed vocabulary: the evidence levels a schema file may declare.
///
/// `game-verified` is **absent on purpose**: no schema or profile file can promote support
/// (CLAUDE.md evidence rule — "documentation never promotes support").
pub const QUALIFICATION: &[&str] = &[
    "implemented",
    "automated-tested",
    "blocked",
    "missing-fixture",
    "unknown",
];

/// Closed vocabulary: physical storage class of a target field.
pub const STORAGE_KIND: &[&str] = &[
    "fixed-bit-uint",
    "fixed-bit-signed",
    "inline-string",
    "varchar-pool",
    "float",
    "binary",
];

/// Closed vocabulary: how a stored integer's sign / "none" bit-packing works.
pub const SIGNEDNESS: &[&str] = &[
    "unsigned",
    "twos-complement",
    "neg1-sentinel",
    "offset-binary",
    "not-applicable",
];

/// Closed vocabulary: byte order.
pub const ENDIAN: &[&str] = &["big", "little"];

/// Closed vocabulary: the scale / offset / index-origin semantics of a transform.
pub const TRANSFORM_KIND: &[&str] = &[
    "identity",
    "index-origin",
    "linear-offset",
    "enum-code",
    "inline-string",
    "opaque",
];

/// Closed vocabulary: how a target encodes "absent".
///
/// **Units of `raw` (C0-REG1a-R2).** `NullSentinelValue::raw` is *the stored code that means
/// absent*:
///
/// * `literal-code` — `raw` is the **unsigned stored bit pattern** of the field, `0 ..= 2^width - 1`,
///   before any storage decoding. A field whose absence is the all-zero pattern declares `raw = 0`
///   even when its storage regime would decode that pattern to something else (an offset-binary
///   field decodes pattern 0 to `range_low`).
/// * `raw-all-ones` — the pattern is every bit set, and by the convention this vocabulary has always
///   used it is **written as `raw = -1`**: the value the `neg1-sentinel` regime decodes that pattern
///   to. That notation is kept as-is for every existing adapter; it is the one encoding whose `raw`
///   is not a bit pattern, and the validator refuses any other `raw` under it.
/// * `none` — no sentinel; `raw` is absent.
///
/// **Processing order.** A reader that honours a sentinel proceeds in this order, and the inverse
/// runs it backwards: (1) inspect the stored bits and compare them with the sentinel pattern;
/// (2) if they match, the canonical value is the sentinel's `canonical_meaning` and nothing further
/// is decoded; (3) otherwise apply the storage regime's decoding (`unsigned`, `offset-binary`,
/// `twos-complement`, `neg1-sentinel`) to obtain the decoded value; (4) only then apply the
/// transform / index mapping (`index-origin`, `linear-offset`, `enum-code`) and, for a reference
/// property, the parent-key mapping — which for a reference exists only when `[mapping.foreign_key]`
/// is established. The sentinel is therefore matched on bits, never on a decoded value, which is why
/// `literal-code` is stated in bit-pattern units. This crate ships the vocabulary and the validator;
/// it ships no executor of this order — `legacy_xbox360_tests` exercises it with a test helper.
pub const SENTINEL_ENCODING: &[&str] = &["none", "raw-all-ones", "literal-code"];

/// Closed vocabulary: what the canonical model reads a sentinel as.
pub const SENTINEL_MEANING: &[&str] = &["absent", "unset"];

/// Closed vocabulary: where a string's bytes live.
pub const STRING_STORAGE: &[&str] = &["inline-fixed", "varchar-pool"];

/// Closed vocabulary: how a string's bytes are interpreted.
pub const STRING_ENCODING: &[&str] = &["byte-transparent", "utf8"];

/// Closed vocabulary: what fills a fixed-width string field past its content.
pub const STRING_PADDING: &[&str] = &["nul-fill", "none"];

/// Closed vocabulary: what happens to a value longer than the field's budget.
pub const STRING_OVERFLOW: &[&str] = &["reject", "truncate"];

/// Closed vocabulary: whether EA metadata declares a foreign key or this repository inferred it.
pub const FK_DECLARATION: &[&str] = &["declared-in-meta", "implicit-by-name"];

/// Closed vocabulary: how a mapping's applicability is decided.
pub const APPLICABILITY_KIND: &[&str] = &["all-records", "canonical-predicate", "target-predicate"];

/// Closed vocabulary: which file a capacity ceiling was read from.
pub const CAPACITY_SOURCE: &[&str] = &["main-db-max-records", "roster-save-max-records"];

/// Closed vocabulary: companion-record cardinality.
pub const CARDINALITY: &[&str] = &["one-to-one", "zero-or-one", "one-to-many"];

/// Closed vocabulary: when a companion record is required.
pub const APPLIES_WHEN: &[&str] = &["always", "position-skater", "position-goalie"];

/// Closed vocabulary: why a canonical property is not mapped.
///
/// `target-absent` is a **proof of absence** and needs provenance; `unknown-field-identity` is an
/// admission of ignorance. Collapsing the two would let an unknown masquerade as a decision.
pub const OMISSION_REASON: &[&str] = &[
    "target-absent",
    "unknown-field-identity",
    "unresolved-semantics",
];

/// Strings that are placeholders, never values. Refused anywhere an identity is expected.
pub const PLACEHOLDERS: &[&str] = &["", "unknown", "tbd", "todo", "n/a", "na", "?", "-", "xxx"];

pub(crate) fn is_placeholder(text: &str) -> bool {
    let lowered = text.trim().to_ascii_lowercase();
    PLACEHOLDERS.contains(&lowered.as_str())
}

/// A transform that changes the value, and therefore needs a proven inverse to be written back.
fn is_identity(transform: &str) -> bool {
    transform == "identity"
}

// ================================================= the Q07a.1 representation map

/// Every database constraint Q07a.1 requires, mapped to where `target-adapter/2` represents it.
///
/// This is the format's own completeness statement, and a test asserts each entry actually parses
/// out of a document. It says nothing at all about how many targets *declare* these — see
/// [`registry_population`].
pub const Q07A1_REPRESENTATION: &[(&str, &str)] = &[
    ("table identity", "mapping.table (+ [[table]] declaration)"),
    (
        "field identity",
        "mapping.field, or [[mapping.component]] when one canonical value spans several fields",
    ),
    ("semantic canonical property", "mapping.canonical"),
    ("type", "[mapping.storage].value.kind"),
    ("endianness", "[mapping.storage].value.endian"),
    ("bit/byte offset", "[mapping.storage].value.bit_offset"),
    ("width", "[mapping.storage].value.bit_width"),
    ("signedness", "[mapping.storage].value.signedness"),
    (
        "range",
        "[mapping.storage].value.range_low/range_high (human range derived, not declared)",
    ),
    (
        "enum vocabulary",
        "[mapping.enumeration].value.entry.canonical",
    ),
    ("enum stored code", "[mapping.enumeration].value.entry.code"),
    (
        "scale/offset/index origin",
        "[mapping.transform].value.kind/offset/index_origin",
    ),
    ("null sentinel", "[mapping.null_sentinel]"),
    ("string encoding/pool behaviour", "[mapping.string_storage]"),
    ("update/key/structural status", "[mapping.status]"),
    ("foreign-key relationships", "[mapping.foreign_key]"),
    (
        "companion/cardinality rules",
        "[[companion_rule]] (+ [position_discriminator])",
    ),
    ("capacity", "[[capacity]]"),
    (
        "presence/applicability",
        "[mapping.applicability] and [[omission]].reason",
    ),
    (
        "verified inverse write transform",
        "mapping.write_transform + inverse_verified + automated_case",
    ),
];

// ====================================================================== rules

/// Every contract rule the validator enforces, named so a failure is diagnosable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    UnsupportedSchema,
    UnsupportedVersion,
    /// A `canonical` path that the project schema does not define.
    UnknownCanonicalProperty,
    /// The same canonical property mapped twice in one adapter.
    DuplicateMapping,
    /// The same canonical property both mapped and declared omitted.
    MappedAndOmitted,
    /// A non-identity read transform whose write transform is `unknown`.
    InverseWriteMissing,
    /// A mapping that claims `automated-tested` without `inverse_verified`.
    InverseUnverifiedButQualified,
    /// `automated-tested` with no named case.
    QualifiedWithoutCase,
    /// A `default` of `"unknown"` — an unknown default must block, not substitute.
    UnknownDefault,
    /// A fact asserted with no provenance while claiming to be known.
    MissingProvenance,
    UnknownConfidence,
    UnknownQualification,
    /// A schema file claiming game verification.
    AdapterClaimsGameVerified,
    /// A `target` that is not `<title>-<platform>`.
    MalformedTarget,
    /// A mapping with an unestablished table or field identity that nonetheless claims to qualify.
    UnknownIdentityButQualified,

    // ------------------------------------------------- the constraint layer
    /// A placeholder string (`"unknown"`, `"tbd"`, `""`, …) used where an identity is expected.
    PlaceholderAsValue,
    /// `state = "established"` with no `value` table.
    EstablishedWithoutValue,
    /// `state = "established"` with no confidence, or an `unknown` one.
    EstablishedWithoutConfidence,
    /// A `value` or `confidence` present under `unknown` / `not-applicable`.
    ValueWithoutEstablishedState,
    /// `state = "not-applicable"` with no reason recorded.
    NotApplicableWithoutReason,
    /// A closed-vocabulary field carrying a term outside its vocabulary.
    UnknownVocabularyTerm,
    /// A `table` reference with no `[[table]]` declaration in this adapter.
    UndeclaredTable,
    /// Two `[[table]]` entries with the same id.
    DuplicateTableDeclaration,
    /// A storage block whose range does not fit its width, or whose bounds are inverted.
    StorageBoundsInconsistent,
    /// Two enum entries sharing a stored code, or sharing a canonical value.
    EnumCodeConflict,
    /// An enum stored code outside the field's declared storage range.
    EnumCodeOutOfRange,
    /// An enum entry naming a value the canonical domain does not define.
    EnumValueNotInDomain,
    /// A constraint block that does not fit the canonical property's type.
    ConstraintKindMismatch,
    /// `[mapping.transform]` and the `read_transform` / `write_transform` names disagree.
    TransformDeclarationConflict,
    /// A null sentinel that contradicts the storage signedness or collides with an enum code.
    SentinelConflict,
    /// A string byte budget that contradicts the declared storage width.
    StringCapacityConflict,
    /// Two `[[capacity]]` entries for the same table.
    DuplicateCapacityDeclaration,
    /// A capacity ceiling below the population it must hold.
    CapacityBelowPopulation,
    /// Two `[[companion_rule]]` entries for the same primary table.
    DuplicateCompanionRule,
    /// One companion rule naming the same companion table twice.
    DuplicateCompanionTable,
    /// A `position-*` companion rule with no established `[position_discriminator]`.
    PositionDiscriminatorMissing,
    /// Skater and goalie position codes that overlap, or an empty code set.
    PositionCodeConflict,
    /// A `default` on a key or non-updatable field — Clear to Defaults could never write it.
    DefaultOnNonEditableField,
    /// An applicability block missing the operands its kind requires.
    ApplicabilityIncomplete,
    /// An omission whose reason claims proof of absence without citing it.
    OmissionWithoutProof,
    /// `field` and `[[mapping.component]]` used together, duplicate component roles, or a
    /// mapping-level storage block on a composite mapping.
    CompositeIdentityConflict,
    /// An `established` constraint carrying `confidence = "asserted"`. An asserted claim is a
    /// candidate, never an operative constraint.
    AssertedAsOperative,
    /// A `[<block>.candidate]` attached to a constraint that is already established or declared
    /// not-applicable — a proposal for something already decided.
    CandidateOnDecidedConstraint,
    /// A candidate with no value, no confidence or no provenance. A proposal with no citation is
    /// not a proposal.
    CandidateIncomplete,
    /// A `derived` constraint that does not record the premises it rests on. A confidence label and
    /// a citation string are not, by themselves, an argument.
    DerivationUnexplained,
    /// `read_transform` and `write_transform` disagree about being unresolved, or an unresolved
    /// transform is paired with a qualification, a verified inverse, a default or an established
    /// `[mapping.transform]`.
    UnresolvedTransformConflict,

    // ------------------------------------------------- the asset profile layer
    /// A profile registering itself against a family the QA matrix does not track.
    AssetFamilyUnknown,
    /// A profile registering itself against a tracked family that is a database, save or
    /// onboarding workflow - a category with no raster, mesh or audio to profile (C0-AS6).
    AssetFamilyNotAssetBearing,
    /// Two `[[target_profile]]` entries for the same target in one profile.
    DuplicateTargetProfile,
    /// Two `[[canonical_source]]` entries sharing a role.
    DuplicateSourceRole,
    /// An asset constraint whose numbers contradict each other or another established block —
    /// a zero extent, a cell outside its page, a mip count a surface cannot have, a vertex
    /// attribute past its stride, a palette larger than its rig.
    AssetBoundsInconsistent,
    /// An asset constraint missing the operands its own kind requires — an atlas with no cell, a
    /// fixed mip count with no count, a path template and a token list that disagree.
    AssetOperandsIncomplete,
    /// An established asset constraint that depends on another block which is **not** established:
    /// an archive install rule with no container, a slug path with no identity.
    AssetConstraintDependencyUnmet,
    /// Two `[[operation]]` entries for the same operation on one target profile.
    DuplicateOperationDeclaration,
    /// An established asset constraint that does not say how far its evidence reaches, or what it
    /// asserts about other values: a missing `scope` or `force`.
    AssetEvidenceScopeMissing,
    /// An established asset constraint whose evidence does not reach far enough for what it claims:
    /// a specimen observation promoted to a target-wide constraint, or a `target-required` force
    /// resting on nothing better than the bytes one copy carries.
    AssetEvidenceScopeInsufficient,
    /// A `[[corpus]]` that cannot support what cites it: no inspected files, no citation, an
    /// unidentified build with no stated limitation, or a duplicate id.
    AssetCorpusIncomplete,
    /// An established constraint, or an observed bucket, citing a corpus the target does not
    /// declare - or citing none at all.
    AssetCorpusUndeclared,
    /// A corpus claimed to be attributable to its target when it is a derived or extracted tree, or
    /// an established constraint resting on a corpus that is not attributable.
    AssetCorpusAttributionUnsupported,
    /// A `[[variant]]` that cannot be chosen: no selector clause, or a clause missing the operands
    /// its dimension takes.
    AssetVariantSelectorIncomplete,
    /// A `[[variant]]` whose selector contradicts itself, contradicts the variant's own established
    /// value, or is indistinguishable from another variant's.
    AssetVariantSelectorContradiction,
    /// A correlated block decided at target level on a target that also declares variants. The
    /// geometry belongs to one place or the other, never both.
    AssetVariantBlockConflict,
    /// A `[<block>.property.<name>]` table for a sub-property the block cannot leave unknown, for
    /// one that is actually known, or with no reason recorded.
    AssetPropertyEvidenceConflict,
    /// A path family that cannot resolve a path: a duplicate or undeclared id, an authoritative
    /// id-to-directory rule this schema cannot spell, or `[path_rule]` established through a family
    /// whose bucket resolution is still unknown.
    AssetPathFamilyUnresolved,
    /// A `[corpus.bridge]` that does not carry attribution: an unknown method, no citation, a
    /// bridge to itself, a bridge to another unattributable corpus, or one declared on a corpus
    /// that already claims attribution and therefore bridges nothing.
    AssetProvenanceBridgeInvalid,
    /// A composite identity that cannot construct an id: a placeholder with no part, a part the
    /// format never uses, a duplicate or malformed part name, an unknown rendering, a part on an
    /// undeclared table or field, or a `composite-id` token with no composite behind it.
    AssetCompositeInvalid,
    /// A resolution chain that cannot resolve safely: an undeclared family, an unknown purpose or
    /// ownership, a duplicate id, no step, a write chain with no owned step, a family that is
    /// shared in one chain and owned in another, two families with the same template, or an
    /// operation that writes through a read-only chain.
    AssetResolutionChainInvalid,
    /// `/5`: `reuses` names no target profile of this document.
    AssetReuseUnresolved,
    /// `/5`: `reuses` names the target itself.
    AssetReuseSelfReference,
    /// `/5`: the reused profile itself reuses another — references do not chain.
    AssetReuseChain,
    /// `/5`: a reusing target declares shared content of its own; nothing is merged.
    AssetReuseConflict,

    // ------------------------------------------- the Build mapping override layer
    /// An override record missing an identity, a revision, a digest or a citation it needs to be
    /// bound to anything: a zero revision, an empty mapping id, an unsourced proposal.
    OverrideRecordIncomplete,
    /// A prior value that is both absent and present, neither absent nor present, or that records
    /// a different setting than the one the proposal changes.
    OverridePriorConflict,
    /// An approval that does not bind to the exact inputs it was granted against — a missing
    /// digest, a zero revision, an unparseable date.
    OverrideApprovalUnbound,
}

impl Rule {
    pub fn as_str(&self) -> &'static str {
        match self {
            Rule::UnsupportedSchema => "unsupported-schema",
            Rule::UnsupportedVersion => "unsupported-version",
            Rule::UnknownCanonicalProperty => "unknown-canonical-property",
            Rule::DuplicateMapping => "duplicate-mapping",
            Rule::MappedAndOmitted => "mapped-and-omitted",
            Rule::InverseWriteMissing => "inverse-write-missing",
            Rule::InverseUnverifiedButQualified => "inverse-unverified-but-qualified",
            Rule::QualifiedWithoutCase => "qualified-without-case",
            Rule::UnknownDefault => "unknown-default",
            Rule::MissingProvenance => "missing-provenance",
            Rule::UnknownConfidence => "unknown-confidence",
            Rule::UnknownQualification => "unknown-qualification",
            Rule::AdapterClaimsGameVerified => "adapter-claims-game-verified",
            Rule::MalformedTarget => "malformed-target",
            Rule::UnknownIdentityButQualified => "unknown-identity-but-qualified",
            Rule::PlaceholderAsValue => "placeholder-as-value",
            Rule::EstablishedWithoutValue => "established-without-value",
            Rule::EstablishedWithoutConfidence => "established-without-confidence",
            Rule::ValueWithoutEstablishedState => "value-without-established-state",
            Rule::NotApplicableWithoutReason => "not-applicable-without-reason",
            Rule::UnknownVocabularyTerm => "unknown-vocabulary-term",
            Rule::UndeclaredTable => "undeclared-table",
            Rule::DuplicateTableDeclaration => "duplicate-table-declaration",
            Rule::StorageBoundsInconsistent => "storage-bounds-inconsistent",
            Rule::EnumCodeConflict => "enum-code-conflict",
            Rule::EnumCodeOutOfRange => "enum-code-out-of-range",
            Rule::EnumValueNotInDomain => "enum-value-not-in-domain",
            Rule::ConstraintKindMismatch => "constraint-kind-mismatch",
            Rule::TransformDeclarationConflict => "transform-declaration-conflict",
            Rule::SentinelConflict => "sentinel-conflict",
            Rule::StringCapacityConflict => "string-capacity-conflict",
            Rule::DuplicateCapacityDeclaration => "duplicate-capacity-declaration",
            Rule::CapacityBelowPopulation => "capacity-below-population",
            Rule::DuplicateCompanionRule => "duplicate-companion-rule",
            Rule::DuplicateCompanionTable => "duplicate-companion-table",
            Rule::PositionDiscriminatorMissing => "position-discriminator-missing",
            Rule::PositionCodeConflict => "position-code-conflict",
            Rule::DefaultOnNonEditableField => "default-on-non-editable-field",
            Rule::ApplicabilityIncomplete => "applicability-incomplete",
            Rule::OmissionWithoutProof => "omission-without-proof",
            Rule::CompositeIdentityConflict => "composite-identity-conflict",
            Rule::AssertedAsOperative => "asserted-as-operative",
            Rule::CandidateOnDecidedConstraint => "candidate-on-decided-constraint",
            Rule::CandidateIncomplete => "candidate-incomplete",
            Rule::DerivationUnexplained => "derivation-unexplained",
            Rule::UnresolvedTransformConflict => "unresolved-transform-conflict",
            Rule::AssetFamilyUnknown => "unknown-asset-family",
            Rule::AssetFamilyNotAssetBearing => "asset-family-not-asset-bearing",
            Rule::DuplicateTargetProfile => "duplicate-target-profile",
            Rule::DuplicateSourceRole => "duplicate-source-role",
            Rule::AssetBoundsInconsistent => "asset-bounds-inconsistent",
            Rule::AssetOperandsIncomplete => "asset-operands-incomplete",
            Rule::AssetConstraintDependencyUnmet => "asset-constraint-dependency-unmet",
            Rule::DuplicateOperationDeclaration => "duplicate-operation-declaration",
            Rule::AssetEvidenceScopeMissing => "asset-evidence-scope-missing",
            Rule::AssetEvidenceScopeInsufficient => "asset-evidence-scope-insufficient",
            Rule::AssetCorpusIncomplete => "asset-corpus-incomplete",
            Rule::AssetCorpusUndeclared => "asset-corpus-undeclared",
            Rule::AssetCorpusAttributionUnsupported => "asset-corpus-attribution-unsupported",
            Rule::AssetVariantSelectorIncomplete => "asset-variant-selector-incomplete",
            Rule::AssetVariantSelectorContradiction => "asset-variant-selector-contradiction",
            Rule::AssetVariantBlockConflict => "asset-variant-block-conflict",
            Rule::AssetPropertyEvidenceConflict => "asset-property-evidence-conflict",
            Rule::AssetPathFamilyUnresolved => "asset-path-family-unresolved",
            Rule::AssetProvenanceBridgeInvalid => "asset-provenance-bridge-invalid",
            Rule::AssetCompositeInvalid => "asset-composite-invalid",
            Rule::AssetResolutionChainInvalid => "asset-resolution-chain-invalid",
            Rule::AssetReuseUnresolved => "asset-reuse-unresolved",
            Rule::AssetReuseSelfReference => "asset-reuse-self-reference",
            Rule::AssetReuseChain => "asset-reuse-chain",
            Rule::AssetReuseConflict => "asset-reuse-conflict",
            Rule::OverrideRecordIncomplete => "override-record-incomplete",
            Rule::OverridePriorConflict => "override-prior-conflict",
            Rule::OverrideApprovalUnbound => "override-approval-unbound",
        }
    }
}

/// One contract violation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub rule: Rule,
    pub subject: String,
    pub detail: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [{}]: {}",
            self.rule.as_str(),
            self.subject,
            self.detail
        )
    }
}

pub(crate) fn finding(
    rule: Rule,
    subject: impl Into<String>,
    detail: impl Into<String>,
) -> Finding {
    Finding {
        rule,
        subject: subject.into(),
        detail: detail.into(),
    }
}

// ==================================================================== parsing

/// Parse a canonical Project schema.
pub fn parse_project_schema(text: &str) -> Result<ProjectSchema, toml::de::Error> {
    toml::from_str(text)
}

/// Parse a `target-adapter/2` document.
///
/// Unknown keys are a **parse error**: a misspelled constraint field is refused rather than being
/// dropped on the floor.
pub fn parse_adapter(text: &str) -> Result<Adapter, toml::de::Error> {
    toml::from_str(text)
}

// ============================================== envelope + vocabulary helpers

pub(crate) fn check_vocab(
    out: &mut Vec<Finding>,
    subject: &str,
    field: &str,
    term: &str,
    allowed: &[&str],
) {
    if !allowed.contains(&term) {
        out.push(finding(
            Rule::UnknownVocabularyTerm,
            subject,
            format!("{field} = {term:?} is outside {allowed:?}"),
        ));
    }
}

pub(crate) fn check_identity(out: &mut Vec<Finding>, subject: &str, field: &str, value: &str) {
    if is_placeholder(value) {
        out.push(finding(
            Rule::PlaceholderAsValue,
            subject,
            format!("{field} = {value:?} is a placeholder; represent an unestablished identity by omitting the key, not by naming it"),
        ));
    }
}

/// A candidate is legal only under `unknown`, and must actually propose something with a citation.
fn check_candidate<T>(out: &mut Vec<Finding>, subject: &str, block: &str, c: &Constraint<T>) {
    let Some(candidate) = &c.candidate else {
        return;
    };
    if c.state != ConstraintState::Unknown {
        out.push(finding(
            Rule::CandidateOnDecidedConstraint,
            subject,
            format!(
                "[{block}.candidate] proposes a value while [{block}] is already {:?}",
                c.state.as_str()
            ),
        ));
    }
    if candidate.value.is_none() {
        out.push(finding(
            Rule::CandidateIncomplete,
            subject,
            format!("[{block}.candidate] declares no value table"),
        ));
    }
    if candidate.provenance.is_empty() {
        out.push(finding(
            Rule::CandidateIncomplete,
            subject,
            format!("[{block}.candidate] must cite where the proposal comes from"),
        ));
    }
    match candidate.confidence.as_deref() {
        None => out.push(finding(
            Rule::CandidateIncomplete,
            subject,
            format!("[{block}.candidate] must state how well the proposal is known"),
        )),
        Some(other) => check_vocab(
            out,
            subject,
            &format!("[{block}.candidate].confidence"),
            other,
            CANDIDATE_CONFIDENCE,
        ),
    }
}

/// The state/confidence/provenance discipline every constraint block shares.
pub(crate) fn check_envelope<T>(
    out: &mut Vec<Finding>,
    subject: &str,
    block: &str,
    c: &Constraint<T>,
) {
    check_candidate(out, subject, block, c);
    match c.state {
        ConstraintState::Established => {
            if c.value.is_none() {
                out.push(finding(
                    Rule::EstablishedWithoutValue,
                    subject,
                    format!("[{block}] is established but declares no value table"),
                ));
            }
            match c.confidence.as_deref() {
                None | Some("unknown") => out.push(finding(
                    Rule::EstablishedWithoutConfidence,
                    subject,
                    format!("[{block}] is established, so it must state a confidence other than \"unknown\""),
                )),
                Some("asserted") => out.push(finding(
                    Rule::AssertedAsOperative,
                    subject,
                    format!(
                        "[{block}] is established at confidence \"asserted\". An asserted claim is not                          operative: move it to [{block}.candidate], which keeps the citation without                          letting anything act on the value"
                    ),
                )),
                Some("derived") if c.derivation.trim().is_empty() => out.push(finding(
                    Rule::DerivationUnexplained,
                    subject,
                    format!(
                        "[{block}] is derived but records no `derivation`: state the premises, why                          each applies to THIS target, and what the argument does not establish"
                    ),
                )),
                Some(other) => check_vocab(
                    out,
                    subject,
                    &format!("[{block}].confidence"),
                    other,
                    OPERATIVE_CONFIDENCE,
                ),
            }
            if c.provenance.is_empty() {
                out.push(finding(
                    Rule::MissingProvenance,
                    subject,
                    format!("[{block}] is established and must cite its evidence"),
                ));
            }
        }
        ConstraintState::Unknown | ConstraintState::NotApplicable => {
            if c.value.is_some() {
                out.push(finding(
                    Rule::ValueWithoutEstablishedState,
                    subject,
                    format!(
                        "[{block}] carries a value under state {:?}",
                        c.state.as_str()
                    ),
                ));
            }
            if c.confidence.is_some() {
                out.push(finding(
                    Rule::ValueWithoutEstablishedState,
                    subject,
                    format!(
                        "[{block}] carries a confidence under state {:?}",
                        c.state.as_str()
                    ),
                ));
            }
            if c.state == ConstraintState::NotApplicable && c.reason.trim().is_empty() {
                out.push(finding(
                    Rule::NotApplicableWithoutReason,
                    subject,
                    format!("[{block}] declares the constraint absent without saying how that was determined"),
                ));
            }
        }
    }
}

// ================================================================= derivations

/// The human-facing range a mapping implies. **Derived**, never declared, so a stated storage range
/// and a stated human range can never contradict each other.
///
/// This is a fact about a **numeric** encoding: the decoded storage range shifted by the transform.
/// A mapping that carries a `[mapping.foreign_key]` block — established or proposed — encodes a
/// **reference**, and a reference has no human range: its decoded numbers are parent-key indices,
/// not values a person reads, and the set of *valid* references is the parent table's key set, which
/// this helper cannot see and must not be mistaken for. Such mappings yield `None` (C0-REG1a-R2).
pub fn human_range(mapping: &Mapping) -> Option<[i64; 2]> {
    if mapping.foreign_key.established().is_some() || mapping.foreign_key.candidate.is_some() {
        return None;
    }
    let storage = mapping.storage.established()?;
    let transform = mapping.transform.established()?;
    let shift = match transform.kind.as_str() {
        "identity" | "enum-code" => 0,
        "linear-offset" => transform.offset?,
        "index-origin" => 1 - transform.index_origin?,
        _ => return None,
    };
    Some([storage.range_low + shift, storage.range_high + shift])
}

/// Whether the field is editable through the validated scalar seam.
///
/// `updatable && !key` — the rule the scalar mutation audit proved and the presentation layer
/// already applies (`FieldView.editable`).
pub fn scalar_editable(mapping: &Mapping) -> Option<bool> {
    let status = mapping.status.established()?;
    Some(status.updatable && !status.key)
}

// =========================================================== schema validation

/// Validate the canonical schema itself.
pub fn validate_project_schema(schema: &ProjectSchema) -> Vec<Finding> {
    let mut out = Vec::new();
    if schema.schema != PROJECT_SCHEMA {
        out.push(finding(
            Rule::UnsupportedSchema,
            &schema.schema,
            format!("expected {PROJECT_SCHEMA}"),
        ));
    }
    let domains: BTreeSet<&str> = schema.domains.iter().map(|d| d.id.as_str()).collect();
    let entities: BTreeSet<&str> = schema.entities.iter().map(|e| e.id.as_str()).collect();
    for entity in &schema.entities {
        for property in &entity.properties {
            let subject = format!("{}.{}", entity.id, property.id);
            if let Some(domain) = &property.domain {
                if !domains.contains(domain.as_str()) {
                    out.push(finding(
                        Rule::UnknownCanonicalProperty,
                        &subject,
                        format!("unknown domain {domain:?}"),
                    ));
                }
            }
            if let Some(target) = &property.references {
                if !entities.contains(target.as_str()) {
                    out.push(finding(
                        Rule::UnknownCanonicalProperty,
                        &subject,
                        format!("references unknown entity {target:?}"),
                    ));
                }
            }
        }
    }
    out.sort();
    out
}

// ========================================================== adapter validation

/// Validate one adapter against the canonical schema and the contract rules.
pub fn validate_adapter(adapter: &Adapter, schema: &ProjectSchema) -> Vec<Finding> {
    let mut out = Vec::new();
    validate_adapter_header(adapter, schema, &mut out);

    let declared_tables = validate_table_declarations(adapter, &mut out);
    let known = schema.property_paths();
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();

    for mapping in &adapter.mappings {
        let subject = format!("{}/{}", adapter.target, mapping.canonical);
        *seen.entry(mapping.canonical.as_str()).or_insert(0) += 1;
        validate_mapping(
            adapter,
            schema,
            &known,
            &declared_tables,
            mapping,
            &subject,
            &mut out,
        );
    }

    for (canonical, count) in &seen {
        if *count > 1 {
            out.push(finding(
                Rule::DuplicateMapping,
                format!("{}/{canonical}", adapter.target),
                format!("mapped {count} times"),
            ));
        }
    }

    for omission in &adapter.omissions {
        let subject = format!("{}/{}", adapter.target, omission.canonical);
        if !known.contains(&omission.canonical) {
            out.push(finding(
                Rule::UnknownCanonicalProperty,
                &subject,
                "omits a property the schema does not define",
            ));
        }
        if seen.contains_key(omission.canonical.as_str()) {
            out.push(finding(
                Rule::MappedAndOmitted,
                &subject,
                "both mapped and declared omitted",
            ));
        }
        check_vocab(
            &mut out,
            &subject,
            "omission.reason",
            &omission.reason,
            OMISSION_REASON,
        );
        if omission.reason == "target-absent" && omission.provenance.is_empty() {
            out.push(finding(
                Rule::OmissionWithoutProof,
                &subject,
                "`target-absent` asserts the target has no such field, which is a finding and must cite how it was established",
            ));
        }
    }

    validate_capacities(adapter, &declared_tables, &mut out);
    validate_companions(adapter, &declared_tables, &mut out);

    out.sort();
    out.dedup();
    out
}

fn validate_adapter_header(adapter: &Adapter, schema: &ProjectSchema, out: &mut Vec<Finding>) {
    if adapter.schema != ADAPTER_SCHEMA {
        let detail = if adapter.schema == ADAPTER_SCHEMA_PREVIOUS {
            format!(
                "{ADAPTER_SCHEMA_PREVIOUS} documents are not validated in place; run the tested \
                 transition `upgrade_v1` first"
            )
        } else {
            format!("expected {ADAPTER_SCHEMA}")
        };
        out.push(finding(Rule::UnsupportedSchema, &adapter.target, detail));
    }
    if adapter.canonical_schema_version != schema.version {
        out.push(finding(
            Rule::UnsupportedVersion,
            &adapter.target,
            format!(
                "adapter was written against canonical schema v{}, the schema is v{}",
                adapter.canonical_schema_version, schema.version
            ),
        ));
    }
    if adapter
        .target
        .split_once('-')
        .is_none_or(|(t, p)| t.is_empty() || p.is_empty())
    {
        out.push(finding(
            Rule::MalformedTarget,
            &adapter.target,
            "expected <title>-<platform>",
        ));
    }
    if !adapter.endian.is_empty() {
        check_vocab(out, &adapter.target, "endian", &adapter.endian, ENDIAN);
    }
    if let Some(discriminator) = &adapter.position_discriminator {
        let subject = format!("{}/[position_discriminator]", adapter.target);
        check_envelope(out, &subject, "position_discriminator", discriminator);
        if let Some(value) = discriminator.established() {
            check_identity(out, &subject, "table", &value.table);
            check_identity(out, &subject, "field", &value.field);
            if value.skater_codes.is_empty() || value.goalie_codes.is_empty() {
                out.push(finding(
                    Rule::PositionCodeConflict,
                    &subject,
                    "both skater_codes and goalie_codes must be non-empty",
                ));
            }
            let skaters: BTreeSet<i64> = value.skater_codes.iter().copied().collect();
            let overlap: Vec<i64> = value
                .goalie_codes
                .iter()
                .copied()
                .filter(|c| skaters.contains(c))
                .collect();
            if !overlap.is_empty() {
                out.push(finding(
                    Rule::PositionCodeConflict,
                    &subject,
                    format!("codes {overlap:?} are declared both skater and goalie"),
                ));
            }
        }
    }
}

fn validate_table_declarations(adapter: &Adapter, out: &mut Vec<Finding>) -> BTreeSet<String> {
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for table in &adapter.tables {
        let subject = format!("{}/table:{}", adapter.target, table.id);
        check_identity(out, &subject, "id", &table.id);
        check_identity(out, &subject, "name", &table.name);
        if table.provenance.is_empty() {
            out.push(finding(
                Rule::MissingProvenance,
                &subject,
                "a table declaration is a fact about the target and must cite its evidence",
            ));
        }
        if !declared.insert(table.id.clone()) {
            out.push(finding(
                Rule::DuplicateTableDeclaration,
                &subject,
                "declared more than once",
            ));
        }
    }
    declared
}

fn require_declared_table(
    out: &mut Vec<Finding>,
    declared: &BTreeSet<String>,
    subject: &str,
    field: &str,
    table: &str,
) {
    if !is_placeholder(table) && !declared.contains(table) {
        out.push(finding(
            Rule::UndeclaredTable,
            subject,
            format!("{field} = {table:?} has no [[table]] declaration in this adapter"),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_mapping(
    adapter: &Adapter,
    schema: &ProjectSchema,
    known: &BTreeSet<String>,
    declared: &BTreeSet<String>,
    mapping: &Mapping,
    subject: &str,
    out: &mut Vec<Finding>,
) {
    // ---- identity and the pre-existing evidence discipline -----------------
    if !known.contains(&mapping.canonical) {
        out.push(finding(
            Rule::UnknownCanonicalProperty,
            subject,
            "not defined by the canonical schema",
        ));
    }
    if let Some(table) = &mapping.table {
        check_identity(out, subject, "table", table);
        require_declared_table(out, declared, subject, "table", table);
    }
    if let Some(field) = &mapping.field {
        check_identity(out, subject, "field", field);
    }
    validate_components(out, subject, adapter, mapping);
    check_vocab(out, subject, "confidence", &mapping.confidence, CONFIDENCE);
    if mapping.qualification == "game-verified" {
        out.push(finding(
            Rule::AdapterClaimsGameVerified,
            subject,
            "a schema file can never promote support; qualification comes from the evidence system",
        ));
    } else if !QUALIFICATION.contains(&mapping.qualification.as_str()) {
        out.push(finding(
            Rule::UnknownQualification,
            subject,
            format!("{:?}", mapping.qualification),
        ));
    }
    validate_unresolved_transform(out, subject, mapping);
    if mapping.write_transform == "unknown" && !is_identity(&mapping.read_transform) {
        out.push(finding(
            Rule::InverseWriteMissing,
            subject,
            format!(
                "read {:?} has no declared inverse write",
                mapping.read_transform
            ),
        ));
    }
    if mapping.qualification == "automated-tested" && !mapping.inverse_verified {
        out.push(finding(
            Rule::InverseUnverifiedButQualified,
            subject,
            "automated-tested requires inverse_verified = true",
        ));
    }
    if mapping.qualification == "automated-tested" && mapping.automated_case.is_none() {
        out.push(finding(
            Rule::QualifiedWithoutCase,
            subject,
            "no automated_case named",
        ));
    }
    if let Some(default) = mapping.default.as_deref() {
        if is_placeholder(default) {
            out.push(finding(
                Rule::UnknownDefault,
                subject,
                "an unknown target default must block export, never substitute a value",
            ));
        }
        if scalar_editable(mapping) == Some(false) {
            out.push(finding(
                Rule::DefaultOnNonEditableField,
                subject,
                "the field is a key or is not updatable, so Clear to Defaults could never write this",
            ));
        }
    }
    if mapping.confidence != "unknown" && mapping.provenance.is_empty() {
        out.push(finding(
            Rule::MissingProvenance,
            subject,
            "a known fact must cite its evidence",
        ));
    }
    if (mapping.table.is_none() || !field_identity_established(mapping))
        && mapping.qualification == "automated-tested"
    {
        out.push(finding(
            Rule::UnknownIdentityButQualified,
            subject,
            "the transform may be proven, but an unestablished table/field identity cannot qualify the mapping",
        ));
    }

    // ---- the constraint blocks --------------------------------------------
    check_envelope(out, subject, "mapping.storage", &mapping.storage);
    check_envelope(out, subject, "mapping.status", &mapping.status);
    check_envelope(out, subject, "mapping.transform", &mapping.transform);
    check_envelope(out, subject, "mapping.enumeration", &mapping.enumeration);
    check_envelope(
        out,
        subject,
        "mapping.null_sentinel",
        &mapping.null_sentinel,
    );
    check_envelope(
        out,
        subject,
        "mapping.string_storage",
        &mapping.string_storage,
    );
    check_envelope(out, subject, "mapping.foreign_key", &mapping.foreign_key);
    check_envelope(
        out,
        subject,
        "mapping.applicability",
        &mapping.applicability,
    );

    let property = schema.property(&mapping.canonical);

    if let Some(storage) = mapping.storage.established() {
        validate_storage(out, subject, adapter, storage);
    }
    if let Some(transform) = mapping.transform.established() {
        validate_transform(out, subject, mapping, transform);
    }
    if let Some(enumeration) = mapping.enumeration.established() {
        validate_enumeration(out, subject, schema, property, mapping, enumeration);
    }
    if let Some(sentinel) = mapping.null_sentinel.established() {
        validate_sentinel(out, subject, mapping, sentinel);
    }
    if let Some(string_storage) = mapping.string_storage.established() {
        validate_string_storage(out, subject, property, mapping, string_storage);
    }
    if let Some(fk) = mapping.foreign_key.established() {
        check_identity(
            out,
            subject,
            "[mapping.foreign_key].parent_table",
            &fk.parent_table,
        );
        check_identity(
            out,
            subject,
            "[mapping.foreign_key].parent_key_field",
            &fk.parent_key_field,
        );
        check_vocab(
            out,
            subject,
            "[mapping.foreign_key].declaration",
            &fk.declaration,
            FK_DECLARATION,
        );
        require_declared_table(
            out,
            declared,
            subject,
            "[mapping.foreign_key].parent_table",
            &fk.parent_table,
        );
        if property.is_some_and(|p| p.kind != "reference") {
            out.push(finding(
                Rule::ConstraintKindMismatch,
                subject,
                "a foreign key is declared on a canonical property that is not a reference",
            ));
        }
    }
    if let Some(applicability) = mapping.applicability.established() {
        validate_applicability(out, subject, known, declared, applicability);
    }
}

/// Whether a mapping declares its encoding **unresolved** rather than claiming one.
pub fn transform_unresolved(mapping: &Mapping) -> bool {
    mapping.read_transform == UNRESOLVED_TRANSFORM
        || mapping.write_transform == UNRESOLVED_TRANSFORM
}

/// `"unresolved"` has to mean the same thing on both sides, and nothing may be built on it.
fn validate_unresolved_transform(out: &mut Vec<Finding>, subject: &str, mapping: &Mapping) {
    if !transform_unresolved(mapping) {
        return;
    }
    if mapping.read_transform != mapping.write_transform {
        out.push(finding(
            Rule::UnresolvedTransformConflict,
            subject,
            format!(
                "one side is {UNRESOLVED_TRANSFORM:?} and the other is not (read {:?}, write {:?});                  an undecided encoding is undecided in both directions",
                mapping.read_transform, mapping.write_transform
            ),
        ));
    }
    if mapping.transform.state == ConstraintState::Established {
        out.push(finding(
            Rule::UnresolvedTransformConflict,
            subject,
            "[mapping.transform] is established while the transform names say unresolved",
        ));
    }
    if mapping.inverse_verified {
        out.push(finding(
            Rule::UnresolvedTransformConflict,
            subject,
            "an unresolved transform cannot have a verified inverse",
        ));
    }
    if mapping.qualification == "automated-tested" {
        out.push(finding(
            Rule::UnresolvedTransformConflict,
            subject,
            "an unresolved transform cannot be automated-tested",
        ));
    }
    if mapping.default.is_some() {
        out.push(finding(
            Rule::UnresolvedTransformConflict,
            subject,
            "Clear to Defaults cannot write a value through an encoding that is not established",
        ));
    }
}

/// Whether the mapping names the field(s) it encodes — one field, or a complete component set.
pub fn field_identity_established(mapping: &Mapping) -> bool {
    mapping.field.is_some() || !mapping.components.is_empty()
}

fn validate_components(
    out: &mut Vec<Finding>,
    subject: &str,
    adapter: &Adapter,
    mapping: &Mapping,
) {
    if mapping.components.is_empty() {
        return;
    }
    if mapping.field.is_some() {
        out.push(finding(
            Rule::CompositeIdentityConflict,
            subject,
            "a mapping declares either a single `field` or a component set, never both",
        ));
    }
    if mapping.storage.state != ConstraintState::Unknown {
        out.push(finding(
            Rule::CompositeIdentityConflict,
            subject,
            "a composite mapping's storage lives on its components; the mapping-level [mapping.storage] must stay unknown",
        ));
    }
    let mut roles: BTreeSet<&str> = BTreeSet::new();
    for component in &mapping.components {
        let component_subject = format!("{subject}#{}", component.role);
        check_identity(out, &component_subject, "component.role", &component.role);
        check_identity(out, &component_subject, "component.field", &component.field);
        if !roles.insert(component.role.as_str()) {
            out.push(finding(
                Rule::CompositeIdentityConflict,
                &component_subject,
                "the same component role is declared twice",
            ));
        }
        check_envelope(
            out,
            &component_subject,
            "mapping.component.storage",
            &component.storage,
        );
        if let Some(storage) = component.storage.established() {
            validate_storage(out, &component_subject, adapter, storage);
        }
    }
}

fn validate_storage(
    out: &mut Vec<Finding>,
    subject: &str,
    adapter: &Adapter,
    storage: &StorageValue,
) {
    check_vocab(
        out,
        subject,
        "[mapping.storage].kind",
        &storage.kind,
        STORAGE_KIND,
    );
    check_vocab(
        out,
        subject,
        "[mapping.storage].signedness",
        &storage.signedness,
        SIGNEDNESS,
    );
    check_vocab(
        out,
        subject,
        "[mapping.storage].endian",
        &storage.endian,
        ENDIAN,
    );
    if !adapter.endian.is_empty() && storage.endian != adapter.endian {
        out.push(finding(
            Rule::StorageBoundsInconsistent,
            subject,
            format!(
                "field endianness {:?} contradicts the adapter's {:?}",
                storage.endian, adapter.endian
            ),
        ));
    }
    if storage.bit_width == 0 {
        out.push(finding(
            Rule::StorageBoundsInconsistent,
            subject,
            "bit_width = 0",
        ));
        return;
    }
    if storage.range_low > storage.range_high {
        out.push(finding(
            Rule::StorageBoundsInconsistent,
            subject,
            format!(
                "range_low {} exceeds range_high {}",
                storage.range_low, storage.range_high
            ),
        ));
    }
    // The declared range must fit the declared width. EA's ranges are inclusive and, for the
    // sentinel/signed regimes, span `2^width` distinct values starting at `range_low`.
    let span = storage
        .range_high
        .checked_sub(storage.range_low)
        .unwrap_or(i64::MAX);
    let capacity = if storage.bit_width >= 63 {
        i64::MAX
    } else {
        (1i64 << storage.bit_width) - 1
    };
    if span > capacity {
        out.push(finding(
            Rule::StorageBoundsInconsistent,
            subject,
            format!(
                "range {}..={} spans {} values, more than {} bits can hold",
                storage.range_low,
                storage.range_high,
                span + 1,
                storage.bit_width
            ),
        ));
    }
    if storage.signedness == "unsigned" && storage.range_low < 0 {
        out.push(finding(
            Rule::StorageBoundsInconsistent,
            subject,
            format!(
                "signedness is unsigned but range_low is {}",
                storage.range_low
            ),
        ));
    }
    if storage.signedness == "neg1-sentinel" && storage.range_low != -1 {
        out.push(finding(
            Rule::StorageBoundsInconsistent,
            subject,
            "the neg1-sentinel regime is defined by range_low == -1",
        ));
    }
}

fn validate_transform(
    out: &mut Vec<Finding>,
    subject: &str,
    mapping: &Mapping,
    transform: &TransformValue,
) {
    check_vocab(
        out,
        subject,
        "[mapping.transform].kind",
        &transform.kind,
        TRANSFORM_KIND,
    );
    let expected_read = match transform.kind.as_str() {
        "identity" => Some("identity".to_string()),
        "index-origin" => transform.index_origin.map(|o| format!("index-origin-{o}")),
        "linear-offset" => transform.offset.map(|o| format!("offset{o:+}")),
        "enum-code" => Some("enum-code".to_string()),
        "inline-string" => Some("inline-string".to_string()),
        _ => None,
    };
    match transform.kind.as_str() {
        "index-origin" if transform.index_origin.is_none() => out.push(finding(
            Rule::TransformDeclarationConflict,
            subject,
            "kind = \"index-origin\" requires index_origin",
        )),
        "linear-offset" if transform.offset.is_none() => out.push(finding(
            Rule::TransformDeclarationConflict,
            subject,
            "kind = \"linear-offset\" requires offset",
        )),
        "identity" | "enum-code" | "inline-string"
            if transform.index_origin.is_some() || transform.offset.is_some() =>
        {
            out.push(finding(
                Rule::TransformDeclarationConflict,
                subject,
                format!("kind = {:?} takes no parameters", transform.kind),
            ));
        }
        _ => {}
    }
    if let Some(expected) = expected_read {
        if mapping.read_transform != expected {
            out.push(finding(
                Rule::TransformDeclarationConflict,
                subject,
                format!(
                    "read_transform {:?} does not spell the structured transform ({expected:?})",
                    mapping.read_transform
                ),
            ));
        }
    }
    // A linear offset's inverse is fully determined, so it can be checked rather than trusted.
    if transform.kind == "linear-offset" {
        if let Some(offset) = transform.offset {
            let expected_write = format!("offset{:+}", -offset);
            if mapping.write_transform != expected_write {
                out.push(finding(
                    Rule::TransformDeclarationConflict,
                    subject,
                    format!(
                        "write_transform {:?} is not the inverse of offset{offset:+} ({expected_write:?})",
                        mapping.write_transform
                    ),
                ));
            }
        }
    }
    if transform.kind == "enum-code" && mapping.enumeration.established().is_none() {
        out.push(finding(
            Rule::TransformDeclarationConflict,
            subject,
            "an enum-code transform needs an established [mapping.enumeration]",
        ));
    }
    if transform.kind == "inline-string" && mapping.string_storage.established().is_none() {
        out.push(finding(
            Rule::TransformDeclarationConflict,
            subject,
            "an inline-string transform needs an established [mapping.string_storage]",
        ));
    }
}

fn validate_enumeration(
    out: &mut Vec<Finding>,
    subject: &str,
    schema: &ProjectSchema,
    property: Option<&Property>,
    mapping: &Mapping,
    enumeration: &EnumerationValue,
) {
    if enumeration.entries.is_empty() {
        out.push(finding(
            Rule::EstablishedWithoutValue,
            subject,
            "[mapping.enumeration] is established with no entries",
        ));
    }
    if property.is_some_and(|p| p.kind != "enum") {
        out.push(finding(
            Rule::ConstraintKindMismatch,
            subject,
            "an enumeration is declared on a canonical property that is not an enum",
        ));
    }
    let mut codes: BTreeSet<i64> = BTreeSet::new();
    let mut values: BTreeSet<&str> = BTreeSet::new();
    let domain_values: Option<BTreeSet<&str>> = property
        .and_then(|p| schema.domain_of(p))
        .map(|d| d.values.iter().map(String::as_str).collect());
    for entry in &enumeration.entries {
        if !codes.insert(entry.code) {
            out.push(finding(
                Rule::EnumCodeConflict,
                subject,
                format!("stored code {} is declared twice", entry.code),
            ));
        }
        if !values.insert(entry.canonical.as_str()) {
            out.push(finding(
                Rule::EnumCodeConflict,
                subject,
                format!("canonical value {:?} is declared twice", entry.canonical),
            ));
        }
        if let Some(allowed) = &domain_values {
            if !allowed.contains(entry.canonical.as_str()) {
                out.push(finding(
                    Rule::EnumValueNotInDomain,
                    subject,
                    format!(
                        "{:?} is not a value of the canonical domain {:?}",
                        entry.canonical, allowed
                    ),
                ));
            }
        }
        if let Some(storage) = mapping.storage.established() {
            if entry.code < storage.range_low || entry.code > storage.range_high {
                out.push(finding(
                    Rule::EnumCodeOutOfRange,
                    subject,
                    format!(
                        "stored code {} is outside the field's {}..={}",
                        entry.code, storage.range_low, storage.range_high
                    ),
                ));
            }
        }
    }
}

fn validate_sentinel(
    out: &mut Vec<Finding>,
    subject: &str,
    mapping: &Mapping,
    sentinel: &NullSentinelValue,
) {
    check_vocab(
        out,
        subject,
        "[mapping.null_sentinel].encoding",
        &sentinel.encoding,
        SENTINEL_ENCODING,
    );
    if let Some(meaning) = &sentinel.canonical_meaning {
        check_vocab(
            out,
            subject,
            "[mapping.null_sentinel].canonical_meaning",
            meaning,
            SENTINEL_MEANING,
        );
    }
    match sentinel.encoding.as_str() {
        "none" if sentinel.raw.is_some() => out.push(finding(
            Rule::SentinelConflict,
            subject,
            "encoding = \"none\" means the field has no sentinel, so it carries no raw code",
        )),
        "raw-all-ones" | "literal-code" if sentinel.raw.is_none() => out.push(finding(
            Rule::SentinelConflict,
            subject,
            format!("encoding = {:?} requires the raw code", sentinel.encoding),
        )),
        _ => {}
    }
    let Some(raw) = sentinel.raw else { return };
    // Units (see `SENTINEL_ENCODING`): `raw-all-ones` is written -1 by convention; `literal-code`
    // is the unsigned stored bit pattern and must fit the field.
    if sentinel.encoding == "raw-all-ones" && raw != -1 {
        out.push(finding(
            Rule::SentinelConflict,
            subject,
            format!("encoding = \"raw-all-ones\" is written as raw = -1 by convention, not {raw}"),
        ));
    }
    if sentinel.encoding == "literal-code" && raw < 0 {
        out.push(finding(
            Rule::SentinelConflict,
            subject,
            format!("encoding = \"literal-code\" states the unsigned stored bit pattern; {raw} is not one"),
        ));
    }
    if let Some(storage) = mapping.storage.established() {
        if sentinel.encoding == "literal-code"
            && storage.bit_width < 63
            && raw >= (1i64 << storage.bit_width)
        {
            out.push(finding(
                Rule::SentinelConflict,
                subject,
                format!(
                    "literal-code pattern {raw} does not fit in {} bits",
                    storage.bit_width
                ),
            ));
        }
        if storage.signedness == "neg1-sentinel"
            && (sentinel.encoding != "raw-all-ones" || raw != -1)
        {
            out.push(finding(
                Rule::SentinelConflict,
                subject,
                "the neg1-sentinel storage regime encodes absence as raw-all-ones = -1",
            ));
        }
        if storage.signedness == "unsigned" && raw < storage.range_low {
            out.push(finding(
                Rule::SentinelConflict,
                subject,
                format!(
                    "sentinel {raw} is unreachable in an unsigned {}..={} field",
                    storage.range_low, storage.range_high
                ),
            ));
        }
    }
    if let Some(enumeration) = mapping.enumeration.established() {
        if let Some(clash) = enumeration.entries.iter().find(|e| e.code == raw) {
            out.push(finding(
                Rule::SentinelConflict,
                subject,
                format!(
                    "sentinel {raw} collides with the enum code for {:?}",
                    clash.canonical
                ),
            ));
        }
    }
}

fn validate_string_storage(
    out: &mut Vec<Finding>,
    subject: &str,
    property: Option<&Property>,
    mapping: &Mapping,
    string_storage: &StringStorageValue,
) {
    check_vocab(
        out,
        subject,
        "[mapping.string_storage].storage",
        &string_storage.storage,
        STRING_STORAGE,
    );
    check_vocab(
        out,
        subject,
        "[mapping.string_storage].encoding",
        &string_storage.encoding,
        STRING_ENCODING,
    );
    check_vocab(
        out,
        subject,
        "[mapping.string_storage].padding",
        &string_storage.padding,
        STRING_PADDING,
    );
    check_vocab(
        out,
        subject,
        "[mapping.string_storage].over_capacity",
        &string_storage.over_capacity,
        STRING_OVERFLOW,
    );
    if property.is_some_and(|p| p.kind != "string") {
        out.push(finding(
            Rule::ConstraintKindMismatch,
            subject,
            "string storage is declared on a canonical property that is not a string",
        ));
    }
    match string_storage.storage.as_str() {
        "inline-fixed" => match string_storage.byte_capacity {
            None => out.push(finding(
                Rule::StringCapacityConflict,
                subject,
                "inline-fixed storage must state its byte_capacity",
            )),
            Some(bytes) => {
                if let Some(storage) = mapping.storage.established() {
                    if storage.bit_width != bytes * 8 {
                        out.push(finding(
                            Rule::StringCapacityConflict,
                            subject,
                            format!(
                                "byte_capacity {bytes} is {} bits, but [mapping.storage] declares {}",
                                bytes * 8,
                                storage.bit_width
                            ),
                        ));
                    }
                }
            }
        },
        "varchar-pool" if string_storage.byte_capacity.is_some() => out.push(finding(
            Rule::StringCapacityConflict,
            subject,
            "a pooled string has no per-record byte budget",
        )),
        _ => {}
    }
}

fn validate_applicability(
    out: &mut Vec<Finding>,
    subject: &str,
    known: &BTreeSet<String>,
    declared: &BTreeSet<String>,
    applicability: &ApplicabilityValue,
) {
    check_vocab(
        out,
        subject,
        "[mapping.applicability].kind",
        &applicability.kind,
        APPLICABILITY_KIND,
    );
    match applicability.kind.as_str() {
        "all-records" => {
            if applicability.property.is_some()
                || applicability.table.is_some()
                || applicability.field.is_some()
                || !applicability.codes.is_empty()
            {
                out.push(finding(
                    Rule::ApplicabilityIncomplete,
                    subject,
                    "kind = \"all-records\" takes no predicate operands",
                ));
            }
        }
        "canonical-predicate" => match applicability.property.as_deref() {
            None => out.push(finding(
                Rule::ApplicabilityIncomplete,
                subject,
                "kind = \"canonical-predicate\" requires `property`",
            )),
            Some(path) => {
                if !known.contains(path) {
                    out.push(finding(
                        Rule::UnknownCanonicalProperty,
                        subject,
                        format!("applicability reads {path:?}, which the canonical schema does not define"),
                    ));
                }
                if applicability.codes.is_empty() {
                    out.push(finding(
                        Rule::ApplicabilityIncomplete,
                        subject,
                        "a predicate with no codes selects nothing",
                    ));
                }
            }
        },
        "target-predicate" => {
            match (&applicability.table, &applicability.field) {
                (Some(table), Some(field)) => {
                    check_identity(out, subject, "[mapping.applicability].table", table);
                    check_identity(out, subject, "[mapping.applicability].field", field);
                    require_declared_table(
                        out,
                        declared,
                        subject,
                        "[mapping.applicability].table",
                        table,
                    );
                }
                _ => out.push(finding(
                    Rule::ApplicabilityIncomplete,
                    subject,
                    "kind = \"target-predicate\" requires both `table` and `field`",
                )),
            }
            if applicability.codes.is_empty() {
                out.push(finding(
                    Rule::ApplicabilityIncomplete,
                    subject,
                    "a predicate with no codes selects nothing",
                ));
            }
        }
        _ => {}
    }
}

fn validate_capacities(adapter: &Adapter, declared: &BTreeSet<String>, out: &mut Vec<Finding>) {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for capacity in &adapter.capacities {
        let subject = format!("{}/capacity:{}", adapter.target, capacity.table);
        check_identity(out, &subject, "table", &capacity.table);
        require_declared_table(out, declared, &subject, "table", &capacity.table);
        if !seen.insert(capacity.table.as_str()) {
            out.push(finding(
                Rule::DuplicateCapacityDeclaration,
                &subject,
                "declared more than once",
            ));
        }
        check_envelope(out, &subject, "capacity.declared", &capacity.declared);
        if let Some(value) = capacity.declared.established() {
            check_vocab(out, &subject, "source", &value.source, CAPACITY_SOURCE);
            if value.ceiling_records < value.population_records {
                out.push(finding(
                    Rule::CapacityBelowPopulation,
                    &subject,
                    format!(
                        "ceiling {} is below the vanilla population {}",
                        value.ceiling_records, value.population_records
                    ),
                ));
            }
        }
    }
}

fn validate_companions(adapter: &Adapter, declared: &BTreeSet<String>, out: &mut Vec<Finding>) {
    let discriminator_established = adapter
        .position_discriminator
        .as_ref()
        .is_some_and(|d| d.established().is_some());
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for rule in &adapter.companion_rules {
        let subject = format!("{}/companion:{}", adapter.target, rule.primary_table);
        check_identity(out, &subject, "primary_table", &rule.primary_table);
        require_declared_table(
            out,
            declared,
            &subject,
            "primary_table",
            &rule.primary_table,
        );
        if !seen.insert(rule.primary_table.as_str()) {
            out.push(finding(
                Rule::DuplicateCompanionRule,
                &subject,
                "declared more than once",
            ));
        }
        check_envelope(out, &subject, "companion_rule.declared", &rule.declared);
        let Some(value) = rule.declared.established() else {
            continue;
        };
        check_identity(out, &subject, "shared_key_field", &value.shared_key_field);
        let mut tables: BTreeSet<&str> = BTreeSet::new();
        for companion in &value.companions {
            check_identity(out, &subject, "companion.table", &companion.table);
            require_declared_table(out, declared, &subject, "companion.table", &companion.table);
            check_vocab(
                out,
                &subject,
                "companion.cardinality",
                &companion.cardinality,
                CARDINALITY,
            );
            check_vocab(
                out,
                &subject,
                "companion.applies_when",
                &companion.applies_when,
                APPLIES_WHEN,
            );
            if !tables.insert(companion.table.as_str()) {
                out.push(finding(
                    Rule::DuplicateCompanionTable,
                    &subject,
                    format!("companion {:?} is listed twice", companion.table),
                ));
            }
            if companion.applies_when.starts_with("position-") && !discriminator_established {
                out.push(finding(
                    Rule::PositionDiscriminatorMissing,
                    &subject,
                    format!(
                        "companion {:?} applies only to {}, but no [position_discriminator] establishes how that is decided",
                        companion.table, companion.applies_when
                    ),
                ));
            }
        }
    }
}

// ==================================================== coverage, reported apart

/// How much of a target's physical schema an adapter actually maps. `None` when the adapter does not
/// declare its totals — which is recorded as unknown, never estimated.
pub fn mapped_fraction(adapter: &Adapter) -> Option<(u32, u32)> {
    let coverage = adapter.coverage_declared.as_ref()?;
    (coverage.fields_total > 0).then_some((coverage.fields_mapped, coverage.fields_total))
}

/// **Format completeness** — how many of the constraints Q07a.1 names the adapter format can express.
/// This is a statement about the schema, and about nothing else.
pub fn representation_completeness() -> (usize, usize) {
    (Q07A1_REPRESENTATION.len(), Q07A1_REPRESENTATION.len())
}

/// One target's declared constraint population, counted per established block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Population {
    pub target: String,
    pub mappings: usize,
    pub omissions: usize,
    /// Established constraint blocks across all mappings, plus adapter-level capacity, companion and
    /// discriminator blocks.
    pub established_constraints: usize,
    pub unknown_constraints: usize,
    pub not_applicable_constraints: usize,
    /// Unknown blocks that record a **non-operative** proposal. Counted apart because a candidate is
    /// not coverage: nothing reads it, and reporting it beside established blocks would restate the
    /// exact over-claim the candidate channel exists to prevent.
    pub candidate_constraints: usize,
}

/// **Registry population** — what a set of adapters actually declares. Synthetic fixtures are
/// excluded, so exercising a representation can never inflate coverage.
pub fn registry_population<'a>(adapters: impl IntoIterator<Item = &'a Adapter>) -> Vec<Population> {
    adapters
        .into_iter()
        .filter(|a| !a.synthetic)
        .map(|adapter| {
            let mut established = 0usize;
            let mut unknown = 0usize;
            let mut not_applicable = 0usize;
            let mut candidates = 0usize;
            let mut tally = |state: ConstraintState| match state {
                ConstraintState::Established => established += 1,
                ConstraintState::Unknown => unknown += 1,
                ConstraintState::NotApplicable => not_applicable += 1,
            };
            for mapping in &adapter.mappings {
                tally(mapping.storage.state);
                tally(mapping.status.state);
                tally(mapping.transform.state);
                tally(mapping.enumeration.state);
                tally(mapping.null_sentinel.state);
                tally(mapping.string_storage.state);
                tally(mapping.foreign_key.state);
                tally(mapping.applicability.state);
                candidates += [
                    mapping.storage.candidate.is_some(),
                    mapping.status.candidate.is_some(),
                    mapping.transform.candidate.is_some(),
                    mapping.enumeration.candidate.is_some(),
                    mapping.null_sentinel.candidate.is_some(),
                    mapping.string_storage.candidate.is_some(),
                    mapping.foreign_key.candidate.is_some(),
                    mapping.applicability.candidate.is_some(),
                ]
                .iter()
                .filter(|present| **present)
                .count();
            }
            for capacity in &adapter.capacities {
                tally(capacity.declared.state);
                candidates += usize::from(capacity.declared.candidate.is_some());
            }
            for rule in &adapter.companion_rules {
                tally(rule.declared.state);
                candidates += usize::from(rule.declared.candidate.is_some());
            }
            if let Some(discriminator) = &adapter.position_discriminator {
                tally(discriminator.state);
                candidates += usize::from(discriminator.candidate.is_some());
            }
            Population {
                target: adapter.target.clone(),
                mappings: adapter.mappings.len(),
                omissions: adapter.omissions.len(),
                established_constraints: established,
                unknown_constraints: unknown,
                not_applicable_constraints: not_applicable,
                candidate_constraints: candidates,
            }
        })
        .collect()
}

#[cfg(test)]
mod asset_coverage_tests;
#[cfg(test)]
mod asset_resolution_tests;
#[cfg(test)]
mod asset_reuse_tests;
#[cfg(test)]
mod asset_tests;
#[cfg(test)]
mod build_override_tests;
#[cfg(test)]
mod legacy_xbox360_tests;
#[cfg(test)]
mod nhl10_ratings_conflict_tests;
#[cfg(test)]
mod nhl15_ps3_tests;
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod target_batch_tests;
#[cfg(test)]
mod target_early_tests;
#[cfg(test)]
mod target_evidence;
#[cfg(test)]
mod target_ps4_tests;
#[cfg(test)]
mod tests;
