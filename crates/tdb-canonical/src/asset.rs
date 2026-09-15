//! `asset-output-profile/3` — the **asset** half of the Q07a.1 registry.
//!
//! An asset output profile says how a canonical source asset becomes the bytes **one target**
//! consumes. `asset-output-profile/1` recorded that with seven free-text strings; `/2` replaced them
//! with fifteen evidence-qualified [`AssetConstraint`] blocks. `/3` keeps every `/2` block and adds
//! the three things C0-POR1-R1 showed `/2` could not express without over-claiming.
//!
//! # 1. Evidence scope is not confidence
//!
//! `/2` had one axis — `measured` / `derived` / `asserted` — and no way to say **how far** a
//! measurement reaches. C0-POR1 measured three private copies exhaustively and `/2` could only
//! record that as "measured", which reads as target-wide. `/3` adds two orthogonal keys to every
//! block:
//!
//! * [`EVIDENCE_SCOPE`] — `specimen-observation`, `inspected-corpus-invariant` or
//!   `supported-target-constraint`. A specimen observation may be *recorded* — somebody decoded
//!   those files and what they found is true of them — and it is never **operative**: it decides no
//!   Build, satisfies no operation's requirement, and cannot describe where a file sat.
//!   [`AssetConstraint::operative`], not `established`, is the accessor a decision goes through.
//! * [`CONSTRAINT_FORCE`] — `observed-source-format` or `target-required`. An observed source format
//!   informs **source-preserving** planning and nothing else: it does not prove that another format
//!   is forbidden, and it does not prove another format is supported. Only a
//!   `supported-target-constraint` may carry `target-required`, and only a `target-required`
//!   constraint refuses an alternative value outright.
//!
//! and one binding key, `corpus`, naming the [`Corpus`] the fact rests on. A corpus records its own
//! kind, its **attribution** to the target and whether the build behind it was identified. A derived
//! or extracted tree is `not-attributable` by construction, so however exhaustively it was measured
//! it cannot speak for the target — which is exactly the C0-POR1-R1 outcome, now enforced rather
//! than narrated.
//!
//! What such a tree *can* support, as of C0-AS3-R1, is a `specimen-observation` about its own
//! files. The two claims are different sizes and the schema keeps them apart rather than refusing
//! both. The exception is a [`PLACEMENT_BLOCK`] — where the asset sits — which extracting is
//! precisely the step that discards: reaching one needs a [`ProvenanceBridge`], the extraction
//! record that survived, and no real corpus in this repository has one.
//!
//! # 2. Correlated variants
//!
//! One target can carry several portrait forms at once. `/2` held **one** `DimensionsValue` per
//! target, so a target with 512x512, 512x256 and 256x128 co-existing could only be left unknown.
//! `/3` adds [`AssetVariant`]: dimensions, raster, mip layout, tiling and path family stay
//! **correlated inside one variant**, so no unobserved cross-product is representable. There is
//! deliberately no per-target list of allowed widths or allowed codecs.
//!
//! The container chain is a [`VARIANT_OPTIONAL_BLOCK`] rather than one of the correlated four,
//! because whether it correlates is itself a fact about the target. On the portrait targets one
//! chain holds every form; elsewhere a loose file and an archive member are two forms of one
//! family. Either is sayable, exactly one level may decide it, and deciding it at both is refused —
//! so resolution is a rule and not a precedence puzzle.
//!
//! Selection is [`select_variant`], and it is deterministic: exactly one definite match selects;
//! nothing matching is [`VariantSelection::NoMatch`]; anything else — several matches, or one match
//! beside a variant the query cannot decide — is [`VariantSelection::Ambiguous`], a refusal. There
//! is no first-match rule and no nearest-looking variant.
//!
//! # 3. Partially known properties
//!
//! `/2` bundled `{codec, color_space, channels, alpha}` into one value, so an unmeasurable colour
//! space blocked the whole `raster` block — and C0-POR1 filled it with a guess to make the bundle
//! well-formed. In `/3` the unmeasurable sub-properties are `Option`, absence means unknown, and
//! `[<block>.property.<name>]` keeps that unknown's own evidence, reason and candidate. An operation
//! names what it actually needs (`raster.codec`, not `raster`), so an unknown sub-property blocks
//! only the operations that require it.
//!
//! # 4. Observed paths without an invented lookup rule
//!
//! [`PathFamily`] records the observed path families — a target may have several — and an
//! [`ObservedBucket`] per directory seen inside each. The detailed membership lives in the census,
//! referenced here by document and digest: a column of id ranges sitting in a profile reads as a
//! lookup table however it is labelled. Membership is an **observation**; the
//! authoritative id-to-directory rule is `bucket_resolution`, which stays `unknown` until something
//! other than a folder label establishes it, and `[path_rule]` cannot be established while any named
//! family is unresolved. The separate `[runtime_lookup]` block carries what the *game* loads,
//! including patch precedence; it is unknown everywhere and blocks `discover` and `install`.
//!
//! # Format capability is not converter capability
//!
//! A profile describes the **target's output constraints**. It contains no key in which a converter,
//! a support level or an operation result could be recorded, and
//! [`operation_constraint_readiness`] — the only function that answers a per-operation question —
//! returns [`OperationReadiness::ConstraintsEstablished`], which says the *format facts* an
//! operation needs are known. It does not say Studio implements that operation, and there is no
//! variant that could. Implementation status lives in the evidence system, never here.
//!
//! # 5. Selection is bound to the source, not to the record
//!
//! Which variant a Build is planning is decided by [`crate::build_override`] from facts read off
//! the **current source**, never from the override record's own account of it. The record states
//! expectations, those are compared against the observation, and a contradicted or unconfirmable
//! expectation is refused. Nothing is ever inferred from the value being proposed.
//!
//! # This module implements no converter
//!
//! Nothing here reads, writes, decodes or encodes an asset. It is specification and validation
//! only, exactly like the database half.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::asset_resolution::{
    format_placeholders, is_part_name, ownership_sets, parse_render, CompositeIdentity,
    ResolutionChain, CHAIN_PURPOSE, CHAIN_REQUIREMENT_PREFIX, COMPOSITE_TOKEN, STEP_OWNERSHIP,
};
use crate::{
    check_identity, check_vocab, finding, ConstraintState, Finding, Rule, ASSET_PROFILE_SCHEMA,
    ASSET_PROFILE_SCHEMAS_NATIVE, ASSET_PROFILE_SCHEMA_LEGACY, ASSET_PROFILE_SCHEMA_PREVIOUS,
    ASSET_PROFILE_SCHEMA_V3, CANDIDATE_CONFIDENCE, OPERATIVE_CONFIDENCE,
};

// ============================================================= vocabularies

/// The 14 feature families of the QA matrix. A profile's `family` must be one of them, so a profile
/// can never register itself against a coverage axis that does not exist.
///
/// Mirrors `tdb_qa::catalog::FAMILIES`; a test asserts the two agree, so this list cannot drift
/// without failing. `tdb-canonical` deliberately does not depend on `tdb-qa` at build time — it is
/// a specification crate and reads no game data.
pub const ASSET_FAMILY: &[&str] = &[
    "db-presentation",
    "db-operations",
    "save-raw-db",
    "save-inner-container",
    "save-outer-package",
    "onboarding-install",
    "onboarding-localization",
    "assets-portraits",
    "assets-logos",
    "assets-jerseys",
    "assets-equipment",
    "assets-heads",
    "assets-arenas",
    "assets-audio",
];

/// Closed vocabulary: what kind of thing a canonical source is. It decides which constraint blocks
/// a target profile may even declare.
pub const ASSET_CLASS: &[&str] = &["raster", "mesh", "audio"];

/// Closed vocabulary: the canonical (non-destructive, high-fidelity) interchange representation.
pub const CANONICAL_INTERCHANGE: &[&str] =
    &["rgba8", "rgba16", "mesh-canonical", "pcm-s16", "pcm-f32"];

/// Which interchange representations belong to which asset class. A raster source declaring
/// `mesh-canonical` is a contradiction, not a preference.
pub const INTERCHANGE_CLASS: &[(&str, &str)] = &[
    ("rgba8", "raster"),
    ("rgba16", "raster"),
    ("mesh-canonical", "mesh"),
    ("pcm-s16", "audio"),
    ("pcm-f32", "audio"),
];

/// Closed vocabulary: what the pipeline is permitted to do to the canonical source before the
/// target conversion. Q14 forbids recrop/reframe/background work on portraits, so
/// `normalized-input` means *the library already normalised it*, never *Studio may normalise it*.
pub const SOURCE_RETENTION: &[&str] = &["unmodified", "normalized-input"];

/// The sixteen constraint blocks a target profile carries. Used by `[[operation]].requires` and by
/// the class-applicability rules.
///
/// `runtime_lookup` is new in `/3`. It is the block for *what the game loads*, as distinct from
/// `discovery` (where a scan finds files) and `install` (how bytes get written). C0-POR1-R1 found
/// the two conflated: an installed copy's file placement was read as a discovery rule while
/// `patch.big` - which could shadow those files entirely - had never been opened.
pub const ASSET_BLOCK: &[&str] = &[
    "discovery",
    "identity",
    "container",
    "path_rule",
    "raster",
    "dimensions",
    "mipmaps",
    "compression",
    "tiling",
    "atlas",
    "mesh_layout",
    "uv",
    "rig",
    "audio",
    "install",
    "runtime_lookup",
];

/// The blocks a `[[target_profile.variant]]` may carry, and which a target that declares variants
/// must therefore leave unknown at target level.
///
/// These four are exactly the correlated ones: a target's stored size, the codec and colour format
/// that store it, the mip layout over it and its memory layout move **together**, and which of them
/// applies is decided by the same selector - the path family among them. Modelling them as
/// independent per-target allowed-value lists is what would invent unobserved combinations.
///
/// The container chain is deliberately **not** here, because it is not always correlated: on the
/// portrait targets one chain holds every form, and forcing it into a variant would mean repeating
/// one corpus-wide invariant per variant and inviting the copies to drift apart. It is a
/// [`VARIANT_OPTIONAL_BLOCK`] instead - shared at target level *or* stated per variant, never both.
pub const VARIANT_BLOCK: &[&str] = &["raster", "dimensions", "mipmaps", "tiling"];

/// The blocks a variant **may** carry while the target may also carry them as a shared fact.
///
/// Unlike [`VARIANT_BLOCK`], declaring one here does not force the target-level block to `unknown`.
/// Exactly one of the two levels may decide it, which is what makes resolution deterministic:
/// a variant that decides the block wins for that variant, and a variant that leaves it `unknown`
/// reads the shared target-level fact. Both deciding it is a contradiction
/// ([`Rule::AssetVariantBlockConflict`]) rather than a precedence puzzle.
///
/// `container` is here because two forms of one family genuinely can arrive in different chains -
/// a loose file beside an archive member - while on the portrait targets they do not, and the
/// schema must be able to say either without duplicating a real fact.
pub const VARIANT_OPTIONAL_BLOCK: &[&str] = &["container"];

/// The blocks that say **where an asset sits**, as opposed to what its bytes are.
///
/// The distinction is what C0-AS3-R1 turns on. Decoding an extracted file establishes things about
/// that file; it establishes nothing about the path it had inside the archive it was pulled from,
/// which archive that was, or what the runtime loads - because the extraction is exactly the step
/// that discarded them. A corpus that cannot be attributed to the target may still support a
/// scoped measurement of its own specimens; it may never establish one of these without a
/// [`ProvenanceBridge`].
pub const PLACEMENT_BLOCK: &[&str] = &["discovery", "path_rule", "install", "runtime_lookup"];

/// Which asset class each block belongs to. A block with no entry here (`discovery`, `identity`,
/// `container`, `path_rule`, `compression`, `install`, `runtime_lookup`) is class-neutral: every
/// family has one.
pub const BLOCK_CLASS: &[(&str, &str)] = &[
    ("raster", "raster"),
    ("dimensions", "raster"),
    ("mipmaps", "raster"),
    ("tiling", "raster"),
    ("atlas", "raster"),
    ("mesh_layout", "mesh"),
    ("uv", "mesh"),
    ("rig", "mesh"),
    ("audio", "audio"),
];

/// Closed vocabulary: the operations a profile may state format requirements for.
///
/// `game-acceptance` is **absent on purpose**, and naming it is refused by
/// [`Rule::AdapterClaimsGameVerified`]: a schema file can never promote support.
/// The operations that put bytes into the target: those may require a resolution chain only if it
/// is declared for writing, and resolve only its owned step.
pub const WRITING_OPERATION: &[&str] = &["import", "replace", "rebuild", "install"];

pub const ASSET_OPERATION: &[&str] = &[
    "discover", "identify", "preview", "export", "import", "replace", "rebuild", "install",
];

/// Closed vocabulary: **how far the evidence behind a fact reaches**. Orthogonal to confidence,
/// which says how well a thing was measured rather than what population the measurement covers.
///
/// * `specimen-observation` - measured on named specimens only. Never grounds for a target-wide
///   constraint, however carefully the specimens were decoded.
/// * `inspected-corpus-invariant` - holds for **every** file inspected in the cited corpus, across
///   all of its groups. A property that holds only inside a group *defined* by that property is
///   circular and is not an invariant.
/// * `supported-target-constraint` - what the target itself requires, shown by something other than
///   reading the bytes one copy happens to carry.
pub const EVIDENCE_SCOPE: &[&str] = &[
    "specimen-observation",
    "inspected-corpus-invariant",
    "supported-target-constraint",
];

/// The scopes at which an established constraint is **operative** - readable as a fact about the
/// target, and therefore able to decide a Build.
///
/// A `specimen-observation` is excluded, and that exclusion is the whole of its meaning under
/// C0-AS3-R1. It may be *established*: "these four decoded specimens are DXT1" is a true, checkable
/// statement, and refusing to record it at all was AS3 over-correcting. What it may never be is a
/// statement about the target - so [`AssetConstraint::operative`] does not return it,
/// [`block_states`] reports it as non-operative, no operation's requirement is met by it, and no
/// Build reads it.
pub const OPERATIVE_SCOPE: &[&str] = &["inspected-corpus-invariant", "supported-target-constraint"];

/// Closed vocabulary: what an established constraint asserts about **other** values.
///
/// * `observed-source-format` - this is what the inspected source carries. It authorises
///   source-preserving planning (reproduce what is there) and nothing else. It does **not** prove
///   another format is forbidden, and it does not prove another format is supported.
/// * `target-required` - the target requires this value; another value is refused.
///
/// Only a `supported-target-constraint` may be `target-required`: reading the bytes a copy ships
/// cannot establish what the target rejects.
pub const CONSTRAINT_FORCE: &[&str] = &["observed-source-format", "target-required"];

/// The (scope, force) pairs that are coherent. `target-required` under anything weaker than a
/// supported target constraint is refused by [`Rule::AssetEvidenceScopeInsufficient`], and a
/// `specimen-observation` can only ever record what its specimens carry.
pub const SCOPE_FORCE: &[(&str, &str)] = &[
    ("specimen-observation", "observed-source-format"),
    ("inspected-corpus-invariant", "observed-source-format"),
    ("supported-target-constraint", "observed-source-format"),
    ("supported-target-constraint", "target-required"),
];

/// Closed vocabulary: what kind of thing an inspected corpus is.
pub const CORPUS_KIND: &[&str] = &[
    "observed-installed-copy",
    "derived-tree",
    "extracted-tree",
    "synthetic-fixture",
];

/// Closed vocabulary: whether a corpus can be attributed to the target it was inspected for.
///
/// A derived or extracted tree is `not-attributable` by construction - it is the output of an
/// extraction or a rebuild, not something the title ships - so
/// [`Rule::AssetCorpusAttributionUnsupported`] refuses the claim rather than leaving it to prose.
///
/// Not attributable is **not** the same as worthless, and C0-AS3-R1 separates the two. An
/// unattributable corpus may support a `specimen-observation` about its own files, which is a fact
/// about those files and is never operative. What it may not do without a [`ProvenanceBridge`] is
/// establish a [`PLACEMENT_BLOCK`] - where the asset sat in the archive it came from - or anything
/// at a scope that speaks for the target.
pub const CORPUS_ATTRIBUTION: &[&str] = &["attributable-to-target", "not-attributable"];

/// Closed vocabulary: how a [`ProvenanceBridge`] ties an unattributable corpus back to the shipped
/// original.
///
/// Both terms name a **record that survives the extraction**: a manifest written by the extractor
/// that says which archive member each file came from, or a byte-identity check against the member
/// still sitting in an attributable corpus. A folder name, a file name and a tool's default output
/// layout are none of these, and there is no term for them.
pub const BRIDGE_METHOD: &[&str] = &[
    "archive-member-manifest",
    "byte-identical-to-archive-member",
];

/// Closed vocabulary: whether the exact build behind a corpus was identified. A title id plus a
/// version field is not a build.
pub const BUILD_IDENTIFICATION: &[&str] = &["complete", "incomplete"];

/// The corpus kinds that can never be attributed to the shipped target.
pub const UNATTRIBUTABLE_CORPUS_KIND: &[&str] = &["derived-tree", "extracted-tree"];

/// Closed vocabulary: the fact a selector clause tests. Every clause is a **structured** predicate
/// over an observable property of the asset being planned, never a free-text pattern.
pub const SELECTOR_DIMENSION: &[&str] = &[
    "path-family",
    "bucket",
    "numeric-id-range",
    "stored-dimensions",
];

/// Closed vocabulary: whether an authoritative id-to-directory rule is established for a path
/// family.
///
/// `unknown` is the honest state for the portrait families: only bucket **membership** was observed,
/// and a folder label is not a function. `template-only` means the template resolves a path by
/// itself, with no derived directory involved.
pub const BUCKET_RESOLUTION: &[&str] = &["unknown", "template-only", "derived-directory"];

/// Closed vocabulary: where the **runtime** reads the asset from.
pub const RUNTIME_SOURCE: &[&str] = &["loose-file", "archive-member", "patch-member"];

/// Closed vocabulary: which copy the runtime prefers when a base and a patch both carry the asset.
pub const PATCH_PRECEDENCE: &[&str] =
    &["no-patch-layer", "patch-shadows-base", "base-shadows-patch"];

/// Closed vocabulary: whether the family shares a surface **between assets**. Measurable by counting
/// texture payloads per container, which is a different and much weaker fact than knowing whether
/// the family uses an atlas, a pool or neither.
pub const ATLAS_SHARING: &[&str] = &["per-asset-surface", "shared-surface"];

/// The sub-properties a block may leave unknown **while still being established**, with the block
/// they belong to. `[<block>.property.<name>]` may only name one of these.
///
/// These are the fields that could not be measured at all rather than the fields that happen to be
/// absent: a mip filter or a tiling alignment was always an optional refinement, and neither blocks
/// anything by being missing. A colour space nobody can read, and an atlas kind a payload count does
/// not answer, are different - they are gaps, and an operation that needs one must be blocked.
pub const OPTIONAL_BLOCK_PROPERTY: &[(&str, &str)] = &[
    ("raster", "color_space"),
    ("raster", "alpha"),
    ("atlas", "kind"),
];

/// Every sub-property an `[[operation]].requires` entry may narrow to, which is the optional ones
/// plus the ones a value always carries.
///
/// Naming a **required** sub-property is not redundant: it says what the operation actually depends
/// on, so a later revision that makes that field optional narrows the operation rather than silently
/// widening it. `raster.codec` is the case that matters - `preview` needs the codec and does not
/// care about the colour space, and writing that down is what keeps an unmeasurable colour space
/// from blocking a preview.
pub const REQUIRABLE_BLOCK_PROPERTY: &[(&str, &str)] = &[
    ("raster", "codec"),
    ("raster", "channels"),
    ("raster", "color_space"),
    ("raster", "alpha"),
    ("atlas", "sharing"),
    ("atlas", "kind"),
    ("mipmaps", "policy"),
    ("dimensions", "width"),
    ("dimensions", "height"),
    ("tiling", "kind"),
];

/// Closed vocabulary: where a target's assets are found.
pub const DISCOVERY_SCOPE: &[&str] = &["loose-tree", "archive-member", "nested-archive-member"];

/// Closed vocabulary: what identifies one asset instance.
pub const IDENTITY_KEY: &[&str] = &[
    "player-slug",
    "numeric-id",
    "archive-entry-path",
    "archive-entry-index",
    "composite",
];

/// Closed vocabulary: what happens when a source matches more than one target asset. *"Never
/// silently bind an ambiguous match"* (C4), so `first-match` is not a legal resolution.
pub const AMBIGUITY_POLICY: &[&str] = &["reject", "manual-resolution"];

/// Closed vocabulary: one container layer's format.
pub const CONTAINER_FORMAT: &[&str] = &[
    "loose-file",
    "bigf",
    "big4",
    "eb3",
    "rx2",
    "rw4",
    "xpr2",
    "gnf",
    "dds",
];

/// Closed vocabulary: how a container layer addresses its members.
pub const INDEX_KIND: &[&str] = &["none", "name-table", "offset-table", "chunk-table"];

/// Closed vocabulary: the substitutable parts of an on-disk path. `composite-id` (`/4`) is the
/// rendered `[identity.value.composite]` of the same target, and a template may use it only when
/// that composite is declared.
pub const PATH_TOKEN: &[&str] = &[
    "player-slug",
    "numeric-id",
    "team-id",
    "variant",
    COMPOSITE_TOKEN,
];

/// Closed vocabulary: path case handling.
pub const PATH_CASE: &[&str] = &["lower", "upper", "preserve"];

/// Closed vocabulary: the encoded pixel format the target stores.
pub const RASTER_CODEC: &[&str] = &[
    "rgba8", "bgra8", "dxt1", "dxt3", "dxt5", "bc4", "bc5", "bc7", "l8", "a8",
];

/// The block-compressed codecs, and the pixel block they quantise over. A block-compressed target
/// constrains the dimensions it can store, which is checkable the moment both blocks are
/// established.
pub const RASTER_BLOCK_PIXELS: &[(&str, u32)] = &[
    ("dxt1", 4),
    ("dxt3", 4),
    ("dxt5", 4),
    ("bc4", 4),
    ("bc5", 4),
    ("bc7", 4),
];

/// Closed vocabulary: colour space of the stored pixels.
pub const COLOR_SPACE: &[&str] = &["linear", "srgb"];

/// Closed vocabulary: a stored channel.
pub const RASTER_CHANNEL: &[&str] = &["r", "g", "b", "a"];

/// Closed vocabulary: how alpha is stored.
pub const ALPHA_MODE: &[&str] = &["none", "straight", "premultiplied"];

/// Closed vocabulary: the target's mip requirement.
pub const MIPMAP_POLICY: &[&str] = &["none", "base-only", "full-chain", "fixed-count"];

/// Closed vocabulary: the filter a target's mip chain was built with, where that is established.
pub const MIPMAP_FILTER: &[&str] = &["box", "kaiser", "lanczos"];

/// Closed vocabulary: the payload (stream) compression wrapped around the stored bytes. Distinct
/// from the pixel codec, which is [`RASTER_CODEC`].
pub const PAYLOAD_CODEC: &[&str] = &["none", "refpack", "chunkzip", "lzx", "zlib"];

/// Closed vocabulary: memory layout of the stored surface.
pub const TILING_KIND: &[&str] = &["linear", "xbox360-tiled", "ps3-swizzled", "gnf-tiled"];

/// Closed vocabulary: whether the family shares a surface, and how.
pub const ATLAS_KIND: &[&str] = &["none", "atlas", "pool"];

/// Closed vocabulary: a vertex attribute's meaning.
pub const VERTEX_SEMANTIC: &[&str] = &[
    "position",
    "normal",
    "tangent",
    "uv",
    "color",
    "blend_index",
    "blend_weight",
];

/// Closed vocabulary: a vertex component's storage type, with its byte size.
pub const COMPONENT_TYPE: &[(&str, u32)] = &[
    ("f32", 4),
    ("f16", 2),
    ("u8", 1),
    ("i8", 1),
    ("u16", 2),
    ("i16", 2),
    ("u8-normalized", 1),
    ("i8-normalized", 1),
];

/// Closed vocabulary: index buffer width.
pub const INDEX_FORMAT: &[&str] = &["u16", "u32"];

/// Closed vocabulary: triangle winding.
pub const WINDING: &[&str] = &["cw", "ccw"];

/// Closed vocabulary: handedness and up-axis of the target's mesh space.
pub const COORDINATE_SYSTEM: &[&str] = &["y-up-rh", "y-up-lh", "z-up-rh", "z-up-lh"];

/// Closed vocabulary: the unit one mesh coordinate means.
pub const MESH_UNIT: &[&str] = &["metre", "centimetre", "millimetre", "game-unit"];

/// Closed vocabulary: where UV (0,0) sits.
pub const UV_ORIGIN: &[&str] = &["top-left", "bottom-left"];

/// Closed vocabulary: addressing outside 0..1.
pub const UV_WRAP: &[&str] = &["clamp", "repeat", "mirror"];

/// Closed vocabulary: how a skin weight is stored.
pub const WEIGHT_QUANTIZATION: &[&str] = &["u8-normalized", "u16-normalized", "f32"];

/// Closed vocabulary: the space the bind pose is expressed in.
pub const BIND_POSE: &[&str] = &["model-space", "bone-local"];

/// Closed vocabulary: whether clips are addressed individually or through a bank index.
pub const AUDIO_BANK: &[&str] = &["loose-stream", "indexed-bank"];

/// Closed vocabulary: the encoded audio codec.
pub const AUDIO_CODEC: &[&str] = &[
    "pcm-s16",
    "ima-adpcm",
    "ea-xas",
    "xma2",
    "atrac9",
    "mp3",
    "vorbis",
];

/// The codecs whose sample width is a declared property rather than an artefact of the codec.
pub const AUDIO_BIT_DEPTH_CODECS: &[&str] = &["pcm-s16"];

/// Closed vocabulary: what the target carries beside the samples, and what a replacement must do
/// with it. C7 requires existing cue/order/loop metadata to survive a clip replacement.
pub const LOOP_METADATA: &[&str] = &["absent", "preserved-opaque", "structured"];

/// Closed vocabulary: how a changed asset reaches the target.
pub const INSTALL_KIND: &[&str] = &[
    "loose-file",
    "archive-replace-in-place",
    "archive-append-relocate",
    "archive-rebuild",
];

/// The install kinds that write into an archive, and therefore need an established `[container]`.
pub const ARCHIVE_INSTALL_KIND: &[&str] = &[
    "archive-replace-in-place",
    "archive-append-relocate",
    "archive-rebuild",
];

/// Closed vocabulary: whether the encoded replacement must fit the original span (C3a).
pub const SIZE_POLICY: &[&str] = &["fit-in-slot", "allow-growth"];

/// Closed vocabulary: what rebuilding the container costs.
pub const REPACK_RULE: &[&str] = &["not-required", "index-update", "full-rebuild"];

/// Closed vocabulary: what the install path must retain for restore (C3 item 4).
pub const BACKUP_RULE: &[&str] = &["original-bytes-retained", "whole-container-retained"];

// ============================================== the Q07a.1 asset representation map

/// Every **asset** constraint Q07a.1 names, mapped to where `asset-output-profile/2` represents it.
///
/// The source sentence is the roadmap's: *"Asset profiles record discovery and identity rules,
/// archives/indexes, paths/slugs/IDs, raster formats, dimensions, channels, mipmaps, compression,
/// swizzle/tiling, atlases/pools, mesh vertex layouts, UV conventions, rigs/bones/weights/palettes/
/// LODs, audio banks/codecs/sample constraints, and repack/install rules. Facts carry fixture
/// provenance, confidence and automated/manual qualification."*
///
/// A test asserts each entry actually parses out of a document. It says nothing about how many
/// (family, target) pairs **declare** these — see [`asset_registry_population`].
pub const Q07A1_ASSET_REPRESENTATION: &[(&str, &str)] = &[
    ("discovery rules", "[target_profile.discovery]"),
    (
        "identity rules",
        "[target_profile.identity] (+ its ambiguity policy)",
    ),
    (
        "archives",
        "[target_profile.container].value.layer.format — one entry per nesting level",
    ),
    ("indexes", "[target_profile.container].value.layer.index"),
    (
        "paths",
        "[[target_profile.path_family]].template + [target_profile.path_rule].value.families",
    ),
    (
        "slugs",
        "[[target_profile.path_family]].tokens = \"player-slug\", bound to [identity].key",
    ),
    (
        "IDs",
        "[[target_profile.path_family]].tokens = \"numeric-id\"/\"team-id\", with observed \
         directory membership in [[observed_bucket]] and the authoritative rule in \
         `bucket_resolution`",
    ),
    (
        "raster formats",
        "[target_profile.raster].value.codec, or [[target_profile.variant]].raster where a target \
         carries several correlated forms",
    ),
    (
        "dimensions",
        "[target_profile.dimensions].value.width/height, or [[target_profile.variant]].dimensions \
         correlated with that variant's codec, mip layout, tiling and path family",
    ),
    (
        "channels",
        "[target_profile.raster].value.channels, with alpha and colour space optional and their \
         absence explained in [raster.property.<name>]",
    ),
    (
        "mipmaps",
        "[target_profile.mipmaps].value.policy/count/filter",
    ),
    (
        "compression",
        "[target_profile.compression].value.payload — the stream wrapper, distinct from the pixel codec",
    ),
    ("swizzle/tiling", "[target_profile.tiling].value.kind"),
    (
        "atlases/pools",
        "[target_profile.atlas].value.sharing, with the atlas/pool kind and its page/cell geometry \
         optional",
    ),
    (
        "mesh vertex layouts",
        "[target_profile.mesh_layout].value.vertex_attribute[] + stride/index_format/winding",
    ),
    (
        "UV conventions",
        "[target_profile.uv].value.origin/wrap/channel_count",
    ),
    ("rigs/bones", "[target_profile.rig].value.bone_count"),
    (
        "weights",
        "[target_profile.rig].value.max_weights_per_vertex/weight_quantization",
    ),
    ("palettes", "[target_profile.rig].value.palette_size"),
    ("LODs", "[target_profile.mesh_layout].value.lod_levels"),
    ("audio banks", "[target_profile.audio].value.bank"),
    ("audio codecs", "[target_profile.audio].value.codec"),
    (
        "audio sample constraints",
        "[target_profile.audio].value.sample_rate_hz/channels/bit_depth/max_duration_ms",
    ),
    ("repack rules", "[target_profile.install].value.repack"),
    (
        "install rules",
        "[target_profile.install].value.kind/size_policy/backup",
    ),
    (
        "fixture provenance, confidence and qualification",
        "every block is an AssetConstraint<T>: state + confidence + EVIDENCE SCOPE + CONSTRAINT \
         FORCE + the [[corpus]] it rests on + provenance + derivation + candidate",
    ),
];

// ======================================================= the asset evidence envelope

/// **Non-operative** candidate information for an asset constraint that is *not* established.
///
/// Mirrors [`crate::Candidate`] and adds the two `/3` scope keys, so a proposal can say how far its
/// own evidence reaches without that reach ever becoming operative. Nothing reads a candidate:
/// [`AssetConstraint::established`] never returns one.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
pub struct AssetCandidate<T> {
    /// How well the *candidate* is known. `asserted` is legal here - that is the point of the
    /// channel - and is not legal on an established constraint.
    #[serde(default)]
    pub confidence: Option<String>,
    /// One of [`EVIDENCE_SCOPE`]. Optional: a proposal may predate any scoping, and a candidate
    /// drives nothing either way.
    #[serde(default)]
    pub scope: Option<String>,
    /// The [`Corpus`] id the proposal rests on, where it rests on one.
    #[serde(default)]
    pub corpus: Option<String>,
    #[serde(default)]
    pub provenance: Vec<String>,
    /// What would have to be shown to promote this candidate to an established constraint.
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub value: Option<T>,
}

/// The evidence a block records about **one sub-property it does not know**.
///
/// `/2` bundled every field of a value together, so one unmeasurable field blocked the whole block -
/// and the temptation was to fill it. `/3` lets the value omit it and lets this table say *why*,
/// with its own citation and its own proposal, at the granularity the evidence actually has.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PropertyEvidence {
    /// Always `unknown`: a sub-property that is known lives in the block's `value`, not here.
    pub state: ConstraintState,
    /// How the absence was determined. Required - silence is not a reason.
    pub reason: String,
    /// One of [`EVIDENCE_SCOPE`], where something *was* observed about it.
    #[serde(default)]
    pub scope: Option<String>,
    /// One of [`CANDIDATE_CONFIDENCE`].
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub corpus: Option<String>,
    #[serde(default)]
    pub provenance: Vec<String>,
    #[serde(default)]
    pub note: String,
    /// A proposed value for this sub-property, recorded as text and **non-operative** - nothing
    /// reads it, exactly like a block candidate.
    #[serde(default)]
    pub candidate_value: Option<String>,
}

/// One evidence-qualified asset constraint.
///
/// Everything [`crate::Constraint`] carries, plus the three keys that make `/3`'s scope discipline
/// checkable: `scope`, `force` and `corpus`. The database half is deliberately untouched - a
/// `target-adapter/2` document still parses exactly as it did, and cannot spell these keys.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
pub struct AssetConstraint<T> {
    pub state: ConstraintState,
    /// Required when `state = "established"`, and then restricted to
    /// [`crate::OPERATIVE_CONFIDENCE`].
    #[serde(default)]
    pub confidence: Option<String>,
    /// One of [`EVIDENCE_SCOPE`]. Required when established, and then restricted to
    /// [`OPERATIVE_SCOPE`].
    #[serde(default)]
    pub scope: Option<String>,
    /// One of [`CONSTRAINT_FORCE`]. Required when established. It decides whether a Build may be
    /// refused a different value or merely blocked from one.
    #[serde(default)]
    pub force: Option<String>,
    /// The [`Corpus`] id this fact rests on. Required when established, and the corpus must be
    /// attributable to the target.
    #[serde(default)]
    pub corpus: Option<String>,
    #[serde(default)]
    pub provenance: Vec<String>,
    /// Required when `state = "not-applicable"` - how the absence was determined.
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub note: String,
    /// Required when `confidence = "derived"`: the premises this conclusion rests on, why each is
    /// applicable to THIS target, and what the derivation does not establish.
    #[serde(default)]
    pub derivation: String,
    #[serde(default)]
    pub value: Option<T>,
    /// A proposed value, recorded but not operative. Only legal under `state = "unknown"`.
    #[serde(default)]
    pub candidate: Option<AssetCandidate<T>>,
    /// Per-sub-property evidence, keyed by the property name. Only for properties this block's
    /// value leaves unknown.
    #[serde(default, rename = "property")]
    pub properties: BTreeMap<String, PropertyEvidence>,
}

impl<T> Default for AssetConstraint<T> {
    fn default() -> Self {
        AssetConstraint {
            state: ConstraintState::Unknown,
            confidence: None,
            scope: None,
            force: None,
            corpus: None,
            provenance: Vec::new(),
            reason: String::new(),
            note: String::new(),
            derivation: String::new(),
            value: None,
            candidate: None,
            properties: BTreeMap::new(),
        }
    }
}

impl<T> AssetConstraint<T> {
    /// The payload, **only** when the constraint is actually established. The single accessor every
    /// derivation goes through, and it deliberately cannot see a candidate.
    pub fn established(&self) -> Option<&T> {
        match self.state {
            ConstraintState::Established => self.value.as_ref(),
            _ => None,
        }
    }

    /// The proposed, **non-operative** value, if one is recorded.
    pub fn candidate_value(&self) -> Option<&T> {
        self.candidate.as_ref()?.value.as_ref()
    }

    /// The payload when the constraint is established **and its evidence speaks for the target**.
    ///
    /// The accessor every Build decision goes through. [`established`](Self::established) answers
    /// "is this block decided", which is what a bounds check wants; this answers "may a Build read
    /// it as a fact about the target", which is a strictly narrower question. A block established
    /// at `specimen-observation` scope is decided and is **not** operative: it records what some
    /// named files carry, and no question about the target is answered by it.
    pub fn operative(&self) -> Option<&T> {
        let scope = self.scope.as_deref()?;
        if !OPERATIVE_SCOPE.contains(&scope) {
            return None;
        }
        self.established()
    }

    /// Whether the block is decided at a scope that speaks for the target. True for an operative
    /// established block **and** for a decided absence, which is a target-level judgement with no
    /// value to scope.
    pub fn is_operative(&self) -> bool {
        match self.state {
            ConstraintState::Established => self.operative().is_some(),
            ConstraintState::NotApplicable => true,
            ConstraintState::Unknown => false,
        }
    }

    /// Whether an established constraint refuses a different value, rather than merely failing to
    /// authorise one. False for anything not established, and false for an observed source format.
    pub fn is_target_required(&self) -> bool {
        self.state == ConstraintState::Established
            && self.force.as_deref() == Some("target-required")
    }
}

// ================================================================== documents

/// `defs/adapters/asset-profiles/<family>.toml`, schema `asset-output-profile/3`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AssetProfile {
    pub schema: String,
    /// The document's own revision. Not the schema version.
    pub profile_version: u32,
    /// One of [`ASSET_FAMILY`].
    pub family: String,
    /// A **labelled synthetic fixture**: it exercises representations for which no real fact is
    /// established. Excluded from [`asset_registry_population`], exactly as for adapters.
    #[serde(default)]
    pub synthetic: bool,
    #[serde(default)]
    pub note: String,
    /// What the Project holds. At least one is required: the classes declared here decide which
    /// constraint blocks a target profile may carry at all.
    #[serde(default, rename = "canonical_source")]
    pub canonical_sources: Vec<CanonicalSource>,
    #[serde(default, rename = "target_profile")]
    pub targets: Vec<TargetProfile>,
}

/// One canonical input the family is built from. A head family has a mesh *and* a texture; a
/// portrait family has one raster.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSource {
    /// Unique within the profile, e.g. `portrait`, `head-mesh`, `head-texture`.
    pub role: String,
    /// One of [`ASSET_CLASS`].
    pub class: String,
    /// One of [`CANONICAL_INTERCHANGE`], and it must belong to `class`.
    pub interchange: String,
    /// One of [`SOURCE_RETENTION`].
    pub retention: String,
    #[serde(default)]
    pub description: String,
}

/// One inspected body of files, with what it is and what it can therefore support.
///
/// A corpus is **evidence identity**, kept apart from the facts read off it. Two corpora can carry
/// byte-identical files and still support entirely different claims, because one is the title's own
/// installed directory and the other is somebody's extraction.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    /// Unique within the target. Blocks cite it by this id.
    pub id: String,
    /// One of [`CORPUS_KIND`].
    pub kind: String,
    /// One of [`CORPUS_ATTRIBUTION`].
    pub attribution: String,
    /// One of [`BUILD_IDENTIFICATION`]. `incomplete` must be accompanied by at least one
    /// `limitation`, so the gap is stated rather than implied.
    pub build_identification: String,
    /// How the corpus's own identity was read - a container header, an SFO, an executable header.
    /// Never a folder label.
    pub identity_source: String,
    /// How many files were inspected. Zero is refused: a corpus nobody looked at supports nothing.
    pub inspected_files: u32,
    /// The neutral group labels the inspection distinguished (`current`, `backup-labelled`, ...).
    /// An invariant must hold across **all** of them.
    #[serde(default)]
    pub groups: Vec<String>,
    /// What this corpus cannot support, stated rather than left to the reader.
    #[serde(default)]
    pub limitations: Vec<String>,
    #[serde(default)]
    pub provenance: Vec<String>,
    /// How an unattributable corpus is tied back to the shipped original, where such a record
    /// exists. Absent on every real corpus in this repository: no extraction here kept one.
    #[serde(default)]
    pub bridge: Option<ProvenanceBridge>,
    #[serde(default)]
    pub note: String,
}

/// A record that survives an extraction and ties its output back to the archive it came from.
///
/// Without one, an extracted tree is a bag of files: measurable, but silent about where its members
/// sat and which archive held them. With one, the placement facts the extraction discarded are
/// recoverable, and the corpus may support them. The bridge names the attributable corpus it
/// bridges to and cites where the tie is recorded.
///
/// # What validating a bridge does and does not do
///
/// C0-AS3-R2 states this plainly, because "the bridge is evidence, not a label" reads as a stronger
/// guarantee than the code gives. What [`validate_asset_profile`] checks is that the **declaration**
/// is well formed: a `method` in [`BRIDGE_METHOD`], an `original_corpus` that is a declared and
/// genuinely attributable corpus on this target, a non-empty `provenance`, and no self-bridge or
/// bridge onto a corpus that already claims attribution. All four are structural.
///
/// It does **not**:
///
/// * open, read or verify anything the `provenance` points at. A manifest that says what it is
///   supposed to say and a manifest that says nothing are the same string here;
/// * check that the extracted files are the members that manifest names, or that any byte-identity
///   claim holds;
/// * establish what the **runtime** loads, or which copy wins when a base and a patch both carry an
///   asset. That is `[runtime_lookup]`, it is a separate block, and a bridge does not touch it. A
///   file's original archive placement and the runtime's lookup precedence are different questions,
///   and knowing the first answers nothing about the second.
///
/// So a valid bridge means the profile has *declared* a tie in the form the schema requires. Whether
/// the tie is real is a matter for separately reviewed evidence linking that specific source corpus
/// to that specific extracted material, and populating a real bridge is authoring against such
/// evidence rather than filling in fields. No real corpus in this repository declares one, and none
/// is authorized by any package to date.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceBridge {
    /// One of [`BRIDGE_METHOD`].
    pub method: String,
    /// The id of a corpus on this target that **is** attributable, and which this one bridges to.
    pub original_corpus: String,
    /// Non-empty: an uncited bridge is an assertion, and an assertion cannot carry attribution.
    #[serde(default)]
    pub provenance: Vec<String>,
    #[serde(default)]
    pub note: String,
}

/// One observed on-disk path shape. A target may have several: portraits on NHL Legacy have two.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PathFamily {
    /// Unique within the target, and the value a `path-family` selector clause matches.
    pub id: String,
    /// `{token}` placeholders plus literal text.
    pub template: String,
    /// Tokens from [`PATH_TOKEN`]. Every `{...}` in `template` must appear here and vice versa.
    #[serde(default)]
    pub tokens: Vec<String>,
    /// One of [`PATH_CASE`].
    pub case: String,
    /// One of [`BUCKET_RESOLUTION`] - whether an **authoritative** id-to-directory rule is known.
    /// Observed bucket membership below is not one, and never promotes this.
    pub bucket_resolution: String,
    #[serde(default, rename = "observed_bucket")]
    pub observed_buckets: Vec<ObservedBucket>,
    #[serde(default)]
    pub note: String,
}

/// A directory that was **observed** to exist, and a pointer to the census that holds which ids
/// were seen inside it.
///
/// The detailed membership does not live here, and C0-AS3-R1 moved it out deliberately. A list of
/// id ranges in a profile reads as a lookup table - the next reader treats "1..999 live in `000`"
/// as the rule for finding id 500, which is exactly the inference nothing supports. What the
/// profile keeps is the *reference*: which census recorded the membership, at which digest, and how
/// far that observation reaches. A reader who needs the ids goes to the census, where they are
/// labelled as an observation and stay one.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservedBucket {
    pub directory: String,
    /// The census record holding the observed membership.
    pub census: String,
    /// Lowercase hex digest of that record, so the reference names a fixed document rather than
    /// whatever the path happens to hold today.
    pub census_digest: String,
    /// One of [`EVIDENCE_SCOPE`], and never `supported-target-constraint`: membership is something
    /// somebody saw in a corpus, not something the target requires.
    pub scope: String,
    /// The corpus the membership was observed in.
    pub corpus: String,
    #[serde(default)]
    pub note: String,
}

/// One correlated output form a target carries.
///
/// The four [`VARIANT_BLOCK`] constraints live **here**, together, so a Build can never pair one
/// variant's dimensions with another's codec: the combination is not spellable. A
/// [`VARIANT_OPTIONAL_BLOCK`] may join them where the forms genuinely differ in it.
#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct AssetVariant {
    /// Unique within the target.
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub note: String,
    /// Non-empty: a variant nothing can select is not a variant.
    #[serde(default, rename = "selector")]
    pub selectors: Vec<SelectorClause>,

    #[serde(default)]
    pub raster: AssetConstraint<RasterValue>,
    #[serde(default)]
    pub dimensions: AssetConstraint<DimensionsValue>,
    #[serde(default)]
    pub mipmaps: AssetConstraint<MipmapValue>,
    #[serde(default)]
    pub tiling: AssetConstraint<TilingValue>,
    /// This form's own container chain, where the forms do not share one. A
    /// [`VARIANT_OPTIONAL_BLOCK`]: leaving it `unknown` reads the target-level `[container]`, and
    /// deciding it while the target also decides one is refused.
    #[serde(default)]
    pub container: AssetConstraint<ContainerValue>,
}

/// One structured predicate over an observable property of the asset being planned.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectorClause {
    /// One of [`SELECTOR_DIMENSION`].
    pub dimension: String,
    /// The operand for `path-family` and `bucket`.
    #[serde(default)]
    pub equals: Option<String>,
    /// Inclusive bounds for `numeric-id-range`.
    #[serde(default)]
    pub id_low: Option<u32>,
    #[serde(default)]
    pub id_high: Option<u32>,
    /// The operands for `stored-dimensions`.
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

/// One target's output constraints. Every block defaults to `unknown`, so an absent block is never
/// a fact - and never an absence either.
#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct TargetProfile {
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub note: String,

    /// The inspected corpora every established block on this target must cite.
    #[serde(default, rename = "corpus")]
    pub corpora: Vec<Corpus>,
    /// The observed path families. Several are allowed; the portrait targets have two.
    #[serde(default, rename = "path_family")]
    pub path_families: Vec<PathFamily>,
    /// The correlated output forms. When any is declared, the four [`VARIANT_BLOCK`] constraints
    /// must be `unknown` at target level - a target cannot describe its geometry twice. A
    /// [`VARIANT_OPTIONAL_BLOCK`] may be decided at either level, but never at both.
    #[serde(default, rename = "variant")]
    pub variants: Vec<AssetVariant>,
    /// `/4`: ordered reading/writing orders over the declared path families, each in its own
    /// evidence envelope. An unknown or specimen-scoped chain enables nothing.
    #[serde(default, rename = "resolution_chain")]
    pub resolution_chains: Vec<ResolutionChain>,

    #[serde(default)]
    pub discovery: AssetConstraint<DiscoveryValue>,
    #[serde(default)]
    pub identity: AssetConstraint<IdentityValue>,
    #[serde(default)]
    pub container: AssetConstraint<ContainerValue>,
    #[serde(default)]
    pub path_rule: AssetConstraint<PathRuleValue>,
    #[serde(default)]
    pub raster: AssetConstraint<RasterValue>,
    #[serde(default)]
    pub dimensions: AssetConstraint<DimensionsValue>,
    #[serde(default)]
    pub mipmaps: AssetConstraint<MipmapValue>,
    #[serde(default)]
    pub compression: AssetConstraint<CompressionValue>,
    #[serde(default)]
    pub tiling: AssetConstraint<TilingValue>,
    #[serde(default)]
    pub atlas: AssetConstraint<AtlasValue>,
    #[serde(default)]
    pub mesh_layout: AssetConstraint<MeshLayoutValue>,
    #[serde(default)]
    pub uv: AssetConstraint<UvValue>,
    #[serde(default)]
    pub rig: AssetConstraint<RigValue>,
    #[serde(default)]
    pub audio: AssetConstraint<AudioValue>,
    #[serde(default)]
    pub install: AssetConstraint<InstallValue>,
    #[serde(default)]
    pub runtime_lookup: AssetConstraint<RuntimeLookupValue>,

    /// Which format constraints each operation needs. **Not** a statement that Studio implements
    /// the operation - there is no key for that here, and no variant of [`OperationReadiness`]
    /// that could say it.
    #[serde(default, rename = "operation")]
    pub operations: Vec<OperationDecl>,
}

/// The format constraints one operation depends on.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperationDecl {
    /// One of [`ASSET_OPERATION`].
    pub operation: String,
    /// Block ids from [`ASSET_BLOCK`], optionally narrowed to one sub-property as
    /// `<block>.<property>` from [`OPTIONAL_BLOCK_PROPERTY`]. Non-empty and unique.
    ///
    /// The narrowing is what keeps an unknown sub-property from blocking everything: `preview` needs
    /// `raster.codec`, and does not care that the colour space was never recorded.
    pub requires: Vec<String>,
    #[serde(default)]
    pub note: String,
}

// ========================================================= constraint payloads

/// Where the target's assets live and how a scan finds them.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryValue {
    /// One of [`DISCOVERY_SCOPE`].
    pub scope: String,
    /// The install-relative root a scan starts from.
    pub root: String,
    /// Whether the scan descends below `root`.
    #[serde(default)]
    pub recursive: bool,
}

/// What identifies one asset instance, and what happens when that is ambiguous.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IdentityValue {
    /// One of [`IDENTITY_KEY`].
    pub key: String,
    /// The database table/field or archive coordinate the key is read from.
    pub source: String,
    /// One of [`AMBIGUITY_POLICY`].
    pub ambiguity: String,
    /// `/4`: how a `key = "composite"` id is built from named fields. Required exactly when the key
    /// is `composite`; refused under any other key.
    #[serde(default)]
    pub composite: Option<CompositeIdentity>,
}

/// The container nesting, outermost first, with how each layer indexes its members.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContainerValue {
    #[serde(default, rename = "layer")]
    pub layers: Vec<ContainerLayer>,
    /// Whether the outermost declared layer is known to be the outermost layer that exists.
    ///
    /// C0-POR1 measured `GNF ` files pulled out of a `.big`, so at least one outer layer was known
    /// to exist and to be unexamined. `/2` had nowhere to say that, and the chain read as complete.
    #[serde(default)]
    pub outermost_layer_examined: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContainerLayer {
    /// One of [`CONTAINER_FORMAT`].
    pub format: String,
    /// One of [`INDEX_KIND`].
    pub index: String,
    #[serde(default)]
    pub note: String,
}

/// How an asset id resolves to a path, through the target's declared [`PathFamily`] entries.
///
/// The families and their observed buckets are recorded whether or not this block is established -
/// observation and authoritative resolution are separate things, and this block is the second one.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PathRuleValue {
    /// The declared `[[path_family]]` ids this target resolves through. Non-empty, each declared.
    pub families: Vec<String>,
    /// How a family is chosen for a given asset. Every named family must additionally have a
    /// `bucket_resolution` other than `unknown`, or the block cannot be established.
    pub family_selection: String,
}

/// The stored pixel format and its channel semantics.
///
/// `color_space` and `alpha` are **optional**: `DDS` and `XPR2` record no colour space at all, and
/// alpha mode is a decode-time observation. Absent means unknown, and
/// `[raster.property.<name>]` carries the reason.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RasterValue {
    /// One of [`RASTER_CODEC`].
    pub codec: String,
    /// Channels from [`RASTER_CHANNEL`], unique and non-empty.
    pub channels: Vec<String>,
    /// One of [`COLOR_SPACE`], where the target records one at all.
    #[serde(default)]
    pub color_space: Option<String>,
    /// One of [`ALPHA_MODE`], where it was established rather than observed on specimens.
    #[serde(default)]
    pub alpha: Option<String>,
}

/// The exact stored surface size. Q10 forbids a release replacement from changing it.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DimensionsValue {
    pub width: u32,
    pub height: u32,
    /// A label naming the form, e.g. `portrait`. It is **not** a selector: a target carrying several
    /// forms declares several `[[variant]]` entries, each with structured selector clauses.
    pub variant: String,
    /// Whether the target requires powers of two.
    #[serde(default)]
    pub power_of_two: bool,
}

/// The target's mip requirement.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MipmapValue {
    /// One of [`MIPMAP_POLICY`].
    pub policy: String,
    /// Required by `fixed-count`, refused otherwise.
    #[serde(default)]
    pub count: Option<u32>,
    /// One of [`MIPMAP_FILTER`], where the original chain's filter is established.
    #[serde(default)]
    pub filter: Option<String>,
}

/// The stream compression wrapped around the stored bytes.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompressionValue {
    /// One of [`PAYLOAD_CODEC`].
    pub payload: String,
    /// Whether re-encoding the same input reproduces the same bytes - the difference between a
    /// no-op replacement being byte-identical and merely semantically equal (C3 item 5).
    #[serde(default)]
    pub deterministic: bool,
}

/// Memory layout of the stored surface.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TilingValue {
    /// One of [`TILING_KIND`].
    pub kind: String,
    /// Row/surface alignment in bytes, where established. Must be a power of two.
    #[serde(default)]
    pub alignment_bytes: Option<u32>,
}

/// Whether the family shares a surface, and where this asset sits on it.
///
/// `kind` is **optional** in `/3`. Counting one texture payload per container establishes
/// `sharing = "per-asset-surface"` and nothing more: whether the family is an atlas, a pool or
/// neither is a further question about page geometry and slot addressing that the count does not
/// answer. C0-POR1 recorded `kind = "none"` off that count; `/3` records the count it measured and
/// leaves the kind unknown.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AtlasValue {
    /// One of [`ATLAS_SHARING`] - whether one stored surface ever carries more than one asset.
    pub sharing: String,
    /// One of [`ATLAS_KIND`], where the full meaning is established.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub page_width: Option<u32>,
    #[serde(default)]
    pub page_height: Option<u32>,
    #[serde(default)]
    pub cell_x: Option<u32>,
    #[serde(default)]
    pub cell_y: Option<u32>,
    #[serde(default)]
    pub cell_width: Option<u32>,
    #[serde(default)]
    pub cell_height: Option<u32>,
    /// The slot this asset occupies in a pool.
    #[serde(default)]
    pub slot: Option<u32>,
}

/// What the **runtime** loads, and what wins when two copies of an asset exist.
///
/// Distinct from `[discovery]`, which is where a scan finds files, and from `[install]`, which is how
/// bytes are written. Observed placement in one installed copy establishes neither this nor those.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLookupValue {
    /// One of [`RUNTIME_SOURCE`].
    pub source: String,
    /// One of [`PATCH_PRECEDENCE`].
    pub patch_precedence: String,
    /// Where the runtime's own lookup was observed - a trace, a disassembly, a load test. Never a
    /// folder listing.
    pub evidence: String,
}

/// The target's vertex buffer layout, index format and LOD count.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MeshLayoutValue {
    #[serde(default, rename = "vertex_attribute")]
    pub vertex_attributes: Vec<VertexAttribute>,
    pub vertex_stride_bytes: u32,
    /// One of [`INDEX_FORMAT`].
    pub index_format: String,
    /// One of [`WINDING`].
    pub winding: String,
    /// One of [`COORDINATE_SYSTEM`].
    pub coordinate_system: String,
    /// One of [`MESH_UNIT`].
    pub unit: String,
    /// How many LOD meshes the target carries. At least the base mesh, so never zero.
    pub lod_levels: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VertexAttribute {
    /// One of [`VERTEX_SEMANTIC`].
    pub semantic: String,
    /// Which stream of this semantic (UV0, UV1, ...).
    #[serde(default)]
    pub index: u32,
    /// One of [`COMPONENT_TYPE`].
    pub component_type: String,
    /// 1..=4.
    pub component_count: u32,
    pub byte_offset: u32,
}

/// UV conventions. Getting the origin wrong flips every texture vertically.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UvValue {
    /// One of [`UV_ORIGIN`].
    pub origin: String,
    /// One of [`UV_WRAP`].
    pub wrap: String,
    /// How many UV sets the target consumes. At least one.
    pub channel_count: u32,
}

/// Rig, bone palette and skin weights.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RigValue {
    pub bone_count: u32,
    /// The per-draw palette. Never larger than the rig.
    pub palette_size: u32,
    /// 1..=8.
    pub max_weights_per_vertex: u32,
    /// One of [`WEIGHT_QUANTIZATION`].
    pub weight_quantization: String,
    /// One of [`BIND_POSE`].
    pub bind_pose: String,
}

/// Bank layout, codec and the sample constraints a converted clip must satisfy.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AudioValue {
    /// One of [`AUDIO_BANK`].
    pub bank: String,
    /// One of [`AUDIO_CODEC`].
    pub codec: String,
    pub sample_rate_hz: u32,
    /// 1..=8.
    pub channels: u32,
    /// Required by the PCM codecs, refused for codecs whose width is fixed by the codec.
    #[serde(default)]
    pub bit_depth: Option<u32>,
    /// One of [`LOOP_METADATA`].
    pub loop_metadata: String,
    #[serde(default)]
    pub max_duration_ms: Option<u32>,
}

/// How the converted bytes reach the target, and what restore retains.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InstallValue {
    /// One of [`INSTALL_KIND`].
    pub kind: String,
    /// One of [`SIZE_POLICY`].
    pub size_policy: String,
    /// One of [`REPACK_RULE`].
    pub repack: String,
    /// One of [`BACKUP_RULE`].
    pub backup: String,
    /// Whether installing this asset also rewrites references elsewhere (a database id, an index
    /// row). Those edits belong in the same Q06 transaction.
    #[serde(default)]
    pub updates_references: bool,
}

// ==================================================================== parsing

/// Parse an `asset-output-profile/2` document. Unknown keys are a **parse error**.
pub fn parse_asset_profile(text: &str) -> Result<AssetProfile, toml::de::Error> {
    toml::from_str(text)
}

// ================================================================= validation

/// Validate one asset output profile.
pub fn validate_asset_profile(profile: &AssetProfile) -> Vec<Finding> {
    let mut out = Vec::new();

    if !ASSET_PROFILE_SCHEMAS_NATIVE.contains(&profile.schema.as_str()) {
        let detail = if profile.schema == ASSET_PROFILE_SCHEMA_PREVIOUS {
            format!(
                "{ASSET_PROFILE_SCHEMA_PREVIOUS} documents are not validated in place; run the \
                 tested transition `upgrade_asset_profile_v2` first"
            )
        } else if profile.schema == ASSET_PROFILE_SCHEMA_LEGACY {
            format!(
                "{ASSET_PROFILE_SCHEMA_LEGACY} documents are not validated in place; run the \
                 tested transition `upgrade_asset_profile_v1` first"
            )
        } else {
            format!("expected {ASSET_PROFILE_SCHEMA}")
        };
        out.push(finding(Rule::UnsupportedSchema, &profile.family, detail));
    }
    // A `/3` document is validated under its own rules, and the only thing `/4` adds for it is a
    // refusal: a construct `/3` cannot spell is not silently read under `/4`'s meaning.
    if profile.schema == ASSET_PROFILE_SCHEMA_V3 {
        for target in &profile.targets {
            let subject = format!("{}/{}", profile.family, target.target);
            let mut used: Vec<String> = Vec::new();
            if !target.resolution_chains.is_empty() {
                used.push("[[resolution_chain]]".into());
            }
            if target
                .identity
                .value
                .as_ref()
                .or(target.identity.candidate_value())
                .is_some_and(|v| v.composite.is_some())
            {
                used.push("[identity.value.composite]".into());
            }
            for family in &target.path_families {
                if family.tokens.iter().any(|t| t == COMPOSITE_TOKEN) {
                    used.push(format!(
                        "path family {:?} token {COMPOSITE_TOKEN:?}",
                        family.id
                    ));
                }
            }
            for construct in used {
                out.push(finding(
                    Rule::UnsupportedSchema,
                    &subject,
                    format!(
                        "{construct} is an {ASSET_PROFILE_SCHEMA} construct; a document that uses \
                         it must declare schema = {ASSET_PROFILE_SCHEMA:?}"
                    ),
                ));
            }
        }
    }
    if profile.profile_version == 0 {
        out.push(finding(
            Rule::UnsupportedVersion,
            &profile.family,
            "profile_version is the document's own revision and starts at 1",
        ));
    }
    if !ASSET_FAMILY.contains(&profile.family.as_str()) {
        out.push(finding(
            Rule::AssetFamilyUnknown,
            &profile.family,
            format!("not one of the {} tracked families", ASSET_FAMILY.len()),
        ));
    }

    let classes = validate_sources(profile, &mut out);
    if profile.canonical_sources.is_empty() {
        out.push(finding(
            Rule::AssetOperandsIncomplete,
            &profile.family,
            "a profile with no [[canonical_source]] declares no asset class, so no target block \
             could be applicable to it",
        ));
    }

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for target in &profile.targets {
        let subject = format!("{}/{}", profile.family, target.target);
        if target
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
        if !seen.insert(target.target.as_str()) {
            out.push(finding(
                Rule::DuplicateTargetProfile,
                &subject,
                "declared more than once in this profile",
            ));
        }
        validate_target(target, &classes, &subject, &mut out);
    }

    out.sort();
    out.dedup();
    out
}

fn validate_sources(profile: &AssetProfile, out: &mut Vec<Finding>) -> BTreeSet<String> {
    let mut classes = BTreeSet::new();
    let mut roles: BTreeSet<&str> = BTreeSet::new();
    for source in &profile.canonical_sources {
        let subject = format!("{}/source:{}", profile.family, source.role);
        check_identity(out, &subject, "role", &source.role);
        check_vocab(out, &subject, "class", &source.class, ASSET_CLASS);
        check_vocab(
            out,
            &subject,
            "interchange",
            &source.interchange,
            CANONICAL_INTERCHANGE,
        );
        check_vocab(
            out,
            &subject,
            "retention",
            &source.retention,
            SOURCE_RETENTION,
        );
        if let Some((_, class)) = INTERCHANGE_CLASS
            .iter()
            .find(|(id, _)| *id == source.interchange)
        {
            if *class != source.class {
                out.push(finding(
                    Rule::ConstraintKindMismatch,
                    &subject,
                    format!(
                        "interchange {:?} is a {class} representation, but the source declares \
                         class {:?}",
                        source.interchange, source.class
                    ),
                ));
            }
        }
        if !roles.insert(source.role.as_str()) {
            out.push(finding(
                Rule::DuplicateSourceRole,
                &subject,
                "the same canonical source role is declared twice",
            ));
        }
        classes.insert(source.class.clone());
    }
    classes
}

/// One block's identity for the population and readiness reports: its id, its state, whether it
/// carries a non-operative candidate, and which of its optional sub-properties are unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockState {
    pub block: &'static str,
    pub state: ConstraintState,
    /// Whether the block is decided **at a scope that speaks for the target**. An established block
    /// at `specimen-observation` scope is `state = Established` and `operative = false`: it is a
    /// fact about named files, and nothing about the target is answered by it.
    pub operative: bool,
    /// Whether the block **refuses** a Build a different value, rather than merely failing to
    /// authorise one. Requires `supported-target-constraint` scope, which nothing real reaches.
    pub target_required: bool,
    pub has_candidate: bool,
    /// Sub-properties from [`OPTIONAL_BLOCK_PROPERTY`] that this block's established value leaves
    /// out. Empty for a block that is not established - an unknown block's properties are all
    /// unknown, and naming them again would double-count the same gap.
    pub unknown_properties: Vec<&'static str>,
}

fn state_of<T>(
    block: &'static str,
    c: &AssetConstraint<T>,
    unknown_properties: Vec<&'static str>,
) -> BlockState {
    BlockState {
        block,
        state: c.state,
        operative: c.is_operative(),
        target_required: c.is_target_required(),
        has_candidate: c.candidate.is_some(),
        unknown_properties: if c.state == ConstraintState::Established {
            unknown_properties
        } else {
            Vec::new()
        },
    }
}

/// Which level decides a [`VARIANT_OPTIONAL_BLOCK`]: the variant when it decides it, the target
/// otherwise. Never a merge, and never a preference between two decided values - validation refuses
/// that profile before this runs.
fn resolve_optional_block<'a, T>(
    shared: &'a AssetConstraint<T>,
    per_variant: Option<&'a AssetConstraint<T>>,
) -> &'a AssetConstraint<T> {
    match per_variant {
        Some(c) if c.state != ConstraintState::Unknown || c.candidate.is_some() => c,
        _ => shared,
    }
}

fn raster_unknown_properties(c: &AssetConstraint<RasterValue>) -> Vec<&'static str> {
    let mut out = Vec::new();
    if let Some(v) = c.established() {
        if v.color_space.is_none() {
            out.push("color_space");
        }
        if v.alpha.is_none() {
            out.push("alpha");
        }
    }
    out
}

fn atlas_unknown_properties(c: &AssetConstraint<AtlasValue>) -> Vec<&'static str> {
    match c.established() {
        Some(v) if v.kind.is_none() => vec!["kind"],
        _ => Vec::new(),
    }
}

/// Every block on one target, in the order [`ASSET_BLOCK`] names them.
///
/// When the target declares variants, the four [`VARIANT_BLOCK`] entries report the **variant** the
/// caller selected rather than the target-level blocks, which validation has already forced to
/// `unknown`. With no variant selected they stay unknown, which is what makes an unselectable
/// geometry block an operation instead of a silent default.
///
/// A [`VARIANT_OPTIONAL_BLOCK`] resolves the other way round, and deterministically: the selected
/// variant decides it if the variant's own block is decided, and otherwise the shared target-level
/// block does. Validation has already refused a profile that decides one at both levels, so there
/// is no case in which this picks between two live values.
pub fn block_states(target: &TargetProfile, variant: Option<&AssetVariant>) -> Vec<BlockState> {
    let geometry = variant;
    let raster = geometry.map_or(&target.raster, |v| &v.raster);
    let dimensions = geometry.map_or(&target.dimensions, |v| &v.dimensions);
    let mipmaps = geometry.map_or(&target.mipmaps, |v| &v.mipmaps);
    let tiling = geometry.map_or(&target.tiling, |v| &v.tiling);
    let container = resolve_optional_block(&target.container, variant.map(|v| &v.container));
    vec![
        state_of("discovery", &target.discovery, Vec::new()),
        state_of("identity", &target.identity, Vec::new()),
        state_of("container", container, Vec::new()),
        state_of("path_rule", &target.path_rule, Vec::new()),
        state_of("raster", raster, raster_unknown_properties(raster)),
        state_of("dimensions", dimensions, Vec::new()),
        state_of("mipmaps", mipmaps, Vec::new()),
        state_of("compression", &target.compression, Vec::new()),
        state_of("tiling", tiling, Vec::new()),
        state_of(
            "atlas",
            &target.atlas,
            atlas_unknown_properties(&target.atlas),
        ),
        state_of("mesh_layout", &target.mesh_layout, Vec::new()),
        state_of("uv", &target.uv, Vec::new()),
        state_of("rig", &target.rig, Vec::new()),
        state_of("audio", &target.audio, Vec::new()),
        state_of("install", &target.install, Vec::new()),
        state_of("runtime_lookup", &target.runtime_lookup, Vec::new()),
    ]
}

fn validate_target(
    target: &TargetProfile,
    classes: &BTreeSet<String>,
    subject: &str,
    out: &mut Vec<Finding>,
) {
    // ---- evidence identity: the corpora everything else cites ---------------
    let corpora = validate_corpora(out, subject, target);
    let families = validate_path_families(out, subject, target, &corpora);

    // ---- the shared evidence envelope, on every block ----------------------
    let ctx = EnvelopeContext {
        corpora: &corpora,
        subject,
    };
    ctx.check(out, "discovery", &target.discovery);
    ctx.check(out, "identity", &target.identity);
    ctx.check(out, "container", &target.container);
    ctx.check(out, "path_rule", &target.path_rule);
    ctx.check(out, "raster", &target.raster);
    ctx.check(out, "dimensions", &target.dimensions);
    ctx.check(out, "mipmaps", &target.mipmaps);
    ctx.check(out, "compression", &target.compression);
    ctx.check(out, "tiling", &target.tiling);
    ctx.check(out, "atlas", &target.atlas);
    ctx.check(out, "mesh_layout", &target.mesh_layout);
    ctx.check(out, "uv", &target.uv);
    ctx.check(out, "rig", &target.rig);
    ctx.check(out, "audio", &target.audio);
    ctx.check(out, "install", &target.install);
    ctx.check(out, "runtime_lookup", &target.runtime_lookup);

    // ---- class applicability ----------------------------------------------
    // A block belonging to a class the profile does not declare is a category error whichever state
    // it carries: a raster family does not have a `not-applicable` rig, it has no rig question.
    for entry in block_states(target, None) {
        let Some((_, class)) = BLOCK_CLASS.iter().find(|(id, _)| *id == entry.block) else {
            continue;
        };
        if classes.contains(*class) {
            continue;
        }
        if entry.state != ConstraintState::Unknown || entry.has_candidate {
            out.push(finding(
                Rule::ConstraintKindMismatch,
                subject,
                format!(
                    "[{}] is a {class} constraint, but this profile declares no {class} \
                     canonical source",
                    entry.block
                ),
            ));
        }
    }

    // ---- per-block bounds, operands and dependencies ----------------------
    if let Some(v) = target.discovery.established() {
        check_vocab(out, subject, "[discovery].scope", &v.scope, DISCOVERY_SCOPE);
        check_identity(out, subject, "[discovery].root", &v.root);
    }
    if let Some(v) = target.identity.established() {
        check_vocab(out, subject, "[identity].key", &v.key, IDENTITY_KEY);
        check_identity(out, subject, "[identity].source", &v.source);
        check_vocab(
            out,
            subject,
            "[identity].ambiguity",
            &v.ambiguity,
            AMBIGUITY_POLICY,
        );
    }
    // The composite is checked wherever it is written - an established value or a candidate - so a
    // proposal that could never construct an id is reported before anyone promotes it.
    for (where_, v) in [
        ("[identity.value]", target.identity.value.as_ref()),
        (
            "[identity.candidate.value]",
            target.identity.candidate_value(),
        ),
    ] {
        if let Some(v) = v {
            validate_composite_declaration(out, subject, where_, v);
        }
    }
    validate_composite_token_use(out, subject, target);
    validate_resolution_chains(out, subject, target, &families, &ctx);
    if let Some(v) = target.path_rule.established() {
        validate_path_rule(out, subject, target, &families, v);
    }
    if let Some(v) = target.compression.established() {
        check_vocab(
            out,
            subject,
            "[compression].payload",
            &v.payload,
            PAYLOAD_CODEC,
        );
    }
    if let Some(v) = target.atlas.established() {
        validate_atlas(out, subject, v);
    }
    if let Some(v) = target.runtime_lookup.established() {
        check_vocab(
            out,
            subject,
            "[runtime_lookup].source",
            &v.source,
            RUNTIME_SOURCE,
        );
        check_vocab(
            out,
            subject,
            "[runtime_lookup].patch_precedence",
            &v.patch_precedence,
            PATCH_PRECEDENCE,
        );
        check_identity(out, subject, "[runtime_lookup].evidence", &v.evidence);
    }
    if let Some(v) = target.mesh_layout.established() {
        validate_mesh_layout(out, subject, v);
    }
    if let Some(v) = target.uv.established() {
        validate_uv(out, subject, &target.mesh_layout, v);
    }
    if let Some(v) = target.rig.established() {
        validate_rig(out, subject, &target.mesh_layout, v);
    }
    if let Some(v) = target.audio.established() {
        validate_audio(out, subject, v);
    }
    if let Some(v) = target.install.established() {
        validate_install(out, subject, target, v);
    }

    // ---- the correlated geometry, at target level or in variants ----------
    if let Some(v) = target.container.established() {
        validate_container(out, subject, v);
    }
    if target.variants.is_empty() {
        validate_geometry(
            out,
            subject,
            &ctx,
            &target.raster,
            &target.dimensions,
            &target.mipmaps,
            &target.tiling,
        );
    } else {
        validate_variants(out, subject, &ctx, target, &families);
    }

    validate_operations(out, subject, target, classes);
}

/// The five correlated blocks, checked together wherever they live.
#[allow(clippy::too_many_arguments)]
fn validate_geometry(
    out: &mut Vec<Finding>,
    subject: &str,
    ctx: &EnvelopeContext<'_>,
    raster: &AssetConstraint<RasterValue>,
    dimensions: &AssetConstraint<DimensionsValue>,
    mipmaps: &AssetConstraint<MipmapValue>,
    tiling: &AssetConstraint<TilingValue>,
) {
    let _ = ctx;
    if let Some(v) = raster.established() {
        validate_raster(out, subject, v);
    }
    if let Some(v) = dimensions.established() {
        validate_dimensions(out, subject, raster, v);
    }
    if let Some(v) = mipmaps.established() {
        validate_mipmaps(out, subject, dimensions, v);
    }
    if let Some(v) = tiling.established() {
        validate_tiling(out, subject, v);
    }
}

// ------------------------------------------------------------- evidence identity

/// What one declared corpus may support, decided once and read everywhere.
///
/// `attribution` is what the document claims; `speaks_for_target` is what that claim survives after
/// the corpus kind and any [`ProvenanceBridge`] are taken into account. The two are separate because
/// a bridged extraction is *not* attributable and may still support a placement fact, and saying so
/// in one boolean would lose the distinction the message needs to explain.
#[derive(Debug, Clone)]
pub struct CorpusStanding {
    pub attribution: String,
    pub speaks_for_target: bool,
}

/// The corpora a target declares, by id, with what each one may support.
struct EnvelopeContext<'a> {
    corpora: &'a BTreeMap<String, CorpusStanding>,
    subject: &'a str,
}

impl EnvelopeContext<'_> {
    fn check<T>(&self, out: &mut Vec<Finding>, block: &str, c: &AssetConstraint<T>) {
        check_asset_envelope(out, self.subject, block, c, self.corpora);
    }
}

/// Validate `[[corpus]]` and return `id -> attribution`.
fn validate_corpora(
    out: &mut Vec<Finding>,
    subject: &str,
    target: &TargetProfile,
) -> BTreeMap<String, CorpusStanding> {
    // Which ids are attributable on their own account, so a bridge can be checked against them.
    let attributable: BTreeSet<&str> = target
        .corpora
        .iter()
        .filter(|c| {
            c.attribution == "attributable-to-target"
                && !UNATTRIBUTABLE_CORPUS_KIND.contains(&c.kind.as_str())
        })
        .map(|c| c.id.as_str())
        .collect();
    let mut corpora: BTreeMap<String, CorpusStanding> = BTreeMap::new();
    for corpus in &target.corpora {
        let which = format!("{subject}#corpus:{}", corpus.id);
        check_identity(out, &which, "[corpus].id", &corpus.id);
        check_vocab(out, &which, "[corpus].kind", &corpus.kind, CORPUS_KIND);
        check_vocab(
            out,
            &which,
            "[corpus].attribution",
            &corpus.attribution,
            CORPUS_ATTRIBUTION,
        );
        check_vocab(
            out,
            &which,
            "[corpus].build_identification",
            &corpus.build_identification,
            BUILD_IDENTIFICATION,
        );
        check_identity(
            out,
            &which,
            "[corpus].identity_source",
            &corpus.identity_source,
        );
        if corpus.inspected_files == 0 {
            out.push(finding(
                Rule::AssetCorpusIncomplete,
                &which,
                "[corpus].inspected_files = 0: a corpus nobody inspected supports nothing",
            ));
        }
        if corpus.provenance.is_empty() {
            out.push(finding(
                Rule::AssetCorpusIncomplete,
                &which,
                "[corpus] must cite where its inspection is recorded",
            ));
        }
        if corpus.build_identification == "incomplete" && corpus.limitations.is_empty() {
            out.push(finding(
                Rule::AssetCorpusIncomplete,
                &which,
                "the build behind this corpus is not identified, so at least one `limitations` \
                 entry must say what that costs. A title id plus a version field is not a build",
            ));
        }
        if UNATTRIBUTABLE_CORPUS_KIND.contains(&corpus.kind.as_str())
            && corpus.attribution == "attributable-to-target"
        {
            out.push(finding(
                Rule::AssetCorpusAttributionUnsupported,
                &which,
                format!(
                    "a {:?} corpus is the output of an extraction or a rebuild, not something the \
                     title ships, so it cannot be attributed to the target however exhaustively it \
                     was measured",
                    corpus.kind
                ),
            ));
        }
        let bridged = validate_bridge(out, &which, corpus, &attributable);
        let speaks_for_target = bridged
            || (corpus.attribution == "attributable-to-target"
                && !UNATTRIBUTABLE_CORPUS_KIND.contains(&corpus.kind.as_str()));
        if corpora
            .insert(
                corpus.id.clone(),
                CorpusStanding {
                    attribution: corpus.attribution.clone(),
                    speaks_for_target,
                },
            )
            .is_some()
        {
            out.push(finding(
                Rule::AssetCorpusIncomplete,
                &which,
                "two [[corpus]] entries share an id, so a citation cannot say which one it means",
            ));
        }
    }
    corpora
}

/// Validate a corpus's `[corpus.bridge]` declaration, and report whether it is well formed enough
/// to carry attribution.
///
/// A bridge is the only way an extracted or derived tree reaches a placement fact, so the
/// declaration is checked rather than accepted: a known method, an existing **attributable** corpus
/// on the other end, and a citation. Anything less returns false and the corpus keeps the standing
/// its kind gives it.
///
/// These are **structural** checks on the declaration. Nothing here reads the cited record, tests
/// the tie it asserts, or says anything about runtime lookup precedence - see
/// [`ProvenanceBridge`] for what that means in practice.
fn validate_bridge(
    out: &mut Vec<Finding>,
    which: &str,
    corpus: &Corpus,
    attributable: &BTreeSet<&str>,
) -> bool {
    let Some(bridge) = &corpus.bridge else {
        return false;
    };
    let mut ok = true;
    if !BRIDGE_METHOD.contains(&bridge.method.as_str()) {
        check_vocab(
            out,
            which,
            "[corpus.bridge].method",
            &bridge.method,
            BRIDGE_METHOD,
        );
        ok = false;
    }
    if bridge.provenance.is_empty() {
        out.push(finding(
            Rule::AssetProvenanceBridgeInvalid,
            which,
            "[corpus.bridge] cites nothing. A bridge is the record that survived the extraction;              an uncited one is the assertion that a record existed",
        ));
        ok = false;
    }
    if bridge.original_corpus == corpus.id {
        out.push(finding(
            Rule::AssetProvenanceBridgeInvalid,
            which,
            "[corpus.bridge] bridges this corpus to itself, which carries nothing across",
        ));
        ok = false;
    } else if !attributable.contains(bridge.original_corpus.as_str()) {
        out.push(finding(
            Rule::AssetProvenanceBridgeInvalid,
            which,
            format!(
                "[corpus.bridge] ties this corpus to {:?}, which is not an attributable corpus on                  this target. Bridging one extraction to another moves the question rather than                  answering it",
                bridge.original_corpus
            ),
        ));
        ok = false;
    }
    if corpus.attribution == "attributable-to-target" {
        out.push(finding(
            Rule::AssetProvenanceBridgeInvalid,
            which,
            "[corpus.bridge] is declared on a corpus that already claims attribution, so it              bridges nothing. A bridge exists to carry attribution to a tree that lacks it",
        ));
        ok = false;
    }
    ok
}

/// Validate `[[path_family]]` and return the declared ids.
fn validate_path_families(
    out: &mut Vec<Finding>,
    subject: &str,
    target: &TargetProfile,
    corpora: &BTreeMap<String, CorpusStanding>,
) -> BTreeSet<String> {
    let mut families = BTreeSet::new();
    for family in &target.path_families {
        let which = format!("{subject}#path_family:{}", family.id);
        check_identity(out, &which, "[path_family].id", &family.id);
        check_identity(out, &which, "[path_family].template", &family.template);
        check_vocab(out, &which, "[path_family].case", &family.case, PATH_CASE);
        check_vocab(
            out,
            &which,
            "[path_family].bucket_resolution",
            &family.bucket_resolution,
            BUCKET_RESOLUTION,
        );
        validate_template_tokens(
            out,
            &which,
            "[path_family]",
            &family.template,
            &family.tokens,
        );
        if !families.insert(family.id.clone()) {
            out.push(finding(
                Rule::AssetPathFamilyUnresolved,
                &which,
                "two [[path_family]] entries share an id",
            ));
        }
        let mut directories: BTreeSet<&str> = BTreeSet::new();
        for bucket in &family.observed_buckets {
            check_identity(
                out,
                &which,
                "[observed_bucket].directory",
                &bucket.directory,
            );
            check_identity(out, &which, "[observed_bucket].census", &bucket.census);
            if !is_lowercase_hex(&bucket.census_digest) || bucket.census_digest.len() < 32 {
                out.push(finding(
                    Rule::AssetPathFamilyUnresolved,
                    &which,
                    format!(
                        "[observed_bucket] {:?} references census {:?} without a usable digest. A                          reference to whatever a path holds today is not a reference to an                          observation",
                        bucket.directory, bucket.census
                    ),
                ));
            }
            check_vocab(
                out,
                &which,
                "[observed_bucket].scope",
                &bucket.scope,
                EVIDENCE_SCOPE,
            );
            if bucket.scope == "supported-target-constraint" {
                out.push(finding(
                    Rule::AssetPathFamilyUnresolved,
                    &which,
                    format!(
                        "[observed_bucket] {:?} claims scope \"supported-target-constraint\".                          Membership is what somebody saw in a corpus; a directory holding an id is                          never the target requiring it to be there",
                        bucket.directory
                    ),
                ));
            }
            if !corpora.contains_key(&bucket.corpus) {
                out.push(finding(
                    Rule::AssetCorpusUndeclared,
                    &which,
                    format!(
                        "[observed_bucket] {:?} cites corpus {:?}, which this target does not \
                         declare",
                        bucket.directory, bucket.corpus
                    ),
                ));
            }
            if !directories.insert(bucket.directory.as_str()) {
                out.push(finding(
                    Rule::AssetBoundsInconsistent,
                    &which,
                    format!("[observed_bucket] {:?} is declared twice", bucket.directory),
                ));
            }
        }
        // Observed membership is not a rule, and must never be read as one. A family that declares
        // buckets while claiming its paths resolve from the template alone is claiming exactly that.
        if family.bucket_resolution == "template-only" && !family.observed_buckets.is_empty() {
            out.push(finding(
                Rule::AssetPathFamilyUnresolved,
                &which,
                "bucket_resolution = \"template-only\" says the template resolves a path by \
                 itself, but the family observes buckets the template does not produce. Observed \
                 membership is not an id-to-directory rule",
            ));
        }
        if family.bucket_resolution == "derived-directory" {
            out.push(finding(
                Rule::AssetPathFamilyUnresolved,
                &which,
                "`derived-directory` names a rule this schema cannot yet spell: PATH_TOKEN has no \
                 token for a directory computed from the id. Recording the rule needs a schema \
                 revision, not a label",
            ));
        }
    }
    families
}

/// Lowercase hex, and non-empty. A digest written in mixed case is a different string from the one
/// a tool emits, and comparing them later would silently fail.
fn is_lowercase_hex(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// A template and its token list must describe the same substitutions in both directions.
fn validate_template_tokens(
    out: &mut Vec<Finding>,
    subject: &str,
    block: &str,
    template: &str,
    tokens: &[String],
) -> BTreeSet<String> {
    let mut declared: BTreeSet<&str> = BTreeSet::new();
    for token in tokens {
        check_vocab(out, subject, &format!("{block}.tokens"), token, PATH_TOKEN);
        if !declared.insert(token.as_str()) {
            out.push(finding(
                Rule::AssetBoundsInconsistent,
                subject,
                format!("{block} declares token {token:?} twice"),
            ));
        }
    }
    let used = template_tokens(template);
    for token in &used {
        if !declared.contains(token.as_str()) {
            out.push(finding(
                Rule::AssetOperandsIncomplete,
                subject,
                format!(
                    "{block}.template substitutes {{{token}}}, which the token list does not \
                     declare"
                ),
            ));
        }
    }
    for token in &declared {
        if !used.contains(*token) {
            out.push(finding(
                Rule::AssetOperandsIncomplete,
                subject,
                format!("{block} declares token {token:?}, which the template never uses"),
            ));
        }
    }
    declared.iter().map(|t| (*t).to_string()).collect()
}

// ------------------------------------------------------------- the asset envelope

fn check_asset_candidate<T>(
    out: &mut Vec<Finding>,
    subject: &str,
    block: &str,
    c: &AssetConstraint<T>,
) {
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
    if let Some(scope) = candidate.scope.as_deref() {
        check_vocab(
            out,
            subject,
            &format!("[{block}.candidate].scope"),
            scope,
            EVIDENCE_SCOPE,
        );
    }
}

/// The state/confidence/provenance discipline of the database half, plus `/3`'s scope, force and
/// corpus binding and the per-sub-property evidence tables.
fn check_asset_envelope<T>(
    out: &mut Vec<Finding>,
    subject: &str,
    block: &str,
    c: &AssetConstraint<T>,
    corpora: &BTreeMap<String, CorpusStanding>,
) {
    check_asset_candidate(out, subject, block, c);
    check_properties(out, subject, block, c);

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
                    format!(
                        "[{block}] is established, so it must state a confidence other than \
                         \"unknown\""
                    ),
                )),
                Some("asserted") => out.push(finding(
                    Rule::AssertedAsOperative,
                    subject,
                    format!(
                        "[{block}] is established at confidence \"asserted\". An asserted claim is \
                         not operative: move it to [{block}.candidate], which keeps the citation \
                         without letting anything act on the value"
                    ),
                )),
                Some("derived") if c.derivation.trim().is_empty() => out.push(finding(
                    Rule::DerivationUnexplained,
                    subject,
                    format!(
                        "[{block}] is derived but records no `derivation`: state the premises, why \
                         each applies to THIS target, and what the argument does not establish"
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
            check_scope_and_force(out, subject, block, c);
            check_corpus_binding(out, subject, block, c, corpora);
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
            if c.force.is_some() {
                out.push(finding(
                    Rule::ValueWithoutEstablishedState,
                    subject,
                    format!(
                        "[{block}] declares a force under state {:?}: force says what an \
                         ESTABLISHED constraint asserts about other values, and an undecided block \
                         asserts nothing",
                        c.state.as_str()
                    ),
                ));
            }
            if c.state == ConstraintState::NotApplicable && c.reason.trim().is_empty() {
                out.push(finding(
                    Rule::NotApplicableWithoutReason,
                    subject,
                    format!(
                        "[{block}] declares the constraint absent without saying how that was \
                         determined"
                    ),
                ));
            }
            if let Some(scope) = c.scope.as_deref() {
                check_vocab(
                    out,
                    subject,
                    &format!("[{block}].scope"),
                    scope,
                    EVIDENCE_SCOPE,
                );
            }
        }
    }
}

fn check_scope_and_force<T>(
    out: &mut Vec<Finding>,
    subject: &str,
    block: &str,
    c: &AssetConstraint<T>,
) {
    let (Some(scope), Some(force)) = (c.scope.as_deref(), c.force.as_deref()) else {
        out.push(finding(
            Rule::AssetEvidenceScopeMissing,
            subject,
            format!(
                "[{block}] is established without both a `scope` and a `force`. How far the \
                 evidence reaches, and whether the target actually REQUIRES this value, are \
                 separate facts from how well it was measured, and neither has a safe default"
            ),
        ));
        return;
    };
    check_vocab(
        out,
        subject,
        &format!("[{block}].scope"),
        scope,
        EVIDENCE_SCOPE,
    );
    check_vocab(
        out,
        subject,
        &format!("[{block}].force"),
        force,
        CONSTRAINT_FORCE,
    );
    // C0-AS3-R1: a `specimen-observation` may be *established* - it is a checkable statement about
    // named files - but it is never operative, so nothing reads it as a fact about the target. The
    // one thing it may not describe is placement: a specimen's own bytes cannot say where it sat.
    if scope == "specimen-observation" && PLACEMENT_BLOCK.contains(&block) {
        out.push(finding(
            Rule::AssetEvidenceScopeInsufficient,
            subject,
            format!(
                "[{block}] is established at scope \"specimen-observation\". Decoding named \
                 specimens establishes what those files ARE; this block says where they sit, which \
                 is a property of the tree that held them rather than of the bytes"
            ),
        ));
    }
    if EVIDENCE_SCOPE.contains(&scope)
        && CONSTRAINT_FORCE.contains(&force)
        && !SCOPE_FORCE.contains(&(scope, force))
    {
        out.push(finding(
            Rule::AssetEvidenceScopeInsufficient,
            subject,
            format!(
                "[{block}] claims force {force:?} at scope {scope:?}. Reading the bytes a copy \
                 carries establishes what the source IS, not what the target REJECTS - so it \
                 authorises source-preserving planning and nothing else"
            ),
        ));
    }
}

fn check_corpus_binding<T>(
    out: &mut Vec<Finding>,
    subject: &str,
    block: &str,
    c: &AssetConstraint<T>,
    corpora: &BTreeMap<String, CorpusStanding>,
) {
    let Some(id) = c.corpus.as_deref() else {
        out.push(finding(
            Rule::AssetCorpusUndeclared,
            subject,
            format!(
                "[{block}] is established without naming the `corpus` it rests on. Two corpora can \
                 hold byte-identical files and support entirely different claims"
            ),
        ));
        return;
    };
    let Some(standing) = corpora.get(id) else {
        out.push(finding(
            Rule::AssetCorpusUndeclared,
            subject,
            format!("[{block}] cites corpus {id:?}, which this target does not declare"),
        ));
        return;
    };
    if standing.speaks_for_target {
        return;
    }
    // C0-AS3-R1: an unattributable corpus is not evidence of nothing. It measures its own files,
    // and a `specimen-observation` says exactly that - a non-operative fact about named specimens.
    // What it cannot do is speak for the target, or recover the placement its extraction discarded.
    let scope = c.scope.as_deref().unwrap_or("");
    if PLACEMENT_BLOCK.contains(&block) {
        out.push(finding(
            Rule::AssetCorpusAttributionUnsupported,
            subject,
            format!(
                "[{block}] rests on corpus {id:?}, which is {}. This block says where the asset \
                 SITS, and extracting or rebuilding a tree is the step that discarded that. \
                 Establishing it needs a [corpus.bridge] - the extraction record that survived - \
                 not a more careful reading of the output",
                standing.attribution
            ),
        ));
        return;
    }
    if scope != "specimen-observation" {
        out.push(finding(
            Rule::AssetCorpusAttributionUnsupported,
            subject,
            format!(
                "[{block}] rests on corpus {id:?}, which is {}, and claims scope {scope:?}. \
                 However exhaustively such a corpus was measured, a fact about it is a fact about \
                 the TARGET only once the corpus is attributable to it. A measurement of the \
                 specimens themselves is available at scope \"specimen-observation\", where it \
                 keeps its citation and decides nothing",
                standing.attribution
            ),
        ));
    }
}

fn check_properties<T>(out: &mut Vec<Finding>, subject: &str, block: &str, c: &AssetConstraint<T>) {
    let known: BTreeSet<&str> = OPTIONAL_BLOCK_PROPERTY
        .iter()
        .filter(|(b, _)| *b == block)
        .map(|(_, property)| *property)
        .collect();
    for (name, evidence) in &c.properties {
        let which = format!("[{block}.property.{name}]");
        if !known.contains(name.as_str()) {
            out.push(finding(
                Rule::AssetPropertyEvidenceConflict,
                subject,
                format!(
                    "{which} names a sub-property [{block}] cannot leave unknown; the optional \
                     ones are {known:?}"
                ),
            ));
            continue;
        }
        if evidence.state != ConstraintState::Unknown {
            out.push(finding(
                Rule::AssetPropertyEvidenceConflict,
                subject,
                format!(
                    "{which} declares state {:?}. A sub-property that IS known lives in the \
                     block's value; this table exists only for the ones that are not",
                    evidence.state.as_str()
                ),
            ));
        }
        if evidence.reason.trim().is_empty() {
            out.push(finding(
                Rule::AssetPropertyEvidenceConflict,
                subject,
                format!("{which} records no reason: silence is not how an absence was determined"),
            ));
        }
        if let Some(scope) = evidence.scope.as_deref() {
            check_vocab(
                out,
                subject,
                &format!("{which}.scope"),
                scope,
                EVIDENCE_SCOPE,
            );
        }
        if let Some(confidence) = evidence.confidence.as_deref() {
            check_vocab(
                out,
                subject,
                &format!("{which}.confidence"),
                confidence,
                CANDIDATE_CONFIDENCE,
            );
        }
    }
}

// --------------------------------------------------------------------- variants

fn validate_variants(
    out: &mut Vec<Finding>,
    subject: &str,
    ctx: &EnvelopeContext<'_>,
    target: &TargetProfile,
    families: &BTreeSet<String>,
) {
    // A target cannot describe its geometry twice.
    for block in VARIANT_BLOCK {
        let state = match *block {
            "raster" => target.raster.state,
            "dimensions" => target.dimensions.state,
            "mipmaps" => target.mipmaps.state,
            "tiling" => target.tiling.state,
            _ => continue,
        };
        if state != ConstraintState::Unknown {
            out.push(finding(
                Rule::AssetVariantBlockConflict,
                subject,
                format!(
                    "[{block}] is decided at target level while this target declares variants. The \
                     five correlated blocks belong to a variant or to the target, never both - \
                     otherwise a caller could read a target-wide value that no variant carries"
                ),
            ));
        }
    }

    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for variant in &target.variants {
        let which = format!("{subject}#variant:{}", variant.id);
        check_identity(out, &which, "[variant].id", &variant.id);
        if !ids.insert(variant.id.as_str()) {
            out.push(finding(
                Rule::AssetVariantSelectorContradiction,
                &which,
                "two [[variant]] entries share an id",
            ));
        }
        ctx.check(out, "raster", &variant.raster);
        ctx.check(out, "dimensions", &variant.dimensions);
        ctx.check(out, "mipmaps", &variant.mipmaps);
        ctx.check(out, "tiling", &variant.tiling);
        ctx.check(out, "container", &variant.container);
        // A VARIANT_OPTIONAL_BLOCK may be shared or per-variant, and deciding it at both levels is
        // not a precedence question to settle - it is two answers to one question, and which of
        // them a reader gets would depend on where they looked.
        if variant.container.state != ConstraintState::Unknown
            && target.container.state != ConstraintState::Unknown
        {
            out.push(finding(
                Rule::AssetVariantBlockConflict,
                &which,
                "[container] is decided here AND at target level. A shared chain and a \
                 variant-specific one are alternatives: state the fact once, at the level it \
                 actually holds at",
            ));
        }
        if let Some(v) = variant.container.established() {
            validate_container(out, &which, v);
        }
        validate_geometry(
            out,
            &which,
            ctx,
            &variant.raster,
            &variant.dimensions,
            &variant.mipmaps,
            &variant.tiling,
        );
        validate_selectors(out, &which, variant, families);
    }

    // Two variants nothing can tell apart are not two variants. Selection would refuse them at
    // resolution time, but a profile that cannot distinguish its own forms is a defect in the
    // profile, not a runtime condition.
    for (n, a) in target.variants.iter().enumerate() {
        for b in target.variants.iter().skip(n + 1) {
            if a.selectors == b.selectors {
                out.push(finding(
                    Rule::AssetVariantSelectorContradiction,
                    subject,
                    format!(
                        "variants {:?} and {:?} declare identical selectors, so no query could \
                         ever choose between them",
                        a.id, b.id
                    ),
                ));
            }
        }
    }
}

fn validate_selectors(
    out: &mut Vec<Finding>,
    subject: &str,
    variant: &AssetVariant,
    families: &BTreeSet<String>,
) {
    if variant.selectors.is_empty() {
        out.push(finding(
            Rule::AssetVariantSelectorIncomplete,
            subject,
            "a variant with no [[selector]] clause can never be chosen, so it is not a variant - \
             it is an unreachable value",
        ));
    }
    let mut dimensions: BTreeSet<&str> = BTreeSet::new();
    for clause in &variant.selectors {
        check_vocab(
            out,
            subject,
            "[selector].dimension",
            &clause.dimension,
            SELECTOR_DIMENSION,
        );
        if !dimensions.insert(clause.dimension.as_str()) {
            out.push(finding(
                Rule::AssetVariantSelectorContradiction,
                subject,
                format!(
                    "[selector] tests {:?} twice; two clauses over one dimension are a conjunction \
                     nothing can satisfy unambiguously",
                    clause.dimension
                ),
            ));
        }
        let operands = (
            clause.equals.is_some(),
            clause.id_low.is_some() || clause.id_high.is_some(),
            clause.width.is_some() || clause.height.is_some(),
        );
        match clause.dimension.as_str() {
            "path-family" | "bucket" => {
                if !operands.0 || operands.1 || operands.2 {
                    out.push(finding(
                        Rule::AssetVariantSelectorIncomplete,
                        subject,
                        format!(
                            "[selector] {:?} takes `equals` and nothing else",
                            clause.dimension
                        ),
                    ));
                }
                if clause.dimension == "path-family" {
                    if let Some(id) = clause.equals.as_deref() {
                        if !families.contains(id) {
                            out.push(finding(
                                Rule::AssetPathFamilyUnresolved,
                                subject,
                                format!(
                                    "[selector] selects on path family {id:?}, which this target \
                                     does not declare"
                                ),
                            ));
                        }
                    }
                }
            }
            "numeric-id-range" => match (clause.id_low, clause.id_high) {
                (Some(low), Some(high)) if low <= high && !operands.0 && !operands.2 => {}
                (Some(low), Some(high)) if low > high => out.push(finding(
                    Rule::AssetBoundsInconsistent,
                    subject,
                    format!("[selector] numeric-id-range {low}..{high} is inverted"),
                )),
                _ => out.push(finding(
                    Rule::AssetVariantSelectorIncomplete,
                    subject,
                    "[selector] numeric-id-range takes `id_low` and `id_high` and nothing else",
                )),
            },
            "stored-dimensions" => match (clause.width, clause.height) {
                (Some(w), Some(h)) if w > 0 && h > 0 && !operands.0 && !operands.1 => {
                    // A selector that names dimensions the variant does not carry is a
                    // contradiction: the variant would be selected by a size it never stores.
                    if let Some(v) = variant.dimensions.established() {
                        if (v.width, v.height) != (w, h) {
                            out.push(finding(
                                Rule::AssetVariantSelectorContradiction,
                                subject,
                                format!(
                                    "[selector] selects on {w}x{h} but [dimensions] establishes \
                                     {}x{}",
                                    v.width, v.height
                                ),
                            ));
                        }
                    }
                }
                _ => out.push(finding(
                    Rule::AssetVariantSelectorIncomplete,
                    subject,
                    "[selector] stored-dimensions takes a non-zero `width` and `height` and \
                     nothing else",
                )),
            },
            _ => {}
        }
    }
}

fn validate_container(out: &mut Vec<Finding>, subject: &str, v: &ContainerValue) {
    if v.layers.is_empty() {
        out.push(finding(
            Rule::AssetOperandsIncomplete,
            subject,
            "[container] is established with no [[container.value.layer]]: a chain of nothing is \
             not a chain",
        ));
    }
    if !v.outermost_layer_examined {
        out.push(finding(
            Rule::AssetOperandsIncomplete,
            subject,
            "[container] is established while `outermost_layer_examined` is false. A chain whose \
             outer end was never looked at is a chain of unknown length, and reading it as complete \
             is how an extracted tree's inner format becomes a target's container",
        ));
    }
    for (n, layer) in v.layers.iter().enumerate() {
        check_vocab(
            out,
            subject,
            &format!("[container].layer[{n}].format"),
            &layer.format,
            CONTAINER_FORMAT,
        );
        check_vocab(
            out,
            subject,
            &format!("[container].layer[{n}].index"),
            &layer.index,
            INDEX_KIND,
        );
        // Only the innermost layer holds the payload itself, so only it may be index-free.
        if layer.index == "none" && n + 1 < v.layers.len() {
            out.push(finding(
                Rule::AssetBoundsInconsistent,
                subject,
                format!(
                    "[container].layer[{n}] ({}) declares no index but is not the innermost layer, \
                     so nothing addresses what sits inside it",
                    layer.format
                ),
            ));
        }
    }
}

fn validate_path_rule(
    out: &mut Vec<Finding>,
    subject: &str,
    target: &TargetProfile,
    families: &BTreeSet<String>,
    v: &PathRuleValue,
) {
    check_identity(
        out,
        subject,
        "[path_rule].family_selection",
        &v.family_selection,
    );
    if v.families.is_empty() {
        out.push(finding(
            Rule::AssetOperandsIncomplete,
            subject,
            "[path_rule] is established while naming no path family, so nothing resolves a path",
        ));
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for id in &v.families {
        if !families.contains(id) {
            out.push(finding(
                Rule::AssetPathFamilyUnresolved,
                subject,
                format!("[path_rule] names path family {id:?}, which this target does not declare"),
            ));
            continue;
        }
        if !seen.insert(id.as_str()) {
            out.push(finding(
                Rule::AssetBoundsInconsistent,
                subject,
                format!("[path_rule] names path family {id:?} twice"),
            ));
        }
        // The whole point of the split: a family whose id-to-directory rule is unknown carries
        // observed membership and nothing more, and observed membership cannot resolve a path.
        let Some(family) = target.path_families.iter().find(|f| &f.id == id) else {
            continue;
        };
        if family.bucket_resolution == "unknown" {
            out.push(finding(
                Rule::AssetPathFamilyUnresolved,
                subject,
                format!(
                    "[path_rule] is established through path family {id:?}, whose \
                     bucket_resolution is \"unknown\". Its [[observed_bucket]] entries record which \
                     ids were SEEN in which directory; that is membership, not a function, and a \
                     folder label is not evidence of the rule behind it"
                ),
            ));
        }
        // A slug in the path is the identity key spelled into a filename. It cannot be established
        // while the identity it names is not.
        if family.tokens.iter().any(|t| t == "player-slug") {
            match target.identity.established() {
                Some(identity) if identity.key == "player-slug" => {}
                Some(identity) => out.push(finding(
                    Rule::AssetConstraintDependencyUnmet,
                    subject,
                    format!(
                        "path family {id:?} substitutes the player slug, but [identity].key is {:?}",
                        identity.key
                    ),
                )),
                None => out.push(finding(
                    Rule::AssetConstraintDependencyUnmet,
                    subject,
                    format!(
                        "path family {id:?} substitutes the player slug, but [identity] is not \
                         established"
                    ),
                )),
            }
        }
        // The same rule for a composite: the rendered id is the identity spelled into the path,
        // and an established path rule through it needs an established composite behind it.
        if family.tokens.iter().any(|t| t == COMPOSITE_TOKEN) {
            match target.identity.established() {
                Some(identity) if identity.key == "composite" && identity.composite.is_some() => {}
                Some(identity) => out.push(finding(
                    Rule::AssetConstraintDependencyUnmet,
                    subject,
                    format!(
                        "path family {id:?} substitutes {COMPOSITE_TOKEN:?}, but [identity].key is \
                         {:?} with no composite",
                        identity.key
                    ),
                )),
                None => out.push(finding(
                    Rule::AssetConstraintDependencyUnmet,
                    subject,
                    format!(
                        "path family {id:?} substitutes {COMPOSITE_TOKEN:?}, but [identity] is not \
                         established"
                    ),
                )),
            }
        }
    }
}

/// `/4`: a composite identity that can construct an id, wherever it is written.
fn validate_composite_declaration(
    out: &mut Vec<Finding>,
    subject: &str,
    where_: &str,
    v: &IdentityValue,
) {
    let Some(composite) = &v.composite else {
        if v.key == "composite" {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                subject,
                format!("{where_}.key is \"composite\" but no [composite] table says how the id is built"),
            ));
        }
        return;
    };
    if v.key != "composite" {
        out.push(finding(
            Rule::AssetCompositeInvalid,
            subject,
            format!(
                "{where_} declares a [composite] table under key {:?}; a composite id needs key \
                 \"composite\"",
                v.key
            ),
        ));
    }
    let block = format!("{where_}.composite");
    if composite.parts.is_empty() {
        out.push(finding(
            Rule::AssetCompositeInvalid,
            subject,
            format!("{block} declares no [[part]], so no id can be built"),
        ));
    }
    let mut names: BTreeSet<&str> = BTreeSet::new();
    let mut sources: BTreeSet<(&str, &str)> = BTreeSet::new();
    for part in &composite.parts {
        let which = format!("{block}.part:{}", part.name);
        if !is_part_name(&part.name) {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                subject,
                format!("{which}: part names are [a-z][a-z0-9-]*"),
            ));
        }
        if !names.insert(part.name.as_str()) {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                subject,
                format!("{which} is declared twice"),
            ));
        }
        check_identity(out, subject, &format!("{which}.table"), &part.table);
        check_identity(out, subject, &format!("{which}.field"), &part.field);
        if part.table.is_empty() || part.field.is_empty() {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                subject,
                format!("{which} names no table or no field"),
            ));
        }
        if !sources.insert((part.table.as_str(), part.field.as_str())) {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                subject,
                format!(
                    "{which} reads {}.{}, which another part already reads",
                    part.table, part.field
                ),
            ));
        }
        if parse_render(&part.render).is_none() {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                subject,
                format!(
                    "{which}.render = {:?} is not one of {:?} (zero-padded takes a width 1..=16 as \
                     \"zero-padded:<n>\")",
                    part.render, COMPOSITE_RENDER_DOC
                ),
            ));
        }
    }
    let placeholders = format_placeholders(&composite.format);
    let mut used: BTreeSet<&str> = BTreeSet::new();
    for placeholder in &placeholders {
        if !names.contains(placeholder.as_str()) {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                subject,
                format!("{block}.format substitutes {{{placeholder}}}, which no part declares"),
            ));
        }
        if !used.insert(placeholder.as_str()) {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                subject,
                format!("{block}.format substitutes {{{placeholder}}} twice"),
            ));
        }
    }
    for name in &names {
        if !used.contains(name) {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                subject,
                format!("{block} declares part {name:?}, which the format never uses"),
            ));
        }
    }
    if placeholders.is_empty() {
        out.push(finding(
            Rule::AssetCompositeInvalid,
            subject,
            format!(
                "{block}.format substitutes nothing, so every instance would render the same id"
            ),
        ));
    }
    if composite.format.matches('{').count() != composite.format.matches('}').count() {
        out.push(finding(
            Rule::AssetCompositeInvalid,
            subject,
            format!("{block}.format has an unbalanced brace"),
        ));
    }
}

const COMPOSITE_RENDER_DOC: &[&str] = crate::asset_resolution::COMPOSITE_RENDER;

/// `/4`: a template may substitute `composite-id` only when the target writes down a composite
/// somewhere - established or proposed. (Whether it may be *established* through it is
/// [`validate_path_rule`]'s question.)
fn validate_composite_token_use(out: &mut Vec<Finding>, subject: &str, target: &TargetProfile) {
    let declared = target
        .identity
        .value
        .as_ref()
        .or(target.identity.candidate_value())
        .is_some_and(|v| v.composite.is_some());
    for family in &target.path_families {
        if family.tokens.iter().any(|t| t == COMPOSITE_TOKEN) && !declared {
            out.push(finding(
                Rule::AssetCompositeInvalid,
                format!("{subject}#path_family:{}", family.id),
                format!(
                    "the template substitutes {{{COMPOSITE_TOKEN}}}, but [identity] declares no \
                     composite (neither established nor as a candidate) to render it from"
                ),
            ));
        }
    }
}

/// `/4`: chains resolve through declared families, in one evidence envelope each, with ownership
/// that is consistent across every chain of the target.
fn validate_resolution_chains(
    out: &mut Vec<Finding>,
    subject: &str,
    target: &TargetProfile,
    families: &BTreeSet<String>,
    ctx: &EnvelopeContext<'_>,
) {
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for chain in &target.resolution_chains {
        let which = format!("{subject}#resolution_chain:{}", chain.id);
        check_identity(out, &which, "[resolution_chain].id", &chain.id);
        if !ids.insert(chain.id.as_str()) {
            out.push(finding(
                Rule::AssetResolutionChainInvalid,
                &which,
                "two [[resolution_chain]] entries share an id",
            ));
        }
        check_vocab(
            out,
            &which,
            "[resolution_chain].purpose",
            &chain.purpose,
            CHAIN_PURPOSE,
        );
        ctx.check(
            out,
            &format!("resolution_chain:{}", chain.id),
            &chain.declared,
        );
        for (where_, value) in [
            (
                "[resolution_chain.declared.value]",
                chain.declared.value.as_ref(),
            ),
            (
                "[resolution_chain.declared.candidate.value]",
                chain.declared.candidate_value(),
            ),
        ] {
            let Some(value) = value else {
                continue;
            };
            if value.steps.is_empty() {
                out.push(finding(
                    Rule::AssetResolutionChainInvalid,
                    &which,
                    format!("{where_} declares no [[step]], so nothing resolves"),
                ));
            }
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            for step in &value.steps {
                check_vocab(
                    out,
                    &which,
                    &format!("{where_}.step.ownership"),
                    &step.ownership,
                    STEP_OWNERSHIP,
                );
                if !families.contains(&step.family) {
                    out.push(finding(
                        Rule::AssetResolutionChainInvalid,
                        &which,
                        format!(
                            "{where_} steps through path family {:?}, which this target does not \
                             declare",
                            step.family
                        ),
                    ));
                }
                if !seen.insert(step.family.as_str()) {
                    out.push(finding(
                        Rule::AssetResolutionChainInvalid,
                        &which,
                        format!("{where_} steps through path family {:?} twice", step.family),
                    ));
                }
            }
            if matches!(chain.purpose.as_str(), "write" | "read-write")
                && !value.steps.iter().any(|s| s.ownership == "owned")
            {
                out.push(finding(
                    Rule::AssetResolutionChainInvalid,
                    &which,
                    format!(
                        "{where_} is declared for writing but has no owned step: every file it \
                         resolves is shared, and a write to a shared file changes every asset that \
                         falls back to it"
                    ),
                ));
            }
        }
    }
    // A family is shared or owned for the whole target. Calling a shared file owned in a second
    // chain is exactly the alias that would make it writable.
    let (shared, owned) = ownership_sets(target);
    for family in shared.intersection(&owned) {
        out.push(finding(
            Rule::AssetResolutionChainInvalid,
            subject,
            format!(
                "path family {family:?} is a shared step in one resolution chain and an owned step \
                 in another; a shared file does not become writable through a second chain"
            ),
        ));
    }
    // Two families with the same template are one file under two names - the alias a write
    // exclusion on one of them would not see.
    if !target.resolution_chains.is_empty() {
        let mut by_template: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for family in &target.path_families {
            by_template
                .entry(family.template.as_str())
                .or_default()
                .push(family.id.as_str());
        }
        for (template, ids) in by_template {
            if ids.len() > 1 {
                out.push(finding(
                    Rule::AssetResolutionChainInvalid,
                    subject,
                    format!(
                        "path families {ids:?} share the template {template:?}; with resolution \
                         chains declared, one file may not carry two ownership labels"
                    ),
                ));
            }
        }
    }
}

/// The `{token}` names a template substitutes. An unterminated `{` yields nothing, which the
/// bidirectional check then reports as a declared token the template never uses.
fn template_tokens(template: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('}') else { break };
        found.insert(rest[..close].to_string());
        rest = &rest[close + 1..];
    }
    found
}

fn validate_raster(out: &mut Vec<Finding>, subject: &str, v: &RasterValue) {
    check_vocab(out, subject, "[raster].codec", &v.codec, RASTER_CODEC);
    if let Some(color_space) = v.color_space.as_deref() {
        check_vocab(
            out,
            subject,
            "[raster].color_space",
            color_space,
            COLOR_SPACE,
        );
    }
    if let Some(alpha) = v.alpha.as_deref() {
        check_vocab(out, subject, "[raster].alpha", alpha, ALPHA_MODE);
    }
    if v.channels.is_empty() {
        out.push(finding(
            Rule::AssetOperandsIncomplete,
            subject,
            "[raster] is established with no channels",
        ));
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for channel in &v.channels {
        check_vocab(out, subject, "[raster].channels", channel, RASTER_CHANNEL);
        if !seen.insert(channel.as_str()) {
            out.push(finding(
                Rule::AssetBoundsInconsistent,
                subject,
                format!("[raster] lists channel {channel:?} twice"),
            ));
        }
    }
    let has_alpha = seen.contains("a");
    // Alpha MODE is optional; the channel set is not, so these checks apply only where a mode was
    // actually established. An unrecorded mode is not "none".
    match v.alpha.as_deref() {
        Some(mode) if mode != "none" && !has_alpha => out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!("[raster].alpha = {mode:?} but no alpha channel is stored"),
        )),
        Some("none") if has_alpha => out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "[raster] stores an alpha channel while declaring alpha = \"none\"",
        )),
        _ => {}
    }
    if v.codec == "l8" && has_alpha {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "the l8 codec stores one luminance channel and cannot carry alpha",
        ));
    }
    if v.codec == "a8" && seen != BTreeSet::from(["a"]) {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "the a8 codec stores alpha alone",
        ));
    }
}

fn validate_dimensions(
    out: &mut Vec<Finding>,
    subject: &str,
    raster: &AssetConstraint<RasterValue>,
    v: &DimensionsValue,
) {
    check_identity(out, subject, "[dimensions].variant", &v.variant);
    if v.width == 0 || v.height == 0 {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!("[dimensions] {}x{} has a zero extent", v.width, v.height),
        ));
        return;
    }
    if v.power_of_two && (!v.width.is_power_of_two() || !v.height.is_power_of_two()) {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!(
                "[dimensions] declares power_of_two but {}x{} is not",
                v.width, v.height
            ),
        ));
    }
    // A block-compressed surface can only store whole blocks. Both facts are established here, so
    // this is a contradiction rather than a caution - and because dimensions and codec are
    // CORRELATED inside one variant, this catches an impossible cross-variant pairing at authoring
    // time rather than at resolution time.
    if let Some(raster) = raster.established() {
        if let Some((_, block)) = RASTER_BLOCK_PIXELS
            .iter()
            .find(|(id, _)| *id == raster.codec)
        {
            if !v.width.is_multiple_of(*block) || !v.height.is_multiple_of(*block) {
                out.push(finding(
                    Rule::AssetBoundsInconsistent,
                    subject,
                    format!(
                        "[dimensions] {}x{} is not a whole number of {block}x{block} blocks, which \
                         the {:?} codec stores",
                        v.width, v.height, raster.codec
                    ),
                ));
            }
        }
    }
}

/// The longest mip chain a surface can have: one level per halving, down to 1x1.
fn full_chain_levels(width: u32, height: u32) -> u32 {
    32 - width.max(height).leading_zeros()
}

fn validate_mipmaps(
    out: &mut Vec<Finding>,
    subject: &str,
    dimensions: &AssetConstraint<DimensionsValue>,
    v: &MipmapValue,
) {
    check_vocab(out, subject, "[mipmaps].policy", &v.policy, MIPMAP_POLICY);
    if let Some(filter) = &v.filter {
        check_vocab(out, subject, "[mipmaps].filter", filter, MIPMAP_FILTER);
    }
    match v.policy.as_str() {
        "fixed-count" => match v.count {
            None => out.push(finding(
                Rule::AssetOperandsIncomplete,
                subject,
                "policy = \"fixed-count\" requires the count",
            )),
            Some(0) => out.push(finding(
                Rule::AssetBoundsInconsistent,
                subject,
                "a fixed mip count of 0 has no base level; that is policy = \"none\"",
            )),
            Some(count) => {
                if let Some(dimensions) = dimensions.established() {
                    let max = full_chain_levels(dimensions.width, dimensions.height);
                    if count > max {
                        out.push(finding(
                            Rule::AssetBoundsInconsistent,
                            subject,
                            format!(
                                "{count} mip levels exceed the {max} a {}x{} surface can have",
                                dimensions.width, dimensions.height
                            ),
                        ));
                    }
                }
            }
        },
        _ if v.count.is_some() => out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!(
                "policy = {:?} determines the level count; declaring one as well creates a second \
                 place to be wrong",
                v.policy
            ),
        )),
        _ => {}
    }
}

fn validate_tiling(out: &mut Vec<Finding>, subject: &str, v: &TilingValue) {
    check_vocab(out, subject, "[tiling].kind", &v.kind, TILING_KIND);
    if let Some(alignment) = v.alignment_bytes {
        if alignment == 0 || !alignment.is_power_of_two() {
            out.push(finding(
                Rule::AssetBoundsInconsistent,
                subject,
                format!("[tiling].alignment_bytes = {alignment} is not a power of two"),
            ));
        }
    }
}

fn validate_atlas(out: &mut Vec<Finding>, subject: &str, v: &AtlasValue) {
    check_vocab(out, subject, "[atlas].sharing", &v.sharing, ATLAS_SHARING);
    let page = (v.page_width, v.page_height);
    let cell = (v.cell_x, v.cell_y, v.cell_width, v.cell_height);
    let any_cell = cell.0.is_some() || cell.1.is_some() || cell.2.is_some() || cell.3.is_some();

    let Some(kind) = v.kind.as_deref() else {
        // The full meaning is unknown. Counting one payload per container establishes `sharing` and
        // nothing else, so page, cell and slot operands - which only a known kind gives meaning to -
        // must be absent rather than half-filled.
        if page.0.is_some() || page.1.is_some() || any_cell || v.slot.is_some() {
            out.push(finding(
                Rule::AssetOperandsIncomplete,
                subject,
                "[atlas] carries page, cell or slot operands while `kind` is unknown. Those \
                 operands only have a meaning once the kind is decided",
            ));
        }
        return;
    };
    check_vocab(out, subject, "[atlas].kind", kind, ATLAS_KIND);
    if kind != "none" && v.sharing == "per-asset-surface" {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!(
                "[atlas].kind = {kind:?} shares a surface between assets, which contradicts \
                 sharing = \"per-asset-surface\""
            ),
        ));
    }
    if kind == "none" && v.sharing != "per-asset-surface" {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "[atlas].kind = \"none\" means each asset owns its surface, which contradicts a shared \
             one",
        ));
    }

    match kind {
        "none" => {
            if page.0.is_some() || page.1.is_some() || any_cell || v.slot.is_some() {
                out.push(finding(
                    Rule::AssetOperandsIncomplete,
                    subject,
                    "kind = \"none\" means the asset has its own surface and takes no page, cell \
                     or slot operands",
                ));
            }
        }
        "atlas" => {
            let (Some(pw), Some(ph)) = page else {
                out.push(finding(
                    Rule::AssetOperandsIncomplete,
                    subject,
                    "kind = \"atlas\" requires the page dimensions",
                ));
                return;
            };
            let (Some(x), Some(y), Some(w), Some(h)) = cell else {
                out.push(finding(
                    Rule::AssetOperandsIncomplete,
                    subject,
                    "kind = \"atlas\" requires the full cell rect (x, y, width, height)",
                ));
                return;
            };
            if v.slot.is_some() {
                out.push(finding(
                    Rule::AssetOperandsIncomplete,
                    subject,
                    "an atlas cell is addressed by its rect, not by a pool slot",
                ));
            }
            if w == 0 || h == 0 {
                out.push(finding(
                    Rule::AssetBoundsInconsistent,
                    subject,
                    "[atlas] cell has a zero extent",
                ));
            } else if x.saturating_add(w) > pw || y.saturating_add(h) > ph {
                out.push(finding(
                    Rule::AssetBoundsInconsistent,
                    subject,
                    format!("[atlas] cell {x},{y} {w}x{h} does not fit the {pw}x{ph} page"),
                ));
            }
        }
        "pool" => {
            if page.0.is_none() || page.1.is_none() {
                out.push(finding(
                    Rule::AssetOperandsIncomplete,
                    subject,
                    "kind = \"pool\" requires the page dimensions each slot carries",
                ));
            }
            if v.slot.is_none() {
                out.push(finding(
                    Rule::AssetOperandsIncomplete,
                    subject,
                    "kind = \"pool\" requires the slot this asset occupies",
                ));
            }
            if any_cell {
                out.push(finding(
                    Rule::AssetOperandsIncomplete,
                    subject,
                    "a pool slot is a whole surface, so it carries no cell rect",
                ));
            }
        }
        _ => {}
    }
}

fn component_bytes(component_type: &str) -> Option<u32> {
    COMPONENT_TYPE
        .iter()
        .find(|(id, _)| *id == component_type)
        .map(|(_, size)| *size)
}

fn validate_mesh_layout(out: &mut Vec<Finding>, subject: &str, v: &MeshLayoutValue) {
    check_vocab(
        out,
        subject,
        "[mesh_layout].index_format",
        &v.index_format,
        INDEX_FORMAT,
    );
    check_vocab(out, subject, "[mesh_layout].winding", &v.winding, WINDING);
    check_vocab(
        out,
        subject,
        "[mesh_layout].coordinate_system",
        &v.coordinate_system,
        COORDINATE_SYSTEM,
    );
    check_vocab(out, subject, "[mesh_layout].unit", &v.unit, MESH_UNIT);
    if v.lod_levels == 0 {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "[mesh_layout].lod_levels = 0: the base mesh is a level",
        ));
    }
    if v.vertex_attributes.is_empty() {
        out.push(finding(
            Rule::AssetOperandsIncomplete,
            subject,
            "[mesh_layout] is established with no [[vertex_attribute]]",
        ));
    }
    if v.vertex_stride_bytes == 0 {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "[mesh_layout].vertex_stride_bytes = 0",
        ));
    }
    let types: Vec<&str> = COMPONENT_TYPE.iter().map(|(id, _)| *id).collect();
    let mut seen: BTreeSet<(&str, u32)> = BTreeSet::new();
    for attribute in &v.vertex_attributes {
        let which = format!("{}[{}]", attribute.semantic, attribute.index);
        check_vocab(
            out,
            subject,
            "[mesh_layout].vertex_attribute.semantic",
            &attribute.semantic,
            VERTEX_SEMANTIC,
        );
        check_vocab(
            out,
            subject,
            "[mesh_layout].vertex_attribute.component_type",
            &attribute.component_type,
            &types,
        );
        if !seen.insert((attribute.semantic.as_str(), attribute.index)) {
            out.push(finding(
                Rule::AssetBoundsInconsistent,
                subject,
                format!("[mesh_layout] declares vertex attribute {which} twice"),
            ));
        }
        if attribute.component_count == 0 || attribute.component_count > 4 {
            out.push(finding(
                Rule::AssetBoundsInconsistent,
                subject,
                format!(
                    "[mesh_layout] {which} declares {} components; a vertex attribute has 1..=4",
                    attribute.component_count
                ),
            ));
            continue;
        }
        let Some(size) = component_bytes(&attribute.component_type) else {
            continue;
        };
        let end = attribute
            .byte_offset
            .saturating_add(size.saturating_mul(attribute.component_count));
        if v.vertex_stride_bytes > 0 && end > v.vertex_stride_bytes {
            out.push(finding(
                Rule::AssetBoundsInconsistent,
                subject,
                format!(
                    "[mesh_layout] {which} spans bytes {}..{end}, past the {}-byte stride",
                    attribute.byte_offset, v.vertex_stride_bytes
                ),
            ));
        }
    }
}

fn validate_uv(
    out: &mut Vec<Finding>,
    subject: &str,
    mesh_layout: &AssetConstraint<MeshLayoutValue>,
    v: &UvValue,
) {
    check_vocab(out, subject, "[uv].origin", &v.origin, UV_ORIGIN);
    check_vocab(out, subject, "[uv].wrap", &v.wrap, UV_WRAP);
    if v.channel_count == 0 {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "[uv].channel_count = 0: a UV convention with no UV set is not a convention",
        ));
        return;
    }
    if let Some(mesh) = mesh_layout.established() {
        let streams = mesh
            .vertex_attributes
            .iter()
            .filter(|a| a.semantic == "uv")
            .count() as u32;
        if v.channel_count > streams {
            out.push(finding(
                Rule::AssetConstraintDependencyUnmet,
                subject,
                format!(
                    "[uv] declares {} UV sets, but [mesh_layout] carries {streams} uv attribute(s)",
                    v.channel_count
                ),
            ));
        }
    }
}

fn validate_rig(
    out: &mut Vec<Finding>,
    subject: &str,
    mesh_layout: &AssetConstraint<MeshLayoutValue>,
    v: &RigValue,
) {
    check_vocab(
        out,
        subject,
        "[rig].weight_quantization",
        &v.weight_quantization,
        WEIGHT_QUANTIZATION,
    );
    check_vocab(out, subject, "[rig].bind_pose", &v.bind_pose, BIND_POSE);
    if v.bone_count == 0 {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "[rig].bone_count = 0",
        ));
    }
    if v.palette_size == 0 {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "[rig].palette_size = 0",
        ));
    } else if v.bone_count > 0 && v.palette_size > v.bone_count {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!(
                "[rig].palette_size {} exceeds the {} bones the rig has",
                v.palette_size, v.bone_count
            ),
        ));
    }
    if v.max_weights_per_vertex == 0 || v.max_weights_per_vertex > 8 {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!(
                "[rig].max_weights_per_vertex = {}; a skinned vertex carries 1..=8",
                v.max_weights_per_vertex
            ),
        ));
        return;
    }
    // The weights have to fit in the vertex the layout declares.
    if let Some(mesh) = mesh_layout.established() {
        let slots: u32 = mesh
            .vertex_attributes
            .iter()
            .filter(|a| a.semantic == "blend_weight")
            .map(|a| a.component_count)
            .sum();
        if slots > 0 && v.max_weights_per_vertex > slots {
            out.push(finding(
                Rule::AssetConstraintDependencyUnmet,
                subject,
                format!(
                    "[rig] allows {} weights per vertex, but [mesh_layout] stores {slots}",
                    v.max_weights_per_vertex
                ),
            ));
        }
    }
}

fn validate_audio(out: &mut Vec<Finding>, subject: &str, v: &AudioValue) {
    check_vocab(out, subject, "[audio].bank", &v.bank, AUDIO_BANK);
    check_vocab(out, subject, "[audio].codec", &v.codec, AUDIO_CODEC);
    check_vocab(
        out,
        subject,
        "[audio].loop_metadata",
        &v.loop_metadata,
        LOOP_METADATA,
    );
    if v.sample_rate_hz == 0 {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "[audio].sample_rate_hz = 0",
        ));
    }
    if v.channels == 0 || v.channels > 8 {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!(
                "[audio].channels = {}; a bank slot carries 1..=8",
                v.channels
            ),
        ));
    }
    let wants_depth = AUDIO_BIT_DEPTH_CODECS.contains(&v.codec.as_str());
    match (wants_depth, v.bit_depth) {
        (true, None) => out.push(finding(
            Rule::AssetOperandsIncomplete,
            subject,
            format!("the {:?} codec stores samples of a declared width", v.codec),
        )),
        (false, Some(_)) => out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!(
                "the {:?} codec fixes its own sample width; declaring one creates a second place \
                 to be wrong",
                v.codec
            ),
        )),
        (true, Some(depth)) if !matches!(depth, 8 | 16 | 24 | 32) => out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            format!("[audio].bit_depth = {depth}"),
        )),
        _ => {}
    }
    if v.max_duration_ms == Some(0) {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "[audio].max_duration_ms = 0 admits no clip at all",
        ));
    }
}

fn validate_install(
    out: &mut Vec<Finding>,
    subject: &str,
    target: &TargetProfile,
    v: &InstallValue,
) {
    check_vocab(out, subject, "[install].kind", &v.kind, INSTALL_KIND);
    check_vocab(
        out,
        subject,
        "[install].size_policy",
        &v.size_policy,
        SIZE_POLICY,
    );
    check_vocab(out, subject, "[install].repack", &v.repack, REPACK_RULE);
    check_vocab(out, subject, "[install].backup", &v.backup, BACKUP_RULE);

    if ARCHIVE_INSTALL_KIND.contains(&v.kind.as_str()) && target.container.established().is_none() {
        out.push(finding(
            Rule::AssetConstraintDependencyUnmet,
            subject,
            format!(
                "[install].kind = {:?} writes into an archive, but [container] is not established",
                v.kind
            ),
        ));
    }
    if v.kind == "loose-file" && v.repack != "not-required" {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "a loose file has no container to repack",
        ));
    }
    if v.kind == "archive-replace-in-place" && v.size_policy != "fit-in-slot" {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "replacing in place is exactly the case where the encoded bytes must fit the original \
             span (C3a); growth requires archive-append-relocate",
        ));
    }
    if v.kind == "archive-append-relocate" && v.size_policy != "allow-growth" {
        out.push(finding(
            Rule::AssetBoundsInconsistent,
            subject,
            "append-and-retarget exists to accommodate growth; declaring fit-in-slot contradicts it",
        ));
    }
}

/// Split a `requires` entry into its block and optional sub-property.
fn split_requirement(entry: &str) -> (&str, Option<&str>) {
    match entry.split_once('.') {
        Some((block, property)) => (block, Some(property)),
        None => (entry, None),
    }
}

fn validate_operations(
    out: &mut Vec<Finding>,
    subject: &str,
    target: &TargetProfile,
    classes: &BTreeSet<String>,
) {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for declaration in &target.operations {
        let which = format!("{subject}#{}", declaration.operation);
        if declaration.operation == "game-acceptance" || declaration.operation == "game-verified" {
            out.push(finding(
                Rule::AdapterClaimsGameVerified,
                &which,
                "a profile can never reach game acceptance; qualification comes from the evidence \
                 system",
            ));
        } else {
            check_vocab(
                out,
                &which,
                "operation",
                &declaration.operation,
                ASSET_OPERATION,
            );
        }
        if !seen.insert(declaration.operation.as_str()) {
            out.push(finding(
                Rule::DuplicateOperationDeclaration,
                &which,
                "declared more than once for this target",
            ));
        }
        if declaration.requires.is_empty() {
            out.push(finding(
                Rule::AssetOperandsIncomplete,
                &which,
                "an operation that requires no format constraint at all is not a claim about the \
                 format",
            ));
        }
        let mut required: BTreeSet<&str> = BTreeSet::new();
        for entry in &declaration.requires {
            if let Some(chain_id) = entry.strip_prefix(CHAIN_REQUIREMENT_PREFIX) {
                if !required.insert(entry.as_str()) {
                    out.push(finding(
                        Rule::AssetBoundsInconsistent,
                        &which,
                        format!("requires {entry:?} twice"),
                    ));
                }
                match target.resolution_chains.iter().find(|c| c.id == chain_id) {
                    None => out.push(finding(
                        Rule::AssetResolutionChainInvalid,
                        &which,
                        format!(
                            "requires resolution chain {chain_id:?}, which this target does not \
                             declare"
                        ),
                    )),
                    Some(chain) => {
                        let writes = WRITING_OPERATION.contains(&declaration.operation.as_str());
                        if writes && chain.purpose == "read" {
                            out.push(finding(
                                Rule::AssetResolutionChainInvalid,
                                &which,
                                format!(
                                    "a writing operation requires resolution chain {chain_id:?}, \
                                     whose purpose is \"read\""
                                ),
                            ));
                        }
                    }
                }
                continue;
            }
            let (block, property) = split_requirement(entry);
            check_vocab(out, &which, "requires", block, ASSET_BLOCK);
            if let Some(property) = property {
                if !REQUIRABLE_BLOCK_PROPERTY
                    .iter()
                    .any(|(b, name)| *b == block && *name == property)
                {
                    out.push(finding(
                        Rule::AssetOperandsIncomplete,
                        &which,
                        format!(
                            "requires {entry:?}, but [{block}] has no sub-property {property:?} to \
                             narrow to"
                        ),
                    ));
                }
            }
            if !required.insert(entry.as_str()) {
                out.push(finding(
                    Rule::AssetBoundsInconsistent,
                    &which,
                    format!("requires {entry:?} twice"),
                ));
            }
            // Requiring both a block and one of its sub-properties says the same thing twice, and
            // the wider one silently subsumes the narrower - which is exactly the confusion the
            // narrowing exists to remove.
            if property.is_some() && declaration.requires.iter().any(|r| r == block) {
                out.push(finding(
                    Rule::AssetBoundsInconsistent,
                    &which,
                    format!(
                        "requires both {block:?} and {entry:?}; the whole block already demands \
                         every sub-property"
                    ),
                ));
            }
            if let Some((_, class)) = BLOCK_CLASS.iter().find(|(id, _)| *id == block) {
                if !classes.contains(*class) {
                    out.push(finding(
                        Rule::ConstraintKindMismatch,
                        &which,
                        format!(
                            "requires the {class} constraint [{block}], but this profile declares \
                             no {class} canonical source"
                        ),
                    ));
                }
            }
        }
    }
}

// ================================================= deterministic variant selection

/// What the caller knows about the asset it is planning. Every field is optional: a fact the caller
/// does not have is **undecidable**, not a wildcard, and a selector clause that tests it can neither
/// match nor mismatch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariantQuery {
    pub path_family: Option<String>,
    pub bucket: Option<String>,
    pub numeric_id: Option<u32>,
    pub stored_dimensions: Option<(u32, u32)>,
}

/// The outcome of selecting a variant. There is no "closest" and no "first".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantSelection {
    /// Exactly one variant definitely matches and no other could.
    Selected(String),
    /// The target declares no variants at all - its constraints, if any, are target-level.
    NoVariants,
    /// Nothing matches. An explicit unsupported result, never a fallback.
    NoMatch,
    /// Several variants match, or one matches beside another the query cannot decide. A refusal:
    /// the named variants are what the caller would have to disambiguate.
    Ambiguous(Vec<String>),
}

#[derive(PartialEq)]
enum ClauseVerdict {
    Match,
    Mismatch,
    Undecidable,
}

fn evaluate_clause(clause: &SelectorClause, query: &VariantQuery) -> ClauseVerdict {
    let decide = |known: bool, matched: bool| {
        if !known {
            ClauseVerdict::Undecidable
        } else if matched {
            ClauseVerdict::Match
        } else {
            ClauseVerdict::Mismatch
        }
    };
    match clause.dimension.as_str() {
        "path-family" => decide(
            query.path_family.is_some(),
            query.path_family.as_deref() == clause.equals.as_deref(),
        ),
        "bucket" => decide(
            query.bucket.is_some(),
            query.bucket.as_deref() == clause.equals.as_deref(),
        ),
        "numeric-id-range" => match (query.numeric_id, clause.id_low, clause.id_high) {
            (Some(id), Some(low), Some(high)) => decide(true, id >= low && id <= high),
            (None, _, _) => ClauseVerdict::Undecidable,
            // A malformed clause is refused by validation; at selection time it decides nothing.
            _ => ClauseVerdict::Undecidable,
        },
        "stored-dimensions" => match (query.stored_dimensions, clause.width, clause.height) {
            (Some(size), Some(w), Some(h)) => decide(true, size == (w, h)),
            (None, _, _) => ClauseVerdict::Undecidable,
            _ => ClauseVerdict::Undecidable,
        },
        // An unknown dimension is refused by validation and decides nothing here.
        _ => ClauseVerdict::Undecidable,
    }
}

/// Choose the one variant a query identifies, **deterministically**.
///
/// A variant matches only when every clause matches. One mismatching clause rules it out. Any clause
/// the query cannot decide leaves the whole variant undecidable, and an undecidable variant is
/// treated as a live possibility - so the result is a refusal rather than a guess.
///
/// | Definite matches | Undecidable | Result |
/// |---|---|---|
/// | 1 | 0 | [`VariantSelection::Selected`] |
/// | 0 | 0 | [`VariantSelection::NoMatch`] |
/// | anything else | | [`VariantSelection::Ambiguous`] |
///
/// The order variants are declared in never enters the decision, and no variant is preferred for
/// being more specific, better populated or closer to the query.
pub fn select_variant(target: &TargetProfile, query: &VariantQuery) -> VariantSelection {
    if target.variants.is_empty() {
        return VariantSelection::NoVariants;
    }
    let mut matched: Vec<String> = Vec::new();
    let mut undecidable: Vec<String> = Vec::new();
    for variant in &target.variants {
        let mut verdict = ClauseVerdict::Match;
        for clause in &variant.selectors {
            match evaluate_clause(clause, query) {
                ClauseVerdict::Mismatch => {
                    verdict = ClauseVerdict::Mismatch;
                    break;
                }
                ClauseVerdict::Undecidable => verdict = ClauseVerdict::Undecidable,
                ClauseVerdict::Match => {}
            }
        }
        // A variant with no clauses cannot be selected; validation already refuses one.
        if variant.selectors.is_empty() {
            continue;
        }
        match verdict {
            ClauseVerdict::Match => matched.push(variant.id.clone()),
            ClauseVerdict::Undecidable => undecidable.push(variant.id.clone()),
            ClauseVerdict::Mismatch => {}
        }
    }
    if matched.len() == 1 && undecidable.is_empty() {
        return VariantSelection::Selected(matched.remove(0));
    }
    if matched.is_empty() && undecidable.is_empty() {
        return VariantSelection::NoMatch;
    }
    let mut live = matched;
    live.extend(undecidable);
    live.sort();
    live.dedup();
    VariantSelection::Ambiguous(live)
}

/// The variant a query selects, or `None` for every other outcome.
pub fn selected_variant<'a>(
    target: &'a TargetProfile,
    query: &VariantQuery,
) -> Option<&'a AssetVariant> {
    match select_variant(target, query) {
        VariantSelection::Selected(id) => target.variants.iter().find(|v| v.id == id),
        _ => None,
    }
}

// ============================================ readiness, reported apart from support

/// Whether the **format constraints** an operation names are established.
///
/// Neither variant says anything about Studio. `ConstraintsEstablished` means the profile knows
/// what the target requires; whether a converter exists, works, or has ever been run is the
/// evidence system's question, and this type deliberately cannot express it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationReadiness {
    /// Every requirement the operation names is decided.
    ConstraintsEstablished,
    /// The named requirements are unknown, so no conversion could be *defined* - whatever code
    /// exists. Sub-property requirements are reported in the `<block>.<property>` form they were
    /// declared in, so the report says which fact is missing rather than which block it sits in.
    BlockedByUnknownConstraints(Vec<String>),
}

/// Report which of an operation's requirements are not decided.
///
/// A `not-applicable` block **satisfies** the requirement: "this target has no atlas" is a decided
/// fact an installer can act on. An `unknown` one does not, a candidate never does, and neither
/// does a block established at `specimen-observation` scope - this function reads the same block
/// states [`AssetConstraint::operative`] gates on, and cannot see a non-operative one.
///
/// A requirement narrowed to a sub-property (`raster.codec`) is met when the block is decided **and**
/// that sub-property is present. A requirement naming the whole block demands every optional
/// sub-property as well, which is why the committed profiles name what they actually need.
///
/// `variant` is the variant the caller selected, where the target declares any. Passing `None` on a
/// target with variants reports the correlated blocks as unknown, which is the correct answer: an
/// operation on an unidentified variant has no geometry to work from.
pub fn operation_constraint_readiness(
    target: &TargetProfile,
    variant: Option<&AssetVariant>,
    declaration: &OperationDecl,
) -> OperationReadiness {
    let states = block_states(target, variant);
    let mut missing: Vec<String> = declaration
        .requires
        .iter()
        .filter(|entry| {
            // A chain requirement is met only by an operative chain: a candidate or a
            // specimen-scoped order is a proposal about files, not a rule the target follows.
            if let Some(chain_id) = entry.strip_prefix(CHAIN_REQUIREMENT_PREFIX) {
                return !target
                    .resolution_chains
                    .iter()
                    .any(|c| c.id == chain_id && c.declared.is_operative());
            }
            let (block, property) = split_requirement(entry);
            let Some(state) = states.iter().find(|s| s.block == block) else {
                return true;
            };
            // Decided is not enough: a `specimen-observation` is a fact about named files, and an
            // operation planned against the target cannot be defined from one.
            if !state.operative {
                return true;
            }
            match property {
                Some(property) => state.unknown_properties.contains(&property),
                None => !state.unknown_properties.is_empty(),
            }
        })
        .cloned()
        .collect();
    missing.sort();
    missing.dedup();
    if missing.is_empty() {
        OperationReadiness::ConstraintsEstablished
    } else {
        OperationReadiness::BlockedByUnknownConstraints(missing)
    }
}

// ==================================================== coverage, reported apart

/// **Format completeness** — how many of the asset constraints Q07a.1 names the profile format can
/// express. A statement about the schema, and about nothing else.
pub fn asset_representation_completeness() -> (usize, usize) {
    (
        Q07A1_ASSET_REPRESENTATION.len(),
        Q07A1_ASSET_REPRESENTATION.len(),
    )
}

/// One (family, target) pair's declared constraint population.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPopulation {
    pub family: String,
    pub target: String,
    /// Blocks in the `established` state, target-level, whatever their scope. The headline number,
    /// and on its own a **misleading** one: see `operative_constraints` below.
    pub established_constraints: usize,
    /// Of those, the ones whose evidence speaks for the target - scope in [`OPERATIVE_SCOPE`].
    /// These are the only established blocks a Build ever reads.
    pub operative_constraints: usize,
    /// Established blocks at `specimen-observation` scope: true statements about named files, and
    /// operative for nothing. Counted apart so "established" is never read as "proven of the
    /// target", which is the conflation this whole schema exists to prevent.
    pub specimen_observations: usize,
    /// Operative constraints whose `force` is `target-required` - the only ones that can refuse a
    /// Build a different value. Zero across every real profile in this repository.
    pub target_required_constraints: usize,
    pub unknown_constraints: usize,
    pub not_applicable_constraints: usize,
    /// Unknown blocks carrying a **non-operative** proposal. Counted apart, because a candidate is
    /// not coverage.
    pub candidate_constraints: usize,
    /// Sub-properties an established block leaves unknown, summed over the target-level blocks and
    /// every variant. Counted apart for the same reason: a partially known block is not a gap in
    /// the block, and pretending otherwise is what made a bundled value tempting to fill.
    pub unknown_properties: usize,
    pub declared_variants: usize,
    /// Established constraints carried by variants rather than by the target as a whole.
    pub established_variant_constraints: usize,
    /// Of those, the ones that are operative.
    pub operative_variant_constraints: usize,
    pub declared_path_families: usize,
    /// Path families whose authoritative id-to-directory rule is still unknown.
    pub unresolved_path_families: usize,
    pub declared_corpora: usize,
    /// Corpora that cannot support an established fact about this target.
    pub unattributable_corpora: usize,
    pub declared_operations: usize,
    /// `/4`: resolution chains declared on this target, and those operative (established at a
    /// scope that speaks for the target). A candidate chain counts in the first and not the second.
    pub declared_resolution_chains: usize,
    pub operative_resolution_chains: usize,
}

pub fn asset_registry_population<'a>(
    profiles: impl IntoIterator<Item = &'a AssetProfile>,
) -> Vec<AssetPopulation> {
    let mut out = Vec::new();
    for profile in profiles.into_iter().filter(|p| !p.synthetic) {
        for target in &profile.targets {
            let mut established = 0usize;
            let mut operative = 0usize;
            let mut specimen = 0usize;
            let mut target_required = 0usize;
            let mut unknown = 0usize;
            let mut not_applicable = 0usize;
            let mut candidates = 0usize;
            let mut unknown_properties = 0usize;
            for entry in block_states(target, None) {
                match entry.state {
                    ConstraintState::Established => {
                        established += 1;
                        if entry.operative {
                            operative += 1;
                        } else {
                            specimen += 1;
                        }
                    }
                    ConstraintState::Unknown => unknown += 1,
                    ConstraintState::NotApplicable => not_applicable += 1,
                }
                target_required += usize::from(entry.target_required);
                candidates += usize::from(entry.has_candidate);
                unknown_properties += entry.unknown_properties.len();
            }
            let mut variant_established = 0usize;
            let mut variant_operative = 0usize;
            for variant in &target.variants {
                for entry in block_states(target, Some(variant)) {
                    // The correlated four always belong to the variant. A VARIANT_OPTIONAL_BLOCK
                    // belongs to it only when the variant is the level that decided it - counting
                    // the shared target-level chain once per variant would report one fact three
                    // times.
                    let owned = VARIANT_BLOCK.contains(&entry.block)
                        || (VARIANT_OPTIONAL_BLOCK.contains(&entry.block)
                            && variant.container.state != ConstraintState::Unknown);
                    if !owned {
                        continue;
                    }
                    if entry.state == ConstraintState::Established {
                        variant_established += 1;
                        if entry.operative {
                            variant_operative += 1;
                        } else {
                            specimen += 1;
                        }
                    }
                    target_required += usize::from(entry.target_required);
                    candidates += usize::from(entry.has_candidate);
                    unknown_properties += entry.unknown_properties.len();
                }
            }
            out.push(AssetPopulation {
                family: profile.family.clone(),
                target: target.target.clone(),
                established_constraints: established,
                operative_constraints: operative,
                specimen_observations: specimen,
                target_required_constraints: target_required,
                unknown_constraints: unknown,
                not_applicable_constraints: not_applicable,
                candidate_constraints: candidates,
                unknown_properties,
                declared_variants: target.variants.len(),
                established_variant_constraints: variant_established,
                operative_variant_constraints: variant_operative,
                declared_path_families: target.path_families.len(),
                unresolved_path_families: target
                    .path_families
                    .iter()
                    .filter(|f| f.bucket_resolution == "unknown")
                    .count(),
                declared_corpora: target.corpora.len(),
                unattributable_corpora: target
                    .corpora
                    .iter()
                    .filter(|c| c.attribution != "attributable-to-target")
                    .count(),
                declared_operations: target.operations.len(),
                declared_resolution_chains: target.resolution_chains.len(),
                operative_resolution_chains: target
                    .resolution_chains
                    .iter()
                    .filter(|c| c.declared.is_operative())
                    .count(),
            });
        }
    }
    out
}
