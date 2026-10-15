//! `asset-output-profile/4` — composite identities and ordered resolution chains (package C0-AS4).
//!
//! Two things `/3` could represent but not spell as rules:
//!
//! 1. an asset id built from **several database fields** (a Legacy kit is the `xjgw` row's
//!    `{style}_{teamartid}_{variant}`), which `/3` stood in as a single `{variant}` token; and
//! 2. an **ordered fallback** between files, some of which are **shared** by many assets (a kit's own
//!    thumbnail, then the `custom*.big` defaults every kit without stock art shows), which `/3`
//!    could not order and could not protect from a write.
//!
//! This module holds the `/4` value types and the **pure** construction and resolution functions a
//! later ingestion or Build step can call. Nothing here reads a database, touches a file system or
//! knows what the game does: a chain's order is a declared reading order, and
//! [`crate::asset::RuntimeLookupValue`] remains the only place runtime behaviour is recorded. The
//! validator in `asset.rs` checks the declarations; the functions here check the *instances*.
//!
//! Every failure is a typed blocker rather than a default. A part whose value is absent yields no
//! id; a rendered path that could escape its root is refused before it exists; a write through a
//! chain resolves only an `owned` destination and never falls through to a `shared` one.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::asset::{AssetConstraint, PathFamily, TargetProfile};
use crate::ConstraintState;

// ============================================================= vocabularies

/// Closed vocabulary: how one composite part is rendered into text.
///
/// `decimal` — the unsigned stored value in base 10, no padding. `zero-padded:<n>` — base 10 padded
/// with leading zeros to exactly `n` digits; a value that needs more digits is a blocker, not a
/// truncation. `lower-hex` — base 16, lowercase, no prefix. `slug` — a text part reduced to the
/// name-slug alphabet (`[a-z0-9_]`, as the portrait slug); an empty result is a blocker.
pub const COMPOSITE_RENDER: &[&str] = &["decimal", "zero-padded", "lower-hex", "slug"];

/// Closed vocabulary: what a chain is declared for.
pub const CHAIN_PURPOSE: &[&str] = &["read", "write", "read-write"];

/// Closed vocabulary: whether a chain step's file belongs to one asset or to many.
pub const STEP_OWNERSHIP: &[&str] = &["owned", "shared"];

/// The path token a template substitutes with a rendered composite id.
pub const COMPOSITE_TOKEN: &str = "composite-id";

/// The prefix an `[[operation]].requires` entry uses to name a resolution chain.
pub const CHAIN_REQUIREMENT_PREFIX: &str = "resolution_chain:";

// =============================================================== documents

/// `[identity.value.composite]` — how a composite id is built from named database fields.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompositeIdentity {
    /// The id text with `{part}` placeholders, e.g. `"{style}_{art}_{variant}"`. Every placeholder
    /// must name a declared part and every part must be used.
    pub format: String,
    #[serde(default, rename = "part")]
    pub parts: Vec<CompositePart>,
    #[serde(default)]
    pub note: String,
}

/// One field a composite id is built from.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompositePart {
    /// The placeholder name: `[a-z][a-z0-9-]*`, unique within the composite.
    pub name: String,
    /// The target table id, as the target adapter's `[[table]]` declares it.
    pub table: String,
    /// The target field id within that table.
    pub field: String,
    /// One of [`COMPOSITE_RENDER`], with `zero-padded:<n>` carrying its width.
    pub render: String,
    /// The stored code that means "no value", **only** where evidence establishes one. Absent means
    /// every stored code is a value: a 0 is then a 0, never a missing part.
    #[serde(default)]
    pub null_code: Option<i64>,
    #[serde(default)]
    pub note: String,
}

/// `[[target_profile.resolution_chain]]` — an ordered reading (and, where declared, writing) order
/// over declared path families, carried in the same evidence envelope as every other block so an
/// `unknown` or `specimen-observation` chain can never enable an operation.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResolutionChain {
    /// Unique within the target; what an `[[operation]].requires` entry
    /// `resolution_chain:<id>` names.
    pub id: String,
    /// One of [`CHAIN_PURPOSE`].
    pub purpose: String,
    #[serde(default)]
    pub note: String,
    pub declared: AssetConstraint<ChainValue>,
}

/// The ordered steps. Order is the declared reading order and nothing more.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChainValue {
    #[serde(default, rename = "step")]
    pub steps: Vec<ChainStep>,
}

/// One step: a declared path family and whether the file it resolves belongs to one asset.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChainStep {
    /// A `[[path_family]]` id of the same target.
    pub family: String,
    /// One of [`STEP_OWNERSHIP`].
    pub ownership: String,
    #[serde(default)]
    pub note: String,
}

// ======================================================== composite construction

/// What a caller found in the database for one part. The caller decides `Absent` (no row) and the
/// function decides `null_code`; nothing else is interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartValue {
    /// No row, or no readable field, for this part.
    Absent,
    /// An unsigned stored code.
    Code(u64),
    /// Stored text, for `slug` parts.
    Text(String),
}

/// Why a composite id could not be constructed. Never a default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositeBlocker {
    /// The part's row or field was absent.
    MissingPart { part: String },
    /// The part stored its declared `null_code`.
    NullPart { part: String, code: i64 },
    /// The stored value cannot be rendered under the part's declared rendering.
    UnrenderablePart { part: String, reason: String },
    /// The declaration itself is unusable (a placeholder with no part, an unknown rendering). The
    /// validator reports these on the profile; this is the instance-level echo.
    InvalidDeclaration { reason: String },
}

/// Parse `render` into its kind and, for `zero-padded:<n>`, its width.
pub fn parse_render(render: &str) -> Option<(&str, Option<usize>)> {
    match render.split_once(':') {
        None if COMPOSITE_RENDER.contains(&render) && render != "zero-padded" => {
            Some((render, None))
        }
        Some(("zero-padded", width)) => {
            let width: usize = width.parse().ok()?;
            (1..=16)
                .contains(&width)
                .then_some(("zero-padded", Some(width)))
        }
        _ => None,
    }
}

/// The `{name}` placeholders of a format string, in order of first appearance, with duplicates
/// kept so the validator can report them.
pub fn format_placeholders(format: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = format;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            break;
        };
        out.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    out
}

/// Whether a part or family name is spelled the way the schema requires: `[a-z][a-z0-9-]*`.
pub fn is_part_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Reduce text to the slug alphabet the portrait families use: ASCII letters lowered, digits kept,
/// every other run collapsed to one `_`, no leading or trailing `_`.
pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut pending = false;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            if pending && !out.is_empty() {
                out.push('_');
            }
            pending = false;
            out.push(c.to_ascii_lowercase());
        } else {
            pending = true;
        }
    }
    out
}

/// Render one part.
pub fn render_part(part: &CompositePart, value: &PartValue) -> Result<String, CompositeBlocker> {
    let Some((kind, width)) = parse_render(&part.render) else {
        return Err(CompositeBlocker::InvalidDeclaration {
            reason: format!(
                "part {:?} renders as {:?}, which is not a rendering",
                part.name, part.render
            ),
        });
    };
    let name = part.name.clone();
    match value {
        PartValue::Absent => Err(CompositeBlocker::MissingPart { part: name }),
        PartValue::Code(code) => {
            if let Some(null) = part.null_code {
                if i64::try_from(*code).is_ok_and(|c| c == null) {
                    return Err(CompositeBlocker::NullPart {
                        part: name,
                        code: null,
                    });
                }
            }
            match kind {
                "decimal" => Ok(code.to_string()),
                "lower-hex" => Ok(format!("{code:x}")),
                "zero-padded" => {
                    let width = width.unwrap_or(1);
                    let text = code.to_string();
                    if text.len() > width {
                        Err(CompositeBlocker::UnrenderablePart {
                            part: name,
                            reason: format!(
                                "{code} needs {} digits, the rendering allows {width}",
                                text.len()
                            ),
                        })
                    } else {
                        Ok(format!("{code:0>width$}"))
                    }
                }
                _ => Err(CompositeBlocker::UnrenderablePart {
                    part: name,
                    reason: "a stored code cannot render as a slug; the part is not text".into(),
                }),
            }
        }
        PartValue::Text(text) => {
            if kind != "slug" {
                return Err(CompositeBlocker::UnrenderablePart {
                    part: name,
                    reason: format!("text cannot render as {kind}"),
                });
            }
            let slug = slugify(text);
            if slug.is_empty() {
                Err(CompositeBlocker::UnrenderablePart {
                    part: name,
                    reason: "the text reduces to an empty slug".into(),
                })
            } else {
                Ok(slug)
            }
        }
    }
}

/// Build the composite id for one instance. `lookup` returns what the database holds for each
/// declared part, by part name; every part is rendered in declaration order, and the first blocker
/// stops construction — nothing is substituted for a missing part.
pub fn render_composite(
    composite: &CompositeIdentity,
    lookup: impl Fn(&CompositePart) -> PartValue,
) -> Result<String, CompositeBlocker> {
    let placeholders = format_placeholders(&composite.format);
    let mut rendered: BTreeMap<&str, String> = BTreeMap::new();
    for part in &composite.parts {
        rendered.insert(part.name.as_str(), render_part(part, &lookup(part))?);
    }
    for placeholder in &placeholders {
        if !rendered.contains_key(placeholder.as_str()) {
            return Err(CompositeBlocker::InvalidDeclaration {
                reason: format!("format substitutes {{{placeholder}}}, which no part declares"),
            });
        }
    }
    let mut out = composite.format.clone();
    for (name, text) in &rendered {
        out = out.replace(&format!("{{{name}}}"), text);
    }
    Ok(out)
}

/// Group the instances whose rendered id is the same. `rows` pairs a caller-chosen row key with a
/// rendered id; the result holds only the ids more than one row renders, each with its rows in
/// input order. An empty map is what "the ids are distinct" looks like — and it is a measurement
/// about the rows given, not a property of the declaration.
pub fn collisions<K: Clone + Ord>(
    rows: impl IntoIterator<Item = (K, String)>,
) -> BTreeMap<String, Vec<K>> {
    let mut by_id: BTreeMap<String, Vec<K>> = BTreeMap::new();
    for (key, id) in rows {
        by_id.entry(id).or_default().push(key);
    }
    by_id.retain(|_, rows| rows.len() > 1);
    by_id
}

// ============================================================ path rendering

/// Why a path could not be produced. A refused path never exists as a string a caller could use
/// by mistake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathBlocker {
    /// The template substitutes a token the caller did not supply.
    MissingToken { token: String },
    /// A token value carries a separator, a traversal or a control character, so substituting it
    /// would change the path's shape.
    UnsafeTokenValue { token: String, value: String },
    /// The rendered path is not a plain relative path under its root: empty, absolute, a drive
    /// prefix, a `..` component, or a control character.
    UnsafePath { path: String },
}

/// Whether one token value may be substituted: no separators, no traversal, no control
/// characters, not empty.
pub fn is_safe_token_value(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains(['/', '\\', ':'])
        && !value.chars().any(|c| c.is_control())
}

/// Whether a rendered path is a plain relative path: the guard `studio_domain::extract::safe_join`
/// and `studio_domain::assets::safe_join_rel` apply at write time, restated on the string so a
/// profile-driven path is refused before any root is joined. Separators are `/` (EA's) and `\`;
/// every component must be a plain name — no empty path, no `.`/`..`, no absolute root, no drive
/// prefix (`c:`), no control characters.
pub fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty() || path.chars().any(|c| c.is_control()) {
        return false;
    }
    if path.starts_with(['/', '\\']) {
        return false;
    }
    let mut pushed = false;
    for raw in path.split(['/', '\\']) {
        if raw.is_empty() || raw == "." {
            continue;
        }
        if raw == ".." || raw.contains(':') {
            return false;
        }
        pushed = true;
    }
    pushed
}

/// Substitute a family's tokens and refuse anything unsafe. `tokens` maps token name to value; a
/// composite id is supplied under [`COMPOSITE_TOKEN`] like any other token, so this function does
/// not know or care how it was built.
pub fn render_path(
    family: &PathFamily,
    tokens: &BTreeMap<&str, &str>,
) -> Result<String, PathBlocker> {
    let mut out = family.template.clone();
    for token in &family.tokens {
        let Some(value) = tokens.get(token.as_str()) else {
            return Err(PathBlocker::MissingToken {
                token: token.clone(),
            });
        };
        if !is_safe_token_value(value) {
            return Err(PathBlocker::UnsafeTokenValue {
                token: token.clone(),
                value: (*value).to_string(),
            });
        }
        out = out.replace(&format!("{{{token}}}"), value);
    }
    let out = match family.case.as_str() {
        "lower" => out.to_ascii_lowercase(),
        "upper" => out.to_ascii_uppercase(),
        _ => out,
    };
    if !is_safe_relative_path(&out) {
        return Err(PathBlocker::UnsafePath { path: out });
    }
    Ok(out)
}

// ========================================================== chain resolution

/// What a read through a chain found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadResolution {
    /// The first step, in declared order, whose rendered path exists.
    Found {
        step: usize,
        family: String,
        path: String,
        shared: bool,
    },
    /// No step's path exists. The rendered paths are reported so the caller can say what was looked
    /// for, in order.
    NotFound { looked_for: Vec<String> },
}

/// Why a write could not be given a destination. Never a shared file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteBlocker {
    /// The chain is not declared for writing.
    ChainNotForWriting { purpose: String },
    /// The chain is not established at a scope that speaks for the target, so it authorises nothing.
    ChainNotOperative,
    /// The chain has no `owned` step: every file it resolves is shared, and a write to a shared file
    /// would change every asset that falls back to it.
    NoOwnedStep,
    /// The owned file does not exist, and creating it needs support the target does not establish.
    /// `missing` names the prerequisites (see [`CreationSupport`]).
    CreationUnsupported {
        path: String,
        missing: Vec<&'static str>,
    },
    /// The owned step's path could not be rendered.
    Path(PathBlocker),
    /// The owned step's rendered destination is the same file as a `shared` step's — in this chain
    /// or in any other chain of the target — once separators and declared casing are normalised.
    /// Two templates that differ as text are not two files (C0-AS4-R1).
    AliasesSharedDestination {
        path: String,
        shared_family: String,
        chain: String,
    },
    /// Whether the owned destination is distinct from a `shared` one could not be decided from
    /// the supplied context: the shared family needs a token the caller did not supply, or the
    /// two paths differ only in case (equality then depends on the destination's case
    /// sensitivity, which no profile field states). Refused rather than guessed.
    SharedDestinationUndetermined {
        path: String,
        shared_family: String,
        chain: String,
        reason: String,
    },
}

/// Where a write goes. Only ever an `owned` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteResolution {
    /// The owned file exists; the write replaces it.
    Existing {
        step: usize,
        family: String,
        path: String,
    },
    /// The owned file does not exist and every creation prerequisite is established; the write
    /// creates it.
    Create {
        step: usize,
        family: String,
        path: String,
    },
}

/// What creating a missing owned file needs, read off the target's established blocks. Every field
/// is a fact the target profile decides; none is inferred from the chain.
///
/// **Describing a format is not the capability to create in it** (C0-AS4-R1). A profile that
/// establishes how an archive member is installed, or which container wraps a payload, records what
/// the target *has*; nothing in this crate can produce a new archive member or a new wrapped
/// payload, and no block represents such a capability. So creation is supported only for the one
/// shape the resolver can actually describe end to end — a loose file with no container and no
/// reference to rewrite — and every other combination is reported as unsupported, by name.
///
/// What the resolver's result then means: `Create` says "this loose path may be written new";
/// it does not say the bytes are right, that the game will read them, or that any reference was
/// updated. `Existing` says "this owned path exists and is not a shared destination"; it says
/// nothing about the file's contents or format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreationSupport {
    /// `[install]` is operative **and** its kind is `loose-file`. An archive install kind describes
    /// how the target stores the file; inserting a new archive member is not represented.
    pub install: bool,
    /// `[container]` is decided **absent** (`not-applicable`). An established container describes
    /// the wrapping an existing file has; creating a new wrapped payload is not represented, so an
    /// established container does not count.
    pub container: bool,
    /// `[install].updates_references` is false. When an install must rewrite references, no
    /// reference-update plan or capability exists in this schema, and an operative `[identity]`
    /// is not one — it says what a reference *is*, not that anything will write it.
    pub references: bool,
}

impl CreationSupport {
    /// Read off a target profile.
    pub fn of(target: &TargetProfile) -> Self {
        let install = target
            .install
            .operative()
            .is_some_and(|v| v.kind == "loose-file");
        let container = target.container.state == ConstraintState::NotApplicable;
        let references = target
            .install
            .operative()
            .is_some_and(|v| !v.updates_references);
        CreationSupport {
            install,
            container,
            references,
        }
    }

    fn missing(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if !self.install {
            out.push("install");
        }
        if !self.container {
            out.push("container");
        }
        if !self.references {
            out.push("references");
        }
        out
    }
}

/// One rendered path in comparison form: `\\` and `/` both read as `/`, empty and `.` segments
/// dropped. Case is **not** folded here — whether two spellings that differ only in case are one
/// file is a property of the destination, which nothing in a profile states; see [`paths_alias`].
pub fn normalize_path(path: &str) -> String {
    path.split(['/', '\\'])
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether two rendered paths name one destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathAlias {
    /// The normalised paths are equal.
    Same,
    /// They differ in more than case.
    Distinct,
    /// They differ only in case. Whether that is one file or two depends on the destination's
    /// case sensitivity, which no profile field states and a family's rendering policy does not
    /// establish (C0-AS4-CLOSE): a `lower` family and an `upper` family describe how each spells
    /// its output, not how the file system compares them. Undecidable here, so refused.
    CaseUndetermined,
}

/// Compare two rendered paths. Exact normalised equality is `Same`; a difference beyond case is
/// `Distinct`; a case-only difference is `CaseUndetermined` **whatever the families' declared
/// casing** — declared casing is a rendering rule, not a statement about the destination.
pub fn paths_alias(a: &str, b: &str) -> PathAlias {
    let (na, nb) = (normalize_path(a), normalize_path(b));
    if na == nb {
        return PathAlias::Same;
    }
    if na.eq_ignore_ascii_case(&nb) {
        return PathAlias::CaseUndetermined;
    }
    PathAlias::Distinct
}

/// Every `shared` step declared on the target, in every chain (established or candidate), as
/// `(chain id, family)`. A shared declaration anywhere on the target excludes that destination
/// from every write, not only from writes through the chain that declared it.
pub fn shared_steps(target: &TargetProfile) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for chain in &target.resolution_chains {
        let Some(value) = chain
            .declared
            .established()
            .or(chain.declared.candidate_value())
        else {
            continue;
        };
        for step in &value.steps {
            if step.ownership == "shared" {
                out.push((chain.id.clone(), step.family.clone()));
            }
        }
    }
    out
}

/// The resolved-path exclusion (C0-AS4-R1): the owned destination must be shown distinct from
/// every shared destination on the target, by rendered path and not by template text. A shared
/// family that cannot be rendered from the supplied tokens, or a case-only difference, is not
/// "distinct" — it is undetermined, and refused as such.
fn check_not_shared(
    target: &TargetProfile,
    path: &str,
    tokens: &BTreeMap<&str, &str>,
) -> Result<(), WriteBlocker> {
    for (chain, family_id) in shared_steps(target) {
        let Some(family) = family_of(target, &family_id) else {
            continue;
        };
        let shared_path = match render_path(family, tokens) {
            Ok(p) => p,
            Err(PathBlocker::MissingToken { token }) => {
                return Err(WriteBlocker::SharedDestinationUndetermined {
                    path: path.to_string(),
                    shared_family: family_id,
                    chain,
                    reason: format!(
                        "the shared family needs token {token:?}, which was not supplied, so its \
                         destination cannot be compared"
                    ),
                })
            }
            // A shared family whose own rendering is unsafe cannot be the file being written;
            // static validation reports the template, and the comparison has nothing to do.
            Err(_) => continue,
        };
        match paths_alias(path, &shared_path) {
            PathAlias::Distinct => {}
            PathAlias::Same => {
                return Err(WriteBlocker::AliasesSharedDestination {
                    path: path.to_string(),
                    shared_family: family_id,
                    chain,
                })
            }
            PathAlias::CaseUndetermined => {
                return Err(WriteBlocker::SharedDestinationUndetermined {
                    path: path.to_string(),
                    shared_family: family_id,
                    chain,
                    reason: format!(
                        "differs from {shared_path:?} only in case; whether the destination treats \
                         those as one file is not stated by any profile field and is not decided \
                         here"
                    ),
                })
            }
        }
    }
    Ok(())
}

fn family_of<'a>(target: &'a TargetProfile, id: &str) -> Option<&'a PathFamily> {
    target.path_families.iter().find(|f| f.id == id)
}

/// Follow a chain's declared order for a read: the first step whose rendered path `exists` wins,
/// shared or owned. Only an **operative** chain may be followed; an unknown or candidate chain
/// yields nothing, and neither does one declared for writing only. `exists` is the caller's
/// file-system or archive-member probe — this function never touches storage.
pub fn resolve_read(
    target: &TargetProfile,
    chain: &ResolutionChain,
    tokens: &BTreeMap<&str, &str>,
    exists: impl Fn(&str) -> bool,
) -> Option<ReadResolution> {
    if !matches!(chain.purpose.as_str(), "read" | "read-write") {
        return None;
    }
    let value = chain.declared.operative()?;
    let mut looked_for = Vec::new();
    for (index, step) in value.steps.iter().enumerate() {
        let family = family_of(target, &step.family)?;
        let Ok(path) = render_path(family, tokens) else {
            continue;
        };
        if exists(&path) {
            return Some(ReadResolution::Found {
                step: index,
                family: step.family.clone(),
                path,
                shared: step.ownership == "shared",
            });
        }
        looked_for.push(path);
    }
    Some(ReadResolution::NotFound { looked_for })
}

/// Resolve the destination of a write through a chain: the **first `owned` step**, whether or not
/// its file exists, and never anything else. A read may have found the asset on a shared step; a
/// write ignores that entirely. Before either outcome the rendered destination is checked against
/// every `shared` destination on the target ([`check_not_shared`]): an owned step whose rendering
/// lands on a shared file is refused, and one that cannot be shown distinct is refused too. When
/// the owned file is missing, creation is authorised only if [`CreationSupport`] is complete;
/// otherwise the blocker names what is missing.
///
/// This is logical resolution over declared families and supplied tokens — not a file-system
/// operation, not race-proof against concurrent writers, and not an installer.
pub fn resolve_write(
    target: &TargetProfile,
    chain: &ResolutionChain,
    tokens: &BTreeMap<&str, &str>,
    exists: impl Fn(&str) -> bool,
) -> Result<WriteResolution, WriteBlocker> {
    if !matches!(chain.purpose.as_str(), "write" | "read-write") {
        return Err(WriteBlocker::ChainNotForWriting {
            purpose: chain.purpose.clone(),
        });
    }
    let Some(value) = chain.declared.operative() else {
        return Err(WriteBlocker::ChainNotOperative);
    };
    let Some((index, step)) = value
        .steps
        .iter()
        .enumerate()
        .find(|(_, s)| s.ownership == "owned")
    else {
        return Err(WriteBlocker::NoOwnedStep);
    };
    let family = family_of(target, &step.family).ok_or(WriteBlocker::NoOwnedStep)?;
    let path = render_path(family, tokens).map_err(WriteBlocker::Path)?;
    check_not_shared(target, &path, tokens)?;
    if exists(&path) {
        return Ok(WriteResolution::Existing {
            step: index,
            family: step.family.clone(),
            path,
        });
    }
    let support = CreationSupport::of(target);
    let missing = support.missing();
    if missing.is_empty() {
        Ok(WriteResolution::Create {
            step: index,
            family: step.family.clone(),
            path,
        })
    } else {
        Err(WriteBlocker::CreationUnsupported { path, missing })
    }
}

/// The families every chain of a target marks `shared`, and those it marks `owned`. A family in
/// both sets is what the validator refuses: a shared file must not become writable through another
/// chain that happens to call it owned.
pub fn ownership_sets(target: &TargetProfile) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut shared = BTreeSet::new();
    let mut owned = BTreeSet::new();
    for chain in &target.resolution_chains {
        let Some(value) = chain
            .declared
            .established()
            .or(chain.declared.candidate_value())
        else {
            continue;
        };
        for step in &value.steps {
            match step.ownership.as_str() {
                "shared" => shared.insert(step.family.clone()),
                "owned" => owned.insert(step.family.clone()),
                _ => false,
            };
        }
    }
    (shared, owned)
}
