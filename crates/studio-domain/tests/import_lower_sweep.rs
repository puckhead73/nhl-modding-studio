//! Players 3.2 (25 Sep 2026, package P3.2-C) — the historical **import** creates a coherent player
//! on every committed main-database anchor (27), not only on the NHL 14 / NHL 10 anchors the
//! Phase 5.2b gate used.
//!
//! One synthetic person-season (no corpus needed) is lowered with `import::lower_create` against
//! each anchor — so every per-title probe flows through the import path: height + 54, the weight
//! base (150 on NHL 07–10, 120 after), the birth month / day index (1-indexed on 07–10, 0-indexed
//! after), the rating scale (50 / 36 / 0) — applied as one cart, read back through the display
//! path, checked to sit on the target team's roster with a whole-database audit no worse than the
//! anchor's, saved to a disposable copy and found again after reopening. Where the schema cannot
//! carry the operation (NHL 07: no slot key, no equipment companions) the refusal is asserted to be
//! clean and is recorded as such. Fields the title lacks are recorded as skipped, never silently.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::{
    build_player_attributes, entity_fields, import, EditSession, Platform, Project, ProjectRole,
};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-import-sweep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn platform_of(dir: &str) -> Platform {
    match dir {
        "ps3" => Platform::Ps3,
        "ps4" => Platform::Ps4,
        "xbox360" => Platform::Xbox,
        other => panic!("platform dir {other}"),
    }
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
                    platform.file_name().to_string_lossy().into_owned(),
                    game.file_name().to_string_lossy().into_owned(),
                    dbs.join(db),
                    dbs.join(meta),
                ));
            }
        }
    }
    assert_eq!(out.len(), 27);
    out
}

fn open(platform: Platform, db: &Path, meta: &Path) -> EditSession {
    EditSession::begin(
        Project::load(
            db.to_path_buf(),
            platform,
            ProjectRole::Nhlng,
            meta.to_path_buf(),
            repo("defs"),
        )
        .unwrap(),
    )
}

/// A real `dSvy` rating field's studio id on this title.
fn companion_rating_field(sess: &EditSession) -> Option<String> {
    let bytes = sess.bytes().to_vec();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = sess.model().unwrap();
    let rec = model.players.iter().find_map(|p| p.skater_ai)?;
    let fields = entity_fields(
        &db,
        sess.project().meta(),
        sess.project().enums(),
        "dSvy",
        rec,
    );
    build_player_attributes(&fields, "dSvy", false)
        .flat
        .iter()
        .find_map(|a| {
            a.field
                .as_ref()
                .filter(|_| a.is_rating)
                .map(|f| f.studio_field.clone())
        })
}

#[test]
fn a_synthetic_person_imports_reads_back_saves_and_reopens_on_every_anchor() {
    let tmp = Tmp::new();
    let mut table: BTreeMap<String, String> = BTreeMap::new();
    for (pdir, game, db_path, meta_path) in anchors() {
        let label = format!("{pdir}/{game}");
        let platform = platform_of(&pdir);
        let mut sess = open(platform, &db_path, &meta_path);
        let before_errors = sess.validate().unwrap().errors().count();
        let team = sess
            .team_summaries()
            .unwrap()
            .first()
            .expect("a team")
            .team_id;
        let rating_field =
            companion_rating_field(&sess).unwrap_or_else(|| panic!("{label}: a rating field"));

        let person = import::PersonSeason {
            first_name: "Synthetic".into(),
            last_name: "Playerton".into(),
            position: 1,
            height_inches: Some(73),
            weight_lb: Some(195),
            birthdate: Some((1990, 5, 15)),
            jersey: Some(91),
            ratings: vec![(rating_field.clone(), 88)],
            ..Default::default()
        };
        let bytes = sess.bytes().to_vec();
        let db = TdbFile::parse(&bytes).unwrap();
        let (op, skipped) = import::lower_create(
            &db,
            sess.project().meta(),
            sess.project().enums(),
            &person,
            team,
        )
        .unwrap_or_else(|| panic!("{label}: lowering produced no op"));
        drop(db);
        let skipped_note = if skipped.is_empty() {
            String::new()
        } else {
            format!("; skipped {skipped:?}")
        };

        match sess.apply_import(&[op]) {
            Err(e) => {
                // A clean refusal: nothing created, history untouched.
                assert!(
                    sess.player_summaries()
                        .unwrap()
                        .iter()
                        .all(|p| p.last_name != "Playerton"),
                    "{label}: a refused import creates nothing"
                );
                assert!(
                    game == "nhl07",
                    "{label}: only NHL 07 is expected to refuse the create (no slot key): {e}"
                );
                // The refusal reads as words, not as the engine's raw field id.
                let msg = e.to_string();
                assert!(
                    msg.contains("no slot-key column")
                        && msg.contains("not supported here")
                        && !msg.contains("toWX"),
                    "{label}: {msg}"
                );
                table.insert(
                    label,
                    format!(
                        "refused clean — {}{skipped_note}",
                        e.to_string().lines().next().unwrap_or_default()
                    ),
                );
                continue;
            }
            Ok(_) => {}
        }
        let created = sess
            .player_summaries()
            .unwrap()
            .into_iter()
            .find(|p| p.last_name == "Playerton")
            .unwrap_or_else(|| panic!("{label}: created player present"));
        assert_eq!(
            created.team_id,
            Some(team),
            "{label}: on the target team's roster"
        );
        let view = sess.player_view(created.player_id).unwrap().unwrap();
        assert_eq!(view.header.full_name, "Synthetic Playerton", "{label}");
        assert_eq!(
            view.header.height.as_deref(),
            Some("6'1\""),
            "{label}: height"
        );
        assert_eq!(
            view.header.weight.as_deref(),
            Some("195 lb"),
            "{label}: weight through the probed base"
        );
        assert_eq!(
            view.header.birthdate.as_deref(),
            Some("1990-05-15"),
            "{label}: birthdate through the probed index"
        );
        assert_eq!(view.header.jersey_number, Some(91), "{label}");
        let model = sess.model().unwrap();
        let ai = model.player(created.player_id).unwrap().skater_ai.unwrap();
        let bytes = sess.bytes().to_vec();
        let db = TdbFile::parse(&bytes).unwrap();
        let fields = entity_fields(
            &db,
            sess.project().meta(),
            sess.project().enums(),
            "dSvy",
            ai,
        );
        let rating = fields
            .iter()
            .find(|f| f.studio_field == rating_field)
            .unwrap();
        assert_eq!(
            rating.display, "88",
            "{label}: rating through the probed scale ({:?})",
            rating.edit_offset
        );
        let after_errors = sess.validate().unwrap().errors().count();
        assert!(
            after_errors <= before_errors,
            "{label}: the import introduced audit errors ({before_errors} → {after_errors})"
        );

        // Save the working bytes to a disposable copy and find the player again after reopening.
        let out = tmp.0.join(format!("{pdir}-{game}-import.db"));
        std::fs::write(&out, sess.bytes()).unwrap();
        let re = open(platform, &out, &meta_path);
        let again = re
            .player_summaries()
            .unwrap()
            .into_iter()
            .find(|p| p.last_name == "Playerton")
            .unwrap_or_else(|| panic!("{label}: created player survives save and reopen"));
        assert_eq!(again.team_id, Some(team), "{label}");
        assert_eq!(
            re.player_view(again.player_id)
                .unwrap()
                .unwrap()
                .header
                .weight
                .as_deref(),
            Some("195 lb"),
            "{label}"
        );
        table.insert(
            label,
            format!("impl+tested — created, 6'1\" / 195 lb / 1990-05-15 / #91 / rating 88 read back (offset {:?}), audit {before_errors} → {after_errors}, saved, reopened{skipped_note}", rating.edit_offset),
        );
    }
    eprintln!("\n| anchor | historical import (synthetic person) |");
    eprintln!("|---|---|");
    for (l, r) in &table {
        eprintln!("| {l} | {r} |");
    }
    assert_eq!(table.len(), 27);
}
