//! Phase 3 — the **entity & relational operation matrix**. For every committed main-DB anchor, on a
//! *temporary copy*, this drives every implemented high-level roster operation through the exact
//! `EditSession` seam the UI/API calls (`studio_api::Registry` → `EditSession`), and proves the
//! contract from `docs/plans/full-game-qa-editability.md` § Phase 3:
//!
//!   * **Applicable** operations (the schema carries the tables/fields they need) run
//!     edit → validate → save → reopen → validate, hold every invariant, and are fully reversible
//!     (undo restores the anchor bytes exactly).
//!   * **Non-applicable** operations (the schema does not carry the feature) are **refused cleanly** —
//!     an `Err` that leaves the working bytes and the undo history untouched, never a half-committed
//!     transaction.
//!
//! Applicability is derived from the *loaded schema* (table + field presence), never the title id, so
//! gen6 (NHL 07 — no slot-key `toWX`, no equipment tables, no retired-number/draft tables) exercises
//! the clean-refusal path for real while the modern anchors exercise the full round-trip.
//!
//! Invariants asserted after every applied operation (the plan's battery):
//!   * the whole-DB audit introduces **no new Error finding** vs the anchor baseline (this single
//!     predicate subsumes dangling-FK, companion cardinality, single-captain, block coherence, and
//!     duplicate-on-team — see `tdb_domain_validation::validate_invariants`);
//!   * the entity model rebuilds; the saved DB re-parses with a valid header CRC and size;
//!   * the operation is reversible — undo returns the working bytes to the anchor byte-for-byte.
//!
//! Anchors are immutable: each session opens a temp copy. The per-anchor per-operation outcome is
//! emitted deterministically to `docs/audits/data/operation-matrix.csv` and guarded against drift —
//! regenerate an intended change with:
//!
//! ```text
//! QA_WRITE=1 cargo test -p studio-domain --test operation_matrix
//! ```

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use studio_domain::{
    EditSession, FieldValue, Platform, Project, ProjectRole, RosterModel, Severity,
    CAREER_SKATER_SEASON,
};
use tdb_core::TdbFile;
use tdb_qa::catalog::CATALOG;
use tdb_qa::evidence::emit::{file_digest, Emitter};
use tdb_qa::evidence::model::Outcome;
use tdb_qa::evidence::CoverageKey;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const CASE: &str = "studio_domain::operation_matrix::operation_matrix_over_all_anchors";

/// This emitter's operation name → the `db-operations` operation of contract §4.1 that its
/// assertions actually cover. Deliberately **partial**: an emitter operation with no honest key in
/// the closed vocabulary maps to `None` and is recorded nowhere (R-ABSENT — absence asserts
/// nothing). Guessing a family for it is exactly the E1/E2 positional mistake this contract exists
/// to remove, so the unmapped names are reported to the architect instead of being forced into a
/// neighbouring key:
///
/// * `release-then-sign`, `add-remove-membership` — free-agency / membership lifecycle, not a
///   slot-to-slot move;
/// * `autocorrect-jerseys`, `retire-unretire-jersey`, `repair-roster-blocks` — bulk repair and
///   team-level jersey administration, none of which is a scalar edit or a roster CRUD op;
/// * `trade-draft-pick` — an asset transfer that moves no roster slot;
/// * `career-add-delete` — a career statistics row, not a roster entity.
fn operation_key(op: &str) -> Option<&'static str> {
    match op {
        "create-skater" | "create-goalie" | "clone-skater" | "clone-goalie" => {
            Some("roster-create")
        }
        "delete-player" => Some("roster-delete"),
        "move-player" => Some("roster-move"),
        "trade-slots" => Some("roster-trade"),
        "set-captaincy" | "set-jersey" | "set-dressed" | "strategy-edit" => Some("scalar-edit"),
        _ => None,
    }
}

/// Why an execution carries no coverage key. Recorded verbatim on every unmapped record, so the
/// gap is countable rather than a comment in a source file.
const UNMAPPED_REASON: &str = "operation-not-in-closed-db-operations-vocabulary";

/// Every `db-operations` operation this emitter can reach, for the declaration.
const MAPPED_OPERATIONS: &[&str] = &[
    "roster-create",
    "roster-delete",
    "roster-move",
    "roster-trade",
    "scalar-edit",
];

// ----------------------------------------------------------------------------- temp copies

/// A throwaway directory holding writable copies of otherwise-immutable anchors.
struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-p3-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    fn copy_in(&self, src: &Path, name: &str) -> PathBuf {
        let dst = self.0.join(name);
        std::fs::copy(src, &dst).unwrap();
        dst
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ----------------------------------------------------------------------------- anchor discovery

/// (label, platform, db path, meta path) for every main-DB anchor — same discovery as the read/mutation matrices.
fn anchors() -> Vec<(String, Platform, PathBuf, PathBuf)> {
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
        let pdir = platform.file_name().to_string_lossy().into_owned();
        let Some(plat) = platform_of(&pdir) else {
            continue;
        };
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
                    format!("{}/{}", pdir, game.file_name().to_string_lossy()),
                    plat,
                    dbs.join(db),
                    dbs.join(meta),
                ));
            }
        }
    }
    // Debug affordance only: `NHLMS_ONLY=ps4/nhl18` narrows the run while iterating. The committed
    // gate always runs the full set (the 27-anchor assert below enforces that in CI).
    if let Ok(only) = std::env::var("NHLMS_ONLY") {
        out.retain(|(label, ..)| label.contains(&only));
    }
    out
}

fn platform_of(dir: &str) -> Option<Platform> {
    match dir {
        "ps3" => Some(Platform::Ps3),
        "xbox360" => Some(Platform::Xbox),
        "ps4" => Some(Platform::Ps4),
        _ => None,
    }
}

// ----------------------------------------------------------------------------- schema presence

/// The per-anchor schema flags each operation's applicability is derived from — table + field
/// presence read straight off the loaded file, never the title id.
struct Schema {
    eglu: bool, // eGlu — the roster-slot table (block repair peeks it; always no-ops cleanly)
    slot_key: bool, // eGlu.toWX — minted by cluster creation (create/clone)
    xswt: bool, // eGlu.XSWT — the occupancy pointer relocation ops move (delete/move/trade/…)
    skater_equip: bool, // xmja — required to build a skater cluster
    goalie_equip: bool, // fMVl — required to build a goalie cluster
    mccl: bool, // eGlu.mCcl — captaincy slot field
    svrt: bool, // eGlu.sVRt — jersey slot field
    hszj: bool, // eGlu.hSZj — dressed slot field
    retired: bool, // wcSL — retired-number table
    draft: bool, // qHav — draft-pick asset table
    strategies: bool, // EeYo — team-strategy table
    career: bool, // the skater-season career table carrying every field add_career_season writes
}

fn has_table(db: &TdbFile, t: &str) -> bool {
    db.table_canonical(t).is_some()
}

fn has_field(db: &TdbFile, t: &str, f: &str) -> bool {
    db.table_canonical(t)
        .map(|tab| tab.fields.iter().any(|fd| fd.id.canonical(db.endian) == f))
        .unwrap_or(false)
}

fn probe_schema(db: &TdbFile) -> Schema {
    Schema {
        eglu: has_table(db, "eGlu"),
        slot_key: has_field(db, "eGlu", "toWX"),
        xswt: has_field(db, "eGlu", "XSWT"),
        skater_equip: has_table(db, "xmja"),
        goalie_equip: has_table(db, "fMVl"),
        mccl: has_field(db, "eGlu", "mCcl"),
        svrt: has_field(db, "eGlu", "sVRt"),
        hszj: has_field(db, "eGlu", "hSZj"),
        retired: has_table(db, "wcSL"),
        draft: has_table(db, "qHav"),
        strategies: has_table(db, "EeYo"),
        // add_career_season writes the join (wBIz), year (qFnd), league flag (xMjj) and team code
        // (lyxL); gen6 (NHL07) ships tsSy without the league flag, so career editing is not applicable.
        career: has_field(db, CAREER_SKATER_SEASON, "qFnd")
            && has_field(db, CAREER_SKATER_SEASON, "xMjj")
            && has_field(db, CAREER_SKATER_SEASON, "lyxL"),
    }
}

// ----------------------------------------------------------------------------- fixtures

/// Deterministic entity representatives picked from a fresh model (stable ordering → drift-guardable).
struct Fixtures {
    team0: u64,
    /// A rostered skater `(team, player)`.
    skater: Option<(u64, u64)>,
    /// A rostered goalie `(team, player)`.
    goalie: Option<(u64, u64)>,
    /// Any rostered player `(team, player)` (skater preferred).
    any_rostered: Option<(u64, u64)>,
    /// A rostered player holding **exactly one** membership `(team, player)` — so releasing that team's
    /// slot makes them a true free agent (PS4 carries multi-roster players who would stay rostered).
    single_slot: Option<(u64, u64)>,
    /// Two rostered players on two distinct teams — the trade pair.
    trade_pair: Option<((u64, u64), (u64, u64))>,
    /// A team that currently owns a vacant (unoccupied) slot — the destination for move/membership.
    free_team: Option<u64>,
    /// A rostered player, on a team other than `free_team` and **not already a member of** `free_team`,
    /// so a move/extra-membership into `free_team`'s open slot can't create a duplicate membership.
    mover: Option<(u64, u64)>,
    /// A draft-pick `(record, to_team)` — a populated qHav row and a different owner.
    draft: Option<(u32, u64)>,
}

fn build_fixtures(model: &RosterModel, db: &TdbFile) -> Fixtures {
    // Rostered slots, stably ordered.
    let mut occupied: Vec<(u64, u64, u32)> = model
        .slots
        .iter()
        .filter_map(|s| s.player_id.map(|p| (s.team_id, p, s.record)))
        .collect();
    occupied.sort_by_key(|&(t, p, r)| (t, p, r));

    let is_skater = |p: u64| model.player(p).is_some_and(|pl| pl.skater_ai.is_some());
    let is_goalie = |p: u64| model.player(p).is_some_and(|pl| pl.goalie_ai.is_some());

    let skater = occupied
        .iter()
        .find(|&&(_, p, _)| is_skater(p))
        .map(|&(t, p, _)| (t, p));
    let goalie = occupied
        .iter()
        .find(|&&(_, p, _)| is_goalie(p))
        .map(|&(t, p, _)| (t, p));
    let any_rostered = skater
        .or(goalie)
        .or_else(|| occupied.first().map(|&(t, p, _)| (t, p)));

    // How many roster slots each player occupies, and the set of teams they belong to.
    let membership_count = |pid: u64| occupied.iter().filter(|&&(_, p, _)| p == pid).count();
    let member_of = |pid: u64, team: u64| occupied.iter().any(|&(t, p, _)| p == pid && t == team);

    // A single-membership rostered player — release frees their *only* slot → a true free agent.
    let single_slot = occupied
        .iter()
        .find(|&&(_, p, _)| membership_count(p) == 1)
        .map(|&(t, p, _)| (t, p));

    // Trade pair: the first two rostered players on distinct teams.
    let trade_pair = occupied.first().and_then(|&(t1, p1, _)| {
        occupied
            .iter()
            .find(|&&(t2, _, _)| t2 != t1)
            .map(|&(t2, p2, _)| ((t1, p1), (t2, p2)))
    });

    // A team owning a vacant slot.
    let mut vacant: Vec<u64> = model
        .slots
        .iter()
        .filter(|s| s.player_id.is_none())
        .map(|s| s.team_id)
        .collect();
    vacant.sort_unstable();
    vacant.dedup();
    let free_team = vacant.first().copied();

    // A mover: a rostered player on some team other than `free_team` who is **not already** a member
    // of `free_team` — so moving/adding them into its open slot can't duplicate an existing membership.
    let mover = free_team.and_then(|ft| {
        occupied
            .iter()
            .find(|&&(t, p, _)| t != ft && !member_of(p, ft))
            .map(|&(t, p, _)| (t, p))
    });

    // A populated draft pick + a different destination team.
    let draft = db.table_canonical("qHav").and_then(|t| {
        let owner_f = t
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "Uwdb")?;
        (0..t.current_records as u32).find_map(|rec| {
            let owner = db.read_u64(t, rec, owner_f).filter(|&v| v > 0)? - 1;
            let dest = model
                .teams
                .iter()
                .map(|tm| tm.team_id)
                .find(|&id| id != owner)?;
            Some((rec, dest))
        })
    });

    Fixtures {
        team0: model.teams.first().map(|t| t.team_id).unwrap_or(0),
        skater,
        goalie,
        any_rostered,
        single_slot,
        trade_pair,
        free_team,
        mover,
        draft,
    }
}

// ----------------------------------------------------------------------------- audit delta

/// The anchor's Error-severity audit fingerprint: the set of `(predicate, team, player)` keys plus the
/// total Error count. Deliberately **record-independent** — a structural op that inserts/removes a
/// record shifts every later record index, so keying on `record` would make identical pre-existing
/// findings look "new". "No newly introduced finding" (the plan's invariant, and exactly what the
/// engine's own delta-audit enforces) means: the post-op key set introduces no new entity, and the
/// total Error count does not rise above baseline.
fn error_fingerprint(sess: &EditSession) -> (BTreeSet<String>, usize) {
    let report = sess.validate().unwrap();
    let errors: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    let keys = errors
        .iter()
        .map(|f| format!("{}|{:?}|{:?}", f.predicate, f.team_id, f.player_id))
        .collect();
    (keys, errors.len())
}

// ----------------------------------------------------------------------------- the driver

/// Per-operation outcome for the CSV.
#[derive(Clone, Copy, PartialEq)]
enum Status {
    /// Applied, all invariants held, fully reversible.
    Pass,
    /// Not carried by this schema — refused cleanly (atomic Err).
    RefusedClean,
    /// Applicable, but this anchor ships no suitable entity to exercise it.
    NoFixture,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::RefusedClean => "refused-clean",
            Status::NoFixture => "no-fixture",
        }
    }
}

/// Run one operation and enforce the full Phase-3 contract. `expected` is the schema-derived
/// applicability; `op` performs the operation(s) through the session seam. The session must be at the
/// anchor baseline on entry and is left at the anchor baseline on exit.
///
/// * `expected == true`  → either `Pass` (op ran + invariants + reversible) or `NoFixture` (op was
///   never attempted because no representative existed). A clean refusal here is a **failure** — the
///   schema says the feature is present, so the op must not error.
/// * `expected == false` → `RefusedClean` — the op is attempted and must return `Err` without moving
///   a byte or the undo history.
fn run_op<F>(
    sess: &mut EditSession,
    anchor: &[u8],
    label: &str,
    op_name: &str,
    expected: bool,
    attempted: bool,
    op: F,
) -> Status
where
    F: FnOnce(&mut EditSession) -> studio_domain::Result<()>,
{
    let tag = format!("{label}/{op_name}");
    // Precondition: at baseline.
    assert_eq!(
        sess.bytes(),
        anchor,
        "{tag}: session not at baseline before the op"
    );
    assert!(!sess.is_dirty(), "{tag}: session dirty before the op");
    let (baseline_keys, baseline_errors) = error_fingerprint(sess);
    let d0 = sess.undo_depth();

    if !attempted {
        // Applicable by schema, but no representative entity exists to exercise it.
        return Status::NoFixture;
    }

    let result = op(sess);

    match result {
        Ok(()) => {
            assert!(
                expected,
                "{tag}: op succeeded but the schema does not carry this feature"
            );
            // (a) No new Error finding vs the anchor baseline (record-independent; count non-increasing).
            let (after_keys, after_errors) = error_fingerprint(sess);
            let introduced: Vec<&String> = after_keys.difference(&baseline_keys).collect();
            assert!(
                introduced.is_empty(),
                "{tag}: op introduced audit finding(s) for a new entity: {introduced:?}"
            );
            assert!(
                after_errors <= baseline_errors,
                "{tag}: op raised the Error-finding count {baseline_errors} → {after_errors}"
            );
            // (b) The entity model rebuilds and the saved DB re-parses CRC/size-valid.
            let saved = sess
                .save()
                .unwrap_or_else(|e| panic!("{tag}: save failed: {e}"));
            let db =
                TdbFile::parse(&saved).unwrap_or_else(|e| panic!("{tag}: re-parse failed: {e}"));
            assert!(
                db.header_crc_ok() && db.size_ok(),
                "{tag}: saved DB failed CRC/size"
            );
            RosterModel::build(&db).unwrap_or_else(|e| panic!("{tag}: reopened model failed: {e}"));
            // A second no-op save is deterministic.
            assert_eq!(
                saved,
                sess.save().unwrap(),
                "{tag}: re-save is not deterministic"
            );
            // (c) Reversible: undo every entry this op pushed → back to the anchor byte-for-byte. An
            // idempotent no-op op (autocorrect/repair on already-clean data) legitimately commits
            // nothing and pushes no entry; then the working bytes must already equal the anchor.
            let pushed = sess.undo_depth().saturating_sub(d0);
            for _ in 0..pushed {
                assert!(sess.undo(), "{tag}: undo returned false unwinding the op");
            }
            assert_eq!(
                sess.bytes(),
                anchor,
                "{tag}: undo did not restore the anchor bytes"
            );
            assert!(
                !sess.is_dirty(),
                "{tag}: session still dirty after full undo"
            );
            Status::Pass
        }
        Err(e) => {
            assert!(
                !expected,
                "{tag}: op refused but the schema carries this feature: {e}"
            );
            // Clean refusal: no bytes moved, no undo history grew.
            assert_eq!(
                sess.bytes(),
                anchor,
                "{tag}: a refused op changed the working bytes"
            );
            assert_eq!(
                sess.undo_depth(),
                d0,
                "{tag}: a refused op grew the undo history"
            );
            Status::RefusedClean
        }
    }
}

// ----------------------------------------------------------------------------- observation

/// [`run_op`] wrapped in an observation. The outcome is recorded **after** `run_op` returns, so a
/// `pass` only ever follows the assertions that earned it; a panic inside is recorded as an
/// executed `fail` and then resumed unchanged by [`Emitter::case`].
#[allow(clippy::too_many_arguments)]
fn observe_op<F>(
    ev: &mut Emitter,
    keys: &std::collections::BTreeMap<&'static str, CoverageKey>,
    fixture: &str,
    target: &str,
    sess: &mut EditSession,
    anchor: &[u8],
    label: &str,
    op_name: &'static str,
    expected: bool,
    attempted: bool,
    op: F,
) -> Status
where
    F: FnOnce(&mut EditSession) -> studio_domain::Result<()>,
{
    let case = format!("{CASE}#{target}-{op_name}");
    let key = operation_key(op_name).and_then(|id| keys.get(id)).cloned();

    // No honest key for this operation: the assertions still run exactly as before, and the
    // execution is recorded as an **unmapped** record — no coverage key, no axis, no qualification,
    // but not lost either. Forcing it onto a neighbouring key is the error the contract exists to
    // remove.
    let Some(key) = key else {
        let status = ev.case_unmapped(target, op_name, &case, UNMAPPED_REASON, || {
            run_op(sess, anchor, label, op_name, expected, attempted, op)
        });
        let (outcome, detail) = outcome_of(status);
        ev.observe_unmapped(
            target,
            op_name,
            &case,
            outcome,
            fixture,
            UNMAPPED_REASON,
            &format!("{op_name}: {detail}"),
        );
        return status;
    };

    let status = ev.case(&key, &case, || {
        run_op(sess, anchor, label, op_name, expected, attempted, op)
    });
    let (outcome, detail) = outcome_of(status);
    ev.observe_hashed(
        &key,
        &case,
        outcome,
        fixture,
        "",
        &format!("{op_name}: {detail}"),
    );
    status
}

/// What a [`Status`] means as an observation outcome. Identical for mapped and unmapped records —
/// the difference between them is whether a key exists, never how the run is judged.
fn outcome_of(status: Status) -> (Outcome, &'static str) {
    match status {
        Status::Pass => (
            Outcome::Pass,
            "applied; no new audit Error finding; saved, re-parsed and model-rebuilt; fully reversible",
        ),
        Status::RefusedClean => (
            Outcome::RefusedClean,
            "not carried by this schema; refused atomically with no byte or undo-history change",
        ),
        Status::NoFixture => (
            Outcome::SkippedMissingFixture,
            "applicable by schema, but this anchor ships no entity to exercise it",
        ),
    }
}

// ----------------------------------------------------------------------------- CSV

fn csv_cell(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ----------------------------------------------------------------------------- the test

#[test]
fn operation_matrix_over_all_anchors() {
    let mut csv = String::from("platform,game,operation,applicable,status\n");
    let mut checked = 0;
    let mut total_pass = 0;
    let mut total_refused = 0;
    let tmp = Tmp::new("op");

    let mut ev = Emitter::from_env("operation_matrix", "db-edit", &repo_root());
    // Declared before anything runs, over the whole catalog × every mapped operation.
    let declared: Vec<(
        String,
        std::collections::BTreeMap<&'static str, CoverageKey>,
    )> = CATALOG
        .iter()
        .map(|unit| {
            let target = format!("{}-{}", unit.game, unit.platform);
            let map = MAPPED_OPERATIONS
                .iter()
                .map(|op| (*op, ev.declare(&format!("{target}/db-operations/{op}"))))
                .collect();
            (target, map)
        })
        .collect();
    let empty: std::collections::BTreeMap<&'static str, CoverageKey> =
        std::collections::BTreeMap::new();
    let mut covered: Vec<String> = Vec::new();

    for (label, platform, db_path, meta_path) in anchors() {
        let (pdir, game) = label.split_once('/').unwrap();
        let target = format!("{game}-{pdir}");
        let keys = declared
            .iter()
            .find(|(t, _)| *t == target)
            .map(|(_, m)| m)
            .unwrap_or(&empty);
        let fixture = file_digest(&db_path).unwrap_or_default();
        if !keys.is_empty() {
            // The fixture's own path, not its directory (C0-EMIT1-R2): discovery verifies exactly
            // the file whose bytes were hashed.
            ev.input_digest(
                &db_path
                    .strip_prefix(repo(""))
                    .unwrap_or(&db_path)
                    .to_string_lossy()
                    .replace('\\', "/"),
                (!fixture.is_empty()).then(|| fixture.clone()),
            );
            covered.push(target.clone());
        }
        let work = tmp.copy_in(&db_path, &format!("{pdir}-{game}.db"));
        let project = Project::load(
            &work,
            platform,
            ProjectRole::Nhlng,
            &meta_path,
            repo("defs"),
        )
        .unwrap_or_else(|e| panic!("{label}: load: {e}"));
        let mut sess = EditSession::begin(project);

        let anchor = sess.bytes().to_vec();
        let db = TdbFile::parse(&anchor).unwrap();
        let schema = probe_schema(&db);
        let model = RosterModel::build(&db).unwrap();
        let fx = build_fixtures(&model, &db);
        drop(db);

        // (operation, expected-applicable, attempted, closure) — fixed order.
        let skater_pos = 2u64; // RW
        let goalie_pos = 4u64;

        // Rows accumulate in a stable, documented order.
        let mut rows: Vec<(&str, bool, Status)> = Vec::new();

        macro_rules! op {
            ($name:expr, $expected:expr, $attempted:expr, $body:expr) => {{
                let st = observe_op(
                    &mut ev, keys, &fixture, &target, &mut sess, &anchor, &label, $name, $expected,
                    $attempted, $body,
                );
                rows.push(($name, $expected, st));
            }};
        }

        // --- Player lifecycle ---

        // create-skater
        {
            let expected = schema.slot_key && schema.skater_equip;
            let team = fx.team0;
            op!("create-skater", expected, true, move |s| {
                s.create_player(team, skater_pos).map(|_| ())
            });
        }
        // create-goalie
        {
            let expected = schema.slot_key && schema.goalie_equip;
            let team = fx.team0;
            op!("create-goalie", expected, true, move |s| {
                s.create_player(team, goalie_pos).map(|_| ())
            });
        }
        // clone-skater
        {
            let expected = schema.slot_key && schema.skater_equip;
            let src = fx.skater;
            let dest = fx.free_team.or(Some(fx.team0));
            let attempted = !expected || (src.is_some() && dest.is_some());
            let arg = src.zip(dest);
            op!("clone-skater", expected, attempted, move |s| match arg {
                Some(((_, p), to)) => s.clone_player(p, to).map(|_| ()),
                // Not-applicable path still needs an invocation to prove clean refusal.
                None => s.clone_player(0xFFFF_FFFF, 0).map(|_| ()),
            });
        }
        // clone-goalie
        {
            let expected = schema.slot_key && schema.goalie_equip;
            let src = fx.goalie;
            let dest = fx.free_team.or(Some(fx.team0));
            let attempted = !expected || (src.is_some() && dest.is_some());
            let arg = src.zip(dest);
            op!("clone-goalie", expected, attempted, move |s| match arg {
                Some(((_, p), to)) => s.clone_player(p, to).map(|_| ()),
                None => s.clone_player(0xFFFF_FFFF, 0).map(|_| ()),
            });
        }
        // delete-player (delete an existing rostered player; undo restores). Delete tears down the
        // whole cluster including the equipment companions, so it needs the modern (fat) schema —
        // gen6 (no xmja/fMVl) refuses cleanly.
        {
            let expected = schema.skater_equip && schema.goalie_equip;
            let tgt = fx.any_rostered;
            let attempted = !expected || tgt.is_some();
            op!("delete-player", expected, attempted, move |s| match tgt {
                Some((_, p)) => s.delete_player(p).map(|_| ()),
                None => s.delete_player(0xFFFF_FFFF).map(|_| ()),
            });
        }
        // move-player (into a free slot on another team)
        {
            let expected = schema.xswt;
            let arg = fx.mover.zip(fx.free_team);
            let attempted = !expected || arg.is_some();
            op!("move-player", expected, attempted, move |s| match arg {
                Some(((from, p), to)) => s.move_player(p, from, to).map(|_| ()),
                None => s.move_player(0xFFFF_FFFF, 0, 0).map(|_| ()),
            });
        }
        // trade-slots (swap two rostered players on distinct teams)
        {
            let expected = schema.xswt;
            let arg = fx.trade_pair;
            let attempted = !expected || arg.is_some();
            op!("trade-slots", expected, attempted, move |s| match arg {
                Some(((ta, pa), (tb, pb))) => s.trade_slots(ta, pa, tb, pb).map(|_| ()),
                None => s.trade_slots(0, 0, 0, 0).map(|_| ()),
            });
        }
        // release-then-sign (release a single-membership player, sign them back into the freed slot)
        {
            let expected = schema.xswt;
            let tgt = fx.single_slot;
            let attempted = !expected || tgt.is_some();
            op!(
                "release-then-sign",
                expected,
                attempted,
                move |s| match tgt {
                    Some((team, p)) => {
                        s.release_to_fa(team, p)?;
                        s.sign_from_fa(p, team).map(|_| ())
                    }
                    None => s.release_to_fa(0, 0xFFFF_FFFF).map(|_| ()),
                }
            );
        }
        // add-then-remove-membership (extra membership on another team, then drop it)
        {
            let expected = schema.xswt;
            let arg = fx.mover.zip(fx.free_team);
            let attempted = !expected || arg.is_some();
            op!(
                "add-remove-membership",
                expected,
                attempted,
                move |s| match arg {
                    Some(((_, p), to)) => {
                        s.add_membership(p, to)?;
                        s.remove_membership(to, p).map(|_| ())
                    }
                    None => s.add_membership(0xFFFF_FFFF, 0).map(|_| ()),
                }
            );
        }

        // --- Team / roster lifecycle ---

        // set-captaincy (single-captain invariant by construction)
        {
            let expected = schema.mccl;
            let tgt = fx.any_rostered;
            let attempted = !expected || tgt.is_some();
            op!("set-captaincy", expected, attempted, move |s| match tgt {
                Some((team, p)) => s.set_captaincy(team, p, 1).map(|_| ()),
                None => s.set_captaincy(0, 0xFFFF_FFFF, 1).map(|_| ()),
            });
        }
        // set-jersey (unique within the team)
        {
            let expected = schema.svrt;
            let tgt = fx.any_rostered;
            let attempted = !expected || tgt.is_some();
            op!("set-jersey", expected, attempted, move |s| match tgt {
                Some((team, p)) => s.set_jersey_number(team, p, 98).map(|_| ()),
                None => s.set_jersey_number(0, 0xFFFF_FFFF, 98).map(|_| ()),
            });
        }
        // autocorrect-jerseys (idempotent; a no-op leaves no undo entry — treat as pass either way)
        {
            let expected = schema.svrt;
            op!("autocorrect-jerseys", expected, true, move |s| {
                s.autocorrect_duplicate_jerseys().map(|_| ())
            });
        }
        // set-dressed (per-slot lineup boolean)
        {
            let expected = schema.hszj;
            let tgt = fx.any_rostered;
            let attempted = !expected || tgt.is_some();
            op!("set-dressed", expected, attempted, move |s| match tgt {
                Some((team, p)) => s.set_dressed(team, p, false).map(|_| ()),
                None => s.set_dressed(0, 0xFFFF_FFFF, false).map(|_| ()),
            });
        }
        // retire-then-unretire a jersey number
        {
            let expected = schema.retired;
            let team = fx.team0;
            op!("retire-unretire-jersey", expected, true, move |s| {
                s.retire_jersey(team, 99)?;
                s.unretire_jersey(team, 99).map(|_| ())
            });
        }
        // repair-roster-blocks (idempotent structural repair; a no-op on a coherent/keyless layout)
        {
            let expected = schema.eglu;
            op!("repair-roster-blocks", expected, true, move |s| {
                s.repair_roster_blocks().map(|_| ())
            });
        }
        // trade-draft-pick
        {
            let expected = schema.draft;
            let arg = fx.draft;
            let attempted = !expected || arg.is_some();
            op!(
                "trade-draft-pick",
                expected,
                attempted,
                move |s| match arg {
                    Some((rec, to)) => s.trade_draft_pick(rec, to).map(|_| ()),
                    None => s.trade_draft_pick(0, 0).map(|_| ()),
                }
            );
        }
        // career-add-then-delete
        {
            let expected = schema.career;
            let tgt = fx.any_rostered;
            let attempted = !expected || tgt.is_some();
            op!(
                "career-add-delete",
                expected,
                attempted,
                move |s| match tgt {
                    Some((_, p)) => {
                        let (rec, _) =
                            s.add_career_season(CAREER_SKATER_SEASON, p, 2016, 1, "PIT")?;
                        s.delete_career_row(CAREER_SKATER_SEASON, rec).map(|_| ())
                    }
                    None => s
                        .add_career_season(CAREER_SKATER_SEASON, 0xFFFF_FFFF, 2016, 1, "PIT")
                        .map(|_| ()),
                }
            );
        }
        // strategy-edit (EeYo cell + JUfQ career mirror)
        {
            let expected = schema.strategies;
            // Resolve an editable, bounded tactic cell for team0.
            let pick = if expected {
                sess.team_strategies(fx.team0)
                    .ok()
                    .flatten()
                    .and_then(|st| {
                        st.tactics.iter().find_map(|c| {
                            let f = &c.field;
                            match (f.editable, f.range_low, f.range_high, f.decoded) {
                                (true, Some(lo), Some(hi), Some(cur)) if hi > lo => {
                                    let next = if cur == lo { hi } else { lo };
                                    Some((st.record, f.studio_field.clone(), next))
                                }
                                _ => None,
                            }
                        })
                    })
            } else {
                None
            };
            let attempted = !expected || pick.is_some();
            op!("strategy-edit", expected, attempted, move |s| match pick {
                Some((rec, field, val)) => {
                    s.set_field("EeYo", rec, &field, FieldValue::Int(val))
                        .map(|_| ())
                }
                None => s
                    .set_field("EeYo", 0, "RBGh", FieldValue::Int(1))
                    .map(|_| ()),
            });
        }

        for (name, expected, st) in &rows {
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                csv_cell(pdir),
                csv_cell(game),
                csv_cell(name),
                if *expected { "yes" } else { "no" },
                st.as_str(),
            ));
            match st {
                Status::Pass => total_pass += 1,
                Status::RefusedClean => total_refused += 1,
                Status::NoFixture => {}
            }
        }
        checked += 1;
    }

    for (target, keys) in &declared {
        if covered.contains(target) {
            continue;
        }
        for key in keys.values() {
            ev.observe(
                key,
                &format!("{CASE}#{target}"),
                Outcome::SkippedMissingFixture,
                "no main-DB anchor for this unit; nothing was executed",
            );
        }
    }

    assert_eq!(checked, 27, "expected 27 anchors, checked {checked}");
    assert!(total_pass >= 1, "no operation ever passed");
    assert!(
        total_refused >= 1,
        "the clean-refusal path was never exercised"
    );

    let path = repo("docs/audits/data/operation-matrix.csv");
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
            "operation-matrix.csv drift — regenerate with `QA_WRITE=1 cargo test -p studio-domain --test operation_matrix`\n{drift}"
        );
    }
    ev.finish();
}
