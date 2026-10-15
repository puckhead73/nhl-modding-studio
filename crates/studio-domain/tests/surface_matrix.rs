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
use tdb_qa::catalog::CATALOG;
use tdb_qa::evidence::emit::{file_digest, Emitter};
use tdb_qa::evidence::model::Outcome;
use tdb_qa::evidence::CoverageKey;
use tdb_relations::RosterModel;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A path under the repository, spelled repository-relative with `/`, as the run attributes it.
fn repo_relative(path: &Path) -> String {
    path.strip_prefix(repo(""))
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The single coverage key this emitter maps onto — contract §2.2.
///
/// What the test below actually asserts, per anchor, is that the main database parses, its metadata
/// index loads, the entity model builds and every user-facing surface probes to a recorded state.
/// That is `db-presentation/read` and nothing else: it saves nothing, so it claims nothing about
/// `save-raw-db`, and a probed `portraits`/`heads` *state* is not asset coverage.
const OPERATION: &str = "db-presentation/read";
const CASE: &str = "studio_domain::surface_matrix::surface_matrix_matches_committed";

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
        let mut games: Vec<_> = std::fs::read_dir(platform.path())
            .unwrap()
            .flatten()
            .collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let dbs = game.path().join("databases");
            if let Some((db, meta)) = pairs
                .iter()
                .find(|(d, m)| dbs.join(d).is_file() && dbs.join(m).is_file())
            {
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

fn build_csv(ev: &mut Emitter) -> String {
    // Declare the whole catalog first: a unit this emitter covers but finds no anchor for is a
    // recorded `skipped-missing-fixture`, never an absence that would read as "not covered".
    let keys: Vec<(String, CoverageKey)> = CATALOG
        .iter()
        .map(|unit| {
            let target = format!("{}-{}", unit.game, unit.platform);
            let key = ev.declare(&format!("{target}/{OPERATION}"));
            (target, key)
        })
        .collect();

    let mut out = String::from("platform,game,surface,state,reason\n");
    let mut covered: Vec<String> = Vec::new();
    for (platform, game, db_path, meta_path) in anchors() {
        let target = format!("{game}-{platform}");
        let Some((_, key)) = keys.iter().find(|(t, _)| *t == target) else {
            // An anchor tree with no catalog unit: it is recorded nowhere rather than guessed at.
            continue;
        };
        let key = key.clone();
        let fixture = file_digest(&db_path).unwrap_or_default();
        // Attribute the fixture on the run (C0-EMIT1-R2): the repository-relative path with its
        // captured content hash, so discovery can compare it with the file's current bytes. An
        // unreadable anchor is recorded as `unknown`, never as a digest-shaped placeholder.
        ev.input_digest(
            &repo_relative(&db_path),
            (!fixture.is_empty()).then(|| fixture.clone()),
        );
        let case = format!("{CASE}#{target}");
        let rows = ev.case(&key, &case, || {
            let bytes = std::fs::read(&db_path).unwrap();
            let db = TdbFile::parse(&bytes).unwrap();
            let meta =
                FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
            let model = RosterModel::build(&db).unwrap();
            let caps = Capabilities::probe(&db, &meta);
            let surf = SurfaceCapabilities::probe(&db, &meta, &model, &caps);
            let mut rows = String::new();
            for (name, cap) in surfaces_in_order(&surf) {
                rows.push_str(&format!(
                    "{},{},{},{},{}\n",
                    platform,
                    game,
                    name,
                    state_str(cap.state),
                    cap.reason.unwrap_or(""),
                ));
            }
            rows
        });
        // Recorded only here: the parse, the metadata load, the model build and all 18 probes are
        // done and none of them panicked.
        ev.observe_hashed(
            &key,
            &case,
            Outcome::Pass,
            &fixture,
            "",
            "parsed the main DB, built the entity model and probed all 18 surfaces",
        );
        covered.push(target);
        out.push_str(&rows);
    }
    for (target, key) in &keys {
        if !covered.contains(target) {
            ev.observe(
                key,
                &format!("{CASE}#{target}"),
                Outcome::SkippedMissingFixture,
                "no main-DB anchor for this unit; nothing was executed",
            );
        }
    }
    out
}

#[test]
fn surface_matrix_matches_committed() {
    let mut ev = Emitter::from_env("surface_matrix", "db-read", &repo_root());
    let csv = build_csv(&mut ev);
    let path = repo("docs/audits/data/surface-capabilities.csv");
    if std::env::var("QA_WRITE").is_ok() {
        std::fs::write(&path, &csv).unwrap();
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    // Record-level, not byte-level: the generator writes LF and a Windows checkout materialises
    // the committed file CRLF, so byte equality failed from a clean checkout while passing in a
    // working tree (C0-EVID-PROBE1 §4.2). `compare_records` treats the two legal record
    // terminators as equivalent and nothing else — a CR inside a quoted field stays payload, and
    // every value, field, row order and row count is still compared verbatim.
    if let Err(drift) = tdb_qa::evidence::csv::compare_records(&csv, &committed) {
        // Deliberately NOT `ev.finish()`: the drop guard must seal this run `failed`, exactly as
        // the previous `assert_eq!` left it. A drift failure never yields a selectable run.
        panic!(
            "surface-capabilities.csv drift — regenerate with `QA_WRITE=1 cargo test -p studio-domain --test surface_matrix`\n{drift}"
        );
    }
    ev.finish();
}
