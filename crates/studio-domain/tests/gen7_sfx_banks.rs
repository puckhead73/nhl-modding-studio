//! Gen7 `.abk` / `.ast` sound banks (25 Sep 2026): the listing, the picker tree, the framing and
//! the diagnostic decode against the **real loose trees** the research measured (R89 / R91 / R92),
//! pinned to the research's own per-bank counts (`r89-abk-bank-tree.json`) and its preview vector
//! #1 (`PREVIEW_REVIEW.md`: PS3 NHL 15 `playercalls.abk` record 0 → 33 207 samples, 22 050 Hz, mono).
//! Read-only over `X:`; nothing is written, listened to or game-verified.
use std::path::Path;

use studio_domain::gen7_sfx_banks::{bank_tree, coverage, list_banks, preview_entry, preview_part};
use studio_domain::Platform;

const PS3_15: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 15\PS3_GAME\USRDIR";
const X360_15: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15";

#[test]
fn coverage_names_the_measured_populations_and_refuses_the_rest_in_words() {
    assert!(coverage("nhl15", Platform::Ps3).is_ok());
    assert!(coverage("nhllegacy", Platform::Xbox).is_ok());
    assert!(coverage("nhllegacy-recompiled", Platform::Recomp).is_ok());
    for g in ["nhl09", "nhl10", "nhl12", "nhl13"] {
        assert!(coverage(g, Platform::Ps3).unwrap().starts_with("R98 / R100"), "{g}: archive-backed PS3");
    }
    // Receipt 36: Xbox 360 NHL 09 is a measured population since research R131 (count rule).
    assert!(coverage("nhl09", Platform::Xbox).unwrap().contains("R131"));
    assert!(coverage("nhl08", Platform::Ps3).is_err());
    let e = coverage("nhl21", Platform::Ps4).unwrap_err().to_string();
    assert!(e.contains("PS4"), "{e}");
    assert!(coverage("nhl08", Platform::Xbox).is_err());
}

#[test]
#[ignore = "real gen7 trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn ps3_nhl15_banks_list_tree_frame_and_decode_to_the_research_pins() {
    let root = Path::new(PS3_15);
    let l = list_banks("nhl15", Platform::Ps3, root).unwrap();
    assert_eq!(l.banks.len(), 217, "R89: 217 loose PS3 NHL 15 banks");
    assert!(
        l.banks.iter().all(|b| b.problem.is_none()),
        "{:?}",
        l.banks
            .iter()
            .filter(|b| b.problem.is_some())
            .map(|b| (&b.name, &b.problem))
            .collect::<Vec<_>>()
    );
    let records: usize = l.banks.iter().map(|b| b.records).sum();
    let streams: usize = l.banks.iter().map(|b| b.stream_records).sum();
    let lists: usize = l.banks.iter().map(|b| b.lists).sum();
    assert_eq!(
        (records, streams),
        (5_099, 3_682),
        "R89 §2 PS3 NHL 15 record totals"
    );
    // R89 §2: 524 sample lists (an empty list in the record-less `100_goal.abk` is not counted).
    assert_eq!(lists, 524, "R89 §2 PS3 NHL 15 sample lists");
    assert!(
        l.banks
            .iter()
            .filter(|b| b.ast.is_some())
            .all(|b| b.ast_id_matches == Some(true)),
        "T5 id match on every bank with an .ast"
    );
    assert_eq!(
        l.banks.iter().filter(|b| b.ast.is_some()).count(),
        14,
        "R89: 14 .ast files"
    );

    // crowds.abk — the discriminating vector (R89 §3).
    let t = bank_tree("nhl15", Platform::Ps3, root, "crowds.abk").unwrap();
    assert_eq!(
        t.bank_sha256,
        "4382317d3de2c754cf477274ea3ade7aef5b9fe40746e8c6ec28247dfe7a4594"
    );
    assert_eq!(
        (t.records, t.stream_records, t.ram_records, t.lists, t.names),
        (204, 86, 118, 44, 311)
    );
    assert_eq!(t.ast_id, 0x460e_d680);
    assert_eq!(t.ast_id_matches, Some(true));
    assert_eq!(t.handler_nodes, 81);
    let loops = t
        .pickers
        .iter()
        .flat_map(|p| &p.entries)
        .filter(|e| e.kind == "stream-loop")
        .count();
    assert_eq!(loops, 3, "three loop bodies (records 0 / 80 / 88)");
    let reset: Vec<&studio_domain::gen7_sfx_banks::Gen7SfxPicker> = t
        .pickers
        .iter()
        .filter(|p| p.owners.iter().any(|o| o == "base_value_reset_send"))
        .collect();
    assert_eq!(
        reset.len(),
        3,
        "the three ambience loops hang off base_value_reset_send"
    );
    let mut loop_records: Vec<u16> = reset
        .iter()
        .flat_map(|p| p.entries.iter().map(|e| e.record))
        .collect();
    loop_records.sort_unstable();
    assert_eq!(loop_records, vec![0, 80, 88]);
    let hi = t
        .pickers
        .iter()
        .find(|p| {
            p.owners.iter().any(|o| o == "hiAmbColorSamplePicker02_msg")
                && p.entries.first().map(|e| e.record) == Some(21)
        })
        .unwrap();
    // The R89 JSON's `owner_examples` show the first eight records of a list; the list itself holds nine (21..=29).
    let hi_records: Vec<u16> = hi.entries.iter().map(|e| e.record).collect();
    assert_eq!(
        &hi_records[..8],
        &(21..=28).collect::<Vec<u16>>()[..],
        "{hi_records:?}"
    );
    assert!(
        t.pickers
            .iter()
            .flat_map(|p| &p.entries)
            .filter(|e| e.kind == "stream")
            .all(|e| e.framing.as_deref()
                == Some("framed; block sample words sum to the record's count")),
        "every one-shot stream frames to its count"
    );
    assert!(t
        .pickers
        .iter()
        .flat_map(|p| &p.entries)
        .filter(|e| e.kind == "stream-loop")
        .all(|e| e
            .framing
            .as_deref()
            .is_some_and(|f| f.starts_with("loop stub"))));
    let weights: std::collections::BTreeMap<u8, usize> = t
        .pickers
        .iter()
        .flat_map(|p| &p.entries)
        .fold(Default::default(), |mut m, e| {
            *m.entry(e.weight).or_default() += 1;
            m
        });
    assert_eq!(weights.get(&100), Some(&186));
    assert_eq!(weights.get(&90), Some(&18));

    // 000_goal.abk — one ram record owned by serviceShutdown_msg, no .ast (R89 §3).
    let g = bank_tree("nhl15", Platform::Ps3, root, "000_goal.abk").unwrap();
    assert_eq!((g.records, g.ram_records, g.lists, g.names), (1, 1, 1, 25));
    assert_eq!(g.ast_id, 0);
    assert!(g.ast_path.is_none());
    assert_eq!(g.pickers[0].owners, vec!["serviceShutdown_msg".to_string()]);
    assert_eq!(g.pickers[0].owner_class.as_deref(), Some("message (_msg)"));
    // The horn's ram record decodes (EALayer3 v1, channel config 0 / 1) to its declared count.
    let horn = preview_entry("nhl15", Platform::Ps3, root, "000_goal.abk", 0).unwrap();
    assert_eq!(horn.kind, "ram");
    assert!(
        horn.wav.len() as u32 >= 44 + horn.num_samples * 2 * horn.channels,
        "{} B for {} × {}",
        horn.wav.len(),
        horn.num_samples,
        horn.channels
    );
    eprintln!(
        "000_goal record 0: {} Hz, {} ch, {} samples, {} granules → {} frames",
        horn.sample_rate, horn.channels, horn.num_samples, horn.granules, horn.rebuilt_frames
    );

    // playercalls.abk record 0 = PREVIEW_REVIEW vector #1.
    let p = bank_tree("nhl15", Platform::Ps3, root, "playercalls.abk").unwrap();
    assert_eq!((p.records, p.stream_records, p.lists), (500, 500, 24));
    assert_eq!(
        p.bank_sha256,
        "317dc321afe5170272212f4949960a7ddfd42f6e17cda32e4b3eb192ad86d24c"
    );
    let board = p
        .pickers
        .iter()
        .find(|k| k.owners.iter().any(|o| o == "boardPlaySamplePicker_msg"))
        .unwrap();
    // The R89 JSON examples show a list's first eight records; this list holds 18 (38..=55).
    let board_records: Vec<u16> = board.entries.iter().map(|e| e.record).collect();
    assert_eq!(&board_records[..8], &(38..=45).collect::<Vec<u16>>()[..], "{board_records:?}");
    let v = preview_entry("nhl15", Platform::Ps3, root, "playercalls.abk", 0).unwrap();
    assert_eq!(
        (v.num_samples, v.sample_rate, v.channels),
        (33_207, 22_050, 1),
        "PREVIEW_REVIEW #1"
    );
    assert_eq!(
        v.wav.len(),
        44 + 33_207 * 2,
        "exactly the declared count in the WAV"
    );
    assert!(v.label.contains("not game-verified"));
    // 26 Sep — record 0 is a loop on channel config 3 (PREVIEW_REVIEW #4–#9): a sub-stream must be
    // chosen; each pair = the stub's 29 PCM samples + the body's 689 959, played once.
    let e = preview_entry("nhl15", Platform::Ps3, root, "crowds.abk", 0).unwrap_err().to_string();
    assert!(e.contains("choose one (1–2)"), "{e}");
    assert!(preview_part("nhl15", Platform::Ps3, root, "crowds.abk", 0, Some(2)).unwrap_err().to_string().contains("does not exist"));
    let loop_entry = t.pickers.iter().flat_map(|p| &p.entries).find(|e| e.record == 0).unwrap();
    assert!(loop_entry.previewable && loop_entry.framing.as_deref().unwrap().contains("stub + body = the record's count"), "{:?}", loop_entry.framing);
    for k in 0..2u16 {
        let p = preview_part("nhl15", Platform::Ps3, root, "crowds.abk", 0, Some(k)).unwrap();
        assert_eq!((p.sub_stream, p.sub_streams, p.loop_start, p.channels), (Some(k), 2, Some(29), 2));
        assert_eq!(p.wav.len(), 44 + 689_988 * 2 * 2, "pair {k}: 29 + 689 959 stereo frames");
        assert!(p.label.contains("inferred de-interleave") && p.label.contains("not game-verified"));
        assert!(p.source.contains("loop stub at 64 + body at"), "{}", p.source);
        eprintln!("NHL 15 crowds 0 pair {}: {} frames, {} granules, loop point 29 — {}", k + 1, (p.wav.len() - 44) / 4, p.granules, p.source);
    }
    // Every config-3 entry of the bank is previewable now (per pair) and still framed.
    let cc3: Vec<_> = t.pickers.iter().flat_map(|p| &p.entries).filter(|e| e.channel_config == 3).collect();
    assert!(!cc3.is_empty() && cc3.iter().all(|e| e.sub_streams == Some(2) && e.previewable), "config-3 entries");
}

#[test]
#[ignore = "real gen7 trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn x360_nhl15_banks_list_frame_and_gate_their_xma_entries() {
    let root = Path::new(X360_15);
    let l = list_banks("nhl15", Platform::Xbox, root).unwrap();
    assert_eq!(l.banks.len(), 217);
    let records: usize = l.banks.iter().map(|b| b.records).sum();
    let streams: usize = l.banks.iter().map(|b| b.stream_records).sum();
    assert_eq!(
        (records, streams),
        (5_088, 3_259),
        "R89 §2 X360 NHL 15 totals"
    );
    let t = bank_tree("nhl15", Platform::Xbox, root, "crowds.abk").unwrap();
    assert_eq!(
        t.bank_sha256,
        "c375f2f5d3ee3353d4e43d76ae561ba9117c284e152e37a064e50c747fd89b91"
    );
    assert_eq!(
        (t.records, t.stream_records, t.ram_records, t.lists, t.names),
        (198, 61, 137, 44, 311)
    );
    assert_eq!(t.bank_platform, "Xbox 360");
    assert_eq!(t.ast_id, 0x0b6f_df00);
    assert_eq!(
        t.pickers
            .iter()
            .flat_map(|p| &p.entries)
            .filter(|e| e.kind == "stream-loop")
            .count(),
        9,
        "nine loop bodies"
    );
    assert!(t
        .pickers
        .iter()
        .flat_map(|p| &p.entries)
        .filter(|e| e.kind == "stream")
        .all(|e| e.framing.as_deref()
            == Some("framed; block sample words sum to the record's count")));
    // Receipts 30 / 33: ram entries (R84) and `.ast` stream entries of the R92-measured shapes
    // preview through an installed XMA2 helper; every other entry refuses with the named reason (no
    // helper, an unmeasured stream shape, a looping or multi-segment ram record, channels).
    let helper = std::env::var_os("NHL_STUDIO_XMA2_HELPER").is_some();
    let (mut streams_ok, mut streams_refused) = (0, 0);
    for e in t.pickers.iter().flat_map(|p| &p.entries) {
        let why = e.preview_refusal.as_deref().unwrap_or("");
        if e.kind.starts_with("stream") {
            if e.previewable {
                streams_ok += 1;
            } else {
                streams_refused += 1;
                assert!(["no XMA2 decoder helper", "no measured EA-XMA stream vector"].iter().any(|w| why.contains(w)), "{} {why}", e.record);
            }
        } else {
            assert!(e.previewable || ["no XMA2 decoder helper", "loop", "segments", "channel config"].iter().any(|w| why.contains(w)), "{} {why}", e.record);
        }
    }
    eprintln!("crowds.abk stream entries: {streams_ok} previewable, {streams_refused} refused (helper named: {helper})");
    assert_eq!(streams_ok > 0, helper, "stream entries preview exactly when a helper is named");
    match preview_entry("nhl15", Platform::Xbox, root, "playercalls.abk", 0) {
        Ok(p) => assert!(helper && p.decoded_frames_per_channel as u32 == p.num_samples),
        Err(e) => assert!(!helper && e.to_string().contains("no XMA2 decoder helper is installed"), "{e}"),
    }
}

/// Research R92 per vector label: part → SHA-256 of that part's decoded 16-bit WAV (`raw` preview:
/// a 44-byte RIFF header + the decoder's PCM).
fn r92_hashes() -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>> {
    let j: serde_json::Value = serde_json::from_slice(&std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/overnight-2026-09-25/data/r92-ast-decode.json")).unwrap()).unwrap();
    let mut out = std::collections::BTreeMap::new();
    for v in j["vectors"].as_array().unwrap() {
        let mut m = std::collections::BTreeMap::new();
        for (part, p) in v["previews"].as_object().unwrap() {
            if let Some(h) = p["raw"]["sha256"].as_str() {
                m.insert(part.clone(), h.to_string());
            }
        }
        out.insert(v["label"].as_str().unwrap().to_string(), m);
    }
    out
}

fn pcm_sha(pcm: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(pcm))
}

/// Receipt 33: the R92 Xbox 360 `.ast` EA-XMA vectors through the domain's stream route with the
/// named helper — decoded count = declared and PCM = the research decode on every part; the loop
/// record (NHL 15 `crowds` 0, channel config 3) per stereo pair as stub + body; unmeasured shapes and
/// titles refuse by name.
#[test]
#[ignore = "real X360 trees under X:/Games/Emulation/Roms and an XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn x360_ast_xma_streams_decode_to_the_r92_pins() {
    assert!(std::env::var_os("NHL_STUDIO_XMA2_HELPER").is_some(), "NHL_STUDIO_XMA2_HELPER names the helper for this case");
    let x = |d: &str| std::path::PathBuf::from(format!(r"X:\Games\Emulation\Roms\XBOX360\{d}"));
    let r92 = r92_hashes();
    let cases: &[(&str, &str, std::path::PathBuf, &str, u16)] = &[
        ("F-x360-15-playercalls-0-xma-mono22k", "nhl15", x(r"NHL 15\NHL 15"), "playercalls.abk", 0),
        ("G-x360-15-crowds-58-xma-stereo-multiblock", "nhl15", x(r"NHL 15\NHL 15"), "crowds.abk", 58),
        ("I2-x360-10-playercalls-first", "nhl10", x(r"NHL 10\NHL 10"), "playercalls.abk", 382),
        ("I4-x360-legacy-playercalls-0", "nhllegacy", x(r"NHL Legacy\NHL Legacy"), "playercalls.abk", 0),
        ("I6-x360-12-sfx-first-stream", "nhl12", x(r"NHL 12\NHL 12"), "sfx.abk", 27),
    ];
    for (label, game, root, bank, record) in cases {
        assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
        let p = preview_entry(game, Platform::Xbox, root, bank, *record).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(p.decoded_frames_per_channel as u32, p.num_samples, "{label}");
        assert_eq!(p.codec, "ea-xma");
        assert!(p.label.contains("not game-verified"));
        // R92 hashed its whole 16-bit WAV file (44-byte header + PCM); Studio's WAV has the same layout.
        assert_eq!(pcm_sha(&p.wav), r92[*label]["stream"], "{label}: WAV = research R92");
        eprintln!("{label}: {} ch, {} samples = declared, WAV = R92 · {}", p.channels, p.num_samples, p.source);
    }
    // The loop record per stereo pair: stub (24 samples) and body decoded separately, joined once.
    let root = x(r"NHL 15\NHL 15");
    let label = "H-x360-15-crowds-0-loop-xma-cc3";
    let e = preview_entry("nhl15", Platform::Xbox, &root, "crowds.abk", 0).unwrap_err().to_string();
    assert!(e.contains("choose one (1–2)"), "{e}");
    for k in 0..2u16 {
        let p = preview_part("nhl15", Platform::Xbox, &root, "crowds.abk", 0, Some(k)).unwrap_or_else(|e| panic!("{label} pair {k}: {e}"));
        assert_eq!((p.channels, p.loop_start, p.sub_stream, p.sub_streams), (2, Some(24), Some(k), 2));
        assert_eq!(p.decoded_frames_per_channel as u32, p.num_samples);
        let pcm: Vec<i16> = p.wav[44..].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
        let stub = 24 * 2;
        let wav = |x: &[i16]| tdb_snd::wav::encode_pcm16(p.sample_rate, 2, x);
        assert_eq!(pcm_sha(&wav(&pcm[..stub])), r92[label][&format!("stub_s{k}")], "pair {k} stub = R92");
        assert_eq!(pcm_sha(&wav(&pcm[stub..])), r92[label][&format!("body_s{k}")], "pair {k} body = R92");
        eprintln!("{label} pair {}: stub 24 + body {} = {} samples, both parts = R92", k + 1, p.num_samples - 24, p.num_samples);
    }
    assert!(preview_part("nhl15", Platform::Xbox, &root, "crowds.abk", 0, Some(2)).unwrap_err().to_string().contains("sub-stream 3 of 2"));
    // A looping stereo stream on NHL Legacy was refused until receipt 56 (R140b swept NHL 10 / 15 / Legacy); it now previews.
    let leg = x(r"NHL Legacy\NHL Legacy");
    let t = bank_tree("nhllegacy", Platform::Xbox, &leg, "crowds.abk").unwrap();
    let s = t.pickers.iter().flat_map(|p| &p.entries).find(|e| e.kind == "stream-loop" && e.channel_config == 1).expect("a Legacy looping stereo stream entry");
    assert!(s.previewable, "{:?}", s.preview_refusal);
    assert!(s.framing.as_deref().unwrap_or("").starts_with("loop stub"), "{:?}", s.framing);
}

/// Owned disposable root: `<tmp>/nhlms-sfx-ident-<pid>/audio/sfx`, removed on drop.
struct Tmp(std::path::PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!("nhlms-sfx-ident-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("audio/sfx")).unwrap();
        Tmp(root)
    }
    fn sfx(&self, name: &str) -> std::path::PathBuf {
        self.0.join("audio/sfx").join(name)
    }
    fn copy(&self, name: &str) {
        std::fs::copy(Path::new(PS3_15).join("audio/sfx").join(name), self.sfx(name)).unwrap();
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn poke(path: &std::path::Path, at: usize) {
    let mut b = std::fs::read(path).unwrap();
    b[at] ^= 0xff;
    std::fs::write(path, b).unwrap();
}

/// Review 2026-09-26 F1: a stream record is framed and decoded only from a stream file whose
/// `S10S` header id equals the bank's word 0x2c — checked on the handle the bytes are read from —
/// and ram records never depend on the sibling file.
#[test]
#[ignore = "real gen7 trees under X:/Games/Emulation/Roms (read only; copies in an owned temp root): select with --ignored"]
fn stream_records_refuse_a_missing_malformed_mismatched_or_replaced_stream_file_and_ram_records_do_not_care() {
    let tmp = Tmp::new("pc");
    tmp.copy("playercalls.abk");
    tmp.copy("playercalls.ast");
    let root = tmp.0.as_path();
    let ast = tmp.sfx("playercalls.ast");

    // Unchanged pair: previews as on the real tree.
    let ok = preview_entry("nhl15", Platform::Ps3, root, "playercalls.abk", 0).unwrap();
    assert_eq!(ok.num_samples, 33_207);
    let t = bank_tree("nhl15", Platform::Ps3, root, "playercalls.abk").unwrap();
    assert_eq!(t.ast_id_matches, Some(true));
    assert!(t.pickers.iter().flat_map(|p| &p.entries).all(|e| e.previewable));

    // Post-listing replacement: the tree above said previewable; flip the id word now.
    poke(&ast, 7);
    let e = preview_entry("nhl15", Platform::Ps3, root, "playercalls.abk", 0).unwrap_err().to_string();
    assert!(e.contains("not this bank's stream file"), "{e}");
    let t = bank_tree("nhl15", Platform::Ps3, root, "playercalls.abk").unwrap();
    assert_eq!(t.ast_id_matches, Some(false));
    for en in t.pickers.iter().flat_map(|p| &p.entries) {
        assert!(!en.previewable, "record {} still previewable", en.record);
        assert!(en.preview_refusal.as_deref().unwrap().contains("not this bank's stream file"));
        assert!(en.blocks.is_none());
    }
    poke(&ast, 7); // restore → works again
    assert!(preview_entry("nhl15", Platform::Ps3, root, "playercalls.abk", 0).is_ok());

    // Malformed header (magic).
    poke(&ast, 0);
    let e = preview_entry("nhl15", Platform::Ps3, root, "playercalls.abk", 0).unwrap_err().to_string();
    assert!(e.contains("S10S"), "{e}");
    poke(&ast, 0);

    // Truncated to less than a header.
    std::fs::write(&ast, b"S10S").unwrap();
    let e = preview_entry("nhl15", Platform::Ps3, root, "playercalls.abk", 0).unwrap_err().to_string();
    assert!(e.contains("shorter than its 8-byte header"), "{e}");

    // Missing.
    std::fs::remove_file(&ast).unwrap();
    let e = preview_entry("nhl15", Platform::Ps3, root, "playercalls.abk", 0).unwrap_err().to_string();
    assert!(e.contains("cannot be opened"), "{e}");
    let t = bank_tree("nhl15", Platform::Ps3, root, "playercalls.abk").unwrap();
    assert!(t.pickers.iter().flat_map(|p| &p.entries).all(|e| !e.previewable));

    // Ram records: a bank with no stream side previews without any sibling file, and a mixed bank's
    // ram entries stay previewable while its stream entries refuse on a mismatched stream file.
    tmp.copy("000_goal.abk");
    assert!(!tmp.sfx("000_goal.ast").exists());
    let horn = preview_entry("nhl15", Platform::Ps3, root, "000_goal.abk", 0).unwrap();
    assert_eq!((horn.kind.as_str(), horn.num_samples), ("ram", 298_234));

    tmp.copy("crowds.abk");
    tmp.copy("crowds.ast");
    let good = bank_tree("nhl15", Platform::Ps3, root, "crowds.abk").unwrap();
    let ram_ok: Vec<u16> = good.pickers.iter().flat_map(|p| &p.entries).filter(|e| e.kind == "ram" && e.previewable).map(|e| e.record).collect();
    assert!(!ram_ok.is_empty(), "a previewable ram entry in crowds");
    poke(&tmp.sfx("crowds.ast"), 7);
    let bad = bank_tree("nhl15", Platform::Ps3, root, "crowds.abk").unwrap();
    for en in bad.pickers.iter().flat_map(|p| &p.entries) {
        if en.kind.starts_with("stream") {
            assert!(!en.previewable && en.preview_refusal.as_deref().unwrap().contains("not this bank's stream file"), "record {}", en.record);
        } else if ram_ok.contains(&en.record) {
            assert!(en.previewable, "ram record {} lost its preview", en.record);
        }
    }
    let r = preview_entry("nhl15", Platform::Ps3, root, "crowds.abk", ram_ok[0]).unwrap();
    assert_eq!(r.kind, "ram");
    eprintln!("F1: ram record {} of crowds.abk previews with a mismatched crowds.ast ({} ram entries previewable)", ram_ok[0], ram_ok.len());
}

/// Review 2026-09-26: the archive-backed PS3 titles list, tree, frame and decode from each title's
/// own members, pinned to R82 §2 (bank members, present records: R98's per-member "records" counts
/// table slots) and R98's census (referenced stream entries = R82 one-shot + loop,
/// stream files, 0 framing refusals) and member hashes, and to R100's decode vectors. R98 is the
/// structural coverage; R100's vectors are the only individually decoded entries.
#[test]
#[ignore = "real gen7 trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn archive_backed_ps3_titles_list_frame_and_decode_from_their_own_members() {
    const PS3: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3";
    struct Title {
        game: &'static str,
        root: String,
        banks: usize,
        records: usize,
        stream_entries: usize,
        stream_files: usize,
        // (bank, stored sha256 per R98, record, declared samples per R100, stream source contains)
        vectors: &'static [(&'static str, &'static str, u16, u32, &'static str)],
    }
    let titles = [
        Title { game: "nhl09", root: format!(r"{PS3}\NHL 09\PS3_GAME\USRDIR"), banks: 186, records: 2_594, stream_entries: 506, stream_files: 7,
            vectors: &[("playercalls.abk", "6f2d34176e6d61dd", 342, 81_768, "data0.big:audio/sfx/streams/playercalls.ast")] },
        Title { game: "nhl10", root: format!(r"{PS3}\NHL 10\PS3_GAME\USRDIR"), banks: 199, records: 2_748, stream_entries: 1_108, stream_files: 7,
            vectors: &[("playercalls.abk", "1c7990772c072c0f", 382, 49_382, "playerCalls.ast")] },
        Title { game: "nhl12", root: format!(r"{PS3}\NHL 12\[BLUS 30791] NHL® 12\PS3_GAME\USRDIR"), banks: 219, records: 4_697, stream_entries: 3_719, stream_files: 15,
            vectors: &[("playercalls.abk", "317dc321afe51702", 0, 33_207, "nocache.big:"), ("sfx.abk", "5695730e280e4180", 92, 133_749, "nocache.big:")] },
        Title { game: "nhl13", root: format!(r"{PS3}\NHL 13\PS3_GAME\USRDIR"), banks: 213, records: 5_117, stream_entries: 3_708, stream_files: 15,
            vectors: &[("playercalls.abk", "317dc321afe51702", 0, 33_207, "cacheaudio.big:")] },
    ];
    for t in &titles {
        let root = Path::new(&t.root);
        let l = list_banks(t.game, Platform::Ps3, root).unwrap();
        let bad: Vec<_> = l.banks.iter().filter(|b| b.problem.is_some()).map(|b| (&b.name, &b.problem)).collect();
        assert!(bad.is_empty(), "{}: {bad:?}", t.game);
        let records: usize = l.banks.iter().map(|b| b.records).sum();
        let with_ast: Vec<_> = l.banks.iter().filter(|b| b.ast.is_some()).collect();
        assert_eq!((l.banks.len(), records, with_ast.len()), (t.banks, t.records, t.stream_files), "{} R98 census", t.game);
        assert!(with_ast.iter().all(|b| b.ast_id_matches == Some(true)), "{}: every stream file id matches", t.game);
        let (mut referenced, mut framed, mut loops) = (0usize, 0usize, 0usize);
        let mut multi: Vec<(String, u16, usize, u32, bool)> = Vec::new();
        let mut wrappers = std::collections::BTreeMap::<String, usize>::new();
        for b in &l.banks {
            let tree = bank_tree(t.game, Platform::Ps3, root, &b.name).unwrap();
            *wrappers.entry(tree.bank_wrapper.clone()).or_default() += 1;
            assert_eq!(tree.bank_copies, 1, "{} {}: one copy (R98 §1)", t.game, b.name);
            for e in tree.pickers.iter().flat_map(|p| &p.entries).filter(|e| e.stream_offset.is_some()) {
                referenced += 1;
                let f = e.framing.as_deref().unwrap_or("");
                if f.starts_with("framed") || f.starts_with("loop stub") {
                    framed += 1;
                } else {
                    panic!("{} {} record {}: {f}", t.game, b.name, e.record);
                }
                if e.kind == "stream-loop" {
                    loops += 1;
                    assert!(e.previewable && f.contains("stub + body = the record's count"), "{} {} loop {}: {f}", t.game, b.name, e.record);
                }
                if let Some(k) = e.sub_streams.filter(|k| *k > 1) {
                    multi.push((b.name.clone(), e.record, k, e.num_samples, e.kind == "stream-loop"));
                }
            }
        }
        assert_eq!((referenced, framed), (t.stream_entries, t.stream_entries), "{}: R98 referenced streams, 0 refused", t.game);
        eprintln!("{}: {} banks, {records} records, {referenced} stream entries framed ({loops} loop), wrappers {wrappers:?}", t.game, l.banks.len());
        // 26 Sep — channel config 3 / 5: R100's named cc 3 vectors, and every cc 5 entry, decode per
        // stereo pair to exactly the declared count; loops as stub + body.
        let cc5: Vec<_> = multi.iter().filter(|m| m.2 == 3).cloned().collect();
        eprintln!("{}: {} multi-stream entries ({} cc 5)", t.game, multi.len(), cc5.len());
        let named: &[(&str, u16, u32)] = match t.game {
            "nhl10" => &[("crowds.abk", 144, 1_270_528)],
            "nhl12" => &[("crowds.abk", 6, 1_270_528)],
            "nhl13" => &[("crowds.abk", 18, 228_096)],
            _ => &[],
        };
        let mut picks: Vec<(String, u16, usize, u32)> = named.iter().map(|(b, r, n)| (b.to_string(), *r, 2, *n)).collect();
        picks.extend(cc5.iter().map(|m| (m.0.clone(), m.1, m.2, m.3)));
        // One loop per title on a multi-stream entry (R100 decoded the 09 / 10 / 12 / 13 stubs).
        picks.extend(multi.iter().filter(|m| m.4).take(1).map(|m| (m.0.clone(), m.1, m.2, m.3)));
        for (bank, record, k, samples) in &picks {
            for part in 0..*k as u16 {
                let p = preview_part(t.game, Platform::Ps3, root, bank, *record, Some(part)).unwrap_or_else(|e| panic!("{} {bank} {record} pair {part}: {e}", t.game));
                assert_eq!(p.num_samples, *samples);
                assert_eq!(p.wav.len() as u64, 44 + *samples as u64 * p.channels as u64 * 2, "{} {bank} {record} pair {part}", t.game);
                eprintln!("{} {bank} record {record} pair {}/{k}: {samples} frames × {} ch, {} granules, loop {:?}", t.game, part + 1, p.channels, p.granules, p.loop_start);
            }
        }
        for (bank, sha, record, samples, src) in t.vectors {
            let tree = bank_tree(t.game, Platform::Ps3, root, bank).unwrap();
            assert!(tree.bank_stored_sha256.starts_with(sha), "{} {bank}: stored sha {} ≠ R98 {sha}", t.game, tree.bank_stored_sha256);
            let p = preview_entry(t.game, Platform::Ps3, root, bank, *record).unwrap();
            assert_eq!(p.num_samples, *samples, "{} {bank} {record}: R100", t.game);
            assert_eq!(p.decoded_frames_per_channel >= *samples as usize, true);
            assert!(p.source.contains(src), "{} {bank}: source {}", t.game, p.source);
            assert!(p.label.contains("not game-verified"));
            eprintln!("{} {bank} record {record}: {} samples from {} ({} blocks)", t.game, p.num_samples, p.source, p.blocks);
        }
    }
}

/// Receipt 30 — the EA-XMA ram route through the XMA2 helper (read-only X360 NHL 15 loose tree).
/// Without a helper the entry stays framed with the missing-helper text; with the helper named by
/// `NHL_STUDIO_XMA2_HELPER` (on this machine only a local oracle build exists — not a Studio
/// artifact) record 0 of `000_goal.abk` previews to exactly its declared 298 234 samples (R84).
#[test]
#[ignore = "real X360 NHL 15 tree under X:/Games/Emulation/Roms and an XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn x360_ram_entries_preview_through_the_helper_or_refuse_without_one() {
    let root = Path::new(r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15");
    assert!(root.is_dir(), "missing (fails, never skips)");
    let helper = std::env::var_os("NHL_STUDIO_XMA2_HELPER").expect("NHL_STUDIO_XMA2_HELPER names the helper for this case");
    let p = preview_entry("nhl15", Platform::Xbox, root, "000_goal.abk", 0).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((p.codec.as_str(), p.num_samples, p.decoded_frames_per_channel, p.channels), ("ea-xma", 298_234, 298_234, 2));
    assert_eq!(p.wav.len(), 44 + 4 * 298_234);
    assert!(p.label.contains("not game-verified"));
    let t = bank_tree("nhl15", Platform::Xbox, root, "000_goal.abk").unwrap();
    assert!(t.pickers.iter().flat_map(|p| &p.entries).any(|e| e.record == 0 && e.previewable));
    // Receipt 59: Legacy Recompiled previews from its own archive members (gen7_xma_ram_residuals); any other title on the
    // Recompiled platform is not a population.
    let e = preview_entry("nhl15", Platform::Recomp, root, "000_goal.abk", 0).unwrap_err().to_string();
    assert!(e.contains("only the Legacy title runs there"), "{e}");
    std::env::remove_var("NHL_STUDIO_XMA2_HELPER");
    let e = preview_entry("nhl15", Platform::Xbox, root, "000_goal.abk", 0).unwrap_err().to_string();
    assert!(e.contains("no XMA2 decoder helper is installed"), "{e}");
    let t = bank_tree("nhl15", Platform::Xbox, root, "000_goal.abk").unwrap();
    let e0 = t.pickers.iter().flat_map(|p| &p.entries).find(|e| e.record == 0).unwrap();
    assert!(!e0.previewable && e0.preview_refusal.as_deref().unwrap().contains("no XMA2 decoder helper is installed"));
    std::env::set_var("NHL_STUDIO_XMA2_HELPER", helper);
}
