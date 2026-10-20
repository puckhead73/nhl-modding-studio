//! Save Editor write-back quality pass over every committed anchor save (19 October 2026).
//!
//! For each anchor save that Save Editor can open, on a temporary copy:
//!
//! 1. **no-op** — open and save without an edit; the file must be byte-identical to the anchor;
//! 2. **edit** — renumber and rename one rostered player, save, and compare the result with the
//!    anchor structurally (container header words, checksums, which inner tables changed);
//! 3. **revert** — reopen the edited file, put the original values back, save; the file must be
//!    byte-identical to the anchor again;
//! 4. **move** — move a player to another team and back, save; byte-identical again.
//!
//! Every case runs; the failures are collected and reported together at the end so one broken
//! title never hides the state of the others. Run with `--nocapture` for the per-case report.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use studio_api::Registry;
use studio_domain::{BaselineStore, FieldValue, Platform};
use tdb_core::TdbFile;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn store() -> BaselineStore {
    let r = repo();
    BaselineStore::open(&r.join("baselines/INVENTORY.json"), &r.join("anchors")).unwrap()
}

fn defs() -> String {
    repo().join("defs").to_string_lossy().into_owned()
}

struct Temp(PathBuf);
impl Temp {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "nhlms-save-matrix-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        Temp(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn copy_save(tmp: &Temp, anchor_rel: &str) -> PathBuf {
    let src = repo().join("anchors").join(anchor_rel);
    let folder = src.parent().unwrap();
    let dest_folder = tmp.0.join("saves").join(folder.file_name().unwrap());
    let mut stack = vec![(folder.to_path_buf(), dest_folder.clone())];
    while let Some((from, to)) = stack.pop() {
        std::fs::create_dir_all(&to).unwrap();
        for e in std::fs::read_dir(&from).unwrap().flatten() {
            let target = to.join(e.file_name());
            if e.path().is_dir() {
                stack.push((e.path(), target));
            } else {
                std::fs::copy(e.path(), target).unwrap();
            }
        }
    }
    dest_folder.join(src.file_name().unwrap())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A roster save (or bare database): edited, reverted and moved.
    Roster,
    /// A mode save (franchise / season / Be a Pro / 3-on-3): no-op only, needs acknowledgement.
    Mode,
}

/// What the writer is expected to achieve on this file.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// A no-op or reverted save reproduces the anchor byte for byte.
    Identical,
    /// The inner database and every other record are reproduced, but the compressed wrapper is
    /// not byte-identical (NHL 16's LZ4 build is not reproduced; see `tdb_savedata::lz4_legacy`).
    Equivalent,
    /// Known-unsupported layout: identification is expected to fail with this text.
    Unidentified(&'static str),
}

struct Case {
    rel: &'static str,
    game: &'static str,
    platform: Platform,
    kind: Kind,
    expect: Expect,
}

const fn c(rel: &'static str, game: &'static str, platform: Platform, kind: Kind) -> Case {
    Case { rel, game, platform, kind, expect: Expect::Identical }
}

const fn eq(rel: &'static str, game: &'static str, platform: Platform, kind: Kind) -> Case {
    Case { rel, game, platform, kind, expect: Expect::Equivalent }
}

const fn un(rel: &'static str, game: &'static str, platform: Platform, why: &'static str) -> Case {
    Case { rel, game, platform, kind: Kind::Mode, expect: Expect::Unidentified(why) }
}

fn cases() -> Vec<Case> {
    use Kind::*;
    use Platform::*;
    vec![
        // --- PS3 roster saves -------------------------------------------------------------
        c("ps3/nhl08/savefiles/BLUS30062-002-000 - ROSTER/SYS-DATA", "nhl08", Ps3, Roster),
        c("ps3/nhl09/savefiles/BLUS3016802002026 - ROSTER/SYS-DATA", "nhl09", Ps3, Roster),
        c("ps3/nhl10/savefiles/BLUS303450200 - ROSTER/SYS-DATA", "nhl10", Ps3, Roster),
        c("ps3/nhl11/savefiles/BLUS305680200 - ROSTER/SYS-DATA", "nhl11", Ps3, Roster),
        c("ps3/nhl12/savefiles/BLUS307910200 - ROSTER/SYS-DATA", "nhl12", Ps3, Roster),
        c("ps3/nhl13/savefiles/BLUS310030200 - ROSTER/SYS-DATA", "nhl13", Ps3, Roster),
        c("ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA", "nhl14", Ps3, Roster),
        c("ps3/nhl15/savefiles/BLUS314360200 - ROSTER/SYS-DATA", "nhl15", Ps3, Roster),
        c("ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA", "nhllegacy", Ps3, Roster),
        // --- Xbox 360 roster saves --------------------------------------------------------
        c("xbox360/nhl07/savefiles/ROSTER1/ROSTER1", "nhl07", Xbox, Roster),
        c("xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658", "nhl08", Xbox, Roster),
        c("xbox360/nhl09/savefiles/ROSTER1 20260717230804/ROSTER1 20260717230804", "nhl09", Xbox, Roster),
        c("xbox360/nhl10/savefiles/ROSTER1 20260627045748/ROSTER1 20260627045748", "nhl10", Xbox, Roster),
        c("xbox360/nhl11/savefiles/ROSTER1 20260627054323/ROSTER1 20260627054323", "nhl11", Xbox, Roster),
        c("xbox360/nhl12/savefiles/ROSTER 20260627052113/ROSTER 20260627052113", "nhl12", Xbox, Roster),
        c("xbox360/nhl13/savefiles/ROSTER 20260627051601/ROSTER 20260627051601", "nhl13", Xbox, Roster),
        c("xbox360/nhl14/savefiles/ROSTER 20260627054746/ROSTER 20260627054746", "nhl14", Xbox, Roster),
        c("xbox360/nhl15/savefiles/ROSTER 20260627055422/ROSTER 20260627055422", "nhl15", Xbox, Roster),
        c("xbox360/nhllegacy/savefiles/ROSTER 20260627060052/ROSTER 20260627060052", "nhllegacy", Xbox, Roster),
        // --- PS4 roster saves -------------------------------------------------------------
        c("ps4/nhl15/savefiles/84f8102f - ROSTER/DATA", "nhl15", Ps4, Roster),
        eq("ps4/nhl16/savefiles/84f8102f - ROSTER/DATA", "nhl16", Ps4, Roster),
        c("ps4/nhl17/savefiles/RosterCustom0/DATA", "nhl17", Ps4, Roster),
        c("ps4/nhl18/savefiles/RosterCustom0/DATA", "nhl18", Ps4, Roster),
        c("ps4/nhl19/savefiles/RosterCustom0/DATA", "nhl19", Ps4, Roster),
        // --- PS4 3-on-3 saves (roster-bearing, acknowledged) --------------------------------
        c("ps4/nhl18/savefiles/ThreeOnThree0/DATA", "nhl18", Ps4, Mode),
        c("ps4/nhl21/savefiles/ThreeOnThree0/DATA", "nhl21", Ps4, Mode),
        // --- Mode saves, single file (no-op only) -------------------------------------------
        c("ps3/nhl09/savefiles/BLUS3016801002026 - DYNASTY/SYS-DATA", "nhl09", Ps3, Mode),
        c("ps3/nhl10/savefiles/BLUS303450100 - BEAGM/SYS-DATA", "nhl10", Ps3, Mode),
        c("ps3/nhl10/savefiles/BLUS303451100 - SEASON/SYS-DATA", "nhl10", Ps3, Mode),
        c("ps3/nhl11/savefiles/BLUS305680100 - BEAGM/SYS-DATA", "nhl11", Ps3, Mode),
        c("ps3/nhl11/savefiles/BLUS305681100 - SEASON/SYS-DATA", "nhl11", Ps3, Mode),
        c("ps3/nhl12/savefiles/BLUS307910100 - BEAGM/SYS-DATA", "nhl12", Ps3, Mode),
        c("ps3/nhl12/savefiles/BLUS307911100 - SEASON/SYS-DATA", "nhl12", Ps3, Mode),
        c("xbox360/nhl10/savefiles/DYNASTY1 20260627045642/DYNASTY1 20260627045642", "nhl10", Xbox, Mode),
        c("xbox360/nhl12/savefiles/SEASON 20260627051102/SEASON 20260627051102", "nhl12", Xbox, Mode),
        c("xbox360/nhl13/savefiles/DYNASTY 20260627051414/DYNASTY 20260627051414", "nhl13", Xbox, Mode),
        c("xbox360/nhl14/savefiles/SEASON 20260627054727/SEASON 20260627054727", "nhl14", Xbox, Mode),
        c("xbox360/nhl15/savefiles/DYNASTY 20260627055335/DYNASTY 20260627055335", "nhl15", Xbox, Mode),
        c("xbox360/nhllegacy/savefiles/SEASON 20260627060034/SEASON 20260627060034", "nhllegacy", Xbox, Mode),
        c("ps4/nhl15/savefiles/c6b05b60 - BEAGM/DATA", "nhl15", Ps4, Mode),
        eq("ps4/nhl16/savefiles/f0d3332a - SEASON/DATA", "nhl16", Ps4, Mode),
        // NHL 17+ PS4 mode saves nest a second LZ4 frame (NHL 18+ a section table, not a TDB):
        // not read yet — docs/formats/ps4-save-container.md.
        un("ps4/nhl17/savefiles/Season0/DATA", "nhl17", Ps4, "missing 'DB' magic"),
        un("ps4/nhl19/savefiles/BeAGM2/DATA", "nhl19", Ps4, "missing 'DB' magic"),
        un("ps4/nhl21/savefiles/Season0/DATA", "nhl21", Ps4, "missing 'DB' magic"),
    ]
}

/// The inner database of a save as the editor reads it (container or PS4 stream), or the file
/// itself for a bare database.
fn inner_db(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if TdbFile::parse(bytes).is_ok() {
        return Ok(bytes.to_vec());
    }
    if let Ok(c) = tdb_savedata::SaveContainer::parse(bytes) {
        return c.extract_db().map_err(|e| e.to_string());
    }
    if tdb_savedata::is_ps4_save(bytes) {
        return tdb_savedata::ps4_extract_roster(bytes).map_err(|e| e.to_string());
    }
    Err("not a database, container or PS4 save".into())
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter().zip(b.iter()).position(|(x, y)| x != y).or_else(|| {
        if a.len() != b.len() {
            Some(a.len().min(b.len()))
        } else {
            None
        }
    })
}

/// Which inner tables differ between two inner databases (by block range), plus the first
/// differing offset.
fn table_diff(a: &[u8], b: &[u8]) -> String {
    let (pa, pb) = match (TdbFile::parse(a), TdbFile::parse(b)) {
        (Ok(x), Ok(y)) => (x, y),
        _ => return "inner database no longer parses".into(),
    };
    let mut out = String::new();
    if a.len() != b.len() {
        let _ = write!(out, "inner length {} -> {}; ", a.len(), b.len());
    }
    if pa.tables.len() != pb.tables.len() {
        let _ = write!(out, "table count {} -> {}; ", pa.tables.len(), pb.tables.len());
        return out;
    }
    // Table byte ranges: block start to the next table's block start, the last to the end.
    let mut ranges: Vec<(String, usize)> = pa
        .tables
        .iter()
        .map(|t| (t.id.raw(), t.block_offset))
        .collect();
    ranges.sort_by_key(|r| r.1);
    let mut changed = Vec::new();
    for i in 0..ranges.len() {
        let start = ranges[i].1;
        let end = if i + 1 < ranges.len() { ranges[i + 1].1 } else { a.len().min(b.len()) };
        if end <= start || end > a.len() || end > b.len() {
            continue;
        }
        if a[start..end] != b[start..end] {
            changed.push(ranges[i].0.clone());
        }
    }
    let header_end = ranges.first().map(|r| r.1).unwrap_or(0);
    if a[..header_end.min(a.len()).min(b.len())] != b[..header_end.min(a.len()).min(b.len())] {
        changed.insert(0, "<file header/directory>".into());
    }
    let _ = write!(out, "changed tables: {}", changed.join(", "));
    out
}

/// A structural description of how `written` differs from `anchor`.
fn describe_diff(anchor: &[u8], written: &[u8]) -> String {
    let mut s = String::new();
    if anchor == written {
        return "byte-identical".into();
    }
    let _ = write!(
        s,
        "differs (len {} -> {}, first diff @0x{:x}); ",
        anchor.len(),
        written.len(),
        first_diff(anchor, written).unwrap_or(0)
    );
    if let (Ok(ca), Ok(cw)) = (
        tdb_savedata::SaveContainer::parse(anchor),
        tdb_savedata::SaveContainer::parse(written),
    ) {
        let _ = write!(
            s,
            "container {:?}: checksums {} -> {}; ",
            ca.header.version,
            ca.checksums_valid(),
            cw.checksums_valid()
        );
        let hlen = ca.header.version.header_len();
        let ha = &anchor[..hlen];
        let hw = &written[..hlen];
        if ha != hw {
            let words: Vec<String> = (0x10..hlen)
                .step_by(4)
                .filter(|&o| ha[o..o + 4] != hw[o..o + 4])
                .map(|o| format!("@0x{o:x} {:02x?}->{:02x?}", &ha[o..o + 4], &hw[o..o + 4]))
                .collect();
            let _ = write!(s, "header words {}; ", words.join(" "));
        }
    }
    match (inner_db(anchor), inner_db(written)) {
        (Ok(a), Ok(w)) => {
            if a == w {
                let _ = write!(s, "inner database identical (wrapper differs only)");
            } else {
                let _ = write!(s, "inner: {}", table_diff(&a, &w));
            }
        }
        (_, Err(e)) => {
            let _ = write!(s, "written file unreadable: {e}");
        }
        (Err(e), _) => {
            let _ = write!(s, "anchor unreadable: {e}");
        }
    }
    s
}

struct Picked {
    player_id: u64,
    team_id: Option<u64>,
    jersey_number: Option<i64>,
    first_name: String,
    last_name: String,
}

struct Outcome {
    label: String,
    lines: Vec<String>,
    failures: Vec<String>,
}

fn run_case(case: &Case) -> Outcome {
    let label = format!("{} {} [{}]", case.game, case.platform.as_str(), case.rel);
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    let tmp = Temp::new(case.game);
    let file = copy_save(&tmp, case.rel);
    let anchor = std::fs::read(&file).unwrap();
    let path = file.to_string_lossy().into_owned();
    let backups = tmp.0.join("backups");
    let store = store();
    let reg = Registry::default();
    let ack = case.kind == Kind::Mode;

    macro_rules! fail {
        ($($arg:tt)*) => {{
            let m = format!($($arg)*);
            failures.push(m.clone());
            lines.push(format!("  FAIL {m}"));
        }};
    }

    // Identify.
    let id = match (reg.save_editor_identify(&path, &store), case.expect) {
        (Err(e), Expect::Unidentified(why)) => {
            if e.to_string().contains(why) {
                lines.push(format!("  unidentified as expected ({why})"));
            } else {
                fail!("expected '{why}', got: {e}");
            }
            return Outcome { label, lines, failures };
        }
        (Ok(_), Expect::Unidentified(why)) => {
            fail!("identified, but expected to be unidentified ({why}) — update the expectation");
            return Outcome { label, lines, failures };
        }
        (Ok(id), _) => id,
        (Err(e), _) => {
            fail!("identify: {e}");
            return Outcome { label, lines, failures };
        }
    };
    lines.push(format!(
        "  identified: {} kind={} version={:?} checksums={:?} writable={} experimental={} decision={} candidates={:?}",
        id.format_label,
        id.kind,
        id.container_version,
        id.checksums_valid,
        id.writable,
        id.write_experimental,
        id.decision,
        id.candidates.iter().map(|c| format!("{}/{}{}", c.game_id, c.platform, if c.leading { "*" } else { "" })).collect::<Vec<_>>()
    ));
    if !id.candidates.iter().any(|c| c.game_id == case.game && c.platform == case.platform.as_str()) {
        fail!("not identified as {} on {}", case.game, case.platform.as_str());
        return Outcome { label, lines, failures };
    }
    if id.decision == "refused" {
        fail!("refused: {:?}", id.decision_reason);
        return Outcome { label, lines, failures };
    }

    let open = |reg: &Registry| {
        reg.save_editor_open(&path, case.game, case.platform, ack, None, &store, &defs(), &backups)
    };

    // 1. No-op save.
    let pid = match open(&reg) {
        Ok(o) => {
            if o.read_only {
                fail!("opened read-only");
                return Outcome { label, lines, failures };
            }
            o.summary.project_id
        }
        Err(e) => {
            fail!("open: {e}");
            return Outcome { label, lines, failures };
        }
    };
    match reg.save_editor_save(pid) {
        Ok(_) => {
            let written = std::fs::read(&file).unwrap();
            let d = describe_diff(&anchor, &written);
            lines.push(format!("  no-op save: {d}"));
            if written != anchor && case.expect == Expect::Equivalent && d.contains("inner database identical") {
                lines.push("  (equivalent, not identical — expected for this title)".into());
            } else if written != anchor {
                fail!("no-op save is not byte-identical: {d}");
                // Put the anchor back so later phases start from the real file.
                std::fs::write(&file, &anchor).unwrap();
            }
        }
        Err(e) => fail!("no-op save: {e}"),
    }
    reg.close_project_outcome(pid);
    if case.kind == Kind::Mode {
        return Outcome { label, lines, failures };
    }

    // 2. Edit: renumber + rename one rostered player.
    let pid = match open(&reg) {
        Ok(o) => o.summary.project_id,
        Err(e) => {
            fail!("reopen for edit: {e}");
            return Outcome { label, lines, failures };
        }
    };
    let players = reg.list_players(pid).unwrap();
    let Some(p) = players
        .iter()
        .find(|p| p.team_id.is_some() && p.jersey_number.is_some() && p.player_id != 0)
        .map(|p| (p.player_id, p.team_id, p.jersey_number, p.first_name.clone(), p.last_name.clone()))
        .map(|(player_id, team_id, jersey_number, first_name, last_name)| Picked { player_id, team_id, jersey_number, first_name, last_name })
    else {
        fail!("no rostered player with a number");
        return Outcome { label, lines, failures };
    };
    let team = p.team_id.unwrap();
    let taken: Vec<i64> = players
        .iter()
        .filter(|q| q.team_id == Some(team))
        .filter_map(|q| q.jersey_number)
        .collect();
    let number = (77..99).chain(2..77).find(|n| !taken.contains(n)).unwrap();
    let view = reg.player_view(pid, p.player_id).unwrap().unwrap();
    let field = |name: &str| {
        view.identity
            .iter()
            .find(|f| f.name == name && f.editable)
            .map(|f| (f.studio_table.clone(), f.studio_field.clone()))
    };
    let (Some(jersey), Some(last)) = (field("jersey"), field("lastname")) else {
        fail!("no editable jersey/lastname field");
        return Outcome { label, lines, failures };
    };
    let original_last = view
        .identity
        .iter()
        .find(|f| f.name == "lastname")
        .and_then(|f| f.text.clone())
        .unwrap_or_else(|| p.last_name.clone());
    let original_jersey = p.jersey_number.unwrap();
    let edits: Vec<(&str, &(String, String), FieldValue, FieldValue)> = vec![
        ("jersey", &jersey, FieldValue::Int(number), FieldValue::Int(original_jersey)),
        ("lastname", &last, FieldValue::Text("Savetest".into()), FieldValue::Text(original_last.clone())),
    ];
    for (name, (t, f), new, _) in &edits {
        if let Err(e) = reg.set_field(pid, t, view.bio_record, f, new.clone()) {
            fail!("set {name}: {e}");
        }
    }
    let edited_bytes = match reg.save_editor_save(pid) {
        Ok(_) => {
            let written = std::fs::read(&file).unwrap();
            let d = describe_diff(&anchor, &written);
            lines.push(format!(
                "  edit ({} {} #{} -> Savetest #{}): {d}",
                p.first_name, p.last_name, original_jersey, number
            ));
            if written == anchor {
                fail!("edited save equals the anchor (edit lost)");
            }
            if let Ok(c) = tdb_savedata::SaveContainer::parse(&written) {
                if !c.checksums_valid() {
                    fail!("container checksums invalid after edit");
                }
            }
            Some(written)
        }
        Err(e) => {
            fail!("save after edit: {e}");
            None
        }
    };
    reg.close_project_outcome(pid);

    // 3. Reopen the edited file, check the edit, revert, save → identical to the anchor.
    if edited_bytes.is_some() {
        let pid = match open(&reg) {
            Ok(o) => o.summary.project_id,
            Err(e) => {
                fail!("reopen edited: {e}");
                return Outcome { label, lines, failures };
            }
        };
        let after = reg.list_players(pid).unwrap();
        match after.iter().find(|q| q.player_id == p.player_id) {
            Some(q) if q.jersey_number == Some(number) && q.last_name == "Savetest" => {}
            Some(q) => fail!(
                "edit not read back: #{:?} {} (expected #{number} Savetest)",
                q.jersey_number, q.last_name
            ),
            None => fail!("edited player missing after reopen"),
        }
        let view2 = reg.player_view(pid, p.player_id).unwrap().unwrap();
        for (name, (t, f), _, back) in &edits {
            if let Err(e) = reg.set_field(pid, t, view2.bio_record, f, back.clone()) {
                fail!("revert {name}: {e}");
            }
        }
        match reg.save_editor_save(pid) {
            Ok(_) => {
                let written = std::fs::read(&file).unwrap();
                let d = describe_diff(&anchor, &written);
                lines.push(format!("  revert save: {d}"));
                if written != anchor && case.expect == Expect::Equivalent && d.contains("inner database identical") {
                    lines.push("  (equivalent, not identical — expected for this title)".into());
                } else if written != anchor {
                    fail!("reverted save is not byte-identical: {d}");
                    std::fs::write(&file, &anchor).unwrap();
                }
            }
            Err(e) => fail!("save after revert: {e}"),
        }
        reg.close_project_outcome(pid);
    }

    // 4. Move a player to another team and back.
    let pid = match open(&reg) {
        Ok(o) => o.summary.project_id,
        Err(e) => {
            fail!("reopen for move: {e}");
            return Outcome { label, lines, failures };
        }
    };
    let players = reg.list_players(pid).unwrap();
    let mut teams: Vec<u64> = players.iter().filter_map(|q| q.team_id).collect();
    teams.sort();
    teams.dedup();
    if teams.len() >= 2 {
        let mover = players
            .iter()
            .find(|q| q.team_id == Some(teams[0]) && q.player_id != 0)
            .map(|p| Picked { player_id: p.player_id, team_id: p.team_id, jersey_number: p.jersey_number, first_name: p.first_name.clone(), last_name: p.last_name.clone() })
            .unwrap();
        let (from, to) = (teams[0], teams[1]);
        let moved = reg.move_player(pid, mover.player_id, from, to);
        match moved {
            Ok(_) => {
                match reg.save_editor_save(pid) {
                    Ok(_) => {
                        let written = std::fs::read(&file).unwrap();
                        lines.push(format!(
                            "  move ({} {} team {from} -> {to}): {}",
                            mover.first_name,
                            mover.last_name,
                            describe_diff(&anchor, &written)
                        ));
                    }
                    Err(e) => fail!("save after move: {e}"),
                }
                reg.close_project_outcome(pid);
                let pid = match open(&reg) {
                    Ok(o) => o.summary.project_id,
                    Err(e) => {
                        fail!("reopen moved: {e}");
                        return Outcome { label, lines, failures };
                    }
                };
                let now = reg.list_players(pid).unwrap();
                let q = now.iter().find(|q| q.player_id == mover.player_id).unwrap();
                if q.team_id != Some(to) {
                    fail!("move not read back: team {:?}", q.team_id);
                }
                match reg.move_player(pid, mover.player_id, to, from) {
                    Ok(_) => match reg.save_editor_save(pid) {
                        Ok(_) => {
                            let written = std::fs::read(&file).unwrap();
                            let d = describe_diff(&anchor, &written);
                            lines.push(format!("  move back: {d}"));
                            if written != anchor {
                                // Informational: a slot may land in a different position.
                                lines.push("  (move back not byte-identical — slot placement differs; informational)".into());
                            }
                        }
                        Err(e) => fail!("save after move back: {e}"),
                    },
                    Err(e) => fail!("move back: {e}"),
                }
                reg.close_project_outcome(pid);
            }
            Err(e) => {
                lines.push(format!("  move refused: {e}"));
                reg.close_project_outcome(pid);
            }
        }
    } else {
        reg.close_project_outcome(pid);
    }

    Outcome { label, lines, failures }
}

#[test]
fn every_anchor_save_round_trips_through_save_editor() {
    let only: Option<String> = std::env::var("SAVE_MATRIX_ONLY").ok();
    let mut all_failures = Vec::new();
    let mut report = String::new();
    for case in cases() {
        if let Some(f) = &only {
            if !case.rel.contains(f.as_str()) {
                continue;
            }
        }
        let o = run_case(&case);
        let _ = writeln!(report, "{} — {}", o.label, if o.failures.is_empty() { "OK" } else { "FAIL" });
        for l in &o.lines {
            let _ = writeln!(report, "{l}");
        }
        for f in o.failures {
            all_failures.push(format!("{}: {f}", o.label));
        }
    }
    println!("{report}");
    if let Ok(p) = std::env::var("SAVE_MATRIX_REPORT") {
        std::fs::write(p, &report).unwrap();
    }
    assert!(
        all_failures.is_empty(),
        "{} failure(s):\n{}",
        all_failures.len(),
        all_failures.join("\n")
    );
}
