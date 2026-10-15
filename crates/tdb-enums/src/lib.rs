//! `tdb-enums` — confirmed value↔label enum registry (EDIT-MODEL Layer 1, enum validation).
//!
//! Ingests the operator-curated enum definitions under the repo's `defs/enums/*.toml` into a typed
//! [`EnumRegistry`]. Each [`EnumDef`] carries a vocabulary of raw-code → label [`EnumEntry`]s plus the
//! `(table, field)` [`EnumBinding`]s the enum applies to, so the edit layer can both **validate enum
//! membership at entry** (an `eGlu.mCcl` captaincy edit must be one of `None/Captain/Alternate`) and
//! **render human labels** on read.
//!
//! ## Id convention (read this)
//!
//! These defs use the **studio / NHLViewNG id convention** (`ubPc`, `vjla`, `eGlu`, `mCcl`, …): the
//! same 4-char ids the metadata XML *names* are derived from. The **on-disk TDB binary** (and the EA
//! `*-meta.xml`) use the **character-reversed** form of each id (`ubPc`↔`cPbu`, `vjla`↔`aljv`,
//! `eGlu`↔`ulGe`). This crate keys the registry by the **studio ids verbatim** — it does not silently
//! transform them. Crossing to the binary id space is the integration layer's job; [`reverse_id`] is
//! provided for that, and the cross-check test in `tests/enums.rs` uses it to verify each enum's
//! field really exists in a real anchor database with a bit-width wide enough to hold its codes.
//!
//! ## Title scope (coordinator decision, 25 Sep 2026)
//!
//! A vocabulary whose labels were established on one title (the equipment catalogues read off NHL
//! Legacy's textures and catalogue CSVs) carries a `[scope]` naming the studio game ids — and,
//! where the evidence requires it, the platforms — it applies to. A scoped def resolves **only**
//! when the registry has been told the project's title ([`EnumRegistry::set_scope`]) and that title
//! is in scope; with no title, or another title, the field has no vocabulary: the stored id is
//! shown and written as a number, never relabelled with another title's catalogue. Defs without a
//! `[scope]` apply everywhere. Three things stay distinct here: the ids a file stores (any in-range
//! value round-trips), the labels evidence established (the scoped entries), and the values an
//! edit may take (range validation in `tdb-defs`; a label never authorises or forbids a write).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Provenance block (`[source]`) — how a def was confirmed.
#[derive(Clone, Debug, Deserialize)]
pub struct EnumSource {
    pub kind: String,
    #[serde(default)]
    pub description: String,
}

/// The titles (and optionally platforms) a def's labels were established for (`[scope]`).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct EnumScope {
    /// Studio game ids (`nhllegacy`, `nhl15`, …). Empty = no title matches.
    #[serde(default)]
    pub games: Vec<String>,
    /// Platform ids (`ps3`, `xbox`, `ps4`, `recomp`). Empty = every platform of those games.
    #[serde(default)]
    pub platforms: Vec<String>,
}

impl EnumScope {
    /// Whether a project opened as `game` on `platform` is inside this scope. An unknown title
    /// (`None`) is never inside a scope — labels are not borrowed on a guess.
    pub fn applies(&self, game: Option<&str>, platform: &str) -> bool {
        let Some(g) = game else {
            return false;
        };
        self.games.iter().any(|x| x == g)
            && (self.platforms.is_empty() || self.platforms.iter().any(|p| p == platform))
    }

    /// Human-readable scope ("NHL Legacy", "NHL 15 (PS4)", …).
    pub fn label(&self) -> String {
        let games: Vec<String> = self.games.iter().map(|g| game_title(g)).collect();
        let mut s = games.join(", ");
        if !self.platforms.is_empty() {
            let ps: Vec<String> = self.platforms.iter().map(|p| platform_title(p)).collect();
            s.push_str(&format!(" ({})", ps.join(" / ")));
        }
        s
    }
}

/// Studio game id → title as shown to the user.
pub fn game_title(game_id: &str) -> String {
    match game_id {
        "nhllegacy" => "NHL Legacy".to_string(),
        "nhllegacyrecompiled" | "recomp" => "NHL Legacy Recompiled".to_string(),
        g if g.starts_with("nhl") && g.len() == 5 => format!("NHL {}", &g[3..]),
        other => other.to_string(),
    }
}

fn platform_title(p: &str) -> String {
    match p {
        "ps3" => "PS3".to_string(),
        "xbox" => "Xbox 360".to_string(),
        "ps4" => "PS4".to_string(),
        "recomp" => "Legacy Recompiled".to_string(),
        other => other.to_string(),
    }
}

/// A `(table, field)` site an enum applies to. Keyed by the **studio** ids (see module docs).
#[derive(Clone, Debug, Deserialize)]
pub struct EnumBinding {
    pub field_name: String,
    pub field_raw_id: String,
    pub table_raw_id: String,
}

/// One raw-code → label mapping.
#[derive(Clone, Debug, Deserialize)]
pub struct EnumEntry {
    pub raw: i64,
    pub label: String,
}

/// A single confirmed enum definition (one `defs/enums/*.toml` file).
#[derive(Clone, Debug, Deserialize)]
pub struct EnumDef {
    pub name: String,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub notes: String,
    pub source: Option<EnumSource>,
    /// The titles this vocabulary was established for; `None` = every title.
    #[serde(default)]
    pub scope: Option<EnumScope>,
    #[serde(default)]
    pub bindings: Vec<EnumBinding>,
    #[serde(default)]
    pub entries: Vec<EnumEntry>,
}

impl EnumDef {
    /// Parse one enum def from TOML text.
    pub fn from_toml(text: &str) -> Result<Self, EnumError> {
        toml::from_str(text).map_err(EnumError::Parse)
    }

    /// The label for a raw code, if listed.
    pub fn label(&self, raw: i64) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.raw == raw)
            .map(|e| e.label.as_str())
    }

    /// The raw code for an exact label, if present.
    pub fn raw_of(&self, label: &str) -> Option<i64> {
        self.entries
            .iter()
            .find(|e| e.label == label)
            .map(|e| e.raw)
    }

    /// Whether a raw code is a labelled member of this enum.
    pub fn contains(&self, raw: i64) -> bool {
        self.entries.iter().any(|e| e.raw == raw)
    }

    /// The largest labelled raw code (used to sanity-check storage bit-width).
    pub fn max_raw(&self) -> Option<i64> {
        self.entries.iter().map(|e| e.raw).max()
    }

    /// Whether this def's labels apply to a project opened as `game` on `platform`. Unscoped defs
    /// apply to every title; scoped defs never apply to an unknown title.
    pub fn applies_to(&self, game: Option<&str>, platform: &str) -> bool {
        match &self.scope {
            None => true,
            Some(s) => s.applies(game, platform),
        }
    }

    /// Where the labels come from, for the vocabulary note ("labels established for NHL Legacy —
    /// Equipment ID catalog CSV …").
    pub fn provenance(&self) -> Option<String> {
        let scope = self.scope.as_ref()?;
        let mut s = format!("labels established for {}", scope.label());
        if let Some(src) = &self.source {
            if !src.description.is_empty() {
                s.push_str(&format!(" — {}", src.description));
            }
        }
        Some(s)
    }
}

/// Errors loading or validating against the enum registry.
#[derive(Debug)]
pub enum EnumError {
    /// A `*.toml` def failed to parse.
    Parse(toml::de::Error),
    /// A directory could not be read.
    Io(std::io::Error),
    /// A value was edited to a code that is not a labelled member of the bound enum.
    NotAMember {
        table: String,
        field: String,
        enum_name: String,
        value: i64,
    },
}

impl std::fmt::Display for EnumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnumError::Parse(e) => write!(f, "enum def parse error: {e}"),
            EnumError::Io(e) => write!(f, "enum def io error: {e}"),
            EnumError::NotAMember {
                table,
                field,
                enum_name,
                value,
            } => write!(
                f,
                "value {value} is not a member of enum {enum_name:?} bound to {table}.{field}"
            ),
        }
    }
}
impl std::error::Error for EnumError {}

/// Reverse a 4-char (or any) raw id between the studio and on-disk/binary conventions.
///
/// `ubPc` ↔ `cPbu`, `vjla` ↔ `aljv`, `eGlu` ↔ `ulGe`. The mapping is its own inverse.
pub fn reverse_id(id: &str) -> String {
    id.chars().rev().collect()
}

/// All confirmed enums, indexed by the studio `(table, field)` they bind to, resolved under the
/// project's title scope.
#[derive(Debug, Default, Clone)]
pub struct EnumRegistry {
    defs: Vec<EnumDef>,
    /// Every def bound to a site, in load order — several when title-scoped vocabularies share it.
    by_field: HashMap<(String, String), Vec<usize>>,
    /// The project's `(game id, platform)` once known. `None` (never told) resolves unscoped defs only.
    scope: Option<(Option<String>, String)>,
}

impl EnumRegistry {
    /// Load every `*.toml` enum def from a directory (e.g. the repo's `defs/enums`).
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, EnumError> {
        let mut reg = EnumRegistry::default();
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .map_err(EnumError::Io)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        // Deterministic load order regardless of filesystem enumeration.
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path).map_err(EnumError::Io)?;
            reg.insert(EnumDef::from_toml(&text)?);
        }
        Ok(reg)
    }

    /// Add one def, registering each of its `(table, field)` bindings.
    pub fn insert(&mut self, def: EnumDef) {
        let i = self.defs.len();
        for b in &def.bindings {
            self.by_field
                .entry((b.table_raw_id.clone(), b.field_raw_id.clone()))
                .or_default()
                .push(i);
        }
        self.defs.push(def);
    }

    /// Tell the registry which title the project is (`None` = unknown) on which platform. Scoped
    /// defs resolve only for a title inside their scope.
    pub fn set_scope(&mut self, game: Option<&str>, platform: &str) {
        self.scope = Some((game.map(str::to_owned), platform.to_owned()));
    }

    /// A copy of this registry resolved for `game` on `platform`.
    pub fn scoped(&self, game: Option<&str>, platform: &str) -> EnumRegistry {
        let mut r = self.clone();
        r.set_scope(game, platform);
        r
    }

    /// The `(game, platform)` this registry resolves under, once told.
    pub fn scope(&self) -> Option<(Option<&str>, &str)> {
        self.scope.as_ref().map(|(g, p)| (g.as_deref(), p.as_str()))
    }

    fn applies(&self, def: &EnumDef) -> bool {
        match &self.scope {
            None => def.scope.is_none(),
            Some((g, p)) => def.applies_to(g.as_deref(), p),
        }
    }

    /// The enum bound to a studio `(table, field)` **for this registry's title**, if any.
    pub fn for_field(&self, table: &str, field: &str) -> Option<&EnumDef> {
        self.by_field
            .get(&(table.to_string(), field.to_string()))?
            .iter()
            .map(|&i| &self.defs[i])
            .find(|d| self.applies(d))
    }

    /// The applicable def with this `name` (equipment slot lookups by vocabulary name).
    pub fn def_named(&self, name: &str) -> Option<&EnumDef> {
        self.defs.iter().find(|d| d.name == name && self.applies(d))
    }

    /// Why a site does or does not carry labels under this title — the human-readable vocabulary
    /// note. `Some` for a scoped vocabulary that applies (its provenance) and for a site whose only
    /// vocabularies are scoped to other titles (no catalogue here); `None` for unscoped vocabularies
    /// and for sites with no vocabulary at all.
    pub fn vocabulary_note(&self, table: &str, field: &str) -> Option<String> {
        let candidates = self.by_field.get(&(table.to_string(), field.to_string()))?;
        if let Some(def) = self.for_field(table, field) {
            return def.provenance();
        }
        let scoped: Vec<&EnumDef> = candidates
            .iter()
            .map(|&i| &self.defs[i])
            .filter(|d| d.scope.is_some())
            .collect();
        let first = scoped.first()?;
        let scopes: Vec<String> = scoped
            .iter()
            .filter_map(|d| d.scope.as_ref().map(|s| s.label()))
            .collect();
        let title = match &self.scope {
            Some((Some(g), _)) => game_title(g),
            _ => "an unknown title".to_string(),
        };
        Some(format!(
            "{}: no catalogue for {title} — its labels were established for {} only; the stored id is shown as a number and written as entered",
            first.name,
            scopes.join(" / ")
        ))
    }

    /// The label for a raw code at a bound `(table, field)`, if both the binding and code are known.
    pub fn label(&self, table: &str, field: &str, raw: i64) -> Option<&str> {
        self.for_field(table, field).and_then(|d| d.label(raw))
    }

    /// Validate an edit's enum membership. Fields with **no** bound enum are unconstrained here
    /// (range/`updatable` checks live in `tdb-defs`); fields **with** one must take a labelled code.
    pub fn validate(&self, table: &str, field: &str, value: i64) -> Result<(), EnumError> {
        match self.for_field(table, field) {
            Some(def) if !def.contains(value) => Err(EnumError::NotAMember {
                table: table.to_string(),
                field: field.to_string(),
                enum_name: def.name.clone(),
                value,
            }),
            _ => Ok(()),
        }
    }

    /// Every loaded def (scoped or not).
    pub fn defs(&self) -> &[EnumDef] {
        &self.defs
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POSITION: &str = r#"
name = "Position"
confidence = "Confirmed"
[[bindings]]
field_name = "Position"
field_raw_id = "vjla"
table_raw_id = "ubPc"
[[entries]]
raw = 0
label = "C"
[[entries]]
raw = 4
label = "G"
"#;

    const STICK_LEGACY: &str = r#"
name = "Stick Model"
[source]
kind = "operator-research"
description = "texlib artwork, NHL Legacy PS3"
[scope]
games = ["nhllegacy"]
[[bindings]]
field_name = "Stick"
field_raw_id = "wXFx"
table_raw_id = "xmja"
[[entries]]
raw = 17
label = "Easton Stealth S17"
"#;

    const STICK_OTHER: &str = r#"
name = "Stick Model (NHL 15 PS4)"
[scope]
games = ["nhl15"]
platforms = ["ps4"]
[[bindings]]
field_name = "Stick"
field_raw_id = "wXFx"
table_raw_id = "xmja"
[[entries]]
raw = 40
label = "Easton Stealth S17"
"#;

    #[test]
    fn parses_and_maps() {
        let d = EnumDef::from_toml(POSITION).unwrap();
        assert_eq!(d.name, "Position");
        assert_eq!(d.label(0), Some("C"));
        assert_eq!(d.label(4), Some("G"));
        assert_eq!(d.raw_of("G"), Some(4));
        assert!(d.contains(0) && !d.contains(2));
        assert_eq!(d.max_raw(), Some(4));
        assert!(d.scope.is_none() && d.applies_to(None, "ps3"));
    }

    #[test]
    fn registry_validates_membership() {
        let mut reg = EnumRegistry::default();
        reg.insert(EnumDef::from_toml(POSITION).unwrap());
        assert_eq!(reg.label("ubPc", "vjla", 4), Some("G"));
        assert!(reg.validate("ubPc", "vjla", 0).is_ok());
        assert!(matches!(
            reg.validate("ubPc", "vjla", 7),
            Err(EnumError::NotAMember { .. })
        ));
        // Unbound field is unconstrained by the enum layer.
        assert!(reg.validate("ubPc", "zzzz", 999).is_ok());
    }

    #[test]
    fn scoped_defs_resolve_only_for_their_title() {
        let mut reg = EnumRegistry::default();
        reg.insert(EnumDef::from_toml(POSITION).unwrap());
        reg.insert(EnumDef::from_toml(STICK_LEGACY).unwrap());
        // Never told a title: unscoped defs only; the scoped one is absent and says why.
        assert_eq!(reg.label("ubPc", "vjla", 4), Some("G"));
        assert!(reg.for_field("xmja", "wXFx").is_none());
        let note = reg.vocabulary_note("xmja", "wXFx").unwrap();
        assert!(
            note.contains("no catalogue for an unknown title") && note.contains("NHL Legacy"),
            "{note}"
        );
        // Another title: same.
        let other = reg.scoped(Some("nhl10"), "xbox");
        assert!(other.for_field("xmja", "wXFx").is_none());
        assert!(
            other.validate("xmja", "wXFx", 190).is_ok(),
            "an unknown id is not an enum violation"
        );
        assert!(other
            .vocabulary_note("xmja", "wXFx")
            .unwrap()
            .contains("no catalogue for NHL 10"));
        // The scoped title, on any platform (no platform list): labels and provenance.
        for p in ["ps3", "xbox", "recomp"] {
            let legacy = reg.scoped(Some("nhllegacy"), p);
            assert_eq!(
                legacy.label("xmja", "wXFx", 17),
                Some("Easton Stealth S17"),
                "{p}"
            );
            let note = legacy.vocabulary_note("xmja", "wXFx").unwrap();
            assert!(
                note.starts_with("labels established for NHL Legacy") && note.contains("texlib"),
                "{note}"
            );
        }
        // Unscoped vocabularies carry no note; unbound sites none either.
        assert!(reg.vocabulary_note("ubPc", "vjla").is_none());
        assert!(reg.vocabulary_note("ubPc", "zzzz").is_none());
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn a_label_shared_by_two_titles_resolves_to_each_titles_own_id_never_the_others() {
        let mut reg = EnumRegistry::default();
        reg.insert(EnumDef::from_toml(STICK_LEGACY).unwrap());
        reg.insert(EnumDef::from_toml(STICK_OTHER).unwrap());
        let legacy = reg.scoped(Some("nhllegacy"), "ps3");
        let ps4 = reg.scoped(Some("nhl15"), "ps4");
        let ps3_15 = reg.scoped(Some("nhl15"), "ps3");
        assert_eq!(
            legacy
                .for_field("xmja", "wXFx")
                .unwrap()
                .raw_of("Easton Stealth S17"),
            Some(17)
        );
        assert_eq!(
            ps4.for_field("xmja", "wXFx")
                .unwrap()
                .raw_of("Easton Stealth S17"),
            Some(40)
        );
        assert_eq!(
            legacy.label("xmja", "wXFx", 40),
            None,
            "the PS4 id is not relabelled on Legacy"
        );
        assert_eq!(ps4.label("xmja", "wXFx", 17), None);
        // The platform list is honoured: NHL 15 on PS3 is outside the PS4-only def.
        assert!(ps3_15.for_field("xmja", "wXFx").is_none());
        let note = ps3_15.vocabulary_note("xmja", "wXFx").unwrap();
        assert!(note.contains("NHL Legacy / NHL 15 (PS4)"), "{note}");
        assert_eq!(
            ps4.def_named("Stick Model (NHL 15 PS4)")
                .map(|d| d.entries.len()),
            Some(1)
        );
        assert!(ps4.def_named("Stick Model").is_none());
    }

    #[test]
    fn reverse_id_is_involution() {
        for id in ["ubPc", "vjla", "eGlu", "mCcl"] {
            assert_eq!(reverse_id(&reverse_id(id)), id);
        }
        assert_eq!(reverse_id("ubPc"), "cPbu");
        assert_eq!(reverse_id("vjla"), "aljv");
    }
}
