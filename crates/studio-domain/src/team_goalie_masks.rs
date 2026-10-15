//! Equipment 4.5 G2 — a team's goalie mask paints (research R125 / R125b, 2026-09-26 handoff 18 §1, §3,
//! observation G2). From NHL 10 each team stores a mask paint for its starter and its backup in up to
//! seven numbered slots (`exhibitionteams.goaliemask<k>_{starter,backup}`; NHL 10 slots 0–5 at 8 bits, NHL 11
//! 0–6, NHL 12 → Legacy 0–6 at 9 bits, PS4 0–8 with 7–8 all zero on 18–21). Every non-zero value research
//! measured on the gen7 titles is a paint file id — the same second number of `goaliehelmet_<shell>_<paint>_*`
//! that a goalie's own `maskid` names — so Studio shows each stored number with the same file check as
//! `maskid` on the loose gen7 trees.
//!
//! What this does **not** claim: which slot a uniform uses, how the team slots and the goalie's own
//! `maskid` combine, or what the game draws (runtime selection is unobserved everywhere). No mask-shell
//! name is invented. PS4 paints are catalogue records (`GoalieMaskTexture`), not files: they are shown as
//! stored numbers, and the shipped values research found outside that catalogue (248 / 249 on every
//! title from NHL 16, plus 173 / 198 / 208 / 261 on 16 and 208 on 17) are **reported**, never refused. A
//! PS4 catalogue check waits for its own contract (G3).
use std::path::Path;

use crate::equipment_slots::{list_equipment_slot_assets, slot_spec};
use crate::project::Platform;
use crate::view::FieldView;

/// What a stored paint number amounts to on this install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskPaintStatus {
    /// 0: no paint stored in this slot.
    Empty,
    /// The paint id names files in the title's `goaliehelmet` folder.
    Files,
    /// The folder is present and holds no file for this paint id.
    NoFiles,
    /// No loose `goaliehelmet` folder to check (archive-only title or no install tree).
    NoFolder,
    /// PS4: a value research measured as shipped outside `GoalieMaskTexture` (reported, not refused).
    ShippedException,
    /// PS4: stored number only (no catalogue check yet).
    NotChecked,
}

impl MaskPaintStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            MaskPaintStatus::Empty => "empty",
            MaskPaintStatus::Files => "files",
            MaskPaintStatus::NoFiles => "no_files",
            MaskPaintStatus::NoFolder => "no_folder",
            MaskPaintStatus::ShippedException => "shipped_exception",
            MaskPaintStatus::NotChecked => "not_checked",
        }
    }
}

/// One stored paint: slot `k`, starter or backup.
#[derive(Debug, Clone)]
pub struct TeamMaskPaint {
    pub slot: u32,
    /// `"starter"` or `"backup"`.
    pub role: &'static str,
    pub field_name: String,
    pub studio_table: String,
    pub studio_field: String,
    pub value: Option<u32>,
    pub files: Vec<String>,
    pub status: MaskPaintStatus,
    pub note: String,
}

/// The team's mask paints and what supports them.
#[derive(Debug, Clone, Default)]
pub struct TeamGoalieMasks {
    /// `false` on titles without team mask slots (NHL 07–09).
    pub applies: bool,
    pub paints: Vec<TeamMaskPaint>,
    /// Distinct paint ids with files in the title's `goaliehelmet` folder (gen7 loose trees).
    pub catalogue_ids: usize,
    pub notes: Vec<String>,
}

/// Research R125b: values shipped in PS4 team slots that are not `GoalieMaskTexture` records.
fn ps4_shipped_exception(game_id: Option<&str>, v: u32) -> bool {
    let from16 = matches!(game_id, Some("nhl16" | "nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21"));
    (from16 && matches!(v, 248 | 249)) || (game_id == Some("nhl16") && matches!(v, 173 | 198 | 208 | 261)) || (game_id == Some("nhl17") && v == 208)
}

/// Parse `goaliemask<k>_<role>`.
fn slot_of(name: &str) -> Option<(u32, &'static str)> {
    let rest = name.strip_prefix("goaliemask")?;
    let (k, role) = rest.split_once('_')?;
    let role = match role {
        "starter" => "starter",
        "backup" => "backup",
        _ => return None,
    };
    Some((k.parse().ok()?, role))
}

/// The team's mask paints from its `exhibitionteams` fields, checked against `rendering_root` on the gen7
/// platforms (the same paint position as the goalie `maskid` file check).
pub fn team_goalie_masks(team_fields: &[FieldView], rendering_root: Option<&Path>, game_id: Option<&str>, platform: Platform) -> TeamGoalieMasks {
    let mut found: Vec<(u32, &'static str, &FieldView)> = team_fields.iter().filter_map(|f| slot_of(&f.name).map(|(k, r)| (k, r, f))).collect();
    if found.is_empty() {
        return TeamGoalieMasks {
            applies: false,
            notes: vec!["this title stores no team goalie mask slots (they exist from NHL 10); a goalie's mask is its own mask fields".into()],
            ..Default::default()
        };
    }
    found.sort_by_key(|(k, r, _)| (*k, *r != "starter"));
    let spec = slot_spec("goalie-helmet").expect("goalie-helmet slot");
    let gen7 = platform != Platform::Ps4;
    let folder_root = rendering_root.filter(|r| gen7 && r.join(spec.folder).is_dir());
    let assets = folder_root.map(|r| list_equipment_slot_assets(r, spec)).unwrap_or_default();
    let mut paints = Vec::with_capacity(found.len());
    for (slot, role, f) in found {
        let value = f.decoded.and_then(|v| u32::try_from(v).ok());
        let (status, files, note) = match value {
            None => (MaskPaintStatus::NotChecked, Vec::new(), "the field carries no number".to_string()),
            Some(0) => (MaskPaintStatus::Empty, Vec::new(), "0 — no paint stored in this slot".to_string()),
            Some(v) if !gen7 => {
                if ps4_shipped_exception(game_id, v) {
                    (MaskPaintStatus::ShippedException, Vec::new(), format!("paint {v}: shipped in this title's team slots but not a GoalieMaskTexture record (research R125b) — reported, not refused"))
                } else {
                    (MaskPaintStatus::NotChecked, Vec::new(), format!("paint {v}: a GoalieMaskTexture catalogue number; Studio does not check PS4 mask records yet (contract G3)"))
                }
            }
            Some(v) => match folder_root {
                None => (MaskPaintStatus::NoFolder, Vec::new(), format!("paint {v}: no `{}` folder under this install's rendering tree to check (archive-only title, or no install tree)", spec.folder)),
                Some(_) => match assets.iter().find(|a| a.id == v) {
                    Some(a) => (MaskPaintStatus::Files, a.files.clone(), format!("paint {v}: {} file{} (`goaliehelmet_<shell>_{v}_…`)", a.files.len(), if a.files.len() == 1 { "" } else { "s" })),
                    None => (MaskPaintStatus::NoFiles, Vec::new(), format!("paint {v}: no `goaliehelmet_<shell>_{v}_…` file in this title's tree (the folder holds {} paint ids) — the stored number names no art here", assets.len())),
                },
            },
        };
        paints.push(TeamMaskPaint { slot, role, field_name: f.name.clone(), studio_table: f.studio_table.clone(), studio_field: f.studio_field.clone(), value, files, status, note });
    }
    let slots = paints.iter().map(|p| p.slot).max().unwrap_or(0) + 1;
    let mut notes = vec![
        format!("{slots} numbered mask slots, each with a starter and a backup paint (research R125: stored per team from NHL 10); which uniform uses which slot, and how a slot combines with the goalie's own mask number, is not established"),
        "numbers are paints, not mask shapes; nothing here says what the game draws".into(),
    ];
    if !gen7 {
        notes.push("PS4: paints are catalogue records (GoalieMaskTexture), not loose files — shown as stored numbers".into());
    }
    TeamGoalieMasks { applies: true, paints, catalogue_ids: assets.len(), notes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_names_parse_and_exceptions_follow_research() {
        assert_eq!(slot_of("goaliemask0_starter"), Some((0, "starter")));
        assert_eq!(slot_of("goaliemask6_backup"), Some((6, "backup")));
        assert_eq!(slot_of("goaliemask_backup"), None);
        assert_eq!(slot_of("goaliemaskx_starter"), None);
        assert_eq!(slot_of("maskid"), None);
        assert!(ps4_shipped_exception(Some("nhl21"), 248) && ps4_shipped_exception(Some("nhl16"), 249));
        assert!(ps4_shipped_exception(Some("nhl16"), 261) && ps4_shipped_exception(Some("nhl17"), 208));
        assert!(!ps4_shipped_exception(Some("nhl15"), 248), "NHL 15 ships no exception");
        assert!(!ps4_shipped_exception(Some("nhl18"), 208));
    }
}
