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
use tdb_enums::EnumRegistry;

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
#[derive(Clone, Copy)]
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
        // The goalie's stick id is `exhibitiongoalieequipment.stick` — the goalie companion
        // (`fMVl`), same studio field id `wXFx` as the skater's `xmja.stick` (research
        // 2026-09-21 handoff 12 §2: `.stick → goaliestick n1`). Was recorded as `xmja` until
        // 21 Sep 2026, which made the goalie stick invisible to per-player checks.
        table: "fMVl",
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
        // NHL 11 (research 2026-09-21 handoff 12 addendum): the brand id sits on `texlib_<id>`.
        matchers: &[m("glove", 0), m("feglove", 0), m("texlib", 0)],
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
        matchers: &[m("blocker", 1), m("feblocker", 1), m("texlib", 0)],
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
        matchers: &[m("trapper", 1), m("fetrapper", 1), m("texlib", 0)],
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
        matchers: &[m("goaliepad", 0), m("fegoaliepad", 0), m("texlib", 0)],
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

/// NHL 12's goalie pads keep the id in the **second** numeric position of `goaliepad_*` /
/// `fegoaliepad_*` (`fegoaliepad_0_11_0_dm`), from NHL 13 in the first (`fegoaliepad_11_0_dm`) —
/// research R125 (2026-09-26 handoff 18 §1: "n2 on 12, n1 from 13"; full containment of every
/// stored value at those positions). Receipt 40.
/// NHL 12 pads (receipt 40): the id is the second number of `(fe)goaliepad_<side>_<id>_…_<map>`. Receipt 47: the
/// title's shared meshes `goaliepad_<side>_<lod>.rx2` (no map suffix; `<lod>` 0 / 2 / 3 on the real X360 tree) are
/// not pads — the `goaliepad` shape requires a map suffix, so they no longer read as pad ids 0 / 2 / 3.
const PADS_NHL12: &[SlotMatcher] = &[
    SlotMatcher { prefix: "goaliepad", id_index: 1, map_suffix: MapSuffixRule::Required, exact_tokens: None },
    m("fegoaliepad", 1),
    m("texlib", 0),
];

impl EquipmentSlotSpec {
    /// This slot as the title names its files: the spec itself, except where research measured a
    /// title-specific id position (NHL 12 pads).
    pub fn for_title(&self, game_id: Option<&str>) -> EquipmentSlotSpec {
        match (self.key, game_id) {
            ("pads", Some("nhl12")) => EquipmentSlotSpec { matchers: PADS_NHL12, ..*self },
            _ => *self,
        }
    }

    /// Whether two readings of the slot use different filename grammars (receipt 47: a slot whose naming differs
    /// by title needs a known title before anything is written).
    pub fn matchers_differ(&self, other: &EquipmentSlotSpec) -> bool {
        !std::ptr::eq(self.matchers, other.matchers)
    }
}

/// NHL 08 / 09 goalie mask **paint** (`maskcolor`): the third numeric position of
/// `goaliehelmet_<a>_<b>_<paint>_<d>_cm` (research R125b: 149 / 149 stored values on NHL 09, 99 / 99
/// on NHL 08). A file check only (no Hockey Shop slot); the field is found by its metadata name.
pub const GOALIE_MASK_PAINT_NHL08_09: EquipmentSlotSpec = EquipmentSlotSpec {
    key: "goalie-mask-paint",
    label: "Mask paint (maskcolor)",
    folder: "goaliehelmet",
    table: "fMVl",
    field: "",
    max_id: 511,
    matchers: &[m("goaliehelmet", 2)],
    thumbnail_prefix: "",
    store_dir: "",
    mesh_prefix: "helmet",
    tape_prefix: Some("cage"),
    enum_name: None,
};

/// Look up a slot spec by key. `None` for an unknown key (the API layer turns that
/// into a user-facing validation error listing the valid keys).
pub fn slot_spec(key: &str) -> Option<&'static EquipmentSlotSpec> {
    SLOT_SPECS.iter().find(|s| s.key == key)
}

/// The operator-facing model name for one equipment id, resolved from the slot's anchored enum
/// vocabulary (`defs/enums/*.toml`). `None` when the slot has no vocabulary (goalie gear) or the id
/// isn't catalogued — the caller then falls back to a generated label. Matched by the slot's
/// `enum_name` rather than its `(table, field)`, so the goalie stick (which shares `xmja.wXFx` with
/// the skater stick but has no vocabulary) does not borrow the skater stick's names.
pub fn slot_enum_label(spec: &EquipmentSlotSpec, enums: &EnumRegistry, id: u32) -> Option<String> {
    // Title-scoped: the vocabulary resolves only when the registry knows a title inside its
    // scope (`EnumRegistry::set_scope`); otherwise the id stays a number.
    let name = spec.enum_name?;
    enums
        .def_named(name)
        .and_then(|d| d.label(id as i64))
        .map(str::to_owned)
}

/// Try to extract this slot's equipment ID from a filename stem.
pub fn match_stem_id(spec: &EquipmentSlotSpec, stem: &str) -> Option<u32> {
    if let Some((_, tokens)) = styled_glove_tokens(spec, stem) {
        return tokens[1].parse().ok();
    }
    spec.matchers
        .iter()
        .find_map(|matcher| match_one(matcher, stem))
}

// NHL12 gloves include a style before the model ID. These map arities are
// distinct from the later glove_<id>_<tint>_tm and glove_<id>_am forms.
fn styled_glove_tokens<'a>(spec: &EquipmentSlotSpec, stem: &'a str) -> Option<(&'a str, Vec<&'a str>)> {
    if spec.key != "gloves" { return None; }
    let (prefix, rest) = stem.split_once('_')?;
    if !matches!(prefix, "glove" | "feglove") { return None; }
    let tokens: Vec<_> = rest.split('_').collect();
    let suffix = *tokens.last()?;
    let matches_shape = (tokens.len() == 4 && matches!(suffix, "dm" | "tm"))
        || (tokens.len() == 3 && matches!(suffix, "am" | "nm" | "sm"));
    if !matches_shape || !matches!(tokens[0], "0" | "1")
        || tokens[..tokens.len()-1].iter().any(|t| t.parse::<u32>().is_err()) { return None; }
    Some((prefix, tokens))
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
    if let Some((prefix, tokens)) = styled_glove_tokens(spec, stem) {
        let mut renamed: Vec<String> = tokens.into_iter().map(str::to_owned).collect();
        renamed[1] = target_id.to_string();
        return Some(format!("{prefix}_{}", renamed.join("_")));
    }
    for matcher in spec.matchers {
        if match_one(matcher, stem).is_none() {
            continue;
        }
        let rest = stem.strip_prefix(matcher.prefix)?.strip_prefix('_')?;
        let mut tokens: Vec<String> = rest.split('_').map(str::to_owned).collect();
        let has_map_suffix =
            MAP_SUFFIXES.contains(&tokens.last().map(String::as_str).unwrap_or(""));
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
        if !TEXLIB_EXTENSIONS
            .iter()
            .any(|e| ext.eq_ignore_ascii_case(e))
        {
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

// ------------------------------------------------------------------ Phase 4 (21 Sep 2026): file check

/// Research 2026-09-21 handoff 12 (NHL 11 addendum): on NHL 11 the families are texture-keyed —
/// the brand id sits on `texlib_<id>` and the meshes are generic — and the player-stick textures
/// live under `stick/`, not `playerstick/`. The slot's own folder is scanned first; this is the
/// second folder to try when it is absent.
pub fn alternate_folder(key: &str) -> Option<&'static str> {
    match key {
        "stick" => Some("stick"),
        _ => None,
    }
}

/// One slot of a player's equipment companion checked against the title's rendering tree
/// (research 2026-09-21 handoff 12 §3 "listing" + the display half of "reassignment"): the
/// files the assigned id owns, the size of the title's catalogue for the family, and the note.
#[derive(Debug, Clone)]
pub struct EquipmentFileCheck {
    pub slot: &'static str,
    pub label: &'static str,
    pub studio_table: String,
    pub studio_field: String,
    pub folder: String,
    pub id: Option<u32>,
    pub model_label: Option<String>,
    /// Files of the assigned id (relative to `<rendering>/`).
    pub files: Vec<String>,
    /// Distinct ids with files in the family's folder on this title.
    pub catalogue_ids: usize,
    pub catalogue_min: Option<u32>,
    pub catalogue_max: Option<u32>,
    pub note: String,
    /// Phase 4 (22 Sep 2026): the component-set contract for this slot on the open title
    /// (research R52) and, when it is measured and the id has files, the per-group check.
    pub set_contract: crate::equipment_components::SetContract,
    pub component_groups: Vec<crate::equipment_components::ComponentGroup>,
}

/// Check every equipment field of a player against the rendering tree. `is_goalie` picks the
/// goalie stick for the shared `wXFx` field. Slots whose field the player's companion does not
/// carry are omitted; an absent tree folder is reported as such, never as "no files".
/// `game_id` / `platform` choose the component-set contract (R52); they never change which files
/// are listed.
pub fn check_player_equipment_files(
    rendering_root: &Path,
    is_goalie: bool,
    fields: &[crate::view::FieldView],
    enums: &EnumRegistry,
    game_id: Option<&str>,
    platform: crate::project::Platform,
) -> Vec<EquipmentFileCheck> {
    let mut out = Vec::new();
    // NHL 08 / 09: the mask paint lives in `maskcolor` (research R125 G1), checked like a slot.
    let paint_nhl0809 = is_goalie.then_some(GOALIE_MASK_PAINT_NHL08_09);
    for spec in SLOT_SPECS.iter().map(|s| s.for_title(game_id)).chain(paint_nhl0809) {
        let spec = &spec;
        let goalie_slot = matches!(spec.key, "goalie-stick" | "blocker" | "trapper" | "pads" | "goalie-helmet" | "goalie-mask-paint");
        if spec.key == "stick" && is_goalie || spec.key == "goalie-stick" && !is_goalie {
            continue;
        }
        if goalie_slot != is_goalie && !matches!(spec.key, "stick" | "goalie-stick") {
            continue;
        }
        // The paint slot is found by its metadata name (`maskcolor`, present on NHL 08 / 09 only).
        let Some(f) = fields.iter().find(|f| f.studio_table == spec.table && if spec.key == "goalie-mask-paint" { f.name == "maskcolor" } else { f.studio_field == spec.field }) else { continue };
        let id = f.decoded.and_then(|v| u32::try_from(v).ok());
        let (folder, assets) = {
            let primary = rendering_root.join(spec.folder);
            if primary.is_dir() {
                (spec.folder.to_string(), list_equipment_slot_assets(rendering_root, spec))
            } else if let Some(alt) = alternate_folder(spec.key).filter(|a| rendering_root.join(a).is_dir()) {
                (alt.to_string(), list_equipment_slot_assets_in(rendering_root, alt, spec))
            } else {
                (spec.folder.to_string(), Vec::new())
            }
        };
        let folder_present = rendering_root.join(&folder).is_dir();
        let files = id.and_then(|i| assets.iter().find(|a| a.id == i)).map(|a| a.files.clone()).unwrap_or_default();
        let note = match (folder_present, id) {
            (false, _) => format!("no `{folder}` folder under this install's rendering tree (archive-only title, or no install tree)"),
            (true, None) => "the field carries no id".to_string(),
            (true, Some(i)) if files.is_empty() => format!("no files for id {i} in `{folder}` — the game has no art for this value on this title (the catalogue holds {} ids{})", assets.len(), range_text(&assets)),
            (true, Some(_)) => format!("{} file{} (catalogue: {} ids{})", files.len(), if files.len() == 1 { "" } else { "s" }, assets.len(), range_text(&assets)),
        };
        let set_contract = crate::equipment_components::set_contract(spec.key, game_id, platform);
        let component_groups = match id {
            Some(i) if !files.is_empty() => crate::equipment_components::check_components(spec.key, i, &files, &set_contract).unwrap_or_default(),
            _ => Vec::new(),
        };
        out.push(EquipmentFileCheck {
            slot: spec.key,
            label: spec.label,
            studio_table: f.studio_table.clone(),
            studio_field: f.studio_field.clone(),
            folder,
            id,
            model_label: id.and_then(|i| slot_enum_label(spec, enums, i)),
            files,
            catalogue_ids: assets.len(),
            catalogue_min: assets.first().map(|a| a.id),
            catalogue_max: assets.last().map(|a| a.id),
            note,
            set_contract,
            component_groups,
        });
    }
    out
}

fn range_text(assets: &[EquipmentSlotAsset]) -> String {
    match (assets.first(), assets.last()) {
        (Some(a), Some(b)) => format!(", {}–{}", a.id, b.id),
        _ => String::new(),
    }
}

/// [`list_equipment_slot_assets`] over an explicit folder (the NHL 11 `stick/` case).
pub fn list_equipment_slot_assets_in(rendering_root: &Path, folder: &str, spec: &EquipmentSlotSpec) -> Vec<EquipmentSlotAsset> {
    let dir = rendering_root.join(folder);
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut by_id: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for entry in read.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else { continue };
        if !TEXLIB_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        let Some(id) = match_stem_id(spec, stem) else { continue };
        let file_name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        by_id.entry(id).or_default().push(format!("{folder}/{file_name}"));
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
    fn styled_gloves_keep_style_and_tint_separate_from_model() {
        let s = spec("gloves");
        for stem in ["glove_0_45_0_dm", "glove_1_45_2_tm", "feglove_1_45_0_dm", "glove_0_45_am", "glove_1_45_nm"] {
            assert_eq!(match_stem_id(s, stem), Some(45), "{stem}");
            let renamed = rename_stem(s, stem, 49).unwrap();
            assert_eq!(renamed, stem.replacen("_45_", "_49_", 1));
            assert_eq!(match_stem_id(s, &renamed), Some(49));
        }
        assert_eq!(match_stem_id(s, "glove_0_2_tm"), Some(0), "later model 0, tint 2 is unchanged");
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
    fn nhl12_pads_read_the_second_number_and_skip_the_shared_meshes() {
        let s = spec("pads").for_title(Some("nhl12"));
        assert_eq!(match_stem_id(&s, "fegoaliepad_0_54_0_dm"), Some(54));
        assert_eq!(match_stem_id(&s, "goaliepad_1_11_2_tm"), Some(11));
        assert_eq!(match_stem_id(&s, "goaliepad_0_11_nm"), Some(11));
        // Receipt 47: `goaliepad_<side>_<lod>` meshes and `goaliepad_<side>_am` are shared, not pads 0 / 2 / 3.
        for mesh in ["goaliepad_0_0", "goaliepad_1_2", "goaliepad_0_3", "goaliepad_1_am"] {
            assert_eq!(match_stem_id(&s, mesh), None, "{mesh}");
        }
        assert_eq!(rename_stem(&s, "fegoaliepad_0_54_0_dm", 60).as_deref(), Some("fegoaliepad_0_60_0_dm"));
        assert_eq!(rename_stem(&s, "goaliepad_0_2", 60), None);
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
        assert_eq!(
            rename_stem(g, "glove_15_0_tm", 99).as_deref(),
            Some("glove_99_0_tm")
        );
        assert_eq!(
            rename_stem(g, "feglove_15_2_tm", 99).as_deref(),
            Some("feglove_99_2_tm")
        );
        let h = spec("helmet");
        assert_eq!(
            rename_stem(h, "helmet_0_15_4_dm", 110).as_deref(),
            Some("helmet_0_110_4_dm")
        );
        assert_eq!(
            rename_stem(h, "helmet_15_0", 110).as_deref(),
            Some("helmet_110_0")
        );
        assert_eq!(
            rename_stem(h, "texlib_15", 110).as_deref(),
            Some("texlib_110")
        );
        let b = spec("blocker");
        assert_eq!(
            rename_stem(b, "blocker_1_84_0_tm", 9).as_deref(),
            Some("blocker_1_9_0_tm")
        );
        let gh = spec("goalie-helmet");
        assert_eq!(
            rename_stem(gh, "goaliehelmet_3_291_cm", 4).as_deref(),
            Some("goaliehelmet_3_4_cm")
        );
    }

    #[test]
    fn unknown_slot_is_none() {
        assert!(slot_spec("elbow-pads").is_none());
    }

    #[test]
    fn stick_slot_matches_texlib_files() {
        let s = spec("stick");
        assert_eq!(match_stem_id(s, "texlib_74"), Some(74));
        assert_eq!(
            rename_stem(s, "texlib_74", 113).as_deref(),
            Some("texlib_113")
        );
    }

    #[test]
    fn goalie_stick_matches_single_id_textures_not_mesh() {
        let s = spec("goalie-stick");
        assert_eq!(match_stem_id(s, "goaliestick_27"), Some(27));
        assert_eq!(match_stem_id(s, "goaliestick_27_dm"), Some(27));
        assert_eq!(match_stem_id(s, "goaliestick_27_tm"), Some(27));
        assert_eq!(
            rename_stem(s, "goaliestick_27_dm", 9).as_deref(),
            Some("goaliestick_9_dm")
        );
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
