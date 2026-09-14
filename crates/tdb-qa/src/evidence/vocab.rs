//! Closed vocabularies — [contract §4.1](../../../../docs/plans/qa-evidence-contract.md) and the
//! R2 rule **R-APPL-OPS** (§12.7).
//!
//! Every vocabulary here is closed: an unknown token is a hard validation error (V10), never a
//! warning and never a silently accepted value.

use crate::catalog::{UnitKind, CATALOG};

/// What an operation *does*, which is what decides whether game acceptance can be required of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationClass {
    /// Reads or reports only. Never requires game acceptance (R-APPL-OPS clause 2).
    ReadOnly,
    /// Produces or installs bytes a game will consume.
    Write,
    /// Manual acceptance on the real title/platform/build.
    Acceptance,
}

/// One declared operation of a family.
#[derive(Debug, Clone, Copy)]
pub struct OperationDef {
    pub id: &'static str,
    pub class: OperationClass,
    /// The family-level declaration of contract §4.1. The *effective* requirement is resolved by
    /// [`applicability`], which applies R-APPL-OPS on top of this.
    pub declared_required: bool,
}

const fn op(id: &'static str, class: OperationClass, declared_required: bool) -> OperationDef {
    OperationDef {
        id,
        class,
        declared_required,
    }
}

use OperationClass::{Acceptance, ReadOnly, Write};

const DB_PRESENTATION: &[OperationDef] = &[op("read", ReadOnly, true)];

const DB_OPERATIONS: &[OperationDef] = &[
    op("scalar-edit", Write, true),
    op("roster-create", Write, true),
    op("roster-move", Write, true),
    op("roster-trade", Write, true),
    op("roster-delete", Write, true),
    op("game-acceptance", Acceptance, true),
];

const SAVE_RAW_DB: &[OperationDef] = &[
    op("save", Write, true),
    op("reopen", ReadOnly, true),
    op("game-acceptance", Acceptance, true),
];

const SAVE_INNER_CONTAINER: &[OperationDef] = &[
    op("container-detect", ReadOnly, true),
    op("container-extract", ReadOnly, true),
    op("container-repack", Write, true),
    op("game-acceptance", Acceptance, true),
];

const SAVE_OUTER_PACKAGE: &[OperationDef] = &[
    op("outer-package", Write, true),
    op("game-acceptance", Acceptance, true),
];

const ONBOARDING: &[OperationDef] = &[
    op("discover", ReadOnly, true),
    op("identify", ReadOnly, false),
];

const ASSETS: &[OperationDef] = &[
    op("discover", ReadOnly, false),
    op("identify", ReadOnly, false),
    op("preview", ReadOnly, false),
    op("export", ReadOnly, false),
    op("replace-modified", Write, true),
    op("rebuild", Write, false),
    op("install", Write, true),
    op("game-acceptance", Acceptance, true),
];

/// The closed operation set of a family, or `None` when the family id is unknown.
pub fn operation_defs(family: &str) -> Option<&'static [OperationDef]> {
    match family {
        "db-presentation" => Some(DB_PRESENTATION),
        "db-operations" => Some(DB_OPERATIONS),
        "save-raw-db" => Some(SAVE_RAW_DB),
        "save-inner-container" => Some(SAVE_INNER_CONTAINER),
        "save-outer-package" => Some(SAVE_OUTER_PACKAGE),
        f if f.starts_with("onboarding-") => Some(ONBOARDING),
        f if f.starts_with("assets-") => Some(ASSETS),
        _ => None,
    }
}

/// Operation ids of a family, for the key vocabulary check (V1).
pub fn operations_for_family(family: &str) -> Option<Vec<&'static str>> {
    operation_defs(family).map(|defs| defs.iter().map(|d| d.id).collect())
}

/// Class of one operation within one family.
pub fn operation_class(family: &str, operation: &str) -> Option<OperationClass> {
    operation_defs(family)?
        .iter()
        .find(|d| d.id == operation)
        .map(|d| d.class)
}

/// Whether a coverage key's operation is conversion-dependent, i.e. V12 requires the `conversion`
/// component. Conversion dependence is a property of the *family and operation*, not of the target:
/// an asset family's producing operations depend on where the source asset came from.
pub fn requires_conversion(family: &str, operation: &str) -> bool {
    family == "assets-heads"
        && matches!(
            operation,
            "replace-modified" | "rebuild" | "install" | "game-acceptance"
        )
}

/// Contract §4.3 axis: `applicability`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    Required,
    Optional,
    NotApplicable,
    Unknown,
}

impl Applicability {
    pub fn as_str(self) -> &'static str {
        match self {
            Applicability::Required => "required",
            Applicability::Optional => "optional",
            Applicability::NotApplicable => "not-applicable",
            Applicability::Unknown => "unknown",
        }
    }
}

/// The resolved applicability of one cell, with the reason V15 requires whenever it is
/// `not-applicable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedApplicability {
    pub value: Applicability,
    /// Non-empty exactly when `value == NotApplicable` (V15), and free-text otherwise unused.
    pub reason: String,
}

/// Look up the catalog unit for a `<title>-<platform>` target.
pub fn unit_kind(target: &str) -> Option<UnitKind> {
    let (game, platform) = target.split_once('-')?;
    CATALOG
        .iter()
        .find(|u| u.game == game && u.platform == platform)
        .map(|u| u.kind)
}

/// **R-APPL-OPS** (§12.7). Resolution order is the contract's, and every `not-applicable` carries a
/// cited reason.
///
/// Returns `None` when the family/operation pair is not in the vocabulary at all.
pub fn applicability(target: &str, family: &str, operation: &str) -> Option<ResolvedApplicability> {
    let def = operation_defs(family)?.iter().find(|d| d.id == operation)?;

    // 1. Research-only units ship nothing, so no write or acceptance cell applies to them.
    if unit_kind(target) == Some(UnitKind::Research)
        && matches!(
            def.class,
            OperationClass::Write | OperationClass::Acceptance
        )
    {
        return Some(ResolvedApplicability {
            value: Applicability::NotApplicable,
            reason: "research-only-unit: roadmap §1 — NHL 07 PS3 is research-only, not a shipping target"
                .to_string(),
        });
    }

    // A target absent from the catalog is an unknown, never an assumed default.
    if unit_kind(target).is_none() {
        return Some(ResolvedApplicability {
            value: Applicability::Unknown,
            reason: String::new(),
        });
    }

    // 2. Game acceptance is required only where the family has a required write operation here.
    if def.class == OperationClass::Acceptance {
        let has_required_write = operation_defs(family)?.iter().any(|d| {
            d.class == OperationClass::Write
                && d.declared_required
                && applicability(target, family, d.id)
                    .is_some_and(|a| a.value == Applicability::Required)
        });
        return Some(ResolvedApplicability {
            value: if has_required_write {
                Applicability::Required
            } else {
                Applicability::Optional
            },
            reason: String::new(),
        });
    }

    // 3/4. R-Q04 keeps `save-outer-package` required on the three console platforms; otherwise the
    // §4.1 declaration stands.
    Some(ResolvedApplicability {
        value: if def.declared_required {
            Applicability::Required
        } else {
            Applicability::Optional
        },
        reason: String::new(),
    })
}

/// Applicability of a whole key, including its `variant` component.
///
/// **A variant cell's applicability is `unknown`, deliberately.** Contract §2.2 admits a `variant`
/// "where it changes coverage", but no document declares *which* variants a target must cover, and
/// [`catalog_universe`] therefore enumerates variant-less cells only. Two things follow, and both
/// are conservative on purpose:
///
/// * a variant cell is never `required`, because inventing a required-variant catalog here would
///   manufacture requirements no accepted document states;
/// * a variant cell is never `automated-qualified` either — `unknown` applicability withholds
///   (§4.3 row 2) — so evidence recorded against `…/container-repack/v4` cannot quietly stand in
///   for the variant-less cell the roll-up actually reads.
///
/// The parent cell is left exactly as its own evidence leaves it. Turning variant evidence into
/// qualification needs an explicit declared variant catalog; that is an architect decision, not
/// something this function may assume.
pub fn applicability_for_key(key: &super::key::CoverageKey) -> Option<ResolvedApplicability> {
    if key.variant.is_some() {
        // Resolve the parent first, so a research-only unit still reports `not-applicable` with its
        // cited reason rather than being masked by the variant rule.
        let parent = applicability(&key.target, &key.family, &key.operation)?;
        if parent.value == Applicability::NotApplicable {
            return Some(parent);
        }
        return Some(ResolvedApplicability {
            value: Applicability::Unknown,
            reason: String::new(),
        });
    }
    applicability(&key.target, &key.family, &key.operation)
}

/// The required operations of `(target, family)` after R-APPL-OPS, sorted by operation id.
pub fn required_operations(target: &str, family: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = operation_defs(family)
        .unwrap_or(&[])
        .iter()
        .filter(|d| {
            applicability(target, family, d.id).is_some_and(|a| a.value == Applicability::Required)
        })
        .map(|d| d.id)
        .collect();
    out.sort_unstable();
    out
}

/// Every non-conversion coverage key the catalog declares: 28 units x 14 families x their
/// operations. This is the **declared universe** the merge emits rows for even with no evidence at
/// all, so an absent emitter can never make a required cell disappear (R-ABSENT, R-CITE-ALL).
///
/// Conversion-dependent keys are **not** enumerated here: their identity comes from the recipe
/// registry, not from the catalog, and inventing pair keys from the catalog would be exactly the
/// fan-out R-NO-BACKFILL forbids.
pub fn catalog_universe() -> std::collections::BTreeSet<super::key::CoverageKey> {
    let mut out = std::collections::BTreeSet::new();
    for unit in CATALOG {
        for family in crate::catalog::FAMILIES {
            let Some(defs) = operation_defs(family.id) else {
                continue;
            };
            for def in defs {
                if requires_conversion(family.id, def.id) {
                    continue;
                }
                let raw = format!("{}-{}/{}/{}", unit.game, unit.platform, family.id, def.id);
                if let Ok(key) = super::key::CoverageKey::parse(&raw) {
                    out.insert(key);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_families_never_require_game_acceptance() {
        // A40 — R-APPL-OPS clause 2.
        assert!(!operations_for_family("db-presentation")
            .expect("family")
            .contains(&"game-acceptance"));
        assert_eq!(
            required_operations("nhl15-xbox360", "db-presentation"),
            vec!["read"]
        );
        assert_eq!(
            required_operations("nhl15-xbox360", "onboarding-install"),
            vec!["discover"]
        );
    }

    #[test]
    fn write_families_still_require_game_acceptance_on_shipping_targets() {
        let req = required_operations("nhl15-xbox360", "assets-portraits");
        assert!(
            req.contains(&"game-acceptance"),
            "Q02 keeps acceptance required for writes"
        );
        assert!(req.contains(&"install"));
        assert!(
            !req.contains(&"preview"),
            "read-only asset ops are reported, not required"
        );
    }

    #[test]
    fn the_research_only_unit_has_no_applicable_write_or_acceptance_cell() {
        // A41 — R-APPL-OPS clause 1.
        for (family, operation) in [
            ("save-raw-db", "save"),
            ("save-raw-db", "game-acceptance"),
            ("db-operations", "scalar-edit"),
        ] {
            let a = applicability("nhl07-ps3", family, operation).expect("known cell");
            assert_eq!(
                a.value,
                Applicability::NotApplicable,
                "{family}/{operation}"
            );
            assert!(
                a.reason.contains("research-only-unit"),
                "V15 needs a cited reason"
            );
        }
        // Read-only operations are untouched.
        let read = applicability("nhl07-ps3", "db-presentation", "read").expect("known cell");
        assert_eq!(read.value, Applicability::Required);
        let reopen = applicability("nhl07-ps3", "save-raw-db", "reopen").expect("known cell");
        assert_eq!(reopen.value, Applicability::Required);
    }

    #[test]
    fn a_shipping_unit_is_never_demoted_by_the_research_rule() {
        let a = applicability("nhl07-xbox360", "save-raw-db", "save").expect("known cell");
        assert_eq!(a.value, Applicability::Required);
    }

    #[test]
    fn outer_package_stays_required_on_every_console_platform() {
        // R-Q04 is preserved by R2, not weakened.
        for target in ["nhl15-xbox360", "nhl15-ps3", "nhl21-ps4"] {
            let a =
                applicability(target, "save-outer-package", "outer-package").expect("known cell");
            assert_eq!(a.value, Applicability::Required, "{target}");
        }
    }

    #[test]
    fn an_uncatalogued_target_is_unknown_rather_than_defaulted() {
        let a = applicability("nhl24-ps4", "assets-heads", "install").expect("known cell");
        assert_eq!(a.value, Applicability::Unknown);
    }

    #[test]
    fn the_catalog_universe_covers_every_unit_and_family_without_inventing_pair_keys() {
        let universe = catalog_universe();
        assert_eq!(
            universe
                .iter()
                .map(|k| k.target.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            CATALOG.len()
        );
        assert!(
            universe.iter().all(|k| k.conversion.is_none()),
            "pair keys come from the recipe registry"
        );
        // Every conversion-dependent head operation is absent; the read-only ones are present.
        assert!(universe
            .iter()
            .any(|k| k.family == "assets-heads" && k.operation == "preview"));
        assert!(!universe
            .iter()
            .any(|k| k.family == "assets-heads" && k.operation == "install"));
    }

    #[test]
    fn head_conversion_operations_require_a_conversion_component() {
        assert!(requires_conversion("assets-heads", "install"));
        assert!(requires_conversion("assets-heads", "game-acceptance"));
        assert!(!requires_conversion("assets-heads", "preview"));
        assert!(!requires_conversion("assets-portraits", "install"));
    }
}
