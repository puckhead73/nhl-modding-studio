//! Xbox 360 NHL 09 sound banks (receipt 36): listed from the loose `audio/sfx/ram` / `audio/sfx/streams`
//! tree, framed (R98), and previewed through Studio's own XMA2 helper with the **count rule** only —
//! NHL 09's packet headers are a constant variant (R98b) and are not read as frame counts (research
//! R131 = 2026-09-26 handoff 24). The sweep decodes every previewable entry of every bank and must
//! reproduce R131: all 471 non-loop mono / stereo `.ast` stream entries at exactly their declared
//! counts (each `.ast` ships loose and in `data0.big`, byte-identical — accepted as one copy), every
//! single-segment in-bank record exact except the two short ones (`0293_chant.abk` entry 0,
//! `crowds.abk` entry 4), refused by name; two-segment records, the two looping stream entries and the
//! > 2-channel segments listed only. R131 used FFmpeg 8.1.1 as its oracle; this runs the helper named by
//! `NHL_STUDIO_XMA2_HELPER` (Studio's FFmpeg 8.1.3 build, receipt 32). Read-only.
use std::collections::BTreeMap;
use std::path::Path;

use studio_domain::gen7_sfx_banks::{bank_tree, coverage, list_banks, preview_part};
use studio_domain::Platform;

const ROOT: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL 09\NHL 09 (USA)";

#[test]
fn nhl09_xbox_is_a_measured_population_now() {
    let c = coverage("nhl09", Platform::Xbox).unwrap();
    assert!(c.contains("R131"), "{c}");
    assert!(coverage("nhl09", Platform::Recomp).is_err() || coverage("nhl09", Platform::Recomp).unwrap().contains("R131"));
}

#[test]
#[ignore = "real X360 NHL 09 tree under X:/Games/Emulation/Roms and the XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn every_nhl09_record_follows_r131_through_studios_helper() {
    assert!(std::env::var_os("NHL_STUDIO_XMA2_HELPER").is_some(), "NHL_STUDIO_XMA2_HELPER names the helper (fails, never skips)");
    let root = Path::new(ROOT);
    assert!(root.is_dir(), "{ROOT}: missing (fails, never skips)");
    let banks = list_banks("nhl09", Platform::Xbox, root).unwrap();
    assert_eq!(banks.banks.len(), 186, "R98: 186 banks");
    let mut tally: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut refused: Vec<String> = Vec::new();
    let mut not_previewable: BTreeMap<String, usize> = BTreeMap::new();
    for b in &banks.banks {
        let t = bank_tree("nhl09", Platform::Xbox, root, &b.name).unwrap_or_else(|e| panic!("{}: {e}", b.name));
        let mut seen = std::collections::BTreeSet::new();
        for e in t.pickers.iter().flat_map(|p| &p.entries) {
            if !seen.insert(e.record) {
                continue; // a record named by several pickers is decoded once
            }
            let stream = e.kind.starts_with("stream");
            if !e.previewable {
                let why = e.preview_refusal.clone().unwrap_or_default();
                let key = if why.contains("framing refused") { format!("framing refused: {why}") } else if why.contains("looping") { "looping stream (listed only)".into() } else if why.contains("channel config") || why.contains("sub-stream") { "more than two channels (listed only)".into() } else { why };
                *not_previewable.entry(key).or_default() += 1;
                continue;
            }
            match preview_part("nhl09", Platform::Xbox, root, &b.name, e.record, None) {
                Ok(p) => {
                    assert_eq!(p.decoded_frames_per_channel as u32, p.num_samples, "{} record {}", b.name, e.record);
                    *tally.entry(if stream { "stream exact" } else { "in-bank exact" }).or_default() += 1;
                }
                Err(err) => refused.push(format!("{} record {}: {err}", b.name, e.record)),
            }
        }
    }
    eprintln!("tally {tally:?}");
    eprintln!("not previewable {not_previewable:#?}");
    eprintln!("refused {refused:#?}");
    assert_eq!(tally.get("stream exact").copied().unwrap_or(0), 471, "R131: 471 / 471 stream entries exact");
    // R131 counted in-bank *segments* (1 985 mono / stereo); Studio previews single-segment records
    // only (a multi-segment record's join is not measured): 1 901 records exact + 41 two-segment
    // records listed only (82 segments) + the 2 short ones = 1 985.
    assert_eq!(tally.get("in-bank exact").copied().unwrap_or(0), 1_901, "single-segment in-bank records exact");
    let two_segment = not_previewable.iter().filter(|(k, _)| k.contains("2 ram segments")).map(|(_, n)| *n).sum::<usize>();
    assert_eq!(two_segment, 41, "{not_previewable:#?}");
    assert_eq!(1_901 + 2 * two_segment + 2, 1_985, "reconciles with R131's segment count");
    assert_eq!(refused.len(), 2, "{refused:#?}");
    assert!(refused.iter().any(|r| r.starts_with("0293_chant.abk record 0:") && r.contains("273945") && r.contains("288281")), "{refused:#?}");
    assert!(refused.iter().any(|r| r.starts_with("crowds.abk record 4:") && r.contains("676416") && r.contains("705600")), "{refused:#?}");
    assert!(!not_previewable.keys().any(|k| k.starts_with("framing refused")), "{not_previewable:#?}");
}
