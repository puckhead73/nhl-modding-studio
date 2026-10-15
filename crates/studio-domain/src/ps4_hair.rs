//! Players 3.4 on PS4 (receipt 66; research R15 = 2026-09-20 handoff 08): the hair styles a PS4 title ships, read from its own
//! Attrib/VLT vault. Every `ge_nhl_skaterhairtexturepack` record carries one typed enum attribute of type `GenericHairStyle`
//! whose value **is** the database `hairid`; a reassignment target must be the value of exactly one such record.
//!
//! Measured on NHL 17–21 only (the class is absent from the NHL 15 / 16 vaults). Identity first, as for heads: the vault is
//! read only from the attached install's `boot.big` and only when its members match the open title's recorded identity.
//! The binding is a texture pack: it says nothing about colour, geometry, helmet compatibility or what the game draws.
//! Read-only.
use std::collections::BTreeMap;

use crate::ps4_vault::{self, Schema, Value, Vault, VaultSource};
use crate::{DomainError, Result};

pub const HAIR_CLASS: &str = "ge_nhl_skaterhairtexturepack";
pub const HAIR_ATTRIBUTE_TYPE: &str = "GenericHairStyle";
/// The PS4 titles whose vault research measured the hair class in (R15 table: NHL 17 → 21).
pub const HAIR_TITLES: [&str; 5] = ["nhl17", "nhl18", "nhl19", "nhl20", "nhl21"];

#[derive(Debug, Clone)]
pub struct HairCatalogue {
    pub title: String,
    pub source: VaultSource,
    /// `GenericHairStyle` value → the number of hair texture-pack records carrying it (1 = unambiguous).
    pub styles: BTreeMap<i64, usize>,
    /// Hair records without a readable value of the attribute.
    pub records_without_value: usize,
}

impl HairCatalogue {
    /// Build from a parsed vault (identity already checked by the caller).
    pub fn from_vault(title: &str, source: VaultSource, vault: &Vault, schema: &Schema) -> Result<Self> {
        if !HAIR_TITLES.contains(&title) {
            return Err(DomainError::Validation(format!(
                "research R15 measured the {HAIR_CLASS} hair binding on NHL 17–21 only; {title} is not admitted"
            )));
        }
        let class = ps4_vault::key(HAIR_CLASS);
        let attrs = schema.classes.get(&class).ok_or_else(|| DomainError::Validation(format!("the {title} vault schema has no {HAIR_CLASS} class")))?;
        let keys: Vec<u32> = attrs.iter().filter(|a| type_is(schema.type_name(a.type_key))).map(|a| a.key).collect();
        let [attr] = keys.as_slice() else {
            return Err(DomainError::Validation(format!(
                "{HAIR_CLASS} carries {} attributes of type {HAIR_ATTRIBUTE_TYPE} in the {title} vault (research R15 measured exactly one); hair is not joined",
                keys.len()
            )));
        };
        let mut styles = BTreeMap::new();
        let mut records_without_value = 0;
        for &(off, size) in vault.records_of(class) {
            match vault.decode(schema, off, size)?.into_iter().find_map(|(k, v)| match (k == *attr, v) {
                (true, Value::Int(i)) => Some(i),
                _ => None,
            }) {
                Some(i) => *styles.entry(i).or_insert(0) += 1,
                None => records_without_value += 1,
            }
        }
        if styles.is_empty() {
            return Err(DomainError::Validation(format!("the {title} vault has no {HAIR_CLASS} record with a {HAIR_ATTRIBUTE_TYPE} value")));
        }
        Ok(HairCatalogue { title: title.to_string(), source, styles, records_without_value })
    }

    /// Parse members already read and identity-checked by [`ps4_vault::read_install_vault`] + [`VaultSource::require_title`].
    pub fn from_members(project_title: &str, source: VaultSource, [v, b, sv, sb]: [Vec<u8>; 4]) -> Result<Self> {
        let schema = Schema::parse(&sv, &sb)?;
        let vault = Vault::parse(v, b)?;
        Self::from_vault(project_title, source, &vault, &schema)
    }

    /// Read the attached install's vault for `project_title` (identity checked) and build the catalogue.
    pub fn load(install_root: &std::path::Path, defs_root: &std::path::Path, project_title: &str) -> Result<Self> {
        let (source, bytes) = ps4_vault::read_install_vault(install_root, defs_root)?;
        source.require_title(project_title)?;
        Self::from_members(project_title, source, bytes)
    }

    /// How many hair records carry `id` (0 = the title ships no hair style of that id).
    pub fn records_for(&self, id: i64) -> usize {
        self.styles.get(&id).copied().unwrap_or(0)
    }

    /// One line naming the vault (title and member hashes) for evidence.
    pub fn describe(&self) -> String {
        format!(
            "{} vault in {} ({})",
            self.title,
            self.source.archive,
            self.source.members.iter().map(|(n, s)| format!("{n} {}…", &s[..12])).collect::<Vec<_>>().join(", ")
        )
    }
}

/// The attribute's type: the enum `…::GenericHairStyle` (the last path segment, never a substring of another name).
fn type_is(name: &str) -> bool {
    name.rsplit("::").next() == Some(HAIR_ATTRIBUTE_TYPE)
}
