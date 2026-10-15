//! Shared asset definitions by reference (`asset-output-profile/5`, package C0-AS5).
//!
//! Q47 (user direction, 2026-09-15): Legacy Recompiled uses the same asset files and the same
//! asset definitions as Legacy Xbox 360. A `[[target_profile]]` may therefore declare
//! `reuses = "<target id>"` and inherit the **shared definitions** of another target profile in the
//! same family document instead of repeating them. What is shared and what is not is fixed here,
//! not by the document:
//!
//! * **Shared, inherited verbatim** ([`SHARED_BLOCKS`], [`SHARED_TABLES`]): the source-side
//!   description of the asset — `identity`, `container`, `path_rule`, the geometry and codec blocks
//!   (`raster`, `dimensions`, `mipmaps`, `compression`, `tiling`, `atlas`), the mesh/audio blocks,
//!   the `[[path_family]]` naming, `[[variant]]`s, `[[resolution_chain]]`s and the `[[operation]]`
//!   requirement declarations — together with the source's `[[corpus]]` records, because a block's
//!   evidence identity travels with the block. Their **states travel unchanged**: a source
//!   unknown stays unknown, a source candidate stays a candidate, a specimen-scoped block stays
//!   non-operative.
//! * **Evidence identity, merged by id** ([`LOCAL_EVIDENCE_TABLES`]): the reusing profile may
//!   declare its own `[[corpus]]` records — an inspection of a Recompiled installation, when one
//!   exists — for its destination blocks to cite; the effective profile carries the source's and
//!   the target's corpora side by side, and a colliding id is refused rather than shadowed.
//! * **Destination, never inherited** ([`DESTINATION_BLOCKS`]): `discovery` (where the destination
//!   keeps the files), `install` (how they are written there), `runtime_lookup` (what the destination
//!   runtime reads). The reusing profile declares these itself or leaves them unknown; nothing about
//!   Legacy Xbox 360's installation, deployment or runtime evidence says anything about Recompiled's.
//! * **Never carried at all**: qualification, automated-test attribution and evidence cells are not
//!   profile content and do not exist here to inherit; database equivalence is a separate registry;
//!   the reference does not create reference-update capability ([`crate::asset_resolution::CreationSupport`]
//!   reads the *destination's* `install`, which is the reusing profile's own).
//!
//! The reusing profile may not declare any shared block or table itself: a local declaration that
//! would compete with the source is a **conflict**, reported by the validator and refused by the
//! resolver — nothing is merged, nothing wins silently. A reference must name a different target
//! profile in the same document, and that source may not itself reuse (no chains, and therefore no
//! cycles). The resolver never touches storage and establishes nothing: it assembles the effective
//! [`TargetProfile`] the existing readiness and resolution APIs are then run on.

use crate::asset::{AssetProfile, TargetProfile};
use crate::ConstraintState;

/// The constraint blocks a reusing target inherits from its source.
pub const SHARED_BLOCKS: &[&str] = &[
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
];

/// The constraint blocks that stay the reusing target's own.
pub const DESTINATION_BLOCKS: &[&str] = &["discovery", "install", "runtime_lookup"];

/// The array tables a reusing target inherits from its source.
pub const SHARED_TABLES: &[&str] = &["path_family", "variant", "resolution_chain", "operation"];

/// The array tables a reusing target may declare for itself, merged with the source's by id.
pub const LOCAL_EVIDENCE_TABLES: &[&str] = &["corpus"];

/// Why a `reuses` reference cannot be followed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReuseError {
    /// The target does not declare `reuses`.
    NotAReuse { target: String },
    /// No such target profile in this document.
    UnresolvedReference { target: String, reuses: String },
    /// `reuses` names the target itself.
    SelfReference { target: String },
    /// The source itself reuses another profile: references do not chain.
    SourceReuses {
        target: String,
        reuses: String,
        source_reuses: String,
    },
    /// The reusing target declares shared content of its own; nothing is merged.
    ConflictingLocalDeclarations {
        target: String,
        declared: Vec<&'static str>,
    },
    /// A locally declared corpus id is already one of the source's: shadowing is refused.
    CorpusIdCollision { target: String, id: String },
    /// The target id is not declared in this document at all.
    UnknownTarget { target: String },
}

impl std::fmt::Display for ReuseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReuseError::NotAReuse { target } => write!(f, "{target}: declares no `reuses`"),
            ReuseError::UnresolvedReference { target, reuses } => write!(
                f,
                "{target}: `reuses = {reuses:?}` names no target profile in this document"
            ),
            ReuseError::SelfReference { target } => {
                write!(f, "{target}: `reuses` names the target itself")
            }
            ReuseError::SourceReuses {
                target,
                reuses,
                source_reuses,
            } => write!(
                f,
                "{target}: `reuses = {reuses:?}`, but that profile itself reuses {source_reuses:?}; \
                 references do not chain"
            ),
            ReuseError::ConflictingLocalDeclarations { target, declared } => write!(
                f,
                "{target}: declares shared content of its own ({}) while reusing another profile; \
                 a shared definition is inherited or declared, never both",
                declared.join(", ")
            ),
            ReuseError::CorpusIdCollision { target, id } => write!(
                f,
                "{target}: declares corpus {id:?}, which is already a corpus of the reused profile; \
                 a local corpus may not shadow an inherited one"
            ),
            ReuseError::UnknownTarget { target } => {
                write!(f, "{target}: not a target profile of this document")
            }
        }
    }
}

/// The effective profile of a target, with where its shared definitions came from.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTarget {
    /// The profile the readiness and resolution APIs run on.
    pub profile: TargetProfile,
    /// The source target whose shared definitions were inherited, when the target reuses one.
    pub reused_from: Option<String>,
    /// The shared blocks and tables actually taken from the source (names from [`SHARED_BLOCKS`]
    /// and [`SHARED_TABLES`]), for the report.
    pub inherited: Vec<&'static str>,
}

/// Whether a block carries anything at all — a state other than `unknown`, or a candidate, or a
/// sub-property record. A bare `unknown` block is the deserialiser's default and is not a
/// declaration.
fn declares<T>(c: &crate::asset::AssetConstraint<T>) -> bool {
    c.state != ConstraintState::Unknown
        || c.candidate.is_some()
        || c.value.is_some()
        || !c.properties.is_empty()
}

/// The shared blocks and tables a target declares locally — on a reusing target, the conflicts.
pub fn shared_declarations(target: &TargetProfile) -> Vec<&'static str> {
    let mut out = Vec::new();
    for (name, present) in [
        ("identity", declares(&target.identity)),
        ("container", declares(&target.container)),
        ("path_rule", declares(&target.path_rule)),
        ("raster", declares(&target.raster)),
        ("dimensions", declares(&target.dimensions)),
        ("mipmaps", declares(&target.mipmaps)),
        ("compression", declares(&target.compression)),
        ("tiling", declares(&target.tiling)),
        ("atlas", declares(&target.atlas)),
        ("mesh_layout", declares(&target.mesh_layout)),
        ("uv", declares(&target.uv)),
        ("rig", declares(&target.rig)),
        ("audio", declares(&target.audio)),
        ("path_family", !target.path_families.is_empty()),
        ("variant", !target.variants.is_empty()),
        ("resolution_chain", !target.resolution_chains.is_empty()),
        ("operation", !target.operations.is_empty()),
    ] {
        if present {
            out.push(name);
        }
    }
    out
}

/// Check a `reuses` reference structurally: resolvable, not self, not chained, no local
/// conflicts. Returns the source profile.
pub fn reuse_source<'a>(
    profile: &'a AssetProfile,
    target: &TargetProfile,
) -> Result<&'a TargetProfile, ReuseError> {
    let reuses = target
        .reuses
        .as_deref()
        .ok_or_else(|| ReuseError::NotAReuse {
            target: target.target.clone(),
        })?;
    if reuses == target.target {
        return Err(ReuseError::SelfReference {
            target: target.target.clone(),
        });
    }
    let source = profile
        .targets
        .iter()
        .find(|t| t.target == reuses)
        .ok_or_else(|| ReuseError::UnresolvedReference {
            target: target.target.clone(),
            reuses: reuses.to_string(),
        })?;
    if let Some(source_reuses) = &source.reuses {
        return Err(ReuseError::SourceReuses {
            target: target.target.clone(),
            reuses: reuses.to_string(),
            source_reuses: source_reuses.clone(),
        });
    }
    let declared = shared_declarations(target);
    if !declared.is_empty() {
        return Err(ReuseError::ConflictingLocalDeclarations {
            target: target.target.clone(),
            declared,
        });
    }
    for local in &target.corpora {
        if source.corpora.iter().any(|c| c.id == local.id) {
            return Err(ReuseError::CorpusIdCollision {
                target: target.target.clone(),
                id: local.id.clone(),
            });
        }
    }
    Ok(source)
}

/// The effective profile of `target_id`: the target itself when it reuses nothing; otherwise the
/// source's shared definitions under the target's own id, note, `reuses` and destination blocks.
pub fn resolve_target(
    profile: &AssetProfile,
    target_id: &str,
) -> Result<ResolvedTarget, ReuseError> {
    let target = profile
        .targets
        .iter()
        .find(|t| t.target == target_id)
        .ok_or_else(|| ReuseError::UnknownTarget {
            target: target_id.to_string(),
        })?;
    if target.reuses.is_none() {
        return Ok(ResolvedTarget {
            profile: target.clone(),
            reused_from: None,
            inherited: Vec::new(),
        });
    }
    let source = reuse_source(profile, target)?;
    let mut effective = source.clone();
    effective.target = target.target.clone();
    effective.note = target.note.clone();
    effective.reuses = target.reuses.clone();
    // Evidence identity: the source's corpora (cited by the shared blocks) and the target's own
    // (cited by its destination blocks), side by side.
    effective.corpora.extend(target.corpora.iter().cloned());
    // Destination blocks are the reusing target's own, whatever the source declares.
    effective.discovery = target.discovery.clone();
    effective.install = target.install.clone();
    effective.runtime_lookup = target.runtime_lookup.clone();
    let mut inherited: Vec<&'static str> = Vec::new();
    for (name, present) in [
        ("identity", declares(&source.identity)),
        ("container", declares(&source.container)),
        ("path_rule", declares(&source.path_rule)),
        ("raster", declares(&source.raster)),
        ("dimensions", declares(&source.dimensions)),
        ("mipmaps", declares(&source.mipmaps)),
        ("compression", declares(&source.compression)),
        ("tiling", declares(&source.tiling)),
        ("atlas", declares(&source.atlas)),
        ("mesh_layout", declares(&source.mesh_layout)),
        ("uv", declares(&source.uv)),
        ("rig", declares(&source.rig)),
        ("audio", declares(&source.audio)),
        ("corpus", !source.corpora.is_empty()),
        ("path_family", !source.path_families.is_empty()),
        ("variant", !source.variants.is_empty()),
        ("resolution_chain", !source.resolution_chains.is_empty()),
        ("operation", !source.operations.is_empty()),
    ] {
        if present {
            inherited.push(name);
        }
    }
    Ok(ResolvedTarget {
        profile: effective,
        reused_from: Some(source.target.clone()),
        inherited,
    })
}
