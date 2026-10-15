//! LOC-W3 — the preserving serializer (research handoff 20 rev. 3 §4-P) on the committed
//! dictionaries: T1a no-op identity on every anchor, T2/T3 insertion positions, T4 hash-collision
//! refusals (incl. the shipped PS4 `TXT_STATS_RANK` pair), T5 class-B refusal with the descent rows
//! (Xbox NHL 10) while O-T works there, T6 text growth, T7 repoint → growth sharer semantics,
//! T9 nulls, T10 non-ASCII, T12 capacity, and the independent accounting of every changed byte.
//! Nothing here says a loader accepts a grown file (runtime gates R-A/R-B).

use std::path::{Path, PathBuf};

use tdb_loc::{apply_growth, apply_repoint, apply_retext, loc_hash, plan_growth, plan_repoint, plan_retext, verify, GrowthOp, GrowthPlan, LocDictionary, OrderClass};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn anchors() -> Vec<String> {
    let mut files = Vec::new();
    for platform in ["ps3", "xbox360"] {
        for game in ["nhl07", "nhl08", "nhl09", "nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"] {
            files.push(format!("anchors/{platform}/{game}/databases/nhl_eng_us.db"));
        }
    }
    for game in ["nhl15", "nhl16", "nhl17", "nhl18", "nhl20"] {
        files.push(format!("anchors/ps4/{game}/databases/eng_us.db"));
    }
    files
}

fn read(rel: &str) -> Vec<u8> {
    std::fs::read(repo(rel)).unwrap()
}

/// The layout facts a test derives on its own from the input (never from the serializer).
struct Facts {
    endian_big: bool,
    block_offset: usize,
    record_block: usize,
    rows: usize,
    pool_start: usize,
    pool_len: usize,
    text_off: usize,
    key_off: usize,
    hash_off: usize,
}

fn facts(bytes: &[u8]) -> Facts {
    let db = tdb_core::TdbFile::parse(bytes).unwrap();
    let t = &db.tables[0];
    let f = |kind: u32| t.fields.iter().find(|f| f.kind == kind).unwrap();
    Facts {
        endian_big: db.endian == tdb_core::Endian::Big,
        block_offset: t.block_offset,
        record_block: t.record_block,
        rows: t.current_records as usize,
        pool_start: t.record_block + t.max_records as usize * 16,
        pool_len: bytes.len() - 4 - (t.record_block + t.max_records as usize * 16),
        text_off: (f(14).bit_offset / 8) as usize,
        key_off: (f(13).bit_offset / 8) as usize,
        hash_off: (f(3).bit_offset / 8) as usize,
    }
}

fn u32_at(b: &[u8], at: usize, big: bool) -> u32 {
    let a = [b[at], b[at + 1], b[at + 2], b[at + 3]];
    if big { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) }
}

/// Independent accounting of a grown output against its input: derives the expected layout from
/// the input facts and the plan's public fields, then checks every byte.
fn account(input: &[u8], plan: &GrowthPlan, out: &[u8]) {
    let f = facts(input);
    let k = plan.inserts.len();
    let shift = 16 * k;
    let appended: usize = plan.appended_len;
    assert_eq!(out.len(), input.len() + shift + appended, "length");
    // Header/directory/table block up to the counts: identical except db_size and the hash.
    for i in 0..f.record_block {
        let expect_same = !((0x08..0x0C).contains(&i) || (0x14..0x18).contains(&i) || (f.block_offset..f.block_offset + 4).contains(&i) || (f.block_offset + 0x14..f.block_offset + 0x18).contains(&i) || (f.block_offset + 0x24..f.block_offset + 0x28).contains(&i));
        if expect_same {
            assert_eq!(input[i], out[i], "prefix byte {i:#x}");
        }
    }
    assert_eq!(u32_at(out, 0x08, f.endian_big) as usize, out.len(), "db_size");
    let counts = |b: &[u8], at: usize| {
        let a = [b[at], b[at + 1]];
        if f.endian_big { u16::from_be_bytes(a) } else { u16::from_le_bytes(a) }
    };
    assert_eq!(counts(out, f.block_offset + 0x14) as usize, f.rows + k, "max_records");
    assert_eq!(counts(out, f.block_offset + 0x16) as usize, f.rows + k, "current_records");
    // Records: existing rows in order (content identical except re-pointed text slots), inserts
    // at position + (inserts before).
    let edited: std::collections::HashMap<usize, u32> = plan.edits.iter().flat_map(|e| e.records.iter().map(move |r| (*r, e))).map(|(r, _)| (r, 0)).collect();
    let mut next = 0usize;
    let mut out_rec = 0usize;
    for i in 0..=f.rows {
        while next < k && plan.inserts[next].position == i {
            let ins = &plan.inserts[next];
            assert_eq!(ins.output_record, out_rec, "insert output index");
            let at = f.record_block + out_rec * 16;
            assert_eq!(u32_at(out, at + f.hash_off, f.endian_big), ins.hash, "inserted hash");
            assert_eq!(&out[at + 12..at + 16], &[0, 0, 0, 0], "reserved word");
            let kp = u32_at(out, at + f.key_off, f.endian_big) as usize;
            assert!(kp >= f.pool_len, "inserted key points into the appended region");
            let tp = u32_at(out, at + f.text_off, f.endian_big);
            if ins.text.is_some() {
                assert!(tp as usize >= f.pool_len, "inserted text points into the appended region");
            } else {
                assert_eq!(tp, 0xFFFF_FFFF, "null text pointer");
            }
            out_rec += 1;
            next += 1;
        }
        if i == f.rows {
            break;
        }
        let src = f.record_block + i * 16;
        let dst = f.record_block + out_rec * 16;
        for b in 0..16 {
            let in_text_slot = (f.text_off..f.text_off + 4).contains(&b);
            if in_text_slot && edited.contains_key(&i) {
                continue;
            }
            assert_eq!(input[src + b], out[dst + b], "record {i} byte {b}");
        }
        if edited.contains_key(&i) {
            let tp = u32_at(out, dst + f.text_off, f.endian_big);
            let e = plan.edits.iter().find(|e| e.records.contains(&i)).unwrap();
            if e.new_text.is_some() {
                assert!(tp as usize >= f.pool_len, "re-pointed row {i} points into the appended region");
            } else {
                assert_eq!(tp, 0xFFFF_FFFF, "nulled row {i}");
            }
        }
        out_rec += 1;
    }
    // Pool verbatim at the shifted position; appended bytes after it; EOF CRC last.
    assert_eq!(&out[f.pool_start + shift..f.pool_start + shift + f.pool_len], &input[f.pool_start..f.pool_start + f.pool_len], "pool copied verbatim");
    assert!(verify(out).unwrap().iter().all(|c| c.matches()), "CRC scopes");
}

// ---------------------------------------------------------------------------------------------

/// T1a: the preserving serializer with an empty plan is the identity on every committed anchor.
#[test]
fn empty_plan_is_byte_identical_on_every_committed_dictionary() {
    let mut n = 0;
    for rel in anchors() {
        let bytes = read(&rel);
        let plan = plan_growth(&bytes, &[]).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(plan.is_noop());
        let out = apply_growth(&bytes, &plan).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(out.bytes, bytes, "{rel}");
        assert!(out.changed_ranges.is_empty() && out.crc_scopes_changed.is_empty(), "{rel}");
        n += 1;
    }
    assert_eq!(n, 25);
}

/// The order classes of the committed anchors match the census (research 20 B2): NHL 08/09/10
/// have descents, everything else is class A.
#[test]
fn order_classes_match_the_census() {
    for rel in anchors() {
        let d = LocDictionary::parse(&read(&rel)).unwrap();
        let class = OrderClass::of(&d.rows);
        let expect_b = ["nhl08", "nhl09", "nhl10"].iter().any(|g| rel.contains(g));
        match (&class, expect_b) {
            (OrderClass::A, false) => {}
            (OrderClass::B { descents }, true) => assert!(!descents.is_empty(), "{rel}"),
            _ => panic!("{rel}: class {} unexpected", class.as_str()),
        }
    }
}

/// T2/T3/T6/T9/T10: inserts at the sorted position (below min, above max, in the middle), text
/// growth, nulls and a non-ASCII text — on the PS4 NHL 18 anchor (LE, with the shipped equal-hash
/// pair) and the Xbox Legacy anchor (BE), fully accounted.
#[test]
fn inserts_growth_and_nulls_are_accounted_on_a_class_a_dictionary() {
    for rel in ["anchors/ps4/nhl18/databases/eng_us.db", "anchors/xbox360/nhllegacy/databases/nhl_eng_us.db"] {
        let bytes = read(rel);
        let d = LocDictionary::parse(&bytes).unwrap();
        let min = d.rows.iter().map(|r| r.hash).min().unwrap();
        let max = d.rows.iter().map(|r| r.hash).max().unwrap();
        // Find keys hashing below the min and above the max, and one in the middle.
        let mut low = None;
        let mut high = None;
        let mut mid = None;
        for i in 0..100_000u32 {
            let k = format!("STUDIO_NEW_KEY_{i}");
            let h = loc_hash(&k);
            if d.rows.iter().any(|r| r.hash == h) {
                continue;
            }
            if h < min && low.is_none() {
                low = Some(k.clone());
            } else if h > max && high.is_none() {
                high = Some(k.clone());
            } else if h > min && h < max && mid.is_none() {
                mid = Some(k.clone());
            }
            if low.is_some() && high.is_some() && mid.is_some() {
                break;
            }
        }
        let (low, high, mid) = (low.unwrap(), high.unwrap(), mid.unwrap());
        // An existing key with text to grow, and one to null.
        let grow = d.rows.iter().find(|r| r.text.as_deref().is_some_and(|t| t.len() > 3 && t.is_ascii())).unwrap().key.clone().unwrap();
        let nul = d.rows.iter().rev().find(|r| r.key.is_some() && r.text.is_some()).unwrap().key.clone().unwrap();
        let old_grow_text = d.text(&grow).unwrap().to_string();
        let longer = format!("{old_grow_text} EXTENDED BY STUDIO");
        let ops = vec![
            GrowthOp::InsertKey { key: mid.clone(), text: Some("Middle".into()) },
            GrowthOp::InsertKey { key: high.clone(), text: Some("Top".into()) },
            GrowthOp::InsertKey { key: low.clone(), text: None },
            GrowthOp::TextGrow { key: grow.clone(), new_text: longer.clone(), all_sharers: false },
            GrowthOp::SetNull { key: nul.clone() },
        ];
        let plan = plan_growth(&bytes, &ops).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(plan.class, OrderClass::A);
        assert!(!plan.studio_touched);
        // Sorted by hash: low (position 0), mid, high (position = rows).
        assert_eq!(plan.inserts.iter().map(|p| p.key.as_str()).collect::<Vec<_>>(), [low.as_str(), mid.as_str(), high.as_str()]);
        assert_eq!(plan.inserts[0].position, 0);
        assert_eq!(plan.inserts[2].position, d.rows.len());
        assert_eq!(plan.inserts[0].output_record, 0);
        assert_eq!(plan.inserts[2].output_record, d.rows.len() + 2);
        assert!(plan.inserts[0].text_encoded.is_none() && plan.inserts[1].text_encoded.is_some());
        assert_eq!(plan.rows_after(), d.rows.len() + 3);
        assert!(plan.notes.iter().any(|n| n.contains("loader-order-compatibility: unknown")));
        let out = apply_growth(&bytes, &plan).unwrap_or_else(|e| panic!("{rel}: {e}"));
        account(&bytes, &plan, &out.bytes);
        assert_eq!(out.pool_shift, 48);
        assert_eq!(out.inserted_records, [0, plan.inserts[1].output_record, d.rows.len() + 2]);
        let after = LocDictionary::parse(&out.bytes).unwrap();
        assert_eq!(after.len(), d.len() + 3);
        assert_eq!(after.text(&mid), Some("Middle"));
        assert_eq!(after.text(&high), Some("Top"));
        assert!(after.contains_key(&low) && after.text(&low).is_none(), "null text decodes as None");
        assert_eq!(after.text(&grow), Some(longer.as_str()));
        assert!(after.contains_key(&nul) && after.text(&nul).is_none());
        assert_eq!(OrderClass::of(&after.rows), OrderClass::A, "no descent introduced");
        // Hash of the inserted rows equals loc_hash; every existing row keeps hash/key/text (except the two edited).
        let mut j = 0;
        for r in &after.rows {
            if r.key.as_deref() == Some(&low) || r.key.as_deref() == Some(&mid) || r.key.as_deref() == Some(&high) {
                assert_eq!(r.hash, loc_hash(r.key.as_deref().unwrap()));
                continue;
            }
            let b = &d.rows[j];
            assert_eq!((r.hash, &r.key), (b.hash, &b.key));
            if r.key.as_deref() == Some(&grow) {
                assert_eq!(r.text.as_deref(), Some(longer.as_str()));
            } else if r.key.as_deref() == Some(&nul) {
                assert!(r.text.is_none());
            } else {
                assert_eq!(r.text, b.text);
            }
            j += 1;
        }
        // The serializer's accounting names the size words, the CRC words, the three inserted
        // records, the two re-pointed slots and the appended range.
        let f = facts(&bytes);
        let claims = &out.changed_ranges;
        assert!(claims.contains(&(0x08..0x0C)) && claims.contains(&(0x14..0x18)));
        assert!(claims.contains(&(f.block_offset + 0x14..f.block_offset + 0x18)) && claims.contains(&(f.block_offset + 0x24..f.block_offset + 0x28)));
        assert!(claims.contains(&(out.bytes.len() - 4..out.bytes.len())));
        assert!(claims.contains(&out.appended));
        assert_eq!(claims.iter().filter(|r| r.end - r.start == 16).count(), 3, "three inserted records");
        assert_eq!(claims.iter().filter(|r| r.end - r.start == 4 && r.start >= f.record_block && r.end <= f.pool_start + 48).count(), 2, "two re-pointed slots");
        assert!(out.crc_scopes_changed.contains(&"body") && out.crc_scopes_changed.contains(&"file-header"));
        // Apply is bound to the exact bytes.
        assert_eq!(apply_growth(&out.bytes, &plan).unwrap_err().kind, "invalid");
        // A grown file grows again (a second plan on the output).
        let plan2 = plan_growth(&out.bytes, &[GrowthOp::TextGrow { key: mid.clone(), new_text: "Middle, longer".into(), all_sharers: false }]).unwrap();
        let out2 = apply_growth(&out.bytes, &plan2).unwrap();
        account(&out.bytes, &plan2, &out2.bytes);
        assert_eq!(LocDictionary::parse(&out2.bytes).unwrap().text(&mid), Some("Middle, longer"));
    }
}

/// T10: a key with non-ASCII characters hashes from the UTF-16 units of the UTF-8 key (the four
/// shipped NHL 21 rows do); a text with `é` on a tree that has it encodes through the UTF-8 leaves.
#[test]
fn non_ascii_keys_hash_over_utf16_units_and_texts_use_the_tree_leaves() {
    let bytes = read("anchors/ps4/nhl18/databases/eng_us.db");
    let d = LocDictionary::parse(&bytes).unwrap();
    let key = "STUDIO\u{2013}KEY\u{a0}X"; // en dash, NBSP
    assert_eq!(loc_hash(key), loc_hash(key), "deterministic");
    assert_ne!(loc_hash(key), loc_hash("STUDIO-KEY X"));
    let has_e_acute = d.rows.iter().any(|r| r.text.as_deref().is_some_and(|t| t.contains('é')));
    let text = if has_e_acute { "Caf\u{e9} Studio" } else { "Cafe Studio" };
    if d.rows.iter().any(|r| r.hash == loc_hash(key)) {
        return; // astronomically unlikely; the collision refusal is T4's subject
    }
    let plan = plan_growth(&bytes, &[GrowthOp::InsertKey { key: key.into(), text: Some(text.into()) }]).unwrap();
    let out = apply_growth(&bytes, &plan).unwrap();
    account(&bytes, &plan, &out.bytes);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.text(key), Some(text));
    assert_eq!(after.rows[plan.inserts[0].output_record].hash, loc_hash(key));
}

/// T4: a new key whose hash equals any existing row's hash is refused naming the row(s) — the
/// shipped PS4 `TXT_STATS_RANK` pair included — while parse and O-T on the same file succeed; an
/// exact existing key is refused as `exists`; two new keys with one hash are refused.
#[test]
fn hash_collisions_and_existing_keys_are_refused_and_never_block_the_file() {
    let bytes = read("anchors/ps4/nhl16/databases/eng_us.db");
    let d = LocDictionary::parse(&bytes).unwrap();
    // The shipped pair: ' TXT_STATS_RANK' and 'TXT_STATS_RANK' share a hash.
    let pair: Vec<usize> = d.rows.iter().enumerate().filter(|(_, r)| r.hash == 0x59D4_32E3).map(|(i, _)| i).collect();
    assert_eq!(pair.len(), 2, "the shipped equal-hash pair");
    // A synthetic key with that hash: the case-folded variant 'txt_stats_rank' (0xDF masking).
    let r = plan_growth(&bytes, &[GrowthOp::InsertKey { key: "txt_stats_rank".into(), text: Some("x".into()) }]).unwrap_err();
    assert_eq!(r.kind, "hash-collision");
    assert!(r.reason.contains(&format!("record {}", pair[0])) && r.reason.contains(&format!("record {}", pair[1])), "{}", r.reason);
    // Exact existing key.
    let r = plan_growth(&bytes, &[GrowthOp::InsertKey { key: "TXT_STATS_RANK".into(), text: Some("x".into()) }]).unwrap_err();
    assert_eq!(r.kind, "exists");
    // Two new keys with equal hashes (case variants).
    let r = plan_growth(&bytes, &[GrowthOp::InsertKey { key: "STUDIO_ONE".into(), text: None }, GrowthOp::InsertKey { key: "studio_one".into(), text: None }]).unwrap_err();
    assert_eq!(r.kind, "hash-collision");
    // The shipped duplicate never blocks the file: O-T on the pair's second row and on any row works.
    let key = d.rows[pair[1]].key.clone().unwrap();
    let plan = plan_growth(&bytes, &[GrowthOp::TextGrow { key: key.clone(), new_text: "Rank (Studio)".into(), all_sharers: false }]).unwrap();
    let out = apply_growth(&bytes, &plan).unwrap();
    account(&bytes, &plan, &out.bytes);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.rows[pair[1]].text.as_deref(), Some("Rank (Studio)"));
    assert_eq!(after.rows[pair[0]].text, d.rows[pair[0]].text, "the other member of the pair is untouched");
    assert_eq!(OrderClass::of(&after.rows), OrderClass::A, "the equal-hash run is intact");
}

/// T5: Xbox NHL 10 is class B (descents): insertion is refused naming the descent rows; text
/// growth on the same file succeeds with the record block byte-identical except the edited row's
/// pointer, and the descent set unchanged.
#[test]
fn class_b_refuses_insertion_but_grows_text() {
    let bytes = read("anchors/xbox360/nhl10/databases/nhl_eng_us.db");
    let d = LocDictionary::parse(&bytes).unwrap();
    let OrderClass::B { descents } = OrderClass::of(&d.rows) else { panic!("NHL 10 is class B") };
    assert!(!descents.is_empty());
    let r = plan_growth(&bytes, &[GrowthOp::InsertKey { key: "STUDIO_NEW".into(), text: Some("X".into()) }]).unwrap_err();
    assert_eq!(r.kind, "baseline-order-anomalous");
    assert!(r.reason.contains(&descents[0].to_string()), "{}", r.reason);
    // O-T (upper-case tree): grows.
    let row = d.rows.iter().find(|r| r.text.as_deref().is_some_and(|t| t.len() > 2 && t.chars().all(|c| c.is_ascii_uppercase() || c == ' '))).unwrap();
    let key = row.key.clone().unwrap();
    let longer = format!("{} LONGER", row.text.as_deref().unwrap());
    let plan = plan_growth(&bytes, &[GrowthOp::TextGrow { key: key.clone(), new_text: longer.clone(), all_sharers: false }]).unwrap_or_else(|e| panic!("{e}"));
    assert!(matches!(plan.class, OrderClass::B { .. }));
    let out = apply_growth(&bytes, &plan).unwrap();
    account(&bytes, &plan, &out.bytes);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.text(&key), Some(longer.as_str()));
    assert_eq!(OrderClass::of(&after.rows), OrderClass::B { descents }, "descent set unchanged");
    // The lowercase text the tree cannot encode is a rebuild refusal.
    let r = plan_growth(&bytes, &[GrowthOp::TextGrow { key, new_text: "lower case".into(), all_sharers: false }]).unwrap_err();
    assert_eq!(r.kind, "needs-rebuild");
    assert!(r.reason.contains("'l'"), "{}", r.reason);
}

/// T7: repoint → growth. From one LOC-W1 baseline where rows A and B share B's text:
/// (a) O-T on A "this row only" → A gets the appended string, B keeps the old one;
/// (b) O-T on A "all sharers" (from the same baseline) → both point at one appended string, the
///     old string stays in the pool (preserved, now unreachable).
#[test]
fn repoint_then_growth_honours_the_sharer_choice() {
    let bytes = read("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db");
    let plan = plan_repoint(&bytes, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
    let baseline = apply_repoint(&bytes, &plan).unwrap().bytes;
    let d = LocDictionary::parse(&baseline).unwrap();
    assert_eq!(d.text("TXT_LEAGUE_NORWAY"), Some("Rest Of World"));
    let a = d.rows.iter().position(|r| r.key.as_deref() == Some("TXT_LEAGUE_NORWAY")).unwrap();
    let b = d.rows.iter().position(|r| r.key.as_deref() == Some("TXT_ROW_LEAGUE")).unwrap();
    // (a)
    let p = plan_growth(&baseline, &[GrowthOp::TextGrow { key: "TXT_LEAGUE_NORWAY".into(), new_text: "Norway and the Nordic countries".into(), all_sharers: false }]).unwrap();
    assert!(p.studio_touched);
    assert_eq!(p.edits[0].sharers.iter().map(|s| s.record).collect::<Vec<_>>(), [b]);
    assert_eq!(p.edits[0].records, [a]);
    assert!(p.notes.iter().any(|n| n.contains("they keep the old string")), "{:?}", p.notes);
    let out = apply_growth(&baseline, &p).unwrap();
    account(&baseline, &p, &out.bytes);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.rows[a].text.as_deref(), Some("Norway and the Nordic countries"));
    assert_eq!(after.rows[b].text.as_deref(), Some("Rest Of World"));
    let f = facts(&baseline);
    let ptr = |bytes: &[u8], i: usize| u32_at(bytes, f.record_block + i * 16 + f.text_off, f.endian_big);
    assert_eq!(ptr(&out.bytes, b), ptr(&baseline, b), "B still points at the old string");
    // (b) from the same baseline.
    let p = plan_growth(&baseline, &[GrowthOp::TextGrow { key: "TXT_LEAGUE_NORWAY".into(), new_text: "Everyone Else".into(), all_sharers: true }]).unwrap();
    assert_eq!(p.edits[0].records, [a, b]);
    let out = apply_growth(&baseline, &p).unwrap();
    account(&baseline, &p, &out.bytes);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.rows[a].text.as_deref(), Some("Everyone Else"));
    assert_eq!(after.rows[b].text.as_deref(), Some("Everyone Else"));
    assert_eq!(ptr(&out.bytes, a), ptr(&out.bytes, b), "one appended string, shared");
    // The old string is still in the pool bytes (preserved, unreachable).
    let old_ptr = ptr(&baseline, b) as usize;
    assert_eq!(&out.bytes[f.pool_start + old_ptr..f.pool_start + old_ptr + 4], &baseline[f.pool_start + old_ptr..f.pool_start + old_ptr + 4]);
}

/// T8 (P half): a LOC-W2 output (zeroed slack inside the pool) grows; the slack bytes are copied
/// verbatim and the new strings sit after the physical pool end.
#[test]
fn retext_then_growth_keeps_the_zeroed_slack() {
    let bytes = read("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db");
    let plan = plan_retext(&bytes, "TXT_LEAGUE_NORWAY", "Nor").unwrap();
    let retexted = apply_retext(&bytes, &plan).unwrap().bytes;
    let p = plan_growth(&retexted, &[GrowthOp::InsertKey { key: "STUDIO_AFTER_RETEXT".into(), text: Some("After".into()) }]).unwrap();
    let out = apply_growth(&retexted, &p).unwrap();
    account(&retexted, &p, &out.bytes);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.text("TXT_LEAGUE_NORWAY"), Some("Nor"));
    assert_eq!(after.text("STUDIO_AFTER_RETEXT"), Some("After"));
    assert!(out.appended.start >= p.pool_start + 16 + p.pool_len, "appended after the physical pool end");
}

/// T9: an already-null row set to null is a no-op (byte-identical output); T12: capacities.
#[test]
fn null_noop_and_capacity_refusals() {
    let bytes = read("anchors/ps4/nhl17/databases/eng_us.db");
    let d = LocDictionary::parse(&bytes).unwrap();
    let null_key = d.rows.iter().find(|r| r.key.is_some() && r.text.is_none()).unwrap().key.clone().unwrap();
    let p = plan_growth(&bytes, &[GrowthOp::SetNull { key: null_key }]).unwrap();
    assert!(p.is_noop());
    let out = apply_growth(&bytes, &p).unwrap();
    assert_eq!(out.bytes, bytes);
    let r = plan_growth(&bytes, &[GrowthOp::InsertKey { key: "K".repeat(101), text: None }]).unwrap_err();
    assert_eq!(r.kind, "capacity");
    let r = plan_growth(&bytes, &[GrowthOp::InsertKey { key: "STUDIO_BIG".into(), text: Some("T".repeat(4001)) }]).unwrap_err();
    assert_eq!(r.kind, "capacity");
    let r = plan_growth(&bytes, &[GrowthOp::TextGrow { key: "TXT_STATS_RANK".into(), new_text: "T".repeat(4001), all_sharers: false }]).unwrap_err();
    assert_eq!(r.kind, "capacity");
    assert_eq!(plan_growth(&bytes, &[GrowthOp::InsertKey { key: "A\0B".into(), text: None }]).unwrap_err().kind, "invalid");
    assert_eq!(plan_growth(&bytes, &[GrowthOp::TextGrow { key: "NO_SUCH_KEY_STUDIO".into(), new_text: "x".into(), all_sharers: false }]).unwrap_err().kind, "not-found");
    // A corrupt input (one CRC word flipped) is refused by verify before anything is planned.
    let mut bad = bytes.clone();
    let n = bad.len();
    bad[n - 1] ^= 0xFF;
    assert_eq!(plan_growth(&bad, &[]).unwrap_err().kind, "verify");
}
