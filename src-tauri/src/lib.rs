//! NHL Modding Studio — Tauri command surface (Phase 0 skeleton).
//!
//! Thin layer over the `tdb-core` engine. DTOs derive `ts-rs::TS` so the TypeScript bindings in
//! `app/src/bindings/` are generated from Rust (run `cargo test -p nhl-studio`), keeping the
//! frontend/backend contract drift-free.

use serde::Serialize;
use tdb_core::TdbFile;
use ts_rs::TS;

/// Summary of a parsed TDB database (one table row per `TableSummary`).
#[derive(Serialize, TS)]
#[ts(export)]
pub struct DbSummary {
    pub endian: String,
    pub table_count: usize,
    pub db_size: u32,
    pub header_crc_ok: bool,
    pub size_ok: bool,
    pub tables: Vec<TableSummary>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct TableSummary {
    /// Canonical (community) id — handles the big-endian reversal.
    pub id: String,
    pub field_count: usize,
    pub records: u32,
}

/// Open a TDB database file and return a structural summary.
#[tauri::command]
fn open_database(path: String) -> Result<DbSummary, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("read {path}: {e}"))?;
    let db = TdbFile::parse(&bytes).map_err(|e| e.to_string())?;
    let tables = db
        .tables
        .iter()
        .map(|t| TableSummary {
            id: t.id.canonical(db.endian),
            field_count: t.fields.len(),
            records: t.current_records as u32,
        })
        .collect();
    Ok(DbSummary {
        endian: format!("{:?}", db.endian),
        table_count: db.tables.len(),
        db_size: db.db_size,
        header_crc_ok: db.header_crc_ok(),
        size_ok: db.size_ok(),
        tables,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![open_database])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod bindings {
    use super::*;
    /// Writes the TypeScript bindings to `app/src/bindings/` (the FE/BE contract).
    #[test]
    fn export_typescript() {
        // Export to the frontend's bindings dir (absolute, via the ts-rs base-dir env var).
        std::env::set_var(
            "TS_RS_EXPORT_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../app/src/bindings"),
        );
        DbSummary::export_all().expect("export DbSummary + deps");
    }
}
