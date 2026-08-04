//! Per-anchor schema snapshots: the full table/field structure of one main database, joined to its
//! metadata (resolved names, ranges, update flags, keys, FK parents). Deterministic — tables and
//! fields are emitted in on-disk order, so the JSON is stable and diffable across runs.

use serde::Serialize;
use tdb_core::{Endian, TdbFile};
use tdb_defs::{FieldMetaIndex, FieldType};

use crate::{catalog, MainAnchor};

#[derive(Serialize)]
pub struct SchemaSnapshot {
    pub game: String,
    pub platform: String,
    pub generation: String,
    pub db: String,
    /// `big` (PS3/Xbox 360) or `little` (PS4).
    pub endian: String,
    /// TDB container version (v6 across the whole catalog).
    pub tdb_version: u32,
    pub meta_present: bool,
    pub meta_file: Option<String>,
    pub table_count: usize,
    pub field_total: usize,
    pub tables: Vec<TableSnapshot>,
}

#[derive(Serialize)]
pub struct TableSnapshot {
    /// On-disk 4-char id (what the metadata XML keys on).
    pub raw_id: String,
    /// Community/canonical id (chars reversed on big-endian).
    pub canonical_id: String,
    pub name: Option<String>,
    pub record_len_bytes: u32,
    pub current_records: u16,
    /// Structural capacity (max records the table's directory can address).
    pub max_records: u16,
    pub field_count: usize,
    pub fields: Vec<FieldSnapshot>,
}

#[derive(Serialize)]
pub struct FieldSnapshot {
    pub raw_id: String,
    pub canonical_id: String,
    pub name: Option<String>,
    /// tdb-core value-class code (0 string, 3 fixed-bit uint, …).
    pub kind_code: u32,
    /// Metadata type (`integer`/`string`/`real`/`other:<x>`), when metadata is present.
    pub meta_type: Option<String>,
    pub bit_offset: u32,
    pub bit_width: u32,
    pub range_low: Option<i64>,
    pub range_high: Option<i64>,
    pub key: Option<bool>,
    pub updatable: Option<bool>,
    pub fk_parent: Option<String>,
}

fn meta_type_str(t: &FieldType) -> String {
    match t {
        FieldType::Integer => "integer".to_string(),
        FieldType::String => "string".to_string(),
        FieldType::Real => "real".to_string(),
        FieldType::Other(s) => format!("other:{s}"),
    }
}

fn endian_str(e: Endian) -> &'static str {
    match e {
        Endian::Big => "big",
        Endian::Little => "little",
    }
}

/// Parse one main-DB anchor + its metadata into a snapshot. Returns a parse error string on failure.
pub fn snapshot(repo_root: &std::path::Path, a: &MainAnchor) -> Result<SchemaSnapshot, String> {
    let bytes = std::fs::read(repo_root.join(&a.db_rel)).map_err(|e| e.to_string())?;
    let db = TdbFile::parse(&bytes).map_err(|e| format!("{}: {e}", a.db_rel))?;
    let meta: Option<FieldMetaIndex> = match &a.meta_rel {
        Some(rel) => {
            let xml = std::fs::read_to_string(repo_root.join(rel)).map_err(|e| e.to_string())?;
            Some(FieldMetaIndex::from_xml(&xml).map_err(|e| format!("{rel}: {e}"))?)
        }
        None => None,
    };

    let mut tables = Vec::with_capacity(db.tables.len());
    let mut field_total = 0usize;
    for t in &db.tables {
        let t_raw = t.id.raw();
        let t_name = meta.as_ref().and_then(|m| m.table_name(&t_raw)).map(|s| s.to_string());
        let mut fields = Vec::with_capacity(t.fields.len());
        for f in &t.fields {
            let f_raw = f.id.raw();
            let fm = meta.as_ref().and_then(|m| m.field(&t_raw, &f_raw));
            fields.push(FieldSnapshot {
                raw_id: f_raw.clone(),
                canonical_id: f.id.canonical(db.endian),
                name: fm.map(|m| m.name.clone()),
                kind_code: f.kind,
                meta_type: fm.map(|m| meta_type_str(&m.kind)),
                bit_offset: f.bit_offset,
                bit_width: f.bit_width,
                range_low: fm.and_then(|m| m.range_low),
                range_high: fm.and_then(|m| m.range_high),
                key: fm.map(|m| m.key),
                updatable: fm.map(|m| m.updatable),
                fk_parent: fm.and_then(|m| m.fk_parent.clone()),
            });
        }
        field_total += fields.len();
        tables.push(TableSnapshot {
            raw_id: t_raw,
            canonical_id: t.id.canonical(db.endian),
            name: t_name,
            record_len_bytes: t.record_len_bytes,
            current_records: t.current_records,
            max_records: t.max_records,
            field_count: fields.len(),
            fields,
        });
    }

    Ok(SchemaSnapshot {
        game: a.game.clone(),
        platform: a.platform.clone(),
        generation: catalog::generation_of(&a.game, &a.platform).to_string(),
        db: a.db_file.clone(),
        endian: endian_str(db.endian).to_string(),
        tdb_version: 6,
        meta_present: meta.is_some(),
        meta_file: a.meta_file.clone(),
        table_count: tables.len(),
        field_total,
        tables,
    })
}

/// Serialize a snapshot to pretty JSON with a trailing newline (stable, diffable output).
pub fn to_json(snap: &SchemaSnapshot) -> String {
    let mut s = serde_json::to_string_pretty(snap).expect("snapshot serializes");
    s.push('\n');
    s
}

/// The committed filename for a snapshot: `<platform>-<game>.json`.
pub fn filename(a: &MainAnchor) -> String {
    format!("{}-{}.json", a.platform, a.game)
}
