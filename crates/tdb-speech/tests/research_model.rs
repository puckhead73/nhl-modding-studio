//! The parsers against the research speech model (2026-09-21 handoff 04, `r27-gen7-speech-model.json`)
//! on the real loose `audio/speech` trees of `X:` (read only): for every tree whose inputs are
//! loose (NHL 11–Legacy Xbox 360; NHL 11 / 14 / 15 / Legacy PS3) the counts this crate produces —
//! sentences, phrases, event refs, the FNV slot rule, one fixed pair per phrase, the graffiti
//! table existing for every phrase, the secret tag id, graffiti tables / distinct refs, events /
//! parameters, bank samples — equal the research figures. A missing tree fails (never skips).

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use tdb_speech::*;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:/Games/Emulation/Roms (read only) + the research model JSON: select with --ignored"]
fn counts_equal_the_research_model_on_every_loose_tree() {
    let m: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-21/data/r27-gen7-speech-model.json")).unwrap()).unwrap();
    let trees = m["trees"].as_object().unwrap();
    let mut ran = 0;
    for (key, t) in trees {
        let inputs = t["inputs"].as_object().unwrap();
        let loose = inputs.values().all(|v| v["source"].as_str().is_some_and(|s| s.starts_with("loose:")));
        let form = t["sentences"]["form"].as_str().unwrap_or("");
        if !loose || form != "bin" {
            eprintln!("{key}: {} inputs, {form} form — not a loose binary tree; covered by the archive route later", if loose { "loose" } else { "archive" });
            continue;
        }
        let root = PathBuf::from(t["root"].as_str().unwrap()).join("audio").join("speech");
        assert!(root.is_dir(), "{key}: {} missing (fails, never skips)", root.display());
        let read = |name: &str| std::fs::read(root.join(name)).unwrap_or_else(|e| panic!("{key}: {name}: {e}"));
        let evt = parse_evt0(&read("pbp_eng_speecheventsubscr.bin")).unwrap_or_else(|e| panic!("{key}: EVT0: {e}"));
        let moir = parse_moir(&read("pbp_eng_speecheventdef.csi")).unwrap_or_else(|e| panic!("{key}: MOIR: {e}"));
        let sent = parse_sentences(&read("pbp_eng_sentencedata.bin")).unwrap_or_else(|e| panic!("{key}: SPC0: {e}"));
        let grf = parse_graffiti(&read("pbp_eng_graffitidata.bin")).unwrap_or_else(|e| panic!("{key}: GRF0: {e}"));
        let bank = parse_bank(&read("pbp_eng_samples.sbr")).unwrap_or_else(|e| panic!("{key}: SBKR: {e}"));
        let e = &t["events"];
        assert_eq!(evt.events.len() as u64, e["events"].as_u64().unwrap(), "{key}: events");
        assert_eq!(evt.params.len() as u64, e["params"].as_u64().unwrap(), "{key}: params");
        assert_eq!(moir.entries.len(), evt.events.len(), "{key}: MOIR entries = events");
        let s = &t["sentences"];
        let ver = s["version"].as_array().unwrap();
        assert_eq!((sent.version.0 as u64, sent.version.1 as u64), (ver[0].as_u64().unwrap(), ver[1].as_u64().unwrap()), "{key}: version");
        let ck = &s["checks"];
        assert_eq!(sent.sentences.len() as u64, ck["sentences"].as_u64().unwrap(), "{key}: sentences");
        assert_eq!(sent.sentences.iter().filter(|x| x.slot_ok).count() as u64, ck["fnv_slot_ok"].as_u64().unwrap(), "{key}: FNV slot rule");
        let phrases: Vec<&Phrase> = sent.sentences.iter().flat_map(|x| x.bodies.iter()).flat_map(|b| b.phrases.iter()).collect();
        assert_eq!(phrases.len() as u64, ck["phrases"].as_u64().unwrap(), "{key}: phrases");
        let refs: u64 = phrases.iter().map(|p| p.tags.iter().filter(|t| matches!(t, PhraseTag::EventRef { .. })).count() as u64).sum();
        assert_eq!(refs, ck["refs"].as_u64().unwrap(), "{key}: event refs");
        let one_fixed = phrases.iter().filter(|p| p.tags.iter().filter(|t| matches!(t, PhraseTag::Fixed { .. })).count() == 1).count() as u64;
        assert_eq!(one_fixed, phrases.len() as u64, "{key}: one fixed pair per phrase");
        // Secret tag census and the graffiti table for every phrase.
        let mut secret: BTreeMap<u32, u64> = BTreeMap::new();
        let mut table_exists = 0u64;
        let mut candidates_total = 0u64;
        for p in &phrases {
            for tag in &p.tags {
                if let PhraseTag::Fixed { tag, value } = tag {
                    *secret.entry(*tag).or_default() += 1;
                    if let Some(tb) = grf.table(*tag, *value, p.num_parameters) {
                        table_exists += 1;
                        candidates_total += tb.sample_ids.len() as u64;
                    }
                }
            }
        }
        let want_secret: BTreeMap<u32, u64> = s["secret_tag_census"].as_object().unwrap().iter().map(|(k, v)| (k.parse().unwrap(), v.as_u64().unwrap())).collect();
        assert_eq!(secret, want_secret, "{key}: secret tag census");
        assert_eq!(table_exists, s["resolution_checks"]["fixed_table_exists"].as_u64().unwrap(), "{key}: graffiti table per phrase");
        assert!(candidates_total > 0);
        // Event refs name the event's parameter at the index.
        let ev_params: BTreeMap<&str, &Vec<u32>> = evt.events.iter().map(|e| (e.name.as_str(), &e.params)).collect();
        let mut at_index = 0u64;
        let mut has_event = 0u64;
        for x in &sent.sentences {
            if let Some(par) = ev_params.get(x.name.as_str()) {
                has_event += 1;
                for b in &x.bodies {
                    for p in &b.phrases {
                        for tag in &p.tags {
                            if let PhraseTag::EventRef { tag, event_index } = tag {
                                if par.get(*event_index as usize) == Some(tag) {
                                    at_index += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        let rc = &s["resolution_checks"];
        assert_eq!(has_event, rc["sentence_has_event"].as_u64().unwrap(), "{key}: sentence has event");
        assert_eq!(at_index, rc["ref_tag_equals_event_param_at_index"].as_u64().unwrap(), "{key}: ref tag = event param at index");
        // Graffiti and bank.
        let g = &t["graffiti"];
        assert_eq!(grf.tables.len() as u64, g["tables"].as_u64().unwrap(), "{key}: graffiti tables");
        assert_eq!(grf.malformed as u64, g["malformed"].as_u64().unwrap());
        let distinct: HashSet<u32> = grf.tables.iter().flat_map(|t| t.sample_ids.iter().copied()).collect();
        assert_eq!(distinct.len() as u64, g["refs_distinct"].as_u64().unwrap(), "{key}: distinct refs");
        let bk = &t["bank"];
        assert_eq!(bank.samples.len() as u64, bk["count"].as_u64().unwrap(), "{key}: bank samples");
        let ids: HashSet<u32> = bank.samples.iter().map(|s| s.id).collect();
        assert_eq!(ids.iter().min().copied().unwrap() as u64, bk["id_min"].as_u64().unwrap());
        assert_eq!(ids.iter().max().copied().unwrap() as u64, bk["id_max"].as_u64().unwrap());
        // On the named v2.1 files the tag names come from the entries.
        if grf.named {
            assert!(!grf.tag_names().is_empty(), "{key}: named v2.1 tag names");
        }
        eprintln!("{key}: {} events / {} params, {} sentences ({} slot ok), {} phrases, {} refs ({} at index), secret {:?}, {} graffiti tables / {} distinct samples, bank {} samples; {} candidate samples over all phrases", evt.events.len(), evt.params.len(), sent.sentences.len(), sent.sentences.iter().filter(|x| x.slot_ok).count(), phrases.len(), refs, at_index, secret, grf.tables.len(), distinct.len(), bank.samples.len(), candidates_total);
        ran += 1;
    }
    assert_eq!(ran, 10, "the ten loose binary trees (X360 11–Legacy, PS3 11 / 14 / 15 / Legacy)");
}
