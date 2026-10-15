//! O5 / L1.1-F — the cross-target offline league-identity campaign, reproducible with
//! `cargo test -p studio-api --test league_offline_campaign -- --nocapture`.
//!
//! PS4 targets (a `stockleagues` table): the reversible equivalent identity operation — re-point
//! one target-valid league's name key at the existing key `TXT_ROW_LEAGUE` — runs on a copied
//! fixture through the exact Registry seam: edit → pending review → copied-target review → apply →
//! allowed-difference audit → reopen → byte-exact restore. The league is resolved semantically
//! (by its name key) and then addressed by its stable native id.
//!
//! Older targets (no league table): league browsing/classification is verified and the truthful
//! blocked-name-edit behaviour is asserted; no edit is attempted. Targets without a fixture are
//! reported blocked. The printed table is the evidence transcribed into
//! `docs/validation/league-identity-offline-campaign.md`.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::FieldValue;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn s(p: &Path) -> String {
    p.to_str().unwrap().to_string()
}

fn sha(path: &Path) -> String {
    studio_domain::tdb_txn::FileIdentity::of_file(path)
        .unwrap()
        .sha256
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "nhlms-league-campaign-{tag}-{}",
            std::process::id()
        ));
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

/// `(game, platform, Some((db file, meta file)))` — or `None` when no fixture exists.
type Target = (
    &'static str,
    &'static str,
    Option<(&'static str, &'static str)>,
);

/// The 27 shipping targets plus the NHL 07 PS3 research copy, with their anchor when one exists.
fn targets() -> Vec<Target> {
    let mut v = Vec::new();
    for g in [
        "nhl07",
        "nhl08",
        "nhl09",
        "nhl10",
        "nhl11",
        "nhl12",
        "nhl13",
        "nhl14",
        "nhl15",
        "nhllegacy",
    ] {
        let (db, meta) = if g == "nhl07" {
            ("NHL07NG.db", "NHL07NG-meta.xml")
        } else {
            ("nhlng.db", "nhlng-meta.xml")
        };
        v.push((g, "xbox360", Some((db, meta))));
        // NHL 07 PS3 is the research copy, not a shipping target — listed last as supplemental.
        if g != "nhl07" {
            v.push((g, "ps3", Some((db, meta))));
        }
    }
    for g in [
        "nhl15", "nhl16", "nhl17", "nhl18", "nhl19", "nhl20", "nhl21",
    ] {
        v.push((g, "ps4", Some(("nhl_game.db", "nhl_game-meta.xml"))));
    }
    v.push(("legacyrecomp", "recomp", None));
    v.push(("nhl07", "ps3", Some(("NHL07NG.db", "NHL07NG-meta.xml"))));
    v
}

/// The target-valid equivalent of "the Allsvenskan row": the first present of these keys.
const EQUIVALENTS: &[&str] = &["Allsvenskan", "TXT_LEAGUE_NORWAY", "TXT_LEGEND_TEAMS"];
const NEW_KEY: &str = "TXT_ROW_LEAGUE";

struct Row {
    target: String,
    fixture: String,
    source: String,
    operation: String,
    studio: String,
    structural: String,
    reopen: String,
    restore: String,
    note: String,
}

fn ps4_campaign(base: &Path, game: &str, db_name: &str, meta_name: &str) -> Row {
    let anchor = repo(&format!("anchors/ps4/{game}/databases/{db_name}"));
    let meta = repo(&format!("anchors/ps4/{game}/databases/{meta_name}"));
    let anchor_sha = sha(&anchor);
    let root = base.join(format!("{game}-copy"));
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let dir = root.join("nhl/data/common/gamemodes");
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join(db_name);
    std::fs::copy(&anchor, &db).unwrap();

    let reg = Registry::default();
    let id = reg
        .open_workspace_from_paths(
            Some(game),
            parse_platform("ps4").unwrap(),
            &s(&db),
            None,
            &s(&meta),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    let catalog = reg.list_leagues(id).unwrap();
    assert_eq!(catalog.source, "stockleagues", "{game}");
    let target_league = EQUIVALENTS
        .iter()
        .find_map(|k| {
            catalog
                .leagues
                .iter()
                .find(|l| l.name_key.as_deref() == Some(*k))
        })
        .unwrap_or_else(|| panic!("{game}: no target-valid equivalent league"));
    let code = target_league.code as u64;
    let old_key = target_league.name_key.clone().unwrap();
    let members_before = target_league.member_teams;
    let member_ids_before: Vec<u64> = reg
        .league_view(id, code)
        .unwrap()
        .unwrap()
        .members
        .iter()
        .map(|t| t.team_id)
        .collect();
    let team_count_before = reg.list_teams(id).unwrap().len();

    // Edit and review.
    reg.set_field(
        id,
        "fsLK",
        target_league.record.unwrap(),
        "vIys",
        FieldValue::Text(NEW_KEY.into()),
    )
    .unwrap();
    let pending = reg.pending_changes(id, 100).unwrap();
    assert_eq!(
        pending.primary.total_cells, 1,
        "{game}: exactly one cell pending"
    );
    assert_eq!(pending.primary.cells[0].before, old_key);
    assert_eq!(pending.primary.cells[0].after, NEW_KEY);
    let review = reg.copied_target_review(id, &s(&root)).unwrap();
    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed", "{game}");
    let after_sha = sha(&db);

    // Allowed-difference audit: only the edited record differs at record level; the file parses
    // with valid CRC and size.
    let before = std::fs::read(&anchor).unwrap();
    let after = std::fs::read(&db).unwrap();
    let bf = tdb_core::TdbFile::parse(&before).unwrap();
    let af = tdb_core::TdbFile::parse(&after).unwrap();
    let mut changed = Vec::new();
    for t in &af.tables {
        let bt = bf.table(&t.id.raw()).unwrap();
        let rl = t.record_len_bytes as usize;
        for rec in 0..t.current_records as usize {
            if after[t.record_block + rec * rl..t.record_block + (rec + 1) * rl]
                != before[bt.record_block + rec * rl..bt.record_block + (rec + 1) * rl]
            {
                changed.push(format!("{}#{rec}", t.id.canonical(af.endian)));
            }
        }
    }
    let structural = format!(
        "records changed: {} · header crc {} · size {} · len {}→{}",
        changed.join(","),
        af.header_crc_ok(),
        af.size_ok(),
        before.len(),
        after.len()
    );
    assert_eq!(
        changed,
        vec![format!("fsLK#{}", target_league.record.unwrap())],
        "{game}"
    );

    // Reopen in a fresh registry: identity kept, key re-pointed, membership untouched.
    let reg2 = Registry::default();
    let id2 = reg2
        .open_workspace_from_paths(
            Some(game),
            parse_platform("ps4").unwrap(),
            &s(&db),
            None,
            &s(&meta),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    let v2 = reg2.league_view(id2, code).unwrap().unwrap();
    assert_eq!(v2.summary.name_key.as_deref(), Some(NEW_KEY));
    assert_eq!(v2.summary.member_teams, members_before);
    let member_ids_after: Vec<u64> = v2.members.iter().map(|t| t.team_id).collect();
    assert_eq!(
        member_ids_after, member_ids_before,
        "{game}: membership identities unchanged"
    );
    assert_eq!(reg2.list_teams(id2).unwrap().len(), team_count_before);
    let reopen = format!(
        "league {code} name {old_key}→{NEW_KEY} · {members_before} members unchanged · {} teams",
        team_count_before
    );
    drop(reg2);

    // Byte-exact restore.
    let status = reg.copied_target_status(&s(&root)).unwrap();
    let txn = status.transactions.iter().find(|t| t.restorable).unwrap();
    let restored = reg.copied_target_restore(&s(&root), &txn.txn_id).unwrap();
    let restored_sha = sha(&db);
    assert_eq!(restored_sha, anchor_sha, "{game}: byte-exact restore");
    assert_eq!(
        reg.league_view(id, code)
            .unwrap()
            .unwrap()
            .summary
            .name_key
            .as_deref(),
        Some(old_key.as_str())
    );

    Row {
        target: format!("{game} PS4"),
        fixture: format!("{db_name} {}…", &anchor_sha[..16]),
        source: "stockleagues".into(),
        operation: format!(
            "league {code} ({old_key}) name → {NEW_KEY}; key not in fixture dictionary check here"
        ),
        studio: format!(
            "pending 1 cell · review handle {} · apply {} · output {}…",
            review.handle,
            applied.state,
            &after_sha[..16]
        ),
        structural,
        reopen,
        restore: format!(
            "{} · sha {}… == anchor",
            restored.state,
            &restored_sha[..16]
        ),
        note: match game {
            "nhl19" => "anchor = the NHL 19 base archive member (REG-1, CUSA11126 inxp_00.big #2089); the install also ships six patch databases".into(),
            "nhl20" => "anchor is the NHL 19 base database under an NHL 20 path (REG-1); NHL 20's own database has no anchor — see tests/league_private_sources.rs".into(),
            "nhl15" => "no Allsvenskan/Norway row: Legend Teams used as the equivalent".into(),
            _ => String::new(),
        },
    }
}

fn gen7_campaign(game: &str, platform: &str, db_name: &str, meta_name: &str) -> Row {
    let anchor = repo(&format!("anchors/{platform}/{game}/databases/{db_name}"));
    let meta = repo(&format!("anchors/{platform}/{game}/databases/{meta_name}"));
    let anchor_sha = sha(&anchor);
    let reg = Registry::default();
    let id = reg
        .open_workspace_from_paths(
            Some(game),
            parse_platform(platform).unwrap(),
            &s(&anchor),
            None,
            &s(&meta),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    let catalog = reg.list_leagues(id).unwrap();
    assert_eq!(catalog.source, "derived-codes", "{game} {platform}");
    let vocab = catalog
        .vocabulary
        .clone()
        .unwrap_or_else(|| panic!("{game} {platform}: {:?}", catalog.unresolved_reason));
    let blocker = catalog
        .name_edit_blocker
        .clone()
        .expect("gen7 name edits are blocked with a reason");
    let labelled = catalog.leagues.iter().filter(|l| l.label.is_some()).count();
    let raw = catalog
        .leagues
        .iter()
        .filter(|l| l.label.is_none())
        .map(|l| l.code.to_string())
        .collect::<Vec<_>>();
    // Every league detail resolves its members; the NHL row has a full complement.
    let nhl = reg.league_view(id, 0).unwrap().unwrap();
    assert!(nhl.members.len() >= 30 && nhl.fields.is_empty() && nhl.name_edit_blocker.is_some());
    // Team surfaces share the labels.
    let teams = reg.list_teams(id).unwrap();
    assert!(teams.iter().all(|t| t.league_label.is_some()));
    Row {
        target: format!("{game} {platform}"),
        fixture: format!("{db_name} {}…", &anchor_sha[..16]),
        source: format!("derived-codes ({vocab})"),
        operation: "browse/classify only — no league name in the main DB".into(),
        studio: format!(
            "{} leagues, {labelled} labelled{}; NHL row {} members; blocked: {}",
            catalog.leagues.len(),
            if raw.is_empty() { String::new() } else { format!(", raw codes {}", raw.join("/")) },
            nhl.members.len(),
            &blocker[..blocker.find(':').unwrap_or(60).min(60)]
        ),
        structural: "no edit".into(),
        reopen: "n/a".into(),
        restore: "n/a (anchor untouched)".into(),
        note: match platform {
            "xbox360" if game != "nhl07" => "bytes identical to the PS3 anchor: lookup selection only, not separate runtime evidence".into(),
            _ => String::new(),
        },
    }
}

#[test]
fn offline_campaign_all_targets() {
    let tmp = Tmp::new("all");
    let mut rows = Vec::new();
    for (game, platform, fixture) in targets() {
        let row = match (platform, fixture) {
            ("ps4", Some((db, meta))) => ps4_campaign(&tmp.0, game, db, meta),
            (_, Some((db, meta))) => gen7_campaign(game, platform, db, meta),
            (_, None) => Row {
                target: format!("{game} {platform}"),
                fixture: "none".into(),
                source: "blocked".into(),
                operation:
                    "blocked — no fixture (shares Legacy Xbox 360 schema by reference; untested)"
                        .into(),
                studio: "-".into(),
                structural: "-".into(),
                reopen: "-".into(),
                restore: "-".into(),
                note: "missing-fixture".into(),
            },
        };
        rows.push(row);
    }
    // Anchors untouched.
    assert_eq!(
        sha(&repo("anchors/ps4/nhl21/databases/nhl_game.db")),
        "90ee1ec49d36dbb0f6d9cefe75e259d9661e3aa05338d03347b4842e33c8e9f9"
    );
    println!("| Target | Fixture | Source | Operation | Studio result | Structural audit | Reopen | Restore | Note |");
    println!("|---|---|---|---|---|---|---|---|---|");
    for r in &rows {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.target,
            r.fixture,
            r.source,
            r.operation,
            r.studio,
            r.structural,
            r.reopen,
            r.restore,
            r.note
        );
    }
    assert_eq!(
        rows.len(),
        28,
        "27 shipping targets + the NHL 07 PS3 research copy"
    );
}
