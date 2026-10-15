//! Phase 4 (Equipment, 22 Sep 2026) — the **component set** an equipment id must carry, per
//! family, and whether an assigned id has it. Research 2026-09-21 pm handoff 10 (R52) measured one
//! dominant file set per equipment-id-keyed family on X360 13 / 15 / Legacy and PS3 15 / Legacy
//! (NHL 13 / 15 identical in shape to Legacy):
//!
//! ```text
//! glove/glove_<id>          am, nm, sm, 0_dm, 0_tm, 1_tm, 2_tm        + feglove_<id>: 0_dm, 0_tm, 1_tm, 2_tm
//! goaliepad/goaliepad_<id>  nm, sm, 0_dm, 0_tm, 1_tm, 2_tm            + fegoaliepad_<id>: the same four
//! blocker/blocker_<s>_<id>  per style s ∈ {0, 1}: nm, sm, 0_dm, 0_tm, 1_tm, 2_tm   + feblocker_<s>_<id>: 0_dm, 0_tm, 1_tm, 2_tm
//! trapper/trapper_<s>_<id>  as blocker                                + fetrapper
//! goaliestick/goaliestick_<id>   dm, tm
//! goaliehelmet/goaliehelmet_0_<id>_cm   (shell 0; shells 1–7 optional)
//! ```
//!
//! The meshes are shared per part (keyed by style, not by id), so an id is **complete when it has
//! the family's majority set** and needs no mesh; an id in the research's incomplete list is valid
//! as shipped but lacks named components (usually the third tint map `2_tm`). Sticks, skates and
//! helmets are texture-library families (one `texlib_<id>` container each), and on NHL 11 / 12
//! gloves / pads / blocker / trapper take that form too (R52 §3, handoff 12 addendum). Everything
//! outside those measurements — NHL 07 → 10, PS4 (vault-bound) — is *not measured*, and the check
//! says so rather than inventing a set.
//!
//! Read-only: an incomplete set is information for the operator, never a refusal; a new-id
//! registration (placing the majority set) is not built here.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::equipment_slots::{list_equipment_slot_assets_in, slot_spec};
use crate::project::Platform;

/// What the title's measurements say an id of this slot must carry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SetContract {
    /// A measured component set (R52 §1).
    Measured { source: String },
    /// One `texlib_<id>` container per id (sticks / skates / helmets; NHL 11 / 12 for the rest).
    SingleContainer { source: String },
    /// Nothing measured for this slot on this title / platform.
    NotMeasured { reason: String },
}

impl SetContract {
    pub fn describe(&self) -> String {
        match self {
            Self::Measured { source } => format!("the family's majority component set ({source})"),
            Self::SingleContainer { source } => format!("one `texlib_<id>` container per id ({source})"),
            Self::NotMeasured { reason } => reason.clone(),
        }
    }
}

/// The set contract for a slot on a title / platform, decided from identity alone.
pub fn set_contract(slot_key: &str, game_id: Option<&str>, platform: Platform) -> SetContract {
    if platform == Platform::Ps4 {
        return SetContract::NotMeasured { reason: "PS4 equipment is vault-bound (research 2026-09-20 handoff 09); no per-id file set is measured".into() };
    }
    let r52 = "research 2026-09-21 pm handoff R52 §1, X360 13 / 15 / Legacy and PS3 15 / Legacy";
    let measured = || SetContract::Measured { source: r52.into() };
    let single = |src: &str| SetContract::SingleContainer { source: src.into() };
    // Both NHL11 console discs ship separate helmet/chinstrap meshes and tint maps,
    // with no helmet texlib files. Do not mark every stock helmet incomplete
    // against the texture-library contract used by the other equipment families.
    if game_id == Some("nhl11") && matches!(platform, Platform::Xbox | Platform::Ps3) && slot_key == "helmet" {
        return SetContract::NotMeasured { reason: "NHL11 helmets use separate helmet/chinstrap files and tint maps; the required component set is not yet measured".into() };
    }
    if game_id == Some("nhl12") && matches!(platform, Platform::Xbox | Platform::Ps3) && matches!(slot_key, "gloves" | "helmet") {
        return SetContract::NotMeasured { reason: "NHL12 uses separate glove/helmet maps rather than one texlib per model; the required component set is not yet measured".into() };
    }
    match (game_id, slot_key) {
        (Some("nhl13" | "nhl14" | "nhl15" | "nhllegacy"), "gloves" | "pads" | "blocker" | "trapper" | "goalie-stick" | "goalie-helmet") => measured(),
        (Some("nhl13" | "nhl14" | "nhl15" | "nhllegacy"), "stick" | "skates" | "helmet") => single("R52 §1: texture-library families"),
        (Some("nhl11" | "nhl12"), "gloves" | "pads" | "blocker" | "trapper" | "stick" | "skates" | "helmet") => single("R52 §3 / research 2026-09-21 handoff 12 addendum: NHL 11 / 12 key these families on `texlib_<id>`"),
        (Some("nhl11" | "nhl12"), _) => SetContract::NotMeasured { reason: format!("the `{slot_key}` component set is not measured on NHL 11 / 12 (R52 §3)") },
        (Some(other), _) => SetContract::NotMeasured { reason: format!("the equipment component sets of `{other}` are not measured (R52 covers NHL 13 → Legacy; NHL 11 / 12 by texture library)") },
        (None, _) => SetContract::NotMeasured { reason: "the open project has no recorded title, so no component set can be chosen".into() },
    }
}

/// One component group of a set (the in-game files or the front-end copy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentGroup {
    /// `in-game` / `front-end`.
    pub group: String,
    /// `complete` / `incomplete`.
    pub status: String,
    /// Expected stems (no extension) that were found.
    pub present: Vec<String>,
    /// Expected stems that were not found.
    pub missing: Vec<String>,
}

/// The expected stems of an id under a measured set, as `(group, stem)`.
pub fn expected_stems(slot_key: &str, id: u32) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let tints = ["0_tm", "1_tm", "2_tm"];
    match slot_key {
        "gloves" => {
            for k in ["am", "nm", "sm", "0_dm"] {
                out.push(("in-game", format!("glove_{id}_{k}")));
            }
            for t in tints {
                out.push(("in-game", format!("glove_{id}_{t}")));
            }
            out.push(("front-end", format!("feglove_{id}_0_dm")));
            for t in tints {
                out.push(("front-end", format!("feglove_{id}_{t}")));
            }
        }
        "pads" => {
            for k in ["nm", "sm", "0_dm"] {
                out.push(("in-game", format!("goaliepad_{id}_{k}")));
            }
            for t in tints {
                out.push(("in-game", format!("goaliepad_{id}_{t}")));
            }
            out.push(("front-end", format!("fegoaliepad_{id}_0_dm")));
            for t in tints {
                out.push(("front-end", format!("fegoaliepad_{id}_{t}")));
            }
        }
        "blocker" | "trapper" => {
            let p = if slot_key == "blocker" { "blocker" } else { "trapper" };
            for s in 0..2u32 {
                for k in ["nm", "sm", "0_dm"] {
                    out.push(("in-game", format!("{p}_{s}_{id}_{k}")));
                }
                for t in tints {
                    out.push(("in-game", format!("{p}_{s}_{id}_{t}")));
                }
                out.push(("front-end", format!("fe{p}_{s}_{id}_0_dm")));
                for t in tints {
                    out.push(("front-end", format!("fe{p}_{s}_{id}_{t}")));
                }
            }
        }
        "goalie-stick" => {
            out.push(("in-game", format!("goaliestick_{id}_dm")));
            out.push(("in-game", format!("goaliestick_{id}_tm")));
        }
        "goalie-helmet" => out.push(("in-game", format!("goaliehelmet_0_{id}_cm"))),
        _ => {}
    }
    out
}

fn stem_of(file: &str) -> &str {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name)
}

/// Check an id's files (any path form; only the stem is read) against the contract. `None` when
/// the contract is not measured.
pub fn check_components(slot_key: &str, id: u32, files: &[String], contract: &SetContract) -> Option<Vec<ComponentGroup>> {
    let stems: BTreeSet<&str> = files.iter().map(|f| stem_of(f)).collect();
    match contract {
        SetContract::NotMeasured { .. } => None,
        SetContract::SingleContainer { .. } => {
            let stem = format!("texlib_{id}");
            let present = stems.contains(stem.as_str());
            Some(vec![ComponentGroup {
                group: "in-game".into(),
                status: if present { "complete" } else { "incomplete" }.into(),
                present: if present { vec![stem.clone()] } else { Vec::new() },
                missing: if present { Vec::new() } else { vec![stem] },
            }])
        }
        SetContract::Measured { .. } => {
            let mut groups: Vec<ComponentGroup> = Vec::new();
            for (group, stem) in expected_stems(slot_key, id) {
                let g = match groups.iter_mut().find(|g| g.group == group) {
                    Some(g) => g,
                    None => {
                        groups.push(ComponentGroup { group: group.into(), status: "complete".into(), present: Vec::new(), missing: Vec::new() });
                        groups.last_mut().unwrap()
                    }
                };
                if stems.contains(stem.as_str()) {
                    g.present.push(stem);
                } else {
                    g.missing.push(stem);
                    g.status = "incomplete".into();
                }
            }
            Some(groups)
        }
    }
}

/// A family-wide census: every id in the slot's folder and whether it carries the majority set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCensus {
    pub slot: String,
    pub ids: usize,
    /// Ids whose in-game group is complete.
    pub complete: usize,
    /// `(id, missing in-game stems)` for the rest.
    pub incomplete: Vec<(u32, Vec<String>)>,
    /// Ids whose front-end copy is incomplete (informational).
    pub front_end_incomplete: Vec<u32>,
}

/// Census of one slot's folder under a measured contract. `None` when the contract is not a
/// measured set or the folder is absent.
pub fn family_census(rendering_root: &Path, slot_key: &str, contract: &SetContract) -> Option<SetCensus> {
    if !matches!(contract, SetContract::Measured { .. }) {
        return None;
    }
    let spec = slot_spec(slot_key)?;
    if !rendering_root.join(spec.folder).is_dir() {
        return None;
    }
    let assets = list_equipment_slot_assets_in(rendering_root, spec.folder, spec);
    let mut census = SetCensus { slot: slot_key.into(), ids: 0, complete: 0, incomplete: Vec::new(), front_end_incomplete: Vec::new() };
    for a in assets {
        // Texture ids only: an id that owns nothing but mesh / tape files (the shared meshes the
        // matchers also key, e.g. blocker style meshes) is not an equipment id of this census. An
        // id with *some* texture map but none of the majority set (Legacy blocker / trapper id 0,
        // `am` only) is an id — counted, and incomplete — exactly as R52 lists it.
        let has_texture = a.files.iter().any(|f| {
            let stem = stem_of(f);
            ["_am", "_nm", "_sm", "_dm", "_tm", "_cm"].iter().any(|suffix| stem.ends_with(suffix))
        });
        if !has_texture {
            continue;
        }
        let Some(groups) = check_components(slot_key, a.id, &a.files, contract) else { continue };
        let in_game = groups.iter().find(|g| g.group == "in-game")?;
        census.ids += 1;
        if in_game.status == "complete" {
            census.complete += 1;
        } else {
            census.incomplete.push((a.id, in_game.missing.clone()));
        }
        if groups.iter().any(|g| g.group == "front-end" && g.status == "incomplete") {
            census.front_end_incomplete.push(a.id);
        }
    }
    Some(census)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(stems: &[&str]) -> Vec<String> {
        stems.iter().map(|s| format!("glove/{s}.rx2")).collect()
    }

    #[test]
    fn a_glove_id_with_the_majority_set_is_complete_and_a_missing_tint_is_named() {
        let c = set_contract("gloves", Some("nhllegacy"), Platform::Xbox);
        assert!(matches!(c, SetContract::Measured { .. }));
        let full = files(&["glove_15_am", "glove_15_nm", "glove_15_sm", "glove_15_0_dm", "glove_15_0_tm", "glove_15_1_tm", "glove_15_2_tm", "feglove_15_0_dm", "feglove_15_0_tm", "feglove_15_1_tm", "feglove_15_2_tm"]);
        let g = check_components("gloves", 15, &full, &c).unwrap();
        assert_eq!(g.len(), 2);
        assert!(g.iter().all(|x| x.status == "complete" && x.missing.is_empty()));
        assert_eq!(g[0].present.len(), 7);
        assert_eq!(g[1].present.len(), 4);
        let partial = files(&["glove_15_am", "glove_15_nm", "glove_15_sm", "glove_15_0_dm", "glove_15_0_tm", "glove_15_1_tm", "feglove_15_0_tm", "feglove_15_1_tm", "feglove_15_2_tm"]);
        let g = check_components("gloves", 15, &partial, &c).unwrap();
        assert_eq!(g[0].status, "incomplete");
        assert_eq!(g[0].missing, vec!["glove_15_2_tm"]);
        assert_eq!(g[1].missing, vec!["feglove_15_0_dm"]);
        // Another id's files never count.
        let other = files(&["glove_16_am"]);
        let g = check_components("gloves", 15, &other, &c).unwrap();
        assert_eq!(g[0].present.len(), 0);
    }

    #[test]
    fn blocker_expects_both_styles_and_the_mask_paint_expects_shell_zero() {
        let c = set_contract("blocker", Some("nhl13"), Platform::Ps3);
        let stems: Vec<(&str, String)> = expected_stems("blocker", 7);
        assert_eq!(stems.iter().filter(|(g, _)| *g == "in-game").count(), 12);
        assert!(stems.iter().any(|(_, s)| s == "blocker_1_7_2_tm") && stems.iter().any(|(_, s)| s == "feblocker_0_7_0_dm"));
        let all: Vec<String> = stems.iter().map(|(_, s)| format!("blocker/{s}.rpsgl")).collect();
        assert!(check_components("blocker", 7, &all, &c).unwrap().iter().all(|g| g.status == "complete"));
        let c = set_contract("goalie-helmet", Some("nhllegacy"), Platform::Xbox);
        let g = check_components("goalie-helmet", 42, &["goaliehelmet/goaliehelmet_0_42_cm.rx2".into(), "goaliehelmet/goaliehelmet_6_42_cm.rx2".into()], &c).unwrap();
        assert_eq!(g[0].status, "complete");
        assert_eq!(g[0].present, vec!["goaliehelmet_0_42_cm"]);
    }

    #[test]
    fn single_container_families_and_unmeasured_titles() {
        let c = set_contract("gloves", Some("nhl11"), Platform::Xbox);
        assert!(matches!(c, SetContract::SingleContainer { .. }));
        let g = check_components("gloves", 3, &["glove/texlib_3.rx2".into()], &c).unwrap();
        assert_eq!((g[0].status.as_str(), g[0].present.len()), ("complete", 1));
        let g = check_components("gloves", 3, &["glove/glove_3_am.rx2".into()], &c).unwrap();
        assert_eq!((g[0].status.as_str(), g[0].missing.as_slice()), ("incomplete", &["texlib_3".to_string()][..]));
        assert!(matches!(set_contract("stick", Some("nhllegacy"), Platform::Xbox), SetContract::SingleContainer { .. }));
        assert!(matches!(set_contract("goalie-stick", Some("nhl11"), Platform::Xbox), SetContract::NotMeasured { .. }));
        let helmet = set_contract("helmet", Some("nhl11"), Platform::Xbox);
        assert!(matches!(helmet, SetContract::NotMeasured { .. }));
        assert!(check_components("helmet", 6, &["helmet/helmet_6_0.rx2".into(), "helmet/helmet_0_6_tm.rx2".into()], &helmet).is_none());
        let ps3_helmet = set_contract("helmet", Some("nhl11"), Platform::Ps3);
        assert!(matches!(ps3_helmet, SetContract::NotMeasured { .. }));
        assert!(check_components("helmet", 6, &["helmet/helmet_6_0.rpsgl".into(), "helmet/helmet_0_6_tm.rpsgl".into()], &ps3_helmet).is_none());
        for slot in ["gloves", "helmet"] {
            assert!(matches!(set_contract(slot, Some("nhl12"), Platform::Xbox), SetContract::NotMeasured { .. }));
            assert!(matches!(set_contract(slot, Some("nhl12"), Platform::Ps3), SetContract::NotMeasured { .. }));
        }
        for (g, p) in [(Some("nhl09"), Platform::Xbox), (Some("nhl21"), Platform::Ps4), (None, Platform::Xbox), (Some("nhl07"), Platform::Xbox)] {
            let c = set_contract("gloves", g, p);
            assert!(matches!(c, SetContract::NotMeasured { .. }), "{g:?} {p:?}");
            assert!(check_components("gloves", 1, &[], &c).is_none());
        }
    }

    #[test]
    fn the_census_counts_complete_ids_and_names_what_the_rest_lack() {
        let d = std::env::temp_dir().join(format!("nhlms-equipset-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let dir = d.join("goaliestick");
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["goaliestick_6_dm", "goaliestick_6_tm", "goaliestick_7_dm", "goaliestick_0_1", "goaliesticktape_0_1"] {
            std::fs::write(dir.join(format!("{f}.rx2")), b"x").unwrap();
        }
        let c = set_contract("goalie-stick", Some("nhllegacy"), Platform::Xbox);
        let census = family_census(&d, "goalie-stick", &c).unwrap();
        assert_eq!((census.ids, census.complete), (2, 1), "{census:?}");
        assert_eq!(census.incomplete, vec![(7, vec!["goaliestick_7_tm".to_string()])]);
        assert!(family_census(&d, "gloves", &c).is_none(), "an absent folder is None");
        assert!(family_census(&d, "goalie-stick", &set_contract("goalie-stick", Some("nhl11"), Platform::Xbox)).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }
}
