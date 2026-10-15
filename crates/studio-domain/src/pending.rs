//! Pending-change review — what differs between a session's **acknowledged baseline** (the bytes
//! loaded from disk, or adopted after a verified save/rebind) and its current working bytes.
//!
//! The comparison is always baseline ↔ working, never against an undo snapshot: undoing past a
//! save is a real pending difference against the saved file, and a net-zero edit sequence is no
//! difference at all. Nothing here keeps a second edit log — the two byte images the session
//! already holds are the only state.
//!
//! Records are compared as byte slices first (cheap on a multi-megabyte database); only records
//! that differ are decoded field by field through the metadata, and every change is labelled
//! with the entity the record belongs to (team, player, league, arena …) rather than a raw index.
//! Table-level structural differences (row counts, record shapes, tables present on one side
//! only) are reported explicitly rather than folded into cell diffs, and bytes outside the record
//! blocks — the file/table headers and their CRCs — are summarised separately as derived integrity
//! changes, because they are consequences of edits, not edits.

use std::collections::HashMap;

use tdb_core::{Table, TdbFile};
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::entity::ids;

use crate::leagues::LeagueCatalog;
use crate::view::{read_field, FieldRaw, FieldView};

/// One changed cell.
#[derive(Clone, Debug)]
pub struct CellChange {
    pub studio_table: String,
    /// Human table name from the metadata (falls back to the studio id).
    pub table_name: String,
    pub record: u32,
    /// Human description of the record: `Boston Bruins`, `Sidney Crosby`, `Allsvenskan` … or
    /// `<table> #<record>` when the row has no resolvable identity.
    pub entity: String,
    pub studio_field: String,
    pub field_name: String,
    pub before: FieldView,
    pub after: FieldView,
}

/// A table whose record count or shape differs between baseline and working.
#[derive(Clone, Debug)]
pub struct TableChange {
    pub studio_table: String,
    pub table_name: String,
    pub before_records: Option<u32>,
    pub after_records: Option<u32>,
    /// Plain-language description (`3 rows added`, `record layout differs`, `table only in working`).
    pub note: String,
    /// Records that exist on only one side (indices in the side that has them).
    pub rows_only_in_working: Vec<u32>,
    pub rows_only_in_baseline: Vec<u32>,
}

/// Derived integrity bytes that changed as a consequence of the edits.
#[derive(Clone, Debug, Default)]
pub struct IntegrityChange {
    /// The file header (size / directory / header CRC) differs.
    pub file_header: bool,
    /// Table headers whose bytes differ (studio ids).
    pub table_headers: Vec<String>,
    /// Bytes that differ outside every known record block and header (trailing / unknown areas).
    pub other_bytes: usize,
}

/// The pending review for one session (one file role).
#[derive(Clone, Debug)]
pub struct PendingChanges {
    pub dirty: bool,
    pub cells: Vec<CellChange>,
    pub tables: Vec<TableChange>,
    pub integrity: IntegrityChange,
    /// Total changed cells before `limit` truncation.
    pub total_cells: usize,
    pub truncated: bool,
    /// Set when the two images could not be compared structurally (parse failure); the caller must
    /// show this, never an empty "no changes".
    pub error: Option<String>,
}

/// Compare `baseline` with `working`. `limit` caps the decoded cell list (the count stays exact).
pub fn pending_changes(
    baseline: &[u8],
    working: &[u8],
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    leagues: Option<&LeagueCatalog>,
    limit: usize,
) -> PendingChanges {
    let dirty = baseline != working;
    let mut out = PendingChanges {
        dirty,
        cells: Vec::new(),
        tables: Vec::new(),
        integrity: IntegrityChange::default(),
        total_cells: 0,
        truncated: false,
        error: None,
    };
    if !dirty {
        return out;
    }
    let (b, w) = match (TdbFile::parse(baseline), TdbFile::parse(working)) {
        (Ok(b), Ok(w)) => (b, w),
        (Err(e), _) => {
            out.error = Some(format!("the baseline database does not parse: {e}"));
            return out;
        }
        (_, Err(e)) => {
            out.error = Some(format!("the working database does not parse: {e}"));
            return out;
        }
    };

    let names = EntityNames::build(&w, meta, leagues);
    let table_name = |t: &Table| -> String {
        meta.table_name(&t.id.raw())
            .map(str::to_string)
            .unwrap_or_else(|| t.id.canonical(w.endian))
    };

    // File header: everything before the first table block.
    let first_block = w.tables.iter().map(|t| t.block_offset).min().unwrap_or(0);
    let first_block_b = b.tables.iter().map(|t| t.block_offset).min().unwrap_or(0);
    out.integrity.file_header = baseline.get(..first_block_b) != working.get(..first_block);

    let mut covered_w: Vec<(usize, usize)> = Vec::new();
    let mut covered_b: Vec<(usize, usize)> = Vec::new();

    for wt in &w.tables {
        let sid = wt.id.canonical(w.endian);
        let Some(bt) = b.table_canonical(&sid) else {
            out.tables.push(TableChange {
                studio_table: sid.clone(),
                table_name: table_name(wt),
                before_records: None,
                after_records: Some(wt.current_records as u32),
                note: "table only in the working database".into(),
                rows_only_in_working: (0..wt.current_records as u32).collect(),
                rows_only_in_baseline: Vec::new(),
            });
            continue;
        };
        // Header bytes (block header + descriptors) for the integrity summary.
        let wh = &working[wt.block_offset..wt.record_block];
        let bh = &baseline[bt.block_offset..bt.record_block];
        if wh != bh {
            out.integrity.table_headers.push(sid.clone());
        }
        let rl_w = wt.record_len_bytes as usize;
        let rl_b = bt.record_len_bytes as usize;
        covered_w.push((
            wt.block_offset,
            wt.record_block + rl_w * wt.max_records as usize,
        ));
        covered_b.push((
            bt.block_offset,
            bt.record_block + rl_b * bt.max_records as usize,
        ));

        if rl_w != rl_b || wt.fields.len() != bt.fields.len() {
            out.tables.push(TableChange {
                studio_table: sid.clone(),
                table_name: table_name(wt),
                before_records: Some(bt.current_records as u32),
                after_records: Some(wt.current_records as u32),
                note: format!(
                    "record layout differs ({} → {} bytes, {} → {} fields); cells are not compared",
                    rl_b,
                    rl_w,
                    bt.fields.len(),
                    wt.fields.len()
                ),
                rows_only_in_working: Vec::new(),
                rows_only_in_baseline: Vec::new(),
            });
            continue;
        }
        let nb = bt.current_records as u32;
        let nw = wt.current_records as u32;
        // Career deletion shifts the remaining records. If all survivors are unchanged,
        // report the actual removals rather than thousands of false per-cell edits.
        if nw < nb && ["tsSy", "OAqO", "USfF", "nCDp"].contains(&sid.as_str()) {
            let mut next = 0u32; let mut removed = Vec::new();
            for rec in 0..nb {
                let bs = bt.record_block + rec as usize * rl_b;
                let ws = wt.record_block + next as usize * rl_w;
                if next < nw && baseline.get(bs..bs + rl_b) == working.get(ws..ws + rl_w) { next += 1; }
                else { removed.push(rec); }
            }
            if next == nw {
                out.tables.push(TableChange { studio_table: sid.clone(), table_name: table_name(wt), before_records: Some(nb), after_records: Some(nw),
                    note: format!("{} row(s) removed", nb - nw), rows_only_in_working: Vec::new(), rows_only_in_baseline: removed });
                continue;
            }
        }
        if nb != nw || bt.max_records != wt.max_records {
            let (added, removed): (Vec<u32>, Vec<u32>) = if nw > nb {
                ((nb..nw).collect(), Vec::new())
            } else {
                (Vec::new(), (nw..nb).collect())
            };
            let mut note = match nw.cmp(&nb) {
                std::cmp::Ordering::Greater => format!("{} row(s) added", nw - nb),
                std::cmp::Ordering::Less => format!("{} row(s) removed", nb - nw),
                std::cmp::Ordering::Equal => String::new(),
            };
            if bt.max_records != wt.max_records {
                if !note.is_empty() {
                    note.push_str("; ");
                }
                note.push_str(&format!(
                    "capacity {} → {} records",
                    bt.max_records, wt.max_records
                ));
            }
            out.tables.push(TableChange {
                studio_table: sid.clone(),
                table_name: table_name(wt),
                before_records: Some(nb),
                after_records: Some(nw),
                note,
                rows_only_in_working: added,
                rows_only_in_baseline: removed,
            });
        }
        // Cell diffs over the records both sides have. Rows a reorder moved read as changed cells
        // on both rows, which is the truthful per-record view of a permutation.
        let common = nb.min(nw);
        for rec in 0..common {
            let ws = wt.record_block + rec as usize * rl_w;
            let bs = bt.record_block + rec as usize * rl_b;
            let (Some(wr), Some(br)) = (working.get(ws..ws + rl_w), baseline.get(bs..bs + rl_b))
            else {
                continue;
            };
            if wr == br {
                continue;
            }
            for (fw, fb) in wt.fields.iter().zip(bt.fields.iter()) {
                let after = read_field(&w, meta, enums, wt, rec, fw);
                let before = read_field(&b, meta, enums, bt, rec, fb);
                if raw_eq(&before.raw, &after.raw) {
                    continue;
                }
                out.total_cells += 1;
                if out.cells.len() >= limit {
                    out.truncated = true;
                    continue;
                }
                out.cells.push(CellChange {
                    studio_table: sid.clone(),
                    table_name: table_name(wt),
                    record: rec,
                    entity: names.entity(&w, meta, &sid, wt, rec),
                    studio_field: fw.id.canonical(w.endian),
                    field_name: after.name.clone(),
                    before,
                    after,
                });
            }
        }
    }
    for bt in &b.tables {
        let sid = bt.id.canonical(b.endian);
        if w.table_canonical(&sid).is_none() {
            out.tables.push(TableChange {
                studio_table: sid.clone(),
                table_name: table_name(bt),
                before_records: Some(bt.current_records as u32),
                after_records: None,
                note: "table only in the baseline database".into(),
                rows_only_in_working: Vec::new(),
                rows_only_in_baseline: (0..bt.current_records as u32).collect(),
            });
        }
    }
    // The final checksum moves when a table grows. Compare the remaining unknown
    // regions after removing each image's own known spans, not absolute offsets.
    // Only a valid footer is derived integrity; a damaged footer stays visible.
    cover_valid_footer(&b, baseline, &mut covered_b);
    cover_valid_footer(&w, working, &mut covered_w);
    let unknown_b = uncovered_bytes(baseline, first_block_b, &covered_b);
    let unknown_w = uncovered_bytes(working, first_block, &covered_w);
    out.integrity.other_bytes = (0..unknown_b.len().max(unknown_w.len()))
        .filter(|&i| unknown_b.get(i) != unknown_w.get(i)).count();
    out
}

fn raw_eq(a: &FieldRaw, b: &FieldRaw) -> bool {
    match (a, b) {
        (FieldRaw::Int(x), FieldRaw::Int(y)) => x == y,
        (FieldRaw::Text(x), FieldRaw::Text(y)) => x == y,
        _ => false,
    }
}

/// Count differing bytes in the working image that no record block or header covers.
fn cover_valid_footer(db: &TdbFile<'_>, bytes: &[u8], covered: &mut Vec<(usize, usize)>) {
    let Some(last) = db.tables.iter().max_by_key(|t| t.block_offset) else { return; };
    let Some(end) = bytes.len().checked_sub(4) else { return; };
    let start = last.block_offset + 0x28;
    if start > end { return; }
    let expected = tdb_core::crc::crc32_mpeg2(&bytes[start..end]);
    let encoded = match db.endian {
        tdb_core::Endian::Big => expected.to_be_bytes(),
        tdb_core::Endian::Little => expected.to_le_bytes(),
    };
    if bytes[end..] == encoded { covered.push((end, bytes.len())); }
}

fn uncovered_bytes(
    bytes: &[u8],
    first_block: usize,
    covered: &[(usize, usize)],
) -> Vec<u8> {
    let mut spans = covered.to_vec();
    spans.sort_unstable();
    let mut unknown = Vec::new();
    let mut cursor = first_block;
    for (s, e) in spans {
        if s > cursor {
            unknown.extend_from_slice(&bytes[cursor.min(bytes.len())..s.min(bytes.len())]);
        }
        cursor = cursor.max(e);
    }
    let end = bytes.len();
    if cursor < end {
        unknown.extend_from_slice(&bytes[cursor..end]);
    }
    unknown
}

#[cfg(test)]
mod integrity_tests {
    use super::uncovered_bytes;

    #[test]
    fn unknown_regions_survive_relocation_and_keep_their_content() {
        let before = [0, 0, 0, 7, 8, 0, 0, 9];
        let after = [0, 0, 0, 0, 7, 8, 0, 0, 9];
        assert_eq!(uncovered_bytes(&before, 1, &[(1, 3), (5, 7)]), vec![7, 8, 9]);
        assert_eq!(uncovered_bytes(&after, 1, &[(1, 4), (6, 8)]), vec![7, 8, 9]);
        let mut changed = after;
        changed[5] = 10;
        assert_ne!(uncovered_bytes(&before, 1, &[(1, 3), (5, 7)]),
            uncovered_bytes(&changed, 1, &[(1, 4), (6, 8)]));
    }
}

/// Resolves a record to the name of the entity it belongs to, built once per review.
struct EntityNames {
    teams: HashMap<u32, String>,
    players: HashMap<u32, String>,
    leagues: HashMap<u32, String>,
}

impl EntityNames {
    fn build(db: &TdbFile, meta: &FieldMetaIndex, leagues: Option<&LeagueCatalog>) -> Self {
        let mut teams = HashMap::new();
        let mut players = HashMap::new();
        let mut league_names = HashMap::new();
        if let Some(kott) = db.table_canonical(ids::TEAM) {
            for rec in 0..kott.current_records as u32 {
                let src = crate::view::team_name_source(db, meta, kott, rec);
                let name = first_nonempty(
                    db,
                    meta,
                    src.table,
                    src.record,
                    &["fullname", "teamname", "abbrname", "shortname"],
                );
                if !name.is_empty() {
                    teams.insert(rec, name);
                }
            }
        }
        if let Some(bio) = db.table_canonical(ids::BIO) {
            for rec in 0..bio.current_records as u32 {
                let first = first_nonempty(db, meta, bio, rec, &["firstname"]);
                let last = first_nonempty(db, meta, bio, rec, &["lastname"]);
                let name = format!("{first} {last}").trim().to_string();
                if !name.is_empty() {
                    players.insert(rec, name);
                }
            }
        }
        if let Some(c) = leagues {
            for l in &c.leagues {
                if let Some(r) = l.record {
                    league_names.insert(r, l.display());
                }
            }
        }
        EntityNames {
            teams,
            players,
            leagues: league_names,
        }
    }

    fn entity(
        &self,
        db: &TdbFile,
        meta: &FieldMetaIndex,
        sid: &str,
        table: &Table,
        rec: u32,
    ) -> String {
        let fallback = || {
            let tn = meta
                .table_name(&table.id.raw())
                .map(str::to_string)
                .unwrap_or_else(|| sid.to_string());
            format!("{tn} #{rec}")
        };
        match sid {
            ids::TEAM => self.teams.get(&rec).cloned().unwrap_or_else(fallback),
            ids::BIO => self.players.get(&rec).cloned().unwrap_or_else(fallback),
            crate::leagues::STOCK_LEAGUES => {
                self.leagues.get(&rec).cloned().unwrap_or_else(fallback)
            }
            _ => {
                // Schedule rows (P1-A): the game's running index and date, which is how the
                // schedule card shows them; the slot → team names need the league catalog and
                // stay on the card.
                let tn = meta.table_name(&table.id.raw()).unwrap_or("");
                // Settings catalogue rows (D3): the setting's name and, for a per-context default,
                // its screen — the same identities the Game Settings page shows.
                if tn == "stocksettings" || tn == "stocksettingdefaults" {
                    let by = |t: &Table, n: &str| {
                        t.fields.iter().find(|f| meta.field(&t.id.raw(), &f.id.raw()).is_some_and(|m| m.name == n)).cloned()
                    };
                    let legacy = by(table, "legacyid").and_then(|f| db.read_u64(table, rec, &f));
                    let setting_name = legacy.and_then(|lid| {
                        let st = db.table("fRvu")?;
                        let (fl, fs) = (by(st, "legacyid")?, by(st, "stringid")?);
                        (0..st.current_records as u32)
                            .find(|&r| db.read_u64(st, r, &fl) == Some(lid))
                            .and_then(|r| db.read_inline_str(st, r, &fs))
                            .map(|b| String::from_utf8_lossy(b).into_owned())
                    });
                    return match (tn, legacy, setting_name) {
                        ("stocksettings", Some(lid), Some(n)) => format!("Setting {n} (id {lid}) — global default"),
                        ("stocksettingdefaults", Some(lid), n) => {
                            let ctx = by(table, "context").and_then(|f| db.read_u64(table, rec, &f)).unwrap_or(0);
                            format!("Setting {} (id {lid}) — stored default for screen context {ctx}", n.unwrap_or_else(|| "?".into()))
                        }
                        _ => fallback(),
                    };
                }
                if tn.contains("schedule") {
                    let by = |n: &str| {
                        table.fields.iter().find(|f| meta.field(&table.id.raw(), &f.id.raw()).is_some_and(|m| m.name == n))
                    };
                    if let (Some(m), Some(d)) = (by("month"), by("day")) {
                        let index = by("index").and_then(|f| db.read_u64(table, rec, f)).unwrap_or(rec as u64);
                        let (month, day) = (db.read_u64(table, rec, m).unwrap_or(0), db.read_u64(table, rec, d).unwrap_or(0));
                        return format!("{tn} game {index} ({})", crate::schedule_edit::date_label(month, day));
                    }
                }
                // Rows keyed by a team or player id resolve through those names when the row
                // carries the reference (roster slots, strategies, companions).
                let team_field = match sid {
                    ids::ROSTER_LINES => Some(ids::SLOT_TEAM),
                    crate::team_tabs::STRATEGIES_TABLE => Some("vfEq"),
                    _ => None,
                };
                let team_ref = team_field
                    .and_then(|f| crate::view::read_u64_by_studio(db, table, rec, f))
                    .and_then(|tid| team_record_by_id(db, tid))
                    .and_then(|r| self.teams.get(&r).cloned());
                match team_ref {
                    Some(t) => format!("{t} — {}", fallback()),
                    None => fallback(),
                }
            }
        }
    }
}

fn team_record_by_id(db: &TdbFile, team_id: u64) -> Option<u32> {
    let kott = db.table_canonical(ids::TEAM)?;
    (0..kott.current_records as u32)
        .find(|&r| crate::view::read_u64_by_studio(db, kott, r, "vfEq") == Some(team_id))
}

fn first_nonempty(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    table: &Table,
    rec: u32,
    names: &[&str],
) -> String {
    for n in names {
        let s = crate::view::read_str_by_name(db, meta, table, rec, n);
        if !s.trim().is_empty() {
            return s.trim().to_string();
        }
    }
    String::new()
}
