//! Versioned freshness scope definitions —
//! [contract §5.3](../../../../docs/plans/qa-evidence-contract.md).
//!
//! **R-CONTENT.** The digest covers file **content** only. A touched file with identical bytes
//! changes nothing; a changed byte invalidates exactly the records that declared the containing
//! scope. No date, commit id or mtime participates.
//!
//! **R-FRESH-UNKNOWN.** If any freshness input is missing or unreadable the digest is `"unknown"`,
//! and `unknown` is not-fresh everywhere. A missing input never produces `fresh`.
//!
//! The digest itself is computed from a *supplied* `(path, content sha256)` map so that the
//! computation is pure and testable; walking the filesystem to build that map is the caller's job.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The literal recorded whenever a digest cannot be computed. Never a default, never inferred.
pub const UNKNOWN: &str = "unknown";

/// One `[[scope]]` entry of `defs/qa/scopes.toml`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Scope {
    pub id: String,
    pub version: u32,
    /// Other scope ids folded into this one's digest.
    #[serde(default)]
    pub shared: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub config: Vec<String>,
    #[serde(default)]
    pub schema: Vec<String>,
    #[serde(default)]
    pub recipe: Vec<String>,
    /// The *mapping and assertion* logic an observation depends on — the emitter test sources
    /// themselves. An observation is a claim about what a case asserted, so changing what the case
    /// asserts, or which coverage key it maps to, invalidates it exactly as changing the
    /// implementation under test does.
    #[serde(default)]
    pub mapping: Vec<String>,
    /// Identity strings, not file paths — resolved by the caller and folded in verbatim.
    #[serde(default)]
    pub toolchain: Vec<String>,
}

impl Scope {
    /// Every glob this scope contributes, in the contract's declared order.
    pub fn globs(&self) -> Vec<&str> {
        self.paths
            .iter()
            .chain(self.config.iter())
            .chain(self.schema.iter())
            .chain(self.recipe.iter())
            .chain(self.mapping.iter())
            .map(String::as_str)
            .collect()
    }
}

/// `defs/qa/scopes.toml`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Scopes {
    pub schema: String,
    #[serde(default, rename = "scope")]
    pub scopes: Vec<Scope>,
}

/// Why a scope file was rejected. There is no lenient mode: a malformed definition is a hard error,
/// because an unreadable freshness definition silently disables freshness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    Parse(String),
    UnknownSchema(String),
    DuplicateId(String),
    UnknownShared { scope: String, shared: String },
    SharedCycle(String),
}

impl std::fmt::Display for ScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeError::Parse(e) => write!(f, "scopes.toml did not parse: {e}"),
            ScopeError::UnknownSchema(s) => write!(f, "unsupported scopes schema {s:?}"),
            ScopeError::DuplicateId(id) => write!(f, "duplicate scope id {id:?}"),
            ScopeError::UnknownShared { scope, shared } => {
                write!(f, "scope {scope:?} shares unknown scope {shared:?}")
            }
            ScopeError::SharedCycle(id) => {
                write!(f, "scope {id:?} participates in a `shared` cycle")
            }
        }
    }
}

impl std::error::Error for ScopeError {}

impl Scopes {
    /// Parse and validate. Every structural defect is rejected here rather than producing a digest
    /// that silently omits an input.
    pub fn parse(text: &str) -> Result<Self, ScopeError> {
        let parsed: Scopes = toml::from_str(text).map_err(|e| ScopeError::Parse(e.to_string()))?;
        if parsed.schema != "qa-scopes/1" {
            return Err(ScopeError::UnknownSchema(parsed.schema));
        }
        let mut seen = BTreeSet::new();
        for scope in &parsed.scopes {
            if !seen.insert(scope.id.clone()) {
                return Err(ScopeError::DuplicateId(scope.id.clone()));
            }
        }
        for scope in &parsed.scopes {
            for shared in &scope.shared {
                if !seen.contains(shared) {
                    return Err(ScopeError::UnknownShared {
                        scope: scope.id.clone(),
                        shared: shared.clone(),
                    });
                }
            }
        }
        for scope in &parsed.scopes {
            if parsed.has_cycle(&scope.id, &mut BTreeSet::new()) {
                return Err(ScopeError::SharedCycle(scope.id.clone()));
            }
        }
        Ok(parsed)
    }

    fn has_cycle(&self, id: &str, seen: &mut BTreeSet<String>) -> bool {
        if !seen.insert(id.to_string()) {
            return true;
        }
        let found = self
            .get(id)
            .into_iter()
            .flat_map(|s| s.shared.iter())
            .any(|next| self.has_cycle(next, seen));
        seen.remove(id);
        found
    }

    pub fn get(&self, id: &str) -> Option<&Scope> {
        self.scopes.iter().find(|s| s.id == id)
    }

    /// The `implementation_digest` of one scope, over a supplied content map.
    ///
    /// `contents` maps a repo-relative slash path to that file's SHA-256. `resolve_toolchain` maps a
    /// toolchain identity name to its resolved string. Returns [`UNKNOWN`] whenever a declared glob
    /// matches nothing or a toolchain identity cannot be resolved — a missing input is never
    /// silently dropped from the digest.
    pub fn digest(
        &self,
        id: &str,
        contents: &BTreeMap<String, String>,
        resolve_toolchain: &dyn Fn(&str) -> Option<String>,
    ) -> String {
        let mut hasher = Sha256::new();
        match self.feed(
            id,
            contents,
            resolve_toolchain,
            &mut hasher,
            &mut BTreeSet::new(),
        ) {
            true => format!("sha256:{:x}", hasher.finalize()),
            false => UNKNOWN.to_string(),
        }
    }

    fn feed(
        &self,
        id: &str,
        contents: &BTreeMap<String, String>,
        resolve_toolchain: &dyn Fn(&str) -> Option<String>,
        hasher: &mut Sha256,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if !visited.insert(id.to_string()) {
            return true; // already folded in; validated acyclic above
        }
        let Some(scope) = self.get(id) else {
            return false;
        };
        hasher.update(scope.id.as_bytes());
        hasher.update([0]);
        hasher.update(scope.version.to_string().as_bytes());
        hasher.update([0]);
        for glob in scope.globs() {
            let matches: Vec<(&String, &String)> = contents
                .iter()
                .filter(|(path, _)| glob_matches(glob, path))
                .collect();
            if matches.is_empty() {
                return false; // R-FRESH-UNKNOWN
            }
            for (path, sha) in matches {
                hasher.update(path.as_bytes());
                hasher.update([0]);
                hasher.update(sha.as_bytes());
                hasher.update([0]);
            }
        }
        for name in &scope.toolchain {
            let Some(value) = resolve_toolchain(name) else {
                return false;
            };
            hasher.update(name.as_bytes());
            hasher.update([0]);
            hasher.update(value.as_bytes());
            hasher.update([0]);
        }
        for shared in &scope.shared {
            if !self.feed(shared, contents, resolve_toolchain, hasher, visited) {
                return false;
            }
        }
        true
    }
}

/// The glob subset the scope definitions use: a literal path, or a prefix ending in `/**`, or a
/// prefix ending in `/*` matching one path segment. Nothing broader is accepted, so a definition can
/// never quietly widen to the whole tree.
fn glob_matches(glob: &str, path: &str) -> bool {
    if let Some(prefix) = glob.strip_suffix("/**") {
        return path.starts_with(prefix)
            && path.len() > prefix.len()
            && path.as_bytes()[prefix.len()] == b'/';
    }
    if let Some(prefix) = glob.strip_suffix("/*") {
        return path
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('/'))
            .is_some_and(|rest| !rest.contains('/'));
    }
    glob == path
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
schema = "qa-scopes/1"

[[scope]]
id = "shared-core"
version = 1
paths = ["crates/tdb-core/src/**"]
toolchain = ["rustc-version"]

[[scope]]
id = "save-containers"
version = 1
shared = ["shared-core"]
paths = ["crates/tdb-savedata/src/**"]
"#;

    fn contents() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("crates/tdb-core/src/lib.rs".to_string(), "a".repeat(64)),
            ("crates/tdb-savedata/src/lib.rs".to_string(), "b".repeat(64)),
        ])
    }

    fn toolchain(name: &str) -> Option<String> {
        (name == "rustc-version").then(|| "rustc 1.96.0".to_string())
    }

    #[test]
    fn a_valid_definition_parses_and_resolves_its_shared_scope() {
        let scopes = Scopes::parse(SAMPLE).expect("valid");
        assert_eq!(scopes.scopes.len(), 2);
        assert_eq!(
            scopes.get("save-containers").expect("present").shared,
            vec!["shared-core"]
        );
    }

    #[test]
    fn a7_identical_bytes_are_fresh_and_one_changed_byte_is_not() {
        let scopes = Scopes::parse(SAMPLE).expect("valid");
        let before = scopes.digest("save-containers", &contents(), &toolchain);
        // Same content map rebuilt from scratch — a "touched" file with identical bytes.
        let again = scopes.digest("save-containers", &contents(), &toolchain);
        assert_eq!(before, again);
        assert_ne!(before, UNKNOWN);

        let mut changed = contents();
        changed.insert("crates/tdb-core/src/lib.rs".to_string(), "c".repeat(64));
        let after = scopes.digest("save-containers", &changed, &toolchain);
        assert_ne!(
            before, after,
            "a byte inside a shared scope must invalidate the digest"
        );
    }

    #[test]
    fn a_scope_that_does_not_share_is_unaffected_by_the_other_scopes_content() {
        let scopes = Scopes::parse(SAMPLE).expect("valid");
        let before = scopes.digest("shared-core", &contents(), &toolchain);
        let mut changed = contents();
        changed.insert("crates/tdb-savedata/src/lib.rs".to_string(), "z".repeat(64));
        assert_eq!(before, scopes.digest("shared-core", &changed, &toolchain));
    }

    #[test]
    fn a_missing_input_is_unknown_rather_than_a_digest_over_what_happens_to_exist() {
        let scopes = Scopes::parse(SAMPLE).expect("valid");
        assert_eq!(
            scopes.digest("save-containers", &BTreeMap::new(), &toolchain),
            UNKNOWN
        );
        assert_eq!(
            scopes.digest("shared-core", &contents(), &|_| None),
            UNKNOWN
        );
        assert_eq!(
            scopes.digest("no-such-scope", &contents(), &toolchain),
            UNKNOWN
        );
    }

    #[test]
    fn structural_defects_are_hard_errors() {
        assert!(matches!(
            Scopes::parse("schema = \"qa-scopes/2\""),
            Err(ScopeError::UnknownSchema(_))
        ));
        let dup = "schema = \"qa-scopes/1\"\n[[scope]]\nid=\"a\"\nversion=1\n[[scope]]\nid=\"a\"\nversion=1\n";
        assert!(matches!(
            Scopes::parse(dup),
            Err(ScopeError::DuplicateId(_))
        ));
        let dangling = "schema = \"qa-scopes/1\"\n[[scope]]\nid=\"a\"\nversion=1\nshared=[\"b\"]\n";
        assert!(matches!(
            Scopes::parse(dangling),
            Err(ScopeError::UnknownShared { .. })
        ));
        let cycle = "schema = \"qa-scopes/1\"\n[[scope]]\nid=\"a\"\nversion=1\nshared=[\"b\"]\n[[scope]]\nid=\"b\"\nversion=1\nshared=[\"a\"]\n";
        assert!(matches!(
            Scopes::parse(cycle),
            Err(ScopeError::SharedCycle(_))
        ));
    }

    #[test]
    fn the_glob_subset_matches_only_what_it_declares() {
        assert!(glob_matches("a/b/**", "a/b/c.rs"));
        assert!(glob_matches("a/b/**", "a/b/c/d.rs"));
        assert!(!glob_matches("a/b/**", "a/b"));
        assert!(!glob_matches("a/b/**", "a/bc/d.rs"));
        assert!(glob_matches("a/b/*", "a/b/c.rs"));
        assert!(!glob_matches("a/b/*", "a/b/c/d.rs"));
        assert!(glob_matches("a/b.rs", "a/b.rs"));
        assert!(!glob_matches("a/b.rs", "a/b.rs.bak"));
    }
}
