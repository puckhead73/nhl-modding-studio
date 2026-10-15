//! `CTX0` v4.5 against the research census (2026-09-21 handoff 01, `r24-ctx0-records.json`) on
//! the seven supported sets (X360 13 / 14 / 15 / Legacy, PS3 14 / 15 / Legacy; read only): the
//! record, pool, item (speech / context), condition and no-trigger counts equal the research's;
//! zero refusals; PS3 = Xbox 360 record for record on 14 / 15 / Legacy; NHL 11 / 12 refuse with
//! the version. A missing tree fails, never skips.

use std::path::{Path, PathBuf};

use tdb_speech::{parse_ctx0, parse_evt0};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:/Games/Emulation/Roms (read only) + the research JSON (35 MB): select with --ignored"]
fn every_supported_set_equals_the_research_census() {
    let m: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-21/data/r24-ctx0-records.json")).unwrap()).unwrap();
    let titles = m["titles"].as_object().unwrap();
    let mut parsed: std::collections::BTreeMap<String, tdb_speech::Ctx0> = Default::default();
    let mut ran = 0;
    for (key, t) in titles {
        let c = &t["ctx0"];
        let path = PathBuf::from(c["file"].as_str().unwrap());
        assert!(path.is_file(), "{key}: {} missing (fails, never skips)", path.display());
        let bytes = std::fs::read(&path).unwrap();
        if c["supported"].as_bool() != Some(true) {
            let e = parse_ctx0(&bytes, None, None).err().map(|e| e.to_string()).unwrap_or_default();
            assert!(e.contains("version 3.1") || e.contains("magic"), "{key}: must refuse by version: {e}");
            eprintln!("{key}: refused as the research does ({e})");
            continue;
        }
        let dir = path.parent().unwrap();
        let speech_ns = parse_evt0(&std::fs::read(dir.parent().unwrap().join("speech").join("pbp_eng_speecheventsubscr.bin")).unwrap()).unwrap().namespace;
        let ctx_ns = parse_evt0(&std::fs::read(dir.join("pbp_eng_contexteventsubscr.bin")).unwrap()).unwrap().namespace;
        assert_eq!(speech_ns as u64, t["speech_namespace"].as_u64().unwrap(), "{key}: speech namespace");
        assert_eq!(ctx_ns as u64, t["context_namespace"].as_u64().unwrap(), "{key}: context namespace");
        let x = parse_ctx0(&bytes, Some(speech_ns), Some(ctx_ns)).unwrap_or_else(|e| panic!("{key}: {e}"));
        let recs = t["records"].as_array().unwrap();
        assert_eq!(x.records.len(), c["count"].as_u64().unwrap() as usize, "{key}: records");
        assert_eq!(x.condition_names as u64, c["pool1_strings"].as_u64().unwrap(), "{key}: pool 1");
        assert_eq!(x.trigger_names.len() as u64, c["pool2_strings"].as_u64().unwrap(), "{key}: pool 2");
        let (mut items, mut speech, mut ctx, mut conds, mut notrig) = (0usize, 0usize, 0usize, 0usize, 0usize);
        for r in &x.records {
            items += r.items.len();
            speech += r.items.iter().filter(|i| i.kind == "speech").count();
            ctx += r.items.iter().filter(|i| i.kind == "context").count();
            conds += r.conditions.len();
            notrig += r.trigger.is_none() as usize;
        }
        let want = |f: &dyn Fn(&serde_json::Value) -> usize| recs.iter().map(f).sum::<usize>();
        assert_eq!(items, want(&|r| r["items"].as_array().unwrap().len()), "{key}: items");
        assert_eq!(speech, want(&|r| r["items"].as_array().unwrap().iter().filter(|i| i["kind"] == "speech").count()), "{key}: speech items");
        assert_eq!(ctx, want(&|r| r["items"].as_array().unwrap().iter().filter(|i| i["kind"] == "context").count()), "{key}: context items");
        assert_eq!(conds, want(&|r| r["conditions"].as_array().unwrap().len()), "{key}: conditions");
        assert_eq!(notrig, recs.iter().filter(|r| r["trigger"].is_null()).count(), "{key}: records without a trigger");
        // Record for record: name, trigger, item names / kinds, condition names / blobs / args.
        for (r, jr) in x.records.iter().zip(recs) {
            assert_eq!(r.name, jr["name"].as_str().unwrap(), "{key} #{}", r.index);
            assert_eq!(r.trigger.as_deref(), jr["trigger"].as_str(), "{key} #{}", r.index);
            let ji = jr["items"].as_array().unwrap();
            assert_eq!(r.items.len(), ji.len());
            for (i, j) in r.items.iter().zip(ji) {
                assert_eq!((i.name.as_str(), i.kind, i.event_id as u64), (j["name"].as_str().unwrap(), j["kind"].as_str().unwrap(), j["event_id"].as_u64().unwrap()), "{key} #{}", r.index);
                assert_eq!(i.params.len(), j["params"].as_array().unwrap().len(), "{key} #{}: params", r.index);
            }
            let jc = jr["conditions"].as_array().unwrap();
            for (cnd, j) in r.conditions.iter().zip(jc) {
                assert_eq!((cnd.name.as_str(), cnd.blob as u64), (j["name"].as_str().unwrap(), j["blob"].as_u64().unwrap()), "{key} #{}: condition", r.index);
                assert_eq!(cnd.args.len(), j["args"].as_array().unwrap().len());
            }
            if let Some(b) = jr["slot2_block"].as_array() {
                assert_eq!(r.slot2_block.map(|x| x.to_vec()), Some(b.iter().map(|v| v.as_u64().unwrap() as u32).collect::<Vec<_>>()), "{key} #{}: slot-2 block", r.index);
            } else {
                assert!(r.slot2_block.is_none(), "{key} #{}", r.index);
            }
        }
        eprintln!("{key}: {} records, {items} items ({speech} speech + {ctx} context), {conds} conditions, {notrig} without a trigger, {} unreferenced pool-1 names", x.records.len(), x.unreferenced_condition_names);
        parsed.insert(key.clone(), x);
        ran += 1;
    }
    assert_eq!(ran, 7, "the seven supported sets ran");
    for game in ["nhl14", "nhl15", "legacy"] {
        let a = &parsed[&format!("ps3-{game}")];
        let b = &parsed[&format!("x360-{game}")];
        assert_eq!(a.records, b.records, "{game}: PS3 = Xbox 360 record for record");
    }
}

/// `CTX0` v3.1 (NHL 11 / 12) against the research census (2026-09-21 handoff 16,
/// `r41-ctx0-v31-records.json`) on the three `pbp_eng` sets: records, triggers / no-trigger,
/// items typed speech / context / other-module, conditions and parameter counts equal the
/// research; PS3 = Xbox 360 on NHL 11 record for record (names, triggers, items, conditions).
#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:/Games/Emulation/Roms (read only) + the research JSON: select with --ignored"]
fn v31_sets_equal_the_research_census() {
    use tdb_speech::{parse_ctx0_v31, parse_moir};
    let m: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-21/data/r41-ctx0-v31-records.json")).unwrap()).unwrap();
    let trees = m["trees"].as_object().unwrap();
    let mut parsed: std::collections::BTreeMap<String, tdb_speech::Ctx0> = Default::default();
    for (tree, t) in trees {
        let s = &t["sets"]["pbp_eng"];
        let path = PathBuf::from(s["file"].as_str().unwrap());
        assert!(path.is_file(), "{tree}: {} missing (fails, never skips)", path.display());
        let dir = path.parent().unwrap();
        let cm = parse_moir(&std::fs::read(dir.join("pbp_eng_contexteventdef.csi")).unwrap()).unwrap();
        let sm = parse_moir(&std::fs::read(dir.parent().unwrap().join("speech").join("pbp_eng_speecheventdef.csi")).unwrap()).unwrap();
        let x = parse_ctx0_v31(&std::fs::read(&path).unwrap(), Some(&sm), Some(&cm)).unwrap_or_else(|e| panic!("{tree}: {e}"));
        let c = &s["checks"];
        let n = |k: &str| c[k].as_u64().unwrap_or(0) as usize;
        assert_eq!(x.records.len(), n("records"), "{tree}: records");
        assert_eq!(x.records.iter().filter(|r| r.trigger.is_some()).count(), n("triggers"), "{tree}: triggers");
        assert_eq!(x.records.iter().filter(|r| r.trigger.is_none()).count(), n("no_trigger"), "{tree}: no trigger");
        let items: usize = x.records.iter().map(|r| r.items.len()).sum();
        assert_eq!(items, n("items"), "{tree}: items");
        assert_eq!(x.records.iter().flat_map(|r| &r.items).filter(|i| i.kind == "speech").count(), n("speech_items"), "{tree}: speech items");
        assert_eq!(x.records.iter().flat_map(|r| &r.items).filter(|i| i.kind == "context").count(), n("raise_items"), "{tree}: raised triggers");
        let other: usize = s["other_module_items"].as_object().map(|o| o.values().map(|v| v.as_u64().unwrap() as usize).sum()).unwrap_or(0);
        assert_eq!(x.records.iter().flat_map(|r| &r.items).filter(|i| i.kind == "other-module").count(), other, "{tree}: other-module items");
        assert_eq!(x.records.iter().map(|r| r.conditions.len()).sum::<usize>(), n("conditions"), "{tree}: conditions");
        assert_eq!(x.records.iter().flat_map(|r| &r.items).map(|i| i.params.len()).sum::<usize>(), n("params"), "{tree}: parameters");
        // The recorded sample (first 400 records): names, triggers, item names, condition names.
        for (r, jr) in x.records.iter().zip(s["records_data"].as_array().unwrap()) {
            assert_eq!(r.name, jr["name"].as_str().unwrap(), "{tree} #{}", r.index);
            assert_eq!(r.trigger.as_deref(), jr["trigger"]["name"].as_str(), "{tree} #{}", r.index);
            if let Some(it) = jr["item"].as_object() {
                assert_eq!(r.items[0].name, it["name"].as_str().unwrap(), "{tree} #{}", r.index);
                assert_eq!(r.items[0].params.len(), it["params"].as_array().unwrap().len(), "{tree} #{}", r.index);
            }
            let jc = jr["conditions"].as_array().unwrap();
            assert_eq!(r.conditions.len(), jc.len());
            for (cnd, j) in r.conditions.iter().zip(jc) {
                assert_eq!((cnd.name.as_str(), cnd.blob as u64, cnd.args.len()), (j["name"].as_str().unwrap(), j["blob"].as_u64().unwrap(), j["args"].as_array().unwrap().len()), "{tree} #{}", r.index);
            }
        }
        eprintln!("{tree}: {} records, {items} items, {} conditions — equal to the research census", x.records.len(), n("conditions"));
        parsed.insert(tree.clone(), x);
    }
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed["ps3-nhl11"].records, parsed["x360-nhl11"].records, "NHL 11: PS3 = Xbox 360 record for record");
}
