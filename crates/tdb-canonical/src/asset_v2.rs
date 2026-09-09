//! `asset-output-profile/2` — the previous asset-profile schema, kept only so a committed `/2`
//! document has a **tested transition** into `asset-output-profile/3` rather than being silently
//! reinterpreted under rules it was never written against.
//!
//! # The transition demotes, and never promotes
//!
//! `/3` added the three keys that decide whether an established constraint is operative at all:
//! how far its evidence reaches ([`crate::asset::EVIDENCE_SCOPE`]), what it asserts about other
//! values ([`crate::asset::CONSTRAINT_FORCE`]), and which inspected [`crate::Corpus`] it rests on.
//! A `/2` document records **none** of them. There is no defensible way to infer one:
//!
//! * a `/2` `confidence = "measured"` says a measurement happened, not what population it covered —
//!   which is precisely the conflation C0-POR1-R1 found and `/3` exists to separate;
//! * a `/2` document has no corpus records at all, so nothing says whether the files behind a fact
//!   are the title's own or somebody's extraction; and
//! * attribution is exactly what decided which of C0-POR1's fourteen promotions survived review.
//!
//! So **every** `/2` block — `established` and `not-applicable` alike — comes out `unknown`, with its
//! value, confidence and provenance preserved in the non-operative [`crate::AssetCandidate`] channel
//! and an [`UpgradeNote`] recording the demotion. The resulting document declares **zero** established
//! constraints, which is the only conservative answer: re-establishing a fact means naming its corpus
//! and its scope, and that is authoring against evidence, not translation.
//!
//! `not-applicable` is demoted too, and deliberately. A decided absence is operative — it *satisfies*
//! an operation's requirement — so carrying one across unexamined would let a `/2`-era judgement keep
//! driving `/3` readiness without ever citing the corpus that judgement rested on.
//!
//! Demoting is not erasing, and C0-AS3-R1 makes the difference explicit in the **document** rather
//! than only in the transition report. An absence has no value to put in the candidate channel, so
//! the note carries what the `/2` block asserted and the reason it gave, verbatim, together with a
//! statement that the demotion is about missing scope rather than a finding that the assertion was
//! wrong. Re-deciding the block is then a matter of citing a corpus, not of rediscovering why
//! somebody wrote it down in the first place.
//!
//! # What is carried across unchanged
//!
//! `[[canonical_source]]` (it describes what the **Project** holds, not what a target requires),
//! target names and notes, and each target's `[[operation]]` requirement lists. Requirements are
//! copied verbatim: `/3` lets an operation narrow a requirement to one sub-property, and choosing to
//! narrow one is an authoring decision about what an operation actually needs.
//!
//! # What no transition can produce
//!
//! Corpora, path families and variants. All three are **new evidence**, not restatements: a `/2`
//! document does not know which files were inspected, does not know that a target has two path
//! families, and holds exactly one dimension per target — the limitation that made variants necessary.
//! The transition creates none of them, and the upgraded document therefore declares no variants.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::asset::{
    AssetCandidate, AssetConstraint, AssetProfile, CanonicalSource, OperationDecl, TargetProfile,
};
use crate::v1::UpgradeNote;

/// `defs/adapters/asset-profiles/<family>.toml` as it was written under `asset-output-profile/2`.
///
/// The constraint blocks are read as untyped tables. The transition does not need their shapes — it
/// demotes every one of them — and reading them structurally would mean maintaining a frozen copy of
/// fifteen `/2` payload structs whose only purpose is to be discarded.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AssetProfileV2 {
    pub schema: String,
    pub profile_version: u32,
    pub family: String,
    #[serde(default)]
    pub synthetic: bool,
    #[serde(default)]
    pub note: String,
    #[serde(default, rename = "canonical_source")]
    pub canonical_sources: Vec<CanonicalSource>,
    #[serde(default, rename = "target_profile")]
    pub targets: Vec<TargetProfileV2>,
}

/// One `/2` target profile: its name, its note, its operations, and its blocks as raw tables.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TargetProfileV2 {
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub note: String,
    #[serde(default, rename = "operation")]
    pub operations: Vec<OperationDecl>,
    /// Every remaining key, which is every constraint block the document declared.
    #[serde(flatten)]
    pub blocks: BTreeMap<String, toml::Value>,
}

/// The result of upgrading a `/2` profile: the `/3` document plus an explicit account of every
/// demotion.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetUpgradeV2 {
    pub profile: AssetProfile,
    pub notes: Vec<UpgradeNote>,
}

/// Parse an `asset-output-profile/2` document.
pub fn parse_asset_profile_v2(text: &str) -> Result<AssetProfileV2, toml::de::Error> {
    toml::from_str(text)
}

/// One `/2` block's envelope, as far as the transition needs to read it.
#[derive(Debug, Clone, Deserialize, Default)]
struct BlockV2 {
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    confidence: Option<String>,
    #[serde(default)]
    provenance: Vec<String>,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    note: String,
    #[serde(default)]
    value: Option<toml::Value>,
    #[serde(default)]
    candidate: Option<CandidateV2>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct CandidateV2 {
    #[serde(default)]
    confidence: Option<String>,
    #[serde(default)]
    provenance: Vec<String>,
    #[serde(default)]
    note: String,
    #[serde(default)]
    value: Option<toml::Value>,
}

/// Demote one `/2` block to a `/3` constraint.
///
/// An established or not-applicable `/2` value becomes a candidate; a `/2` candidate stays one. If
/// both are present the established value wins the candidate slot and the `/2` candidate is reported
/// as dropped, because two proposals in one channel cannot both be recorded and silently keeping
/// either would be a choice made by a translation.
fn demote<T>(
    family: &str,
    target: &str,
    block: &str,
    raw: &toml::Value,
    notes: &mut Vec<UpgradeNote>,
) -> AssetConstraint<T>
where
    T: serde::de::DeserializeOwned,
{
    let subject = format!("{family}/{target}");
    let parsed: BlockV2 = raw.clone().try_into().unwrap_or_default();
    let state = parsed
        .state
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let mut note = |field: &str, value: String, disposition: String| {
        notes.push(UpgradeNote {
            subject: subject.clone(),
            field: field.to_string(),
            value,
            disposition,
        });
    };

    let mut out = AssetConstraint::<T> {
        note: parsed.note.clone(),
        reason: parsed.reason.clone(),
        ..Default::default()
    };

    // A `/2` value, whatever state carried it, becomes a proposal.
    let carried = match state.as_str() {
        "established" => parsed.value.clone(),
        "not-applicable" => None,
        _ => None,
    };

    if state == "established" {
        note(
            &format!("[{block}].state"),
            "established".to_string(),
            format!(
                "demoted to \"unknown\". `/3` requires an established block to name its evidence \
                 SCOPE, its FORCE over other values and the [[corpus]] it rests on. A `/2` document \
                 records none of the three, and a measured value says nothing about which \
                 population it covers or whether that population is attributable to this target - \
                 the exact conflation `/3` exists to separate. The value is preserved in \
                 [{block}.candidate], where nothing acts on it"
            ),
        );
    } else if state == "not-applicable" {
        note(
            &format!("[{block}].state"),
            "not-applicable".to_string(),
            "demoted to \"unknown\", with the `/2` assertion and its reason preserved verbatim in \
             the note. A decided absence is OPERATIVE - it satisfies an operation's requirement - \
             so carrying one across would let a `/2`-era judgement keep driving `/3` readiness \
             without ever citing the corpus behind it. Re-deciding it needs a corpus and a scope"
                .to_string(),
        );
        // Demoting must not erase. What the `/2` document ASSERTED, and why, is the whole of the
        // evidence somebody would re-examine to decide the block again - and unlike the value
        // channel there is nowhere else for it to go, because an absence has no value to carry.
        let mut preserved = String::from(
            "`asset-output-profile/2` asserted this constraint NOT-APPLICABLE. It is demoted to \
             \"unknown\" because `/3` requires the evidence scope and the [[corpus]] a `/2` \
             document does not record - not because the assertion was found to be wrong.",
        );
        if parsed.reason.trim().is_empty() {
            preserved.push_str(
                " The `/2` block recorded no reason for the absence, which is itself part of why \
                 it cannot be carried across.",
            );
        } else {
            preserved.push_str("\n\nThe `/2` reason, verbatim: ");
            preserved.push_str(parsed.reason.trim());
        }
        out.note = if out.note.trim().is_empty() {
            preserved
        } else {
            format!("{}\n\n{}", out.note, preserved)
        };
        out.reason = String::new();
    }

    let candidate_value: Option<T> = carried
        .clone()
        .and_then(|v| v.try_into().ok())
        .or_else(|| {
            if carried.is_some() {
                note(
                    &format!("[{block}].value"),
                    "(unparseable under the /3 payload shape)".to_string(),
                    format!(
                        "dropped. `/3` changed this payload's shape, so the `/2` value could not be \
                         re-read as one. Nothing is guessed in its place; the block is unknown and \
                         [{block}] must be re-authored against evidence"
                    ),
                );
            }
            None
        });

    match (candidate_value, parsed.candidate.clone()) {
        (Some(value), existing) => {
            if existing.is_some() {
                note(
                    &format!("[{block}].candidate"),
                    "(the /2 candidate)".to_string(),
                    "dropped: the demoted established value took the candidate slot. Two \
                     proposals cannot occupy one channel, and choosing between them is an \
                     authoring decision rather than a translation"
                        .to_string(),
                );
            }
            out.candidate = Some(AssetCandidate {
                confidence: Some(match parsed.confidence.as_deref() {
                    Some("measured") => "measured".to_string(),
                    Some("derived") => "derived".to_string(),
                    // A `/2` confidence this transition cannot map is reported as `asserted`, the
                    // weakest term, never as the value it resembles.
                    _ => "asserted".to_string(),
                }),
                scope: None,
                corpus: None,
                provenance: if parsed.provenance.is_empty() {
                    vec![format!(
                        "carried from asset-output-profile/2 {family}/{target} [{block}], which \
                         cited nothing"
                    )]
                } else {
                    parsed.provenance.clone()
                },
                note: "Carried from an `asset-output-profile/2` established block. It \
                       records no evidence scope and no corpus, so `scope` and `corpus` are \
                       deliberately absent rather than filled in."
                    .to_string(),
                value: Some(value),
            });
        }
        (None, Some(existing)) => {
            let value: Option<T> = existing.value.clone().and_then(|v| v.try_into().ok());
            if value.is_some() {
                out.candidate = Some(AssetCandidate {
                    confidence: Some(
                        existing
                            .confidence
                            .clone()
                            .unwrap_or_else(|| "asserted".to_string()),
                    ),
                    scope: None,
                    corpus: None,
                    provenance: if existing.provenance.is_empty() {
                        vec![format!(
                            "carried from asset-output-profile/2 {family}/{target} \
                             [{block}.candidate], which cited nothing"
                        )]
                    } else {
                        existing.provenance.clone()
                    },
                    note: existing.note.clone(),
                    value,
                });
            } else {
                note(
                    &format!("[{block}.candidate]"),
                    "(unparseable under the /3 payload shape)".to_string(),
                    "dropped rather than guessed".to_string(),
                );
            }
        }
        (None, None) => {}
    }

    out
}

/// The three `/3` additions no translation can produce, and why. Reported once per target so the
/// gap sits in the record rather than in the reader's head.
const V3_ADDITION: &[(&str, &str)] = &[
    (
        "[[corpus]]",
        "not produced. A corpus records which files were inspected, whether that body is          attributable to this target and whether the build behind it was identified. A `/2`          document holds none of that, and attribution is exactly what decided which C0-POR1          promotions survived review",
    ),
    (
        "[[path_family]]",
        "not produced. `/2` held one path template per target and could not say that a target has          several path families, nor record which ids were OBSERVED in which directory as distinct          from an authoritative rule",
    ),
    (
        "[[variant]]",
        "not produced. `/2` held exactly one dimension per target, which is the limitation          variants exist to remove. Inventing variants from a single value would manufacture forms          nobody observed",
    ),
];

/// The `/2` blocks this transition reads, in [`crate::ASSET_BLOCK`] order. `runtime_lookup` is
/// absent: `/3` introduced it, so no `/2` document can carry one.
const V2_BLOCK: &[&str] = &[
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
];

/// Translate an `asset-output-profile/2` document into `asset-output-profile/3`.
///
/// Conservative by construction: the result declares **zero** established constraints, zero corpora,
/// zero path families and zero variants, and every demotion is reported. See the module docs for
/// why none of the three new evidence keys can be inferred from a `/2` document.
pub fn upgrade_asset_profile_v2(source: &AssetProfileV2) -> AssetUpgradeV2 {
    let mut notes = Vec::new();
    let mut targets = Vec::new();

    for v2 in &source.targets {
        let mut target = TargetProfile {
            target: v2.target.clone(),
            note: v2.note.clone(),
            operations: v2.operations.clone(),
            ..TargetProfile::default()
        };
        for block in V2_BLOCK {
            let Some(raw) = v2.blocks.get(*block) else {
                continue;
            };
            match *block {
                "discovery" => {
                    target.discovery = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "identity" => {
                    target.identity = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "container" => {
                    target.container = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "path_rule" => {
                    target.path_rule = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "raster" => {
                    target.raster = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "dimensions" => {
                    target.dimensions = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "mipmaps" => {
                    target.mipmaps = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "compression" => {
                    target.compression = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "tiling" => {
                    target.tiling = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "atlas" => {
                    target.atlas = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "mesh_layout" => {
                    target.mesh_layout = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "uv" => target.uv = demote(&source.family, &v2.target, block, raw, &mut notes),
                "rig" => target.rig = demote(&source.family, &v2.target, block, raw, &mut notes),
                "audio" => {
                    target.audio = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                "install" => {
                    target.install = demote(&source.family, &v2.target, block, raw, &mut notes)
                }
                _ => {}
            }
        }

        // The three things no translation can produce, reported once per target so the gap is in
        // the record rather than in the reader's head.
        for (field, disposition) in V3_ADDITION {
            notes.push(UpgradeNote {
                subject: format!("{}/{}", source.family, v2.target),
                field: field.to_string(),
                value: "(absent)".to_string(),
                disposition: disposition.to_string(),
            });
        }

        targets.push(target);
    }

    notes.sort();
    AssetUpgradeV2 {
        profile: AssetProfile {
            schema: crate::ASSET_PROFILE_SCHEMA.to_string(),
            profile_version: source.profile_version,
            family: source.family.clone(),
            synthetic: source.synthetic,
            note: source.note.clone(),
            canonical_sources: source.canonical_sources.clone(),
            targets,
        },
        notes,
    }
}
