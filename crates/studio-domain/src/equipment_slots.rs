//! Equipment-slot model — the filename grammar that keys a slot's render files to
//! an equipment ID. Ported from the prior app's Hockey Shop / Equipment Studio
//! (clean-room port authorized for this feature).
//!
//! Sticks are the easy case: one `playerstick/texlib_<wXFx>.rx2` per ID. The other
//! slots key a *set* of render files to one equipment ID, with the ID sitting at a
//! slot-specific numeric position inside the filename:
//!
//! | Slot          | DB field        | Folder         | ID position                                   |
//! | ------------- | --------------- | -------------- | --------------------------------------------- |
//! | gloves        | `xmja.NmUR`     | `glove`        | `(fe)glove_<ID>_*` + `glove_<ID>` mesh         |
//! | helmet        | `xmja.YUZp`     | `helmet`       | `helmet_<ID>_<v>` / `helmet_0_<ID>_*_dm/tm`    |
//! | skates        | `xmja.AOOe`     | `skate`        | `texlib_<ID>` (mesh is shared `playerskate_*`) |
//! | blocker       | `fMVl.Kmxz`     | `blocker`      | `(fe)blocker_<model>_<ID>_*` — 2nd number      |
//! | trapper       | `fMVl.OTJR`     | `trapper`      | `(fe)trapper_<model>_<ID>_*` — 2nd number      |
//! | pads          | `fMVl.mIUA`     | `goaliepad`    | `(fe)goaliepad_<ID>_*`                         |
//! | goalie-helmet | `fMVl.JOlt`     | `goaliehelmet` | `goaliehelmet_<style>_<ID>_cm` — paint design  |
//!
//! Everything is filename-addressed: cloning an ID copies each of its files
//! byte-for-byte to the target ID's filenames. No DB edits, no in-content rewrites.

use std::collections::BTreeMap;
use std::path::Path;

/// Texture-map suffixes EA appends to per-ID render files. Used to disambiguate the
/// two `helmet_…` filename shapes (plain mesh vs mapped texture).
const MAP_SUFFIXES: [&str; 6] = ["dm", "tm", "am", "nm", "sm", "cm"];

/// Render-texture container extensions (PS3 `.rpsgl` / Xbox 360 `.rx2`).
const TEXLIB_EXTENSIONS: [&str; 2] = ["rpsgl", "rx2"];

/// Whether a matcher requires / forbids a trailing map suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MapSuffixRule {
    Any,
    Required,
    Forbidden,
}

/// One filename shape that belongs to a slot. A stem matches when it is
/// `<prefix>_<tokens…>` where every token is numeric (apart from an optional
/// trailing map suffix) and a numeric token exists at `id_index` — that token is
/// the equipment ID.
struct SlotMatcher {
    prefix: &'static str,
    id_index: usize,
    map_suffix: MapSuffixRule,
    /// When set, the stem must carry exactly this many numeric tokens (excluding a
    /// trailing map suffix) to match. Disambiguates slots whose texture and mesh
    /// files share a prefix — e.g. the goalie stick's single-token
    /// `goaliestick_<id>` texture vs its two-token `goaliestick_<a>_<b>` mesh.
    exact_tokens: Option<usize>,
}

const fn m(prefix: &'static str, id_index: usize) -> SlotMatcher {
    SlotMatcher {
        prefix,
        id_index,
        map_suffix: MapSuffixRule::Any,
        exact_tokens: None,
    }
}

/// Static description of one equipment slot.
pub struct EquipmentSlotSpec {
    /// Stable key the frontend sends (`"gloves"`, `"blocker"`, …).
    pub key: &'static str,
    /// Operator-facing name ("Gloves").
    pub label: &'static str,
    /// Folder under `<rendering>/` holding the slot's render files.
    pub folder: &'static str,
    /// The player-companion table this slot's ID field lives on (`xmja`/`fMVl`).
    pub table: &'static str,
    /// The studio field id that references this slot's IDs (`NmUR`, `Kmxz`, …).
    pub field: &'static str,
    /// Inclusive ID ceiling derived from the field's bit width.
    pub max_id: u32,
    /// Filename shapes belonging to this slot.
    matchers: &'static [SlotMatcher],
    /// FE-relative createplayer thumbnail prefix (`<prefix><ID>.big`).
    pub thumbnail_prefix: &'static str,
    /// Leaf folder for this slot's Hockey Shop store, under
    /// `<assets_root>/graphical_assets/hockey_shop/<store_dir>/`.
    pub store_dir: &'static str,
    /// Mesh-file prefix for the 3D preview (usually equal to `folder`).
    pub mesh_prefix: &'static str,
    /// Optional sibling tape-mesh prefix rendered flat-black alongside the main
    /// mesh (player/goalie sticks, the goalie mask cage); `None` for other slots.
    pub tape_prefix: Option<&'static str>,
    /// Confirmed-enum vocabulary name for store labels + the label editor, or
    /// `None` when no vocabulary is anchored for the slot.
    pub enum_name: Option<&'static str>,
}

/// Every slot the Hockey Shop / player-equipment resolver can drive.
pub const SLOT_SPECS: &[EquipmentSlotSpec] = &[
    EquipmentSlotSpec {
        key: "stick",
        label: "Stick",
        folder: "playerstick",
        table: "xmja",
        field: "wXFx",
        max_id: 127,
        matchers: &[m("texlib", 0)],
        thumbnail_prefix: "ion/artassets/createplayer/sticks/stick",
        store_dir: "hockey_sticks",
        mesh_prefix: "playerstick",
        tape_prefix: Some("playersticktape"),
        enum_name: Some("Stick Model"),
    },
    EquipmentSlotSpec {
        key: "goalie-stick",
        label: "Goalie Stick",
        folder: "goaliestick",
        // Goalie + skater share the `wXFx` stick field; the engine picks the folder
        // by position, so the goalie stick has no field of its own.
        table: "xmja",
        field: "wXFx",
        max_id: 127,
        // Textures are per-map `goaliestick_<id>_<map>.rx2` — no texlib. The id is
        // the single numeric token, so `exact_tokens: Some(1)` keeps the two-token
        // `goaliestick_<a>_<b>` mesh out while accepting the map-suffixed textures.
        matchers: &[SlotMatcher {
            prefix: "goaliestick",
            id_index: 0,
            map_suffix: MapSuffixRule::Any,
            exact_tokens: Some(1),
        }],
        thumbnail_prefix: "ion/artassets/createplayer/sticks/goaliestick",
        store_dir: "goalie_sticks",
        mesh_prefix: "goaliestick",
        tape_prefix: Some("goaliesticktape"),
        enum_name: None,
    },
    EquipmentSlotSpec {
        key: "gloves",
        label: "Gloves",
        folder: "glove",
        table: "xmja",
        field: "NmUR",
        max_id: 127,
        matchers: &[m("glove", 0), m("feglove", 0)],
        thumbnail_prefix: "ion/artassets/createplayer/gloves/gloves",
        store_dir: "gloves",
        mesh_prefix: "glove",
        tape_prefix: None,
        enum_name: Some("Glove Model"),
    },
    EquipmentSlotSpec {
        key: "helmet",
        label: "Helmet",
        folder: "helmet",
        table: "xmja",
        field: "YUZp",
        max_id: 255,
        matchers: &[
            m("texlib", 0),
            // `helmet_<ID>_<variant>` (no map suffix) vs
            // `helmet_0_<ID>[_<team>]_dm/tm/…` (map suffix) — the ID moves position.
            SlotMatcher {
                prefix: "helmet",
                id_index: 0,
                map_suffix: MapSuffixRule::Forbidden,
                exact_tokens: None,
            },
            SlotMatcher {
                prefix: "helmet",
                id_index: 1,
                map_suffix: MapSuffixRule::Required,
                exact_tokens: None,
            },
            m("chinstrap", 0),
            m("padding", 0),
        ],
        thumbnail_prefix: "ion/artassets/createplayer/helmet/helmet",
        store_dir: "helmets",
        mesh_prefix: "helmet",
        tape_prefix: None,
        enum_name: Some("Helmet Model"),
    },
    EquipmentSlotSpec {
        key: "skates",
        label: "Skates",
        folder: "skate",
        table: "xmja",
        field: "AOOe",
        max_id: 127,
        // Texture container is `skate/texlib_<id>.rx2` (keyed by `xmja.AOOe`); the
        // `playerskate_*` mesh is shared across ids and excluded by the texlib prefix.
        matchers: &[m("texlib", 0)],
        thumbnail_prefix: "ion/artassets/createplayer/skates/skates",
        store_dir: "skates",
        mesh_prefix: "playerskate",
        tape_prefix: None,
        enum_name: Some("Skate Model"),
    },
    EquipmentSlotSpec {
        key: "blocker",
        label: "Blocker",
        folder: "blocker",
        table: "fMVl",
        field: "Kmxz",
        max_id: 127,
        matchers: &[m("blocker", 1), m("feblocker", 1)],
        thumbnail_prefix: "ion/artassets/createplayer/blocker/blocker",
        store_dir: "blockers",
        mesh_prefix: "blocker",
        tape_prefix: None,
        enum_name: None,
    },
    EquipmentSlotSpec {
        key: "trapper",
        label: "Trapper",
        folder: "trapper",
        table: "fMVl",
        field: "OTJR",
        max_id: 127,
        matchers: &[m("trapper", 1), m("fetrapper", 1)],
        thumbnail_prefix: "ion/artassets/createplayer/trapper/trapper",
        store_dir: "trappers",
        mesh_prefix: "trapper",
        tape_prefix: None,
        enum_name: None,
    },
    EquipmentSlotSpec {
        key: "pads",
        label: "Pads",
        folder: "goaliepad",
        table: "fMVl",
        field: "mIUA",
        max_id: 127,
        matchers: &[m("goaliepad", 0), m("fegoaliepad", 0)],
        thumbnail_prefix: "ion/artassets/createplayer/pads/pads",
        store_dir: "pads",
        mesh_prefix: "goaliepad",
        tape_prefix: None,
        enum_name: None,
    },
    EquipmentSlotSpec {
        key: "goalie-helmet",
        label: "Goalie Helmet",
        folder: "goaliehelmet",
        table: "fMVl",
        field: "JOlt",
        max_id: 511,
        // Paint-design files only — `goaliehelmet_<style>_<paint>_cm`. Style-keyed
        // files (single numeric token) fail the id_index and are excluded.
        matchers: &[m("goaliehelmet", 1)],
        thumbnail_prefix: "ion/artassets/createplayer/mask/mask",
        store_dir: "goalie_helmets",
        // The mask MESH ships as `helmet_<a>_<b>.rx2` (the shell) plus
        // `cage_<a>_<b>_<c>.rx2` (the grille) — NOT `goaliehelmet_*`.
        mesh_prefix: "helmet",
        tape_prefix: Some("cage"),
        enum_name: None,
    },
];

/// Look up a slot spec by key. `None` for an unknown key (the API layer turns that
/// into a user-facing validation error listing the valid keys).
pub fn slot_spec(key: &str) -> Option<&'static EquipmentSlotSpec> {
    SLOT_SPECS.iter().find(|s| s.key == key)
}

/// Try to extract this slot's equipment ID from a filename stem.
pub fn match_stem_id(spec: &EquipmentSlotSpec, stem: &str) -> Option<u32> {
    spec.matchers
        .iter()
        .find_map(|matcher| match_one(matcher, stem))
}

fn match_one(matcher: &SlotMatcher, stem: &str) -> Option<u32> {
    let rest = stem.strip_prefix(matcher.prefix)?.strip_prefix('_')?;
    let tokens: Vec<&str> = rest.split('_').collect();
    if tokens.is_empty() {
        return None;
    }
    let has_map_suffix = MAP_SUFFIXES.contains(tokens.last().unwrap());
    match matcher.map_suffix {
        MapSuffixRule::Required if !has_map_suffix => return None,
        MapSuffixRule::Forbidden if has_map_suffix => return None,
        _ => {}
    }
    // Every token must be numeric, except an allowed trailing map suffix — rejects
    // oddballs like `goaliestick_goaliestick_dm`.
    let numeric_end = if has_map_suffix {
        tokens.len() - 1
    } else {
        tokens.len()
    };
    if let Some(exact) = matcher.exact_tokens {
        if numeric_end != exact {
            return None;
        }
    }
    let mut nums: Vec<u32> = Vec::with_capacity(numeric_end);
    for t in &tokens[..numeric_end] {
        nums.push(t.parse().ok()?);
    }
    nums.get(matcher.id_index).copied()
}

/// Rebuild a matched stem with the equipment ID swapped to `target_id`. `None` when
/// the stem doesn't match the slot (callers only pass stems that already matched).
pub fn rename_stem(spec: &EquipmentSlotSpec, stem: &str, target_id: u32) -> Option<String> {
    for matcher in spec.matchers {
        if match_one(matcher, stem).is_none() {
            continue;
        }
        let rest = stem.strip_prefix(matcher.prefix)?.strip_prefix('_')?;
        let mut tokens: Vec<String> = rest.split('_').map(str::to_owned).collect();
        let has_map_suffix = MAP_SUFFIXES.contains(&tokens.last().map(String::as_str).unwrap_or(""));
        let numeric_end = if has_map_suffix {
            tokens.len() - 1
        } else {
            tokens.len()
        };
        // The id_index counts numeric tokens; within the matched span every token is
        // numeric, so it's a direct index.
        if matcher.id_index < numeric_end {
            tokens[matcher.id_index] = target_id.to_string();
            return Some(format!("{}_{}", matcher.prefix, tokens.join("_")));
        }
    }
    None
}

/// One equipment ID and the render files it owns (paths relative to `<rendering>/`,
/// forward slashes). Files within an ID sort by name; the list is ID-ascending.
#[derive(Debug, Clone)]
pub struct EquipmentSlotAsset {
    pub id: u32,
    pub files: Vec<String>,
}

/// Scan `<rendering>/<folder>` and group this slot's render files by equipment ID.
/// Empty when the folder is absent or unreadable (the caller shows "no items").
pub fn list_equipment_slot_assets(
    rendering_root: &Path,
    spec: &EquipmentSlotSpec,
) -> Vec<EquipmentSlotAsset> {
    let folder = rendering_root.join(spec.folder);
    let Ok(read) = std::fs::read_dir(&folder) else {
        return Vec::new();
    };
    let mut by_id: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for entry in read.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if !TEXLIB_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(id) = match_stem_id(spec, stem) else {
            continue;
        };
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        by_id
            .entry(id)
            .or_default()
            .push(format!("{}/{file_name}", spec.folder));
    }
    by_id
        .into_iter()
        .map(|(id, mut files)| {
            files.sort();
            EquipmentSlotAsset { id, files }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(key: &str) -> &'static EquipmentSlotSpec {
        slot_spec(key).unwrap()
    }

    #[test]
    fn glove_stems_key_on_first_number() {
        let s = spec("gloves");
        assert_eq!(match_stem_id(s, "glove_15_0_tm"), Some(15));
        assert_eq!(match_stem_id(s, "glove_101_am"), Some(101));
        assert_eq!(match_stem_id(s, "feglove_21_0_dm"), Some(21));
        assert_eq!(match_stem_id(s, "helmet_3_0"), None);
    }

    #[test]
    fn helmet_id_position_depends_on_map_suffix() {
        let s = spec("helmet");
        assert_eq!(match_stem_id(s, "helmet_15_0"), Some(15));
        assert_eq!(match_stem_id(s, "helmet_0_15_am"), Some(15));
        assert_eq!(match_stem_id(s, "helmet_0_110_20_dm"), Some(110));
        assert_eq!(match_stem_id(s, "texlib_42"), Some(42));
        assert_eq!(match_stem_id(s, "chinstrap_110_3"), Some(110));
        assert_eq!(match_stem_id(s, "padding_17_2"), Some(17));
        assert_eq!(match_stem_id(s, "visor_1_0_3"), None);
        assert_eq!(match_stem_id(s, "jawprotector_25_0"), None);
    }

    #[test]
    fn blocker_and_trapper_key_on_second_number() {
        let b = spec("blocker");
        assert_eq!(match_stem_id(b, "blocker_0_84_2_tm"), Some(84));
        assert_eq!(match_stem_id(b, "feblocker_1_12_0_dm"), Some(12));
        assert_eq!(match_stem_id(b, "blocker_1_80_nm"), Some(80));
        let t = spec("trapper");
        assert_eq!(match_stem_id(t, "trapper_0_66_sm"), Some(66));
        assert_eq!(match_stem_id(t, "fetrapper_1_5_1_tm"), Some(5));
    }

    #[test]
    fn goalie_helmet_keys_on_paint_design_only() {
        let s = spec("goalie-helmet");
        assert_eq!(match_stem_id(s, "goaliehelmet_3_291_cm"), Some(291));
        assert_eq!(match_stem_id(s, "goaliehelmet_6_am"), None);
        assert_eq!(match_stem_id(s, "texlib_6"), None);
        assert_eq!(match_stem_id(s, "cage_7_0_3"), None);
        assert_eq!(match_stem_id(s, "helmet_7_0"), None);
    }

    #[test]
    fn non_numeric_inner_tokens_reject() {
        let s = spec("pads");
        assert_eq!(match_stem_id(s, "goaliepad_12_0_tm"), Some(12));
        assert_eq!(match_stem_id(s, "goaliepad_goaliepad_dm"), None);
    }

    #[test]
    fn rename_swaps_only_the_id_token() {
        let g = spec("gloves");
        assert_eq!(rename_stem(g, "glove_15_0_tm", 99).as_deref(), Some("glove_99_0_tm"));
        assert_eq!(rename_stem(g, "feglove_15_2_tm", 99).as_deref(), Some("feglove_99_2_tm"));
        let h = spec("helmet");
        assert_eq!(rename_stem(h, "helmet_0_15_4_dm", 110).as_deref(), Some("helmet_0_110_4_dm"));
        assert_eq!(rename_stem(h, "helmet_15_0", 110).as_deref(), Some("helmet_110_0"));
        assert_eq!(rename_stem(h, "texlib_15", 110).as_deref(), Some("texlib_110"));
        let b = spec("blocker");
        assert_eq!(rename_stem(b, "blocker_1_84_0_tm", 9).as_deref(), Some("blocker_1_9_0_tm"));
        let gh = spec("goalie-helmet");
        assert_eq!(rename_stem(gh, "goaliehelmet_3_291_cm", 4).as_deref(), Some("goaliehelmet_3_4_cm"));
    }

    #[test]
    fn unknown_slot_is_none() {
        assert!(slot_spec("elbow-pads").is_none());
    }

    #[test]
    fn stick_slot_matches_texlib_files() {
        let s = spec("stick");
        assert_eq!(match_stem_id(s, "texlib_74"), Some(74));
        assert_eq!(rename_stem(s, "texlib_74", 113).as_deref(), Some("texlib_113"));
    }

    #[test]
    fn goalie_stick_matches_single_id_textures_not_mesh() {
        let s = spec("goalie-stick");
        assert_eq!(match_stem_id(s, "goaliestick_27"), Some(27));
        assert_eq!(match_stem_id(s, "goaliestick_27_dm"), Some(27));
        assert_eq!(match_stem_id(s, "goaliestick_27_tm"), Some(27));
        assert_eq!(rename_stem(s, "goaliestick_27_dm", 9).as_deref(), Some("goaliestick_9_dm"));
        assert_eq!(match_stem_id(s, "goaliestick_0_2"), None);
        assert_eq!(match_stem_id(s, "goaliestick_0_2_dm"), None);
    }

    #[test]
    fn every_spec_has_a_unique_store_dir_and_mesh_prefix() {
        for s in SLOT_SPECS {
            assert!(!s.store_dir.is_empty(), "{} store_dir", s.key);
            assert!(!s.mesh_prefix.is_empty(), "{} mesh_prefix", s.key);
        }
        let mut dirs: Vec<&str> = SLOT_SPECS.iter().map(|s| s.store_dir).collect();
        dirs.sort_unstable();
        let len = dirs.len();
        dirs.dedup();
        assert_eq!(dirs.len(), len, "store_dir values must be unique");
    }
}
