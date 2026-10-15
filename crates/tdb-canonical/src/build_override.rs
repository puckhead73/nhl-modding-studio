//! `build-mapping-override/1` — a **target Build mapping** override record (Q07a.3a).
//!
//! # What an override is
//!
//! An override changes a target Build's **requested derivation settings**, and only ever to a value
//! the target's *proven* constraints already permit. It never authorises a departure from those
//! constraints. Canonical assets are not touched; the target profile is not touched; the record
//! lives with the Build mapping, exactly as Q07a.3a settled for invalid database values.
//!
//! If a proven profile permits several output dimensions, a Build may select one of them. If a
//! codec is required, no approval authorises a different codec — [`resolve_build_override`]
//! evaluates the constraint **before** it looks at the approval, so an approval cannot rescue a
//! violation and there is no code path in which it could.
//!
//! # What it is not
//!
//! * **Not a property bag.** The proposed value is a [`SettingChoice`], a closed set of typed
//!   settings with typed operands. A setting outside that set cannot be spelled, and
//!   `deny_unknown_fields` makes an unknown key a **parse** error rather than a validation warning.
//! * **Not a permission.** There is no key here, and none in a profile, that widens what a target
//!   accepts. The permitted set is *read off* the established constraint and is never declared by
//!   the record.
//! * **Not a support claim.** A valid, approved override says nothing about whether a converter
//!   exists, works, or has ever run, and nothing about game acceptance. No variant of
//!   [`OverrideOutcome`] can express either, and the vocabulary has no such term.
//!
//! # Unknown constraints block, they do not default
//!
//! A setting whose constraint is `unknown` yields [`OverrideOutcome::BlockedByUnknownConstraint`].
//! Nothing about the record — an approval least of all — can turn that into a permitted choice: a
//! value cannot be checked against a requirement nobody has established.
//!
//! # Approval is bound to the exact proposal
//!
//! An [`Approval`] carries **no copy of any input**. A copied revision or digest can only prove
//! that *that field* did not move, which is precisely how a differently-valued but still permitted
//! proposal could reuse an approval. Instead it stores one [`override_fingerprint`]: a versioned
//! SHA-256 over a canonical encoding of every binding input — source identity, revision, hash
//! algorithm and digest; target, family, mapping identity and revision; the typed setting and its
//! proposed value; the prior value or its explicit absence; the resolution method and the
//! provenance; and the profile's identity, version and **content digest**. Changing any of them
//! makes the stored digest stop matching.
//!
//! Nor does resolution trust the record's own account of the world. The caller supplies a
//! [`BuildContext`] — the current source and mapping — and a [`ValidatedProfile`] whose content
//! digest is computed from the profile actually supplied. The record is compared against those, so
//! a source re-exported, an asset renamed or a profile edited in place is detected even though
//! nothing inside the record changed. None of this needs a filesystem or a Build engine.
//!
//! An unapproved but permitted record is [`OverrideOutcome::Proposed`] — a proposal, never a
//! mapping.
//!
//! # Malformed input cannot reach an approval
//!
//! [`resolve_build_override`] takes [`ValidatedOverride`] and [`ValidatedProfile`], each
//! constructible only by passing its validator. A malformed record or profile therefore cannot be
//! resolved at all, and [`OverrideOutcome::ApprovedMapping`] is unreachable for one.
//!
//! # This module converts nothing
//!
//! It reads no asset and writes no file. Specification and validation only, like the rest of the
//! crate.

use serde::Deserialize;

use crate::asset::{
    select_variant, AssetConstraint, AssetProfile, AssetVariant, TargetProfile, VariantQuery,
    VariantSelection, VARIANT_BLOCK, VARIANT_OPTIONAL_BLOCK,
};
use crate::{
    check_identity, check_vocab, finding, is_placeholder, Finding, Rule, ASSET_FAMILY,
    ASSET_PROFILE_SCHEMAS_NATIVE, BUILD_OVERRIDE_SCHEMA,
};

// ============================================================= vocabularies

/// Closed vocabulary: how the proposed value was arrived at. Q07a.3a stores the resolution method
/// beside the correction, so this is a vocabulary and not prose.
///
/// Every term describes a way of **choosing among permitted values**. None of them describes a way
/// of exceeding a constraint, because there is no such operation.
pub const SETTING_RESOLUTION: &[&str] = &[
    "operator-selected",
    "preset-applied",
    "automatic-fit",
    "imported-from-build",
];

/// Closed vocabulary: how a source asset's content hash was computed.
pub const HASH_ALGORITHM: &[&str] = &["blake3", "sha256"];

/// The settings a record may propose, by their `setting` tag.
pub const OVERRIDE_SETTING: &[&str] = &[
    "dimensions",
    "mipmap-count",
    "raster-codec",
    "compression-payload",
    "tiling",
    "atlas-slot",
    "audio-sample-rate",
];

// ================================================================== document

/// One override record: `<build>/overrides/<id>.toml`, schema `build-mapping-override/1`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BuildOverride {
    pub schema: String,
    /// The record's own revision. Not the schema version.
    pub record_version: u32,
    /// A **labelled synthetic fixture**, exactly as for profiles and adapters.
    #[serde(default)]
    pub synthetic: bool,
    #[serde(default)]
    pub note: String,
    pub source: SourceIdentity,
    pub mapping: MappingIdentity,
    pub profile: ProfileIdentity,
    pub proposal: Proposal,
    /// Absent until someone approves the proposal. Absence is the default and is not an approval.
    #[serde(default)]
    pub approval: Option<Approval>,
}

/// Which canonical asset, at which revision, with which bytes.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub asset_id: String,
    pub revision: u32,
    /// One of [`HASH_ALGORITHM`].
    pub hash_algorithm: String,
    /// Lowercase hex digest of the canonical source bytes.
    pub hash: String,
}

/// Which target's Build mapping this record belongs to.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MappingIdentity {
    /// `<title>-<platform>`.
    pub target: String,
    /// One of [`ASSET_FAMILY`].
    pub family: String,
    /// Unique within the Build. One record addresses one mapping on one target, so a record can
    /// never reach another target's mapping.
    pub mapping_id: String,
    pub revision: u32,
}

/// Which profile revision the proposal was written against.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProfileIdentity {
    pub family: String,
    /// Must be one of [`ASSET_PROFILE_SCHEMAS_NATIVE`] — a record cannot claim to have been checked against a
    /// schema this crate does not validate.
    pub schema: String,
    pub profile_version: u32,
    /// Lowercase hex SHA-256 of the profile's operative content — see
    /// [`profile_content_digest`]. A version number alone cannot detect a profile whose
    /// constraints were edited in place, so the digest is what actually binds.
    pub content_digest: String,
}

/// The requested setting, what it replaces, and why.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub proposed: SettingChoice,
    /// What the record knows about **which** of the target's correlated variants this asset is.
    ///
    /// A record cannot name a variant by id, and there is no key in which it could. It states the
    /// observable facts, and [`resolve_build_override`] runs the profile's own deterministic
    /// selection over them - so a caller cannot reach a convenient variant by asking for it, and a
    /// query that does not pin one down is refused rather than resolved to the nearest match.
    #[serde(default)]
    pub variant_selector: Option<VariantSelector>,
    pub prior: PriorValue,
    /// One of [`SETTING_RESOLUTION`].
    pub resolution: String,
    /// Non-empty: an unsourced proposal is not a proposal.
    pub provenance: Vec<String>,
    #[serde(default)]
    pub note: String,
}

/// What the record **expects** the current source to be, as observable facts.
///
/// Deliberately **not** a variant id. Selection is the profile's, run by [`crate::select_variant`]
/// over the facts the caller observed of the current source; a record that named a variant directly
/// would be choosing its own constraints, which is the thing an override must never do.
///
/// C0-AS3-R1 makes the weaker point too: these fields are not facts either. A record is a document
/// somebody wrote, and nothing stops it saying the source is 512 wide when it is 256. So they are
/// treated as **expectations to compare** against [`SourceObservation`], and a record whose
/// expectation the source contradicts - or that the source cannot confirm - is refused. Selection
/// itself never reads them: the facts do the choosing, so writing a convenient one changes nothing
/// except whether the record is refused.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct VariantSelector {
    #[serde(default)]
    pub path_family: Option<String>,
    #[serde(default)]
    pub bucket: Option<String>,
    #[serde(default)]
    pub numeric_id: Option<u32>,
    /// The stored size the existing asset carries. Both or neither.
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

/// One typed derivation setting and its typed operand.
///
/// Adjacently tagged: the `setting` term selects the variant and `operand` carries its fields, so
/// the vocabulary is closed at the type level and no untyped payload exists.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "setting", content = "operand", rename_all = "kebab-case")]
pub enum SettingChoice {
    Dimensions(DimensionsChoice),
    MipmapCount(MipmapCountChoice),
    RasterCodec(RasterCodecChoice),
    CompressionPayload(CompressionPayloadChoice),
    Tiling(TilingChoice),
    AtlasSlot(AtlasSlotChoice),
    AudioSampleRate(AudioSampleRateChoice),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DimensionsChoice {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MipmapCountChoice {
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RasterCodecChoice {
    pub codec: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompressionPayloadChoice {
    pub payload: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TilingChoice {
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AtlasSlotChoice {
    pub slot: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AudioSampleRateChoice {
    pub hz: u32,
}

impl SettingChoice {
    /// The `setting` term, for diagnostics.
    pub fn setting(&self) -> &'static str {
        match self {
            SettingChoice::Dimensions(_) => "dimensions",
            SettingChoice::MipmapCount(_) => "mipmap-count",
            SettingChoice::RasterCodec(_) => "raster-codec",
            SettingChoice::CompressionPayload(_) => "compression-payload",
            SettingChoice::Tiling(_) => "tiling",
            SettingChoice::AtlasSlot(_) => "atlas-slot",
            SettingChoice::AudioSampleRate(_) => "audio-sample-rate",
        }
    }

    /// The profile constraint block whose established value decides what this setting may be.
    pub fn block(&self) -> &'static str {
        match self {
            SettingChoice::Dimensions(_) => "dimensions",
            SettingChoice::MipmapCount(_) => "mipmaps",
            SettingChoice::RasterCodec(_) => "raster",
            SettingChoice::CompressionPayload(_) => "compression",
            SettingChoice::Tiling(_) => "tiling",
            SettingChoice::AtlasSlot(_) => "atlas",
            SettingChoice::AudioSampleRate(_) => "audio",
        }
    }
}

/// What the mapping held before, or a recorded statement that it held nothing.
///
/// Q07a.3a stores the prior value with every correction. An *unrecorded* prior and an *absent* one
/// are different facts, so absence must be stated and explained rather than left to silence.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PriorValue {
    /// `true` when the mapping carried no value for this setting.
    #[serde(default)]
    pub absent: bool,
    /// Required when `absent`: how the absence was determined.
    #[serde(default)]
    pub reason: String,
    /// Required unless `absent`, and its setting must match the proposal's.
    #[serde(default)]
    pub value: Option<SettingChoice>,
}

/// An approval, bound to the **exact** proposal it was granted for.
///
/// It carries no copy of any input. A copied revision or digest can only prove that *that field*
/// did not move, which is what let a differently-valued but still permitted proposal reuse an
/// approval. Instead the approval stores one [`override_fingerprint`] over every binding input, so
/// changing any of them — the proposed value included — makes the stored digest stop matching.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub approved_by: String,
    /// `YYYY-MM-DD`.
    pub approved_at: String,
    /// Which fingerprint construction was used. An approval carrying a version this build does not
    /// implement is invalidated rather than trusted or reinterpreted.
    pub fingerprint_version: u32,
    /// Lowercase hex SHA-256 of the canonical encoding described on [`override_fingerprint`].
    pub fingerprint: String,
    #[serde(default)]
    pub note: String,
}

/// Closed vocabulary: how a selector fact about the current source was obtained.
///
/// Every term names a place the fact was **read from**. There is deliberately no term for deriving
/// one from the proposed output value: a Build that wants a 256-wide output learns nothing about
/// what its source is by wanting it, and inferring the fact from the request would make the whole
/// comparison circular.
///
/// The terms are **not** of equal standing. See [`OPERATIVE_OBSERVATION_METHOD`].
pub const OBSERVATION_METHOD: &[&str] =
    &["header-read", "path-read", "index-read", "operator-entered"];

/// The methods whose facts may **decide** a variant.
///
/// `operator-entered` is excluded, and C0-AS3-R2 excludes it deliberately. The whole of C0-AS3-R1
/// was moving selection off what a document claims about the source and onto what was read from it;
/// a term that lets a person type the facts in reopens exactly that door, one step further along.
/// A typed-in width and a width read out of a header are both strings in the same struct, and only
/// one of them is a measurement.
///
/// So an `operator-entered` observation stays fully representable - it keeps its facts, its method
/// and the source identity it claims to describe, which is worth more than discarding it - and it
/// yields [`OverrideOutcome::BlockedByUnverifiedObservation`] wherever a variant has to be chosen.
/// **No approval can proceed past that**: the check runs at step 3, before an approval is looked at
/// at all, so re-approving an operator-entered observation produces a matching fingerprint and
/// changes nothing.
///
/// # What this is not
///
/// It is **not** a rule about who ran the tool. A person who opens a file and has the parser read
/// its header has produced a `header-read`, and it is operative: the method names where the fact
/// came from, not who initiated the inspection. Recording that read as `operator-entered` would be
/// mislabelling it, and this constant neither asks for nor rewards that.
///
/// Nor is it a verification workflow. Nothing here defines how a typed-in fact could be confirmed,
/// and no such route exists in this crate: the outcome says the observation is unverified and stops.
pub const OPERATIVE_OBSERVATION_METHOD: &[&str] = &["header-read", "path-read", "index-read"];

/// What was actually read off the current source. Every field is optional and an absent one is
/// **unobserved**, never a wildcard: a fact nobody read cannot decide a selector clause, and the
/// selection is left ambiguous rather than completed with a guess.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObservedFacts {
    pub path_family: Option<String>,
    pub bucket: Option<String>,
    pub numeric_id: Option<u32>,
    /// The stored size read off the source. Both or neither - half a size decides nothing.
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Selector facts read off a source, with the identity of the source they were read from.
///
/// The identity is the load-bearing part. Facts with no provenance are indistinguishable from the
/// record's own claims, and the point of the whole mechanism is that they are not the same thing:
/// [`resolve_build_override`] checks that these were read from **the source the Build is currently
/// planning**, byte digest included, before any of them is allowed to select a variant.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceObservation {
    /// The asset, revision, hash algorithm and digest these facts were read from.
    pub of: SourceIdentity,
    /// One of [`OBSERVATION_METHOD`].
    pub method: String,
    pub facts: ObservedFacts,
}

/// The **current** source and mapping state, supplied by the caller.
///
/// Resolution never trusts the record's own account of the world. The record says which asset and
/// which mapping it was written against; this says what they are *now*, and
/// [`resolve_build_override`] compares the two. It is a plain value: no filesystem, no Build
/// engine, no I/O of any kind is required to produce one. Reading a header is the caller's job,
/// and the result arrives here as an [`ObservedFacts`] rather than through a reader this crate
/// would have to own.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildContext {
    pub source: SourceIdentity,
    pub mapping: MappingIdentity,
    /// What the caller read off the current source. `None` means nothing was read - which blocks
    /// any setting whose constraint lives in a variant, because nothing identifies which form the
    /// source is.
    pub observation: Option<SourceObservation>,
}

impl BuildContext {
    /// A context with no observation. Enough for a target that declares no variants; anything
    /// variant-borne is blocked until [`observing`](Self::observing) supplies the facts.
    pub fn new(source: SourceIdentity, mapping: MappingIdentity) -> Self {
        BuildContext {
            source,
            mapping,
            observation: None,
        }
    }

    pub fn observing(mut self, observation: SourceObservation) -> Self {
        self.observation = Some(observation);
        self
    }
}

pub fn parse_build_override(text: &str) -> Result<BuildOverride, toml::de::Error> {
    toml::from_str(text)
}

// ============================================ canonical encoding and fingerprints

/// The fingerprint construction this build implements.
///
/// It is part of the stored approval, so a construction change is detectable rather than silent: an
/// approval carrying another version is invalidated, never reinterpreted under today's rules.
///
/// Version 2 is C0-AS3-R1's: the construction now covers the **observed source facts** the variant
/// was selected from, and their own provenance. A version 1 approval was bound to what the record
/// claimed about the source rather than to what the source turned out to be, so every one of them
/// is invalidated rather than re-read - which is exactly what this constant exists to do.
pub const APPROVAL_FINGERPRINT_VERSION: u32 = 2;

/// A deterministic, unambiguous encoder.
///
/// Every field is written as `name` US `byte-length` US `value` RS. Length-prefixing is what makes
/// the encoding injective: no combination of field values can produce the bytes of a different
/// combination, so two different proposals cannot share a fingerprint by concatenation accident.
/// Field order is fixed by the code below, never by map iteration, so the output does not depend on
/// hash seeds, locale or platform.
struct Canon {
    buf: String,
}

impl Canon {
    fn new(tag: &str, version: u32) -> Self {
        let mut canon = Canon { buf: String::new() };
        canon.text("canon.tag", tag);
        canon.num("canon.version", u64::from(version));
        canon
    }

    fn text(&mut self, name: &str, value: &str) {
        self.buf.push_str(name);
        self.buf.push('\u{1f}');
        self.buf.push_str(&value.len().to_string());
        self.buf.push('\u{1f}');
        self.buf.push_str(value);
        self.buf.push('\u{1e}');
    }

    fn num(&mut self, name: &str, value: u64) {
        self.text(name, &value.to_string());
    }

    fn flag(&mut self, name: &str, value: bool) {
        self.text(name, if value { "true" } else { "false" });
    }

    /// `None` is encoded as a marker that no present value can spell, so an absent operand and an
    /// empty one are different bytes.
    fn opt_num(&mut self, name: &str, value: Option<u32>) {
        match value {
            Some(v) => self.num(name, u64::from(v)),
            None => self.text(name, "\u{0}absent"),
        }
    }

    fn opt_text(&mut self, name: &str, value: Option<&str>) {
        match value {
            Some(v) => self.text(name, v),
            None => self.text(name, "\u{0}absent"),
        }
    }

    /// The count is written first, so a list cannot be confused with a shorter list whose last
    /// element happens to contain a separator.
    fn list(&mut self, name: &str, items: &[String]) {
        self.num(&format!("{name}.count"), items.len() as u64);
        for (n, item) in items.iter().enumerate() {
            self.text(&format!("{name}.{n}"), item);
        }
    }

    fn digest(self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.buf.as_bytes());
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

fn encode_choice(canon: &mut Canon, name: &str, choice: &SettingChoice) {
    canon.text(&format!("{name}.setting"), choice.setting());
    match choice {
        SettingChoice::Dimensions(v) => {
            canon.num(&format!("{name}.width"), u64::from(v.width));
            canon.num(&format!("{name}.height"), u64::from(v.height));
        }
        SettingChoice::MipmapCount(v) => canon.num(&format!("{name}.count"), u64::from(v.count)),
        SettingChoice::RasterCodec(v) => canon.text(&format!("{name}.codec"), &v.codec),
        SettingChoice::CompressionPayload(v) => canon.text(&format!("{name}.payload"), &v.payload),
        SettingChoice::Tiling(v) => canon.text(&format!("{name}.kind"), &v.kind),
        SettingChoice::AtlasSlot(v) => canon.num(&format!("{name}.slot"), u64::from(v.slot)),
        SettingChoice::AudioSampleRate(v) => canon.num(&format!("{name}.hz"), u64::from(v.hz)),
    }
}

/// The fingerprint an approval is granted against: **every input that decides what was approved**.
///
/// # Included
///
/// | Group | Fields |
/// |---|---|
/// | source | `asset_id`, `revision`, `hash_algorithm`, `hash` |
/// | mapping | `target`, `family`, `mapping_id`, `revision` |
/// | proposal | the typed `setting` and every field of its operand; the **variant selector**; `resolution`; `provenance` in order |
/// | prior | `absent`, `reason`, and the typed prior value when present |
/// | profile | `family`, `schema`, `profile_version`, and the profile **content digest** |
///
/// Identity is covered as well as revision, so renaming an asset or a mapping while its revision
/// and bytes stay put changes the fingerprint. The proposed value is covered, so exchanging one
/// permitted value for another permitted value changes it — the defect this construction exists to
/// close. The profile's content digest is covered, so editing a constraint in place without
/// bumping `profile_version` changes it.
///
/// # Deliberately excluded
///
/// | Excluded | Why |
/// |---|---|
/// | `[approval]` itself | a fingerprint that covered the approval could not be stored inside it |
/// | `note` on the record, the proposal and the approval | free prose, describing rather than deciding |
/// | `synthetic` | a fixture label, not an input to any derivation |
/// | `record_version` | the record's own bookkeeping; every substantive field it could track is already covered, so bumping it alone must not invalidate an otherwise identical approval |
///
/// The exclusions are exactly the fields that cannot change what a Build produces. Everything that
/// can is inside.
pub fn override_fingerprint(
    record: &BuildOverride,
    profile_content_digest: &str,
    observation: Option<&SourceObservation>,
) -> String {
    let mut canon = Canon::new(
        "build-mapping-override/approval",
        APPROVAL_FINGERPRINT_VERSION,
    );

    canon.text("source.asset_id", &record.source.asset_id);
    canon.num("source.revision", u64::from(record.source.revision));
    canon.text("source.hash_algorithm", &record.source.hash_algorithm);
    canon.text("source.hash", &record.source.hash);

    canon.text("mapping.target", &record.mapping.target);
    canon.text("mapping.family", &record.mapping.family);
    canon.text("mapping.mapping_id", &record.mapping.mapping_id);
    canon.num("mapping.revision", u64::from(record.mapping.revision));

    canon.text("profile.family", &record.profile.family);
    canon.text("profile.schema", &record.profile.schema);
    canon.num(
        "profile.profile_version",
        u64::from(record.profile.profile_version),
    );
    canon.text("profile.content_digest", profile_content_digest);

    encode_choice(&mut canon, "proposal.proposed", &record.proposal.proposed);
    // The selector decides WHICH variant's constraints the proposal was checked against, so an
    // approval that did not cover it could be reused across two entirely different output forms.
    match &record.proposal.variant_selector {
        Some(selector) => {
            canon.flag("proposal.variant_selector.present", true);
            canon.opt_text(
                "proposal.variant_selector.path_family",
                selector.path_family.as_deref(),
            );
            canon.opt_text(
                "proposal.variant_selector.bucket",
                selector.bucket.as_deref(),
            );
            canon.opt_num("proposal.variant_selector.numeric_id", selector.numeric_id);
            canon.opt_num("proposal.variant_selector.width", selector.width);
            canon.opt_num("proposal.variant_selector.height", selector.height);
        }
        None => canon.flag("proposal.variant_selector.present", false),
    }
    // The facts the variant was actually selected from, and where they were read. An approval
    // granted while the source was the 512-wide form must not survive the source becoming the
    // 256-wide one, and the record's own selector cannot carry that: it is a claim, and the claim
    // does not change when the source does.
    match observation {
        Some(observation) => {
            canon.flag("observation.present", true);
            canon.text("observation.of.asset_id", &observation.of.asset_id);
            canon.num(
                "observation.of.revision",
                u64::from(observation.of.revision),
            );
            canon.text(
                "observation.of.hash_algorithm",
                &observation.of.hash_algorithm,
            );
            canon.text("observation.of.hash", &observation.of.hash);
            canon.text("observation.method", &observation.method);
            canon.opt_text(
                "observation.facts.path_family",
                observation.facts.path_family.as_deref(),
            );
            canon.opt_text(
                "observation.facts.bucket",
                observation.facts.bucket.as_deref(),
            );
            canon.opt_num("observation.facts.numeric_id", observation.facts.numeric_id);
            canon.opt_num("observation.facts.width", observation.facts.width);
            canon.opt_num("observation.facts.height", observation.facts.height);
        }
        None => canon.flag("observation.present", false),
    }
    canon.text("proposal.resolution", &record.proposal.resolution);
    canon.list("proposal.provenance", &record.proposal.provenance);
    canon.flag("proposal.prior.absent", record.proposal.prior.absent);
    canon.text("proposal.prior.reason", &record.proposal.prior.reason);
    match &record.proposal.prior.value {
        Some(value) => encode_choice(&mut canon, "proposal.prior.value", value),
        None => canon.text("proposal.prior.value", "\u{0}absent"),
    }

    canon.digest()
}

/// A digest over a profile's **operative** content: everything that can change what a target
/// accepts.
///
/// # Included
///
/// `family`, `profile_version`, `synthetic`, every `[[canonical_source]]` in order, and for every
/// target in order: the target name, then each of the fifteen blocks by its fixed position with its
/// `state`, `confidence`, `provenance`, `reason` and — when established — every field of its value;
/// then each declared operation and the blocks it requires.
///
/// # Deliberately excluded
///
/// `note` and `derivation` (free prose) and the `candidate` channel. A candidate is non-operative
/// by construction — [`crate::Constraint::established`] cannot see one — so it can never change
/// what a proposal is checked against, and an editorial change to a profile must not invalidate an
/// approval that nothing about the target actually moved.
pub fn profile_content_digest(profile: &AssetProfile) -> String {
    let mut canon = Canon::new("asset-output-profile/content", APPROVAL_FINGERPRINT_VERSION);
    canon.text("family", &profile.family);
    canon.num("profile_version", u64::from(profile.profile_version));
    canon.flag("synthetic", profile.synthetic);

    canon.num(
        "canonical_source.count",
        profile.canonical_sources.len() as u64,
    );
    for (n, source) in profile.canonical_sources.iter().enumerate() {
        canon.text(&format!("canonical_source.{n}.role"), &source.role);
        canon.text(&format!("canonical_source.{n}.class"), &source.class);
        canon.text(
            &format!("canonical_source.{n}.interchange"),
            &source.interchange,
        );
        canon.text(
            &format!("canonical_source.{n}.retention"),
            &source.retention,
        );
    }

    canon.num("target_profile.count", profile.targets.len() as u64);
    for (n, target) in profile.targets.iter().enumerate() {
        let at = format!("target_profile.{n}");
        canon.text(&format!("{at}.target"), &target.target);

        // Evidence identity. A corpus's attribution decides whether anything resting on it may be
        // established at all, so re-labelling one must move the digest.
        canon.num(&format!("{at}.corpus.count"), target.corpora.len() as u64);
        for (m, corpus) in target.corpora.iter().enumerate() {
            let c = format!("{at}.corpus.{m}");
            canon.text(&format!("{c}.id"), &corpus.id);
            canon.text(&format!("{c}.kind"), &corpus.kind);
            canon.text(&format!("{c}.attribution"), &corpus.attribution);
            canon.text(
                &format!("{c}.build_identification"),
                &corpus.build_identification,
            );
            canon.text(&format!("{c}.identity_source"), &corpus.identity_source);
            canon.num(
                &format!("{c}.inspected_files"),
                u64::from(corpus.inspected_files),
            );
            canon.list(&format!("{c}.groups"), &corpus.groups);
            canon.list(&format!("{c}.limitations"), &corpus.limitations);
            // A bridge is what lets an unattributable corpus carry a placement fact, so adding or
            // removing one changes what the profile may establish and must move the digest.
            match &corpus.bridge {
                Some(bridge) => {
                    canon.flag(&format!("{c}.bridge"), true);
                    canon.text(&format!("{c}.bridge.method"), &bridge.method);
                    canon.text(
                        &format!("{c}.bridge.original_corpus"),
                        &bridge.original_corpus,
                    );
                    canon.list(&format!("{c}.bridge.provenance"), &bridge.provenance);
                }
                None => canon.flag(&format!("{c}.bridge"), false),
            }
        }

        // Path families, including the observed membership and - decisively - whether an
        // authoritative rule exists, which is what `[path_rule]` can be established through.
        canon.num(
            &format!("{at}.path_family.count"),
            target.path_families.len() as u64,
        );
        for (m, family) in target.path_families.iter().enumerate() {
            let f = format!("{at}.path_family.{m}");
            canon.text(&format!("{f}.id"), &family.id);
            canon.text(&format!("{f}.template"), &family.template);
            canon.list(&format!("{f}.tokens"), &family.tokens);
            canon.text(&format!("{f}.case"), &family.case);
            canon.text(&format!("{f}.bucket_resolution"), &family.bucket_resolution);
            canon.num(
                &format!("{f}.observed_bucket.count"),
                family.observed_buckets.len() as u64,
            );
            for (k, bucket) in family.observed_buckets.iter().enumerate() {
                canon.text(
                    &format!("{f}.observed_bucket.{k}.directory"),
                    &bucket.directory,
                );
                canon.text(&format!("{f}.observed_bucket.{k}.census"), &bucket.census);
                canon.text(
                    &format!("{f}.observed_bucket.{k}.census_digest"),
                    &bucket.census_digest,
                );
                canon.text(&format!("{f}.observed_bucket.{k}.scope"), &bucket.scope);
                canon.text(&format!("{f}.observed_bucket.{k}.corpus"), &bucket.corpus);
            }
        }

        // Variants: their selectors AND their correlated constraints. A selector edit changes which
        // variant a Build lands on without changing any value, so it has to be inside the binding.
        canon.num(&format!("{at}.variant.count"), target.variants.len() as u64);
        for (m, variant) in target.variants.iter().enumerate() {
            let v = format!("{at}.variant.{m}");
            canon.text(&format!("{v}.id"), &variant.id);
            canon.num(
                &format!("{v}.selector.count"),
                variant.selectors.len() as u64,
            );
            for (k, clause) in variant.selectors.iter().enumerate() {
                let sel = format!("{v}.selector.{k}");
                canon.text(&format!("{sel}.dimension"), &clause.dimension);
                canon.opt_text(&format!("{sel}.equals"), clause.equals.as_deref());
                canon.opt_num(&format!("{sel}.id_low"), clause.id_low);
                canon.opt_num(&format!("{sel}.id_high"), clause.id_high);
                canon.opt_num(&format!("{sel}.width"), clause.width);
                canon.opt_num(&format!("{sel}.height"), clause.height);
            }
            encode_geometry(
                &mut canon,
                &v,
                &variant.raster,
                &variant.dimensions,
                &variant.mipmaps,
                &variant.tiling,
            );
            // The variant's own chain, where it has one. Whether a variant overrides the shared
            // container decides which chain a Build plans against, so it belongs in the binding.
            encode_container(&mut canon, &v, &variant.container);
        }

        encode_block(
            &mut canon,
            &at,
            "discovery",
            &target.discovery,
            |c, p, v| {
                c.text(&format!("{p}.scope"), &v.scope);
                c.text(&format!("{p}.root"), &v.root);
                c.flag(&format!("{p}.recursive"), v.recursive);
            },
        );
        encode_block(&mut canon, &at, "identity", &target.identity, |c, p, v| {
            c.text(&format!("{p}.key"), &v.key);
            c.text(&format!("{p}.source"), &v.source);
            c.text(&format!("{p}.ambiguity"), &v.ambiguity);
        });
        encode_block(
            &mut canon,
            &at,
            "path_rule",
            &target.path_rule,
            |c, p, v| {
                c.list(&format!("{p}.families"), &v.families);
                c.text(&format!("{p}.family_selection"), &v.family_selection);
            },
        );
        encode_container(&mut canon, &at, &target.container);
        encode_geometry(
            &mut canon,
            &at,
            &target.raster,
            &target.dimensions,
            &target.mipmaps,
            &target.tiling,
        );
        encode_block(
            &mut canon,
            &at,
            "compression",
            &target.compression,
            |c, p, v| {
                c.text(&format!("{p}.payload"), &v.payload);
                c.flag(&format!("{p}.deterministic"), v.deterministic);
            },
        );
        encode_block(&mut canon, &at, "atlas", &target.atlas, |c, p, v| {
            c.text(&format!("{p}.sharing"), &v.sharing);
            c.opt_text(&format!("{p}.kind"), v.kind.as_deref());
            c.opt_num(&format!("{p}.page_width"), v.page_width);
            c.opt_num(&format!("{p}.page_height"), v.page_height);
            c.opt_num(&format!("{p}.cell_x"), v.cell_x);
            c.opt_num(&format!("{p}.cell_y"), v.cell_y);
            c.opt_num(&format!("{p}.cell_width"), v.cell_width);
            c.opt_num(&format!("{p}.cell_height"), v.cell_height);
            c.opt_num(&format!("{p}.slot"), v.slot);
        });
        encode_block(
            &mut canon,
            &at,
            "mesh_layout",
            &target.mesh_layout,
            |c, p, v| {
                c.num(
                    &format!("{p}.attribute.count"),
                    v.vertex_attributes.len() as u64,
                );
                for (m, a) in v.vertex_attributes.iter().enumerate() {
                    c.text(&format!("{p}.attribute.{m}.semantic"), &a.semantic);
                    c.num(&format!("{p}.attribute.{m}.index"), u64::from(a.index));
                    c.text(
                        &format!("{p}.attribute.{m}.component_type"),
                        &a.component_type,
                    );
                    c.num(
                        &format!("{p}.attribute.{m}.component_count"),
                        u64::from(a.component_count),
                    );
                    c.num(
                        &format!("{p}.attribute.{m}.byte_offset"),
                        u64::from(a.byte_offset),
                    );
                }
                c.num(
                    &format!("{p}.vertex_stride_bytes"),
                    u64::from(v.vertex_stride_bytes),
                );
                c.text(&format!("{p}.index_format"), &v.index_format);
                c.text(&format!("{p}.winding"), &v.winding);
                c.text(&format!("{p}.coordinate_system"), &v.coordinate_system);
                c.text(&format!("{p}.unit"), &v.unit);
                c.num(&format!("{p}.lod_levels"), u64::from(v.lod_levels));
            },
        );
        encode_block(&mut canon, &at, "uv", &target.uv, |c, p, v| {
            c.text(&format!("{p}.origin"), &v.origin);
            c.text(&format!("{p}.wrap"), &v.wrap);
            c.num(&format!("{p}.channel_count"), u64::from(v.channel_count));
        });
        encode_block(&mut canon, &at, "rig", &target.rig, |c, p, v| {
            c.num(&format!("{p}.bone_count"), u64::from(v.bone_count));
            c.num(&format!("{p}.palette_size"), u64::from(v.palette_size));
            c.num(
                &format!("{p}.max_weights_per_vertex"),
                u64::from(v.max_weights_per_vertex),
            );
            c.text(&format!("{p}.weight_quantization"), &v.weight_quantization);
            c.text(&format!("{p}.bind_pose"), &v.bind_pose);
        });
        encode_block(&mut canon, &at, "audio", &target.audio, |c, p, v| {
            c.text(&format!("{p}.bank"), &v.bank);
            c.text(&format!("{p}.codec"), &v.codec);
            c.num(&format!("{p}.sample_rate_hz"), u64::from(v.sample_rate_hz));
            c.num(&format!("{p}.channels"), u64::from(v.channels));
            c.opt_num(&format!("{p}.bit_depth"), v.bit_depth);
            c.text(&format!("{p}.loop_metadata"), &v.loop_metadata);
            c.opt_num(&format!("{p}.max_duration_ms"), v.max_duration_ms);
        });
        encode_block(&mut canon, &at, "install", &target.install, |c, p, v| {
            c.text(&format!("{p}.kind"), &v.kind);
            c.text(&format!("{p}.size_policy"), &v.size_policy);
            c.text(&format!("{p}.repack"), &v.repack);
            c.text(&format!("{p}.backup"), &v.backup);
            c.flag(&format!("{p}.updates_references"), v.updates_references);
        });
        encode_block(
            &mut canon,
            &at,
            "runtime_lookup",
            &target.runtime_lookup,
            |c, p, v| {
                c.text(&format!("{p}.source"), &v.source);
                c.text(&format!("{p}.patch_precedence"), &v.patch_precedence);
                c.text(&format!("{p}.evidence"), &v.evidence);
            },
        );

        canon.num(
            &format!("{at}.operation.count"),
            target.operations.len() as u64,
        );
        for (m, operation) in target.operations.iter().enumerate() {
            canon.text(
                &format!("{at}.operation.{m}.operation"),
                &operation.operation,
            );
            canon.list(&format!("{at}.operation.{m}.requires"), &operation.requires);
        }
    }

    canon.digest()
}

/// One constraint block: envelope first - including `/3`'s scope, force and corpus, all three of
/// which decide what a proposal is checked against - then the established value if there is one, then
/// the per-sub-property evidence.
fn encode_block<T>(
    canon: &mut Canon,
    at: &str,
    name: &str,
    block: &AssetConstraint<T>,
    encode_value: impl FnOnce(&mut Canon, &str, &T),
) {
    let prefix = format!("{at}.{name}");
    canon.text(&format!("{prefix}.state"), block.state.as_str());
    canon.opt_text(&format!("{prefix}.confidence"), block.confidence.as_deref());
    canon.opt_text(&format!("{prefix}.scope"), block.scope.as_deref());
    canon.opt_text(&format!("{prefix}.force"), block.force.as_deref());
    canon.opt_text(&format!("{prefix}.corpus"), block.corpus.as_deref());
    canon.list(&format!("{prefix}.provenance"), &block.provenance);
    canon.text(&format!("{prefix}.reason"), &block.reason);
    // Which sub-properties are unknown is operative: it decides which operations a partially known
    // block can support, so an approval must not survive one being filled in or emptied out.
    canon.num(
        &format!("{prefix}.property.count"),
        block.properties.len() as u64,
    );
    for (n, (property, evidence)) in block.properties.iter().enumerate() {
        canon.text(&format!("{prefix}.property.{n}.name"), property);
        canon.text(
            &format!("{prefix}.property.{n}.state"),
            evidence.state.as_str(),
        );
        canon.opt_text(
            &format!("{prefix}.property.{n}.scope"),
            evidence.scope.as_deref(),
        );
        canon.text(&format!("{prefix}.property.{n}.reason"), &evidence.reason);
    }
    match block.established() {
        Some(value) => {
            canon.flag(&format!("{prefix}.established"), true);
            encode_value(canon, &format!("{prefix}.value"), value);
        }
        None => canon.flag(&format!("{prefix}.established"), false),
    }
}

/// One container chain, wherever it lives: shared on the target, or inside one variant.
fn encode_container(
    canon: &mut Canon,
    at: &str,
    container: &AssetConstraint<crate::asset::ContainerValue>,
) {
    encode_block(canon, at, "container", container, |c, p, v| {
        c.num(&format!("{p}.layer.count"), v.layers.len() as u64);
        for (m, layer) in v.layers.iter().enumerate() {
            c.text(&format!("{p}.layer.{m}.format"), &layer.format);
            c.text(&format!("{p}.layer.{m}.index"), &layer.index);
        }
        c.flag(
            &format!("{p}.outermost_layer_examined"),
            v.outermost_layer_examined,
        );
    });
}

/// The four correlated blocks, wherever they live: on the target, or inside one variant.
fn encode_geometry(
    canon: &mut Canon,
    at: &str,
    raster: &AssetConstraint<crate::asset::RasterValue>,
    dimensions: &AssetConstraint<crate::asset::DimensionsValue>,
    mipmaps: &AssetConstraint<crate::asset::MipmapValue>,
    tiling: &AssetConstraint<crate::asset::TilingValue>,
) {
    encode_block(canon, at, "raster", raster, |c, p, v| {
        c.text(&format!("{p}.codec"), &v.codec);
        c.list(&format!("{p}.channels"), &v.channels);
        c.opt_text(&format!("{p}.color_space"), v.color_space.as_deref());
        c.opt_text(&format!("{p}.alpha"), v.alpha.as_deref());
    });
    encode_block(canon, at, "dimensions", dimensions, |c, p, v| {
        c.num(&format!("{p}.width"), u64::from(v.width));
        c.num(&format!("{p}.height"), u64::from(v.height));
        c.text(&format!("{p}.variant"), &v.variant);
        c.flag(&format!("{p}.power_of_two"), v.power_of_two);
    });
    encode_block(canon, at, "mipmaps", mipmaps, |c, p, v| {
        c.text(&format!("{p}.policy"), &v.policy);
        c.opt_num(&format!("{p}.count"), v.count);
        c.opt_text(&format!("{p}.filter"), v.filter.as_deref());
    });
    encode_block(canon, at, "tiling", tiling, |c, p, v| {
        c.text(&format!("{p}.kind"), &v.kind);
        c.opt_num(&format!("{p}.alignment_bytes"), v.alignment_bytes);
    });
}

// ================================================================= validation

/// Validate one override record as a **document**: identity, completeness, vocabularies.
///
/// This says nothing about whether the proposal is permitted — that is
/// [`resolve_build_override`], which needs the profile.
pub fn validate_build_override(record: &BuildOverride) -> Vec<Finding> {
    let mut out = Vec::new();
    let subject = format!("{}#{}", record.mapping.target, record.mapping.mapping_id);

    if record.schema != BUILD_OVERRIDE_SCHEMA {
        out.push(finding(
            Rule::UnsupportedSchema,
            &subject,
            format!("expected {BUILD_OVERRIDE_SCHEMA}"),
        ));
    }
    if record.record_version == 0 {
        out.push(finding(
            Rule::UnsupportedVersion,
            &subject,
            "record_version is the record's own revision and starts at 1",
        ));
    }

    // ---- source identity ---------------------------------------------------
    check_identity(
        &mut out,
        &subject,
        "[source].asset_id",
        &record.source.asset_id,
    );
    require_text(
        &mut out,
        &subject,
        "[source].asset_id",
        &record.source.asset_id,
    );
    if record.source.revision == 0 {
        out.push(finding(
            Rule::OverrideRecordIncomplete,
            &subject,
            "[source].revision starts at 1: an override is bound to a revision of the source, and \
             revision zero identifies nothing",
        ));
    }
    check_vocab(
        &mut out,
        &subject,
        "[source].hash_algorithm",
        &record.source.hash_algorithm,
        HASH_ALGORITHM,
    );
    check_hash(&mut out, &subject, "[source].hash", &record.source.hash);

    // ---- mapping identity --------------------------------------------------
    if record
        .mapping
        .target
        .split_once('-')
        .is_none_or(|(t, p)| t.is_empty() || p.is_empty())
    {
        out.push(finding(
            Rule::MalformedTarget,
            &subject,
            "expected <title>-<platform>",
        ));
    }
    check_vocab(
        &mut out,
        &subject,
        "[mapping].family",
        &record.mapping.family,
        ASSET_FAMILY,
    );
    check_identity(
        &mut out,
        &subject,
        "[mapping].mapping_id",
        &record.mapping.mapping_id,
    );
    require_text(
        &mut out,
        &subject,
        "[mapping].mapping_id",
        &record.mapping.mapping_id,
    );
    if record.mapping.revision == 0 {
        out.push(finding(
            Rule::OverrideRecordIncomplete,
            &subject,
            "[mapping].revision starts at 1",
        ));
    }

    // ---- profile identity --------------------------------------------------
    check_vocab(
        &mut out,
        &subject,
        "[profile].family",
        &record.profile.family,
        ASSET_FAMILY,
    );
    if !ASSET_PROFILE_SCHEMAS_NATIVE.contains(&record.profile.schema.as_str()) {
        out.push(finding(
            Rule::UnsupportedSchema,
            &subject,
            format!("[profile].schema: expected one of {ASSET_PROFILE_SCHEMAS_NATIVE:?}"),
        ));
    }
    if record.profile.profile_version == 0 {
        out.push(finding(
            Rule::OverrideRecordIncomplete,
            &subject,
            "[profile].profile_version starts at 1: an approval is bound to a profile revision",
        ));
    }
    check_hash(
        &mut out,
        &subject,
        "[profile].content_digest",
        &record.profile.content_digest,
    );
    if record.profile.content_digest.len() != 64 {
        out.push(finding(
            Rule::OverrideRecordIncomplete,
            &subject,
            format!(
                "[profile].content_digest is {} hex characters; a SHA-256 digest is 64",
                record.profile.content_digest.len()
            ),
        ));
    }
    if record.profile.family != record.mapping.family {
        out.push(finding(
            Rule::OverrideRecordIncomplete,
            &subject,
            format!(
                "[profile].family {:?} and [mapping].family {:?} name different families, so it is \
                 not clear which profile this record was written against",
                record.profile.family, record.mapping.family
            ),
        ));
    }

    // ---- the proposal ------------------------------------------------------
    check_vocab(
        &mut out,
        &subject,
        "[proposal].resolution",
        &record.proposal.resolution,
        SETTING_RESOLUTION,
    );
    if record.proposal.provenance.is_empty() {
        out.push(finding(
            Rule::OverrideRecordIncomplete,
            &subject,
            "[proposal].provenance is empty: a proposal with no citation is not a proposal",
        ));
    }
    validate_prior(&mut out, &subject, &record.proposal);

    // ---- the approval ------------------------------------------------------
    if let Some(approval) = &record.approval {
        validate_approval(&mut out, &subject, approval);
    }

    out.sort();
    out.dedup();
    out
}

fn require_text(out: &mut Vec<Finding>, subject: &str, field: &str, value: &str) {
    if value.trim().is_empty() {
        out.push(finding(
            Rule::OverrideRecordIncomplete,
            subject,
            format!("{field} is empty"),
        ));
    }
}

fn check_hash(out: &mut Vec<Finding>, subject: &str, field: &str, value: &str) {
    if value.trim().is_empty() {
        out.push(finding(
            Rule::OverrideRecordIncomplete,
            subject,
            format!("{field} is empty: an approval bound to no digest is bound to nothing"),
        ));
        return;
    }
    if is_placeholder(value) || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        out.push(finding(
            Rule::PlaceholderAsValue,
            subject,
            format!("{field} = {value:?} is not a hex digest"),
        ));
    }
}

fn validate_prior(out: &mut Vec<Finding>, subject: &str, proposal: &Proposal) {
    let prior = &proposal.prior;
    match (prior.absent, &prior.value) {
        (true, Some(_)) => out.push(finding(
            Rule::OverridePriorConflict,
            subject,
            "[proposal.prior] is marked absent and still carries a value",
        )),
        (false, None) => out.push(finding(
            Rule::OverridePriorConflict,
            subject,
            "[proposal.prior] records neither a value nor an explicit absence; silence is not a \
             prior value",
        )),
        (true, None) => {
            if prior.reason.trim().is_empty() {
                out.push(finding(
                    Rule::OverrideRecordIncomplete,
                    subject,
                    "[proposal.prior].absent records how the absence was determined",
                ));
            }
        }
        (false, Some(value)) => {
            if !prior.reason.trim().is_empty() {
                out.push(finding(
                    Rule::OverridePriorConflict,
                    subject,
                    "[proposal.prior].reason explains an absence, and this prior has a value",
                ));
            }
            if value.setting() != proposal.proposed.setting() {
                out.push(finding(
                    Rule::OverridePriorConflict,
                    subject,
                    format!(
                        "[proposal.prior] is a {:?} value but the proposal changes {:?}: a prior \
                         value is what THIS setting held",
                        value.setting(),
                        proposal.proposed.setting()
                    ),
                ));
            }
        }
    }
}

fn validate_approval(out: &mut Vec<Finding>, subject: &str, approval: &Approval) {
    check_identity(
        out,
        subject,
        "[approval].approved_by",
        &approval.approved_by,
    );
    require_text(
        out,
        subject,
        "[approval].approved_by",
        &approval.approved_by,
    );
    if !is_iso_date(&approval.approved_at) {
        out.push(finding(
            Rule::OverrideApprovalUnbound,
            subject,
            format!(
                "[approval].approved_at = {:?} is not a YYYY-MM-DD date",
                approval.approved_at
            ),
        ));
    }
    check_hash(
        out,
        subject,
        "[approval].fingerprint",
        &approval.fingerprint,
    );
    if approval.fingerprint.len() != 64 {
        out.push(finding(
            Rule::OverrideApprovalUnbound,
            subject,
            format!(
                "[approval].fingerprint is {} hex characters; a SHA-256 digest is 64",
                approval.fingerprint.len()
            ),
        ));
    }
    if approval.fingerprint_version == 0 {
        out.push(finding(
            Rule::OverrideApprovalUnbound,
            subject,
            "[approval].fingerprint_version says which construction produced the digest, and \
             version zero names none",
        ));
    }
}

fn is_iso_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(n, b)| matches!(n, 4 | 7) || b.is_ascii_digit())
}

// ============================================== resolution against the profile

/// What a Build may do with one override record.
///
/// No variant says a converter exists, works, or has run, and none says anything about game
/// acceptance. Those live in the evidence system, and this type deliberately cannot express them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverrideOutcome {
    /// The choice is permitted, and nobody has approved it. A **proposal**, not a mapping: a Build
    /// does not act on this.
    Proposed,
    /// Permitted, approved, and every input the approval was bound to still matches.
    ApprovedMapping,
    /// Permitted and approved, but the named inputs have moved since the approval was granted.
    /// The approval no longer applies and the record is a proposal again.
    ApprovalInvalidated(Vec<String>),
    /// The proposal contradicts a constraint the target is known to **require**. Returned whether or
    /// not an approval is present, because approval is never consulted for a value the target does
    /// not accept.
    RefusedByConstraint { block: String, detail: String },
    /// The constraint is established, but only as the format the inspected source **carries** - not
    /// as something the target requires. Reproducing the observed value is authorised; departing
    /// from it is neither authorised nor refused, because nothing here shows the target would reject
    /// it or accept it. Blocked, and no approval can proceed past it.
    ///
    /// This is the outcome C0-POR1-R1 makes necessary. Reading `BIGF` on every file in an installed
    /// copy establishes what those files are. It does not establish that the game refuses anything
    /// else, and treating it as a refusal would promote an observation into a requirement.
    BlockedByUnprovenConstraint { block: String, detail: String },
    /// The constraint that decides this setting is not established, so the choice cannot be checked
    /// at all. Blocked, never defaulted, and no approval can proceed past it.
    BlockedByUnknownConstraint(String),
    /// The target declares correlated variants and the observed source matches **none** of them.
    /// An explicit unsupported result, never a fallback to a nearby form.
    NoMatchingVariant(String),
    /// The setting's constraint lives in a variant, and the caller supplied no observation of the
    /// current source - so nothing says which of the target's forms is being planned. Blocked: an
    /// unobserved source is not a source that matches everything.
    BlockedByUnobservedSource(String),
    /// The only observation of the current source was **entered by hand**. Representable, kept, and
    /// not authority: a typed-in fact is a claim about the source in the same way the record's own
    /// selector is, and selection was moved off claims on purpose. Blocked before any approval is
    /// consulted, so approving it changes nothing.
    BlockedByUnverifiedObservation(String),
    /// The observation was read from something other than the source the Build is planning. The
    /// named fields differ, and stale facts about a previous export cannot select a variant for
    /// this one.
    ObservationNotOfSource(Vec<String>),
    /// The record's selector states something the current source contradicts, or something the
    /// observation cannot confirm. A refusal, and the one that matters most: it is what stops a
    /// record from reaching another variant's approved mapping by describing the source as that
    /// variant.
    SelectorExpectationMismatch(Vec<String>),
    /// The selector does not pin one variant down - several match, or one matches beside another the
    /// selector cannot decide. A **refusal**: choosing among them would be the Build picking its own
    /// constraints, which is exactly what an override may not do.
    RefusedByAmbiguousVariant(Vec<String>),
    /// The record names a target the profile does not describe.
    NoSuchTarget(String),
    /// The record was written against a different profile than the one supplied - a different
    /// family, a different version, or the **same version with different contents**.
    RecordProfileMismatch(Vec<String>),
    /// The record describes a source or mapping that is not the current one. The named fields
    /// differ between the record and the supplied [`BuildContext`], so the record is stale
    /// regardless of whether anyone approved it.
    StaleContext(Vec<String>),
}

/// A record that has passed [`validate_build_override`].
///
/// [`resolve_build_override`] takes this rather than a bare [`BuildOverride`], so a malformed
/// record cannot reach a resolution at all — [`OverrideOutcome::ApprovedMapping`] least of all.
/// The check is at the type boundary rather than inside the resolver, so it cannot be forgotten by
/// a future caller.
#[derive(Debug, Clone, Copy)]
pub struct ValidatedOverride<'a>(&'a BuildOverride);

impl<'a> ValidatedOverride<'a> {
    /// Validate, or hand back every finding that stopped it.
    pub fn new(record: &'a BuildOverride) -> Result<Self, Vec<Finding>> {
        let findings = validate_build_override(record);
        if findings.is_empty() {
            Ok(ValidatedOverride(record))
        } else {
            Err(findings)
        }
    }

    pub fn record(&self) -> &'a BuildOverride {
        self.0
    }
}

/// A profile that has passed `validate_asset_profile`, carrying its content digest.
///
/// The digest is computed once, here, from the profile that was actually validated — never taken
/// on trust from a record, which is what makes "the profile was edited in place" detectable.
#[derive(Debug, Clone)]
pub struct ValidatedProfile<'a> {
    profile: &'a AssetProfile,
    content_digest: String,
}

impl<'a> ValidatedProfile<'a> {
    pub fn new(profile: &'a AssetProfile) -> Result<Self, Vec<Finding>> {
        let findings = crate::validate_asset_profile(profile);
        if !findings.is_empty() {
            return Err(findings);
        }
        Ok(ValidatedProfile {
            content_digest: profile_content_digest(profile),
            profile,
        })
    }

    pub fn profile(&self) -> &'a AssetProfile {
        self.profile
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }
}

/// Resolve one validated override record against the current context and the current profile.
///
/// Every argument is borrowed and none is mutated: canonical assets and target profiles are
/// unchanged by resolving an override, which is the whole point of putting the record in the Build
/// mapping. Nothing here touches a filesystem or needs a Build engine — `context` is a plain value
/// the caller supplies.
///
/// # Order, which is load-bearing
///
/// 1. **profile** — family, version *and content digest*;
/// 2. **target** — present in that profile;
/// 3. **variant, then constraint** — which of the target's correlated forms the **current source**
///    is, decided from `context.observation` (deterministically, refusing an ambiguous one) after
///    the record's own selector has been checked against those facts as an expectation, and then
///    whether the proposed value is one that form's operative constraint permits;
/// 4. **context** — does the record still describe the current source and mapping;
/// 5. **approval** — only now, and only as a fingerprint comparison.
///
/// An approval is never read for a proposal the target refuses, and never read past an unknown
/// constraint. That property is structural: steps 3 and 4 return before step 5 exists.
///
/// # Where the selector facts come from, and where they do not
///
/// Selection reads `context.observation` and nothing else. The record's `variant_selector` is
/// compared against it and is never substituted for it, and no fact is ever inferred from the value
/// being proposed. A record that claims the source is the 256-wide form when the source is the
/// 512-wide one is refused at step 3 whatever its approval says - the approval is not reached, and
/// would not match in any case, because the observed facts are inside the fingerprint.
///
/// An observation only carries that authority if it was **read**. One whose method is outside
/// [`OPERATIVE_OBSERVATION_METHOD`] - which today means `operator-entered` - is blocked at the same
/// step. A hand-entered fact is a second claim standing where the source should be, and two claims
/// agreeing is not a measurement.
pub fn resolve_build_override(
    record: &ValidatedOverride<'_>,
    context: &BuildContext,
    profile: &ValidatedProfile<'_>,
) -> OverrideOutcome {
    let record = record.record();

    // ---- 1. the profile the record was written against ---------------------
    let mut moved = Vec::new();
    if record.profile.family != profile.profile().family {
        moved.push("profile.family".to_string());
    }
    if record.profile.profile_version != profile.profile().profile_version {
        moved.push("profile.profile_version".to_string());
    }
    // The digest is what catches a profile edited in place: a constraint can change while the
    // version number stays exactly where it was.
    if !record
        .profile
        .content_digest
        .eq_ignore_ascii_case(profile.content_digest())
    {
        moved.push("profile.content_digest".to_string());
    }
    if !moved.is_empty() {
        return OverrideOutcome::RecordProfileMismatch(moved);
    }

    // ---- 2. the target -----------------------------------------------------
    let Some(target) = profile
        .profile()
        .targets
        .iter()
        .find(|t| t.target == record.mapping.target)
    else {
        return OverrideOutcome::NoSuchTarget(record.mapping.target.clone());
    };

    // ---- 3. the variant, then the constraint, before anything about approval ---
    let variant = match resolve_variant(target, &record.proposal, context) {
        Ok(variant) => variant,
        Err(outcome) => return outcome,
    };
    match permitted_choice(target, variant, &record.proposal.proposed) {
        Permission::Permitted => {}
        Permission::Refused { block, detail } => {
            return OverrideOutcome::RefusedByConstraint { block, detail }
        }
        Permission::Unproven { block, detail } => {
            return OverrideOutcome::BlockedByUnprovenConstraint { block, detail }
        }
        Permission::Unknown(block) => return OverrideOutcome::BlockedByUnknownConstraint(block),
    }

    // ---- 4. the record's account of the world against the current one ------
    let stale = context_drift(record, context);
    if !stale.is_empty() {
        return OverrideOutcome::StaleContext(stale);
    }

    // ---- 5. the approval ---------------------------------------------------
    let Some(approval) = &record.approval else {
        return OverrideOutcome::Proposed;
    };
    if approval.fingerprint_version != APPROVAL_FINGERPRINT_VERSION {
        return OverrideOutcome::ApprovalInvalidated(vec![
            "approval.fingerprint_version".to_string()
        ]);
    }
    let expected = override_fingerprint(
        record,
        profile.content_digest(),
        context.observation.as_ref(),
    );
    if approval.fingerprint.eq_ignore_ascii_case(&expected) {
        OverrideOutcome::ApprovedMapping
    } else {
        OverrideOutcome::ApprovalInvalidated(vec!["approval.fingerprint".to_string()])
    }
}

/// Which of the record's declared inputs differ from the current context.
///
/// Identity as well as revision: an asset renamed while its bytes and revision stay put is a
/// different asset, and a mapping renamed while its revision stays put is a different mapping.
pub fn context_drift(record: &BuildOverride, context: &BuildContext) -> Vec<String> {
    let mut drift = Vec::new();
    if record.source.asset_id != context.source.asset_id {
        drift.push("source.asset_id".to_string());
    }
    if record.source.revision != context.source.revision {
        drift.push("source.revision".to_string());
    }
    if record.source.hash_algorithm != context.source.hash_algorithm {
        drift.push("source.hash_algorithm".to_string());
    }
    if !record
        .source
        .hash
        .eq_ignore_ascii_case(&context.source.hash)
    {
        drift.push("source.hash".to_string());
    }
    if record.mapping.target != context.mapping.target {
        drift.push("mapping.target".to_string());
    }
    if record.mapping.family != context.mapping.family {
        drift.push("mapping.family".to_string());
    }
    if record.mapping.mapping_id != context.mapping.mapping_id {
        drift.push("mapping.mapping_id".to_string());
    }
    if record.mapping.revision != context.mapping.revision {
        drift.push("mapping.revision".to_string());
    }
    drift
}

enum Permission {
    Permitted,
    /// The target is known to require something else.
    Refused {
        block: String,
        detail: String,
    },
    /// The constraint is established as the **observed source format** only, and the proposal is not
    /// that value. Neither authorised nor forbidden.
    Unproven {
        block: String,
        detail: String,
    },
    Unknown(String),
}

/// Deny a choice, at the strength the constraint's `force` actually supports.
///
/// A `target-required` constraint refuses. An `observed-source-format` one blocks: it establishes
/// what the source carries, which authorises reproducing that value and says nothing whatever about
/// any other. Reading the second as the first is how "every file we looked at is DXT5" becomes "the
/// game rejects anything else".
fn deny<T>(constraint: &AssetConstraint<T>, block: &str, detail: impl Into<String>) -> Permission {
    let detail = detail.into();
    if constraint.is_target_required() {
        return Permission::Refused {
            block: block.to_string(),
            detail,
        };
    }
    Permission::Unproven {
        block: block.to_string(),
        detail: format!(
            "{detail}. That constraint is established at force {:?} and scope {:?}: it records what \
             the inspected source carries, so it authorises reproducing that value and neither \
             permits nor forbids another",
            constraint.force.as_deref().unwrap_or("(none)"),
            constraint.scope.as_deref().unwrap_or("(none)")
        ),
    }
}

/// Which of the target's variants the **current source** is, if the setting needs one.
///
/// Returns `Ok(None)` when the setting's block is not variant-borne, or when the target declares no
/// variants at all - in both cases the target-level block decides.
///
/// Otherwise this is where the record stops being believed. The facts come from
/// `context.observation`; the record's selector is compared against them and contributes nothing
/// else; and a fact nobody observed leaves its clause undecidable, which
/// [`crate::select_variant`] reports as ambiguity rather than resolving.
fn resolve_variant<'a>(
    target: &'a TargetProfile,
    proposal: &Proposal,
    context: &BuildContext,
) -> Result<Option<&'a AssetVariant>, OverrideOutcome> {
    let block = proposal.proposed.block();
    let variant_borne = VARIANT_BLOCK.contains(&block)
        || (VARIANT_OPTIONAL_BLOCK.contains(&block)
            && target
                .variants
                .iter()
                .any(|v| v.container.state != crate::ConstraintState::Unknown));
    if target.variants.is_empty() || !variant_borne {
        return Ok(None);
    }

    let Some(observation) = &context.observation else {
        return Err(OverrideOutcome::BlockedByUnobservedSource(format!(
            "[{block}] is decided per variant on this target, and nothing was observed of the \
             current source. Which form it is has to be read off the source; the record cannot \
             supply it, and the value being proposed certainly cannot"
        )));
    };
    let misattributed = observation_drift(observation, &context.source);
    if !misattributed.is_empty() {
        return Err(OverrideOutcome::ObservationNotOfSource(misattributed));
    }
    // Authority, checked before the facts are used for anything. Comparing the record's
    // expectations against hand-entered facts would be theatre: two claims agreeing with each
    // other is not a source, and reporting the agreement would read like a confirmation.
    if !OPERATIVE_OBSERVATION_METHOD.contains(&observation.method.as_str()) {
        return Err(OverrideOutcome::BlockedByUnverifiedObservation(format!(
            "[{block}] is decided per variant, and the only observation of the current source \
             carries method {:?}. That is a claim about the source, not a reading of it, and \
             selection runs on readings. The observation is kept with its provenance; it decides \
             nothing, and no approval reaches past this",
            observation.method
        )));
    }
    let mismatched = selector_expectation_drift(proposal.variant_selector.as_ref(), observation);
    if !mismatched.is_empty() {
        return Err(OverrideOutcome::SelectorExpectationMismatch(mismatched));
    }

    let query = observed_query(&observation.facts);
    match select_variant(target, &query) {
        VariantSelection::Selected(id) => Ok(target.variants.iter().find(|v| v.id == id)),
        VariantSelection::NoMatch => Err(OverrideOutcome::NoMatchingVariant(
            "the observed source matches none of this target's declared variants".to_string(),
        )),
        VariantSelection::Ambiguous(ids) => Err(OverrideOutcome::RefusedByAmbiguousVariant(ids)),
        // Unreachable: the target was just checked for variants.
        VariantSelection::NoVariants => Ok(None),
    }
}

/// The query selection actually runs on: the observed facts, and only those.
fn observed_query(facts: &ObservedFacts) -> VariantQuery {
    VariantQuery {
        path_family: facts.path_family.clone(),
        bucket: facts.bucket.clone(),
        numeric_id: facts.numeric_id,
        // Half a size is not a size. Reporting one dimension as if it were both would let a partial
        // read decide a `stored-dimensions` clause.
        stored_dimensions: match (facts.width, facts.height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        },
    }
}

/// Whether an observation was read from the source the Build is currently planning.
///
/// Identity, revision, algorithm and digest, exactly as [`context_drift`] compares the record: facts
/// read off a previous export describe that export, and a re-exported asset can be a different form
/// while keeping its name.
pub fn observation_drift(observation: &SourceObservation, current: &SourceIdentity) -> Vec<String> {
    let mut drift = Vec::new();
    if observation.of.asset_id != current.asset_id {
        drift.push("observation.of.asset_id".to_string());
    }
    if observation.of.revision != current.revision {
        drift.push("observation.of.revision".to_string());
    }
    if observation.of.hash_algorithm != current.hash_algorithm {
        drift.push("observation.of.hash_algorithm".to_string());
    }
    if !observation.of.hash.eq_ignore_ascii_case(&current.hash) {
        drift.push("observation.of.hash".to_string());
    }
    drift
}

/// Which of the record's stated expectations the current source does not bear out.
///
/// Two ways to fail, both refusals. The source says something else - the record expects the
/// `playerheadssmall` family and the source is in `playerheads` - or the source says nothing at
/// all, and the expectation is simply unchecked. The second is refused rather than ignored: a
/// record that states a fact is asking to be held to it, and quietly dropping the claim would let a
/// record describe the source however it liked as long as nobody looked.
fn selector_expectation_drift(
    expected: Option<&VariantSelector>,
    observation: &SourceObservation,
) -> Vec<String> {
    let Some(expected) = expected else {
        return Vec::new();
    };
    let facts = &observation.facts;
    let mut drift = Vec::new();
    let mut text = |field: &str, want: Option<&str>, got: Option<&str>| {
        if let Some(want) = want {
            match got {
                Some(got) if got == want => {}
                Some(got) => drift.push(format!(
                    "proposal.variant_selector.{field}: record {want:?}, source {got:?}"
                )),
                None => drift.push(format!(
                    "proposal.variant_selector.{field}: record {want:?}, source unobserved"
                )),
            }
        }
    };
    text(
        "path_family",
        expected.path_family.as_deref(),
        facts.path_family.as_deref(),
    );
    text(
        "bucket",
        expected.bucket.as_deref(),
        facts.bucket.as_deref(),
    );
    let mut number = |field: &str, want: Option<u32>, got: Option<u32>| {
        if let Some(want) = want {
            match got {
                Some(got) if got == want => {}
                Some(got) => drift.push(format!(
                    "proposal.variant_selector.{field}: record {want}, source {got}"
                )),
                None => drift.push(format!(
                    "proposal.variant_selector.{field}: record {want}, source unobserved"
                )),
            }
        }
    };
    number("numeric_id", expected.numeric_id, facts.numeric_id);
    number("width", expected.width, facts.width);
    number("height", expected.height, facts.height);
    drift.sort();
    drift
}

/// Whether the target's **established** constraint permits this exact choice.
///
/// The permitted set is derived from the constraint. Nothing in the record contributes to it, and
/// a setting whose block is unknown has no permitted set at all. Where the target declares variants,
/// the correlated blocks come from the variant the selector identified - never from a variant the
/// record chose.
fn permitted_choice(
    target: &TargetProfile,
    variant: Option<&AssetVariant>,
    choice: &SettingChoice,
) -> Permission {
    let block = choice.block();
    match choice {
        SettingChoice::Dimensions(want) => {
            let constraint = variant.map_or(&target.dimensions, |v| &v.dimensions);
            let Some(v) = constraint.operative() else {
                return not_operative(block, constraint);
            };
            if want.width == 0 || want.height == 0 {
                return Permission::Refused {
                    block: block.to_string(),
                    detail: "a zero extent is not an output size".to_string(),
                };
            }
            if v.power_of_two && (!want.width.is_power_of_two() || !want.height.is_power_of_two()) {
                return deny(
                    constraint,
                    block,
                    format!(
                        "{}x{} is not a power of two, and this target's established constraint is \
                         a power-of-two surface",
                        want.width, want.height
                    ),
                );
            }
            if (want.width, want.height) != (v.width, v.height) {
                return deny(
                    constraint,
                    block,
                    format!(
                        "the established output size for variant {:?} is {}x{}; {}x{} is outside it",
                        v.variant, v.width, v.height, want.width, want.height
                    ),
                );
            }
            Permission::Permitted
        }
        SettingChoice::MipmapCount(want) => {
            let constraint = variant.map_or(&target.mipmaps, |v| &v.mipmaps);
            let Some(v) = constraint.operative() else {
                return not_operative(block, constraint);
            };
            if v.policy != "fixed-count" {
                return deny(
                    constraint,
                    block,
                    format!(
                        "under policy {:?} the chain length follows from the surface and the \
                         policy, so it is not a Build setting",
                        v.policy
                    ),
                );
            }
            match v.count {
                Some(count) if count == want.count => Permission::Permitted,
                Some(count) => deny(
                    constraint,
                    block,
                    format!(
                        "the established fixed count is {count}; {} is outside it",
                        want.count
                    ),
                ),
                None => Permission::Refused {
                    block: block.to_string(),
                    detail: "the policy is fixed-count but no count is established, so nothing \
                             decides what is permitted"
                        .to_string(),
                },
            }
        }
        SettingChoice::RasterCodec(want) => {
            let constraint = variant.map_or(&target.raster, |v| &v.raster);
            let Some(v) = constraint.operative() else {
                return not_operative(block, constraint);
            };
            if want.codec == v.codec {
                Permission::Permitted
            } else {
                deny(
                    constraint,
                    block,
                    format!(
                        "the established codec is {:?}; {:?} is a different one",
                        v.codec, want.codec
                    ),
                )
            }
        }
        SettingChoice::CompressionPayload(want) => {
            let constraint = &target.compression;
            let Some(v) = constraint.operative() else {
                return not_operative(block, constraint);
            };
            if want.payload == v.payload {
                Permission::Permitted
            } else {
                deny(
                    constraint,
                    block,
                    format!(
                        "the established payload codec is {:?}; {:?} is a different one",
                        v.payload, want.payload
                    ),
                )
            }
        }
        SettingChoice::Tiling(want) => {
            let constraint = variant.map_or(&target.tiling, |v| &v.tiling);
            let Some(v) = constraint.operative() else {
                return not_operative(block, constraint);
            };
            if want.kind == v.kind {
                Permission::Permitted
            } else {
                deny(
                    constraint,
                    block,
                    format!(
                        "the established tiling is {:?}; {:?} is a different one",
                        v.kind, want.kind
                    ),
                )
            }
        }
        SettingChoice::AtlasSlot(want) => {
            let constraint = &target.atlas;
            let Some(v) = constraint.operative() else {
                return not_operative(block, constraint);
            };
            // `/3` lets `[atlas]` be established on the one thing a payload count measures - whether
            // a surface is shared - while the atlas/pool kind stays unknown. A slot is a coordinate
            // in a scheme, so without the kind there is no scheme to index into.
            let Some(kind) = v.kind.as_deref() else {
                return Permission::Unknown(block.to_string());
            };
            if kind == "none" {
                return Permission::Refused {
                    block: block.to_string(),
                    detail: "this target stores one surface per file: there is no slot".to_string(),
                };
            }
            let (Some(pw), Some(ph), Some(cw), Some(ch)) =
                (v.page_width, v.page_height, v.cell_width, v.cell_height)
            else {
                return Permission::Refused {
                    block: block.to_string(),
                    detail: "the page and cell extents are not both established, so the number of \
                             slots is not decided"
                        .to_string(),
                };
            };
            if cw == 0 || ch == 0 {
                return Permission::Refused {
                    block: block.to_string(),
                    detail: "a zero cell extent yields no slots".to_string(),
                };
            }
            let slots = (pw / cw) * (ph / ch);
            if want.slot >= slots {
                return deny(
                    constraint,
                    block,
                    format!(
                        "the established {pw}x{ph} page of {cw}x{ch} cells holds {slots} slots \
                         (0..{}); slot {} is outside it",
                        slots.saturating_sub(1),
                        want.slot
                    ),
                );
            }
            // Under an observed source format only the slot the source occupies is authorised: the
            // page geometry says a slot EXISTS, not that the target accepts this asset in it.
            if !constraint.is_target_required() && v.slot != Some(want.slot) {
                return deny(
                    constraint,
                    block,
                    format!(
                        "the inspected source occupies slot {}; slot {} is a different one",
                        v.slot
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "(unrecorded)".to_string()),
                        want.slot
                    ),
                );
            }
            Permission::Permitted
        }
        SettingChoice::AudioSampleRate(want) => {
            let constraint = &target.audio;
            let Some(v) = constraint.operative() else {
                return not_operative(block, constraint);
            };
            if want.hz == v.sample_rate_hz {
                Permission::Permitted
            } else {
                deny(
                    constraint,
                    block,
                    format!(
                        "the established sample rate is {} Hz; {} Hz is a different one",
                        v.sample_rate_hz, want.hz
                    ),
                )
            }
        }
    }
}

/// An unknown constraint blocks; one the target does not have refuses. They are different answers
/// because they are different facts.
/// Why a block yields no permitted set: absent, unknown, or decided about the wrong population.
///
/// The third case is C0-AS3-R1's. A block established at `specimen-observation` scope IS decided -
/// somebody measured those files and wrote down what they found - and it still cannot decide a
/// Build, because the thing it decided is not the target. Blocked, with the scope named, rather
/// than reported as unknown: "nobody knows" and "somebody knows about four other files" are
/// different states and a reader acts differently on them.
fn not_operative<T>(block: &str, c: &AssetConstraint<T>) -> Permission {
    match c.state {
        crate::ConstraintState::NotApplicable => Permission::Refused {
            block: block.to_string(),
            detail: format!(
                "this target has no [{block}] constraint, so it has no setting to choose"
            ),
        },
        crate::ConstraintState::Established => Permission::Unproven {
            block: block.to_string(),
            detail: format!(
                "[{block}] is established at scope {:?}, which speaks for the specimens it was \
                 measured on and not for this target. It is evidence, and it is not a constraint a \
                 Build can be checked against",
                c.scope.as_deref().unwrap_or("(none)")
            ),
        },
        crate::ConstraintState::Unknown => Permission::Unknown(block.to_string()),
    }
}
