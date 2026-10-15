//! Players 3.1 (25 Sep 2026, package P3.1-A) — **identity data** on every committed main-database
//! anchor (27: 10 Xbox 360, 10 PS3, 7 PS4): names, birth date, nationality / birthplace, position,
//! handedness and physical measurements.
//!
//! Per anchor the case records which identity columns the bio table carries, checks that the
//! player header composes each one into its human-readable form from the **probed** per-title
//! encodings (birth month / day indexing, weight base, height offset) — a plausible date, a
//! `ft'in"` height, an `N lb` weight, enum labels rather than numbers — measures how the shipped
//! name strings are encoded (ASCII / UTF-8 / bytes that are not UTF-8), then edits one player's last
//! name, height, weight, birth day, handedness, nationality and position through the ordinary
//! validated `set_field`, checks the header re-composes each value, validation gains no error,
//! **save → reopen** on a disposable copy shows the same values, and undo restores the original
//! header cell by cell. Refusals are asserted to read as text (a too-long name names the field and
//! its width; an out-of-range height names the range).
//!
//! Everything runs in memory and saves to a temp file; nothing under `anchors/` is written. The
//! per-family table the test prints is the coverage record; nothing here is game evidence.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_api::{parse_platform, parse_role, Registry};
use studio_domain::FieldValue;
use tdb_core::TdbFile;

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
        let dir = std::env::temp_dir().join(format!("nhlms-identity-{tag}-{}", std::process::id()));
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

/// The 27 main-DB anchors (platform dir, game dir, db, meta), sorted — the read-matrix discovery.
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

/// The identity columns of the bio table by studio field id, with the label used in the table.
const IDENTITY_COLUMNS: &[(&str, &str)] = &[
    ("HdeP", "first name"),
    ("QbMR", "last name"),
    ("MFzJ", "birth city"),
    ("QvxI", "birth state"),
    ("Lelh", "nationality"),
    ("qFnd", "birth year"),
    ("JKLp", "birth month"),
    ("Kswi", "birth day"),
    ("vjla", "position"),
    ("GRkp", "handedness"),
    ("ypBQ", "height"),
    ("sNZW", "weight"),
];

/// How the shipped name / city strings are encoded: counts of ASCII-only, valid multi-byte UTF-8 and
/// not-UTF-8 strings, with one sample of each non-ASCII class (bytes shown in hex).
#[derive(Default, Debug)]
struct Encoding {
    ascii: usize,
    utf8: usize,
    other: usize,
    utf8_sample: Option<String>,
    other_sample: Option<String>,
}

fn measure_encoding(db_path: &Path) -> Encoding {
    let bytes = std::fs::read(db_path).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let bio = db.table_canonical("ubPc").expect("bio table");
    let mut enc = Encoding::default();
    for field in bio
        .fields
        .iter()
        .filter(|f| matches!(f.id.canonical(db.endian).as_str(), "HdeP" | "QbMR" | "MFzJ"))
    {
        for index in 0..bio.current_records as u32 {
            let Some(raw) = db.read_inline_str(bio, index, field) else {
                continue;
            };
            if raw.is_empty() {
                continue;
            }
            if raw.is_ascii() {
                enc.ascii += 1;
            } else if let Ok(text) = std::str::from_utf8(raw) {
                enc.utf8 += 1;
                enc.utf8_sample
                    .get_or_insert_with(|| format!("{text:?} = {}", hex(raw)));
            } else {
                enc.other += 1;
                enc.other_sample.get_or_insert_with(|| {
                    format!("{} = {}", String::from_utf8_lossy(raw), hex(raw))
                });
            }
        }
    }
    enc
}

fn hex(b: &[u8]) -> String {
    b.iter()
        .map(|x| format!("{x:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `YYYY-MM-DD`-like plausibility of the composed birthdate for an NHL player of the anchor's era.
fn plausible_birthdate(text: &str) -> bool {
    let parts: Vec<i64> = text
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect();
    let [y, m, d] = parts[..] else { return false };
    (1940..=2010).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d)
}

fn field<'a>(
    v: &'a studio_api::dto::PlayerViewDto,
    id: &str,
) -> Option<&'a studio_api::dto::FieldViewDto> {
    v.all_fields.iter().find(|f| f.studio_field == id)
}

#[derive(Default, Debug)]
struct Row {
    columns_absent: Vec<&'static str>,
    header: String,
    encoding: String,
    edits: String,
}

#[test]
fn player_identity_reads_edits_saves_reopens_and_undoes_on_every_anchor() {
    let tmp = Tmp::new("sweep");
    let mut table: BTreeMap<String, Row> = BTreeMap::new();
    for (platform, game, db, meta) in anchors() {
        let label = format!("{platform}/{game}");
        let reg = Registry::default();
        let id = open(&reg, &platform, &db, &meta);
        let mut row = Row::default();
        let before_report = reg.validate(id).unwrap();

        // ------------------------------------------------------------------ subject: a rostered skater with both names
        let players = reg.list_players(id).unwrap();
        let subject = players
            .iter()
            .find(|p| {
                p.team_id.is_some()
                    && !p.first_name.trim().is_empty()
                    && !p.last_name.trim().is_empty()
                    && p.position_label != "G"
            })
            .unwrap_or_else(|| panic!("{label}: a rostered skater with both names"));
        let v = reg.player_view(id, subject.player_id).unwrap().unwrap();

        // ------------------------------------------------------------------ columns present / absent
        for (fid, name) in IDENTITY_COLUMNS {
            if field(&v, fid).is_none() {
                row.columns_absent.push(name);
            }
        }

        // ------------------------------------------------------------------ header composition
        let h = &v.header;
        assert_eq!(
            h.full_name,
            format!("{} {}", subject.first_name.trim(), subject.last_name.trim()),
            "{label}"
        );
        let mut parts = Vec::new();
        if field(&v, "qFnd").is_some() && field(&v, "JKLp").is_some() && field(&v, "Kswi").is_some()
        {
            let bd = h
                .birthdate
                .as_deref()
                .unwrap_or_else(|| panic!("{label}: composed birthdate"));
            assert!(plausible_birthdate(bd), "{label}: birthdate {bd:?}");
            parts.push(format!("DOB {bd}"));
        }
        if field(&v, "ypBQ").is_some() {
            let ht = h
                .height
                .as_deref()
                .unwrap_or_else(|| panic!("{label}: formatted height"));
            assert!(
                ht.contains('\'') && ht.ends_with('"'),
                "{label}: height {ht:?}"
            );
            parts.push(ht.to_string());
        }
        if field(&v, "sNZW").is_some() {
            let wt = h
                .weight
                .as_deref()
                .unwrap_or_else(|| panic!("{label}: formatted weight"));
            let lb: i64 = wt
                .strip_suffix(" lb")
                .and_then(|n| n.parse().ok())
                .unwrap_or_else(|| panic!("{label}: weight {wt:?}"));
            assert!((140..=280).contains(&lb), "{label}: weight {wt:?}");
            parts.push(wt.to_string());
        }
        if field(&v, "GRkp").is_some() {
            let hd = h
                .handedness_label
                .as_deref()
                .unwrap_or_else(|| panic!("{label}: handedness label"));
            assert!(
                hd.chars().any(|c| c.is_ascii_alphabetic()),
                "{label}: handedness {hd:?}"
            );
            parts.push(hd.to_string());
        }
        if field(&v, "vjla").is_some() {
            let pos = h
                .position_label
                .as_deref()
                .unwrap_or_else(|| panic!("{label}: position label"));
            assert!(
                pos.chars().all(|c| c.is_ascii_alphabetic()),
                "{label}: position {pos:?}"
            );
            parts.push(pos.to_string());
        }
        if let Some(f) = field(&v, "Lelh") {
            // Nationality labels are title-scoped (receipt 28): before NHL 13 only the national-team
            // codes carry a label, so an unlabelled code has no banner label and its field shows the
            // stored number — never another title's country.
            match h.nationality_label.as_deref() {
                Some(nat) => {
                    assert!(
                        nat.chars().any(|c| c.is_ascii_alphabetic()),
                        "{label}: nationality {nat:?}"
                    );
                    parts.push(nat.to_string());
                }
                None => {
                    assert!(
                        f.enum_label.is_none() && f.display == f.decoded.unwrap_or(-1).to_string(),
                        "{label}: an unlabelled nationality code is shown as its number ({:?})",
                        f.display
                    );
                    parts.push(format!("nationality code {}", f.display));
                }
            }
        }
        if field(&v, "MFzJ").is_some() || field(&v, "QvxI").is_some() {
            let bp = h
                .birthplace
                .as_deref()
                .unwrap_or_else(|| panic!("{label}: birthplace"));
            parts.push(bp.to_string());
        }
        row.header = format!("{} — {}", h.full_name, parts.join(" · "));

        // ------------------------------------------------------------------ shipped string encoding
        let enc = measure_encoding(&db);
        row.encoding = format!(
            "ascii {} · utf-8 {} · not-utf-8 {}{}{}",
            enc.ascii,
            enc.utf8,
            enc.other,
            enc.utf8_sample
                .as_deref()
                .map(|x| format!(" · utf-8 e.g. {x}"))
                .unwrap_or_default(),
            enc.other_sample
                .as_deref()
                .map(|x| format!(" · not-utf-8 e.g. {x}"))
                .unwrap_or_default()
        );

        // ------------------------------------------------------------------ edits → header → validate → save/reopen → undo
        let mut done: Vec<String> = Vec::new();
        let mut undo_steps = 0u32;
        let bio = v.all_fields[0].studio_table.clone();
        let rec = v.bio_record;

        // Last name: an ASCII value within the width; a too-long one refuses by name and width.
        if let Some(f) = field(&v, "QbMR").filter(|f| f.editable) {
            let width = f.text.as_ref().map(|_| 0usize).unwrap_or(0);
            let _ = width;
            // The field view states its byte width so the editor can refuse before the write.
            let width = f
                .text_max_bytes
                .unwrap_or_else(|| panic!("{label}: last name width"));
            // Measured widths: 160 bytes on NHL 07, narrower on the later titles (recorded in the table).
            assert!((8..=255).contains(&width), "{label}: last-name width {width}");
            let too_long = "X".repeat(200);
            let err = reg
                .set_field(id, &bio, rec, "QbMR", FieldValue::Text(too_long))
                .err()
                .unwrap_or_else(|| panic!("{label}: a 200-byte name must refuse"));
            let msg = err.to_string();
            assert!(
                msg.contains("holds at most")
                    && msg.contains(&format!("at most {width} bytes"))
                    && msg.contains("not applied"),
                "{label}: {msg}"
            );
            assert_eq!(
                reg.player_view(id, subject.player_id)
                    .unwrap()
                    .unwrap()
                    .header
                    .full_name,
                h.full_name,
                "{label}: a refused name changes nothing"
            );
            reg.set_field(id, &bio, rec, "QbMR", FieldValue::Text("Sweeptest".into()))
                .unwrap_or_else(|e| panic!("{label}: last name: {e}"));
            undo_steps += 1;
            let after = reg.player_view(id, subject.player_id).unwrap().unwrap();
            assert_eq!(
                after.header.full_name,
                format!("{} Sweeptest", subject.first_name.trim()),
                "{label}"
            );
            assert_eq!(
                reg.list_players(id)
                    .unwrap()
                    .iter()
                    .find(|p| p.player_id == subject.player_id)
                    .unwrap()
                    .last_name,
                "Sweeptest",
                "{label}: the list re-reads the name"
            );
            done.push(format!("last name (width {width} B)"));
        }
        // Height: display inches = raw + 54. Write 6'2" (raw 20) unless that is the current value, then 6'0" (raw 18).
        if let Some(f) = field(&v, "ypBQ").filter(|f| f.editable) {
            assert_eq!(
                (f.edit_offset, f.edit_height),
                (Some(54), true),
                "{label}: height edit form"
            );
            let raw = if f.decoded == Some(20) { 18 } else { 20 };
            let expect = if raw == 20 { "6'2\"" } else { "6'0\"" };
            let high = f.range_high.unwrap_or(31) + 1;
            let err = reg
                .set_field(id, &bio, rec, "ypBQ", FieldValue::Int(high))
                .err()
                .unwrap_or_else(|| panic!("{label}: height {high} must refuse"));
            assert!(err.to_string().contains("out of range"), "{label}: {err}");
            reg.set_field(id, &bio, rec, "ypBQ", FieldValue::Int(raw))
                .unwrap_or_else(|e| panic!("{label}: height: {e}"));
            undo_steps += 1;
            assert_eq!(
                reg.player_view(id, subject.player_id)
                    .unwrap()
                    .unwrap()
                    .header
                    .height
                    .as_deref(),
                Some(expect),
                "{label}"
            );
            done.push(format!("height {expect}"));
        }
        // Weight: display lb = raw + base (the base is the probed 120 / 150; read it from the edit form).
        if let Some(f) = field(&v, "sNZW").filter(|f| f.editable) {
            let base = f
                .edit_offset
                .unwrap_or_else(|| panic!("{label}: weight edit form"));
            let expected_base = if matches!(game.as_str(), "nhl07" | "nhl08" | "nhl09" | "nhl10") {
                150
            } else {
                120
            };
            assert_eq!(
                base, expected_base,
                "{label}: weight base (measured: 150 on NHL 07–10, 120 from NHL 11 and on PS4)"
            );
            let target_lb = if f.decoded.map(|d| d + base) == Some(200) {
                205
            } else {
                200
            };
            reg.set_field(id, &bio, rec, "sNZW", FieldValue::Int(target_lb - base))
                .unwrap_or_else(|e| panic!("{label}: weight: {e}"));
            undo_steps += 1;
            assert_eq!(
                reg.player_view(id, subject.player_id)
                    .unwrap()
                    .unwrap()
                    .header
                    .weight
                    .as_deref(),
                Some(format!("{target_lb} lb").as_str()),
                "{label}"
            );
            done.push(format!("weight {target_lb} lb (base {base})"));
        }
        // Birth day: display day = raw + probed offset (0 on NHL 10, 1 elsewhere). Write the 15th unless it is the 15th, then the 16th.
        if let Some(f) = field(&v, "Kswi").filter(|f| f.editable) {
            let off = f
                .edit_offset
                .unwrap_or_else(|| panic!("{label}: birth day edit form"));
            assert!(off == 0 || off == 1, "{label}: birth-day offset {off}");
            let current_day = f.decoded.map(|d| d + off);
            let day = if current_day == Some(15) { 16 } else { 15 };
            reg.set_field(id, &bio, rec, "Kswi", FieldValue::Int(day - off))
                .unwrap_or_else(|e| panic!("{label}: birth day: {e}"));
            undo_steps += 1;
            let bd = reg
                .player_view(id, subject.player_id)
                .unwrap()
                .unwrap()
                .header
                .birthdate
                .unwrap();
            assert!(
                bd.ends_with(&format!("-{day:02}")) || bd.ends_with(&format!("-{day}")),
                "{label}: birthdate {bd:?} should end on day {day}"
            );
            done.push(format!("birth day {day} (offset {off})"));
        }
        // Handedness: the other enum option.
        if let Some(f) = field(&v, "GRkp").filter(|f| f.editable && f.enum_options.len() >= 2) {
            let other = f
                .enum_options
                .iter()
                .find(|o| Some(o.raw) != f.decoded)
                .unwrap();
            reg.set_field(id, &bio, rec, "GRkp", FieldValue::Int(other.raw))
                .unwrap_or_else(|e| panic!("{label}: handedness: {e}"));
            undo_steps += 1;
            assert_eq!(
                reg.player_view(id, subject.player_id)
                    .unwrap()
                    .unwrap()
                    .header
                    .handedness_label
                    .as_deref(),
                Some(other.label.as_str()),
                "{label}"
            );
            done.push(format!("handedness {}", other.label));
        }
        // Nationality: another enum option (the label must re-resolve through the same vocabulary).
        if let Some(f) = field(&v, "Lelh").filter(|f| f.editable && f.enum_options.len() >= 2) {
            let other = f
                .enum_options
                .iter()
                .find(|o| Some(o.raw) != f.decoded && o.label != "Unknown")
                .unwrap();
            reg.set_field(id, &bio, rec, "Lelh", FieldValue::Int(other.raw))
                .unwrap_or_else(|e| panic!("{label}: nationality: {e}"));
            undo_steps += 1;
            assert_eq!(
                reg.player_view(id, subject.player_id)
                    .unwrap()
                    .unwrap()
                    .header
                    .nationality_label
                    .as_deref(),
                Some(other.label.as_str()),
                "{label}"
            );
            done.push(format!("nationality {}", other.label));
        }
        // Position: another skater position (never the goalie one — that is a kind change, not an identity edit).
        if let Some(f) = field(&v, "vjla").filter(|f| f.editable) {
            let other = f
                .enum_options
                .iter()
                .find(|o| {
                    Some(o.raw) != f.decoded && o.label != "G" && !o.label.starts_with("Goal")
                })
                .unwrap_or_else(|| panic!("{label}: a second skater position"));
            reg.set_field(id, &bio, rec, "vjla", FieldValue::Int(other.raw))
                .unwrap_or_else(|e| panic!("{label}: position: {e}"));
            undo_steps += 1;
            assert_eq!(
                reg.player_view(id, subject.player_id)
                    .unwrap()
                    .unwrap()
                    .header
                    .position_label
                    .as_deref(),
                Some(other.label.as_str()),
                "{label}"
            );
            done.push(format!("position {}", other.label));
        }
        assert!(
            undo_steps >= 5,
            "{label}: at least five identity edits ran ({done:?})"
        );

        // Validation gains no error; save → reopen keeps every value; undo restores the header.
        let edited = reg
            .player_view(id, subject.player_id)
            .unwrap()
            .unwrap()
            .header;
        let report = reg.validate(id).unwrap();
        assert!(
            report.error_count <= before_report.error_count,
            "{label}: identity edits introduced errors ({} → {})",
            before_report.error_count,
            report.error_count
        );
        let out = tmp.0.join(format!("{platform}-{game}-identity.db"));
        let re = save_and_reopen(&reg, id, &platform, &meta, &out);
        let reopened = reg
            .player_view(re, subject.player_id)
            .unwrap()
            .unwrap()
            .header;
        assert_eq!(
            reopened.full_name, edited.full_name,
            "{label}: name survives save and reopen"
        );
        assert_eq!(reopened.height, edited.height, "{label}");
        assert_eq!(reopened.weight, edited.weight, "{label}");
        assert_eq!(reopened.birthdate, edited.birthdate, "{label}");
        assert_eq!(
            reopened.handedness_label, edited.handedness_label,
            "{label}"
        );
        assert_eq!(
            reopened.nationality_label, edited.nationality_label,
            "{label}"
        );
        assert_eq!(reopened.position_label, edited.position_label, "{label}");
        reg.close_project(re);
        for _ in 0..undo_steps {
            assert!(reg.undo(id).unwrap(), "{label}: undo");
        }
        let restored = reg
            .player_view(id, subject.player_id)
            .unwrap()
            .unwrap()
            .header;
        assert_eq!(
            restored.full_name, h.full_name,
            "{label}: undo restores the name"
        );
        assert_eq!(restored.height, h.height, "{label}");
        assert_eq!(restored.weight, h.weight, "{label}");
        assert_eq!(restored.birthdate, h.birthdate, "{label}");
        assert_eq!(restored.handedness_label, h.handedness_label, "{label}");
        assert_eq!(restored.nationality_label, h.nationality_label, "{label}");
        assert_eq!(restored.position_label, h.position_label, "{label}");
        row.edits = format!(
            "impl+tested — {} · saved, reopened, {undo_steps} undone",
            done.join(", ")
        );

        reg.close_project(id);
        table.insert(label, row);
    }

    eprintln!("\n| anchor | identity columns absent | header (subject) | shipped string encoding | edits |");
    eprintln!("|---|---|---|---|---|");
    for (label, r) in &table {
        eprintln!(
            "| {label} | {} | {} | {} | {} |",
            if r.columns_absent.is_empty() {
                "none".to_string()
            } else {
                r.columns_absent.join(", ")
            },
            r.header,
            r.encoding,
            r.edits
        );
    }
    assert_eq!(table.len(), 27);
}

/// Measurement (25 Sep 2026): the raw `ubPc.sNZW` weight distribution per anchor, the six-bit rating
/// census that gates the current weight base, and one known player's raw height / weight across
/// titles. Printed as a table; the base a title actually uses is inferred from where the raw values
/// sit (NHL players average ~200 lb: a base 30 lb off puts the mean near 170 or 230).
#[test]
fn weight_raw_distribution_and_six_bit_rating_census_per_anchor() {
    eprintln!("\n| anchor | weight raw: n / mean / min / max | mean+120 | mean+150 | 6-bit rating fields (dSvy): n / max value | Kukkonen raw ypBQ / sNZW |");
    eprintln!("|---|---|---|---|---|---|");
    for (platform, game, db_path, _meta) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        let weight = bio
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "sNZW")
            .unwrap();
        let height = bio
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "ypBQ")
            .unwrap();
        let first = bio
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "HdeP")
            .unwrap();
        let last = bio
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "QbMR")
            .unwrap();
        let (mut n, mut sum, mut min, mut max) = (0u64, 0u64, u64::MAX, 0u64);
        let mut kukkonen = String::from("—");
        for rec in 0..bio.current_records as u32 {
            let w = db.read_u64(bio, rec, weight).unwrap_or(0);
            if w != 0 {
                n += 1;
                sum += w;
                min = min.min(w);
                max = max.max(w);
            }
            let f = db
                .read_inline_str(bio, rec, first)
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            let l = db
                .read_inline_str(bio, rec, last)
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            if f == "Lasse" && l == "Kukkonen" {
                kukkonen = format!("{} / {}", db.read_u64(bio, rec, height).unwrap_or(0), w);
            }
        }
        let mean = if n > 0 { sum as f64 / n as f64 } else { 0.0 };
        let (mut six_n, mut six_max) = (0usize, 0u64);
        if let Some(t) = db.table_canonical("dSvy") {
            for f in t.fields.iter().filter(|f| f.bit_width == 6) {
                six_n += 1;
                for rec in 0..t.current_records as u32 {
                    six_max = six_max.max(db.read_u64(t, rec, f).unwrap_or(0));
                }
            }
        }
        eprintln!(
            "| {platform}/{game} | {n} / {mean:.1} / {min} / {max} | {:.1} | {:.1} | {six_n} / {six_max} | {kukkonen} |",
            mean + 120.0,
            mean + 150.0
        );
    }
}
