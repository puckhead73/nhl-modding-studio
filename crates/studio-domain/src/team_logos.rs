//! Phase 2.4 (C9, overnight 2026-09-20) — **team logo families**: the team's art id
//! (`kOtt.artid` / `exhibitionteams.artid`) → every frontend logo package / registry member the
//! title ships for it, per title, so the Team page can preview and replace each one through the
//! existing texture surface (loose `import_texture`, archive-member placement).
//!
//! The family table is **measured**, not inferred: the developer census
//! `local/runs/overnight-development-2026-09-20/extract_teamlogo_fixtures.py` listed every
//! `teamlogos*` folder of every gen7 source archive / loose tree (Xbox 360 07–Legacy, PS3
//! 08–Legacy) and of every PS4 `inxp_10*.big` (15–21), with member-name pattern, count and id range
//! (private fixture manifest `teamlogos/manifest.json`; receipt 08). Only families whose members are
//! keyed `t<artid>` (or the 13+ letter prefixes `c`/`d`/`s`/`w`/`r`) and whose count matches the
//! title's team art-id space are offered; the small HUT / nation / Spengler sets are named in the
//! notes and not offered (their key is not established as the team art id).
//!
//! What this module does **not** know: which screen draws which family (research unknown → user
//! cards), and whether a PS3 title reads its loose copy or the `cache.big` member (disclosed, as for
//! league logos).

use std::path::Path;

use crate::league_logos::{
    gen7_logo_archive_at, gen7_logo_archive_is_big4, ps4_logo_archive, LogoVariant,
};

/// One measured family: folder (under `fe/ion/artassets/` or `nhl/ui/<tree>/teamlogos/`), the
/// member-name prefix before the art id, and the extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    pub folder: &'static str,
    pub prefix: &'static str,
    pub ext: &'static str,
    /// The highest art id the family ships, when the census showed a **closed** small set (the
    /// 30-member `teamlogosreflection` of 13–Legacy: ids 0–29, the NHL clubs); `None` for the
    /// team-sized families (sparse across the title's art-id space — existence is per file).
    pub max_id: Option<u64>,
}

const fn fam(folder: &'static str, prefix: &'static str, ext: &'static str) -> Family {
    Family {
        folder,
        prefix,
        ext,
        max_id: None,
    }
}

const fn fam_upto(folder: &'static str, prefix: &'static str, ext: &'static str, max_id: u64) -> Family {
    Family {
        folder,
        prefix,
        ext,
        max_id: Some(max_id),
    }
}

/// gen7 families per title (Xbox 360 measured on the archives; PS3 measured on the loose trees /
/// archives — identical folder and name grammar on every title both consoles ship).
fn gen7_families(game_id: &str) -> Option<&'static [Family]> {
    const F07: &[Family] = &[
        fam("teamlogos", "t", "big"),
        fam("teamlogoscalendar", "t", "big"),
        fam("teamlogossmall", "t", "big"),
        fam("teamlogoswide", "t", "big"),
    ];
    const F08_09: &[Family] = &[
        fam("teamlogos", "t", "big"),
        fam("teamlogosbroadcast", "t", "big"),
        fam("teamlogoscalendar", "t", "big"),
        fam("teamlogosdynasty", "t", "big"),
        fam("teamlogossmall", "t", "big"),
        fam("teamlogoswide", "t", "big"),
    ];
    const F10_12: &[Family] = &[
        fam("teamlogos", "t", "big"),
        fam("teamlogoscalendar", "t", "big"),
        fam("teamlogosdynasty", "t", "big"),
        fam("teamlogossmall", "t", "big"),
        fam("teamlogoswide", "t", "big"),
    ];
    const F13_LEGACY: &[Family] = &[
        fam("teamlogos", "t", "big"),
        fam("teamlogoscalendar", "c", "big"),
        fam("teamlogosdynasty", "d", "big"),
        fam("teamlogossmall", "s", "big"),
        fam("teamlogoswide", "w", "big"),
        fam_upto("teamlogosreflection", "r", "big", 29),
    ];
    Some(match game_id {
        "nhl07" => F07,
        "nhl08" | "nhl09" => F08_09,
        "nhl10" | "nhl11" | "nhl12" => F10_12,
        "nhl13" | "nhl14" | "nhl15" | "nhllegacy" | "nhllegacy-recompiled" | "legacyrecomp" => F13_LEGACY,
        _ => return None,
    })
}

/// PS4 families per title: `nhl/ui/<tree>/teamlogos/<folder>/t<artid>.<ext>`.
fn ps4_families(game_id: &str) -> Option<(&'static str, &'static [Family])> {
    const F15: &[Family] = &[
        fam("fullsizeflat", "t", "png"),
        fam("halfsizeflat", "t", "png"),
        fam("jerseylogo", "t", "png"),
        fam("overlaylogo", "t", "png"),
        // a second team-sized PNG set under `playnow/` (235 members; consumer untraced)
        fam("playnow/teamlogos", "t", "png"),
    ];
    const F16: &[Family] = &[
        fam("fullsizeflat", "t", "gnf"),
        fam("halfsizeflat", "t", "gnf"),
        fam("jerseylogo", "t", "gnf"),
        fam("overlaylogo", "t", "gnf"),
        fam("overlaylogohalf", "t", "gnf"),
    ];
    const F17: &[Family] = &[
        fam("fullsizeflat", "t", "gnf"),
        fam("fullsizebw", "t", "gnf"),
        fam("halfsizeflat", "t", "gnf"),
        fam("smallsizeflat", "t", "gnf"),
        fam("overlaylogo", "t", "gnf"),
        fam("overlaylogohalf", "t", "gnf"),
        fam("hutcardbw", "t", "gnf"),
    ];
    const F18: &[Family] = &[
        fam("large", "t", "gnf"),
        fam("largemono", "t", "gnf"),
        fam("extralargemono", "t", "gnf"),
        fam("medium", "t", "gnf"),
        fam("mediumglossy", "t", "gnf"),
        fam("smallglossy", "t", "gnf"),
        fam("extrasmall", "t", "gnf"),
        fam("hutcardbw", "t", "gnf"),
    ];
    const F19: &[Family] = &[
        fam("large", "t", "gnf"),
        fam("largemono", "t", "gnf"),
        fam("extralargemono", "t", "gnf"),
        fam("medium", "t", "gnf"),
        fam("mediumglossy", "t", "gnf"),
        fam("smallglossy", "t", "gnf"),
        fam("extrasmall", "t", "gnf"),
        fam("hutcardbw", "t", "gnf"),
        fam("hutcardmedium", "t", "gnf"),
        fam("hutcardsmalllogo", "t", "gnf"),
        fam("hutcardrare", "t_bronze_", "gnf"),
        fam("hutcardrare", "t_silver_", "gnf"),
        fam("hutcardrare", "t_gold_", "gnf"),
    ];
    const F20: &[Family] = &[
        fam("large", "t", "gnf"),
        fam("largemono", "t", "gnf"),
        fam("extralargemono", "t", "gnf"),
        fam("medium", "t", "gnf"),
        fam("mediumglossy", "t", "gnf"),
        fam("smallglossy", "t", "gnf"),
        fam("extrasmall", "t", "gnf"),
        fam("extrasmallmono", "t", "gnf"),
        fam("hutcardbw", "t", "gnf"),
        fam("hutcardmedium", "t", "gnf"),
        fam("hutcardsmalllogo", "t", "gnf"),
        fam("hutcardrare", "t_bronze_", "gnf"),
        fam("hutcardrare", "t_silver_", "gnf"),
        fam("hutcardrare", "t_gold_", "gnf"),
    ];
    const F21: &[Family] = &[
        fam("large", "t", "gnf"),
        fam("largemono", "t", "gnf"),
        fam("extralargemono", "t", "gnf"),
        fam("medium", "t", "gnf"),
        fam("mediumglossy", "t", "gnf"),
        fam("small", "t", "gnf"),
        fam("smallglossy", "t", "gnf"),
        fam("extrasmall", "t", "gnf"),
        fam("extrasmallmono", "t", "gnf"),
        fam("hutcardbw", "t", "gnf"),
        fam("hutcardmedium", "t", "gnf"),
        fam("hutcardsmalllogo", "t", "gnf"),
        fam("hutcardrare", "t_bronze_", "gnf"),
        fam("hutcardrare", "t_silver_", "gnf"),
        fam("hutcardrare", "t_gold_", "gnf"),
    ];
    Some(match game_id {
        "nhl15" => ("assets", F15),
        "nhl16" => ("kettle_assets", F16),
        "nhl17" => ("kettle_assets", F17),
        "nhl18" => ("kettle_assets", F18),
        "nhl19" => ("kettle_assets", F19),
        "nhl20" => ("kettle_assets", F20),
        "nhl21" => ("kettle_assets", F21),
        _ => return None,
    })
}

/// A team's logo packages / members on this title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamLogoSet {
    pub artid: u64,
    /// `measured` — the family table was measured on this title's shipped archive / tree.
    pub evidence: String,
    pub variants: Vec<LogoVariant>,
    pub notes: Vec<String>,
}

/// Every family path for `artid` under the install / extraction root (`exists` reports whether the
/// loose file is present under the root; the archive the console reads is named per variant).
pub fn team_logo_set(
    game_id: Option<&str>,
    platform: Option<crate::Platform>,
    artid: u64,
    install_root: Option<&Path>,
) -> Option<TeamLogoSet> {
    let g = game_id?;
    let mut notes = Vec::new();
    if platform == Some(crate::Platform::Ps4) {
        let (tree, families) = ps4_families(g)?;
        let mut variants = Vec::new();
        for f in families {
            // `<folder>` under `teamlogos/`, or a full sub-tree (`playnow/teamlogos` on NHL 15).
            let dir = if f.folder.contains('/') {
                f.folder.to_string()
            } else {
                format!("teamlogos/{}", f.folder)
            };
            let rel = format!("nhl/ui/{tree}/{dir}/{}{artid}.{}", f.prefix, f.ext);
            variants.push(LogoVariant {
                family: if f.prefix == "t" {
                    f.folder.to_string()
                } else {
                    format!("{}/{}", f.folder, f.prefix.trim_end_matches('_'))
                },
                exists: install_root.map(|r| crate::league_logos::loose_logo_path(r, &rel).is_some()),
                rel_path: rel,
                archive: ps4_logo_archive(game_id),
            });
        }
        notes.push(match g {
            "nhl15" => "PS4 NHL 15: PNG members of the ui/assets tree (a `kettle_assets/overlaylogo` GNF twin and a `capilano_assets` DDS twin of `overlaylogo` also ship — not offered: the PS4 build reads the kettle/assets trees, the GNF twin's consumer is untraced); which screen draws which family is research unknown".to_string(),
            "nhl16" | "nhl17" => "PS4: GNF members of the kettle_assets tree (inside inxp_10.big on disc — extracted copies are addressed here; the archive route places the member the console reads); `iconsizeflat` (23 members, nations) is not offered; which screen draws which family is research unknown".to_string(),
            _ => "PS4: GNF members of the kettle_assets tree (inside inxp_10*.big on disc); the small HUT sets (`hutcardplayoffs`, `hutcardfreeagency`, `hutcardheritage`, `hutcardstadium`, NHL 21 `hutcardwinternational` / `hutnationsofhockey` / `mediummono`) and the six `spengler_#` members (20–21) are not offered — their key is not established as the team art id; which screen draws which family is research unknown".to_string(),
        });
        return Some(TeamLogoSet {
            artid,
            evidence: "measured".into(),
            variants,
            notes,
        });
    }
    let families = gen7_families(g)?;
    let archive = gen7_logo_archive_at(game_id, platform, install_root);
    let mut variants = Vec::new();
    for f in families {
        let rel = format!(
            "fe/ion/artassets/{}/{}{artid}.{}",
            f.folder, f.prefix, f.ext
        );
        // A closed family (measured id ceiling) is reported absent for an art id beyond it, with
        // or without a root, so no replace is offered against a member the title never ships.
        let beyond = f.max_id.is_some_and(|m| artid > m);
        if beyond {
            notes.push(format!(
                "`{}` ships members for art ids 0–{} only (measured); art id {artid} has none, so nothing is offered there",
                f.folder,
                f.max_id.unwrap_or(0)
            ));
        }
        variants.push(LogoVariant {
            family: f.folder.to_string(),
            exists: if beyond { Some(false) } else { install_root.map(|r| r.join(&rel).is_file()) },
            rel_path: rel,
            archive: if beyond { None } else { archive.clone() },
        });
    }
    notes.push("packages are BIGF with a RefPack'd texture (XPR2 on NHL 12–Legacy, RW4 on 07–11, DDS on PS3); `teamlogosreflection` (13–Legacy) ships 30 members (art ids 0–29, the NHL clubs) and `teamlogosbroadcast` (08–09) 131; a `tblank` / `tcustom` placeholder per folder is not a team member; which screen draws which family is research unknown".into());
    if let Some(a) = &archive {
        if gen7_logo_archive_is_big4(game_id) {
            notes.push(format!("the console reads the packages from the BIG4 archive {a} (patched together with its `.bh` index; NHL 07: nothing is placed — `boot.big` cascade)"));
        } else if platform == Some(crate::Platform::Ps3) {
            notes.push(format!("PS3: the packages ship twice — loose under USRDIR/fe/ion/artassets and as members of {a}; which copy the console reads is research unknown, so the loose replace and the archive replace are separate operations"));
        } else {
            notes.push(format!("the console reads the packages from {a}; a loose file under the install root is an extraction copy"));
        }
    }
    Some(TeamLogoSet {
        artid,
        evidence: "measured".into(),
        variants,
        notes,
    })
}

/// The measured family table for tests / receipts: `(folder, prefix, ext)` per title.
pub fn families_for(game_id: &str, platform: crate::Platform) -> Vec<(String, String, String)> {
    let list: &[Family] = if platform == crate::Platform::Ps4 {
        match ps4_families(game_id) {
            Some((_, f)) => f,
            None => return Vec::new(),
        }
    } else {
        gen7_families(game_id).unwrap_or(&[])
    };
    list.iter()
        .map(|f| {
            (
                f.folder.to_string(),
                f.prefix.to_string(),
                f.ext.to_string(),
            )
        })
        .collect()
}
