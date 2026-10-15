//! Supplementing a partial database with the tables it lacks, taken from a complete one.
//!
//! Console saves carry only part of the game database: a roster save holds the roster tables
//! (the game reads the rest from its own database), and a franchise or season save holds its mode
//! tables. The Save Editor shows such a file over its game's bundled anchor database: the save's
//! own tables stay editable and are the only bytes written back; the tables it lacks are appended
//! from the anchor as **frozen**, read-only reference data.
//!
//! The composite is a single TDB, so every view and operation runs on it unchanged:
//!
//! ```text
//! [save header] [save directory entries] [appended directory entries]
//! [save table region — verbatim]         [appended anchor table blocks] [EOF CRC]
//! ```
//!
//! Directory offsets are relative to the end of the directory, so lengthening the directory
//! leaves every save offset valid; the save's table region is copied without a byte changed.
//! [`strip`] reverses this — it drops the appended entries and everything from the first appended
//! block on, then recomputes the CRCs — so an unedited save round-trips byte for byte.

use std::collections::BTreeSet;
use std::sync::Arc;

use tdb_core::{Endian, TdbFile};

use crate::{DomainError, Result};

const FILE_HEADER: usize = 0x18;

/// The appended (frozen) part of a composite and the anchor it came from.
#[derive(Clone, Debug)]
pub struct Overlay {
    /// How many directory entries (the leading ones) belong to the save.
    pub save_tables: usize,
    /// Raw ids of the appended, read-only tables.
    pub frozen: BTreeSet<String>,
    /// The complete database the frozen tables are taken from.
    pub base: Arc<Vec<u8>>,
    /// Bytes the save's payload carries after its database (`db_size`), re-appended verbatim on
    /// strip. PS4 rosters end with a short record that includes the database size; mode saves
    /// carry padding or JSON.
    pub trailer: Vec<u8>,
    /// The save database's own length when it was read.
    pub own_len: usize,
}

fn put_u32(b: &mut [u8], o: usize, v: u32, e: Endian) {
    let w = match e {
        Endian::Big => v.to_be_bytes(),
        Endian::Little => v.to_le_bytes(),
    };
    b[o..o + 4].copy_from_slice(&w);
}

fn invalid(msg: impl Into<String>) -> DomainError {
    DomainError::Validation(msg.into())
}

/// Each table's block range `(start, end)` in file layout: from its block offset to the next
/// table's block (or the EOF CRC for the last one).
fn block_ranges(db: &TdbFile<'_>, len: usize) -> Vec<(usize, usize)> {
    let mut order: Vec<usize> = (0..db.tables.len()).collect();
    order.sort_by_key(|&i| db.tables[i].block_offset);
    let mut out = vec![(0, 0); db.tables.len()];
    for (k, &i) in order.iter().enumerate() {
        let end = order
            .get(k + 1)
            .map_or(len - 4, |&j| db.tables[j].block_offset);
        out[i] = (db.tables[i].block_offset, end);
    }
    out
}

/// Append to `save` every table of `base` that it lacks. `None` when nothing is missing.
pub fn compose(payload: &[u8], base: Arc<Vec<u8>>) -> Result<Option<(Vec<u8>, Overlay)>> {
    let declared = TdbFile::parse(payload)?.db_size as usize;
    if declared < FILE_HEADER + 4 || declared > payload.len() {
        return Err(invalid(
            "the save's database length does not match its header",
        ));
    }
    let (save, trailer) = payload.split_at(declared);
    let s = TdbFile::parse(save)?;
    let b = TdbFile::parse(&base)?;
    if s.endian != b.endian {
        return Err(invalid(
            "the save and the game database have different byte orders",
        ));
    }
    let present: BTreeSet<String> = s.tables.iter().map(|t| t.id.raw()).collect();
    let ranges = block_ranges(&b, base.len());
    let missing: Vec<usize> = (0..b.tables.len())
        .filter(|&i| !present.contains(&b.tables[i].id.raw()))
        .collect();
    if missing.is_empty() {
        return Ok(None);
    }
    let e = s.endian;
    let (ns, total) = (s.tables.len(), s.tables.len() + missing.len());
    let save_dir_end = FILE_HEADER + 8 * ns;
    let save_body = &save[save_dir_end..save.len() - 4];

    let mut out = Vec::with_capacity(save.len() + base.len());
    out.extend_from_slice(&save[..FILE_HEADER]);
    out.extend_from_slice(&save[FILE_HEADER..save_dir_end]);
    // Appended directory entries: offsets relative to the new directory end.
    let mut rel = save_body.len();
    for &i in &missing {
        let mut entry = [0u8; 8];
        entry[..4].copy_from_slice(&b.tables[i].id.0);
        put_u32(&mut entry, 4, rel as u32, e);
        out.extend_from_slice(&entry);
        rel += ranges[i].1 - ranges[i].0;
    }
    out.extend_from_slice(save_body);
    for &i in &missing {
        out.extend_from_slice(&base[ranges[i].0..ranges[i].1]);
    }
    out.extend_from_slice(&[0u8; 4]);
    let len = out.len() as u32;
    put_u32(&mut out, 0x08, len, e);
    put_u32(&mut out, 0x10, total as u32, e);
    let view = out.clone();
    TdbFile::parse(&view)?.recompute_crcs(&mut out);

    let frozen = missing.iter().map(|&i| b.tables[i].id.raw()).collect();
    Ok(Some((
        out,
        Overlay {
            save_tables: ns,
            frozen,
            base,
            trailer: trailer.to_vec(),
            own_len: save.len(),
        },
    )))
}

/// Remove the appended tables from a composite, returning the save's own database.
pub fn strip(composite: &[u8], overlay: &Overlay) -> Result<Vec<u8>> {
    let c = TdbFile::parse(composite)?;
    let e = c.endian;
    let k = overlay.save_tables;
    if c.tables.len() != k + overlay.frozen.len()
        || c.tables[..k]
            .iter()
            .any(|t| overlay.frozen.contains(&t.id.raw()))
        || c.tables[k..]
            .iter()
            .any(|t| !overlay.frozen.contains(&t.id.raw()))
    {
        return Err(invalid("the supplemented database's table directory changed; it cannot be separated from the save"));
    }
    let cut = c.tables[k..]
        .iter()
        .map(|t| t.block_offset)
        .min()
        .unwrap_or(composite.len() - 4);
    if c.tables[..k].iter().any(|t| t.block_offset >= cut) {
        return Err(invalid("a save table was moved after the supplemented tables; it cannot be separated from the save"));
    }
    let dir_end = FILE_HEADER + 8 * c.tables.len();
    let mut out = Vec::with_capacity(cut);
    out.extend_from_slice(&composite[..FILE_HEADER + 8 * k]);
    out.extend_from_slice(&composite[dir_end..cut]);
    out.extend_from_slice(&[0u8; 4]);
    let len = out.len() as u32;
    put_u32(&mut out, 0x08, len, e);
    put_u32(&mut out, 0x10, k as u32, e);
    let view = out.clone();
    TdbFile::parse(&view)?.recompute_crcs(&mut out);
    if !overlay.trailer.is_empty() && out.len() != overlay.own_len {
        return Err(invalid(
            "These changes resize the save's database, and this save stores its database size in a trailing record Studio cannot rewrite yet. Undo the change that added rows (for example a new player or roster slot) to save.",
        ));
    }
    out.extend_from_slice(&overlay.trailer);
    Ok(out)
}

/// The first frozen table whose content differs between two composites (table header and body;
/// the chained CRC words are ignored, since they change whenever an earlier table does).
pub fn frozen_changed(
    before: &[u8],
    after: &[u8],
    frozen: &BTreeSet<String>,
) -> Result<Option<String>> {
    let (a, b) = (TdbFile::parse(before)?, TdbFile::parse(after)?);
    let (ra, rb) = (
        block_ranges(&a, before.len()),
        block_ranges(&b, after.len()),
    );
    let content = |bytes: &[u8], (s, end): (usize, usize)| -> (Vec<u8>, Vec<u8>) {
        (
            bytes[s + 4..s + 0x24].to_vec(),
            bytes[s + 0x28..end].to_vec(),
        )
    };
    for id in frozen {
        let ia = a.tables.iter().position(|t| t.id.raw() == *id);
        let ib = b.tables.iter().position(|t| t.id.raw() == *id);
        match (ia, ib) {
            (Some(ia), Some(ib)) => {
                if content(before, ra[ia]) != content(after, rb[ib]) {
                    return Ok(Some(id.clone()));
                }
            }
            (None, None) => {}
            _ => return Ok(Some(id.clone())),
        }
    }
    Ok(None)
}

thread_local! {
    static FROZEN_SCOPE: std::cell::RefCell<Option<BTreeSet<String>>> = const { std::cell::RefCell::new(None) };
}

/// Run `f` with `frozen` (raw table ids) marking which tables' fields read as non-editable. The
/// field reader ([`crate::read_field`]) has no project in hand; the registry sets this around each
/// call on a supplemented session so every view built during it reports frozen fields read-only.
pub fn with_frozen_scope<R>(frozen: Option<&BTreeSet<String>>, f: impl FnOnce() -> R) -> R {
    let prev = FROZEN_SCOPE.with(|s| s.replace(frozen.cloned()));
    let out = f();
    FROZEN_SCOPE.with(|s| *s.borrow_mut() = prev);
    out
}

/// `true` when the current scope freezes table `raw_id`.
pub fn is_frozen(raw_id: &str) -> bool {
    FROZEN_SCOPE.with(|s| s.borrow().as_ref().is_some_and(|f| f.contains(raw_id)))
}

/// `true` when the current scope freezes table `table` or its field `field` (a `table.field`
/// entry: a field a roster change list cannot record).
pub fn is_frozen_field(table: &str, field: &str) -> bool {
    FROZEN_SCOPE.with(|s| {
        s.borrow()
            .as_ref()
            .is_some_and(|f| f.contains(table) || f.contains(&format!("{table}.{field}")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn anchor(rel: &str) -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../anchors")
                .join(rel),
        )
        .unwrap()
    }

    #[test]
    fn unedited_roster_save_round_trips_and_frozen_tables_read_from_the_base() {
        let base = Arc::new(anchor("ps3/nhl14/databases/nhlng.db"));
        let container = anchor("ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA");
        let save = tdb_savedata::extract_db(&container).unwrap();
        let (composite, overlay) = compose(&save, base.clone())
            .unwrap()
            .expect("tables to add");
        let c = TdbFile::parse(&composite).unwrap();
        assert!(c.size_ok() && c.header_crc_ok());
        assert_eq!(c.tables.len(), overlay.save_tables + overlay.frozen.len());
        // A frozen table reads exactly as in the base.
        let b = TdbFile::parse(&base).unwrap();
        let id = overlay
            .frozen
            .iter()
            .find(|id| b.table(id).unwrap().current_records > 0)
            .unwrap();
        let (tb, tc) = (b.table(id).unwrap(), c.table(id).unwrap());
        assert_eq!(tb.current_records, tc.current_records);
        let len = tb.record_len_bytes as usize * tb.current_records as usize;
        assert_eq!(
            &base[tb.record_block..tb.record_block + len],
            &composite[tc.record_block..tc.record_block + len]
        );
        // Stripping reproduces the save byte for byte.
        assert_eq!(strip(&composite, &overlay).unwrap(), save);
        assert_eq!(
            frozen_changed(&composite, &composite, &overlay.frozen).unwrap(),
            None
        );
    }

    #[test]
    fn an_edit_to_a_save_table_survives_the_strip_and_a_frozen_edit_is_detected() {
        let base = Arc::new(anchor("ps4/nhl17/databases/nhl_game.db"));
        let save =
            tdb_savedata::ps4_extract_roster(&anchor("ps4/nhl17/savefiles/RosterCustom0/DATA"))
                .unwrap();
        let (composite, overlay) = compose(&save, base).unwrap().unwrap();
        let c = TdbFile::parse(&composite).unwrap();
        // Flip one bit-field value in the first populated save table.
        let poke = |which: &dyn Fn(&tdb_core::Table) -> bool| -> (Vec<u8>, String) {
            let t = c
                .tables
                .iter()
                .find(|t| {
                    which(t)
                        && t.current_records > 0
                        && t.fields.iter().any(|f| f.kind == 3 && f.bit_width <= 32)
                })
                .unwrap();
            let f = t
                .fields
                .iter()
                .find(|f| f.kind == 3 && f.bit_width <= 32)
                .unwrap();
            let old = c.read_u64(t, 0, f).unwrap();
            let mut buf = composite.clone();
            assert!(c.set_u64(&mut buf, t, 0, f, old ^ 1));
            c.recompute_crcs(&mut buf);
            (buf, t.id.raw())
        };
        let (edited, id) = poke(&|t| !overlay.frozen.contains(&t.id.raw()));
        assert_eq!(
            frozen_changed(&composite, &edited, &overlay.frozen).unwrap(),
            None
        );
        // NHL 17's payload ends with a short trailer after the database: kept verbatim.
        assert!(!overlay.trailer.is_empty());
        assert_eq!(
            strip(&composite, &overlay).unwrap(),
            save,
            "unedited payload round-trips"
        );
        let stripped = strip(&edited, &overlay).unwrap();
        assert_ne!(stripped, save);
        assert!(stripped.ends_with(&overlay.trailer));
        let own = &stripped[..overlay.own_len];
        let (s, e) = (TdbFile::parse(&save).unwrap(), TdbFile::parse(own).unwrap());
        assert!(e.size_ok() && e.header_crc_ok() && e.tables.len() == s.tables.len());
        assert!(e.table(&id).is_some());
        let (frozen_edit, fid) = poke(&|t| overlay.frozen.contains(&t.id.raw()));
        assert_eq!(
            frozen_changed(&composite, &frozen_edit, &overlay.frozen).unwrap(),
            Some(fid)
        );
    }
}
