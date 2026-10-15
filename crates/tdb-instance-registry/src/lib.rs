//! `tdb-instance-registry` — a **non-blocking** SQLite sidecar that remembers which corpus
//! person-seasons have already been imported into which EA target, and as which player.
//!
//! It exists to power one gentle UX signal: when a Roster Builder cart item was **already** imported
//! to this target, preview *warns* — it never blocks. Nothing in the import pipeline depends on the
//! ledger being present or writable, so every failure mode here degrades to **"no ledger"** rather
//! than failing an import:
//!
//! * a missing file is created;
//! * a file that cannot be opened, migrated, or is not a valid database opens **disabled** — reads
//!   return "not found", writes are silent no-ops;
//! * a write that fails mid-session is surfaced as an error the caller is free to ignore (the service
//!   layer does), never propagated into the edit transaction.
//!
//! The natural key is `(person_id, season_year, target)`, where `target` is an opaque, caller-chosen
//! identity for the EA database an import lands in (e.g. a project/instance id or a db fingerprint) —
//! the crate stays agnostic about how that identity is formed.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

/// The ledger schema version. Bump when the table shape changes; an older/newer file that fails to
/// migrate simply opens disabled (the ledger is advisory, so a version skew never blocks an import).
const SCHEMA_VERSION: i64 = 1;

/// One recorded import: a corpus person-season that landed on an EA target as `ea_player_id`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerEntry {
    pub person_id: i64,
    pub season_year: i64,
    /// Opaque identity of the EA database this import targeted.
    pub target: String,
    /// The `wBIz` created (or updated) for this person-season on the target.
    pub ea_player_id: i64,
    /// When it was recorded (unix seconds), supplied by the caller so the crate stays clock-free.
    pub imported_at: i64,
}

/// Anything that can go wrong touching the ledger. Callers treat these as **non-fatal** — the import
/// has already succeeded (or is being previewed); a ledger error only means the advisory record was
/// not written/read.
#[derive(Debug)]
pub enum RegistryError {
    Sqlite(rusqlite::Error),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Sqlite(e) => write!(f, "instance-registry ledger error: {e}"),
        }
    }
}
impl std::error::Error for RegistryError {}
impl From<rusqlite::Error> for RegistryError {
    fn from(e: rusqlite::Error) -> Self {
        RegistryError::Sqlite(e)
    }
}

/// The import ledger. Holds an open SQLite connection when enabled, or nothing when it has degraded to
/// "no ledger" — every method stays valid in both states.
pub struct InstanceRegistry {
    conn: Option<Connection>,
}

impl InstanceRegistry {
    /// Open (creating if absent) the ledger at `path`. **Never fails**: any error opening or migrating
    /// the file yields a **disabled** ledger rather than an error, so a corrupt or read-only sidecar
    /// can never block an import. Use [`is_enabled`](Self::is_enabled) to tell which state you got.
    pub fn open(path: impl AsRef<Path>) -> Self {
        // Any error opening/migrating — or a schema-version skew — degrades to "no ledger".
        match Connection::open(path) {
            Ok(conn) if matches!(migrate(&conn), Ok(true)) => InstanceRegistry { conn: Some(conn) },
            _ => InstanceRegistry { conn: None },
        }
    }

    /// An in-memory ledger (tests / ephemeral sessions). Enabled on success.
    pub fn open_in_memory() -> Result<Self, RegistryError> {
        let conn = Connection::open_in_memory()?;
        migrate(&conn)?; // a fresh db always migrates to the current version
        Ok(InstanceRegistry { conn: Some(conn) })
    }

    /// A permanently-disabled ledger ("no ledger"): reads are empty, writes are no-ops. Handy when a
    /// caller wants the import pipeline to run with the ledger explicitly off.
    pub fn disabled() -> Self {
        InstanceRegistry { conn: None }
    }

    /// True when a backing database is open. When false, every read is empty and every write no-ops.
    pub fn is_enabled(&self) -> bool {
        self.conn.is_some()
    }

    /// Record (or overwrite) one import. A re-import of the same `(person, season, target)` updates the
    /// stored `ea_player_id`/`imported_at`. A **no-op** (returns `Ok`) when disabled.
    pub fn record(&self, entry: &LedgerEntry) -> Result<(), RegistryError> {
        let Some(conn) = &self.conn else {
            return Ok(());
        };
        conn.execute(
            "INSERT INTO imports(person_id, season_year, target, ea_player_id, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(person_id, season_year, target)
             DO UPDATE SET ea_player_id = excluded.ea_player_id, imported_at = excluded.imported_at",
            params![
                entry.person_id,
                entry.season_year,
                entry.target,
                entry.ea_player_id,
                entry.imported_at
            ],
        )?;
        Ok(())
    }

    /// The prior import of `(person_id, season_year)` on `target`, if any — the dedup-warning source.
    /// `None` when never imported **or** when the ledger is disabled (indistinguishable by design: a
    /// disabled ledger simply has nothing to warn about).
    pub fn lookup(
        &self,
        person_id: i64,
        season_year: i64,
        target: &str,
    ) -> Result<Option<LedgerEntry>, RegistryError> {
        let Some(conn) = &self.conn else {
            return Ok(None);
        };
        let row = conn
            .query_row(
                "SELECT person_id, season_year, target, ea_player_id, imported_at
                 FROM imports WHERE person_id = ?1 AND season_year = ?2 AND target = ?3",
                params![person_id, season_year, target],
                row_to_entry,
            )
            .optional()?;
        Ok(row)
    }

    /// Every import recorded against `target` (newest first). Empty when disabled.
    pub fn entries_for_target(&self, target: &str) -> Result<Vec<LedgerEntry>, RegistryError> {
        let Some(conn) = &self.conn else {
            return Ok(Vec::new());
        };
        let mut stmt = conn.prepare(
            "SELECT person_id, season_year, target, ea_player_id, imported_at
             FROM imports WHERE target = ?1 ORDER BY imported_at DESC, person_id",
        )?;
        let rows = stmt
            .query_map(params![target], row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

fn row_to_entry(r: &rusqlite::Row<'_>) -> rusqlite::Result<LedgerEntry> {
    Ok(LedgerEntry {
        person_id: r.get(0)?,
        season_year: r.get(1)?,
        target: r.get(2)?,
        ea_player_id: r.get(3)?,
        imported_at: r.get(4)?,
    })
}

/// Create the schema if absent and check the version. Returns `Ok(true)` when the ledger is usable at
/// the current version, `Ok(false)` on a version skew (caller opens disabled). A hard SQLite error
/// (including "not a database") bubbles up so [`InstanceRegistry::open`] also degrades to disabled.
fn migrate(conn: &Connection) -> rusqlite::Result<bool> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS imports (
             person_id    INTEGER NOT NULL,
             season_year  INTEGER NOT NULL,
             target       TEXT    NOT NULL,
             ea_player_id INTEGER NOT NULL,
             imported_at  INTEGER NOT NULL,
             PRIMARY KEY (person_id, season_year, target)
         );
         CREATE INDEX IF NOT EXISTS idx_imports_target ON imports(target);",
    )?;
    let found: Option<i64> = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    match found {
        Some(v) => Ok(v == SCHEMA_VERSION), // skew → false → open disabled
        None => {
            conn.execute(
                "INSERT INTO meta(key, value) VALUES ('schema_version', ?1)",
                params![SCHEMA_VERSION.to_string()],
            )?;
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(person: i64, season: i64, target: &str, ea: i64, at: i64) -> LedgerEntry {
        LedgerEntry {
            person_id: person,
            season_year: season,
            target: target.into(),
            ea_player_id: ea,
            imported_at: at,
        }
    }

    #[test]
    fn round_trip_and_dedup_lookup() {
        let reg = InstanceRegistry::open_in_memory().unwrap();
        assert!(reg.is_enabled());
        assert!(
            reg.lookup(1, 2024, "gameA").unwrap().is_none(),
            "nothing imported yet"
        );

        let e = entry(1, 2024, "gameA", 5001, 1000);
        reg.record(&e).unwrap();

        // A re-preview of the same item surfaces the dedup warning (the prior entry).
        assert_eq!(reg.lookup(1, 2024, "gameA").unwrap(), Some(e.clone()));
        // Scoped by target: the same person-season on another target is not a dup.
        assert!(reg.lookup(1, 2024, "gameB").unwrap().is_none());
    }

    #[test]
    fn reimport_overwrites_same_key() {
        let reg = InstanceRegistry::open_in_memory().unwrap();
        reg.record(&entry(1, 2024, "gameA", 5001, 1000)).unwrap();
        reg.record(&entry(1, 2024, "gameA", 6002, 2000)).unwrap(); // re-imported as a different player
        let got = reg.lookup(1, 2024, "gameA").unwrap().unwrap();
        assert_eq!(got.ea_player_id, 6002, "the latest import wins");
        assert_eq!(got.imported_at, 2000);
        assert_eq!(
            reg.entries_for_target("gameA").unwrap().len(),
            1,
            "still one row for the key"
        );
    }

    #[test]
    fn disabled_ledger_is_a_silent_noop() {
        let reg = InstanceRegistry::disabled();
        assert!(!reg.is_enabled());
        // Writes no-op, reads are empty — never an error, so an import is never blocked.
        reg.record(&entry(1, 2024, "gameA", 5001, 1000)).unwrap();
        assert!(reg.lookup(1, 2024, "gameA").unwrap().is_none());
        assert!(reg.entries_for_target("gameA").unwrap().is_empty());
    }

    #[test]
    fn corrupt_file_opens_disabled_not_error() {
        // A non-database file must degrade to "no ledger", never fail the import.
        let dir = std::env::temp_dir().join(format!("tir-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.db");
        std::fs::write(&path, b"this is definitely not sqlite").unwrap();

        let reg = InstanceRegistry::open(&path);
        assert!(!reg.is_enabled(), "corrupt file opened disabled");
        assert!(reg.lookup(1, 2024, "gameA").unwrap().is_none());

        std::fs::remove_dir_all(&dir).ok();
    }
}
