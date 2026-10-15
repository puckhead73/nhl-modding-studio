//! Phase 4.4 gate — roster capacity normalization (docs/audits/roster-capacity.md).
//!
//! No committed anchor is trimmed below its ceiling (S3's empirical finding), so growth is
//! exercised against a **synthetically-packed fixture**: the nhl14 PS3 ROSTER save with each
//! roster-bearing table's `max_records` packed down to `current_records` (the slack bytes cut),
//! then normalized back up at workspace open. All on temp copies; the anchors are never written.

use std::path::{Path, PathBuf};

use studio_domain::capacity::{gen7_ceilings, ROSTER_TABLES};
use studio_domain::{EditSession, Platform, Project, ProjectRole, Workspace};
use tdb_core::TdbFile;
use tdb_savedata::SaveContainer;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const NHLNG: &str = "anchors/ps3/nhl14/databases/nhlng.db";
const META: &str = "anchors/ps3/nhl14/databases/nhlng-meta.xml";
const DEFS: &str = "defs";
const ROSTER: &str = "anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA";

/// A temp dir cleaned up on drop.
struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-cap-{tag}-{}", std::process::id()));
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

/// Pack one table's allocation down to its population — the exact inverse of
/// `TdbFile::grow_table_max`: cut the slack bytes off the end of the record region, pull every
/// later table's directory offset back, shrink `max_records` + `db_size`, recompute CRCs. This is
/// the shape a game-rewritten (trimmed) save arrives in.
fn pack_table(bytes: &[u8], canonical: &str) -> Vec<u8> {
    let db = TdbFile::parse(bytes).unwrap();
    let t = db.table_canonical(canonical).unwrap();
    let (cur, max) = (t.current_records, t.max_records);
    assert!(
        cur < max,
        "{canonical}: fixture table must have slack to cut"
    );
    let reclen = t.record_len_bytes as usize;
    let cut = (max - cur) as usize * reclen;
    let cut_at = t.record_block + cur as usize * reclen;

    let mut out = Vec::with_capacity(bytes.len() - cut);
    out.extend_from_slice(&bytes[..cut_at]);
    out.extend_from_slice(&bytes[cut_at + cut..]);

    // Pull back the directory offset of every table whose block begins after the cut region.
    let file_header = 0x18usize; // TDB v6 header length (per tdb-core layout)
    let dir_end = file_header + 8 * db.tables.len();
    let endian = db.endian;
    for i in 0..db.tables.len() {
        let entry = file_header + i * 8;
        let off = read_u32(&out, entry + 4, endian) as usize;
        if dir_end + off >= cut_at + cut {
            write_u32(&mut out, entry + 4, (off - cut) as u32, endian);
        }
    }
    write_u16(&mut out, t.block_offset + 0x14, cur, endian);
    let db_size = read_u32(&out, 0x08, endian);
    write_u32(&mut out, 0x08, db_size - cut as u32, endian);

    let view = out.clone();
    let re = TdbFile::parse(&view).unwrap();
    re.recompute_crcs(&mut out);
    out
}

fn read_u32(b: &[u8], at: usize, e: tdb_core::Endian) -> u32 {
    let a: [u8; 4] = b[at..at + 4].try_into().unwrap();
    match e {
        tdb_core::Endian::Big => u32::from_be_bytes(a),
        tdb_core::Endian::Little => u32::from_le_bytes(a),
    }
}
fn write_u32(b: &mut [u8], at: usize, v: u32, e: tdb_core::Endian) {
    let a = match e {
        tdb_core::Endian::Big => v.to_be_bytes(),
        tdb_core::Endian::Little => v.to_le_bytes(),
    };
    b[at..at + 4].copy_from_slice(&a);
}
fn write_u16(b: &mut [u8], at: usize, v: u16, e: tdb_core::Endian) {
    let a = match e {
        tdb_core::Endian::Big => v.to_be_bytes(),
        tdb_core::Endian::Little => v.to_le_bytes(),
    };
    b[at..at + 2].copy_from_slice(&a);
}

/// Write a synthetically-packed copy of the nhl14 ROSTER save (every roster-bearing table's `max`
/// packed to `cur`) into `tmp` and return its path.
fn packed_roster_path(tmp: &Tmp) -> String {
    let container = std::fs::read(repo(ROSTER)).unwrap();
    let parsed = SaveContainer::parse(&container).unwrap();
    let mut inner = parsed.extract_db().unwrap();
    for cid in ROSTER_TABLES {
        inner = pack_table(&inner, cid);
    }
    // Sanity: the packed fixture is a valid, packed TDB.
    let db = TdbFile::parse(&inner).unwrap();
    for cid in ROSTER_TABLES {
        let t = db.table_canonical(cid).unwrap();
        assert_eq!(t.max_records, t.current_records, "{cid} packed");
    }
    let repacked = parsed.repack(&inner).unwrap();
    let path = tmp.0.join("ROSTER-SYS-DATA-packed");
    std::fs::write(&path, repacked).unwrap();
    path.to_string_lossy().into_owned()
}

fn open_ws(tmp: &Tmp, roster_path: &str) -> Workspace {
    let nhlng_path = tmp.0.join("nhlng.db");
    std::fs::copy(repo(NHLNG), &nhlng_path).unwrap();
    let nhlng_path = nhlng_path.to_string_lossy().into_owned();
    let nhlng = EditSession::begin(
        Project::load(
            &nhlng_path,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let roster = EditSession::begin(
        Project::load(
            roster_path,
            Platform::Ps3,
            ProjectRole::Roster,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let mut ws =
        Workspace::with_roster(nhlng, nhlng_path, roster, roster_path.to_string()).unwrap();
    ws.set_game_id("nhl14");
    ws
}

/// The S3 verify script: open a packed roster → every table reaches the audited ceiling, every
/// pre-existing record intact, CRC-valid, alignment preserved; re-run → no further growth; the
/// vanilla anchor is a no-op.
#[test]
fn normalization_grows_packed_roster_to_ceiling_and_is_idempotent() {
    let tmp = Tmp::new("grow");
    let packed = packed_roster_path(&tmp);
    let mut ws = open_ws(&tmp, &packed);
    assert!(
        ws.roster_aligned(),
        "packing max_records must not break alignment (cur untouched)"
    );

    // Snapshot a populated bio field before normalization to prove records survive.
    let before = {
        let inner = SaveContainer::parse(&std::fs::read(&packed).unwrap())
            .unwrap()
            .extract_db()
            .unwrap();
        read_first_field(&inner, "ubPc", 0)
    };

    let grown = ws.normalize_roster_capacity().unwrap();
    let ceilings = gen7_ceilings("nhl14").unwrap();
    assert_eq!(grown.len(), 7, "all seven packed tables grow");
    for (i, cid) in ROSTER_TABLES.iter().enumerate() {
        let g = grown
            .iter()
            .find(|g| g.table == *cid)
            .unwrap_or_else(|| panic!("{cid} grew"));
        assert_eq!(g.to, ceilings[i], "{cid} grows to the audited ceiling");
        assert!(g.from < g.to, "{cid} grew upward from the packed size");
    }
    assert!(ws.roster_aligned(), "alignment survives normalization");
    assert!(ws.is_dirty(), "growth is a pending change until saved");

    // Idempotent: a second pass finds nothing to do.
    assert!(
        ws.normalize_roster_capacity().unwrap().is_empty(),
        "second pass is a no-op"
    );

    // Save both files in place, reopen, and verify the persisted shape from raw bytes.
    ws.save_in_place().unwrap();
    let inner = SaveContainer::parse(&std::fs::read(&packed).unwrap())
        .unwrap()
        .extract_db()
        .unwrap();
    let db = TdbFile::parse(&inner).unwrap();
    assert!(
        db.header_crc_ok() && db.size_ok(),
        "saved roster CRC/size valid"
    );
    for (i, cid) in ROSTER_TABLES.iter().enumerate() {
        let t = db.table_canonical(cid).unwrap();
        assert_eq!(t.max_records, ceilings[i], "{cid} persisted at the ceiling");
        assert!(
            t.current_records < t.max_records,
            "{cid} has the vanilla cur<max shape"
        );
    }
    assert_eq!(
        read_first_field(&inner, "ubPc", 0),
        before,
        "populated records preserved"
    );

    // A create now consumes pre-allocated slack: the roster file length must not change.
    let len_before = ws_roster_inner_len(&packed);
    let mut ws2 = open_ws(&tmp, &packed);
    let team = ws2
        .primary()
        .model()
        .unwrap()
        .slots
        .iter()
        .find(|s| s.player_id.is_some())
        .unwrap()
        .team_id;
    ws2.create_player(team, 0).unwrap();
    ws2.save_in_place().unwrap();
    assert_eq!(
        ws_roster_inner_len(&packed),
        len_before,
        "create fills slack, no growth"
    );
}

/// The vanilla anchor already meets its ceiling — normalization must be a byte-level no-op.
#[test]
fn normalization_is_noop_on_vanilla_anchor() {
    let tmp = Tmp::new("noop");
    let roster_path = tmp.0.join("ROSTER-SYS-DATA");
    std::fs::copy(repo(ROSTER), &roster_path).unwrap();
    let roster_path = roster_path.to_string_lossy().into_owned();
    let mut ws = open_ws(&tmp, &roster_path);
    assert!(
        ws.normalize_roster_capacity().unwrap().is_empty(),
        "vanilla file: nothing to grow"
    );
    assert!(!ws.is_dirty(), "no-op leaves the workspace clean");
}

fn read_first_field(db_bytes: &[u8], canonical: &str, record: u32) -> u64 {
    let db = TdbFile::parse(db_bytes).unwrap();
    let t = db.table_canonical(canonical).unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.bit_width > 0 && f.bit_width <= 64)
        .unwrap();
    db.read_u64(t, record, f).unwrap()
}

fn ws_roster_inner_len(path: &str) -> usize {
    SaveContainer::parse(&std::fs::read(path).unwrap())
        .unwrap()
        .extract_db()
        .unwrap()
        .len()
}
