//! Phase-0 QA inventory generator (deterministic).
//!
//! Reproduces four committed artifacts from the immutable `anchors/` ground truth:
//!   * `docs/audits/data/anchors-manifest.csv`   — every anchor file, role, size, sha256
//!   * `docs/audits/data/schema/<platform>-<game>.json` (×27) — per main-DB schema snapshots
//!   * `docs/audits/data/schema-diff.csv`         — field-level cross-title / cross-platform diff
//!   * `docs/audits/data/game-qa-matrix.csv`      — the QA ledger (game×platform×feature-family)
//!
//! Everything is derived from bytes on disk (no title-based assumptions) and emitted in a stable
//! order, so regeneration is byte-identical and drift (anchor add/remove/edit, schema change) shows
//! up as a diff. The library exposes pure builders so the regression tests can regenerate in memory
//! and compare against the committed files; the `qa-inventory` bin writes them to disk.

use std::path::Path;

pub mod catalog;
pub mod diff;
pub mod evidence;
pub mod manifest;
pub mod matrix;
pub mod schema;

/// One committed main-DB anchor (the 27 the schema snapshots + diff are built from).
#[derive(Debug, Clone)]
pub struct MainAnchor {
    /// Filesystem platform dir: `ps3` | `xbox360` | `ps4`.
    pub platform: String,
    pub game: String,
    pub db_file: String,
    pub meta_file: Option<String>,
    /// Repo-relative slash path to the DB.
    pub db_rel: String,
    /// Repo-relative slash path to the metadata XML, when present.
    pub meta_rel: Option<String>,
}

/// The (db, meta) name pairs that identify a main database, in preference order.
const MAIN_DB_PAIRS: &[(&str, &str)] = &[
    ("NHL07NG.db", "NHL07NG-meta.xml"),
    ("nhlng.db", "nhlng-meta.xml"),
    ("nhl_game.db", "nhl_game-meta.xml"),
];

/// Auto-discover every committed main-DB anchor under `<repo_root>/anchors`, mirroring the
/// `field_profile` gate's discovery. Sorted by (platform, game) for deterministic output. A main DB
/// is included even if its sibling metadata is absent (recorded as `meta_rel = None`), but the
/// catalog's 27 all ship metadata.
pub fn discover_main_anchors(repo_root: &Path) -> std::io::Result<Vec<MainAnchor>> {
    let root = repo_root.join("anchors");
    let mut out = Vec::new();
    let mut platforms: Vec<_> = std::fs::read_dir(&root)?
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    platforms.sort_by_key(|e| e.file_name());
    for platform in platforms {
        let mut games: Vec<_> = std::fs::read_dir(platform.path())?
            .flatten()
            .filter(|e| e.path().is_dir())
            .collect();
        games.sort_by_key(|e| e.file_name());
        for game in games {
            let databases = game.path().join("databases");
            let Some((db_file, meta_file)) = MAIN_DB_PAIRS
                .iter()
                .find(|(db, _)| databases.join(db).is_file())
            else {
                continue;
            };
            let platform_name = platform.file_name().to_string_lossy().to_string();
            let game_name = game.file_name().to_string_lossy().to_string();
            let meta_present = databases.join(meta_file).is_file();
            out.push(MainAnchor {
                db_rel: format!("anchors/{platform_name}/{game_name}/databases/{db_file}"),
                meta_rel: meta_present
                    .then(|| format!("anchors/{platform_name}/{game_name}/databases/{meta_file}")),
                platform: platform_name,
                game: game_name,
                db_file: db_file.to_string(),
                meta_file: meta_present.then(|| meta_file.to_string()),
            });
        }
    }
    Ok(out)
}

/// Resolve the repository root from this crate's manifest dir (`crates/tdb-qa` → repo root).
pub fn repo_root_from_manifest() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
