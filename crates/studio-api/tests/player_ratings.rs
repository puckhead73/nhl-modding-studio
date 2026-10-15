//! Players 3.2 (25 Sep 2026, package P3.2-B) — **ratings, style and potential** on every committed
//! main-database anchor (27), for one skater and one goalie each.
//!
//! Per anchor the case reads the Attributes view, pins the rating **scale** the title stores
//! (display = raw + offset; the offset is classified from the field's own metadata and, for the
//! 6-bit titles, probed from the data: **50** on NHL 07–10, **36** on NHL 11–Legacy, **0** on the PS4
//! 0..99 storage), checks every rating cell shows the display-scale number, then edits one rating
//! to a chosen display value, the style to another eligible option and the potential range where
//! the title has one, through the ordinary validated `set_field` on the companion record; the tab
//! re-reads each value, an out-of-range rating refuses by range, validation gains no error,
//! **save → reopen** on a disposable copy shows the same values, and undo restores them.
//!
//! Everything runs in memory and saves to a temp file; nothing under `anchors/` is written. The
//! per-family table the test prints is the coverage record; nothing here is game evidence.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_api::dto::{AttributeViewDto, PlayerViewDto};
use studio_api::{parse_platform, parse_role, Registry};
use studio_domain::FieldValue;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-ratings-{tag}-{}", std::process::id()));
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

/// The 27 main-DB anchors (platform dir, game dir, db, meta), sorted.
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
    assert_eq!(out.len(), 27, "27 main-database anchors are committed");
    out
}

fn open(reg: &Registry, platform: &str, db: &Path, meta: &Path) -> u32 {
    reg.load_project(
        &s(db),
        parse_platform(platform).unwrap(),
        parse_role("nhlng").unwrap(),
        &s(meta),
        &s(&repo("defs")),
    )
    .unwrap_or_else(|e| panic!("{}: load: {e}", db.display()))
    .project_id
}

fn save_and_reopen(reg: &Registry, id: u32, platform: &str, meta: &Path, out: &Path) -> u32 {
    let _ = std::fs::remove_file(out);
    reg.save(id, &s(out))
        .unwrap_or_else(|e| panic!("save {}: {e}", out.display()));
    open(reg, platform, out, meta)
}

/// The rating scale each family stores (display = raw + offset).
fn expected_offset(game: &str, platform: &str) -> i64 {
    if platform == "ps4" {
        0
    } else if matches!(game, "nhl07" | "nhl08" | "nhl09" | "nhl10") {
        50
    } else {
        36
    }
}

fn rating_cells(v: &PlayerViewDto) -> Vec<&AttributeViewDto> {
    v.attributes
        .flat
        .iter()
        .filter(|c| c.is_rating && c.field.is_some())
        .collect()
}

fn cell_display<'a>(v: &'a PlayerViewDto, label: &str) -> Option<&'a str> {
    v.attributes
        .flat
        .iter()
        .find(|c| c.label == label)
        .and_then(|c| c.field.as_ref())
        .map(|f| f.display.as_str())
}

#[derive(Default, Debug)]
struct Row {
    scale: String,
    skater: String,
    goalie: String,
}

/// One subject (skater or goalie): pin the scale, edit a rating / style / potential, save, reopen, undo.
fn exercise(
    reg: &Registry,
    id: u32,
    label: &str,
    player_id: u64,
    offset: i64,
    tmp: &Path,
    platform: &str,
    game: &str,
    meta: &Path,
    before_errors: usize,
) -> String {
    let v = reg.player_view(id, player_id).unwrap().unwrap();
    let cells = rating_cells(&v);
    assert!(
        cells.len() >= 10,
        "{label} {}: {} rating cells labelled",
        v.kind,
        cells.len()
    );
    let table = v.ratings_table.clone();
    let rec = v
        .ratings_record
        .unwrap_or_else(|| panic!("{label}: ratings record"));
    // Every rating cell: the stamped offset is the family's, the display is raw + offset, on 1..99.
    for c in &cells {
        let f = c.field.as_ref().unwrap();
        assert_eq!(
            f.edit_offset.unwrap_or(0),
            offset,
            "{label}: {} edit offset",
            c.label
        );
        let shown: i64 = f
            .display
            .parse()
            .unwrap_or_else(|_| panic!("{label}: {} display {:?}", c.label, f.display));
        assert_eq!(
            shown,
            f.decoded.unwrap() + offset,
            "{label}: {} display = raw + offset",
            c.label
        );
        // The six-bit scales cannot go below their offset; the PS4 0..99 storage can hold 0 (an
        // unauthored rating — PS4 NHL 16 ships a skater with Backskating 0).
        let floor = if offset == 0 { 0 } else { offset };
        assert!(
            (floor..=99).contains(&shown),
            "{label}: {} shows {shown}",
            c.label
        );
    }
    let mut done = Vec::new();
    let mut undo_steps = 0u32;

    // One rating → display 88 (or 87 when it already is 88); 100 refuses by range.
    let first = cells[0];
    let f = first.field.as_ref().unwrap();
    let current: i64 = f.display.parse().unwrap();
    let target = if current == 88 { 87 } else { 88 };
    let over = f.range_high.unwrap() + 1;
    let err = reg
        .set_field(id, &table, rec, &f.studio_field, FieldValue::Int(over))
        .err()
        .unwrap_or_else(|| panic!("{label}: raw {over} must refuse"));
    assert!(err.to_string().contains("out of range"), "{label}: {err}");
    reg.set_field(
        id,
        &table,
        rec,
        &f.studio_field,
        FieldValue::Int(target - offset),
    )
    .unwrap_or_else(|e| panic!("{label}: {}: {e}", first.label));
    undo_steps += 1;
    let after = reg.player_view(id, player_id).unwrap().unwrap();
    assert_eq!(
        cell_display(&after, &first.label),
        Some(target.to_string().as_str()),
        "{label}: {} re-reads",
        first.label
    );
    // On the calibrated era the computed Overall follows the edit; elsewhere it stays absent.
    if v.attributes.computed_overall.is_some() {
        assert!(
            after.attributes.computed_overall.is_some(),
            "{label}: computed overall stays available"
        );
    } else {
        assert!(
            after.attributes.computed_overall.is_none(),
            "{label}: no derived overall outside the calibrated era"
        );
    }
    done.push(format!("{} → {target}", first.label));

    // Style: another eligible option.
    if let Some(style) = v
        .attributes
        .style
        .as_ref()
        .and_then(|a| a.field.as_ref())
        .filter(|f| f.editable && f.enum_options.len() >= 2)
    {
        let other = style
            .enum_options
            .iter()
            .find(|o| Some(o.raw) != style.decoded)
            .unwrap();
        reg.set_field(
            id,
            &table,
            rec,
            &style.studio_field,
            FieldValue::Int(other.raw),
        )
        .unwrap_or_else(|e| panic!("{label}: style: {e}"));
        undo_steps += 1;
        let after = reg.player_view(id, player_id).unwrap().unwrap();
        assert_eq!(
            after
                .attributes
                .style
                .as_ref()
                .and_then(|a| a.field.as_ref())
                .and_then(|f| f.enum_label.as_deref()),
            Some(other.label.as_str()),
            "{label}: style re-reads"
        );
        assert_eq!(
            reg.list_players(id)
                .unwrap()
                .iter()
                .find(|p| p.player_id == player_id)
                .unwrap()
                .style_label
                .as_deref(),
            Some(other.label.as_str()),
            "{label}: the list's archetype follows"
        );
        done.push(format!("style → {}", other.label));
    } else {
        done.push("style: no editable vocabulary".into());
    }

    // Potential range where the title has one.
    if let Some(pot) = v
        .attributes
        .potential_range
        .as_ref()
        .and_then(|a| a.field.as_ref())
        .filter(|f| f.editable && f.enum_options.len() >= 2)
    {
        let other = pot
            .enum_options
            .iter()
            .find(|o| Some(o.raw) != pot.decoded && o.raw != 0)
            .unwrap();
        reg.set_field(
            id,
            &table,
            rec,
            &pot.studio_field,
            FieldValue::Int(other.raw),
        )
        .unwrap_or_else(|e| panic!("{label}: potential: {e}"));
        undo_steps += 1;
        let after = reg.player_view(id, player_id).unwrap().unwrap();
        assert_eq!(
            after
                .attributes
                .potential_range
                .as_ref()
                .and_then(|a| a.field.as_ref())
                .and_then(|f| f.enum_label.as_deref()),
            Some(other.label.as_str()),
            "{label}: potential re-reads"
        );
        done.push(format!("potential → {}", other.label));
    } else {
        done.push("potential: absent".into());
    }

    // Validate, save → reopen, undo.
    let edited = reg.player_view(id, player_id).unwrap().unwrap();
    let report = reg.validate(id).unwrap();
    assert!(
        report.error_count <= before_errors,
        "{label}: rating edits introduced errors ({before_errors} → {})",
        report.error_count
    );
    let out = tmp.join(format!("{platform}-{game}-{}-ratings.db", edited.kind));
    let re = save_and_reopen(reg, id, platform, meta, &out);
    let reopened = reg.player_view(re, player_id).unwrap().unwrap();
    assert_eq!(
        cell_display(&reopened, &first.label),
        cell_display(&edited, &first.label),
        "{label}: the rating survives save and reopen"
    );
    assert_eq!(
        reopened
            .attributes
            .style
            .as_ref()
            .and_then(|a| a.field.as_ref())
            .and_then(|f| f.enum_label.clone()),
        edited
            .attributes
            .style
            .as_ref()
            .and_then(|a| a.field.as_ref())
            .and_then(|f| f.enum_label.clone()),
        "{label}: style survives"
    );
    assert_eq!(
        reopened.attributes.computed_overall, edited.attributes.computed_overall,
        "{label}: computed overall survives"
    );
    reg.close_project(re);
    for _ in 0..undo_steps {
        assert!(reg.undo(id).unwrap(), "{label}: undo");
    }
    let restored = reg.player_view(id, player_id).unwrap().unwrap();
    assert_eq!(
        cell_display(&restored, &first.label),
        Some(current.to_string().as_str()),
        "{label}: undo restores the rating"
    );
    assert_eq!(
        restored
            .attributes
            .style
            .as_ref()
            .and_then(|a| a.field.as_ref())
            .and_then(|f| f.decoded),
        v.attributes
            .style
            .as_ref()
            .and_then(|a| a.field.as_ref())
            .and_then(|f| f.decoded),
        "{label}: undo restores the style"
    );
    format!(
        "impl+tested — {} cells; {}; saved, reopened, {undo_steps} undone",
        cells.len(),
        done.join(", ")
    )
}

#[test]
fn ratings_style_and_potential_edit_save_reopen_and_undo_on_every_anchor() {
    let tmp = Tmp::new("sweep");
    let mut table: BTreeMap<String, Row> = BTreeMap::new();
    for (platform, game, db, meta) in anchors() {
        let label = format!("{platform}/{game}");
        let reg = Registry::default();
        let id = open(&reg, &platform, &db, &meta);
        let before = reg.validate(id).unwrap().error_count;
        let offset = expected_offset(&game, &platform);
        let players = reg.list_players(id).unwrap();
        let skater = players
            .iter()
            .find(|p| {
                p.team_id.is_some() && p.position_label != "G" && !p.last_name.trim().is_empty()
            })
            .unwrap_or_else(|| panic!("{label}: a rostered skater"));
        let goalie = players
            .iter()
            .find(|p| {
                p.team_id.is_some() && p.position_label == "G" && !p.last_name.trim().is_empty()
            })
            .unwrap_or_else(|| panic!("{label}: a rostered goalie"));
        let mut row = Row {
            scale: match offset {
                0 => "stored 0..99 (metadata 0..99)".into(),
                o => format!("raw + {o} → {o}..99 (6-bit, probed)"),
            },
            ..Default::default()
        };
        row.skater = exercise(
            &reg,
            id,
            &format!("{label} skater"),
            skater.player_id,
            offset,
            &tmp.0,
            &platform,
            &game,
            &meta,
            before,
        );
        row.goalie = exercise(
            &reg,
            id,
            &format!("{label} goalie"),
            goalie.player_id,
            offset,
            &tmp.0,
            &platform,
            &game,
            &meta,
            before,
        );
        reg.close_project(id);
        table.insert(label, row);
    }
    eprintln!("\n| anchor | rating scale | skater | goalie |");
    eprintln!("|---|---|---|---|");
    for (label, r) in &table {
        eprintln!("| {label} | {} | {} | {} |", r.scale, r.skater, r.goalie);
    }
    assert_eq!(table.len(), 27);
}
