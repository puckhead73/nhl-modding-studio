//! `asset-output-profile/1` — the previous asset-profile schema, kept only so the committed
//! document has a **tested transition** into `asset-output-profile/2` rather than being silently
//! reinterpreted.
//!
//! The transition is deliberately **conservative**, and for asset profiles that matters more than
//! it did for adapters, because v1's payload is entirely prose. `container_chain =
//! "BIGF->refpack->XPR2/RX2"` is a sentence, not a parsed container chain; `swizzle = "xbox-tiled"`
//! resembles a v2 vocabulary term but was never checked against one; and the whole-target
//! `confidence`/`qualification` pair was read off an **operations** CSV, which records what Studio
//! did, not what the format requires.
//!
//! So **nothing is promoted**. Every v2 constraint block comes out `unknown` with no candidate —
//! not even where a v1 string happens to spell a v2 term — and every v1 field is returned in
//! [`crate::Upgrade`]-style notes with the reason it did not become a constraint. Turning any of
//! those sentences into an established block is re-authoring against evidence, which a human does
//! with a citation, not a translation.

use serde::Deserialize;

use crate::asset::{AssetProfile, CanonicalSource, TargetProfile};
use crate::v1::UpgradeNote;

/// `defs/adapters/asset-profiles/<family>.toml` as it was written under `asset-output-profile/1`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AssetProfileV1 {
    pub schema: String,
    pub profile_version: u32,
    pub family: String,
    #[serde(default)]
    pub canonical_source: Option<CanonicalSourceV1>,
    #[serde(default, rename = "target_profile")]
    pub targets: Vec<TargetProfileV1>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSourceV1 {
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub format: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TargetProfileV1 {
    pub target: String,
    #[serde(default)]
    pub container_chain: String,
    #[serde(default)]
    pub raster_codec: String,
    #[serde(default)]
    pub dimensions: String,
    #[serde(default)]
    pub mipmaps: String,
    #[serde(default)]
    pub swizzle: String,
    #[serde(default)]
    pub atlas: String,
    #[serde(default)]
    pub install_rule: String,
    pub confidence: String,
    pub qualification: String,
    #[serde(default)]
    pub provenance: Vec<String>,
    #[serde(default)]
    pub note: String,
}

/// The result of upgrading a v1 profile: the v2 document plus an explicit account of everything the
/// transition could not carry across as a constraint.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetUpgrade {
    pub profile: AssetProfile,
    pub notes: Vec<UpgradeNote>,
}

/// The literal placeholder v1 used for an unestablished asset fact.
pub const V1_UNKNOWN: &str = "unknown";

/// Parse an `asset-output-profile/1` document.
pub fn parse_asset_profile_v1(text: &str) -> Result<AssetProfileV1, toml::de::Error> {
    toml::from_str(text)
}

/// Which v1 string maps onto which v2 block, purely so the note can say where the fact *would*
/// live once someone establishes it.
const V1_FIELD_DESTINATION: &[(&str, &str)] = &[
    ("container_chain", "[target_profile.container]"),
    ("raster_codec", "[target_profile.raster]"),
    ("dimensions", "[target_profile.dimensions]"),
    ("mipmaps", "[target_profile.mipmaps]"),
    ("swizzle", "[target_profile.tiling]"),
    ("atlas", "[target_profile.atlas]"),
    ("install_rule", "[target_profile.install]"),
];

/// Translate an `asset-output-profile/1` document into `asset-output-profile/2`.
///
/// Every constraint block comes out `unknown` with **no candidate**: a v1 string is undifferentiated
/// prose, and manufacturing a candidate from it would put an uncited proposal into the registry
/// under this function's authority rather than an author's.
///
/// The one thing carried across is `[canonical_source]`, and only when it is fully expressible:
/// it describes what the **Project** holds, not what a target requires, and its `kind`/`format`
/// are the two v1 fields that were already drawn from a fixed set. A `kind` or `format` outside the
/// v2 vocabularies is reported and dropped rather than written through — the resulting document
/// then declares no source and is refused by the validator, which is the correct outcome for a
/// profile whose canonical input is not expressible.
pub fn upgrade_asset_profile_v1(source: &AssetProfileV1) -> AssetUpgrade {
    let mut notes = Vec::new();
    let mut note = |subject: &str, field: &str, value: String, disposition: &str| {
        notes.push(UpgradeNote {
            subject: subject.to_string(),
            field: field.to_string(),
            value,
            disposition: disposition.to_string(),
        });
    };

    let mut canonical_sources = Vec::new();
    if let Some(v1_source) = &source.canonical_source {
        let subject = format!("{}/canonical_source", source.family);
        let class = crate::asset::ASSET_CLASS
            .contains(&v1_source.kind.as_str())
            .then(|| v1_source.kind.clone());
        let interchange = crate::asset::CANONICAL_INTERCHANGE
            .contains(&v1_source.format.as_str())
            .then(|| v1_source.format.clone());
        match (class, interchange) {
            (Some(class), Some(interchange)) => {
                note(
                    &subject,
                    "kind",
                    v1_source.kind.clone(),
                    "carried across as [[canonical_source]].class; v1 had no `role`, so the \
                     transition names it \"source\"",
                );
                canonical_sources.push(CanonicalSource {
                    role: "source".to_string(),
                    class,
                    interchange,
                    // v1 had no retention field. Q14's normalization decision is about the
                    // portrait LIBRARY, not about every family, so the transition asserts the
                    // weaker, always-true statement: the canonical source is retained unmodified.
                    retention: "unmodified".to_string(),
                    description: v1_source.description.clone(),
                });
            }
            (class, interchange) => {
                if class.is_none() {
                    note(
                        &subject,
                        "kind",
                        v1_source.kind.clone(),
                        "not one of the v2 asset classes; the source is dropped rather than \
                         guessed, and the resulting profile is refused for declaring none",
                    );
                }
                if interchange.is_none() {
                    note(
                        &subject,
                        "format",
                        v1_source.format.clone(),
                        "not one of the v2 canonical interchange representations; the source is \
                         dropped rather than guessed",
                    );
                }
            }
        }
    }

    let mut targets = Vec::new();
    for v1_target in &source.targets {
        let subject = format!("{}/{}", source.family, v1_target.target);
        let strings = [
            ("container_chain", &v1_target.container_chain),
            ("raster_codec", &v1_target.raster_codec),
            ("dimensions", &v1_target.dimensions),
            ("mipmaps", &v1_target.mipmaps),
            ("swizzle", &v1_target.swizzle),
            ("atlas", &v1_target.atlas),
            ("install_rule", &v1_target.install_rule),
        ];
        for (field, value) in strings {
            if value.trim().is_empty() {
                continue;
            }
            let destination = V1_FIELD_DESTINATION
                .iter()
                .find(|(id, _)| *id == field)
                .map(|(_, d)| *d)
                .unwrap_or("(no v2 destination)");
            let disposition = if value.trim() == V1_UNKNOWN {
                format!(
                    "v1 placeholder string; v2 spells an unestablished fact as {destination} left \
                     at state = \"unknown\""
                )
            } else {
                format!(
                    "free text, not a structured constraint. Its v2 home is {destination}; \
                     establishing it there needs a citation and a state, which re-authoring \
                     supplies and a translation cannot"
                )
            };
            note(&subject, field, value.clone(), &disposition);
        }
        note(
            &subject,
            "confidence",
            v1_target.confidence.clone(),
            "v1 carried ONE evidence label for the whole target; v2 carries a state, confidence, \
             provenance and derivation per constraint block, so there is nothing for a single \
             target-wide label to attach to",
        );
        note(
            &subject,
            "qualification",
            v1_target.qualification.clone(),
            "an implementation/evidence level describes what STUDIO can do, not what the target \
             format requires. v2 profiles carry no converter status at all; that belongs to the \
             evidence system",
        );
        if !v1_target.provenance.is_empty() {
            note(
                &subject,
                "provenance",
                v1_target.provenance.join(", "),
                "a target-wide citation cannot be attached to any particular v2 block; re-cite it \
                 on the specific constraint it supports",
            );
        }

        targets.push(TargetProfile {
            target: v1_target.target.clone(),
            note: v1_target.note.clone(),
            ..TargetProfile::default()
        });
    }

    notes.sort();
    AssetUpgrade {
        profile: AssetProfile {
            schema: crate::ASSET_PROFILE_SCHEMA.to_string(),
            profile_version: source.profile_version,
            family: source.family.clone(),
            synthetic: false,
            note: String::new(),
            canonical_sources,
            targets,
        },
        notes,
    }
}
