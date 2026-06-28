//! A plain structural summary of the working database — the data behind the dashboard/project view,
//! produced by the domain layer so `studio-api` never has to parse TDB bytes itself.

use tdb_core::{Endian, TdbFile};

/// One table's at-a-glance shape.
#[derive(Clone, Debug)]
pub struct TableOverview {
    /// Canonical (community) id — the big-endian reversal already applied.
    pub id: String,
    pub field_count: usize,
    pub records: u32,
}

/// A structural summary of a parsed database.
#[derive(Clone, Debug)]
pub struct DbOverview {
    pub endian: Endian,
    pub db_size: u32,
    pub header_crc_ok: bool,
    pub size_ok: bool,
    pub table_count: usize,
    pub tables: Vec<TableOverview>,
}

impl DbOverview {
    /// Summarise an already-parsed database.
    pub fn of(db: &TdbFile) -> Self {
        let tables = db
            .tables
            .iter()
            .map(|t| TableOverview {
                id: t.id.canonical(db.endian),
                field_count: t.fields.len(),
                records: t.current_records as u32,
            })
            .collect();
        DbOverview {
            endian: db.endian,
            db_size: db.db_size,
            header_crc_ok: db.header_crc_ok(),
            size_ok: db.size_ok(),
            table_count: db.tables.len(),
            tables,
        }
    }
}
