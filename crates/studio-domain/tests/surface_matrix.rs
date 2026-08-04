//! Phase 1A machine-readable artifact: the per-anchor **surface-capability matrix**. For every main-DB
//! anchor, the probed state of every user-facing surface, emitted deterministically to
//! `docs/audits/data/surface-capabilities.csv` and guarded against drift here (same pattern as the
//! Phase-0 `tdb-qa` snapshots). Regenerate after an intentional probe change with:
//!
//! ```text
//! QA_WRITE=1 cargo test -p studio-domain --test surface_matrix
//! ```

use std::path::{Path, PathBuf};

use studio_domain::{Capabilities, SurfaceCapabilities, SurfaceCapability, SurfaceState};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_relations::RosterModel;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn anchors() -> Vec<(String, String, PathBuf, PathBuf)> {
    let root = repo("anchors");
    let pairs = [
        ("NHL07NG.db", "NHL07NG-meta.xml"),
        ("nhlng.db", "nhlng-meta.xml"),
        ("nhl_game.db", "nhl_game-meta.xml"),
    ];
    let mut out = Vec::new();
    let mut platforms: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().collect();
    platforms.sort_by_key(|e| e.file_name());
    for platform in platforms.iter().filter(|e| e.path().is_dir()) {
        let mut games: Vec<_> = std::fs::read_dir(platform.path()).unwrap().flatten().collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let dbs = game.path().join("databases");
            if let Some((db, meta)) = pairs.iter().find(|(d, m)| dbs.join(d).is_file() && dbs.join(m).is_file()) {
                out.push((
                    platform.file_name().to_string_lossy().to_string(),
                    game.file_name().to_string_lossy().to_string(),
                    dbs.join(db),
                    dbs.join(meta),
                ));
            }
        }
    }
    out
}

fn state_str(s: SurfaceState) -> &'static str {
    match s {
        SurfaceState::NotApplicable => "not-applicable",
        SurfaceState::Readable => "readable",
        SurfaceState::Editable => "editable",
        SurfaceState::Disabled => "disabled",
        SurfaceState::Unverified => "unverified",
    }
}

/// The 18 surfaces in a fixed order (matches the DTO/plan list).
fn surfaces_in_order(s: &SurfaceCapabilities) -> [(&'static str, SurfaceCapability); 18] {
    [
        ("players", s.players),
        ("teams", s.teams),
        ("arenas", s.arenas),
        ("career", s.career),
        ("attributes", s.attributes),
        ("traits", s.traits),
        ("lines", s.lines),
        ("strategies", s.strategies),
        ("captaincy", s.captaincy),
        ("contracts", s.contracts),
        ("draft_picks", s.draft_picks),
        ("jerseys", s.jerseys),
        ("team_equipment", s.team_equipment),
        ("player_equipment", s.player_equipment),
        ("portraits", s.portraits),
        ("logos", s.logos),
        ("heads", s.heads),
        ("audio", s.audio),
    ]
}

fn build_csv() -> String {
    let mut out = String::from("platform,game,surface,state,reason\n");
    for (platform, game, db_path, meta_path) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let model = RosterModel::build(&db).unwrap();
        let caps = Capabilities::probe(&db, &meta);
        let surf = SurfaceCapabilities::probe(&db, &meta, &model, &caps);
        for (name, cap) in surfaces_in_order(&surf) {
            out.push_str(&format!(
                "{},{},{},{},{}\n",
                platform,
                game,
                name,
                state_str(cap.state),
                cap.reason.unwrap_or(""),
            ));
        }
    }
    out
}

#[test]
fn surface_matrix_matches_committed() {
    let csv = build_csv();
    let path = repo("docs/audits/data/surface-capabilities.csv");
    if std::env::var("QA_WRITE").is_ok() {
        std::fs::write(&path, &csv).unwrap();
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        csv, committed,
        "surface-capabilities.csv drift — regenerate with `QA_WRITE=1 cargo test -p studio-domain --test surface_matrix`"
    );
}
