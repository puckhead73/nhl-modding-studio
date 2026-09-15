//! Phase 2 — the **scalar field mutation matrix**. For every committed main-DB anchor, on a
//! *temporary copy*, this drives the exact edit seam the UI/API calls (`EditSession::set_field`, the
//! same path `studio_api::Registry::set_field` invokes) across **one representative field of every
//! distinct on-disk codec present in that schema**, and proves the full round-trip contract from
//! `docs/plans/full-game-qa-editability.md` § Phase 2:
//!
//!   1. read the baseline raw + friendly (decoded) value;
//!   2. choose a *different* valid value, including the range boundaries;
//!   3. stage the edit through the domain path (validated + encoded + transactional);
//!   4. confirm the working state now decodes to the chosen friendly value (pending edit is correct)
//!      and the session is dirty with one new undo entry;
//!   5. serialize (`save`), re-parse, and confirm the raw bits *and* decoded value survive, with a
//!      valid CRC/size — and that a second no-op save is byte-identical (deterministic);
//!   6. undo and prove **byte equality** with the baseline (only the target field moved);
//!   7. redo and prove the saved output is **byte-identical** to the first save (deterministic redo);
//!   8. confirm an out-of-range value is rejected with **no** byte or undo-history change.
//!
//! Plus two negative gates the plan requires: `update=False` fields and key/identity fields cannot be
//! edited as scalars.
//!
//! Anchors are immutable: each session opens a temp copy (`Project::load` over a copied `.db`, so
//! `save()` is a raw-db pass-through and byte-equality is exact). The per-anchor codec coverage is
//! emitted deterministically to `docs/audits/data/mutation-matrix.csv` and guarded against drift —
//! regenerate an intended change with:
//!
//! ```text
//! QA_WRITE=1 cargo test -p studio-domain --test mutation_matrix
//! ```

use std::path::{Path, PathBuf};

use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole};
use tdb_core::TdbFile;
use tdb_defs::{FieldMeta, FieldMetaIndex, FieldType};
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

/// The single coverage key this emitter maps onto — contract §2.2.
///
/// Per anchor and per on-disk codec the test stages a bounded value through `EditSession`, saves,
/// re-parses, checks the decoded value and the CRC, undoes to byte equality, redoes to byte
/// equality, and proves an out-of-range value is refused. That is `db-operations/scalar-edit`.
///
/// It deliberately claims **nothing else**. The session's `save()` on a copied loose `.db` is
/// mechanically the raw-DB write path, so `save-raw-db/save` + `/reopen` are a *plausible* second
/// mapping — but the test asserts a field round-trip, not a save contract, so claiming those keys
/// would be inferring coverage from an adjacent assertion. Left absent for the architect to decide
/// (R-ABSENT: absence asserts nothing).
const OPERATION: &str = "db-operations/scalar-edit";
const CASE: &str = "studio_domain::mutation_matrix::mutation_matrix_over_all_anchors";

// ----------------------------------------------------------------------------- temp copies

/// A throwaway directory holding writable copies of otherwise-immutable anchors.
struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-p2-{tag}-{}", std::process::id()));
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

/// (label, platform, db path, meta path) for every main-DB anchor — same discovery as the read matrix.
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

// ----------------------------------------------------------------------------- codec taxonomy

/// The distinct on-disk value codecs a scalar edit must round-trip. Endianness (BE on PS3/Xbox360,
/// LE on PS4) is a whole-DB property, so it is covered by running the matrix over both anchor kinds
/// rather than as a separate class here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Codec {
    /// 1-bit integer flag (`0..=1`) — the boolean encoding.
    Boolean,
    /// Plain unsigned integer (`range_low >= 0`), the general enum/rating/count encoding.
    Unsigned,
    /// Two's-complement signed integer (negative `range_low` that fits the bit width).
    Signed,
    /// Unsigned with an all-ones `-1` "none" sentinel (`range_low == -1`) — the nullable/FK encoding.
    Neg1Sentinel,
    /// Offset-binary (`raw = value - range_low`) — the anchored career plus/minus sites.
    OffsetBinary,
    /// Inline fixed-width string.
    Text,
}

impl Codec {
    fn as_str(self) -> &'static str {
        match self {
            Codec::Boolean => "boolean",
            Codec::Unsigned => "unsigned",
            Codec::Signed => "signed-twos-complement",
            Codec::Neg1Sentinel => "neg1-sentinel",
            Codec::OffsetBinary => "offset-binary",
            Codec::Text => "inline-string",
        }
    }
}

/// Classify a field's codec from its metadata + on-disk bit width. Returns `None` for fields this
/// scalar matrix does not drive (reals, non-byte-aligned strings, ranges we can't bound).
fn classify(m: &FieldMeta, bit_width: u32) -> Option<Codec> {
    match m.kind {
        FieldType::String => (bit_width > 0 && bit_width.is_multiple_of(8)).then_some(Codec::Text),
        FieldType::Integer => {
            if m.offset_binary {
                return Some(Codec::OffsetBinary);
            }
            if m.signed() {
                return Some(Codec::Signed);
            }
            if m.neg1_sentinel() {
                return Some(Codec::Neg1Sentinel);
            }
            let (lo, hi) = (m.range_low?, m.range_high?);
            if hi <= lo {
                return None; // no distinct valid value to move to
            }
            if m.bit_depth == 1 && lo >= 0 && hi <= 1 {
                return Some(Codec::Boolean);
            }
            (lo >= 0).then_some(Codec::Unsigned)
        }
        _ => None,
    }
}

// ----------------------------------------------------------------------------- representative pick

/// A candidate editable field of one codec: canonical (studio) ids + record + its metadata.
#[derive(Clone)]
struct Rep {
    codec: Codec,
    table: String,
    field: String,
    record: u32,
    meta: FieldMeta,
    /// On-disk byte width (for the string max-length policy).
    width_bytes: usize,
}

/// Collect **every** editable, non-key candidate field, grouped by codec, in a stable
/// (canonical-id-sorted) order so selection is byte-deterministic across runs/hosts. Integer codecs
/// require a two-valued range so a distinct boundary edit always exists. The caller trial-runs
/// candidates in order and keeps the first that round-trips cleanly (a structural field whose edit
/// would trip the audit self-filters — those live in the Phase 3 relational matrix, not here).
fn candidates(db: &TdbFile, meta: &FieldMetaIndex) -> std::collections::BTreeMap<Codec, Vec<Rep>> {
    let mut out: std::collections::BTreeMap<Codec, Vec<Rep>> = std::collections::BTreeMap::new();

    let mut tables: Vec<&tdb_core::Table> = db.tables.iter().collect();
    tables.sort_by_key(|t| t.id.canonical(db.endian));
    for t in tables {
        if t.current_records == 0 {
            continue;
        }
        let traw = t.id.raw();
        let mut fields: Vec<&tdb_core::FieldDescriptor> = t.fields.iter().collect();
        fields.sort_by_key(|f| f.id.canonical(db.endian));
        for f in fields {
            let Some(fm) = meta.field(&traw, &f.id.raw()) else {
                continue;
            };
            if fm.key || !fm.updatable {
                continue; // keys/identity + read-only are covered by the negative gates, not here
            }
            let Some(codec) = classify(fm, f.bit_width) else {
                continue;
            };
            if codec != Codec::Text {
                // Need a genuinely two-valued range to move between boundaries.
                match (fm.range_low, fm.range_high) {
                    (Some(lo), Some(hi)) if hi > lo => {}
                    _ => continue,
                }
            }
            out.entry(codec).or_default().push(Rep {
                codec,
                table: t.id.canonical(db.endian),
                field: f.id.canonical(db.endian),
                record: 0,
                meta: fm.clone(),
                width_bytes: (f.bit_width / 8) as usize,
            });
        }
    }
    out
}

/// The valid values this representative's cycle will stage — used both to trial-select a candidate
/// and to run the asserting cycle, so the two agree exactly.
fn valid_values(sess: &EditSession, rep: &Rep) -> Vec<FieldValue> {
    match rep.codec {
        Codec::Text => text_values(sess, rep)
            .into_iter()
            .map(FieldValue::Text)
            .collect(),
        _ => {
            let base =
                rep.meta
                    .decode_raw(read_raw(sess.bytes(), &rep.table, rep.record, &rep.field));
            int_values(&rep.meta, base)
                .into_iter()
                .map(FieldValue::Int)
                .collect()
        }
    }
}

/// `true` if every value in the candidate's cycle can be staged (and undone) with no error — i.e. the
/// field is a real editable scalar whose edits don't trip the transactional audit. Leaves the session
/// exactly at baseline.
fn probe(sess: &mut EditSession, rep: &Rep) -> bool {
    let values = valid_values(sess, rep);
    if values.is_empty() {
        return false;
    }
    for v in values {
        match sess.set_field(&rep.table, rep.record, &rep.field, v) {
            Ok(_) => {
                assert!(
                    sess.undo(),
                    "probe: undo after a successful stage must succeed"
                );
            }
            Err(_) => return false,
        }
    }
    true
}

// ----------------------------------------------------------------------------- raw read helpers

fn read_raw(bytes: &[u8], table: &str, rec: u32, field: &str) -> u64 {
    let db = TdbFile::parse(bytes).unwrap();
    let t = db.table_canonical(table).unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == field)
        .unwrap();
    db.read_u64(t, rec, f).unwrap()
}

fn read_text(bytes: &[u8], table: &str, rec: u32, field: &str) -> String {
    let db = TdbFile::parse(bytes).unwrap();
    let t = db.table_canonical(table).unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == field)
        .unwrap();
    db.read_inline_str(t, rec, f)
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .unwrap_or_default()
}

// ----------------------------------------------------------------------------- the cycle

/// Run the full 8-step round-trip for one integer-codec representative, returning `(baseline, edited)`
/// friendly values for the CSV. Every valid value in `values` gets an independent cycle.
fn cycle_int(sess: &mut EditSession, rep: &Rep, values: &[i64], label: &str) -> (i64, i64) {
    let baseline = sess.bytes().to_vec();
    let base_val = rep
        .meta
        .decode_raw(read_raw(&baseline, &rep.table, rep.record, &rep.field));
    let mut first_edit = base_val;

    for (i, &v) in values.iter().enumerate() {
        let tag = format!("{label} {}.{} #{} = {v}", rep.table, rep.field, rep.record);
        let d0 = sess.undo_depth();

        // (3) stage through the domain path.
        sess.set_field(&rep.table, rep.record, &rep.field, FieldValue::Int(v))
            .unwrap_or_else(|e| panic!("{tag}: staging a valid value failed: {e}"));
        if i == 0 {
            first_edit = v;
        }
        // (4) the pending edit is correct: dirty, one new undo entry, working decodes to `v`.
        assert!(
            sess.is_dirty(),
            "{tag}: session not dirty after a value change"
        );
        assert_eq!(
            sess.undo_depth(),
            d0 + 1,
            "{tag}: expected one new undo entry"
        );
        let staged =
            rep.meta
                .decode_raw(read_raw(sess.bytes(), &rep.table, rep.record, &rep.field));
        assert_eq!(
            staged, v,
            "{tag}: working state does not decode to the staged value"
        );

        // (5) serialize → re-parse → raw + friendly survive; CRC/size valid; deterministic re-save.
        let saved = sess
            .save()
            .unwrap_or_else(|e| panic!("{tag}: save failed: {e}"));
        let reparsed =
            TdbFile::parse(&saved).unwrap_or_else(|e| panic!("{tag}: re-parse failed: {e}"));
        assert!(
            reparsed.header_crc_ok() && reparsed.size_ok(),
            "{tag}: saved DB failed CRC/size"
        );
        assert_eq!(
            rep.meta
                .decode_raw(read_raw(&saved, &rep.table, rep.record, &rep.field)),
            v,
            "{tag}: saved+reopened value differs"
        );
        assert_eq!(
            saved,
            sess.save().unwrap(),
            "{tag}: a second no-op save is not deterministic"
        );

        // (6) undo → byte equality with baseline (only this field ever moved).
        assert!(sess.undo(), "{tag}: undo returned false");
        assert_eq!(
            sess.bytes(),
            &baseline[..],
            "{tag}: undo did not restore the baseline bytes"
        );
        assert_eq!(
            sess.save().unwrap(),
            baseline,
            "{tag}: baseline save != anchor after undo"
        );

        // (7) redo → byte-identical to the first save (deterministic).
        assert!(sess.redo(), "{tag}: redo returned false");
        assert_eq!(
            sess.save().unwrap(),
            saved,
            "{tag}: redo output is not byte-identical to the edit"
        );

        // Reset to baseline for the next value.
        assert!(sess.undo(), "{tag}: reset undo returned false");
        assert_eq!(
            sess.bytes(),
            &baseline[..],
            "{tag}: not back at baseline for the next value"
        );
    }

    (base_val, first_edit)
}

/// The valid strings a string field's cycle stages: a max-length ASCII fill (width-exact, no NUL
/// terminator), a short ASCII value, and a multi-byte UTF-8 value trimmed to the byte budget (the
/// Unicode policy is a byte-length budget, not a character count). All are distinct from the baseline.
fn text_values(sess: &EditSession, rep: &Rep) -> Vec<String> {
    let base_txt = read_text(sess.bytes(), &rep.table, rep.record, &rep.field);
    let max_fill: String = std::iter::repeat_n('Q', rep.width_bytes).collect();
    let unicode = fit_utf8("café-Ölmüz", rep.width_bytes);
    let mut values = vec![max_fill, "QA".to_string(), unicode];
    values.retain(|s| !s.is_empty() && s.len() <= rep.width_bytes && *s != base_txt);
    values.dedup();
    if values.is_empty() {
        values.push(if base_txt == "Q" {
            "R".into()
        } else {
            "Q".into()
        });
    }
    values
}

/// The full cycle for the inline-string representative (max-length + Unicode policy), returning
/// `(baseline_text, edited_text)`.
fn cycle_text(sess: &mut EditSession, rep: &Rep, label: &str) -> (String, String) {
    let baseline = sess.bytes().to_vec();
    let base_txt = read_text(&baseline, &rep.table, rep.record, &rep.field);
    let values = text_values(sess, rep);
    let first_edit = values[0].clone();

    for v in &values {
        let tag = format!(
            "{label} {}.{} #{} = {v:?}",
            rep.table, rep.field, rep.record
        );
        let d0 = sess.undo_depth();
        sess.set_field(
            &rep.table,
            rep.record,
            &rep.field,
            FieldValue::Text(v.clone()),
        )
        .unwrap_or_else(|e| panic!("{tag}: staging a valid string failed: {e}"));
        assert!(sess.is_dirty(), "{tag}: not dirty after a string change");
        assert_eq!(
            sess.undo_depth(),
            d0 + 1,
            "{tag}: expected one new undo entry"
        );
        assert_eq!(
            read_text(sess.bytes(), &rep.table, rep.record, &rep.field),
            *v,
            "{tag}: staged text differs"
        );

        let saved = sess
            .save()
            .unwrap_or_else(|e| panic!("{tag}: save failed: {e}"));
        let reparsed = TdbFile::parse(&saved).unwrap();
        assert!(
            reparsed.header_crc_ok() && reparsed.size_ok(),
            "{tag}: saved DB failed CRC/size"
        );
        assert_eq!(
            read_text(&saved, &rep.table, rep.record, &rep.field),
            *v,
            "{tag}: saved+reopened text differs"
        );
        assert_eq!(
            saved,
            sess.save().unwrap(),
            "{tag}: string re-save not deterministic"
        );

        assert!(sess.undo(), "{tag}: undo returned false");
        assert_eq!(
            sess.bytes(),
            &baseline[..],
            "{tag}: undo did not restore baseline"
        );
        assert!(sess.redo(), "{tag}: redo returned false");
        assert_eq!(
            sess.save().unwrap(),
            saved,
            "{tag}: string redo not byte-identical"
        );
        assert!(sess.undo(), "{tag}: reset undo returned false");
        assert_eq!(sess.bytes(), &baseline[..], "{tag}: not back at baseline");
    }

    (base_txt, first_edit)
}

/// Truncate `s` to the largest UTF-8 prefix that fits `max_bytes` bytes (never splitting a char).
fn fit_utf8(s: &str, max_bytes: usize) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if out.len() + c.len_utf8() > max_bytes {
            break;
        }
        out.push(c);
    }
    out
}

/// The integer test values for a codec: both range boundaries, minus the baseline (which needs no
/// move). The `-1` sentinel is naturally the low boundary of the neg1 class.
fn int_values(m: &FieldMeta, baseline: i64) -> Vec<i64> {
    let (lo, hi) = (m.range_low.unwrap(), m.range_high.unwrap());
    let mut v = vec![hi, lo];
    v.dedup();
    v.retain(|&x| x != baseline);
    v
}

// ----------------------------------------------------------------------------- negative gates

/// Prove `update=False` and key/identity fields cannot be edited as scalars, and that an out-of-range
/// value is rejected without touching bytes or undo history. Runs once per anchor over the working
/// session (which is at baseline). Returns whether a read-only field and a key field were exercised.
fn assert_negative_gates(
    sess: &mut EditSession,
    db: &TdbFile,
    meta: &FieldMetaIndex,
    label: &str,
) -> (bool, bool) {
    let before = sess.bytes().to_vec();
    let d0 = sess.undo_depth();

    let mut tables: Vec<&tdb_core::Table> = db.tables.iter().collect();
    tables.sort_by_key(|t| t.id.canonical(db.endian));

    let mut ro_hit = false;
    let mut key_hit = false;
    for t in &tables {
        if t.current_records == 0 {
            continue;
        }
        let traw = t.id.raw();
        let mut fields: Vec<&tdb_core::FieldDescriptor> = t.fields.iter().collect();
        fields.sort_by_key(|f| f.id.canonical(db.endian));
        for f in fields {
            let Some(fm) = meta.field(&traw, &f.id.raw()) else {
                continue;
            };
            let tc = t.id.canonical(db.endian);
            let fc = f.id.canonical(db.endian);
            // A read-only (update=False) integer field must reject an in-range value.
            if !ro_hit && !fm.updatable && fm.kind == FieldType::Integer {
                let v = fm.range_low.unwrap_or(0);
                let err = sess.set_field(&tc, 0, &fc, FieldValue::Int(v)).is_err();
                assert!(err, "{label}: read-only field {tc}.{fc} accepted an edit");
                ro_hit = true;
            }
            // A key/identity field must be refused by the write guardrail regardless of its `update`
            // flag — some schemas (NHL07 `vfEq`) mark a primary key `update=True`, and editing a key
            // in place would orphan its foreign keys.
            if !key_hit && fm.key {
                let v = fm.range_low.unwrap_or(0);
                assert!(
                    sess.set_field(&tc, 0, &fc, FieldValue::Int(v)).is_err(),
                    "{label}: key field {tc}.{fc} accepted a scalar edit"
                );
                key_hit = true;
            }
            if ro_hit && key_hit {
                break;
            }
        }
        if ro_hit && key_hit {
            break;
        }
    }

    // Nothing above may have mutated bytes or undo history.
    assert_eq!(
        sess.bytes(),
        &before[..],
        "{label}: a rejected edit changed the working bytes"
    );
    assert_eq!(
        sess.undo_depth(),
        d0,
        "{label}: a rejected edit grew the undo history"
    );
    (ro_hit, key_hit)
}

/// Prove one representative's out-of-range value is rejected leaving bytes + history intact.
fn assert_out_of_range(sess: &mut EditSession, rep: &Rep, label: &str) {
    // Text: a string one byte over the field width. Int: one past the high boundary.
    let before = sess.bytes().to_vec();
    let d0 = sess.undo_depth();
    let tag = format!("{label} {}.{}", rep.table, rep.field);
    let rejected = match rep.codec {
        Codec::Text => {
            let over: String = std::iter::repeat_n('Z', rep.width_bytes + 1).collect();
            sess.set_field(&rep.table, rep.record, &rep.field, FieldValue::Text(over))
                .is_err()
        }
        _ => {
            let hi = rep.meta.range_high.unwrap();
            match hi.checked_add(1) {
                Some(over) => sess
                    .set_field(&rep.table, rep.record, &rep.field, FieldValue::Int(over))
                    .is_err(),
                None => true, // saturated high bound — nothing above it to reject; treated as pass
            }
        }
    };
    assert!(rejected, "{tag}: an out-of-range value was accepted");
    assert_eq!(
        sess.bytes(),
        &before[..],
        "{tag}: a rejected edit changed the working bytes"
    );
    assert_eq!(
        sess.undo_depth(),
        d0,
        "{tag}: a rejected edit grew the undo history"
    );
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
fn mutation_matrix_over_all_anchors() {
    let mut csv = String::from(
        "platform,game,codec,table,field,bit_depth,record,baseline,edited,roundtrip,invalid_rejected\n",
    );
    let mut checked = 0;
    let mut total_reps = 0;
    let tmp = Tmp::new("mut");

    let mut ev = Emitter::from_env("mutation_matrix", "db-edit", &repo_root());
    // Declared before anything runs: a catalog unit with no anchor is a recorded missing fixture.
    let keys: Vec<(String, CoverageKey)> = CATALOG
        .iter()
        .map(|unit| {
            let target = format!("{}-{}", unit.game, unit.platform);
            let key = ev.declare(&format!("{target}/{OPERATION}"));
            (target, key)
        })
        .collect();
    let mut covered: Vec<String> = Vec::new();

    for (label, platform, db_path, meta_path) in anchors() {
        let (pdir, game) = label.split_once('/').unwrap();
        let target = format!("{game}-{pdir}");
        let key = keys
            .iter()
            .find(|(t, _)| *t == target)
            .map(|(_, k)| k.clone());
        let fixture = file_digest(&db_path).unwrap_or_default();
        if let Some(key) = &key {
            ev.input_digest(
                &db_path
                    .strip_prefix(repo(""))
                    .unwrap_or(&db_path)
                    .to_string_lossy()
                    .replace('\\', "/"),
                (!fixture.is_empty()).then(|| fixture.clone()),
            );
            let _ = key;
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

        // Snapshot the schema for candidate selection (independent of the mutable session).
        let bytes = sess.bytes().to_vec();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let by_codec = candidates(&db, &meta);
        assert!(
            !by_codec.is_empty(),
            "{label}: no editable codec candidates found"
        );

        // Negative gates first (session is at baseline, unmutated afterwards).
        match &key {
            Some(key) => {
                let case = format!("{CASE}#{target}-negative-gates");
                ev.case(&key.clone(), &case, || {
                    assert_negative_gates(&mut sess, &db, &meta, &label);
                });
                ev.observe_hashed(
                    key,
                    &case,
                    Outcome::RefusedClean,
                    &fixture,
                    "",
                    "update=False and key/identity fields refuse a scalar edit, and an out-of-range \
                     value moves neither bytes nor undo history",
                );
            }
            None => {
                assert_negative_gates(&mut sess, &db, &meta, &label);
            }
        }

        // For each codec present, pick the first candidate (stable order) whose cycle round-trips
        // cleanly, then run the asserting cycle over it. Emit one CSV row per codec, in fixed order.
        let mut rows: std::collections::BTreeMap<Codec, String> = std::collections::BTreeMap::new();
        for (codec, reps) in &by_codec {
            let Some(rep) = reps.iter().find(|r| probe(&mut sess, r)) else {
                continue;
            };

            let case = format!("{CASE}#{target}-{}", rep.codec.as_str());
            let run_cycle = |sess: &mut EditSession| {
                let (baseline, edited) = match rep.codec {
                    Codec::Text => cycle_text(sess, rep, &label),
                    _ => {
                        let base_val = rep.meta.decode_raw(read_raw(
                            sess.bytes(),
                            &rep.table,
                            rep.record,
                            &rep.field,
                        ));
                        let vals = int_values(&rep.meta, base_val);
                        let (b, e) = cycle_int(sess, rep, &vals, &label);
                        (b.to_string(), e.to_string())
                    }
                };
                assert_out_of_range(sess, rep, &label);
                assert!(
                    !sess.is_dirty(),
                    "{label}: session left dirty after {}",
                    rep.field
                );
                (baseline, edited)
            };
            let (baseline, edited) = match &key {
                Some(key) => {
                    let key = key.clone();
                    let out = ev.case(&key, &case, || run_cycle(&mut sess));
                    // Only now: staged, saved, re-parsed, undone, redone and out-of-range refused.
                    ev.observe_hashed(
                        &key,
                        &case,
                        Outcome::Pass,
                        &fixture,
                        "",
                        &format!(
                            "{} codec {}.{} #{}: stage -> save -> reopen -> undo -> redo, \
                             out-of-range refused",
                            rep.codec.as_str(),
                            rep.table,
                            rep.field,
                            rep.record
                        ),
                    );
                    out
                }
                None => run_cycle(&mut sess),
            };

            rows.insert(
                *codec,
                format!(
                    "{},{},{},{},{},{},{},{},{},yes,yes\n",
                    csv_cell(pdir),
                    csv_cell(game),
                    csv_cell(rep.codec.as_str()),
                    csv_cell(&rep.table),
                    csv_cell(&rep.field),
                    rep.meta.bit_depth,
                    rep.record,
                    csv_cell(&baseline),
                    csv_cell(&edited),
                ),
            );
            total_reps += 1;
        }
        assert!(
            !rows.is_empty(),
            "{label}: no codec produced a clean round-trip"
        );
        for (_, row) in rows {
            csv.push_str(&row);
        }
        checked += 1;
    }

    for (target, key) in &keys {
        if !covered.contains(target) {
            ev.observe(
                key,
                &format!("{CASE}#{target}"),
                Outcome::SkippedMissingFixture,
                "no main-DB anchor for this unit; nothing was executed",
            );
        }
    }

    assert_eq!(checked, 27, "expected 27 anchors, checked {checked}");
    assert!(
        total_reps >= 27,
        "expected at least one codec proof per anchor, got {total_reps}"
    );

    let path = repo("docs/audits/data/mutation-matrix.csv");
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
            "mutation-matrix.csv drift — regenerate with `QA_WRITE=1 cargo test -p studio-domain --test mutation_matrix`\n{drift}"
        );
    }
    ev.finish();
}
