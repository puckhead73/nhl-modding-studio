//! Coverage keys — [contract §2.2](../../../../docs/plans/qa-evidence-contract.md).
//!
//! ```abnf
//! key         = target "/" family "/" operation [ "/" variant ] [ "?" conversion ]
//! target      = title "-" platform
//! conversion  = "from=" token ";class=" token ";path=" token ";recipe=" token
//! token       = 1*( %x61-7A / DIGIT / "." / "_" / "-" / "@" )
//! ```
//!
//! A key says *what is covered*. It is never a case id and never a run id (R-MULTI), and its
//! position in a file carries no meaning (R-ORDER).

use std::fmt;

/// Every parse failure is a distinct, nameable condition — a key never "mostly parses".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    Empty,
    /// Wrong number of `/`-separated parts (2 or 3 expected before the conversion component).
    Shape(String),
    /// A part contained a character outside the token grammar, or was empty.
    Token {
        part: &'static str,
        value: String,
    },
    /// `target` did not split into `title "-" platform`.
    Target(String),
    /// The conversion component was present but malformed.
    Conversion(String),
    /// A part parsed but is not in its declared vocabulary (V1).
    Vocabulary {
        part: &'static str,
        value: String,
    },
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyError::Empty => write!(f, "coverage key is empty"),
            KeyError::Shape(k) => write!(f, "coverage key has the wrong shape: {k:?}"),
            KeyError::Token { part, value } => {
                write!(f, "coverage key {part} {value:?} is not a valid token")
            }
            KeyError::Target(v) => {
                write!(f, "coverage key target {v:?} is not `<title>-<platform>`")
            }
            KeyError::Conversion(v) => {
                write!(f, "coverage key conversion component {v:?} is malformed")
            }
            KeyError::Vocabulary { part, value } => {
                write!(
                    f,
                    "coverage key {part} {value:?} is not in its declared vocabulary"
                )
            }
        }
    }
}

impl std::error::Error for KeyError {}

/// `from` / `class` / `path` / `recipe`, always all four and always in this order (R-CONV).
///
/// `from` is the **source** target and the key's own `target` is the **destination**; the pair is
/// never collapsed (R-CONV-PAIR).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Conversion {
    pub from: String,
    pub class: String,
    pub path: String,
    pub recipe: String,
}

impl Conversion {
    /// A conversion whose fields are all recorded. An `unknown` in any field is legal in the key but
    /// blocks qualification (contract §2.2 R-CONV).
    pub fn is_fully_known(&self) -> bool {
        [&self.from, &self.class, &self.path, &self.recipe]
            .iter()
            .all(|v| v.as_str() != "unknown")
    }
}

impl fmt::Display for Conversion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "from={};class={};path={};recipe={}",
            self.from, self.class, self.path, self.recipe
        )
    }
}

/// A parsed coverage key. Ordering is the canonical sort order used by every emitted file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoverageKey {
    pub target: String,
    pub family: String,
    pub operation: String,
    pub variant: Option<String>,
    pub conversion: Option<Conversion>,
}

/// `token = 1*( lowercase / DIGIT / "." / "_" / "-" / "@" )`
fn is_token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-' | b'@')
        })
}

impl CoverageKey {
    /// Parse a key against the grammar only. Vocabulary membership is checked separately by
    /// [`CoverageKey::validate_vocabulary`] so that migration can hold a syntactically valid key
    /// whose family is not yet resolved.
    pub fn parse(raw: &str) -> Result<Self, KeyError> {
        if raw.is_empty() {
            return Err(KeyError::Empty);
        }
        let (path, conversion) = match raw.split_once('?') {
            Some((p, c)) => (p, Some(parse_conversion(c)?)),
            None => (raw, None),
        };
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 3 || parts.len() > 4 {
            return Err(KeyError::Shape(raw.to_string()));
        }
        let target = parts[0];
        let (title, platform) = target
            .split_once('-')
            .ok_or_else(|| KeyError::Target(target.to_string()))?;
        if !is_token(title) || !is_token(platform) {
            return Err(KeyError::Target(target.to_string()));
        }
        for (part, value) in [("family", parts[1]), ("operation", parts[2])] {
            if !is_token(value) {
                return Err(KeyError::Token {
                    part,
                    value: value.to_string(),
                });
            }
        }
        let variant = match parts.get(3) {
            Some(v) if is_token(v) => Some((*v).to_string()),
            Some(v) => {
                return Err(KeyError::Token {
                    part: "variant",
                    value: (*v).to_string(),
                });
            }
            None => None,
        };
        Ok(CoverageKey {
            target: target.to_string(),
            family: parts[1].to_string(),
            operation: parts[2].to_string(),
            variant,
            conversion,
        })
    }

    /// V1's second half: every part is in its declared vocabulary.
    pub fn validate_vocabulary(&self) -> Result<(), KeyError> {
        if !crate::catalog::FAMILIES.iter().any(|f| f.id == self.family) {
            return Err(KeyError::Vocabulary {
                part: "family",
                value: self.family.clone(),
            });
        }
        if super::vocab::operations_for_family(&self.family)
            .is_some_and(|ops| !ops.contains(&self.operation.as_str()))
        {
            return Err(KeyError::Vocabulary {
                part: "operation",
                value: self.operation.clone(),
            });
        }
        Ok(())
    }

    /// `title` half of the target.
    pub fn title(&self) -> &str {
        self.target
            .split_once('-')
            .map_or(self.target.as_str(), |(t, _)| t)
    }

    /// `platform` half of the target.
    pub fn platform(&self) -> &str {
        self.target.split_once('-').map_or("", |(_, p)| p)
    }

    /// The roll-up scope this key belongs to (R-CONV-PAIR): a conversion-dependent key rolls up
    /// under `(target, family, conversion)`, never under `(target, family)` alone.
    pub fn rollup_scope(&self) -> String {
        match &self.conversion {
            Some(c) => format!("{}/{}?{}", self.target, self.family, c),
            None => format!("{}/{}", self.target, self.family),
        }
    }
}

fn parse_conversion(raw: &str) -> Result<Conversion, KeyError> {
    let mut from = None;
    let mut class = None;
    let mut path = None;
    let mut recipe = None;
    let fields: Vec<&str> = raw.split(';').collect();
    if fields.len() != 4 {
        return Err(KeyError::Conversion(raw.to_string()));
    }
    // The order is fixed, so index and name are both checked.
    for (i, (name, slot)) in [
        ("from", &mut from),
        ("class", &mut class),
        ("path", &mut path),
        ("recipe", &mut recipe),
    ]
    .into_iter()
    .enumerate()
    {
        let Some(value) = fields[i]
            .strip_prefix(name)
            .and_then(|r| r.strip_prefix('='))
        else {
            return Err(KeyError::Conversion(raw.to_string()));
        };
        if !is_token(value) {
            return Err(KeyError::Conversion(raw.to_string()));
        }
        *slot = Some(value.to_string());
    }
    Ok(Conversion {
        from: from.expect("field 0 assigned above"),
        class: class.expect("field 1 assigned above"),
        path: path.expect("field 2 assigned above"),
        recipe: recipe.expect("field 3 assigned above"),
    })
}

impl fmt::Display for CoverageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.target, self.family, self.operation)?;
        if let Some(v) = &self.variant {
            write!(f, "/{v}")?;
        }
        if let Some(c) = &self.conversion {
            write!(f, "?{c}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(s: &str) -> CoverageKey {
        CoverageKey::parse(s).expect("valid key")
    }

    #[test]
    fn a_minimal_key_round_trips() {
        let key = k("nhl07-ps3/save-raw-db/save");
        assert_eq!(key.target, "nhl07-ps3");
        assert_eq!(key.title(), "nhl07");
        assert_eq!(key.platform(), "ps3");
        assert_eq!(key.variant, None);
        assert_eq!(key.to_string(), "nhl07-ps3/save-raw-db/save");
    }

    #[test]
    fn a_variant_key_round_trips() {
        let key = k("nhl15-ps3/save-inner-container/container-repack/v4");
        assert_eq!(key.variant.as_deref(), Some("v4"));
        assert_eq!(
            key.to_string(),
            "nhl15-ps3/save-inner-container/container-repack/v4"
        );
    }

    #[test]
    fn a_conversion_key_round_trips_with_all_four_fields() {
        let raw = "nhllegacy-xbox360/assets-heads/install?from=nhl24;class=head-authored;path=authored-weights;recipe=patha@1";
        let key = k(raw);
        let c = key.conversion.clone().expect("conversion present");
        assert_eq!(c.from, "nhl24");
        assert_eq!(c.recipe, "patha@1");
        assert!(c.is_fully_known());
        assert_eq!(key.to_string(), raw);
    }

    #[test]
    fn conversion_keys_differing_only_in_source_are_different_keys() {
        // R-CONV / R-CONV-PAIR: two pairs into one destination never collapse.
        let a = k("nhllegacy-xbox360/assets-heads/install?from=nhl24;class=head-authored;path=authored-weights;recipe=patha@1");
        let b = k("nhllegacy-xbox360/assets-heads/install?from=nhl21;class=head-authored;path=authored-weights;recipe=patha@1");
        assert_ne!(a, b);
        assert_ne!(a.rollup_scope(), b.rollup_scope());
    }

    #[test]
    fn an_unknown_conversion_field_parses_but_is_not_fully_known() {
        let key = k("nhl15-xbox360/assets-heads/install?from=nhl24;class=unknown;path=unknown;recipe=unknown");
        assert!(!key.conversion.expect("present").is_fully_known());
    }

    #[test]
    fn malformed_keys_are_rejected_with_a_named_condition() {
        assert_eq!(CoverageKey::parse(""), Err(KeyError::Empty));
        assert!(matches!(
            CoverageKey::parse("nhl07-ps3/save-raw-db"),
            Err(KeyError::Shape(_))
        ));
        assert!(matches!(
            CoverageKey::parse("nhl07-ps3/a/b/c/d"),
            Err(KeyError::Shape(_))
        ));
        assert!(matches!(
            CoverageKey::parse("nhl07/save-raw-db/save"),
            Err(KeyError::Target(_))
        ));
        assert!(matches!(
            CoverageKey::parse("NHL07-ps3/save-raw-db/save"),
            Err(KeyError::Target(_))
        ));
        assert!(matches!(
            CoverageKey::parse("nhl07-ps3/save raw db/save"),
            Err(KeyError::Token { part: "family", .. })
        ));
    }

    #[test]
    fn a_conversion_component_must_carry_all_four_fields_in_order() {
        assert!(matches!(
            CoverageKey::parse("a-b/assets-heads/install?from=nhl24;class=x;path=y"),
            Err(KeyError::Conversion(_))
        ));
        assert!(matches!(
            CoverageKey::parse("a-b/assets-heads/install?class=x;from=nhl24;path=y;recipe=z"),
            Err(KeyError::Conversion(_))
        ));
    }

    #[test]
    fn vocabulary_is_checked_separately_from_syntax() {
        let good = k("nhl07-ps3/save-raw-db/save");
        good.validate_vocabulary().expect("in vocabulary");
        let bad_family = k("nhl07-ps3/not-a-family/save");
        assert!(matches!(
            bad_family.validate_vocabulary(),
            Err(KeyError::Vocabulary { part: "family", .. })
        ));
        let bad_op = k("nhl07-ps3/save-raw-db/not-an-operation");
        assert!(matches!(
            bad_op.validate_vocabulary(),
            Err(KeyError::Vocabulary {
                part: "operation",
                ..
            })
        ));
    }

    #[test]
    fn keys_sort_canonically_and_order_carries_no_meaning() {
        let mut keys = [
            k("nhl15-ps3/save-raw-db/save"),
            k("nhl07-ps3/save-raw-db/save"),
            k("nhl07-ps3/db-presentation/read"),
        ];
        keys.sort();
        assert_eq!(
            keys.iter().map(ToString::to_string).collect::<Vec<_>>(),
            vec![
                "nhl07-ps3/db-presentation/read",
                "nhl07-ps3/save-raw-db/save",
                "nhl15-ps3/save-raw-db/save",
            ]
        );
    }
}
