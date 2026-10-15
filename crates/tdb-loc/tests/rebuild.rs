//! LOC-W3 — the **canonical rebuild serializer C** (research handoff 20 rev. 3 §4-C), C1/C2
//! proofs: T1b no-op rebuild with the tree kept on every committed dictionary (decoded equality,
//! opaque metadata preserved, every byte difference in an accounted class — for shipped files only
//! the trailing slack, `db_size` and the CRC words), T11 O-G (a symbol the tree lacks → the tree is
//! rebuilt, the requested rows decode, the tree-byte difference is accounted), T8 (C half: a
//! LOC-W2 output's zeroed slack is the identified discarded range), T7 (C: a LOC-W1 shared
//! pointer is emitted once and kept shared), a forced tree rebuild, the tree builder's round trip,
//! and the serializer-mismatch refusals. Nothing here says a loader accepts a rebuilt pool or that
//! a font draws a new symbol (runtime / glyph gates).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tdb_loc::{apply_growth, apply_rebuild, apply_repoint, apply_retext, build_tree_bytes, plan_growth, plan_rebuild, plan_repoint, plan_retext, verify, GrowthOp, LocDictionary, OrderClass};

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

struct Facts {
    endian_big: bool,
    block_offset: usize,
    record_block: usize,
    pool_start: usize,
    text_off: usize,
    key_off: usize,
}

fn facts(bytes: &[u8]) -> Facts {
    let db = tdb_core::TdbFile::parse(bytes).unwrap();
    let t = &db.tables[0];
    let f = |kind: u32| t.fields.iter().find(|f| f.kind == kind).unwrap();
    Facts {
        endian_big: db.endian == tdb_core::Endian::Big,
        block_offset: t.block_offset,
        record_block: t.record_block,
        pool_start: t.record_block + t.max_records as usize * 16,
        text_off: (f(14).bit_offset / 8) as usize,
        key_off: (f(13).bit_offset / 8) as usize,
    }
}

fn u32_at(b: &[u8], at: usize, big: bool) -> u32 {
    let a = [b[at], b[at + 1], b[at + 2], b[at + 3]];
    if big { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) }
}

/// Independent C1 check of a no-op rebuild with the tree kept against its input: header words
/// only where allowed; the pool a prefix of the input pool up to the trailing slack; the row
/// pointers equal to the input's (nothing moved on a shipped file).
fn account_noop(label: &str, input: &[u8], out: &[u8], acc: &tdb_loc::RebuildAccounting) {
    let f = facts(input);
    assert!(!acc.tree_rebuilt, "{label}: tree kept");
    assert_eq!(acc.inserted_records, 0);
    let allowed = |i: usize| (0x08..0x0C).contains(&i) || (0x14..0x18).contains(&i) || (f.block_offset..f.block_offset + 4).contains(&i) || (f.block_offset + 0x14..f.block_offset + 0x18).contains(&i) || (f.block_offset + 0x24..f.block_offset + 0x28).contains(&i);
    for i in 0..f.record_block {
        if !allowed(i) {
            assert_eq!(input[i], out[i], "{label}: header byte {i:#x}");
        }
    }
    // Every discarded range is trailing slack on a shipped file (no unreachable storage).
    assert!(acc.discarded.iter().all(|d| d.kind == "trailing-slack"), "{label}: {:?}", acc.discarded);
    assert!(acc.discarded.len() <= 1);
    let slack = acc.discarded.first().map(|d| d.range.end - d.range.start).unwrap_or(0);
    assert_eq!(acc.pool_len_after + slack, acc.pool_len_before, "{label}: pool shrank by exactly the slack");
    assert!(acc.pool_prefix_identical, "{label}: the repacked pool is the input pool minus the slack");
    assert_eq!(&out[f.pool_start..f.pool_start + acc.pool_len_after], &input[f.pool_start..f.pool_start + acc.pool_len_after], "{label}: pool bytes");
    assert_eq!(out.len(), input.len() - slack, "{label}: length");
    // Records: identical (pointers unchanged because nothing moved).
    assert_eq!(&out[f.record_block..f.pool_start], &input[f.record_block..f.pool_start], "{label}: record block");
    assert_eq!(acc.moved_strings, 0, "{label}");
    assert_eq!(acc.repointed_records, 0, "{label}");
    for c in verify(out).unwrap() {
        assert!(c.matches(), "{label}: CRC {}", c.name);
    }
}

/// T1b on every committed dictionary.
#[test]
fn noop_rebuild_with_the_tree_kept_is_the_input_minus_its_slack_on_every_committed_dictionary() {
    let mut n = 0;
    let mut slack_total = 0usize;
    for rel in anchors() {
        let bytes = read(&rel);
        let plan = plan_rebuild(&bytes, &[], false).unwrap_or_else(|e| panic!("{rel}: {e:?}"));
        assert!(plan.rebuild.is_some());
        assert!(plan.is_noop());
        let out = apply_rebuild(&bytes, &plan).unwrap_or_else(|e| panic!("{rel}: {e:?}"));
        account_noop(&rel, &bytes, &out.bytes, &out.accounting);
        let (a, b) = (LocDictionary::parse(&bytes).unwrap(), LocDictionary::parse(&out.bytes).unwrap());
        assert_eq!(a.rows, b.rows, "{rel}: decoded rows");
        slack_total += out.accounting.pool_len_before - out.accounting.pool_len_after;
        // The preserving serializer still refuses a rebuild plan and vice versa.
        assert_eq!(apply_growth(&bytes, &plan).unwrap_err().kind, "invalid");
        let p_plan = plan_growth(&bytes, &[]).unwrap();
        assert_eq!(apply_rebuild(&bytes, &p_plan).unwrap_err().kind, "invalid");
        n += 1;
    }
    assert_eq!(n, 25);
    eprintln!("no-op rebuild: 25 files, {slack_total} slack bytes discarded in total");
}

/// T11: a text needing `~` (absent from every tree) → the preserving planner refuses
/// `needs-rebuild`, the rebuild planner records the symbol, the output decodes to the requested
/// rows with a rebuilt tree; the rebuilt file then takes a preserving growth (it is a valid input).
#[test]
fn a_new_glyph_rebuilds_the_tree_and_the_requested_rows_decode() {
    let bytes = read("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db");
    let ops = [
        GrowthOp::TextGrow { key: "TXT_LEAGUE_NORWAY".into(), new_text: "Norway ~ Nordics".into(), all_sharers: false },
        GrowthOp::InsertKey { key: "STUDIO_TILDE_KEY".into(), text: Some("Tilde ~ text".into()) },
    ];
    let refused = plan_growth(&bytes, &ops).unwrap_err();
    assert_eq!(refused.kind, "needs-rebuild");
    assert!(refused.reason.contains('~'));
    let plan = plan_rebuild(&bytes, &ops, false).unwrap();
    let rb = plan.rebuild.as_ref().unwrap();
    assert_eq!(rb.new_symbols, vec!['~']);
    assert!(plan.notes.iter().any(|n| n.contains("tree lacks")));
    assert!(plan.notes.iter().any(|n| n.contains("glyph")));
    let out = apply_rebuild(&bytes, &plan).unwrap();
    let acc = &out.accounting;
    assert!(acc.tree_rebuilt);
    assert_ne!(acc.tree_bytes_after, 0);
    assert_eq!(acc.inserted_records, 1);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.text("TXT_LEAGUE_NORWAY"), Some("Norway ~ Nordics"));
    assert_eq!(after.text("STUDIO_TILDE_KEY"), Some("Tilde ~ text"));
    let before = LocDictionary::parse(&bytes).unwrap();
    assert_eq!(after.rows.len(), before.rows.len() + 1);
    // Every other row is unchanged and the order class stays A.
    let pos = after.rows.iter().position(|r| r.key.as_deref() == Some("STUDIO_TILDE_KEY")).unwrap();
    let mut expected = before.rows.clone();
    for r in &mut expected {
        if r.key.as_deref() == Some("TXT_LEAGUE_NORWAY") {
            r.text = Some("Norway ~ Nordics".into());
        }
    }
    expected.insert(pos, after.rows[pos].clone());
    assert_eq!(after.rows, expected);
    assert!(matches!(OrderClass::of(&after.rows), OrderClass::A));
    assert_eq!(after.stats.symbols, before.stats.symbols + 1, "one new leaf");
    // Header opaque; the reserved record bytes are zero as before.
    for c in verify(&out.bytes).unwrap() {
        assert!(c.matches());
    }
    // A rebuilt file is a valid input for the preserving serializer.
    let p2 = plan_growth(&out.bytes, &[GrowthOp::InsertKey { key: "STUDIO_AFTER_REBUILD".into(), text: Some("ok".into()) }]).unwrap();
    let out2 = apply_growth(&out.bytes, &p2).unwrap();
    assert_eq!(LocDictionary::parse(&out2.bytes).unwrap().text("STUDIO_AFTER_REBUILD"), Some("ok"));
}

/// The rebuild honours the growth planner's refusals: class-B insertion, collisions, capacity.
#[test]
fn rebuild_never_bypasses_the_growth_refusals() {
    let b = read("anchors/xbox360/nhl10/databases/nhl_eng_us.db");
    let e = plan_rebuild(&b, &[GrowthOp::InsertKey { key: "STUDIO_NEW~".into(), text: Some("x~".into()) }], false).unwrap_err();
    assert_eq!(e.kind, "baseline-order-anomalous");
    let a = read("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db");
    let e = plan_rebuild(&a, &[GrowthOp::InsertKey { key: "TXT_ROW_LEAGUE".into(), text: Some("~".into()) }], false).unwrap_err();
    assert_eq!(e.kind, "exists");
    let e = plan_rebuild(&a, &[GrowthOp::TextGrow { key: "TXT_ROW_LEAGUE".into(), new_text: "~".repeat(4001), all_sharers: false }], false).unwrap_err();
    assert_eq!(e.kind, "capacity");
    // O-T with a new glyph on a class-B file is allowed (order untouched): the first row's key.
    let first = LocDictionary::parse(&b).unwrap().rows[0].key.clone().unwrap();
    let p = plan_rebuild(&b, &[GrowthOp::TextGrow { key: first.clone(), new_text: "Rest ~ World".into(), all_sharers: false }], false).unwrap();
    let out = apply_rebuild(&b, &p).unwrap();
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.text(&first), Some("Rest ~ World"));
    assert_eq!(OrderClass::of(&after.rows), OrderClass::of(&LocDictionary::parse(&b).unwrap().rows), "class B descents kept");
}

/// T8 (C half): a LOC-W2 output's zeroed slack is the identified discarded range and every
/// string after it moves; the rows decode to the requested result.
#[test]
fn retext_then_rebuild_discards_the_identified_slack_and_accounts_the_moves() {
    let bytes = read("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db");
    let plan = plan_retext(&bytes, "TXT_LEAGUE_NORWAY", "Nor").unwrap();
    let retexted = apply_retext(&bytes, &plan).unwrap().bytes;
    let p = plan_rebuild(&retexted, &[GrowthOp::InsertKey { key: "STUDIO_AFTER_RETEXT".into(), text: Some("After".into()) }], false).unwrap();
    let out = apply_rebuild(&retexted, &p).unwrap();
    let acc = &out.accounting;
    assert!(!acc.tree_rebuilt);
    assert!(acc.discarded.iter().any(|d| d.kind == "unreachable"), "{:?}", acc.discarded);
    assert!(acc.moved_strings > 0);
    assert!(acc.repointed_records > 0);
    assert!(!acc.pool_prefix_identical);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.text("TXT_LEAGUE_NORWAY"), Some("Nor"));
    assert_eq!(after.text("STUDIO_AFTER_RETEXT"), Some("After"));
    assert_eq!(after.rows.len(), LocDictionary::parse(&retexted).unwrap().rows.len() + 1);
    // No slack at all in the output: the pool ends at the last string.
    let p2 = plan_rebuild(&out.bytes, &[], false).unwrap();
    let again = apply_rebuild(&out.bytes, &p2).unwrap();
    assert_eq!(again.bytes, out.bytes, "a rebuilt file rebuilds to itself (no slack left)");
}

/// T7 (C): a LOC-W1 shared string is emitted once and both rows keep pointing at it; then an
/// O-T "this row only" splits the sharing as requested.
#[test]
fn repoint_then_rebuild_preserves_sharing_and_a_split_edit_is_honoured() {
    let bytes = read("anchors/ps4/nhl18/databases/eng_us.db");
    let plan = plan_repoint(&bytes, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
    let baseline = apply_repoint(&bytes, &plan).unwrap().bytes;
    let d = LocDictionary::parse(&baseline).unwrap();
    let a = d.rows.iter().position(|r| r.key.as_deref() == Some("TXT_LEAGUE_NORWAY")).unwrap();
    let b = d.rows.iter().position(|r| r.key.as_deref() == Some("TXT_ROW_LEAGUE")).unwrap();
    let f = facts(&baseline);
    let ptr = |bytes: &[u8], i: usize| u32_at(bytes, f.record_block + i * 16 + f.text_off, f.endian_big);
    assert_eq!(ptr(&baseline, a), ptr(&baseline, b));
    let p = plan_rebuild(&baseline, &[], false).unwrap();
    assert!(p.studio_touched);
    let out = apply_rebuild(&baseline, &p).unwrap();
    let acc = &out.accounting;
    assert_eq!(ptr(&out.bytes, a), ptr(&out.bytes, b), "still one shared string");
    assert!(acc.discarded.iter().any(|d| d.kind == "unreachable"), "the re-pointed-away string is unreachable storage: {:?}", acc.discarded);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.rows, d.rows);
    // Key pointers of unchanged rows are shared as before (none), and a split edit works.
    let p = plan_rebuild(&baseline, &[GrowthOp::TextGrow { key: "TXT_LEAGUE_NORWAY".into(), new_text: "Norway ~".into(), all_sharers: false }], false).unwrap();
    let out = apply_rebuild(&baseline, &p).unwrap();
    assert!(out.accounting.tree_rebuilt);
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.rows[a].text.as_deref(), Some("Norway ~"));
    assert_eq!(after.rows[b].text.as_deref(), Some("Rest Of World"));
    assert_ne!(ptr(&out.bytes, a), ptr(&out.bytes, b));
}

/// A forced tree rebuild on an unchanged alphabet: the rows decode, the tree bytes differ (a
/// different but valid Huffman shape), the accounting says so; the key/text pointers are all
/// re-pointed because every string re-encodes.
#[test]
fn a_forced_tree_rebuild_re_encodes_everything_and_decodes_back() {
    let bytes = read("anchors/ps3/nhl15/databases/nhl_eng_us.db");
    let p = plan_rebuild(&bytes, &[], true).unwrap();
    let out = apply_rebuild(&bytes, &p).unwrap();
    let acc = &out.accounting;
    assert!(acc.tree_rebuilt);
    let (a, b) = (LocDictionary::parse(&bytes).unwrap(), LocDictionary::parse(&out.bytes).unwrap());
    assert_eq!(a.rows, b.rows);
    assert_eq!(a.stats.symbols, b.stats.symbols);
    for c in verify(&out.bytes).unwrap() {
        assert!(c.matches());
    }
}

/// The tree builder: every symbol of the histogram gets a code, the serialised form re-parses to
/// its own length with the measured numbering, the codes are prefix-free, and a string encoded
/// with them decodes through the crate's own decoder (via a synthetic dictionary rebuild).
#[test]
fn tree_builder_round_trips_through_the_parser() {
    let mut hist = BTreeMap::new();
    for (i, b) in b"the quick brown fox jumps over the lazy dog 0123456789".iter().enumerate() {
        *hist.entry(*b).or_insert(0u64) += (i % 7) as u64 + 1;
    }
    let tree = build_tree_bytes(&hist).unwrap();
    assert_eq!(tree.len() % 4, 0);
    assert_eq!(tree.len() / 4, hist.len() - 1, "internal nodes = symbols - 1");
    // Markers: the internal children are numbered by their read order (1-based), leaves 0.
    let mut expected_marker = 1u8;
    for rec in tree.chunks(4) {
        for (m, v) in [(rec[2], rec[3]), (rec[0], rec[1])] {
            if m != 0 {
                assert_eq!(m, expected_marker);
                assert_eq!(v, 0);
                expected_marker += 1;
            } else {
                assert!(hist.contains_key(&v));
            }
        }
    }
    // A one-symbol alphabet and a full byte alphabet still build.
    let mut one = BTreeMap::new();
    one.insert(b'a', 5u64);
    assert_eq!(build_tree_bytes(&one).unwrap().len(), 4);
    let full: BTreeMap<u8, u64> = (0..=255u8).map(|b| (b, 1 + b as u64)).collect();
    let t = build_tree_bytes(&full).unwrap();
    assert_eq!(t.len(), 255 * 4);
}

fn fixture_root() -> PathBuf {
    PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private locale fixtures cannot be resolved"))
}

/// T1b on the 100 loose locale dictionaries (53 Xbox 360 incl. `lng_lg`, 47 PS3) and the six
/// install dictionaries: the no-op rebuild with the tree kept is the input minus its trailing
/// slack on every one, and a new-glyph rebuild decodes on each (O-T on class B, O-K on class A).
#[test]
#[ignore = "private fixtures loc/<platform>-<title>/*.db + loc/manifest.json and databases/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn noop_rebuild_and_new_glyph_rebuild_on_every_private_dictionary() {
    use sha2::Digest;
    let root = fixture_root();
    let loc = root.join("loc");
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(loc.join("manifest.json")).unwrap()).unwrap();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for (dir, entries) in manifest.as_object().unwrap() {
        for (name, meta) in entries.as_object().unwrap() {
            let path = loc.join(dir).join(name);
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            assert_eq!(format!("{:x}", sha2::Sha256::digest(&bytes)), meta["sha256"].as_str().unwrap(), "{dir}/{name}: fixture identity");
            files.push((format!("{dir}/{name}"), bytes));
        }
    }
    for rel in [
        "databases/nhl20-ps4-cusa15439/inxp_00.big/eng_us.db",
        "databases/nhl19-ps4-cusa11126/inxp_00.big/eng_us.db",
        "databases/nhl19-ps4-cusa11126/patch_inxp_56.big/eng_us.db",
        "databases/nhl21-ps4-cusa18182/inxp_00.big/eng_us.db",
        "databases/nhl08-ps3-blus30062/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db",
        "databases/nhl09-ps3-blus30168/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db",
    ] {
        let path = root.join(rel);
        files.push((rel.to_string(), std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))));
    }
    assert_eq!(files.len(), 106);
    let mut slack_total = 0usize;
    let mut rebuilt = 0usize;
    for (label, bytes) in &files {
        let plan = plan_rebuild(bytes, &[], false).unwrap_or_else(|e| panic!("{label}: {e:?}"));
        let out = apply_rebuild(bytes, &plan).unwrap_or_else(|e| panic!("{label}: {e:?}"));
        account_noop(label, bytes, &out.bytes, &out.accounting);
        assert_eq!(LocDictionary::parse(bytes).unwrap().rows, LocDictionary::parse(&out.bytes).unwrap().rows, "{label}");
        slack_total += out.accounting.pool_len_before - out.accounting.pool_len_after;
        // New glyph: `~` is absent from every measured tree; O-T on the first row (any class).
        let d = LocDictionary::parse(bytes).unwrap();
        let first = d.rows[0].key.clone().unwrap();
        let ops = [GrowthOp::TextGrow { key: first.clone(), new_text: "Studio ~ probe".into(), all_sharers: false }];
        let plan = plan_rebuild(bytes, &ops, false).unwrap_or_else(|e| panic!("{label}: {e:?}"));
        let out = apply_rebuild(bytes, &plan).unwrap_or_else(|e| panic!("{label}: {e:?}"));
        let after = LocDictionary::parse(&out.bytes).unwrap();
        assert_eq!(after.text(&first), Some("Studio ~ probe"), "{label}");
        assert_eq!(after.rows.len(), d.rows.len(), "{label}");
        assert_eq!(OrderClass::of(&after.rows).as_str(), OrderClass::of(&d.rows).as_str(), "{label}: order class");
        // `~` is absent from most trees (then the tree is rebuilt); a locale whose tree has it
        // keeps its tree — both are correct outcomes, recorded in the accounting.
        if out.accounting.tree_rebuilt {
            rebuilt += 1;
        }
    }
    eprintln!("private dictionaries: 106 files, {slack_total} slack bytes discarded in total; {rebuilt} trees rebuilt for `~`");
}

// ------------------------------------------------------------------ P1 audit (21 Sep 2026)
//
// Malformed and unusual inputs of the canonical rebuild: a null KEY pointer (the reader keeps it
// distinct from an empty key — the rebuild must too), bytes that changed between plan and apply, a
// CRC-corrupt input, a key pointer aimed at the tree bytes, and a plan whose record count would
// overflow the 16-bit count words. Each is a refusal or an exact preservation, never a panic and
// never a silent semantic change.

fn recrc(bytes: &mut Vec<u8>) {
    let snap = bytes.clone();
    tdb_core::TdbFile::parse(&snap).unwrap().recompute_crcs(bytes);
}

#[test]
fn a_null_key_pointer_survives_the_rebuild_as_null_not_as_an_empty_key() {
    let mut bytes = read("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db");
    let f = facts(&bytes);
    let rec = 5usize;
    let at = f.record_block + rec * 16 + f.key_off;
    bytes[at..at + 4].copy_from_slice(&[0xff; 4]);
    recrc(&mut bytes);
    let before = LocDictionary::parse(&bytes).unwrap();
    assert_eq!(before.rows[rec].key, None, "the reader distinguishes a null key");
    for force in [false, true] {
        let plan = plan_rebuild(&bytes, &[], force).unwrap();
        let out = apply_rebuild(&bytes, &plan).unwrap();
        let after = LocDictionary::parse(&out.bytes).unwrap();
        assert_eq!(after.rows, before.rows, "force_tree={force}: rows incl. the null key");
        assert_eq!(u32_at(&out.bytes, f.record_block + rec * 16 + f.key_off, f.endian_big), 0xffff_ffff);
        for c in verify(&out.bytes).unwrap() {
            assert!(c.matches(), "CRC {}", c.name);
        }
    }
}

#[test]
fn changed_bytes_between_plan_and_apply_are_refused_without_output() {
    let bytes = read("anchors/ps4/nhl18/databases/eng_us.db");
    let plan = plan_rebuild(&bytes, &[GrowthOp::InsertKey { key: "STUDIO_AUDIT_KEY".into(), text: Some("audit ~".into()) }], false).unwrap();
    let mut other = bytes.clone();
    let f = facts(&other);
    other[f.pool_start + 200] ^= 0x01; // one pool bit, no CRC repair
    let e = apply_rebuild(&other, &plan).unwrap_err();
    assert_eq!(e.kind, "invalid");
    assert!(e.reason.contains("no longer match"), "{e}");
    // A repaired-CRC variant is still a different file.
    recrc(&mut other);
    assert_eq!(apply_rebuild(&other, &plan).unwrap_err().kind, "invalid");
    // And the untouched bytes still apply.
    apply_rebuild(&bytes, &plan).unwrap();
}

#[test]
fn a_crc_corrupt_input_is_refused_at_planning() {
    let mut bytes = read("anchors/ps3/nhl12/databases/nhl_eng_us.db");
    let f = facts(&bytes);
    bytes[f.pool_start + 100] ^= 0x80;
    let e = plan_rebuild(&bytes, &[], false).unwrap_err();
    assert_eq!(e.kind, "verify");
    assert!(e.reason.contains("body"), "{e}");
}

#[test]
fn a_key_pointer_inside_the_tree_or_past_the_pool_is_a_parse_error_not_a_panic() {
    let bytes = read("anchors/xbox360/nhl10/databases/nhl_eng_us.db");
    let f = facts(&bytes);
    for bad in [1u32, 0x7fff_fff0] {
        let mut b = bytes.clone();
        let at = f.record_block + 3 * 16 + f.key_off;
        b[at..at + 4].copy_from_slice(&if f.endian_big { bad.to_be_bytes() } else { bad.to_le_bytes() });
        recrc(&mut b);
        assert!(LocDictionary::parse(&b).is_err(), "pointer {bad:#x} must not parse");
        assert!(plan_rebuild(&b, &[], false).is_err());
    }
}

#[test]
fn a_record_count_past_the_16_bit_words_is_refused_at_planning() {
    // The smallest committed dictionary plus enough distinct keys to overflow: refused by the
    // planner's capacity check before any encoding — the count words are u16 in the file.
    let bytes = read("anchors/xbox360/nhl07/databases/nhl_eng_us.db");
    let rows = LocDictionary::parse(&bytes).unwrap().len();
    let need = u16::MAX as usize + 1 - rows;
    let ops: Vec<GrowthOp> = (0..need).map(|i| GrowthOp::InsertKey { key: format!("STUDIO_OVERFLOW_{i:06}"), text: None }).collect();
    let e = plan_rebuild(&bytes, &ops, false).unwrap_err();
    assert!(e.kind == "capacity" || e.kind == "hash-collision", "{e}");
}
