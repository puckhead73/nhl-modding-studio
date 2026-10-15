//! Players 3.4 on PS4 (receipt 63; research R15 = 2026-09-20 handoff 08, R124 row 78): the heads a PS4 title ships, read from
//! its own Attrib/VLT vault. Every `ge_nhl_skaterhead` collection carries one typed enum attribute of type
//! `SkaterNameAttribute` whose value **is** the database `headid`; a reassignment target must be such a value.
//!
//! Identity first: the vault is read only from the attached install's `boot.big`, and only when its four members match the
//! recorded identity of the **open project's title** (`defs/ps4/vault-identities.json`); another title's vault is refused as a
//! source mismatch, never borrowed. The attribute is found by its type name and must be unique on the class; a value carried by
//! two head records is ambiguous and not offered. Read-only.
use std::collections::BTreeMap;
use std::path::Path;

use crate::ps4_vault::{self, Schema, Value, Vault, VaultSource};
use crate::{DomainError, Result};

pub const HEAD_CLASS: &str = "ge_nhl_skaterhead";
pub const HEAD_ATTRIBUTE_TYPE: &str = "SkaterNameAttribute";

#[derive(Debug, Clone)]
pub struct HeadCatalogue {
    pub title: String,
    pub source: VaultSource,
    /// `SkaterNameAttribute` value → the number of head records carrying it (1 = unambiguous).
    pub heads: BTreeMap<i64, usize>,
    /// Head records without a readable value of the attribute.
    pub records_without_value: usize,
}

impl HeadCatalogue {
    /// Build from a parsed vault (identity already checked by the caller).
    pub fn from_vault(title: &str, source: VaultSource, vault: &Vault, schema: &Schema) -> Result<Self> {
        let class = ps4_vault::key(HEAD_CLASS);
        let attrs = schema.classes.get(&class).ok_or_else(|| DomainError::Validation(format!("the {title} vault schema has no {HEAD_CLASS} class")))?;
        let keys: Vec<u32> = attrs.iter().filter(|a| schema.type_name(a.type_key).ends_with(HEAD_ATTRIBUTE_TYPE)).map(|a| a.key).collect();
        let [attr] = keys.as_slice() else {
            return Err(DomainError::Validation(format!(
                "{HEAD_CLASS} carries {} attributes of type {HEAD_ATTRIBUTE_TYPE} in the {title} vault (research R15 measured exactly one); heads are not joined",
                keys.len()
            )));
        };
        let mut heads = BTreeMap::new();
        let mut records_without_value = 0;
        for &(off, size) in vault.records_of(class) {
            match vault.decode(schema, off, size)?.into_iter().find_map(|(k, v)| match (k == *attr, v) {
                (true, Value::Int(i)) => Some(i),
                _ => None,
            }) {
                Some(i) => *heads.entry(i).or_insert(0) += 1,
                None => records_without_value += 1,
            }
        }
        if heads.is_empty() {
            return Err(DomainError::Validation(format!("the {title} vault has no {HEAD_CLASS} record with a {HEAD_ATTRIBUTE_TYPE} value")));
        }
        Ok(HeadCatalogue { title: title.to_string(), source, heads, records_without_value })
    }

    /// Read the attached install's vault for `project_title` (identity checked) and build the catalogue.
    pub fn load(install_root: &Path, defs_root: &Path, project_title: &str) -> Result<Self> {
        let (source, bytes) = ps4_vault::read_install_vault(install_root, defs_root)?;
        source.require_title(project_title)?;
        Self::from_members(project_title, source, bytes)
    }

    /// Parse members already read and identity-checked by [`ps4_vault::read_install_vault`] + [`VaultSource::require_title`].
    pub fn from_members(project_title: &str, source: VaultSource, [v, b, sv, sb]: [Vec<u8>; 4]) -> Result<Self> {
        let schema = Schema::parse(&sv, &sb)?;
        let vault = Vault::parse(v, b)?;
        Self::from_vault(project_title, source, &vault, &schema)
    }

    /// How many head records carry `id` (0 = the title ships no head of that id).
    pub fn records_for(&self, id: i64) -> usize {
        self.heads.get(&id).copied().unwrap_or(0)
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
