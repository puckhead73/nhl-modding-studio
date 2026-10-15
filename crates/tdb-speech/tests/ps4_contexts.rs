//! P2 (overnight 2026-09-22) — the PS4 `0XTC` context walk against research 2026-09-21 pm handoff
//! 15 (`research-r58-ps4-ctx0`), on the real extracted trees of all seven PS4 titles.
//!
//! The research measured, per title: the file version, how many records the pointer list holds,
//! how many carry a trigger, how many speech and context items resolve, and how many conditions
//! there are. This test reproduces those numbers with Studio's own parser. It is the parity
//! evidence for extending the Contexts card from gen7 NHL 11–Legacy to PS4 NHL 15–21; nothing here
//! claims anything about runtime selection or the `.gas` bytecode.
//!
//! Read-only over `H:` — no copy, no write.

use std::path::{Path, PathBuf};

/// `(title, CUSA, version, records, with_trigger, speech_items, context_items, conditions)` —
/// every figure from handoff 15 §2's table.
const EXPECTED: &[(&str, &str, u32, usize, usize, usize, usize, usize)] = &[
    ("nhl15", "CUSA00561", 3, 3_864, 3_813, 4_005, 515, 15_434),
    ("nhl16", "CUSA01909", 3, 4_396, 4_336, 4_534, 650, 18_565),
    ("nhl17", "CUSA04326", 3, 5_178, 5_119, 5_316, 777, 22_355),
    ("nhl18", "CUSA07569", 3, 2_676, 2_640, 2_692, 552, 10_277),
    ("nhl19", "CUSA11126", 3, 2_998, 2_962, 3_019, 586, 11_790),
    ("nhl20", "CUSA15439", 4, 1_418, 1_397, 1_414, 273, 2_704),
    ("nhl21", "CUSA18182", 4, 2_153, 2_119, 2_147, 479, 4_313),
];

fn speech_dir(cusa: &str) -> PathBuf {
    Path::new(r"H:\Emulators\ShadPS4\Games").join(cusa).join(r"databigs\Extracted\nhl\data\kettle\audio\speech\eng")
}

#[test]
#[ignore = "real extracted PS4 trees under H:/Emulators/ShadPS4/Games (read only): select with --ignored"]
fn every_ps4_title_reproduces_the_research_context_census() {
    let mut covered = 0usize;
    for (title, cusa, version, records, with_trigger, speech_items, context_items, conditions) in EXPECTED {
        let dir = speech_dir(cusa);
        assert!(dir.is_dir(), "{title}: {} missing (fails, never skips)", dir.display());
        let read = |n: &str| std::fs::read(dir.join(n)).unwrap_or_else(|e| panic!("{title}: {n}: {e}"));

        let sm = tdb_speech::parse_moir_le(&read("pbp_speecheventdef.csi")).unwrap_or_else(|e| panic!("{title}: speech MOIR: {e}"));
        let cm = tdb_speech::parse_moir_le(&read("pbp_contexteventdef.csi")).unwrap_or_else(|e| panic!("{title}: context MOIR: {e}"));
        let bytes = read("pbp_contexts.bin");
        assert!(tdb_speech::is_ctx0_ps4(&bytes), "{title}: not a 0XTC container");

        let (ctx, notes) = tdb_speech::parse_ctx0_ps4(&bytes, Some(&sm), Some(&cm)).unwrap_or_else(|e| panic!("{title}: {e}"));

        assert_eq!(notes.version, *version, "{title}: file version");
        assert_eq!(ctx.records.len(), *records, "{title}: records walked");
        assert_eq!(notes.declared_records as usize, records + 2, "{title}: the header counts two more than the pointer list holds");
        assert_eq!(ctx.records.iter().filter(|r| r.trigger.is_some()).count(), *with_trigger, "{title}: records with a trigger");
        let sp = ctx.records.iter().flat_map(|r| &r.items).filter(|i| i.kind == "speech").count();
        let cx = ctx.records.iter().flat_map(|r| &r.items).filter(|i| i.kind == "context").count();
        let un = ctx.records.iter().flat_map(|r| &r.items).filter(|i| i.kind == "unknown-namespace").count();
        assert_eq!(sp, *speech_items, "{title}: speech items");
        assert_eq!(cx, *context_items, "{title}: context items");
        assert_eq!(un, 0, "{title}: handoff 15 §2 records no other namespaces");
        assert_eq!(ctx.condition_names, *conditions, "{title}: conditions");
        assert_eq!(ctx.platform, "ps4");

        // Every trigger name is a `trigger_*` string from pool 2, and every record's own name and
        // item names are non-empty — the resolution the research reports at 100 %.
        assert!(ctx.trigger_names.iter().all(|t| t.starts_with("trigger_")), "{title}: a trigger name is not `trigger_*`");
        assert!(ctx.records.iter().all(|r| !r.name.is_empty()), "{title}: a record has no name");
        assert!(ctx.records.iter().flat_map(|r| &r.items).all(|i| !i.name.is_empty()), "{title}: an item has no name");
        // Kind 1 = triggered, and every record with a trigger is kind 1 (handoff 15 §2).
        for r in &ctx.records {
            let kind = r.w4 >> 16;
            assert_eq!(r.trigger.is_some(), kind == 1, "{title}: record {} kind {kind} vs trigger {:?}", r.index, r.trigger);
        }
        eprintln!("{title}: v{} {} records, {} triggered, {sp} speech + {cx} context items, {} conditions, {} block entries, {} groups", notes.version, ctx.records.len(), with_trigger, ctx.condition_names, notes.block_entries, notes.groups);
        covered += 1;
    }
    assert_eq!(covered, 7, "all seven PS4 titles must be exercised");
}

/// The gen7 big-endian `CTX0` and the PS4 little-endian `0XTC` are not confused for one another,
/// and a truncated / wrong-version file refuses rather than walking garbage.
#[test]
fn the_ps4_walk_refuses_what_is_not_a_measured_0xtc() {
    assert!(!tdb_speech::is_ctx0_ps4(b"CTX0X360"));
    assert!(tdb_speech::parse_ctx0_ps4(b"CTX0X360\x00\x04\x00\x05", None, None).is_err());
    let mut hdr = b"0XTCTTEK".to_vec();
    hdr.extend_from_slice(&9u32.to_le_bytes()); // version 9
    hdr.extend_from_slice(&[0u8; 0x80]);
    let e = tdb_speech::parse_ctx0_ps4(&hdr, None, None).expect_err("version 9 must refuse");
    assert!(e.to_string().contains("version 9"), "{e}");
    let mut short = b"0XTCTTEK".to_vec();
    short.extend_from_slice(&4u32.to_le_bytes());
    assert!(tdb_speech::parse_ctx0_ps4(&short, None, None).is_err(), "a truncated header must refuse");
}
