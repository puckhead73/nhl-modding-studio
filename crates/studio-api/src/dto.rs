//! Data-transfer objects — the typed FE/BE contract. Each derives `serde::Serialize` and
//! `ts_rs::TS`, so the TypeScript definitions are generated from Rust (no hand-maintained `dto.ts`).
//! The domain↔DTO mapping lives in [`crate::convert`].

use serde::Serialize;
use ts_rs::TS;

/// One table's at-a-glance shape.
#[derive(Serialize, TS)]
pub struct TableSummaryDto {
    /// Canonical (community) id — the big-endian reversal already applied.
    pub id: String,
    pub field_count: usize,
    pub records: u32,
}

/// A loaded project plus a structural summary of its current working database.
#[derive(Serialize, TS)]
pub struct ProjectSummaryDto {
    pub project_id: u32,
    pub platform: String,
    pub role: String,
    /// How the file was loaded / will be saved: `"raw_db"` or `"container"`.
    pub source: String,
    pub endian: String,
    pub table_count: usize,
    pub db_size: u32,
    pub header_crc_ok: bool,
    pub size_ok: bool,
    /// `true` once the working bytes differ from the loaded baseline.
    pub dirty: bool,
    pub tables: Vec<TableSummaryDto>,
}

/// One integrity finding from the Layer 4 audit.
#[derive(Serialize, TS)]
pub struct FindingDto {
    /// `"error"` | `"warning"` | `"info"`.
    pub severity: String,
    pub predicate: String,
    pub message: String,
    pub table: String,
    pub field: String,
    pub record: Option<u32>,
}

/// The result of a whole-DB audit.
#[derive(Serialize, TS)]
pub struct IntegrityReportDto {
    /// `true` when there are no `error`-severity findings (safe to write).
    pub clean: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub findings: Vec<FindingDto>,
}

/// What an edit operation did: cell-edit count plus a description of each auto-repair.
#[derive(Serialize, TS)]
pub struct OpOutcomeDto {
    pub edits: u32,
    pub repairs: Vec<String>,
}

/// The outcome of creating a player: the new `wBIz` id plus the op outcome.
#[derive(Serialize, TS)]
pub struct CreatePlayerDto {
    pub player_id: u64,
    pub outcome: OpOutcomeDto,
}
