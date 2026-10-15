//! `baseline-inventory` — generate, check or bundle the Studio's baseline inventory (ONB-002 §2, §4).
//!
//! The **approval table** below is the reviewed statement of which committed anchor file is admitted
//! as the fresh-start baseline of which shipping target, with its provenance and the witness for it.
//! Nothing is inferred from a folder name: every admitted file is hashed, cross-checked against the
//! audited anchors manifest (`docs/audits/data/anchors-manifest.csv`), and save files are probed
//! with the same codecs the Studio edits through. Gaps are recorded as gaps, never filled from a
//! neighbouring title or platform.
//!
//! ```sh
//! cargo run -p studio-api --bin baseline-inventory -- --write            # baselines/INVENTORY.json
//! cargo run -p studio-api --bin baseline-inventory -- --check            # exit 1 on drift
//! cargo run -p studio-api --bin baseline-inventory -- --bundle <dir>     # <dir>/INVENTORY.json + files/
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::baselines::{
    self, Component, Gap, GeneratedFrom, Inventory, TargetBaseline, INVENTORY_FILE,
    INVENTORY_SCHEMA, ROLE_DICTIONARY, ROLE_DICTIONARY_META, ROLE_MAIN_DB, ROLE_META,
    ROLE_ROSTER_SAVE, ROLE_ROSTER_SAVE_AUX, STATUS_AVAILABLE, STATUS_PARTIAL, STATUS_UNAVAILABLE,
};
use studio_domain::{anchor_platform_dir, Platform, SHIPPING_TARGETS};

const VERIFIED: &str = "verified-vanilla";
const SCHEMA_X: &str = "cross-platform-schema";
const DERIVED: &str = "derived";

/// How one component of one target is admitted.
enum Admit {
    /// From this anchors-relative path, with a provenance status and its witness.
    From(&'static str, &'static str, &'static str),
    /// Not bundled, for this reason.
    Gap(&'static str),
}

struct Approval {
    game_id: &'static str,
    platform: Platform,
    main_db: Admit,
    meta: Admit,
    dictionary: Admit,
    dictionary_meta: Admit,
    /// The roster save's container directory (anchors-relative) and its data file name.
    roster_save: Result<(&'static str, &'static str, &'static str, &'static str), &'static str>,
}

const W_REG1_PS4: &str = "REG-1 (league-parity-batch-2026-09-18 §1–2): anchor re-hashed against the installed title's own archive member";
const W_REG1_PS3: &str = "REG-1 (league-parity-batch-2026-09-18 §1–2): anchor re-hashed against the PS3 disc's USRDIR file";
const W_ANCHOR_ONLY: &str = "anchors manifest (docs/audits/anchors-manifest.md): Xbox 360 NHL 07–15 have no installed counterpart; the anchor is the only vanilla witness";
const W_LEGACY_X360: &str = "REG-1: re-hashed against the Xbox 360 Legacy loose tree and cacheboot.big member";
const W_R128: &str = "handoff R128 (research 2026-09-26 §21): the registered Recompiled install's cacheboot.big member db/nhlng.db hashes 105d2186… = this Xbox 360 Legacy anchor; scope is that install";
const W_SAVE_ANCHOR: &str = "committed vanilla save container (anchors manifest savefile inventory); codec probe recorded on this file";
const W_PS3_10_META: &str = "PS3 NHL 10 ships no meta XML; the committed ps3/nhl10 nhlng-meta.xml is the Xbox 360 NHL 10 anchor's schema (byte-identical file, 6c335a62…), admitted because the two titles' nhlng.db are byte-identical (anchors manifest)";

fn gen7_x360(game: &'static str, dir: &'static str, witness: &'static str, roster: (&'static str, &'static str)) -> Approval {
    Approval {
        game_id: game,
        platform: Platform::Xbox,
        main_db: Admit::From(leak(format!("{dir}/databases/nhlng.db")), VERIFIED, witness),
        meta: Admit::From(leak(format!("{dir}/databases/nhlng-meta.xml")), VERIFIED, witness),
        dictionary: Admit::From(leak(format!("{dir}/databases/nhl_eng_us.db")), VERIFIED, witness),
        dictionary_meta: Admit::From(leak(format!("{dir}/databases/nhl_eng_us-meta.xml")), VERIFIED, witness),
        roster_save: Ok((leak(format!("{dir}/savefiles/{}", roster.0)), roster.1, VERIFIED, W_SAVE_ANCHOR)),
    }
}

fn gen7_ps3(game: &'static str, dir: &'static str, dict: Admit, dict_meta: Admit, roster: (&'static str, &'static str)) -> Approval {
    Approval {
        game_id: game,
        platform: Platform::Ps3,
        main_db: Admit::From(leak(format!("{dir}/databases/nhlng.db")), VERIFIED, W_REG1_PS3),
        meta: if game == "nhl10" {
            Admit::From("ps3/nhl10/databases/nhlng-meta.xml", SCHEMA_X, W_PS3_10_META)
        } else {
            Admit::From(leak(format!("{dir}/databases/nhlng-meta.xml")), VERIFIED, W_REG1_PS3)
        },
        dictionary: dict,
        dictionary_meta: dict_meta,
        roster_save: Ok((leak(format!("{dir}/savefiles/{}", roster.0)), roster.1, VERIFIED, W_SAVE_ANCHOR)),
    }
}

fn ps4(game: &'static str, dict: Admit, roster: Result<(&'static str, &'static str), &'static str>) -> Approval {
    let dir = leak(format!("ps4/{game}"));
    Approval {
        game_id: game,
        platform: Platform::Ps4,
        main_db: Admit::From(leak(format!("{dir}/databases/nhl_game.db")), VERIFIED, W_REG1_PS4),
        meta: Admit::From(leak(format!("{dir}/databases/nhl_game-meta.xml")), VERIFIED, W_REG1_PS4),
        dictionary: dict,
        dictionary_meta: Admit::From(leak(format!("{dir}/databases/eng_us-meta.xml")), VERIFIED, W_REG1_PS4),
        roster_save: roster.map(|(d, f)| (leak(format!("{dir}/savefiles/{d}")), f, VERIFIED, W_SAVE_ANCHOR)),
    }
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn incompatible_roster(mut approval: Approval) -> Approval {
    // Verified during the NHL07 mirror-guard regression sweep: these NHL08
    // fixtures also share zero canonical tables with their own main databases.
    approval.roster_save = Err("the committed roster save shares no database tables with this game's main database and cannot mirror its edits. The project opens the game database alone until a compatible roster save is supplied");
    approval
}

fn approvals() -> Vec<Approval> {
    const PS3_DICT_ALIAS: &str = "the committed PS3 anchor nhl_eng_us.db is the Xbox 360 file (REG-1); the genuine PS3 disc dictionary is a private fixture (NHL_STUDIO_FIXTURES databases/<title>/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db) and is not bundled. The project opens without a dictionary; names show as keys until one is attached";
    const PS3_DICT_META_ABSENT: &str = "no nhl_eng_us-meta.xml is committed for this PS3 title; the dictionary attaches without it";
    let dict_ps3 = |dir: &'static str| Admit::From(leak(format!("{dir}/databases/nhl_eng_us.db")), VERIFIED, W_REG1_PS3);
    let dict_meta_ps3 = |dir: &'static str| Admit::From(leak(format!("{dir}/databases/nhl_eng_us-meta.xml")), VERIFIED, W_REG1_PS3);
    let dict_ps4 = |g: &'static str| Admit::From(leak(format!("ps4/{g}/databases/eng_us.db")), VERIFIED, W_REG1_PS4);
    vec![
        // ---- Xbox 360 (anchor-only titles, plus Legacy re-hashed against the install)
        Approval {
            game_id: "nhl07",
            platform: Platform::Xbox,
            main_db: Admit::From("xbox360/nhl07/databases/NHL07NG.db", VERIFIED, W_ANCHOR_ONLY),
            meta: Admit::From("xbox360/nhl07/databases/NHL07NG-meta.xml", VERIFIED, W_ANCHOR_ONLY),
            dictionary: Admit::From("xbox360/nhl07/databases/nhl_eng_us.db", VERIFIED, W_ANCHOR_ONLY),
            dictionary_meta: Admit::From("xbox360/nhl07/databases/nhl_eng_us-meta.xml", VERIFIED, W_ANCHOR_ONLY),
            roster_save: Err("NHL 07 stores roster changes relative to the game database. Editing this change-list save model is not yet enabled; the project opens the game database alone"),
        },
        incompatible_roster(gen7_x360("nhl08", "xbox360/nhl08", W_ANCHOR_ONLY, ("ROSTER1 20260718064658", "ROSTER1 20260718064658"))),
        gen7_x360("nhl09", "xbox360/nhl09", W_ANCHOR_ONLY, ("ROSTER1 20260717230804", "ROSTER1 20260717230804")),
        gen7_x360("nhl10", "xbox360/nhl10", W_ANCHOR_ONLY, ("ROSTER1 20260627045748", "ROSTER1 20260627045748")),
        gen7_x360("nhl11", "xbox360/nhl11", W_ANCHOR_ONLY, ("ROSTER1 20260627054323", "ROSTER1 20260627054323")),
        gen7_x360("nhl12", "xbox360/nhl12", W_ANCHOR_ONLY, ("ROSTER 20260627052113", "ROSTER 20260627052113")),
        gen7_x360("nhl13", "xbox360/nhl13", W_ANCHOR_ONLY, ("ROSTER 20260627051601", "ROSTER 20260627051601")),
        gen7_x360("nhl14", "xbox360/nhl14", W_ANCHOR_ONLY, ("ROSTER 20260627054746", "ROSTER 20260627054746")),
        gen7_x360("nhl15", "xbox360/nhl15", W_ANCHOR_ONLY, ("ROSTER 20260627055422", "ROSTER 20260627055422")),
        gen7_x360("nhllegacy", "xbox360/nhllegacy", W_LEGACY_X360, ("ROSTER 20260627060052", "ROSTER 20260627060052")),
        // ---- PS3
        incompatible_roster(gen7_ps3("nhl08", "ps3/nhl08", Admit::Gap(PS3_DICT_ALIAS), Admit::Gap(PS3_DICT_ALIAS), ("BLUS30062-002-000 - ROSTER", "SYS-DATA"))),
        gen7_ps3("nhl09", "ps3/nhl09", Admit::Gap(PS3_DICT_ALIAS), Admit::Gap(PS3_DICT_ALIAS), ("BLUS3016802002026 - ROSTER", "SYS-DATA")),
        gen7_ps3("nhl10", "ps3/nhl10", dict_ps3("ps3/nhl10"), Admit::Gap(PS3_DICT_META_ABSENT), ("BLUS303450200 - ROSTER", "SYS-DATA")),
        gen7_ps3("nhl11", "ps3/nhl11", dict_ps3("ps3/nhl11"), dict_meta_ps3("ps3/nhl11"), ("BLUS305680200 - ROSTER", "SYS-DATA")),
        gen7_ps3("nhl12", "ps3/nhl12", dict_ps3("ps3/nhl12"), dict_meta_ps3("ps3/nhl12"), ("BLUS307910200 - ROSTER", "SYS-DATA")),
        gen7_ps3("nhl13", "ps3/nhl13", dict_ps3("ps3/nhl13"), dict_meta_ps3("ps3/nhl13"), ("BLUS310030200 - ROSTER", "SYS-DATA")),
        gen7_ps3("nhl14", "ps3/nhl14", dict_ps3("ps3/nhl14"), dict_meta_ps3("ps3/nhl14"), ("BLUS311790200 - ROSTER", "SYS-DATA")),
        gen7_ps3("nhl15", "ps3/nhl15", dict_ps3("ps3/nhl15"), dict_meta_ps3("ps3/nhl15"), ("BLUS314360200 - ROSTER", "SYS-DATA")),
        gen7_ps3("nhllegacy", "ps3/nhllegacy", dict_ps3("ps3/nhllegacy"), dict_meta_ps3("ps3/nhllegacy"), ("BLUS315400200 - ROSTER", "SYS-DATA")),
        // ---- PS4
        ps4(
            "nhl15",
            dict_ps4("nhl15"),
            Ok(("84f8102f - ROSTER", "DATA")),
        ),
        ps4("nhl16", dict_ps4("nhl16"), Ok(("84f8102f - ROSTER", "DATA"))),
        ps4("nhl17", dict_ps4("nhl17"), Ok(("RosterCustom0", "DATA"))),
        ps4("nhl18", dict_ps4("nhl18"), Ok(("RosterCustom0", "DATA"))),
        ps4(
            "nhl19",
            Admit::Gap("no eng_us.db is committed for NHL 19 (anchors manifest); the NHL 19 base and patch-56 dictionaries exist only as private fixtures (databases/nhl19-ps4-cusa11126/…/eng_us.db) and are not bundled. The project opens without a dictionary"),
            Ok(("RosterCustom0", "DATA")),
        ),
        Approval {
            game_id: "nhl20",
            platform: Platform::Ps4,
            main_db: Admit::Gap("the committed anchors/ps4/nhl20 database set is NHL 19 base bytes under an NHL 20 path (REG-1, anchors manifest). Genuine NHL 20 (CUSA15439 01.00) nhl_game.db 1e200478… and nhl_game-meta.xml 2fac0e73… exist only as private fixtures (databases/nhl20-ps4-cusa15439/inxp_00.big/…) and are not bundled; the alias is never bundled as NHL 20"),
            meta: Admit::Gap("see main_db: the committed nhl_game-meta.xml is NHL 19 base"),
            dictionary: Admit::Gap("the committed eng_us.db is the NHL 19 base dictionary (REG-1); NHL 20's own 0fc95ec8… is a private fixture, not bundled"),
            dictionary_meta: Admit::Gap("see dictionary"),
            roster_save: Err("no vanilla roster save is committed for NHL 20 (BeAGM0, BeAPro0, Profile0, Season0, ThreeOnThree0 only)"),
        },
        ps4(
            "nhl21",
            Admit::Gap("no eng_us.db is committed for NHL 21; its own dictionary 24693e2f… (CUSA18182 inxp_00.big #1156) is a private fixture by coordinator decision (not committed) and is not bundled. The project opens without a dictionary"),
            Err("no vanilla roster save is committed for NHL 21 (BeAGM0, BeAPro0, Profile0, Season0, ThreeOnThree0 only; none of these is a roster container). The project opens the database alone; a roster save cannot be produced from it"),
        ),
        // ---- Legacy Recompiled: Xbox 360 Legacy anchors, by measured identity (R128)
        Approval {
            game_id: "legacyrecomp",
            platform: Platform::Recomp,
            main_db: Admit::From("xbox360/nhllegacy/databases/nhlng.db", DERIVED, W_R128),
            meta: Admit::From("xbox360/nhllegacy/databases/nhlng-meta.xml", DERIVED, W_R128),
            dictionary: Admit::From("xbox360/nhllegacy/databases/nhl_eng_us.db", DERIVED, "handoff R128: the Recompiled cacheboot.big is byte-identical to the Xbox 360 Legacy archive (member 3 = this dictionary, f00926ce…)"),
            dictionary_meta: Admit::From("xbox360/nhllegacy/databases/nhl_eng_us-meta.xml", DERIVED, W_R128),
            roster_save: Ok((
                "xbox360/nhllegacy/savefiles/ROSTER 20260627060052",
                "ROSTER 20260627060052",
                DERIVED,
                "handoff R128 §1: the Recompiled saves are raw inner saves of the same layout and size (2 456 120 B) as this Xbox 360 Legacy anchor; the anchor's own bytes are the vanilla witness",
            )),
        },
    ]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let anchors = repo.join("anchors");
    let out = repo.join("baselines").join(INVENTORY_FILE);
    let manifest = read_anchors_manifest(&repo.join("docs/audits/data/anchors-manifest.csv"));
    let inventory = match build(&anchors, &manifest) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("refusing: {e}");
            std::process::exit(2);
        }
    };
    match args.first().map(String::as_str) {
        Some("--write") => {
            let text = serde_json::to_string_pretty(&inventory).unwrap();
            std::fs::create_dir_all(out.parent().unwrap()).unwrap();
            std::fs::write(&out, format!("{text}\n")).unwrap();
            summarise(&inventory);
            println!("wrote {}", out.display());
        }
        Some("--check") => {
            let current = std::fs::read_to_string(&out).unwrap_or_default();
            let fresh = format!("{}\n", serde_json::to_string_pretty(&inventory).unwrap());
            if normalise(&current) == normalise(&fresh) {
                println!("in sync: {}", out.display());
            } else {
                eprintln!("drift: {} differs from the anchors; run --write", out.display());
                std::process::exit(1);
            }
        }
        Some("--bundle") => {
            let dir = PathBuf::from(args.get(1).expect("--bundle <dir>"));
            let files = dir.join(baselines::BUNDLED_FILES_DIR);
            let store = baselines::BaselineStore { inventory: inventory.clone(), inventory_path: out.clone(), files_root: anchors.clone() };
            let mut n = 0usize;
            for t in &inventory.targets {
                for c in &t.components {
                    let dest = files.join(c.path.replace('/', std::path::MAIN_SEPARATOR_STR));
                    if dest.is_file() {
                        continue; // shared components (Recompiled reuses Legacy) copy once
                    }
                    store.copy_verified(c, &dest).unwrap_or_else(|e| panic!("{}: {e}", c.path));
                    n += 1;
                }
            }
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(INVENTORY_FILE), format!("{}\n", serde_json::to_string_pretty(&inventory).unwrap())).unwrap();
            let bundled = baselines::BaselineStore::open_bundled(&dir).unwrap().unwrap();
            let bad = bundled.verify_all();
            assert!(bad.is_empty(), "bundled files failed verification: {bad:?}");
            println!("bundled {n} files into {} (verified)", dir.display());
        }
        _ => {
            eprintln!("usage: baseline-inventory --write | --check | --bundle <dir>");
            std::process::exit(2);
        }
    }
}

fn normalise(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn summarise(inv: &Inventory) {
    for t in &inv.targets {
        let roles: Vec<&str> = t.components.iter().map(|c| c.role.as_str()).collect();
        println!(
            "{:<13} {:<7} {:<12} components {:<3} gaps {}",
            t.game_id,
            t.platform,
            t.status,
            roles.len(),
            t.gaps.iter().map(|g| g.component.as_str()).collect::<Vec<_>>().join(",")
        );
    }
}

/// `relpath → sha256` from the audited anchors manifest.
fn read_anchors_manifest(path: &Path) -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut out = BTreeMap::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() >= 7 {
            out.insert(cols[0].trim_start_matches("anchors/").to_string(), cols[6].trim().to_string());
        }
    }
    out
}

fn build(anchors: &Path, manifest: &BTreeMap<String, String>) -> Result<Inventory, String> {
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(anchors.parent().unwrap())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    let approvals = approvals();
    let mut targets = Vec::new();
    for t in SHIPPING_TARGETS {
        let a = approvals
            .iter()
            .find(|a| a.game_id == t.game_id && a.platform == t.platform)
            .ok_or_else(|| format!("no approval row for {} {:?}", t.game_id, t.platform))?;
        let mut components = Vec::new();
        let mut gaps = Vec::new();
        for (role, admit) in [
            (ROLE_MAIN_DB, &a.main_db),
            (ROLE_META, &a.meta),
            (ROLE_DICTIONARY, &a.dictionary),
            (ROLE_DICTIONARY_META, &a.dictionary_meta),
        ] {
            match admit {
                Admit::From(rel, prov, witness) => {
                    components.push(component(anchors, manifest, role, rel, None, None, prov, witness, false)?)
                }
                Admit::Gap(reason) => gaps.push(Gap { component: role.into(), reason: (*reason).into() }),
            }
        }
        match &a.roster_save {
            Ok((dir, data, prov, witness)) => {
                let container_dir = Path::new(dir).file_name().unwrap().to_string_lossy().into_owned();
                let data_rel = format!("{dir}/{data}");
                components.push(component(anchors, manifest, ROLE_ROSTER_SAVE, &data_rel, Some(&container_dir), Some(data), prov, witness, true)?);
                let mut aux = Vec::new();
                walk(&anchors.join(dir), &mut aux);
                aux.sort();
                for p in aux {
                    let rel_in = p.strip_prefix(anchors.join(dir)).unwrap().to_string_lossy().replace('\\', "/");
                    if rel_in == *data {
                        continue;
                    }
                    let rel = format!("{dir}/{rel_in}");
                    components.push(component(anchors, manifest, ROLE_ROSTER_SAVE_AUX, &rel, Some(&container_dir), Some(&rel_in), prov, witness, false)?);
                }
                let probe = components.iter().find(|c| c.role == ROLE_ROSTER_SAVE).and_then(|c| c.codec.clone()).unwrap();
                if !probe.readable || !probe.writable || probe.kind != "roster" {
                    return Err(format!("{} {:?}: the admitted roster save {data_rel} does not probe as a readable, writable roster ({probe:?})", t.game_id, t.platform));
                }
            }
            Err(reason) => gaps.push(Gap { component: ROLE_ROSTER_SAVE.into(), reason: (*reason).into() }),
        }
        let has = |r: &str| components.iter().any(|c| c.role == r);
        let status = if !has(ROLE_MAIN_DB) || !has(ROLE_META) {
            STATUS_UNAVAILABLE
        } else if gaps.is_empty() {
            STATUS_AVAILABLE
        } else {
            STATUS_PARTIAL
        };
        targets.push(TargetBaseline {
            game_id: t.game_id.into(),
            platform: t.platform.as_str().into(),
            title: format!("{} ({})", t.title, platform_label(t.platform)),
            status: status.into(),
            components,
            gaps,
        });
    }
    Ok(Inventory {
        schema: INVENTORY_SCHEMA.into(),
        generated_from: GeneratedFrom {
            head,
            generator: "crates/studio-api/src/bin/baseline_inventory.rs".into(),
            anchors_manifest: "docs/audits/data/anchors-manifest.csv".into(),
        },
        targets,
    })
}

fn platform_label(p: Platform) -> &'static str {
    match p {
        Platform::Ps3 => "PS3",
        Platform::Xbox => "Xbox 360",
        Platform::Ps4 => "PS4",
        Platform::Recomp => "Recompiled",
    }
}

#[allow(clippy::too_many_arguments)]
fn component(
    anchors: &Path,
    manifest: &BTreeMap<String, String>,
    role: &str,
    rel: &str,
    container_dir: Option<&str>,
    container_relative: Option<&str>,
    provenance: &str,
    witness: &str,
    probe: bool,
) -> Result<Component, String> {
    let path = anchors.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let sha = baselines::sha256_hex(&bytes);
    match manifest.get(rel) {
        Some(m) if *m == sha => {}
        Some(m) => return Err(format!("{rel}: hashes {sha} but the audited anchors manifest records {m}; the anchor changed — regenerate the audit before admitting it")),
        None => return Err(format!("{rel}: not in the audited anchors manifest; an unaudited file is never admitted")),
    }
    Ok(Component {
        role: role.into(),
        path: rel.into(),
        file_name: Path::new(rel).file_name().unwrap().to_string_lossy().into_owned(),
        container_dir: container_dir.map(str::to_string),
        container_relative: container_relative.map(str::to_string),
        len: bytes.len() as u64,
        sha256: sha,
        provenance: provenance.into(),
        witness: witness.into(),
        codec: probe.then(|| baselines::probe_save_codec(&bytes)),
    })
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// Recompiled shares its files with Xbox 360 Legacy: the inventory lists them under both targets.
#[allow(dead_code)]
fn _anchor_dir_note(p: Platform) -> &'static str {
    anchor_platform_dir(p)
}
