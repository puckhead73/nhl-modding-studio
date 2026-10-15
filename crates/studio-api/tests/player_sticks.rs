//! Equipment 4.1 (25 Sep 2026, package P4.1-A) — **hockey sticks** on every committed anchor: the
//! skater's equipment companion carries the stick model, curve, flex and tape columns; each is
//! edited through the ordinary validated `set_field` on the companion record, re-read on the
//! Equipment tab's field list, kept through **save → reopen** on a disposable copy, and undone.
//!
//! Per anchor the case records which stick columns the title carries (their widths / ranges and
//! whether the stick model resolves to a catalogue label), then performs the edits. NHL 07 carries
//! no equipment companion at all — recorded, not skipped. Nothing under `anchors/` is written; the
//! per-family table the test prints is the coverage record; nothing here is game evidence.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_api::dto::{FieldViewDto, PlayerViewDto};
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
        let dir = std::env::temp_dir().join(format!("nhlms-sticks-{tag}-{}", std::process::id()));
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

fn equip<'a>(v: &'a PlayerViewDto, name: &str) -> Option<&'a FieldViewDto> {
    v.equipment.iter().find(|f| f.name == name)
}

/// The stick-related columns by metadata name, in the order they are reported.
const STICK_COLUMNS: &[&str] = &[
    "stick",
    "stickcurve",
    "stickflex",
    "sticktapecolor",
    "sticktapestylecode",
    "stickbladetapestyle",
    "stickbladetapecolor",
    "stickshafttapestyle",
    "stickshafttapecolor",
    "stickbladetapetype",
    "stickshafttapetype",
];

/// Pick a new in-range value for a numeric column: another enum option when the field has a
/// vocabulary, else the next raw value (wrapping to the range floor).
fn other_value(f: &FieldViewDto) -> (i64, Option<String>) {
    if f.enum_options.len() >= 2 {
        let o = f
            .enum_options
            .iter()
            .find(|o| Some(o.raw) != f.decoded)
            .unwrap();
        return (o.raw, Some(o.label.clone()));
    }
    let lo = f.range_low.unwrap_or(0);
    let hi = f.range_high.unwrap_or(1);
    let cur = f.decoded.unwrap_or(lo);
    (if cur + 1 <= hi { cur + 1 } else { lo }, None)
}

#[test]
fn stick_model_curve_flex_and_tape_edit_save_reopen_and_undo_on_every_anchor() {
    let tmp = Tmp::new("sweep");
    let mut table: BTreeMap<String, String> = BTreeMap::new();
    for (platform, game, db, meta) in anchors() {
        let label = format!("{platform}/{game}");
        let reg = Registry::default();
        let id = open(&reg, &platform, &db, &meta);
        let before = reg.validate(id).unwrap().error_count;
        let players = reg.list_players(id).unwrap();
        let subject = players
            .iter()
            .find(|p| {
                p.team_id.is_some() && p.position_label != "G" && !p.last_name.trim().is_empty()
            })
            .unwrap_or_else(|| panic!("{label}: a rostered skater"));
        let v = reg.player_view(id, subject.player_id).unwrap().unwrap();

        let Some(rec) = v.equipment_record else {
            // NHL 07 (gen6): no equipment companion table — the tab says so, nothing to edit.
            assert!(v.equipment.is_empty(), "{label}: no companion, no fields");
            assert_eq!(
                game, "nhl07",
                "{label}: only NHL 07 lacks the equipment companion"
            );
            table.insert(
                label,
                "n/a — no equipment companion table on this title (the tab states it)".into(),
            );
            reg.close_project(id);
            continue;
        };
        let equip_table = v.equipment_table.clone();
        let present: Vec<String> = STICK_COLUMNS
            .iter()
            .filter_map(|n| {
                equip(&v, n).map(|f| {
                    format!(
                        "{n}[{}..{}{}]",
                        f.range_low.unwrap_or(0),
                        f.range_high.unwrap_or(0),
                        if f.enum_options.is_empty() {
                            ""
                        } else {
                            ", labelled"
                        }
                    )
                })
            })
            .collect();
        let stick = equip(&v, "stick").unwrap_or_else(|| panic!("{label}: a stick column"));
        assert!(
            equip(&v, "stickcurve").is_some() && equip(&v, "stickflex").is_some(),
            "{label}: curve and flex columns"
        );

        // Edits: stick model, curve, flex, and one tape colour column where the title has one.
        let mut done = Vec::new();
        let mut undo_steps = 0u32;
        for name in [
            "stick",
            "stickcurve",
            "stickflex",
            "sticktapecolor",
            "stickbladetapestyle",
        ] {
            let Some(f) = equip(&v, name).filter(|f| f.editable) else {
                continue;
            };
            let (raw, label_text) = other_value(f);
            reg.set_field(id, &equip_table, rec, &f.studio_field, FieldValue::Int(raw))
                .unwrap_or_else(|e| panic!("{label}: {name}: {e}"));
            undo_steps += 1;
            let after = reg.player_view(id, subject.player_id).unwrap().unwrap();
            let g = equip(&after, name).unwrap();
            assert_eq!(g.decoded, Some(raw), "{label}: {name} re-reads");
            if let Some(t) = &label_text {
                assert_eq!(
                    g.enum_label.as_deref(),
                    Some(t.as_str()),
                    "{label}: {name} label"
                );
            }
            done.push(match label_text {
                Some(t) => format!("{name} → {t}"),
                None => format!("{name} → {raw}"),
            });
        }
        // Out of range refuses by range.
        let over = stick.range_high.unwrap() + 1;
        let err = reg
            .set_field(
                id,
                &equip_table,
                rec,
                &stick.studio_field,
                FieldValue::Int(over),
            )
            .err()
            .unwrap_or_else(|| panic!("{label}: stick {over} must refuse"));
        assert!(err.to_string().contains("out of range"), "{label}: {err}");

        let edited = reg.player_view(id, subject.player_id).unwrap().unwrap();
        let report = reg.validate(id).unwrap();
        assert!(
            report.error_count <= before,
            "{label}: stick edits introduced errors ({before} → {})",
            report.error_count
        );
        let out = tmp.0.join(format!("{platform}-{game}-sticks.db"));
        let re = save_and_reopen(&reg, id, &platform, &meta, &out);
        let reopened = reg.player_view(re, subject.player_id).unwrap().unwrap();
        for name in ["stick", "stickcurve", "stickflex"] {
            assert_eq!(
                equip(&reopened, name).map(|f| f.decoded),
                equip(&edited, name).map(|f| f.decoded),
                "{label}: {name} survives save and reopen"
            );
        }
        reg.close_project(re);
        for _ in 0..undo_steps {
            assert!(reg.undo(id).unwrap(), "{label}: undo");
        }
        let restored = reg.player_view(id, subject.player_id).unwrap().unwrap();
        for name in ["stick", "stickcurve", "stickflex"] {
            assert_eq!(
                equip(&restored, name).map(|f| f.decoded),
                equip(&v, name).map(|f| f.decoded),
                "{label}: undo restores {name}"
            );
        }
        reg.close_project(id);
        table.insert(
            label,
            format!(
                "impl+tested — {} ({} equipment fields); {}; saved, reopened, {undo_steps} undone",
                present.join(" "),
                v.equipment.len(),
                done.join(", ")
            ),
        );
    }
    eprintln!("\n| anchor | sticks |");
    eprintln!("|---|---|");
    for (l, r) in &table {
        eprintln!("| {l} | {r} |");
    }
    assert_eq!(table.len(), 27);
}
